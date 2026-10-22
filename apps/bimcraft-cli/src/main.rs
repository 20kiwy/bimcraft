//! BimCraft headless CLI (differentiator parity with cadcraft-cli):
//!
//! ```text
//! bimcraft-cli sample   <out.bimcraft.json>        write the sample house
//! bimcraft-cli info     <file.bimcraft.json>        print a model summary
//! bimcraft-cli validate <file.bimcraft.json>        consistency checks
//! bimcraft-cli schedule <file.bimcraft.json> [--csv]  quantity schedule
//! ```

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    let cmd = args[1].as_str();
    match cmd {
        "sample" => {
            let out = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("sample.bimcraft.json"));
            let model = bimcraft_bim::sample();
            std::fs::write(&out, bimcraft_io::save_json(&model)).unwrap_or_else(|e| die(&format!("cannot write {}: {e}", out.display())));
            println!("wrote {} ({} elements)", out.display(), model.elements.len());
        }
        "info" => {
            let m = load_arg(&args, 2);
            print!("{}", bimcraft_io::summary_text(&m));
        }
        "validate" => {
            let m = load_arg(&args, 2);
            let issues = bimcraft_io::validate(&m);
            if issues.is_empty() {
                println!("OK — no issues found ({} elements)", m.elements.len());
            } else {
                for i in &issues {
                    println!("issue: {i}");
                }
                std::process::exit(1);
            }
        }
        "schedule" => {
            let m = load_arg(&args, 2);
            let csv = args.iter().any(|a| a == "--csv");
            if csv {
                print!("{}", bimcraft_io::schedule_csv(&m));
            } else {
                print!("{}", bimcraft_io::schedule_table(&m));
            }
        }
        "--help" | "-h" | "help" => usage(),
        other => {
            eprintln!("unknown command: {other}");
            usage();
        }
    }
}

fn load_arg(args: &[String], idx: usize) -> bimcraft_bim::Model {
    let path = args.get(idx).unwrap_or_else(|| die("missing file argument"));
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("cannot read {path}: {e}")));
    match bimcraft_io::load_json(&text) {
        Ok(m) => m,
        Err(e) => die(&format!("{path}: {e}")),
    }
}

fn die(msg: &str) -> ! {
    eprintln!("bimcraft-cli: {msg}");
    std::process::exit(2);
}

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  bimcraft-cli sample   <out.bimcraft.json>         write the sample house");
    eprintln!("  bimcraft-cli info     <file.bimcraft.json>        model summary");
    eprintln!("  bimcraft-cli validate <file.bimcraft.json>        consistency checks");
    eprintln!("  bimcraft-cli schedule <file.bimcraft.json> [--csv] quantity schedule");
    std::process::exit(2);
}
