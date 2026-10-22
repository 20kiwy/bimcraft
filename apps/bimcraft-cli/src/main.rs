//! BimCraft headless CLI (differentiator parity with cadcraft-cli):
//!
//! ```text
//! bimcraft-cli sample      <out.bimcraft.json>          write the sample house
//! bimcraft-cli info        <file.bimcraft.json>         print a model summary
//! bimcraft-cli validate    <file.bimcraft.json>         consistency checks
//! bimcraft-cli schedule    <file.bimcraft.json> [--csv] quantity schedule
//! bimcraft-cli rooms       <file.bimcraft.json>         detected rooms + annotations
//! bimcraft-cli export-dxf  <file.bimcraft.json> <out.dxf>   DXF R12 wireframe export
//! bimcraft-cli import-dxf  <in.dxf> <out.bimcraft.json> [--scale 0.001]  sketch import
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
        "rooms" => {
            let m = load_arg(&args, 2);
            for l in &m.levels {
                let loops = m.detect_rooms(l.id);
                println!("{} (elev {:.2} m): {} rooms, {} outlines", l.name, l.elevation, loops.rooms.len(), loops.outlines.len());
                for (i, r) in loops.rooms.iter().enumerate() {
                    let ann = m
                        .rooms
                        .iter()
                        .find(|a| a.level == l.id && bimcraft_bim::rooms::in_poly(a.label.x, a.label.y, &r.poly))
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| format!("Room {}", i + 1));
                    println!("  {:<14} area {:7.2} m²  perimeter {:7.2} m  holes {}", ann, r.area, r.perimeter, r.holes.len());
                }
            }
        }
        "export-dxf" => {
            let m = load_arg(&args, 2);
            let out = args.get(3).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("model.dxf"));
            std::fs::write(&out, bimcraft_io::export_dxf(&m)).unwrap_or_else(|e| die(&format!("cannot write {}: {e}", out.display())));
            println!("wrote {} (ASCII DXF R12, meters)", out.display());
        }
        "import-dxf" => {
            let path = args.get(2).unwrap_or_else(|| die("missing DXF argument"));
            let text = std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("cannot read {path}: {e}")));
            let scale = args
                .iter()
                .position(|a| a == "--scale")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.parse::<f64>().unwrap_or_else(|_| die("--scale must be a number")))
                .unwrap_or(0.001);
            let mut m = bimcraft_bim::Model::new();
            match bimcraft_io::import_dxf_into(&mut m, &text, scale) {
                Ok((imp, w, s)) => {
                    let out = args.get(3).cloned().unwrap_or_else(|| "imported.bimcraft.json".into());
                    std::fs::write(&out, bimcraft_io::save_json(&m)).unwrap_or_else(|e| die(&format!("cannot write {out}: {e}")));
                    println!("read {} entities → {w} walls, {s} slabs (scale {scale}) → {out}", imp.entity_count);
                    for warn in &imp.warnings {
                        println!("note: {warn}");
                    }
                }
                Err(e) => die(&format!("{path}: {e}")),
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
    eprintln!("  bimcraft-cli sample      <out.bimcraft.json>           write the sample house");
    eprintln!("  bimcraft-cli info        <file.bimcraft.json>         model summary");
    eprintln!("  bimcraft-cli validate    <file.bimcraft.json>         consistency checks");
    eprintln!("  bimcraft-cli schedule    <file.bimcraft.json> [--csv] quantity schedule");
    eprintln!("  bimcraft-cli rooms       <file.bimcraft.json>         detected rooms + annotations");
    eprintln!("  bimcraft-cli export-dxf  <file.bimcraft.json> <out.dxf>          DXF R12 wireframe export");
    eprintln!("  bimcraft-cli import-dxf  <in.dxf> <out.bimcraft.json> [--scale 0.001]  sketch import");
    std::process::exit(2);
}
