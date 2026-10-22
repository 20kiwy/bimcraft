//! DXF exchange (ASCII R12).
//!
//! **Export** writes the model as a 3-D wireframe: wall outline loops at sill
//! and head height, opening rectangles, slab/ceiling loops, grid lines, room
//! and section annotations — one ACI-coloured layer per element kind, meters
//! as the unit.
//!
//! **Import** is a minimal reader for plan sketches: `LINE`, `LWPOLYLINE`,
//! `POLYLINE`/`VERTEX`, `CIRCLE` and `TEXT` entities from the ENTITIES section
//! (blocks are counted, not expanded — full reader parity with the JavaScript
//! edition's `dxf-model.js` is a P4 roadmap item). [import_into] maps closed
//! polylines to floor slabs and open polylines / lines to wall centerlines.
//!
//! Written from the public DXF group-code reference (clean-room; the JS edition's
//! module is likewise written from the reference only).

use crate::{ElementData, ElementKind, Id, Model};
use bimcraft_geom::Vec2;

// ------------------------------------------------------------- export -----

const L_WALLS: &str = "BIM-WALLS";
const L_OPEN: &str = "BIM-OPENINGS";
const L_SLABS: &str = "BIM-SLABS";
const L_CEIL: &str = "BIM-CEILINGS";
const L_ITEMS: &str = "BIM-ITEMS";
const L_GRID: &str = "BIM-GRID";
const L_ROOMS: &str = "BIM-ROOMS";
const L_SECT: &str = "BIM-SECTIONS";

fn layer_color(name: &str) -> i64 {
    match name {
        L_WALLS => 7,
        L_OPEN => 4,
        L_SLABS => 5,
        L_CEIL => 6,
        L_ITEMS => 3,
        L_GRID => 1,
        L_ROOMS => 2,
        L_SECT => 8,
        _ => 7,
    }
}

/// Serialize the whole model into an ASCII DXF R12 file (units: meters).
pub fn export_dxf(model: &Model) -> String {
    let mut e: Vec<String> = Vec::new();

    let mut bounds_min = Vec2::new(f64::INFINITY, f64::INFINITY);
    let mut bounds_max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let grow = |p: Vec2, g: &mut (Vec2, Vec2)| {
        g.0.x = g.0.x.min(p.x);
        g.0.y = g.0.y.min(p.y);
        g.1.x = g.1.x.max(p.x);
        g.1.y = g.1.y.max(p.y);
    };
    for wall in model.walls() {
        if let ElementData::Wall(w) = &wall.data {
            for p in &w.pts {
                grow(*p, &mut (bounds_min, bounds_max));
            }
        }
    }
    if bounds_min.x.is_finite() {
        // pad so the file opens with a visible margin
        bounds_min = bounds_min - Vec2::new(2.0, 2.0);
        bounds_max = bounds_max + Vec2::new(2.0, 2.0);
    } else {
        bounds_min = Vec2::new(0.0, 0.0);
        bounds_max = Vec2::new(10.0, 10.0);
    }

    let dline = |e: &mut Vec<String>, layer: &str, a: (f64, f64, f64), b: (f64, f64, f64)| {
        for (code, val) in
            [(0, "LINE".into()), (8, layer.into()), (10, fmt(a.0)), (20, fmt(a.1)), (30, fmt(a.2)), (11, fmt(b.0)), (21, fmt(b.1)), (31, fmt(b.2))]
        {
            e.push(code.to_string());
            e.push(val);
        }
    };

    let dtext = |e: &mut Vec<String>, layer: &str, p: (f64, f64, f64), h: f64, text: &str| {
        for (code, val) in
            [(0, "TEXT".into()), (8, layer.into()), (10, fmt(p.0)), (20, fmt(p.1)), (30, fmt(p.2)), (40, fmt(h)), (1, text.to_string())]
        {
            e.push(code.to_string());
            e.push(val);
        }
    };

    let dpoly = |e: &mut Vec<String>, layer: &str, pts: &[Vec2], z: f64, closed: bool| {
        if pts.len() < 2 {
            return;
        }
        for (code, val) in [(0, "POLYLINE".to_string()), (8, layer.into()), (66, "1".into()), (70, if closed { "1".into() } else { "0".into() })] {
            e.push(code.to_string());
            e.push(val);
        }
        for p in pts {
            for (code, val) in [(0, "VERTEX".to_string()), (8, layer.into()), (10, fmt(p.x)), (20, fmt(p.y)), (30, fmt(z))] {
                e.push(code.to_string());
                e.push(val);
            }
        }
        e.push("0".into());
        e.push("SEQEND".into());
    };

    // Walls: outline loop at floor and head height + verticals at every vertex.
    for el in model.walls() {
        let ElementData::Wall(w) = &el.data else { continue };
        let Some(level) = model.level(w.level) else { continue };
        let outline = w.outline();
        dpoly(&mut e, L_WALLS, &outline, level.elevation, true);
        dpoly(&mut e, L_WALLS, &outline, level.elevation + w.height, true);
        for p in &outline {
            dline(&mut e, L_WALLS, (p.x, p.y, level.elevation), (p.x, p.y, level.elevation + w.height));
        }
    }

    // Openings: the jamb rectangle standing in the wall.
    for el in &model.elements {
        if !matches!(el.kind, ElementKind::Door | ElementKind::Window) {
            continue;
        }
        if let ElementData::Opening(o) = &el.data {
            if let Some(q) = model.opening_rect(o) {
                for k in 0..4 {
                    let a = q[k];
                    let b = q[(k + 1) % 4];
                    dline(&mut e, L_OPEN, (a.x, a.y, a.z), (b.x, b.y, b.z));
                }
            }
        }
    }

    // Slabs + ceilings: bottom and top loops.
    for el in &model.elements {
        let Some((lvl, pts, thk, elev)) = el.data.plan_poly() else { continue };
        let Some(level) = model.level(lvl) else { continue };
        let (layer, z0, z1) = if el.kind == ElementKind::Slab {
            (L_SLABS, level.elevation + elev - thk, level.elevation + elev)
        } else {
            (L_CEIL, level.elevation + elev, level.elevation + elev + thk)
        };
        dpoly(&mut e, layer, pts, z0, true);
        dpoly(&mut e, layer, pts, z1, true);
    }

    // Items: footprint rectangle on the floor.
    for el in model.items() {
        if let ElementData::Item(i) = &el.data {
            let Some(level) = model.level(i.level) else { continue };
            let bb = i.footprint();
            let pts = [bb.min, Vec2::new(bb.max.x, bb.min.y), bb.max, Vec2::new(bb.min.x, bb.max.y)];
            dpoly(&mut e, L_ITEMS, &pts, level.elevation, true);
        }
    }

    // Grids: datum lines spanning the model bounds + name text.
    for g in &model.grids {
        let (a, b, tpos) = match g.axis {
            crate::GridAxis::V => {
                (Vec2::new(g.position, bounds_min.y), Vec2::new(g.position, bounds_max.y), Vec2::new(g.position, bounds_max.y - 0.6))
            }
            crate::GridAxis::H => {
                (Vec2::new(bounds_min.x, g.position), Vec2::new(bounds_max.x, g.position), Vec2::new(bounds_max.x - 0.6, g.position))
            }
        };
        dline(&mut e, L_GRID, (a.x, a.y, 0.0), (b.x, b.y, 0.0));
        dtext(&mut e, L_GRID, (tpos.x, tpos.y, 0.0), 0.35, &g.name);
    }

    // Rooms: name + area label.
    for l in &model.levels {
        let walls: Vec<&crate::Wall> = model
            .elements
            .iter()
            .filter_map(|el| match &el.data {
                ElementData::Wall(w) if w.level == l.id => Some(w),
                _ => None,
            })
            .collect();
        let loops = crate::rooms::plan_loops(&crate::rooms::wall_joined_outlines(walls));
        for (i, r) in loops.rooms.iter().enumerate() {
            let ann = model.rooms.iter().find(|a| a.level == l.id && crate::rooms::in_poly(a.label.x, a.label.y, &r.poly));
            let name = ann.map(|a| a.name.clone()).unwrap_or_else(|| format!("Room {}", i + 1));
            dtext(&mut e, L_ROOMS, (r.label.x, r.label.y, l.elevation), 0.3, &format!("{name} · {:.1} m²", r.area));
        }
    }

    // Sections: cut line + name.
    for s in &model.sections {
        let z = model.level(s.level).map(|l| l.elevation).unwrap_or(0.0);
        dline(&mut e, L_SECT, (s.a.x, s.a.y, z), (s.b.x, s.b.y, z));
        let m = (s.a + s.b) * 0.5;
        dtext(&mut e, L_SECT, (m.x, m.y, z), 0.3, &s.name);
    }

    // ---- assemble the file ----
    fn w(out: &mut String, code: i32, val: &str) {
        out.push_str(&code.to_string());
        out.push('\n');
        out.push_str(val);
        out.push('\n');
    }
    let mut out = String::with_capacity(e.len() * 6 + 400);
    w(&mut out, 0, "SECTION");
    w(&mut out, 2, "HEADER");
    w(&mut out, 9, "$ACADVER");
    w(&mut out, 1, "AC1009");
    w(&mut out, 9, "$INSBASE");
    w(&mut out, 10, "0.0");
    w(&mut out, 20, "0.0");
    w(&mut out, 30, "0.0");
    w(&mut out, 9, "$EXTMIN");
    w(&mut out, 10, &fmt(bounds_min.x));
    w(&mut out, 20, &fmt(bounds_min.y));
    w(&mut out, 30, "0.0");
    w(&mut out, 9, "$EXTMAX");
    w(&mut out, 10, &fmt(bounds_max.x));
    w(&mut out, 20, &fmt(bounds_max.y));
    w(&mut out, 30, "0.0");
    w(&mut out, 0, "ENDSEC");
    w(&mut out, 0, "SECTION");
    w(&mut out, 2, "TABLES");
    w(&mut out, 0, "TABLE");
    w(&mut out, 2, "LTYPE");
    w(&mut out, 70, "1");
    w(&mut out, 0, "LTYPE");
    w(&mut out, 2, "CONTINUOUS");
    w(&mut out, 70, "64");
    w(&mut out, 3, "Solid line");
    w(&mut out, 72, "65");
    w(&mut out, 73, "0");
    w(&mut out, 40, "0.0");
    w(&mut out, 0, "ENDTAB");
    w(&mut out, 0, "TABLE");
    w(&mut out, 2, "LAYER");
    w(&mut out, 70, "8");
    for layer in [L_WALLS, L_OPEN, L_SLABS, L_CEIL, L_ITEMS, L_GRID, L_ROOMS, L_SECT] {
        w(&mut out, 0, "LAYER");
        w(&mut out, 2, layer);
        w(&mut out, 70, "64");
        w(&mut out, 62, &layer_color(layer).to_string());
        w(&mut out, 6, "CONTINUOUS");
    }
    w(&mut out, 0, "ENDTAB");
    w(&mut out, 0, "ENDSEC");
    w(&mut out, 0, "SECTION");
    w(&mut out, 2, "ENTITIES");
    for chunk in e.chunks(2) {
        out.push_str(chunk[0].as_str());
        out.push('\n');
        out.push_str(chunk[1].as_str());
        out.push('\n');
    }
    w(&mut out, 0, "ENDSEC");
    w(&mut out, 0, "EOF");
    out
}

fn fmt(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".into()
    } else {
        s.to_string()
    }
}

// ------------------------------------------------------------- import -----

/// One polyline read from a DXF file (plan coordinates after scaling is left
/// to the caller — values are raw file units).
#[derive(Clone, Debug)]
pub struct DxfPoly {
    pub pts: Vec<Vec2>,
    pub closed: bool,
    pub layer: String,
}

#[derive(Clone, Debug, Default)]
pub struct DxfImport {
    pub polylines: Vec<DxfPoly>,
    pub lines: Vec<(Vec2, Vec2, String)>,
    pub circles: Vec<(Vec2, f64, String)>,
    pub texts: Vec<(Vec2, String)>,
    pub layers: Vec<String>,
    /// Non-fatal notes (unsupported constructs, blocks not expanded, …).
    pub warnings: Vec<String>,
    pub entity_count: usize,
}

/// Minimal ASCII DXF reader for plan sketches (see module docs).
pub fn import_dxf(text: &str) -> Result<DxfImport, String> {
    let mut pairs: Vec<(i32, &str)> = Vec::new();
    let mut lines = text.lines();
    while let Some(code_s) = lines.next() {
        let Some(val) = lines.next() else { break };
        let code: i32 = code_s.trim().parse().map_err(|_| format!("bad group code: {code_s:?}"))?;
        pairs.push((code, val.trim_end_matches('\r')));
    }
    if pairs.is_empty() {
        return Err("empty DXF".into());
    }

    let mut imp = DxfImport::default();
    let mut section = String::new();
    let mut i = 0usize;

    // LAYER table.
    while i < pairs.len() {
        let (code, val) = pairs[i];
        if code == 0 && val == "SECTION" {
            section = pairs.get(i + 1).map(|(_, v)| v.to_string()).unwrap_or_default();
            if section == "ENTITIES" {
                break;
            }
        }
        if code == 0 && val == "LAYER" && section == "TABLES" {
            let mut name = String::new();
            let mut j = i + 1;
            while j < pairs.len() && pairs[j].0 != 0 {
                if pairs[j].0 == 2 {
                    name = pairs[j].1.to_string();
                }
                j += 1;
            }
            if !name.is_empty() && !imp.layers.iter().any(|l| l == &name) {
                imp.layers.push(name);
            }
        }
        i += 1;
    }

    // ENTITIES section.
    let mut warned_blocks = false;
    while i < pairs.len() {
        let (code, val) = pairs[i];
        if code == 0 && val == "ENDSEC" {
            break;
        }
        if code != 0 {
            i += 1;
            continue;
        }
        let kind = val;
        match kind {
            "SECTION" => {
                // Entities-section header — nothing to import here.
                let mut j = i + 1;
                while j < pairs.len() && pairs[j].0 != 0 {
                    j += 1;
                }
                i = j;
            }
            "LINE" => {
                let mut p0 = Vec2::ZERO;
                let mut p1 = Vec2::ZERO;
                let mut layer = "0".into();
                let mut j = i + 1;
                while j < pairs.len() && pairs[j].0 != 0 {
                    match pairs[j].0 {
                        8 => layer = pairs[j].1.to_string(),
                        10 => p0.x = num(pairs[j].1),
                        20 => p0.y = num(pairs[j].1),
                        11 => p1.x = num(pairs[j].1),
                        21 => p1.y = num(pairs[j].1),
                        _ => {}
                    }
                    j += 1;
                }
                imp.lines.push((p0, p1, layer));
                imp.entity_count += 1;
                i = j;
            }
            "LWPOLYLINE" => {
                let mut pts: Vec<Vec2> = Vec::new();
                let mut closed = false;
                let mut layer = "0".into();
                let mut px = 0.0;
                let mut have_x = false;
                let mut j = i + 1;
                while j < pairs.len() && pairs[j].0 != 0 {
                    match pairs[j].0 {
                        8 => layer = pairs[j].1.to_string(),
                        70 => closed = num(pairs[j].1) as i64 & 1 != 0,
                        10 => {
                            px = num(pairs[j].1);
                            have_x = true;
                        }
                        20 if have_x => {
                            pts.push(Vec2::new(px, num(pairs[j].1)));
                            have_x = false;
                        }
                        20 => {}
                        _ => {}
                    }
                    j += 1;
                }
                if pts.len() >= 2 {
                    imp.polylines.push(DxfPoly { pts, closed, layer });
                    imp.entity_count += 1;
                }
                i = j;
            }
            "POLYLINE" => {
                let mut closed = false;
                let mut layer = "0".into();
                let mut pts: Vec<Vec2> = Vec::new();
                let mut j = i + 1;
                while j < pairs.len() && pairs[j].0 != 0 {
                    match pairs[j].0 {
                        8 => layer = pairs[j].1.to_string(),
                        70 => closed = num(pairs[j].1) as i64 & 1 != 0,
                        _ => {}
                    }
                    j += 1;
                }
                // Fold VERTEX lines in until SEQEND.
                while j < pairs.len() {
                    if pairs[j].0 == 0 && pairs[j].1 == "SEQEND" {
                        // SEQEND may carry attribute pairs — skip them too.
                        j += 1;
                        while j < pairs.len() && pairs[j].0 != 0 {
                            j += 1;
                        }
                        break;
                    }
                    if pairs[j].0 == 0 && pairs[j].1 == "VERTEX" {
                        let mut vx = 0.0;
                        let mut vy = 0.0;
                        let mut k = j + 1;
                        while k < pairs.len() && pairs[k].0 != 0 {
                            match pairs[k].0 {
                                10 => vx = num(pairs[k].1),
                                20 => vy = num(pairs[k].1),
                                _ => {}
                            }
                            k += 1;
                        }
                        pts.push(Vec2::new(vx, vy));
                        j = k;
                        continue;
                    }
                    if pairs[j].0 == 0 {
                        break;
                    }
                    j += 1;
                }
                if pts.len() >= 2 {
                    imp.polylines.push(DxfPoly { pts, closed, layer });
                    imp.entity_count += 1;
                }
                i = j;
            }
            "CIRCLE" => {
                let mut c = Vec2::ZERO;
                let mut r = 0.0;
                let mut layer = "0".into();
                let mut j = i + 1;
                while j < pairs.len() && pairs[j].0 != 0 {
                    match pairs[j].0 {
                        8 => layer = pairs[j].1.to_string(),
                        10 => c.x = num(pairs[j].1),
                        20 => c.y = num(pairs[j].1),
                        40 => r = num(pairs[j].1),
                        _ => {}
                    }
                    j += 1;
                }
                imp.circles.push((c, r, layer));
                imp.entity_count += 1;
                i = j;
            }
            "TEXT" | "MTEXT" => {
                let mut p = Vec2::ZERO;
                let mut content = String::new();
                let mut j = i + 1;
                while j < pairs.len() && pairs[j].0 != 0 {
                    match pairs[j].0 {
                        10 => p.x = num(pairs[j].1),
                        20 => p.y = num(pairs[j].1),
                        1 => content = pairs[j].1.to_string(),
                        _ => {}
                    }
                    j += 1;
                }
                if !content.is_empty() {
                    imp.texts.push((p, content));
                    imp.entity_count += 1;
                }
                i = j;
            }
            "INSERT" => {
                if !warned_blocks {
                    imp.warnings.push("block references (INSERT) are counted but not expanded — full block support is a P4 roadmap item".into());
                    warned_blocks = true;
                }
                imp.entity_count += 1;
                let mut j = i + 1;
                while j < pairs.len() && pairs[j].0 != 0 {
                    j += 1;
                }
                i = j;
            }
            other => {
                let known = ["POINT", "ARC", "ELLIPSE", "SPLINE", "HATCH", "3DFACE", "SOLID", "DIMENSION", "ATTRIB"];
                if !known.contains(&other) && !imp.warnings.iter().any(|w| w.contains(other)) {
                    imp.warnings.push(format!("entity {other} ignored"));
                }
                let mut j = i + 1;
                while j < pairs.len() && pairs[j].0 != 0 {
                    j += 1;
                }
                i = j;
            }
        }
    }
    Ok(imp)
}

fn num(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

/// Map an imported sketch onto the model: closed polylines become floor slabs,
/// open polylines and lines become wall centerlines. `scale` converts file
/// units to meters (e.g. 0.001 for millimetre plans). Elements land on the
/// first level.
pub fn import_into(model: &mut Model, imp: &DxfImport, scale: f64) -> (usize, usize) {
    let level = model.levels.first().map(|l| l.id).unwrap_or(0);
    let mut walls = 0usize;
    let mut slabs = 0usize;
    let put_wall = |m: &mut Model, level: Id, pts: Vec<Vec2>| -> bool {
        let pts: Vec<Vec2> = pts.iter().map(|p| *p * scale).collect();
        if pts.len() < 2 || (pts[0] - pts[pts.len() - 1]).length() < 1e-6 {
            return false;
        }
        m.add_wall(level, pts, 0.2, 3.0);
        true
    };
    for poly in &imp.polylines {
        if poly.closed && poly.pts.len() >= 3 {
            let pts: Vec<Vec2> = poly.pts.iter().map(|p| *p * scale).collect();
            if Model::poly_valid(&pts) {
                model.add_slab(level, pts, 0.25, 0.0);
                slabs += 1;
            }
        } else if put_wall(model, level, poly.pts.clone()) {
            walls += 1;
        }
    }
    for (a, b, _) in &imp.lines {
        if put_wall(model, level, vec![*a, *b]) {
            walls += 1;
        }
    }
    (walls, slabs)
}

/// Convenience: parse + map in one call.
pub fn import_dxf_into(model: &mut Model, text: &str, scale: f64) -> Result<(DxfImport, usize, usize), String> {
    let imp = import_dxf(text)?;
    let (w, s) = import_into(model, &imp, scale);
    Ok((imp, w, s))
}

/// Suggested name for the next section ("Section 1", …).
pub fn section_name(model: &Model) -> String {
    format!("Section {}", model.sections.len() + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElementKind, Opening};

    #[test]
    fn export_contains_sections_and_layers() {
        let m = crate::sample();
        let dxf = export_dxf(&m);
        assert!(dxf.contains("AC1009"));
        assert!(dxf.contains("BIM-WALLS"));
        assert!(dxf.contains("POLYLINE"));
        assert!(dxf.contains("SEQEND"));
        assert!(dxf.ends_with("0\nEOF\n") || dxf.contains("\nEOF\n"));
        // wall outline loops exist at floor + head height (z=0 and z=3)
        assert!(dxf.contains("\n30\n0\n") || dxf.contains("\n30\n3\n"));
    }

    #[test]
    fn dxf_roundtrip_walls_and_slabs() {
        let mut m = Model::new();
        let lvl = m.levels[0].id;
        m.add_wall(lvl, vec![Vec2::ZERO, Vec2::new(6.0, 0.0)], 0.2, 3.0);
        m.add_slab(lvl, Model::rect_poly(Vec2::ZERO, Vec2::new(6.0, 4.0)), 0.25, 0.0);
        let dxf = export_dxf(&m);
        let mut back = Model::new();
        let (imp, w, s) = import_dxf_into(&mut back, &dxf, 1.0).unwrap();
        assert!(w >= 1, "walls imported: {w} (import: {} polylines, {} lines)", imp.polylines.len(), imp.lines.len());
        assert!(s >= 1, "slabs imported: {s}");
        assert!(back.walls().count() >= 1);
    }

    #[test]
    fn import_handwritten_minimal_dxf() {
        let dxf = "0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n8\nPLAN\n70\n1\n10\n0.0\n20\n0.0\n10\n5000.0\n20\n0.0\n10\n5000.0\n20\n4000.0\n10\n0.0\n20\n4000.0\n0\nLINE\n8\nPLAN\n10\n0.0\n20\n0.0\n11\n3000.0\n21\n0.0\n0\nCIRCLE\n8\nX\n10\n1.0\n20\n1.0\n40\n0.5\n0\nTEXT\n8\nNOTES\n10\n0.0\n20\n-1.0\n40\n0.3\n1\nHello DXF\n0\nENDSEC\n0\nEOF\n";
        let imp = import_dxf(dxf).unwrap();
        assert_eq!(imp.polylines.len(), 1);
        assert!(imp.polylines[0].closed);
        assert_eq!(imp.lines.len(), 1);
        assert_eq!(imp.circles.len(), 1);
        assert_eq!(imp.texts.len(), 1);
        let mut m = Model::new();
        let (w, s) = import_into(&mut m, &imp, 0.001);
        assert_eq!((w, s), (1, 1));
        // mm → m: the slab is 5 m × 4 m
        let slab = m.elements.iter().find(|e| e.kind == ElementKind::Slab).unwrap();
        match &slab.data {
            ElementData::Slab(s) => assert!((s.area() - 20.0).abs() < 1e-6),
            _ => panic!(),
        }
    }

    #[test]
    fn export_import_has_no_parser_warnings() {
        let m = crate::sample();
        let dxf = export_dxf(&m);
        let imp = import_dxf(&dxf).unwrap();
        assert!(imp.warnings.is_empty(), "unexpected warnings: {:?}", imp.warnings);
        assert!(imp.entity_count > 100, "entities: {}", imp.entity_count);
        // Layers of our own export are preserved.
        assert!(imp.layers.iter().any(|l| l == "BIM-WALLS"));
    }

    #[test]
    fn import_rejects_garbage() {
        assert!(import_dxf("").is_err());
        assert!(import_dxf("0\nSECTION\n2\nENTITIES\nX\nY\n").is_err());
    }

    #[test]
    fn sections_and_rooms_in_schedule() {
        let mut m = crate::sample();
        let lvl = m.levels[0].id;
        m.add_section(lvl, "Section 1", Vec2::new(-1.0, 3.5), Vec2::new(10.0, 3.5), 4.0);
        let rows = m.schedule();
        assert!(rows.iter().any(|r| r.group == "Rooms" && r.count >= 2), "rooms in schedule: {rows:?}");
        assert_eq!(m.sections.len(), 1);
        let s = m.section(1).unwrap_or_else(|| m.sections.first().unwrap());
        assert!((s.length() - 11.0).abs() < 1e-9);
        // Sign convention: with d = a→b, points to the LEFT of the line are negative.
        assert!(s.signed_dist(Vec2::new(4.0, 0.0)) > 0.0);
        assert!(s.signed_dist(Vec2::new(4.0, 7.0)) < 0.0);
    }

    #[test]
    fn delete_level_cascades_sections_and_rooms() {
        let mut m = crate::sample();
        let l1 = m.levels[1].id;
        m.add_section(l1, "S", Vec2::ZERO, Vec2::new(1.0, 0.0), 2.0);
        m.add_room(l1, "Bedroom", Vec2::new(2.0, 2.0));
        let n_elements = m.elements.len();
        m.delete_level(l1);
        assert!(m.sections.is_empty());
        assert!(m.rooms.is_empty());
        assert!(m.elements.len() < n_elements);
    }

    #[test]
    fn opening_rect_for_export() {
        let mut m = Model::new();
        let lvl = m.levels[0].id;
        let w = m.add_wall(lvl, vec![Vec2::ZERO, Vec2::new(6.0, 0.0)], 0.2, 3.0);
        m.add_opening(ElementKind::Window, Opening { wall: w, s: 3.0, width: 1.2, height: 1.4, sill: 0.9 });
        let o = m.elements.iter().find(|e| e.kind == ElementKind::Window).unwrap();
        match &o.data {
            ElementData::Opening(op) => {
                let q = m.opening_rect(op).unwrap();
                assert!((q[0].z - 0.9).abs() < 1e-9 && (q[2].z - 2.3).abs() < 1e-9);
            }
            _ => panic!(),
        }
    }
}
