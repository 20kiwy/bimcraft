//! BimCraft file I/O: the versioned native JSON format and schedule CSV.
//!
//! Native format (parity: `model-write.js` / `app-autosave.js` semantics):
//!
//! ```json
//! { "format": "bimcraft.model/v2", "app": "bimcraft-rs 0.2.0", "model": { … } }
//! ```
//!
//! Deserialization is tolerant: unknown fields are ignored and missing
//! optional fields fall back to defaults, so older files open in newer apps.

use bimcraft_bim::{Model, ScheduleRow};
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "bimcraft.model/v2";
pub const APP_TAG: &str = concat!("bimcraft-rs ", env!("CARGO_PKG_VERSION"));

#[derive(Serialize, Deserialize)]
struct NativeFile {
    format: String,
    app: String,
    model: Model,
}

#[derive(Debug)]
pub struct LoadError(pub String);

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LoadError {}

/// Serialize a model into the native JSON format.
pub fn save_json(model: &Model) -> String {
    let file = NativeFile { format: FORMAT.into(), app: APP_TAG.into(), model: model.clone() };
    serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".into())
}

/// Parse the native JSON format. Accepts bare models (without the envelope)
/// as a convenience for hand-written files.
pub fn load_json(text: &str) -> Result<Model, LoadError> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| LoadError(format!("not valid JSON: {e}")))?;
    if v.get("format").and_then(|f| f.as_str()) == Some(FORMAT) {
        let file: NativeFile = serde_json::from_value(v).map_err(|e| LoadError(format!("bad native file: {e}")))?;
        return Ok(file.model);
    }
    // Bare model (no envelope).
    serde_json::from_value::<Model>(v).map_err(|e| LoadError(format!("unrecognized file: {e}")))
}

/// Serialize the schedule table as CSV (parity: schedule export).
pub fn schedule_csv(model: &Model) -> String {
    let mut out = String::from("group,name,count,unit,quantity\n");
    for r in model.schedule() {
        out.push_str(&format!("{},{},{},{},{:.2}\n", r.group, csv_escape(&r.name), r.count, r.unit, r.quantity));
    }
    out
}

/// A compact text summary for the CLI `info` command.
pub fn summary_text(model: &Model) -> String {
    let mut s = String::new();
    s.push_str(&format!("format  : {FORMAT}\n"));
    s.push_str(&format!("levels  : {}\n", model.levels.len()));
    for l in &model.levels {
        s.push_str(&format!("  - {}  elev {:.2} m  height {:.2} m\n", l.name, l.elevation, l.height));
    }
    s.push_str(&format!("grids   : {}\n", model.grids.len()));
    s.push_str(&format!("walls   : {}\n", model.walls().count()));
    s.push_str(&format!("doors   : {}\n", model.elements.iter().filter(|e| e.kind == bimcraft_bim::ElementKind::Door).count()));
    s.push_str(&format!("windows : {}\n", model.elements.iter().filter(|e| e.kind == bimcraft_bim::ElementKind::Window).count()));
    s.push_str(&format!("floors  : {}\n", model.elements.iter().filter(|e| e.kind == bimcraft_bim::ElementKind::Slab).count()));
    s.push_str(&format!("ceilings: {}\n", model.elements.iter().filter(|e| e.kind == bimcraft_bim::ElementKind::Ceiling).count()));
    s.push_str(&format!("items   : {}\n", model.items().count()));
    if let Some((min, max)) = model.bounds() {
        s.push_str(&format!("bounds  : X {:.2}..{:.2}  Y {:.2}..{:.2}  Z {:.2}..{:.2}\n", min.x, max.x, min.y, max.y, min.z, max.z));
    }
    s
}

/// Consistency checks for the CLI `validate` command.
pub fn validate(model: &Model) -> Vec<String> {
    let mut issues = Vec::new();
    if model.levels.is_empty() {
        issues.push("model has no levels".into());
    }
    for w in model.walls() {
        if let bimcraft_bim::ElementData::Wall(wall) = &w.data {
            if wall.pts.len() < 2 {
                issues.push(format!("{}: wall has fewer than 2 points", w.name));
            }
            if wall.thickness <= 0.0 {
                issues.push(format!("{}: non-positive thickness", w.name));
            }
            for p in &wall.pts {
                if !p.x.is_finite() || !p.y.is_finite() {
                    issues.push(format!("{}: non-finite vertex", w.name));
                }
            }
            if wall.level != 0 && model.level(wall.level).is_none() {
                issues.push(format!("{}: references missing level", w.name));
            }
        }
    }
    for e in &model.elements {
        if let bimcraft_bim::ElementData::Opening(o) = &e.data {
            if model.wall(o.wall).is_none() {
                issues.push(format!("{}: host wall {} missing", e.name, o.wall));
                continue;
            }
            let len = model.wall(o.wall).map(|w| w.total_length()).unwrap_or(0.0);
            if o.s < 0.0 || o.s > len {
                issues.push(format!("{}: opening position {:.2} m outside wall ({} m)", e.name, o.s, len));
            }
            if o.width <= 0.0 || o.height <= 0.0 {
                issues.push(format!("{}: non-positive size", e.name));
            }
        }
        let poly: &[bimcraft_geom::Vec2] = match &e.data {
            bimcraft_bim::ElementData::Slab(sl) => &sl.pts,
            bimcraft_bim::ElementData::Ceiling(c) => &c.pts,
            _ => continue,
        };
        if !Model::poly_valid(poly) {
            issues.push(format!("{}: degenerate polygon", e.name));
        }
    }
    issues
}

/// Pretty schedule table for the CLI.
pub fn schedule_table(model: &Model) -> String {
    let rows: &[ScheduleRow] = &model.schedule();
    let w_name = rows.iter().map(|r| r.name.len()).max().unwrap_or(4).max(4);
    let w_group = rows.iter().map(|r| r.group.len()).max().unwrap_or(5).max(5);
    let mut out = format!(
        "{:<w_group$}  {:<w_name$}  {:>5}  {:>4}  {:>10}\n",
        "group",
        "name",
        "count",
        "unit",
        "quantity",
        w_group = w_group,
        w_name = w_name
    );
    out.push_str(&"-".repeat(out.len() - 1));
    out.push('\n');
    for r in rows {
        out.push_str(&format!(
            "{:<w_group$}  {:<w_name$}  {:>5}  {:>4}  {:>10.2}\n",
            r.group,
            r.name,
            r.count,
            r.unit,
            r.quantity,
            w_group = w_group,
            w_name = w_name
        ));
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bimcraft_bim::sample;

    #[test]
    fn roundtrip_with_envelope() {
        let m = sample();
        let json = save_json(&m);
        let back = load_json(&json).unwrap();
        assert_eq!(back.elements.len(), m.elements.len());
        assert_eq!(back.levels.len(), m.levels.len());
        assert_eq!(back.next_id_peek(), m.next_id_peek());
    }

    #[test]
    fn bare_model_accepted() {
        let m = sample();
        let json = serde_json::to_string(&m).unwrap();
        let back = load_json(&json).unwrap();
        assert_eq!(back.elements.len(), m.elements.len());
    }

    #[test]
    fn rejects_garbage() {
        assert!(load_json("nope").is_err());
        assert!(load_json("{\"format\": \"bimcraft.model/v9\"}").is_err());
    }

    #[test]
    fn csv_and_summary_render() {
        let m = sample();
        let csv = schedule_csv(&m);
        assert!(csv.starts_with("group,name,count,unit,quantity"));
        assert!(csv.lines().count() > 5);
        let sum = summary_text(&m);
        assert!(sum.contains("walls   :"));
    }

    #[test]
    fn validate_sample_is_clean() {
        let issues = validate(&sample());
        assert!(issues.is_empty(), "unexpected issues: {issues:?}");
    }

    #[test]
    fn validate_catches_broken_host() {
        let mut m = sample();
        m.elements.push(bimcraft_bim::Element {
            id: 99999,
            kind: bimcraft_bim::ElementKind::Door,
            name: "Orphan".into(),
            data: bimcraft_bim::ElementData::Opening(bimcraft_bim::Opening { wall: 424242, s: 1.0, width: 0.9, height: 2.1, sill: 0.0 }),
        });
        let issues = validate(&m);
        assert!(issues.iter().any(|i| i.contains("host wall")));
    }
}
