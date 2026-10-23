//! BimCraft MCP: a DOM-free automation server for the BIM CAD app.
//!
//! Clean-room port of the cadcraft dispatcher design (see
//! docs/PARITY_ROADMAP.md phase 4): JSON-RPC 2.0 over stdio, one line per
//! message, permission groups `read|edit|save|export`, and every edit tool =
//! exactly one undo step. The same dispatcher is usable in-process (embed it
//! next to the UI) or headless via the `bimcraft-mcp` binary:
//!
//! ```text
//! bimcraft-mcp --allow read,edit,save,export [--root DIR] [--log FILE]
//! ```
//!
//! Protocol surface: `initialize`, `notifications/initialized`, `ping`,
//! `tools/list`, `tools/call`. Unknown methods answer JSON-RPC `-32601`, bad
//! params `-32602`, tool failures come back as `isError: true` content so an
//! agent can read the message and self-correct.

pub mod handlers;
pub mod rpc;
pub mod tools;

use bimcraft_bim::Model;
use bimcraft_engine::Session;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;

pub use tools::ToolSpec;

/// Permission groups (parity: cadcraft `--allow read,edit,save,export`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// Safe queries (describe, list, get).
    Read,
    /// Model mutations — each edit tool is one undo step.
    Edit,
    /// Native file save / load / new.
    Save,
    /// Exchange formats (DXF).
    Export,
}

impl Group {
    /// The `--allow` spelling of the group.
    pub fn key(self) -> &'static str {
        match self {
            Group::Read => "read",
            Group::Edit => "edit",
            Group::Save => "save",
            Group::Export => "export",
        }
    }

    fn parse(s: &str) -> Option<Group> {
        match s.trim().to_ascii_lowercase().as_str() {
            "read" => Some(Group::Read),
            "edit" => Some(Group::Edit),
            "save" => Some(Group::Save),
            "export" => Some(Group::Export),
            _ => None,
        }
    }
}

/// The set of enabled permission groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Groups {
    read: bool,
    edit: bool,
    save: bool,
    export: bool,
}

impl Groups {
    /// No groups enabled.
    pub fn none() -> Groups {
        Groups { read: false, edit: false, save: false, export: false }
    }

    /// All groups enabled (in-process / test convenience).
    pub fn all() -> Groups {
        Groups { read: true, edit: true, save: true, export: true }
    }

    /// Parse a comma-separated `--allow` spec: `"read,edit"`. Unknown groups
    /// are an error so a typo never silently widens permissions.
    pub fn parse(spec: &str) -> Result<Groups, String> {
        let mut g = Groups::none();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let group = Group::parse(part).ok_or_else(|| format!("unknown permission group '{part}' (expected read|edit|save|export)"))?;
            g.set(group);
        }
        Ok(g)
    }

    /// Enable one group.
    pub fn set(&mut self, g: Group) {
        match g {
            Group::Read => self.read = true,
            Group::Edit => self.edit = true,
            Group::Save => self.save = true,
            Group::Export => self.export = true,
        }
    }

    /// Whether calls that require `g` may run.
    pub fn allows(self, g: Group) -> bool {
        match g {
            Group::Read => self.read,
            Group::Edit => self.edit,
            Group::Save => self.save,
            Group::Export => self.export,
        }
    }

    /// Enabled group keys, catalog order.
    pub fn keys(self) -> Vec<&'static str> {
        [Group::Read, Group::Edit, Group::Save, Group::Export].into_iter().filter(|g| self.allows(*g)).map(Group::key).collect()
    }
}

/// The automation server: one [Session] plus permissions and optional root /
/// log files. No DOM, no UI state beyond what the session needs for edit
/// semantics.
pub struct Dispatcher {
    session: Session,
    allow: Groups,
    root: Option<PathBuf>,
    log: Option<PathBuf>,
}

impl Dispatcher {
    /// A dispatcher over the sample house (what the app opens with).
    pub fn new(allow: Groups) -> Self {
        Self::with_session(Session::default(), allow)
    }

    /// A dispatcher over an explicit model (tests, headless pipelines).
    pub fn with_model(model: Model, allow: Groups) -> Self {
        Self::with_session(Session::with_model(model), allow)
    }

    /// A dispatcher over an existing session (shares undo history).
    ///
    /// The session model's `auto_foundation` is normalized to the documented
    /// default (`true`, the serde default and what "floors remember their
    /// foundations" promises). In-memory `Model::default()`-built models carry
    /// `false` only because the derive ignores the serde field default, which
    /// would silently disable the engine's auto-grounding for MCP edits.
    /// Models loaded through `model_load` keep whatever the file says.
    pub fn with_session(mut session: Session, allow: Groups) -> Self {
        session.model.auto_foundation = true;
        Dispatcher { session, allow, root: None, log: None }
    }

    /// Restrict save/load/export paths to `root` (canonicalized at setup so
    /// relative paths and `..` escapes resolve against the real folder).
    pub fn with_root(mut self, root: PathBuf) -> Result<Self, String> {
        let canon = root.canonicalize().map_err(|e| format!("--root {}: {e}", root.display()))?;
        self.root = Some(canon);
        Ok(self)
    }

    /// Append a `< request` / `> response` trace to a log file.
    pub fn with_log(mut self, log: PathBuf) -> Self {
        self.log = Some(log);
        self
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    pub fn allow(&self) -> Groups {
        self.allow
    }

    /// Handle one newline-delimited message. Returns the response line to
    /// write, or `None` for notifications (never answered) and blank lines.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        if line.trim().is_empty() {
            return None;
        }
        let response = match rpc::parse_line(line) {
            rpc::Incoming::Request { id, method, params } => self.handle_request(id, &method, params),
            rpc::Incoming::Notification { method, params } => {
                self.handle_notification(&method, &params);
                return None;
            }
            rpc::Incoming::Malformed { code, message } => rpc::error(Value::Null, code, &message).to_string(),
        };
        self.log_trace(line, &response);
        Some(response)
    }

    fn handle_notification(&mut self, method: &str, _params: &Value) {
        // `notifications/initialized` and friends have no effect yet; unknown
        // notifications are ignored per JSON-RPC (no error replies to them).
        let _ = method;
    }

    fn handle_request(&mut self, id: Value, method: &str, params: Value) -> String {
        let result = match method {
            "initialize" => Ok(self.initialize()),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": self.allowed_tools() })),
            "tools/call" => self.tools_call(params),
            other => Err((rpc::METHOD_NOT_FOUND, format!("method not found: {other}"))),
        };
        match result {
            Ok(v) => rpc::success(id, v).to_string(),
            Err((code, message)) => rpc::error(id, code, &message).to_string(),
        }
    }

    fn initialize(&self) -> Value {
        json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": { "name": "bimcraft-mcp", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": { "tools": { "listChanged": false } }
        })
    }

    /// `tools/list` filtered to the enabled permission groups.
    pub fn allowed_tools(&self) -> Vec<Value> {
        tools::catalog()
            .into_iter()
            .filter(|t| self.allow.allows(t.group))
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema,
                    "group": t.group.key(),
                })
            })
            .collect()
    }

    /// `tools/call`: { name, arguments } → MCP content result.
    fn tools_call(&mut self, params: Value) -> Result<Value, (i64, String)> {
        let name = params.get("name").and_then(Value::as_str).ok_or((rpc::INVALID_PARAMS, "'name' is required (string)".to_string()))?.to_string();
        let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
        if !args.is_object() {
            return Err((rpc::INVALID_PARAMS, "'arguments' must be an object".into()));
        }
        let Some(spec) = tools::catalog().into_iter().find(|t| t.name == name) else {
            return Err((rpc::INVALID_PARAMS, format!("unknown tool '{name}'")));
        };
        if !self.allow.allows(spec.group) {
            return Err((
                rpc::INVALID_PARAMS,
                format!("tool '{name}' requires the '{}' permission group (enabled: {})", spec.group.key(), self.allow.keys().join(",")),
            ));
        }
        let root = self.root.clone();
        match handlers::run_tool(&mut self.session, root.as_deref(), &name, &args) {
            Ok(v) => Ok(text_content(v, false)),
            Err(message) => Ok(text_content(json!({ "error": message }), true)),
        }
    }

    fn log_trace(&self, request: &str, response: &str) {
        let Some(log) = &self.log else { return };
        let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log) else {
            eprintln!("bimcraft-mcp: cannot open log file {}", log.display());
            return;
        };
        let _ = writeln!(f, "< {request}");
        let _ = writeln!(f, "> {response}");
    }
}

/// MCP tool result: one text content block carrying the tool's JSON payload.
fn text_content(v: Value, is_error: bool) -> Value {
    let mut out = json!({
        "content": [{ "type": "text", "text": v.to_string() }],
    });
    if is_error {
        out["isError"] = json!(true);
    }
    out
}

/// Run the newline-delimited JSON-RPC loop over stdio until EOF.
pub fn run_stdio(dispatcher: &mut Dispatcher) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in std::io::BufRead::lines(stdin.lock()) {
        let line = line?;
        if let Some(response) = dispatcher.handle_line(&line) {
            writeln!(out, "{response}")?;
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
