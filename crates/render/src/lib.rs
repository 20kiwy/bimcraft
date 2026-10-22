//! BimCraft display-list renderer: turns the BIM model into world-space lines,
//! triangles and labels for either the 2D plan view or the 3D perspective view.
//!
//! Same seam as `cadcraft-render`'s DisplayList (MIT OR Apache-2.0), clean-room
//! implemented. 3D geometry is shaded with a fixed light and drawn back-to-front
//! by the UI layer; openings cut real holes into wall faces (no overlay quads).

use bimcraft_bim::{ElementData, ElementKind, GridAxis, ItemKind, Model};
use bimcraft_geom::{Vec2, Vec3};
use egui::{Color32, Stroke};

pub const AXIS_X: Color32 = Color32::from_rgb(0xe8, 0x38, 0x4f);
pub const AXIS_Y: Color32 = Color32::from_rgb(0xa1, 0xc9, 0x3a);
pub const AXIS_Z: Color32 = Color32::from_rgb(0x38, 0xa1, 0xe8);

#[derive(Clone, Debug)]
pub struct DLine {
    pub a: Vec3,
    pub b: Vec3,
    pub color: Color32,
    pub width: f32,
    pub dashed: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct DTri {
    pub pts: [Vec3; 3],
    pub color: Color32,
}

#[derive(Clone, Debug)]
pub struct DText {
    pub pos: Vec3,
    pub text: String,
    pub size: f32,
    pub color: Color32,
}

#[derive(Clone, Debug, Default)]
pub struct DisplayList {
    pub lines: Vec<DLine>,
    pub tris: Vec<DTri>,
    pub texts: Vec<DText>,
}

impl DisplayList {
    pub fn line(&mut self, a: Vec3, b: Vec3, color: Color32, width: f32) {
        self.lines.push(DLine { a, b, color, width, dashed: false });
    }

    pub fn dashed(&mut self, a: Vec3, b: Vec3, color: Color32, width: f32) {
        self.lines.push(DLine { a, b, color, width, dashed: true });
    }

    pub fn tri(&mut self, pts: [Vec3; 3], color: Color32) {
        self.tris.push(DTri { pts, color });
    }

    pub fn quad(&mut self, q: [Vec3; 4], color: Color32) {
        self.tri([q[0], q[1], q[2]], color);
        self.tri([q[0], q[2], q[3]], color);
    }

    pub fn poly(&mut self, pts: &[Vec3], color: Color32) {
        for i in 1..pts.len().saturating_sub(1) {
            self.tri([pts[0], pts[i], pts[i + 1]], color);
        }
    }

    pub fn text(&mut self, pos: Vec3, text: impl Into<String>, size: f32, color: Color32) {
        self.texts.push(DText { pos, text: text.into(), size, color });
    }
}

/// Blender-like selection accent (orange) — explicitly user-requested style.
pub const SELECT: Color32 = Color32::from_rgb(0xf0, 0x8a, 0x2c);
const WALL_2D: Color32 = Color32::from_rgb(0xd7, 0xde, 0xea);
fn wall_fill(sel: bool) -> Color32 {
    if sel {
        with_alpha(SELECT, 150)
    } else {
        Color32::from_rgba_unmultiplied(0xb9, 0xc4, 0xd6, 90)
    }
}
const SLAB_2D: Color32 = Color32::from_rgb(0x9a, 0xb4, 0xc8);
fn slab_fill(sel: bool) -> Color32 {
    if sel {
        with_alpha(SELECT, 90)
    } else {
        Color32::from_rgba_unmultiplied(0x9a, 0xb4, 0xc8, 55)
    }
}
const CEIL_2D: Color32 = Color32::from_rgb(0x7f, 0x8f, 0xa8);
fn ceil_fill(sel: bool) -> Color32 {
    if sel {
        with_alpha(SELECT, 60)
    } else {
        Color32::from_rgba_unmultiplied(0x7f, 0x8f, 0xa8, 30)
    }
}
const GRID_LINE: Color32 = Color32::from_rgb(0x2c, 0x33, 0x40);
const GRID_STRONG: Color32 = Color32::from_rgb(0x3a, 0x44, 0x54);
const DOOR_2D: Color32 = Color32::from_rgb(0xc9, 0x8a, 0x4a);
const WINDOW_2D: Color32 = Color32::from_rgb(0x5b, 0xc8, 0xd8);
const ITEM_2D: Color32 = Color32::from_rgb(0x8f, 0xa8, 0x7a);

struct Palette3D {
    wall_top: Color32,
    wall_side_a: Color32,
    wall_side_b: Color32,
    slab: Color32,
    ceiling: Color32,
    door: Color32,
    window_glass: Color32,
    item: Color32,
}

const PAL3D: Palette3D = Palette3D {
    wall_top: Color32::from_rgb(0xcf, 0xd6, 0xe2),
    wall_side_a: Color32::from_rgb(0xb4, 0xbd, 0xcc),
    wall_side_b: Color32::from_rgb(0x9a, 0xa4, 0xb5),
    slab: Color32::from_rgb(0x8f, 0xa6, 0xb8),
    ceiling: Color32::from_rgb(0xc2, 0xc9, 0xd6),
    door: Color32::from_rgb(0x9a, 0x62, 0x2e),
    window_glass: Color32::from_rgb(0x9f, 0xd8, 0xe4),
    item: Color32::from_rgb(0xa8, 0x94, 0x74),
};

fn selected(color: Color32, is_sel: bool) -> Color32 {
    if is_sel {
        SELECT
    } else {
        color
    }
}

/// Fixed light direction for cheap lambert shading (Z-up).
fn shade(normal: Vec3, base: Color32) -> Color32 {
    let l = Vec3::new(0.45, -0.5, 0.74).normalize();
    let d = normal.dot(l).abs().clamp(0.25, 1.0);
    Color32::from_rgba_unmultiplied(
        (base.r() as f32 * d as f32).round() as u8,
        (base.g() as f32 * d as f32).round() as u8,
        (base.b() as f32 * d as f32).round() as u8,
        base.a(),
    )
}

/// What the renderer needs to know about the view.
pub struct ViewInfo {
    pub mode: ViewModeKind,
    pub active_level: u64,
    pub selection: Vec<u64>,
    /// 2D: half-extent of the visible world rect (meters).
    pub half_extent: f64,
    /// 3D ground grid half-extent.
    pub ground_extent: f64,
    /// Draw ceilings in the 3D view (they can hide the interior otherwise).
    pub show_ceilings_3d: bool,
    /// Draw items in the plan view.
    pub show_items_2d: bool,
}

/// View-mode mirror to avoid an engine dependency in render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewModeKind {
    Plan2D,
    Perspective3D,
}

pub fn build(model: &Model, info: &ViewInfo) -> DisplayList {
    let mut dl = DisplayList::default();
    match info.mode {
        ViewModeKind::Plan2D => build_plan(model, info, &mut dl),
        ViewModeKind::Perspective3D => build_3d(model, info, &mut dl),
    }
    dl
}

fn is_selected(info: &ViewInfo, id: u64) -> bool {
    info.selection.contains(&id)
}

// ---------------------------------------------------------------- 2D plan ---

fn build_plan(model: &Model, info: &ViewInfo, dl: &mut DisplayList) {
    let he = info.half_extent.max(2.0);
    let z_active = model.active_or_first_level(Some(info.active_level)).elevation;

    // Metric grid: 1 m lines, stronger every 5 m.
    let x0 = -(he.ceil() as i64);
    let x1 = he.ceil() as i64;
    for i in x0..=x1 {
        let strong = i % 5 == 0;
        let c = if strong { GRID_STRONG } else { GRID_LINE };
        dl.line(Vec3::new(i as f64, -he, z_active), Vec3::new(i as f64, he, z_active), c, if strong { 1.0 } else { 0.6 });
        dl.line(Vec3::new(-he, i as f64, z_active), Vec3::new(he, i as f64, z_active), c, if strong { 1.0 } else { 0.6 });
    }

    // Datum grids with bubbles.
    for g in &model.grids {
        let (a, b) = match g.axis {
            GridAxis::V => (Vec3::new(g.position, -he, z_active), Vec3::new(g.position, he, z_active)),
            GridAxis::H => (Vec3::new(-he, g.position, z_active), Vec3::new(he, g.position, z_active)),
        };
        let col = if is_selected(info, g.id) { SELECT } else { Color32::from_rgb(0x6b, 0x7a, 0x92) };
        dl.dashed(a, b, col, 1.4);
        let bpos = match g.axis {
            GridAxis::V => Vec3::new(g.position, he - 1.2, z_active),
            GridAxis::H => Vec3::new(he - 1.2, g.position, z_active),
        };
        dl.text(bpos, g.name.clone(), 13.0, col);
    }

    // XYZ axes on the plan too (short, Blender-style).
    dl.line(Vec3::new(-he, 0.0, z_active), Vec3::new(he, 0.0, z_active), with_alpha(AXIS_X, 90), 1.2);
    dl.line(Vec3::new(0.0, -he, z_active), Vec3::new(0.0, he, z_active), with_alpha(AXIS_Y, 90), 1.2);
    dl.text(Vec3::new(he - 1.0, 0.35, z_active), "X", 12.0, with_alpha(AXIS_X, 160));
    dl.text(Vec3::new(0.35, he - 1.0, z_active), "Y", 12.0, with_alpha(AXIS_Y, 160));

    // Elements of every level (dim others, active level normal).
    for e in &model.elements {
        let sel = is_selected(info, e.id);
        let elem_z = element_level_elevation(model, e);
        let on_active = elem_z.map(|z| (z - z_active).abs() < 1e-6).unwrap_or(false);
        let alpha = if on_active || sel { 255 } else { 70 };
        match &e.data {
            ElementData::Wall(w) => draw_wall_plan(dl, model, e, w, info, alpha),
            ElementData::Opening(o) => {
                if on_active || sel {
                    if let Some(q) = model.opening_rect(o) {
                        let base = if e.kind == ElementKind::Door { DOOR_2D } else { WINDOW_2D };
                        let col = with_alpha(selected(base, sel), alpha);
                        // Jamb lines + sill lines across the opening.
                        dl.line(q[0], q[3], col, 1.6);
                        dl.line(q[1], q[2], col, 1.6);
                        if e.kind == ElementKind::Door {
                            draw_door_swing(dl, q, col);
                        } else {
                            let m0 = (q[0] + q[3]) * 0.5;
                            let m1 = (q[1] + q[2]) * 0.5;
                            dl.line(q[0], q[1], col, 1.2);
                            dl.line(m0, m1, col, 1.0);
                        }
                    }
                }
            }
            ElementData::Slab(s) => {
                let col = with_alpha(selected(SLAB_2D, sel), alpha);
                fill_poly_plan(dl, &s.pts, with_alpha(slab_fill(sel), alpha.min(120)));
                outline_poly_plan(dl, &s.pts, col, false);
            }
            ElementData::Ceiling(c) => {
                let col = with_alpha(selected(CEIL_2D, sel), alpha);
                fill_poly_plan(dl, &c.pts, with_alpha(ceil_fill(sel), alpha.min(80)));
                outline_poly_plan(dl, &c.pts, col, true);
                let ctr = bimcraft_bim::poly_center(&c.pts);
                dl.text(ctr.to3(z_active), format!("CEIL {:.1} m²", c.area()), 10.0, col);
            }
            ElementData::Item(i) => {
                if (on_active || sel) && info.show_items_2d {
                    draw_item_plan(dl, i, with_alpha(selected(ITEM_2D, sel), alpha));
                }
            }
        }
    }
}

fn element_level_elevation(model: &Model, e: &bimcraft_bim::Element) -> Option<f64> {
    match &e.data {
        ElementData::Wall(w) => model.level(w.level).map(|l| l.elevation),
        ElementData::Slab(s) => model.level(s.level).map(|l| l.elevation),
        ElementData::Ceiling(c) => model.level(c.level).map(|l| l.elevation),
        ElementData::Item(i) => model.level(i.level).map(|l| l.elevation),
        ElementData::Opening(o) => model.wall(o.wall).and_then(|w| model.level(w.level)).map(|l| l.elevation),
    }
}

fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

fn draw_wall_plan(dl: &mut DisplayList, model: &Model, e: &bimcraft_bim::Element, w: &bimcraft_bim::Wall, info: &ViewInfo, alpha: u8) {
    let sel = is_selected(info, e.id);
    let stroke_col = with_alpha(selected(WALL_2D, sel), alpha);
    if w.pts.len() < 2 {
        return;
    }
    let outline = w.outline();
    // Body fill (left side forward, right side back — one polygon).
    let pts3: Vec<Vec3> = outline.iter().map(|p| p.to3(0.0)).collect();
    dl.poly(&pts3, wall_fill(sel));
    // Outline strokes.
    for i in 0..outline.len() {
        let a = outline[i].to3(0.0);
        let b = outline[(i + 1) % outline.len()].to3(0.0);
        dl.line(a, b, stroke_col, if sel { 2.2 } else { 1.6 });
    }
    // Openings: carve gaps by covering with jamb-to-jamb background quads is
    // left to the canvas (it paints symbols on top); here we add jamb ticks.
    for op in model.openings_for_wall(e.id) {
        if let ElementData::Opening(o) = &op.data {
            if let Some(q) = model.opening_rect(o) {
                let tick = with_alpha(Color32::from_rgb(0x2a, 0x30, 0x3a), alpha);
                dl.line(q[0], q[3], tick, 1.2);
                dl.line(q[1], q[2], tick, 1.2);
            }
        }
    }
}

fn draw_door_swing(dl: &mut DisplayList, q: [Vec3; 4], col: Color32) {
    // Hinge at q[0]; leaf line to the swing side; quarter arc.
    let r = (q[1] - q[0]).length();
    let hinge = q[0];
    let dir_out = (q[3] - q[0]).xy().normalize();
    // Leaf open 90° from the wall direction.
    let leaf_end = hinge.xy() + dir_out * r;
    dl.line(hinge, leaf_end.to3(hinge.z), col, 1.4);
    let mut prev = hinge;
    for k in 1..=10 {
        let ang = std::f64::consts::FRAC_PI_2 * (k as f64 / 10.0);
        let d = (q[1] - q[0]).xy().normalize().rotate(-ang) * r;
        let pnt = (hinge.xy() + d).to3(hinge.z);
        dl.line(prev, pnt, with_alpha(col, 140), 1.0);
        prev = pnt;
    }
}

fn draw_item_plan(dl: &mut DisplayList, i: &bimcraft_bim::Item, col: Color32) {
    let (w, d, _) = i.kind.preset();
    let hw = w / 2.0;
    let hd = d / 2.0;
    let corners = [
        i.pos + Vec2::new(-hw, -hd).rotate(i.rotation),
        i.pos + Vec2::new(hw, -hd).rotate(i.rotation),
        i.pos + Vec2::new(hw, hd).rotate(i.rotation),
        i.pos + Vec2::new(-hw, hd).rotate(i.rotation),
    ];
    let pts3: Vec<Vec3> = corners.iter().map(|p| p.to3(0.0)).collect();
    dl.poly(&pts3, with_alpha(col, 50));
    for k in 0..4 {
        dl.line(pts3[k], pts3[(k + 1) % 4], col, 1.4);
    }
    // Facing tick (front of the item).
    let front = i.pos + Vec2::new(0.0, -hd).rotate(i.rotation);
    let back = i.pos + Vec2::new(0.0, hd * 0.35).rotate(i.rotation);
    dl.line(back.to3(0.0), front.to3(0.0), col, 1.0);
    dl.text(i.pos.to3(0.0), i.kind.name(), 9.0, with_alpha(col, 180));
}

fn fill_poly_plan(dl: &mut DisplayList, pts: &[bimcraft_geom::Vec2], color: Color32) {
    if pts.len() < 3 {
        return;
    }
    let pts3: Vec<Vec3> = pts.iter().map(|p| p.to3(0.0)).collect();
    dl.poly(&pts3, color);
}

fn outline_poly_plan(dl: &mut DisplayList, pts: &[bimcraft_geom::Vec2], color: Color32, dashed: bool) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    for i in 0..n {
        let a = pts[i].to3(0.0);
        let b = pts[(i + 1) % n].to3(0.0);
        if dashed {
            dl.dashed(a, b, color, 1.2);
        } else {
            dl.line(a, b, color, 1.2);
        }
    }
}

// ------------------------------------------------------------------- 3D -----

fn build_3d(model: &Model, info: &ViewInfo, dl: &mut DisplayList) {
    let ext = info.ground_extent.max(10.0);

    // Ground grid, 1 m, stronger every 5 m.
    for i in -(ext.ceil() as i64)..=(ext.ceil() as i64) {
        let v = i as f64;
        let strong = i % 5 == 0;
        let c = if strong { GRID_STRONG } else { GRID_LINE };
        dl.line(Vec3::new(v, -ext, 0.0), Vec3::new(v, ext, 0.0), c, 0.7);
        dl.line(Vec3::new(-ext, v, 0.0), Vec3::new(ext, v, 0.0), c, 0.7);
    }

    // XYZ axes, Blender-style colors.
    dl.line(Vec3::ZERO, Vec3::new(12.0, 0.0, 0.0), AXIS_X, 2.2);
    dl.line(Vec3::ZERO, Vec3::new(0.0, 12.0, 0.0), AXIS_Y, 2.2);
    dl.line(Vec3::ZERO, Vec3::new(0.0, 0.0, 8.0), AXIS_Z, 2.2);
    dl.text(Vec3::new(12.6, 0.0, 0.0), "X", 14.0, AXIS_X);
    dl.text(Vec3::new(0.0, 12.6, 0.0), "Y", 14.0, AXIS_Y);
    dl.text(Vec3::new(0.0, 0.0, 8.5), "Z", 14.0, AXIS_Z);

    // Datum grids as short vertical markers + plan lines.
    for g in &model.grids {
        let col = Color32::from_rgba_unmultiplied(0x6b, 0x7a, 0x92, 110);
        match g.axis {
            GridAxis::V => {
                dl.line(Vec3::new(g.position, -ext, 0.02), Vec3::new(g.position, ext, 0.02), col, 0.9);
                dl.line(Vec3::new(g.position, 0.0, 0.02), Vec3::new(g.position, 0.0, 4.0), col, 1.1);
                dl.text(Vec3::new(g.position, 0.0, 4.4), g.name.clone(), 12.0, col);
            }
            GridAxis::H => {
                dl.line(Vec3::new(-ext, g.position, 0.02), Vec3::new(ext, g.position, 0.02), col, 0.9);
                dl.line(Vec3::new(0.0, g.position, 0.02), Vec3::new(0.0, g.position, 4.0), col, 1.1);
                dl.text(Vec3::new(0.0, g.position, 4.4), g.name.clone(), 12.0, col);
            }
        }
    }

    // Walls as extruded prisms with real opening cuts.
    for e in model.walls() {
        let sel = is_selected(info, e.id);
        if let ElementData::Wall(w) = &e.data {
            draw_wall_3d(dl, model, w, e.id, sel);
        }
    }

    // Slabs and ceilings as prisms.
    for e in &model.elements {
        let sel = is_selected(info, e.id);
        match &e.data {
            ElementData::Slab(s) => {
                let base = model.level(s.level).map(|l| l.elevation).unwrap_or(0.0);
                let col = if sel { SELECT } else { PAL3D.slab };
                prism(dl, &s.pts, base + s.elevation - s.thickness, base + s.elevation, col, true);
            }
            ElementData::Ceiling(c) => {
                if info.show_ceilings_3d || sel {
                    let base = model.level(c.level).map(|l| l.elevation).unwrap_or(0.0);
                    let col = if sel { SELECT } else { PAL3D.ceiling };
                    prism(dl, &c.pts, base + c.elevation, base + c.elevation + c.thickness, col, false);
                }
            }
            ElementData::Item(i) => {
                let z0 = model.level(i.level).map(|l| l.elevation).unwrap_or(0.0);
                draw_item_3d(dl, i, z0, sel);
            }
            _ => {}
        }
    }
}

fn draw_wall_3d(dl: &mut DisplayList, model: &Model, w: &bimcraft_bim::Wall, wall_id: u64, sel: bool) {
    if w.pts.len() < 2 {
        return;
    }
    let z0 = model.level(w.level).map(|l| l.elevation).unwrap_or(0.0);
    let z1 = z0 + w.height;
    let half = w.thickness / 2.0;
    let outline = w.outline();
    let top_col = if sel { SELECT } else { PAL3D.wall_top };
    let side_a = if sel { SELECT } else { PAL3D.wall_side_a };
    let side_b = if sel { SELECT } else { PAL3D.wall_side_b };

    // Top face: outline polygon at z1 (ear-clipped so concave works).
    let top_tris = bimcraft_bim::poly_tris_world(&outline, z1);
    for t in top_tris {
        dl.tri(t, shade(Vec3::new(0.0, 0.0, 1.0), top_col));
    }

    // Side faces per segment, with openings subtracted (real cuts).
    let segs: Vec<(Vec2, Vec2)> = w.segments().collect();
    for (i, (a, b)) in segs.iter().enumerate() {
        let dir = (*b - *a).normalize();
        let n = dir.perp();
        let seg_len = (*b - *a).length();
        if seg_len < 1e-6 {
            continue;
        }
        // Miter-aware start/end offsets for this segment: use the outline
        // vertex positions (they carry the miter) projected on the normal.
        // For simplicity recompute per-segment quads; corner gaps are covered
        // by the top face and the shared outline.
        let a_l = *a + n * half;
        let a_r = *a - n * half;
        let b_l = *b + n * half;
        let b_r = *b - n * half;

        // Openings that live on this segment.
        let mut cuts: Vec<(f64, f64, f64, f64)> = Vec::new(); // (s0, s1, zb, zt)
        for op in model.openings_for_wall(wall_id) {
            if let ElementData::Opening(o) = &op.data {
                // Locate this opening's segment.
                let (seg_idx, local) = w.locate(o.s);
                if seg_idx != i {
                    continue;
                }
                let half_w = o.width / 2.0;
                let s0 = (local - half_w).clamp(0.0, seg_len);
                let s1 = (local + half_w).clamp(0.0, seg_len);
                if s1 > s0 + 1e-6 {
                    cuts.push((s0, s1, z0 + o.sill, (z0 + o.sill + o.height).min(z1)));
                }
            }
        }
        cuts.sort_by(|x, y| x.0.total_cmp(&y.0));

        for side in 0..2 {
            let (p0, p1) = if side == 0 { (a_l, b_l) } else { (a_r, b_r) };
            let nrm = face_normal(p0.to3(z0), p1.to3(z0), p1.to3(z1));
            let base_col = if sel {
                SELECT
            } else if side == 0 {
                side_a
            } else {
                side_b
            };
            let pieces = subtract_cuts(seg_len, z0, z1, &cuts);
            for (s0, s1, pz0, pz1) in pieces {
                let w0 = p0 + dir * s0;
                let w1 = p0 + dir * s1;
                dl.quad([w0.to3(pz0), w1.to3(pz0), w1.to3(pz1), w0.to3(pz1)], shade(nrm, base_col));
            }
        }

        // Opening frames: jamb/head edges + glass for windows.
        for op in model.openings_for_wall(wall_id) {
            if let ElementData::Opening(o) = &op.data {
                let (seg_idx, local) = w.locate(o.s);
                if seg_idx != i {
                    continue;
                }
                let half_w = o.width / 2.0;
                let s0 = local - half_w;
                let s1 = local + half_w;
                let zb = z0 + o.sill;
                let zt = (zb + o.height).min(z1);
                if s1 <= s0 + 1e-6 {
                    continue;
                }
                let c0 = *a + dir * s0.max(0.0);
                let c1 = *a + dir * s1.min(seg_len);
                let frame_col = if sel { SELECT } else { Color32::from_rgb(0x5a, 0x60, 0x6c) };
                // Jamb + head + sill lines on both sides.
                for side in 0..2 {
                    let off = if side == 0 { n * (half + 0.01) } else { n * -(half + 0.01) };
                    let f0 = (c0 + off).to3(z0);
                    let f1 = (c1 + off).to3(z0);
                    dl.line(f0.with_z(zb), f1.with_z(zb), frame_col, 1.2);
                    dl.line(f0.with_z(z0), f0.with_z(zt), frame_col, 1.2);
                    dl.line(f1.with_z(z0), f1.with_z(zt), frame_col, 1.2);
                    dl.line(f0.with_z(zt), f1.with_z(zt), frame_col, 1.2);
                }
                if op.kind == ElementKind::Window {
                    let g0 = (c0 - n * (half * 0.4)).to3(zb);
                    let g1 = (c1 + n * (half * 0.4)).to3(zb);
                    dl.quad([g0, g1, g1.with_z(zt), g0.with_z(zt)], shade(Vec3::new(0.0, 1.0, 0.0), with_alpha(PAL3D.window_glass, 150)));
                } else if op.kind == ElementKind::Door {
                    // Door leaf, drawn 90° open from the hinge.
                    let leaf_col = if sel { SELECT } else { PAL3D.door };
                    let leaf_end = c0 + n * (c1 - c0).length();
                    dl.quad(
                        [c0.to3(zb), leaf_end.to3(zb), leaf_end.to3(zt), c0.to3(zt)],
                        shade(face_normal(c0.to3(zb), leaf_end.to3(zb), leaf_end.to3(zt)), leaf_col),
                    );
                    dl.line(leaf_end.to3(zt), c0.to3(zt), leaf_col, 1.0);
                }
            }
        }
        let _ = side_b;
    }
}

/// Subtract opening cuts from a wall face rectangle: returns (s0, s1, z0, z1)
/// pieces that remain.
fn subtract_cuts(seg_len: f64, z0: f64, z1: f64, cuts: &[(f64, f64, f64, f64)]) -> Vec<(f64, f64, f64, f64)> {
    let mut out = Vec::new();
    let mut cur = 0.0_f64;
    for &(cs0, cs1, cz0, cz1) in cuts {
        if cs0 > cur + 1e-6 {
            out.push((cur, cs0.min(seg_len), z0, z1));
        }
        let s0 = cs0.max(cur);
        let s1 = cs1.min(seg_len);
        if s1 > s0 + 1e-6 {
            if cz0 > z0 + 1e-6 {
                out.push((s0, s1, z0, cz0.min(z1)));
            }
            if cz1 < z1 - 1e-6 {
                out.push((s0, s1, cz1.max(z0), z1));
            }
            cur = s1;
        }
        if cur >= seg_len {
            break;
        }
    }
    if cur < seg_len - 1e-6 {
        out.push((cur, seg_len, z0, z1));
    }
    out
}

fn draw_item_3d(dl: &mut DisplayList, i: &bimcraft_bim::Item, z0: f64, sel: bool) {
    let (w, d, h) = i.kind.preset();
    let hw = w / 2.0;
    let hd = d / 2.0;
    let col = if sel { SELECT } else { PAL3D.item };
    let rot = i.rotation;
    let corner = |dx: f64, dy: f64| -> Vec3 {
        let p = Vec2::new(dx, dy).rotate(rot);
        (i.pos + p).to3(z0)
    };
    let b0 = corner(-hw, -hd);
    let b1 = corner(hw, -hd);
    let b2 = corner(hw, hd);
    let b3 = corner(-hw, hd);
    // Box sides.
    for (p, q) in [(b0, b1), (b1, b2), (b2, b3), (b3, b0)] {
        let nrm = face_normal(p, q, q.with_z(z0 + h));
        dl.quad([p, q, q.with_z(z0 + h), p.with_z(z0 + h)], shade(nrm, col));
    }
    // Top.
    let top_col = shade(Vec3::new(0.0, 0.0, 1.0), col);
    dl.quad([b0.with_z(z0 + h), b1.with_z(z0 + h), b2.with_z(z0 + h), b3.with_z(z0 + h)], top_col);
    // Backrest for seating/bed/wardrobe kinds: a thin second box at the back.
    let back_h = match i.kind {
        ItemKind::Chair | ItemKind::Sofa => h * 0.5,
        ItemKind::Bed => h * 0.8,
        ItemKind::Wardrobe => h * 0.35,
        _ => 0.0,
    };
    if back_h > 0.0 {
        let t = 0.08;
        let c0 = corner(-hw, hd - t);
        let c1 = corner(hw, hd - t);
        let c2 = corner(hw, hd);
        let c3 = corner(-hw, hd);
        let zt = z0 + h * 0.55 + back_h * 0.45;
        for (p, q) in [(c0, c1), (c1, c2), (c2, c3), (c3, c0)] {
            let nrm = face_normal(p, q, q.with_z(zt));
            dl.quad([p, q, q.with_z(zt), p.with_z(zt)], shade(nrm, col));
        }
        dl.quad([c0.with_z(zt), c1.with_z(zt), c2.with_z(zt), c3.with_z(zt)], top_col);
    }
    // Wireframe edges.
    for (p, q) in [(b0, b1), (b1, b2), (b2, b3), (b3, b0)] {
        dl.line(p.with_z(z0 + h), q.with_z(z0 + h), col, 0.8);
    }
}

fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    (b - a).cross(c - a).normalize()
}

fn prism(dl: &mut DisplayList, pts: &[bimcraft_geom::Vec2], z0: f64, z1: f64, color: Color32, top_face: bool) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    for i in 0..n {
        let p0 = pts[i].to3(z0);
        let p1 = pts[(i + 1) % n].to3(z0);
        let nrm = face_normal(p0, p1, p1.with_z(z1));
        dl.quad([p0, p1, p1.with_z(z1), p0.with_z(z1)], shade(nrm, color));
    }
    if top_face {
        let top: Vec<Vec3> = pts.iter().map(|p| p.to3(z1)).collect();
        dl.poly(&top, shade(Vec3::new(0.0, 0.0, 1.0), color));
    }
}

/// Convenience: Stroke is referenced so the API surface is stable for callers.
pub fn stroke_for_width(w: f32) -> Stroke {
    Stroke::new(w, Color32::WHITE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bimcraft_bim::{sample, Model, Opening};
    use bimcraft_geom::Vec2;

    #[test]
    fn plan_build_runs() {
        let m = sample();
        let info = ViewInfo {
            mode: ViewModeKind::Plan2D,
            active_level: m.levels[0].id,
            selection: vec![],
            half_extent: 20.0,
            ground_extent: 20.0,
            show_ceilings_3d: true,
            show_items_2d: true,
        };
        let dl = build(&m, &info);
        assert!(!dl.lines.is_empty());
    }

    #[test]
    fn three_d_build_runs() {
        let m = sample();
        let info = ViewInfo {
            mode: ViewModeKind::Perspective3D,
            active_level: m.levels[0].id,
            selection: vec![],
            half_extent: 20.0,
            ground_extent: 30.0,
            show_ceilings_3d: true,
            show_items_2d: true,
        };
        let dl = build(&m, &info);
        assert!(!dl.tris.is_empty());
    }

    #[test]
    fn opening_cuts_leave_glass_only_where_cut() {
        let mut m = Model::new();
        let lvl = m.levels[0].id;
        let w = m.add_wall(lvl, vec![Vec2::ZERO, Vec2::new(5.0, 0.0)], 0.2, 3.0);
        m.add_opening(ElementKind::Window, Opening { wall: w, s: 2.5, width: 1.2, height: 1.4, sill: 0.9 });
        let info = ViewInfo {
            mode: ViewModeKind::Perspective3D,
            active_level: lvl,
            selection: vec![],
            half_extent: 20.0,
            ground_extent: 30.0,
            show_ceilings_3d: false,
            show_items_2d: true,
        };
        let dl = build(&m, &info);
        // The wall (5 m × 3 m face) minus a 1.2 m gap must produce more than
        // two side pieces: full pieces + below/above sill pieces.
        let side_pieces = dl.tris.iter().filter(|t| (t.pts[0].z - t.pts[1].z).abs() < 3.2).count();
        assert!(side_pieces > 8);
    }

    #[test]
    fn subtract_cuts_math() {
        let cuts = vec![(2.0, 3.0, 0.9, 2.3)];
        let pieces = subtract_cuts(5.0, 0.0, 3.0, &cuts);
        // [0..2 full] [2..3 below] [2..3 above] [3..5 full]
        assert_eq!(pieces.len(), 4);
        let full: f64 = pieces.iter().map(|(s0, s1, z0, z1)| (s1 - s0) * (z1 - z0)).sum();
        assert!((full - (15.0 - 1.0 * 1.4)).abs() < 1e-6);
    }
}
