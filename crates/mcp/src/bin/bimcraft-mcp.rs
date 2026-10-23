//! `bimcraft-mcp` — headless JSON-RPC/MCP automation server for BimCraft.
//!
//! Reads newline-delimited JSON-RPC 2.0 on stdin, writes one response line
//! per request on stdout (notifications are never answered). The session
//! starts on the sample house, exactly like the app's first run.
//!
//! ```text
//! bimcraft-mcp --allow read,edit,save,export [--root DIR] [--log FILE]
//! ```

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut allow_spec = String::from("read");
    let mut root: Option<PathBuf> = None;
    let mut log: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--allow" => allow_spec = next(&args, &mut i, "--allow"),
            "--root" => root = Some(PathBuf::from(next(&args, &mut i, "--root"))),
            "--log" => log = Some(PathBuf::from(next(&args, &mut i, "--log"))),
            "--help" | "-h" => {
                print_usage();
                return;
            }
            other => die(&format!("unknown argument '{other}' (see --help)")),
        }
        i += 1;
    }

    let groups = bimcraft_mcp::Groups::parse(&allow_spec).unwrap_or_else(|e| die(&e));
    let mut dispatcher = bimcraft_mcp::Dispatcher::new(groups);
    if let Some(root) = root {
        dispatcher = dispatcher.with_root(root).unwrap_or_else(|e| die(&e));
    }
    if let Some(log) = log {
        dispatcher = dispatcher.with_log(log);
    }
    if let Err(e) = bimcraft_mcp::run_stdio(&mut dispatcher) {
        die(&format!("stdio loop failed: {e}"));
    }
}

fn next(args: &[String], i: &mut usize, flag: &str) -> String {
    *i += 1;
    args.get(*i).cloned().unwrap_or_else(|| die(&format!("{flag} needs a value")))
}

fn print_usage() {
    eprintln!(
        "bimcraft-mcp — JSON-RPC 2.0 / MCP automation server over stdio\n\
         \n\
         usage: bimcraft-mcp [--allow GROUPS] [--root DIR] [--log FILE]\n\
         \n\
         --allow GROUPS   comma-separated permission groups to enable:\n\
         read|edit|save|export (default: read)\n\
         --root DIR       confine model_save/model_load/dxf_export paths to DIR\n\
         --log FILE       append a request/response trace to FILE\n\
         \n\
         one JSON-RPC message per stdin line; responses on stdout."
    );
}

fn die(msg: &str) -> ! {
    eprintln!("bimcraft-mcp: {msg}");
    std::process::exit(2);
}
