//! Tool implementations: every `tools/call` lands here.
//!
//! Conventions (parity with the cadcraft dispatcher):
//! * every mutating tool takes exactly one undo snapshot ([Session::begin_edit])
//!   before it touches the model, so one tool call = one undo step;
//! * edits go through [Session] methods where their semantics match, otherwise
//!   they mirror them (snapshot → `Model::add_*` → derived re-derivation →
//!   selection + status);
//! * failures are `Err(String)` and surface as `isError: true` MCP content, so
//!   an agent can read the message and self-correct.

use bimcraft_bim::{brick_type, opening_type, Element, ElementData, ElementKind, ItemKind, Model, Opening, SiteElement, SiteShape};
use bimcraft_engine::{Selection, Session};
use bimcraft_geom::{poly_area, Vec2};
use serde_json::{json, Value};
use std::path::Path;

/// Dispatch a tool call. `root` is the resolved `--root` folder (path
/// restriction for save/load/export), `None` when unrestricted.
pub fn run_tool(session: &mut Session, root: Option<&Path>, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        // ------------------------------------------------------------- read --
        "model_describe" => Ok(model_describe(session)),
        "elements_list" => elements_list(session, args),
        "element_get" => element_get(session, args),
        "site_list" => Ok(site_list(session)),
        "foundation_list" => Ok(foundation_list(session)),
        // ------------------------------------------------------------- edit --
        "level_add" => level_add(session, args),
        "wall_add" => wall_add(session, args),
        "opening_add" => opening_add(session, args),
        "slab_add" => slab_add(session, args),
        "item_place" => item_place(session, args),
        "site_object_add" => site_object_add(session, args),
        "corner_detail" => corner_detail(session, args),
        "section_add" => section_add(session, args),
        "undo" => {
            session.undo();
            Ok(json!({ "applied": session.status == "Undo", "status": session.status }))
        }
        "redo" => {
            session.redo();
            Ok(json!({ "applied": session.status == "Redo", "status": session.status }))
        }
        // ------------------------------------------------------------- save --
        "model_save" => model_save(session, root, args),
        "model_load" => model_load(session, root, args),
        "model_new" => {
            let mut model = Model::new();
            // Fresh models follow the documented default (serde default is
            // "on"; only the in-memory derive quirk says otherwise).
            model.auto_foundation = true;
            *session = Session::with_model(model);
            session.status = "New model".into();
            Ok(json!({ "levels": session.model.levels.len(), "grids": session.model.grids.len(), "elements": session.model.elements.len() }))
        }
        // ----------------------------------------------------------- export --
        "dxf_export" => dxf_export(session, root, args),
        other => Err(format!("unknown tool '{other}'")),
    }
}

// ---------------------------------------------------------------- helpers --

fn arg<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.as_object().and_then(|o| o.get(key))
}

fn is_set(args: &Value, key: &str) -> bool {
    match arg(args, key) {
        None => false,
        Some(Value::Null) => false,
        Some(_) => true,
    }
}

fn opt_f64(args: &Value, key: &str) -> Result<Option<f64>, String> {
    match arg(args, key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().map(Some).ok_or_else(|| format!("'{key}' must be a number")),
    }
}

fn req_f64(args: &Value, key: &str) -> Result<f64, String> {
    opt_f64(args, key)?.ok_or_else(|| format!("'{key}' is required (number)"))
}

fn opt_u64(args: &Value, key: &str) -> Result<Option<u64>, String> {
    match arg(args, key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_u64().map(Some).ok_or_else(|| format!("'{key}' must be a non-negative integer")),
    }
}

fn req_u64(args: &Value, key: &str) -> Result<u64, String> {
    opt_u64(args, key)?.ok_or_else(|| format!("'{key}' is required (integer id)"))
}

fn opt_str(args: &Value, key: &str) -> Result<Option<String>, String> {
    match arg(args, key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("'{key}' must be a string")),
    }
}

fn req_str(args: &Value, key: &str) -> Result<String, String> {
    opt_str(args, key)?.ok_or_else(|| format!("'{key}' is required (string)"))
}

fn opt_bool(args: &Value, key: &str) -> Result<Option<bool>, String> {
    match arg(args, key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_bool().map(Some).ok_or_else(|| format!("'{key}' must be a boolean")),
    }
}

/// Parse a `[x, y]` pair (objects `{x, y}` are accepted as a convenience).
fn read_point(v: &Value, ctx: &str) -> Result<Vec2, String> {
    let (x, y) = match v {
        Value::Array(a) if a.len() == 2 => {
            let x = a[0].as_f64().ok_or_else(|| format!("{ctx}: coordinates must be numbers"))?;
            let y = a[1].as_f64().ok_or_else(|| format!("{ctx}: coordinates must be numbers"))?;
            (x, y)
        }
        Value::Object(o) => {
            let x = o.get("x").and_then(Value::as_f64).ok_or_else(|| format!("{ctx}: expected [x, y]"))?;
            let y = o.get("y").and_then(Value::as_f64).ok_or_else(|| format!("{ctx}: expected [x, y]"))?;
            (x, y)
        }
        _ => return Err(format!("{ctx}: expected an [x, y] pair")),
    };
    if !x.is_finite() || !y.is_finite() {
        return Err(format!("{ctx}: coordinates must be finite numbers"));
    }
    Ok(Vec2::new(x, y))
}

fn opt_points(args: &Value, key: &str) -> Result<Option<Vec<Vec2>>, String> {
    match arg(args, key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => {
            let mut pts = Vec::with_capacity(items.len());
            for it in items {
                pts.push(read_point(it, &format!("'{key}'"))?);
            }
            Ok(Some(pts))
        }
        Some(_) => Err(format!("'{key}' must be an array of [x, y] pairs")),
    }
}

fn req_points(args: &Value, key: &str) -> Result<Vec<Vec2>, String> {
    opt_points(args, key)?.ok_or_else(|| format!("'{key}' is required (array of [x, y] pairs)"))
}

fn req_point2(args: &Value, key: &str) -> Result<Vec2, String> {
    let v = arg(args, key).ok_or_else(|| format!("'{key}' is required ([x, y])"))?;
    read_point(v, &format!("'{key}'"))
}

/// Lowercase alphanumerics only, so "Kitchen unit", "KitchenUnit" and
/// "kitchen_unit" all match the same preset.
fn normalize(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

/// Snake-ish key for a preset display name ("Deciduous tree 6 m" →
/// "deciduous_tree_6_m").
fn kind_key(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn parse_element_kind(s: &str) -> Option<ElementKind> {
    match normalize(s).as_str() {
        "wall" => Some(ElementKind::Wall),
        "door" => Some(ElementKind::Door),
        "window" => Some(ElementKind::Window),
        "slab" | "floor" => Some(ElementKind::Slab),
        "ceiling" => Some(ElementKind::Ceiling),
        "item" | "furniture" => Some(ElementKind::Item),
        _ => None,
    }
}

fn parse_item_kind(s: &str) -> Option<ItemKind> {
    let n = normalize(s);
    ItemKind::ALL.into_iter().find(|k| normalize(k.name()) == n)
}

fn parse_site_kind(s: &str) -> Option<bimcraft_bim::SiteKind> {
    let n = normalize(s);
    if n.is_empty() {
        return None;
    }
    // Accept the display name ("Fence run"), the snake key ("fence_run") and
    // the enum-style name ("TreeDeciduous6"); then a key-prefix shorthand
    // ("Fence" → fence_run, "hedge" → hedge_run).
    bimcraft_bim::SiteKind::ALL
        .into_iter()
        .find(|k| {
            let variant = normalize(format!("{k:?}").as_str());
            normalize(k.name()) == n || kind_key(k.name()) == n || variant == n
        })
        .or_else(|| bimcraft_bim::SiteKind::ALL.into_iter().find(|k| kind_key(k.name()).starts_with(&n)))
}

fn item_kind_list() -> String {
    ItemKind::ALL.iter().map(|k| k.name()).collect::<Vec<_>>().join(", ")
}

fn site_kind_list() -> String {
    bimcraft_bim::SiteKind::ALL.iter().map(|k| kind_key(k.name())).collect::<Vec<_>>().join(", ")
}

/// The level an element belongs to (openings inherit their host wall's level).
fn element_level(m: &Model, e: &Element) -> Option<u64> {
    match &e.data {
        ElementData::Opening(o) => m.wall(o.wall).map(|w| w.level),
        ElementData::Wall(w) => Some(w.level),
        ElementData::Slab(s) => Some(s.level),
        ElementData::Ceiling(c) => Some(c.level),
        ElementData::Item(i) => Some(i.level),
    }
}

fn level_name(m: &Model, id: Option<u64>) -> Value {
    match id.and_then(|id| m.level(id)) {
        Some(l) => json!(l.name),
        None => Value::Null,
    }
}

fn element_summary(e: &Element) -> String {
    match &e.data {
        ElementData::Wall(w) => {
            let finish = if w.finish.is_empty() { String::new() } else { format!(", finish {}", w.finish) };
            format!("len {:.2} m × t {:.2} m × h {:.2} m{}", w.total_length(), w.thickness, w.height, finish)
        }
        ElementData::Opening(o) => {
            let kind = if o.type_key.is_empty() { String::new() } else { format!(" ({})", o.type_key) };
            format!("{:.2} × {:.2} m @ s {:.2} m{}", o.width, o.height, o.s, kind)
        }
        ElementData::Slab(s) => format!("area {:.2} m², t {:.2} m, {}", s.area(), s.thickness, s.kind),
        ElementData::Ceiling(c) => format!("area {:.2} m², t {:.2} m", c.area(), c.thickness),
        ElementData::Item(i) => format!("{} at ({:.2}, {:.2}), rot {:.0}°", i.kind.name(), i.pos.x, i.pos.y, i.rotation.to_degrees()),
    }
}

/// Resolve a level reference: absent/null → active level; number → id;
/// string → id-then-name (name match falls back to case-insensitive).
fn resolve_level(session: &Session, spec: Option<&Value>) -> Result<u64, String> {
    let Some(spec) = spec else { return active_level(session) };
    match spec {
        Value::Null => active_level(session),
        Value::Number(_) => {
            let id = spec.as_u64().ok_or("'level' must be a level id (unsigned integer) or name")?;
            session.model.level(id).map(|_| id).ok_or_else(|| format!("no level with id {id}"))
        }
        Value::String(s) => {
            if let Ok(id) = s.parse::<u64>() {
                if session.model.level(id).is_some() {
                    return Ok(id);
                }
            }
            session
                .model
                .levels
                .iter()
                .find(|l| l.name == *s || l.name.eq_ignore_ascii_case(s))
                .map(|l| l.id)
                .ok_or_else(|| format!("no level named '{s}'"))
        }
        _ => Err("'level' must be a level id (number) or name (string)".into()),
    }
}

fn active_level(session: &Session) -> Result<u64, String> {
    if session.model.levels.is_empty() {
        return Err("model has no levels — add one with level_add first".into());
    }
    Ok(session.active_level_id())
}

fn finite(v: f64, what: &str) -> Result<f64, String> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(format!("'{what}' must be a finite number"))
    }
}

fn positive(v: f64, what: &str) -> Result<f64, String> {
    let v = finite(v, what)?;
    if v > 0.0 {
        Ok(v)
    } else {
        Err(format!("'{what}' must be > 0"))
    }
}

// ------------------------------------------------------------------ read --

fn model_describe(session: &Session) -> Value {
    let m = &session.model;
    let count = |k: ElementKind| m.elements.iter().filter(|e| e.kind == k).count();
    let counts = json!({
        "walls": count(ElementKind::Wall),
        "doors": count(ElementKind::Door),
        "windows": count(ElementKind::Window),
        "slabs": count(ElementKind::Slab),
        "ceilings": count(ElementKind::Ceiling),
        "items": count(ElementKind::Item),
        "rooms": m.rooms.len(),
        "sections": m.sections.len(),
        "grids": m.grids.len(),
        "foundations": m.foundations.len(),
        "site_objects": m.site.len(),
    });
    let levels: Vec<Value> = m
        .levels
        .iter()
        .map(|l| json!({ "id": l.id, "name": l.name, "elevation": l.elevation, "height": l.height, "grid_shift": l.grid_shift }))
        .collect();
    let bounds = m.bounds().map(|(min, max)| {
        json!({
            "min": { "x": min.x, "y": min.y, "z": min.z },
            "max": { "x": max.x, "y": max.y, "z": max.z }
        })
    });
    json!({
        "units": "meters",
        "axis": "z-up",
        "levels": levels,
        "counts": counts,
        "foundations": m.foundations.len(),
        "site_objects": m.site.len(),
        "bounds": bounds,
    })
}

fn elements_list(session: &Session, args: &Value) -> Result<Value, String> {
    let kind_filter = match opt_str(args, "kind")? {
        Some(s) => Some(parse_element_kind(&s).ok_or_else(|| format!("unknown kind '{s}' (expected Wall|Door|Window|Slab|Ceiling|Item)"))?),
        None => None,
    };
    let level_filter = if is_set(args, "level") { Some(resolve_level(session, arg(args, "level"))?) } else { None };
    let offset = opt_u64(args, "offset")?.unwrap_or(0) as usize;
    let limit = opt_u64(args, "limit")?.unwrap_or(100).clamp(1, 1000) as usize;
    let m = &session.model;
    let mut rows = Vec::new();
    for e in &m.elements {
        if let Some(k) = kind_filter {
            if e.kind != k {
                continue;
            }
        }
        if let Some(lvl) = level_filter {
            if element_level(m, e) != Some(lvl) {
                continue;
            }
        }
        rows.push(json!({
            "id": e.id,
            "name": e.name,
            "kind": format!("{:?}", e.kind),
            "level": level_name(m, element_level(m, e)),
            "summary": element_summary(e),
        }));
    }
    let total = rows.len();
    let page: Vec<Value> = rows.into_iter().skip(offset).take(limit).collect();
    Ok(json!({ "total": total, "offset": offset, "limit": limit, "elements": page }))
}

fn element_get(session: &Session, args: &Value) -> Result<Value, String> {
    let id = req_u64(args, "id")?;
    let e = session.model.element(id).ok_or_else(|| format!("no element with id {id}"))?;
    serde_json::to_value(e).map_err(|err| format!("cannot serialize element: {err}"))
}

fn site_list(session: &Session) -> Value {
    let rows: Vec<Value> = session
        .model
        .site
        .iter()
        .map(|s| {
            let shape = match s.kind.shape() {
                SiteShape::Point => "point",
                SiteShape::Chain => "chain",
                SiteShape::Polygon => "polygon",
            };
            let mut row = json!({
                "id": s.id,
                "kind": s.kind.name(),
                "key": kind_key(s.kind.name()),
                "shape": shape,
                "pos": { "x": s.pos.x, "y": s.pos.y },
                "pts": s.pts.iter().map(|p| json!([p.x, p.y])).collect::<Vec<_>>(),
                "rotation_deg": s.rotation.to_degrees(),
            });
            if s.kind.shape() == SiteShape::Chain {
                row["length_m"] = json!(s.chain_length());
            }
            if s.kind.shape() == SiteShape::Polygon {
                row["area_m2"] = json!(s.area());
            }
            row
        })
        .collect();
    let count = rows.len();
    json!({ "count": count, "site_objects": rows })
}

fn foundation_list(session: &Session) -> Value {
    use bimcraft_bim::foundation::{foundation_concrete_volume, foundation_geometry};
    use bimcraft_bim::rebar::footing_rebar;
    let m = &session.model;
    let rows: Vec<Value> = m
        .foundations
        .iter()
        .map(|f| {
            let mut row = json!({
                "id": f.id,
                "name": f.name,
                "kind": f.kind.label(),
                "key": f.kind.key(),
                "level": level_name(m, Some(f.level)),
                "seed": { "x": f.seed.x, "y": f.seed.y },
            });
            if let Some((p, outline)) = m.foundation_resolved(f) {
                let parts = foundation_geometry(&outline, &p);
                let floor_z = m.level(f.level).map(|l| l.elevation).unwrap_or(0.0);
                let rebar = footing_rebar(&parts, &p, &f.rebar, floor_z);
                row["resolved"] = json!(true);
                row["sizes_m"] = json!({
                    "slab_t": p.slab_t, "edge_w": p.edge_w, "edge_d": p.edge_d, "stem_t": p.stem_t,
                    "foot_w": p.foot_w, "foot_d": p.foot_d, "bottom": p.bottom,
                    "stem_drop": p.stem_drop, "basement_h": p.basement_h,
                });
                row["outline_area_m2"] = json!(poly_area(&outline).abs());
                row["concrete_volume_m3"] = json!(foundation_concrete_volume(&parts));
                row["rebar"] = json!({
                    "bars": rebar.bars.len(),
                    "mass_kg": rebar.total_mass(),
                    "length_m": rebar.total_length(),
                });
            } else {
                row["resolved"] = json!(false);
                row["note"] = json!("seed sits outside every building outline on its level");
            }
            row
        })
        .collect();
    let count = rows.len();
    json!({ "count": count, "foundations": rows })
}

// ------------------------------------------------------------------ edit --

fn level_add(session: &mut Session, args: &Value) -> Result<Value, String> {
    let elevation = finite(req_f64(args, "elevation")?, "elevation")?;
    let height = positive(req_f64(args, "height")?, "height")?;
    let name = match opt_str(args, "name")? {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => format!("Level {}", session.model.levels.len()),
    };
    session.begin_edit();
    let id = session.model.add_level(&name, elevation, height);
    session.set_active_level(id);
    session.status = format!("{name} added");
    Ok(json!({ "id": id, "name": name, "elevation": elevation, "height": height }))
}

fn wall_add(session: &mut Session, args: &Value) -> Result<Value, String> {
    let pts = req_points(args, "points")?;
    if pts.len() < 2 {
        return Err("'points' needs at least 2 [x, y] vertices".into());
    }
    let lvl = resolve_level(session, arg(args, "level"))?;
    let level_height = session.model.level(lvl).map(|l| l.height.max(0.5)).unwrap_or(3.0);
    let thickness = positive(opt_f64(args, "thickness")?.unwrap_or(0.2), "thickness")?;
    let height = positive(opt_f64(args, "height")?.unwrap_or(level_height), "height")?;
    let brick = match opt_str(args, "brick")? {
        Some(b) if !b.is_empty() => {
            if brick_type(&b).is_none() {
                return Err(format!("unknown brick finish '{b}'"));
            }
            Some(b)
        }
        _ => None,
    };
    // Session::add_wall semantics: snapshot → Model::add_wall → finish →
    // ensure_foundations → select + status.
    session.begin_edit();
    let id = session.model.add_wall(lvl, pts, thickness, height);
    if let Some(b) = brick {
        if let Some(e) = session.model.element_mut(id) {
            if let ElementData::Wall(w) = &mut e.data {
                w.finish = b;
            }
        }
    }
    let created = session.model.ensure_foundations();
    session.select(Selection::Element(id));
    session.status =
        if created.is_empty() { format!("Wall {id} added") } else { format!("Wall {id} added — {} foundation auto-placed", created.len()) };
    Ok(json!({ "id": id, "level": lvl, "foundations_added": created }))
}

fn opening_add(session: &mut Session, args: &Value) -> Result<Value, String> {
    let kind = match normalize(&req_str(args, "kind")?).as_str() {
        "door" => ElementKind::Door,
        "window" => ElementKind::Window,
        other => return Err(format!("'kind' must be \"door\" or \"window\" (got '{other}')")),
    };
    let wall_id = req_u64(args, "wall")?;
    let wall = session.model.wall(wall_id).cloned().ok_or_else(|| format!("no wall with id {wall_id}"))?;
    let len = wall.total_length();
    let s = finite(req_f64(args, "s")?, "s")?;
    let type_key = opt_str(args, "type_key")?;
    let preset = type_key.as_deref().and_then(opening_type);
    let (d_width, d_height, d_sill, d_key) = match kind {
        ElementKind::Door => (0.9, 2.1, 0.0, "door-single"),
        _ => (1.2, 1.4, 0.9, "win-single"),
    };
    let (p_width, p_height) = preset.map(|t| (t.width, t.height)).unwrap_or((d_width, d_height));
    let width = positive(opt_f64(args, "width")?.unwrap_or(p_width), "width")?;
    let height = positive(opt_f64(args, "height")?.unwrap_or(p_height), "height")?;
    let sill = finite(opt_f64(args, "sill")?.unwrap_or(d_sill), "sill")?;
    let type_key = type_key.unwrap_or_else(|| d_key.to_string());
    // Clamp into the wall body with the same margin the drag path uses.
    let margin = (width / 2.0 + 0.05).min(len / 2.0);
    let s = s.clamp(margin, (len - margin).max(margin));
    session.begin_edit();
    let id = session.model.add_opening(kind, Opening { wall: wall_id, s, width, height, sill, type_key, hand: 'a', face: 'a' });
    session.select(Selection::Element(id));
    session.status = format!("{} {id} placed", if kind == ElementKind::Door { "Door" } else { "Window" });
    Ok(json!({ "id": id, "wall": wall_id, "s": s, "width": width, "height": height, "sill": sill }))
}

fn slab_add(session: &mut Session, args: &Value) -> Result<Value, String> {
    let pts = req_points(args, "points")?;
    if pts.len() < 3 || !Model::poly_valid(&pts) {
        return Err("'points' must be a valid polygon (at least 3 distinct points)".into());
    }
    let lvl = resolve_level(session, arg(args, "level"))?;
    let thickness = positive(opt_f64(args, "thickness")?.unwrap_or(0.25), "thickness")?;
    let elevation = finite(opt_f64(args, "elevation")?.unwrap_or(0.0), "elevation")?;
    session.begin_edit();
    let id = session.model.add_slab(lvl, pts, thickness, elevation);
    session.model.ensure_foundations();
    session.select(Selection::Element(id));
    session.status = format!("Floor {id} added");
    Ok(json!({ "id": id, "level": lvl, "area_m2": session.model.element(id).map(|e| match &e.data {
        ElementData::Slab(s) => s.area(),
        _ => 0.0,
    }).unwrap_or(0.0) }))
}

fn item_place(session: &mut Session, args: &Value) -> Result<Value, String> {
    let kind_s = req_str(args, "kind")?;
    let kind = parse_item_kind(&kind_s).ok_or_else(|| format!("unknown item kind '{kind_s}' (expected one of: {})", item_kind_list()))?;
    let x = finite(req_f64(args, "x")?, "x")?;
    let y = finite(req_f64(args, "y")?, "y")?;
    let lvl = resolve_level(session, arg(args, "level"))?;
    let rot = finite(opt_f64(args, "rotation_deg")?.unwrap_or(0.0), "rotation_deg")?.to_radians();
    session.set_active_level(lvl);
    session.item_kind = kind;
    session.item_rotation = rot;
    session.place_item(Vec2::new(x, y));
    let id = match session.selection {
        Selection::Element(id) => id,
        _ => return Err("item placement failed".into()),
    };
    let (w, d, h) = kind.preset();
    Ok(json!({ "id": id, "kind": kind.name(), "level": lvl, "size_m": { "width": w, "depth": d, "height": h } }))
}

fn site_object_add(session: &mut Session, args: &Value) -> Result<Value, String> {
    let kind_s = req_str(args, "kind")?;
    let kind = parse_site_kind(&kind_s).ok_or_else(|| format!("unknown site kind '{kind_s}' (expected one of: {})", site_kind_list()))?;
    match kind.shape() {
        SiteShape::Point => {
            let x = finite(req_f64(args, "x")?, "x")?;
            let y = finite(req_f64(args, "y")?, "y")?;
            let rot = finite(opt_f64(args, "rotation_deg")?.unwrap_or(0.0), "rotation_deg")?.to_radians();
            session.site_kind = kind;
            session.item_rotation = rot;
            session.place_site(Vec2::new(x, y));
            let id = match session.selection {
                Selection::Site(id) => id,
                _ => return Err("site placement failed".into()),
            };
            Ok(json!({ "id": id, "kind": kind.name(), "key": kind_key(kind.name()) }))
        }
        SiteShape::Chain | SiteShape::Polygon => {
            let pts = req_points(args, "points")?;
            let (ok, need, what) = match kind.shape() {
                SiteShape::Chain => (pts.len() >= 2, "at least 2 points", "a chain"),
                _ => (Model::poly_valid(&pts), "a valid closed polygon (3+ distinct points)", "a closed polygon"),
            };
            if !ok {
                return Err(format!("'{kind_s}' is drawn as {what}: 'points' needs {need}"));
            }
            let stats = match kind.shape() {
                SiteShape::Chain => json!({ "length_m": pts.windows(2).map(|w| (w[1] - w[0]).length()).sum::<f64>() }),
                _ => json!({ "area_m2": poly_area(&pts).abs() }),
            };
            // Mirror Session::finish_site_chain: centroid pos, derived variant.
            let pos = pts.iter().fold(Vec2::ZERO, |a, p| a + *p) / pts.len() as f64;
            session.begin_edit();
            let id = session.model.add_site_element(SiteElement {
                id: 0,
                kind,
                pos,
                pts,
                rotation: 0.0,
                variant: (session.model.site.len() as u32) * 7 + 1,
            });
            session.select(Selection::Site(id));
            session.status = format!("{} added", kind.name());
            let mut out = json!({ "id": id, "kind": kind.name(), "key": kind_key(kind.name()) });
            if let (Some(a), Some(b)) = (out.as_object_mut(), stats.as_object()) {
                for (k, v) in b {
                    a.insert(k.clone(), v.clone());
                }
            }
            Ok(out)
        }
    }
}

fn corner_detail(session: &mut Session, args: &Value) -> Result<Value, String> {
    let x = finite(req_f64(args, "x")?, "x")?;
    let y = finite(req_f64(args, "y")?, "y")?;
    let post = finite(opt_f64(args, "post")?.unwrap_or(0.0), "post")?;
    if post < 0.0 {
        return Err("'post' must be >= 0 (0 = formwork default)".into());
    }
    let starters = opt_bool(args, "starters")?.unwrap_or(true);
    let note = opt_str(args, "note")?.unwrap_or_default();
    session.begin_edit();
    let id = session.model.set_corner_detail(Vec2::new(x, y), post, starters, &note);
    session.status = format!("Corner {id} detailed");
    Ok(json!({ "id": id, "at": { "x": x, "y": y }, "post": post, "starters": starters }))
}

fn section_add(session: &mut Session, args: &Value) -> Result<Value, String> {
    let a = req_point2(args, "a")?;
    let b = req_point2(args, "b")?;
    if a.dist(b) < 0.5 {
        return Err("section line too short (needs >= 0.5 m)".into());
    }
    let lvl = resolve_level(session, arg(args, "level"))?;
    let depth = positive(opt_f64(args, "depth")?.unwrap_or(4.0), "depth")?;
    session.begin_edit();
    let name = bimcraft_bim::dxf::section_name(&session.model);
    let id = session.model.add_section(lvl, &name, a, b, depth);
    session.select(Selection::Section(id));
    session.active_section = Some(id);
    session.status = format!("{name} created");
    Ok(json!({ "id": id, "name": name, "level": lvl, "length_m": a.dist(b), "depth": depth }))
}

// ------------------------------------------------------------------ save --

fn resolve_path(root: Option<&Path>, raw: &str) -> Result<std::path::PathBuf, String> {
    if raw.trim().is_empty() {
        return Err("'path' must not be empty".into());
    }
    let Some(root) = root else {
        return Ok(Path::new(raw).to_path_buf());
    };
    let p = Path::new(raw);
    let abs = if p.is_absolute() { p.to_path_buf() } else { root.join(p) };
    // Canonicalize the deepest existing ancestor, then re-append the missing
    // tail: catches `..` and symlink escapes even for not-yet-written files.
    let mut anc: &Path = abs.as_path();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    while !anc.exists() {
        match (anc.file_name(), anc.parent()) {
            (Some(name), Some(parent)) => {
                tail.push(name.to_os_string());
                anc = parent;
            }
            _ => return Err(format!("cannot resolve path {}", abs.display())),
        }
    }
    let canon = anc.canonicalize().map_err(|e| format!("cannot resolve {}: {e}", abs.display()))?;
    if !canon.starts_with(root) {
        return Err(format!("path '{}' is outside --root {}", raw, root.display()));
    }
    let mut out = canon;
    for part in tail.iter().rev() {
        out.push(part);
    }
    Ok(out)
}

fn model_save(session: &mut Session, root: Option<&Path>, args: &Value) -> Result<Value, String> {
    let raw = req_str(args, "path")?;
    let path = resolve_path(root, &raw)?;
    let text = bimcraft_io::save_json(&session.model);
    std::fs::write(&path, &text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(json!({ "saved": path.display().to_string(), "bytes": text.len(), "elements": session.model.elements.len() }))
}

fn model_load(session: &mut Session, root: Option<&Path>, args: &Value) -> Result<Value, String> {
    let raw = req_str(args, "path")?;
    let path = resolve_path(root, &raw)?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let model = bimcraft_io::load_json(&text).map_err(|e| format!("{raw}: {e}"))?;
    let levels = model.levels.len();
    let elements = model.elements.len();
    *session = Session::with_model(model);
    session.status = format!("Loaded {raw}");
    Ok(json!({ "loaded": path.display().to_string(), "levels": levels, "elements": elements }))
}

fn dxf_export(session: &mut Session, root: Option<&Path>, args: &Value) -> Result<Value, String> {
    let raw = req_str(args, "path")?;
    let path = resolve_path(root, &raw)?;
    let text = bimcraft_io::export_dxf(&session.model);
    std::fs::write(&path, &text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(json!({ "exported": path.display().to_string(), "bytes": text.len() }))
}
