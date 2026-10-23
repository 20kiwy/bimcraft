//! JSON-RPC 2.0 message handling for the stdio transport.
//!
//! One line = one message (newline-delimited, parity with the cadcraft-mcp
//! headless server). A message with an `id` field is a request and gets a
//! response; a message without one is a notification and never gets a
//! response — not even an error reply (JSON-RPC 2.0 §4.1).

use serde_json::{json, Value};

/// Invalid JSON on one line.
pub const PARSE_ERROR: i64 = -32700;
/// Valid JSON but not a valid request object.
pub const INVALID_REQUEST: i64 = -32600;
/// No such method.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// Bad (or missing) params, unknown tool, missing permission group.
pub const INVALID_PARAMS: i64 = -32602;
/// Internal server error.
pub const INTERNAL_ERROR: i64 = -32603;

/// A parsed incoming line.
pub enum Incoming {
    /// A request (has an `id`) that must be answered.
    Request { id: Value, method: String, params: Value },
    /// A notification (no `id`); executed for its side effects, never answered.
    Notification { method: String, params: Value },
    /// The line could not be understood at the transport level. Answered with
    /// a JSON-RPC error under `id: null` (per spec we cannot know the id).
    Malformed { code: i64, message: String },
}

/// Parse one newline-delimited message.
pub fn parse_line(line: &str) -> Incoming {
    let v: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Incoming::Malformed { code: PARSE_ERROR, message: format!("parse error: {e}") },
    };
    let Some(obj) = v.as_object() else {
        return Incoming::Malformed { code: INVALID_REQUEST, message: "request must be a JSON object".into() };
    };
    let method = match obj.get("method").and_then(Value::as_str) {
        Some(m) => m.to_string(),
        None => return Incoming::Malformed { code: INVALID_REQUEST, message: "request is missing a string 'method'".into() },
    };
    let params = obj.get("params").cloned().unwrap_or_else(|| json!({}));
    match obj.get("id") {
        Some(id) => Incoming::Request { id: id.clone(), method, params },
        None => Incoming::Notification { method, params },
    }
}

/// A successful JSON-RPC response.
pub fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// A JSON-RPC error response.
pub fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_requests_from_notifications() {
        match parse_line(r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#) {
            Incoming::Request { id, method, params } => {
                assert_eq!(id, json!(7));
                assert_eq!(method, "ping");
                assert_eq!(params, json!({}));
            }
            _ => panic!("expected a request"),
        }
        match parse_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#) {
            Incoming::Notification { method, params } => {
                assert_eq!(method, "notifications/initialized");
                assert_eq!(params, json!({}));
            }
            _ => panic!("expected a notification"),
        }
    }

    #[test]
    fn malformed_lines_report_errors() {
        match parse_line("{oops") {
            Incoming::Malformed { code, .. } => assert_eq!(code, PARSE_ERROR),
            _ => panic!("expected a parse error"),
        }
        match parse_line(r#"{"jsonrpc":"2.0","id":1}"#) {
            Incoming::Malformed { code, .. } => assert_eq!(code, INVALID_REQUEST),
            _ => panic!("expected an invalid request"),
        }
    }

    #[test]
    fn explicit_null_id_still_gets_a_response() {
        match parse_line(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#) {
            Incoming::Request { id, .. } => assert_eq!(id, Value::Null),
            _ => panic!("expected a request"),
        }
    }
}
