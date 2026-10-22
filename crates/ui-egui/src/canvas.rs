//! BimCraft canvas: 2D plan and 3D perspective viewports with a Blender-style
//! ground grid, XYZ axes and tool interaction. Everything is drawn through the
//! egui painter with hand-rolled projection math (no extra 3D deps).

use crate::theme;
use bimcraft_engine::{Selection, Tool, ViewMode};
use bimcraft_geom::Vec2 as GVec2;
use bimcraft_geom::Vec3;
use bimcraft_render::{build, ViewInfo, ViewModeKind};
use egui::{Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Vec2};

pub fn show(ui: &mut egui::Ui, app: &mut crate::BimcraftApp) {
    let t = theme::tokens();
    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(0), t.viewport_bg);

    let ppp = ui.ctx().pixels_per_point();
    let mode = match app.session.view {
        ViewMode::Plan2D => ViewModeKind::Plan2D,
        ViewMode::Perspective3D | ViewMode::Walk => ViewModeKind::Perspective3D,
        ViewMode::Section => ViewModeKind::Section,
    };

    // Handle input first (mutates session/camera), then render.
    handle_input(ui, app, rect, resp, ppp);

    match app.session.view {
        ViewMode::Plan2D | ViewMode::Perspective3D | ViewMode::Walk => {
            let eye = match app.session.view {
                ViewMode::Walk => app.session.walk.eye(app.session.level_elevation()),
                _ => app.session.camera.eye(),
            };
            let info = ViewInfo {
                mode,
                active_level: app.session.active_level_id(),
                selection: match app.session.selection {
                    Selection::Element(id) => vec![id],
                    _ => vec![],
                },
                half_extent: plan_half_extent(app.session.plan.zoom, rect.width()),
                ground_extent: 40.0,
                show_ceilings_3d: app.show_ceilings_3d,
                show_items_2d: app.show_items_2d,
                eye,
            };
            let dl = build(&app.session.model, &info);
            match app.session.view {
                ViewMode::Plan2D => draw_plan(ui, app, &painter, rect, &dl),
                ViewMode::Perspective3D | ViewMode::Walk => draw_3d(ui, app, &painter, rect, &dl),
                ViewMode::Section => unreachable!(),
            }
        }
        ViewMode::Section => {
            if let Some(sid) = app.session.active_or_first_section() {
                if let Some(sec) = app.session.model.section(sid).cloned() {
                    let dl = bimcraft_render::build_section(&app.session.model, &sec);
                    draw_section(ui, app, &painter, rect, &dl, &sec);
                }
            } else {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No sections yet — pick the Section tool and click two points in the plan",
                    theme::font_ui(13.0),
                    Color32::from_rgb(0x8a, 0x93, 0xa3),
                );
            }
        }
    }

    // Tool overlays (pending rubber-band etc.).
    draw_overlays(ui, app, &painter, rect);
}

fn plan_half_extent(zoom: f64, width_px: f32) -> f64 {
    (width_px as f64 / zoom.max(1.0)) * 0.6 + 2.0
}

// -------------------------------------------------------------- input -----

fn handle_input(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, rect: Rect, resp: egui::Response, ppp: f32) {
    let scroll = ui.ctx().input(|i| i.smooth_scroll_delta.y);
    let pos = resp.hovered().then(|| ui.ctx().input(|i| i.pointer.latest_pos())).flatten();

    // World cursor (for the status bar + tool preview). Plan tools work from
    // the snapped position; camera moves use the raw position.
    if let Some(p) = pos {
        let world_raw = screen_world(app, p, rect, ppp);
        if let Some(w) = world_raw {
            app.session.cursor = w;
            if app.session.view == ViewMode::Plan2D {
                app.session.cursor_plan_snapped = Some(app.session.snap(w.xy()));
            } else {
                app.session.cursor_plan_snapped = None;
            }
        }
    }

    // Walk mode: continuous movement + mouse-look; other tools idle.
    if app.session.view == ViewMode::Walk {
        let dt = ui.ctx().input(|i| i.stable_dt).max(1.0 / 240.0) as f64;
        let (mut fwd, mut side) = (0.0_f64, 0.0_f64);
        ui.ctx().input(|i| {
            let shift = i.modifiers.shift;
            let speed = if shift { 4.4 } else { 2.4 } * dt;
            if i.key_down(egui::Key::W) {
                fwd += speed;
            }
            if i.key_down(egui::Key::S) && !i.modifiers.ctrl {
                fwd -= speed;
            }
            if i.key_down(egui::Key::D) {
                side += speed;
            }
            if i.key_down(egui::Key::A) {
                side -= speed;
            }
        });
        if fwd != 0.0 || side != 0.0 {
            app.session.walk.move_by(fwd, side);
        }
        // Wheel: step forward / back.
        if scroll.abs() > 0.5 && resp.hovered() {
            app.session.walk.move_by(scroll as f64 * 0.004, 0.0);
        }
        let drag = resp.drag_delta();
        if resp.dragged() && drag != Vec2::ZERO {
            app.session.walk.look(drag.x as f64 * 0.006, drag.y as f64 * 0.005);
        }
        // Status cursor: a probe point a couple meters ahead.
        let w = &app.session.walk;
        let ahead = GVec2::new(w.yaw.cos(), w.yaw.sin()) * 2.5;
        app.session.cursor = Vec3::new(w.pos.x + ahead.x, w.pos.y + ahead.y, w.eye_z);
        app.session.cursor_plan_snapped = None;
        return;
    }

    // Wheel zoom.
    if scroll.abs() > 0.5 && resp.hovered() {
        match app.session.view {
            ViewMode::Plan2D | ViewMode::Section => {
                let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
                let new_zoom = (app.session.plan.zoom * factor).clamp(6.0, 400.0);
                if let Some(p) = pos {
                    // Zoom toward the cursor.
                    let before = screen_to_view2d(app, p, rect);
                    app.session.plan.zoom = new_zoom;
                    let after = screen_to_view2d(app, p, rect);
                    app.session.plan.center = app.session.plan.center + before - after;
                } else {
                    app.session.plan.zoom = new_zoom;
                }
            }
            ViewMode::Perspective3D => {
                app.session.camera.distance = (app.session.camera.distance * if scroll > 0.0 { 0.9 } else { 1.1 }).clamp(3.0, 200.0);
            }
            ViewMode::Walk => {}
        }
    }

    // Drag interactions.
    let drag = resp.drag_delta();
    if resp.drag_started() {
        // Select tool: start moving / shifting the selected element when the
        // drag begins over it; otherwise pan.
        if app.session.view == ViewMode::Plan2D && app.session.tool == Tool::Select {
            if let Some(p) = pos {
                let world = screen_to_plan(app, p, rect);
                let hit = app.session.pick_plan(world);
                let hovering_selection = !matches!(hit, Selection::None)
                    && match (hit, app.session.selection) {
                        (Selection::Element(a), Selection::Element(b)) => a == b,
                        (Selection::Grid(a), Selection::Grid(b)) => a == b,
                        _ => false,
                    };
                if hovering_selection && app.session.begin_drag(world) {
                    // drag started on the selection
                }
            }
        }
    }
    if resp.dragged() && drag != Vec2::ZERO {
        match app.session.view {
            ViewMode::Plan2D => {
                if app.session.dragging() {
                    if let Some(p) = pos {
                        let world = screen_to_plan(app, p, rect);
                        app.session.drag_to(world);
                    }
                } else {
                    let delta = GVec2::new(drag.x as f64, -drag.y as f64) * (1.0 / app.session.plan.zoom);
                    app.session.plan.center = app.session.plan.center - delta;
                }
            }
            ViewMode::Section => {
                let delta = GVec2::new(drag.x as f64, -drag.y as f64) * (1.0 / app.session.plan.zoom);
                app.session.plan.center = app.session.plan.center - delta;
            }
            ViewMode::Perspective3D => {
                if ui.ctx().input(|i| i.modifiers.shift) {
                    pan_target(app, drag, rect, ppp);
                } else {
                    app.session.camera.yaw -= drag.x as f64 * 0.008;
                    app.session.camera.pitch = (app.session.camera.pitch + drag.y as f64 * 0.006).clamp(0.05, 1.55);
                }
            }
            ViewMode::Walk => {}
        }
    }
    if resp.drag_stopped() {
        app.session.end_drag();
    }

    // Clicks.
    if resp.clicked() {
        if let Some(p) = pos {
            match app.session.view {
                ViewMode::Plan2D => {
                    let world = app.session.snap(screen_to_plan(app, p, rect));
                    app.session.click_plan(world);
                }
                ViewMode::Perspective3D => {
                    if let Some(g) = pick_ground_at(app, p, rect) {
                        let world = app.session.snap(g.xy());
                        app.session.click_plan(world);
                    }
                }
                ViewMode::Section => {
                    // Clicking a section in the section view selects it.
                    if app.session.tool == Tool::Select {
                        if let Some(sid) = app.session.active_or_first_section() {
                            app.session.select(Selection::Section(sid));
                        }
                    }
                }
                ViewMode::Walk => {}
            }
        }
    }
    if resp.double_clicked() {
        app.session.finish_draft();
    }
    if resp.secondary_clicked() {
        app.session.cancel_pending();
    }
}

fn screen_world(app: &crate::BimcraftApp, p: Pos2, rect: Rect, _ppp: f32) -> Option<Vec3> {
    match app.session.view {
        ViewMode::Plan2D => Some(screen_to_plan(app, p, rect).to3(app.session.level_elevation())),
        ViewMode::Perspective3D => pick_ground_at(app, p, rect),
        ViewMode::Walk => None,
        ViewMode::Section => {
            let v = screen_to_view2d(app, p, rect);
            Some(Vec3::new(v.x, 0.0, v.y))
        }
    }
}

/// In Plan2D this maps to world XY; in Section view it maps to (u, z).
fn screen_to_view2d(app: &crate::BimcraftApp, p: Pos2, rect: Rect) -> GVec2 {
    let cx = rect.center().x as f64;
    let cy = rect.center().y as f64;
    let wx = (p.x as f64 - cx) / app.session.plan.zoom + app.session.plan.center.x;
    let wy = (cy - p.y as f64) / app.session.plan.zoom + app.session.plan.center.y;
    GVec2::new(wx, wy)
}

fn pick_ground_at(app: &crate::BimcraftApp, p: Pos2, rect: Rect) -> Option<Vec3> {
    // Logical pixels: matches the logical-unit projection in draw_3d.
    app.session.camera.pick_ground(p.x as f64, p.y as f64, rect.width() as f64, rect.height() as f64, FOV)
}

fn pan_target(app: &mut crate::BimcraftApp, drag: Vec2, rect: Rect, ppp: f32) {
    let (r, u, _) = app.session.camera.basis();
    let scale = app.session.camera.distance / (rect.height() as f64 * ppp as f64);
    let move_r = r * (-drag.x as f64 * ppp as f64 * scale);
    let move_u = u * (drag.y as f64 * ppp as f64 * scale);
    let m = move_r + move_u;
    app.session.camera.target = Vec3::new(app.session.camera.target.x + m.x, app.session.camera.target.y + m.y, app.session.camera.target.z);
}

fn screen_to_plan(app: &crate::BimcraftApp, p: Pos2, rect: Rect) -> bimcraft_geom::Vec2 {
    let cx = rect.center().x as f64;
    let cy = rect.center().y as f64;
    let wx = (p.x as f64 - cx) / app.session.plan.zoom + app.session.plan.center.x;
    let wy = (cy - p.y as f64) / app.session.plan.zoom + app.session.plan.center.y;
    bimcraft_geom::Vec2::new(wx, wy)
}

fn plan_to_screen(app: &crate::BimcraftApp, w: Vec3, rect: Rect) -> Pos2 {
    let cx = rect.center().x;
    let cy = rect.center().y;
    let sx = ((w.x - app.session.plan.center.x) * app.session.plan.zoom + cx as f64) as f32;
    let sy = (cy as f64 - (w.y - app.session.plan.center.y) * app.session.plan.zoom) as f32;
    Pos2::new(sx, sy)
}

// --------------------------------------------------------------- 2D ------

fn draw_plan(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, painter: &egui::Painter, rect: Rect, dl: &bimcraft_render::DisplayList) {
    let _ = ui;
    for l in &dl.lines {
        let a = plan_to_screen(app, l.a, rect);
        let b = plan_to_screen(app, l.b, rect);
        if l.dashed {
            draw_dashed(painter, a, b, l.color, l.width);
        } else {
            painter.line_segment([a, b], Stroke::new(l.width, l.color));
        }
    }
    for tri in &dl.tris {
        let pts: Vec<Pos2> = tri.pts.iter().map(|p| plan_to_screen(app, *p, rect)).collect();
        painter.add(egui::Shape::convex_polygon(pts, tri.color, Stroke::NONE));
    }
    for tx in &dl.texts {
        painter.text(plan_to_screen(app, tx.pos, rect), egui::Align2::CENTER_CENTER, &tx.text, theme::font_ui(tx.size), tx.color);
    }
    // Active level watermark.
    let lvl = app.session.model.active_or_first_level(app.session.active_level);
    painter.text(
        rect.left_bottom() + Vec2::new(10.0, -10.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{}  ·  elev {:.2} m", lvl.name, lvl.elevation),
        theme::font_ui(11.0),
        Color32::from_rgb(0x8a, 0x93, 0xa3),
    );
}

/// Section view: the display list is in section space (x = u along the cut,
/// z = height); reuse the plan transform by feeding (u, z) as world (x, y).
fn draw_section(
    ui: &mut egui::Ui,
    app: &mut crate::BimcraftApp,
    painter: &egui::Painter,
    rect: Rect,
    dl: &bimcraft_render::DisplayList,
    sec: &bimcraft_bim::Section,
) {
    let _ = ui;
    let t = theme::tokens();
    let map = |p: Vec3| -> Pos2 {
        let w = Vec3::new(p.x, p.z, 0.0);
        plan_to_screen(app, w, rect)
    };
    for l in &dl.lines {
        let a = map(l.a);
        let b = map(l.b);
        if l.dashed {
            draw_dashed(painter, a, b, l.color, l.width);
        } else {
            painter.line_segment([a, b], Stroke::new(l.width, l.color));
        }
    }
    for tri in &dl.tris {
        let pts: Vec<Pos2> = tri.pts.iter().map(|p| map(*p)).collect();
        painter.add(egui::Shape::convex_polygon(pts, tri.color, Stroke::NONE));
    }
    for tx in &dl.texts {
        painter.text(map(tx.pos), egui::Align2::CENTER_CENTER, &tx.text, theme::font_ui(tx.size), tx.color);
    }
    // Viewport hint.
    painter.text(
        rect.right_top() + Vec2::new(-14.0, 14.0),
        egui::Align2::RIGHT_TOP,
        format!("{}  ·  drag: pan  ·  wheel: zoom", sec.name),
        theme::font_ui(11.0),
        Color32::from_rgb(0x8a, 0x93, 0xa3),
    );
    // Cursor readout in section space.
    painter.text(
        rect.left_bottom() + Vec2::new(10.0, -10.0),
        egui::Align2::LEFT_BOTTOM,
        format!("u {:.2} m · z {:.2} m", app.session.cursor.x, app.session.cursor.z),
        theme::font_mono(11.0),
        t.text_dim,
    );
}

fn draw_dashed(painter: &egui::Painter, a: Pos2, b: Pos2, color: Color32, width: f32) {
    let seg = 7.0;
    let gap = 5.0;
    let v = b - a;
    let len = v.length();
    if len < 1.0 {
        return;
    }
    let dir = v / len;
    let mut t = 0.0;
    while t < len {
        let end = (t + seg).min(len);
        painter.line_segment([a + dir * t, a + dir * end], Stroke::new(width, color));
        t = end + gap;
    }
}

// --------------------------------------------------------------- 3D ------

const FOV: f64 = std::f64::consts::FRAC_PI_3;

struct Proj {
    focal: f64,
    cx: f64,
    cy: f64,
    w: f64,
    h: f64,
}

impl Proj {
    fn point(&self, p: Vec3, eye: Vec3, r: Vec3, u: Vec3, f: Vec3) -> Option<(Pos2, f64)> {
        let d = p - eye;
        let x = d.dot(r);
        let y = d.dot(u);
        let z = d.dot(f);
        if z < 0.08 {
            return None;
        }
        let sx = self.cx + x / z * self.focal;
        let sy = self.cy - y / z * self.focal;
        if sx < -self.w || sx > self.w * 2.0 || sy < -self.h || sy > self.h * 2.0 {
            return None;
        }
        Some((Pos2::new(sx as f32, sy as f32), z))
    }
}

fn draw_3d(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, painter: &egui::Painter, rect: Rect, dl: &bimcraft_render::DisplayList) {
    let _ = ui;
    let walking = app.session.view == ViewMode::Walk;
    // Camera: orbit camera or the first-person walk camera.
    let (eye, (r, u, f)) = if walking {
        let w = app.session.walk;
        (w.eye(app.session.level_elevation()), w.basis())
    } else {
        let cam = app.session.camera;
        (cam.eye(), cam.basis())
    };
    let ppp = 1.0_f64; // projection runs in logical pixels
    let proj = Proj {
        focal: (rect.height() as f64 * ppp) / (2.0 * (FOV / 2.0).tan()),
        cx: rect.center().x as f64 * ppp,
        cy: rect.center().y as f64 * ppp,
        w: rect.width() as f64 * ppp,
        h: rect.height() as f64 * ppp,
    };

    // Depth-sorted triangles (keep per-vertex view depth for the z-buffer).
    let mut tris: Vec<([Pos2; 3], [f64; 3], f64, Color32)> = Vec::with_capacity(dl.tris.len());
    for tri in &dl.tris {
        let mut pts = [Pos2::ZERO; 3];
        let mut zs = [0.0_f64; 3];
        let mut ok = true;
        for k in 0..3 {
            match proj.point(tri.pts[k], eye, r, u, f) {
                Some((sp, z)) => {
                    pts[k] = sp;
                    zs[k] = z;
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            tris.push((pts, zs, (zs[0] + zs[1] + zs[2]) / 3.0, tri.color));
        }
    }
    tris.sort_by(|a, b| b.2.total_cmp(&a.2));
    for (pts, _, _, color) in &tris {
        painter.add(egui::Shape::convex_polygon(pts.to_vec(), *color, Stroke::NONE));
    }

    // Depth buffer of the projected triangles: helper lines (grid, datums,
    // axes) are then occluded by the model instead of drawing over it.
    let mut depth = DepthBuf::new(rect.width().ceil() as usize, rect.height().ceil() as usize);
    for (pts, zs, _, _) in &tris {
        depth.rasterize(*pts, *zs);
    }

    // Lines with occlusion (was: plain wireframe pass on top).
    for l in &dl.lines {
        draw_line_occluded(painter, &proj, &depth, eye, r, u, f, l.a, l.b, l.color, l.width);
    }

    // Texts (projected; skip when behind camera or hidden by the model).
    for tx in &dl.texts {
        if let Some((sp, z)) = proj.point(tx.pos, eye, r, u, f) {
            if depth.visible(sp.x, sp.y, z + 0.25) {
                painter.text(sp, egui::Align2::CENTER_CENTER, &tx.text, theme::font_ui(tx.size), tx.color);
            }
        }
    }

    // Viewport hint.
    let hint = if walking {
        "WASD: move · drag: look · Shift: run · wheel: step · Esc: exit"
    } else {
        "drag: orbit · shift+drag: pan · wheel: zoom · V: walk"
    };
    painter.text(rect.right_top() + Vec2::new(-14.0, 14.0), egui::Align2::RIGHT_TOP, hint, theme::font_ui(11.0), Color32::from_rgb(0x8a, 0x93, 0xa3));
}

/// Coarse software depth buffer (logical pixels) built from the projected
/// triangles; used to occlude helper lines against the model.
struct DepthBuf {
    w: usize,
    h: usize,
    data: Vec<f32>,
}

impl DepthBuf {
    fn new(w: usize, h: usize) -> Self {
        let w = w.max(1);
        let h = h.max(1);
        Self { w, h, data: vec![f32::INFINITY; w * h] }
    }

    /// Barycentric rasterization with linearly interpolated view depth.
    fn rasterize(&mut self, pts: [Pos2; 3], zs: [f64; 3]) {
        let (p0, p1, p2) = (pts[0], pts[1], pts[2]);
        let area = (p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y);
        if area.abs() < 0.01 {
            return;
        }
        let inv = 1.0_f32 / area;
        let minx = (p0.x.min(p1.x).min(p2.x).floor().max(0.0) as usize).min(self.w.saturating_sub(1));
        let maxx = (p0.x.max(p1.x).max(p2.x).ceil().min(self.w as f32 - 1.0).max(0.0) as usize).min(self.w.saturating_sub(1));
        let miny = (p0.y.min(p1.y).min(p2.y).floor().max(0.0) as usize).min(self.h.saturating_sub(1));
        let maxy = (p0.y.max(p1.y).max(p2.y).ceil().min(self.h as f32 - 1.0).max(0.0) as usize).min(self.h.saturating_sub(1));
        if minx > maxx || miny > maxy {
            return;
        }
        for y in miny..=maxy {
            let row = y * self.w;
            for x in minx..=maxx {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                // Signed barycentric weights (orientation-agnostic).
                let w0 = ((p1.y - p2.y) * (px - p2.x) + (p2.x - p1.x) * (py - p2.y)) * inv;
                let w1 = ((p2.y - p0.y) * (px - p2.x) + (p0.x - p2.x) * (py - p2.y)) * inv;
                let w2 = 1.0 - w0 - w1;
                // Inside test in either winding.
                let inside = (w0 >= -1e-4 && w1 >= -1e-4 && w2 >= -1e-4) || (w0 <= 1e-4 && w1 <= 1e-4 && w2 <= 1e-4);
                if !inside {
                    continue;
                }
                let z = (w0 as f64) * zs[0] + (w1 as f64) * zs[1] + (w2 as f64) * zs[2];
                let slot = &mut self.data[row + x];
                if (z as f32) < *slot {
                    *slot = z as f32;
                }
            }
        }
    }

    fn visible(&self, x: f32, y: f32, z: f64) -> bool {
        let xi = (x.max(0.0) as usize).min(self.w - 1);
        let yi = (y.max(0.0) as usize).min(self.h - 1);
        z <= self.data[yi * self.w + xi] as f64 + 0.045
    }
}

/// Draw a 3D line, clipped at the near plane and occluded where the model's
/// depth buffer is in front of it (sampled every few screen pixels).
fn draw_line_occluded(
    painter: &egui::Painter,
    proj: &Proj,
    depth: &DepthBuf,
    eye: Vec3,
    r: Vec3,
    u: Vec3,
    f: Vec3,
    a: Vec3,
    b: Vec3,
    color: Color32,
    width: f32,
) {
    const NEAR: f64 = 0.08;
    let view = |p: Vec3| -> (f64, f64, f64) {
        let d = p - eye;
        (d.dot(r), d.dot(u), d.dot(f))
    };
    let (mut ax, mut ay, mut az) = view(a);
    let (mut bx, mut by, mut bz) = view(b);
    if az < NEAR && bz < NEAR {
        return;
    }
    if az < NEAR {
        let t = (NEAR - az) / (bz - az);
        ax += (bx - ax) * t;
        ay += (by - ay) * t;
        az = NEAR;
    } else if bz < NEAR {
        let t = (NEAR - bz) / (az - bz);
        bx += (ax - bx) * t;
        by += (ay - by) * t;
        bz = NEAR;
    }
    // Rebuild clipped 3D endpoints from view-space back to world space.
    let to_world = |v: (f64, f64, f64)| eye + r * v.0 + u * v.1 + f * v.2;
    let wa = to_world((ax, ay, az));
    let wb = to_world((bx, by, bz));
    // Screen span decides the sample count (about one sample per 3 px).
    let sa = Pos2::new((proj.cx + ax / az * proj.focal) as f32, (proj.cy - ay / az * proj.focal) as f32);
    let sb = Pos2::new((proj.cx + bx / bz * proj.focal) as f32, (proj.cy - by / bz * proj.focal) as f32);
    let span = ((sb.x - sa.x).abs().max((sb.y - sa.y).abs()) as usize).clamp(2, 512);
    let mut prev: Option<(Pos2, bool)> = None;
    for k in 0..=span {
        let t = k as f64 / span as f64;
        let p = wa + (wb - wa) * t;
        let Some((sp, z)) = proj.point(p, eye, r, u, f) else {
            prev = None;
            continue;
        };
        let vis = depth.visible(sp.x, sp.y, z);
        if let Some((last, last_vis)) = prev {
            if vis && last_vis {
                painter.line_segment([last, sp], Stroke::new(width, color));
            }
        }
        prev = Some((sp, vis));
    }
}

// ------------------------------------------------------------ overlays ---

fn draw_overlays(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, painter: &egui::Painter, rect: Rect) {
    let _ = ui;
    let t = theme::tokens();
    if app.session.view != ViewMode::Plan2D {
        return;
    }

    // Snapped cursor world position for previews.
    let cursor = app.session.cursor_plan_snapped.unwrap_or_else(|| app.session.cursor.xy());

    // Snap indicator dot.
    if app.session.snap_grid || app.session.snap_endpoints {
        let sp = plan_to_screen(app, cursor.to3(app.session.level_elevation()), rect);
        if rect.contains(sp) {
            painter.circle_filled(sp, 2.5, t.accent.gamma_multiply(0.7));
        }
    }

    // Wall chain preview.
    if app.session.tool == Tool::Wall && !app.session.pending.is_empty() {
        let mut prev = app.session.pending[0];
        for &pt in &app.session.pending[1..] {
            draw_preview_segment(app, painter, rect, prev, pt, t.accent);
            prev = pt;
        }
        draw_preview_segment(app, painter, rect, prev, cursor, t.accent);
        let total: f64 = app.session.pending.windows(2).map(|w| (w[1] - w[0]).length()).sum::<f64>() + prev.dist(cursor);
        let b = plan_to_screen(app, cursor.to3(0.0), rect);
        painter.text(b + Vec2::new(10.0, -10.0), egui::Align2::LEFT_BOTTOM, format!("Σ {total:.2} m"), theme::font_mono(11.0), t.accent);
        for &pt in &app.session.pending {
            painter.circle_filled(plan_to_screen(app, pt.to3(0.0), rect), 3.0, t.accent);
        }
    }

    // Polygon plate preview (floor / ceiling).
    if matches!(app.session.tool, Tool::Floor | Tool::Ceiling) && !app.session.pending.is_empty() {
        let mut pts: Vec<Pos2> = app.session.pending.iter().map(|p| plan_to_screen(app, p.to3(0.0), rect)).collect();
        pts.push(plan_to_screen(app, cursor.to3(0.0), rect));
        if pts.len() >= 3 {
            painter.add(egui::Shape::convex_polygon(pts.clone(), t.accent.gamma_multiply(0.12), Stroke::NONE));
        }
        for i in 0..pts.len() {
            draw_dashed(painter, pts[i], pts[(i + 1) % pts.len()], t.accent, 1.4);
        }
        for p in &pts {
            painter.circle_filled(*p, 3.0, t.accent);
        }
        let poly: Vec<GVec2> = app.session.pending.iter().copied().chain(std::iter::once(cursor)).collect();
        let area = bimcraft_geom::poly_area(&poly).abs();
        painter.text(
            pts[pts.len() - 1] + Vec2::new(10.0, -10.0),
            egui::Align2::LEFT_BOTTOM,
            format!("A {area:.2} m²"),
            theme::font_mono(11.0),
            t.accent,
        );
    }

    // Item placement ghost.
    if app.session.tool == Tool::Item {
        let (w, d, _) = app.session.item_kind.preset();
        let rot = app.session.item_rotation;
        let corners = [
            cursor + GVec2::new(-w / 2.0, -d / 2.0).rotate(rot),
            cursor + GVec2::new(w / 2.0, -d / 2.0).rotate(rot),
            cursor + GVec2::new(w / 2.0, d / 2.0).rotate(rot),
            cursor + GVec2::new(-w / 2.0, d / 2.0).rotate(rot),
        ];
        let pts: Vec<Pos2> = corners.iter().map(|p| plan_to_screen(app, p.to3(0.0), rect)).collect();
        painter.add(egui::Shape::convex_polygon(pts.clone(), t.accent.gamma_multiply(0.15), Stroke::NONE));
        for i in 0..4 {
            painter.line_segment([pts[i], pts[(i + 1) % 4]], Stroke::new(1.4, t.accent));
        }
        painter.text(pts[0] + Vec2::new(4.0, -4.0), egui::Align2::LEFT_BOTTOM, app.session.item_kind.name(), theme::font_ui(10.0), t.accent);
    }

    // Measure overlay.
    if let Some(m) = app.session.measure {
        let a = plan_to_screen(app, m.a.to3(0.0), rect);
        let b = plan_to_screen(app, (if m.a == m.b { cursor } else { m.b }).to3(0.0), rect);
        painter.line_segment([a, b], Stroke::new(1.8, Color32::from_rgb(0x5b, 0xc8, 0xd8)));
        painter.circle_filled(a, 3.0, Color32::from_rgb(0x5b, 0xc8, 0xd8));
        painter.circle_filled(b, 3.0, Color32::from_rgb(0x5b, 0xc8, 0xd8));
        let len = m.a.dist(if m.a == m.b { cursor } else { m.b });
        painter.text(
            Pos2::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5) + Vec2::new(0.0, -12.0),
            egui::Align2::CENTER_BOTTOM,
            format!("{len:.2} m"),
            theme::font_mono(12.0),
            Color32::from_rgb(0x5b, 0xc8, 0xd8),
        );
    }

    // Origin marker.
    let o = plan_to_screen(app, Vec3::ZERO, rect);
    if rect.contains(o) {
        painter.circle_stroke(o, 4.0, Stroke::new(1.2, bimcraft_render::AXIS_X));
        painter.line_segment([o - Vec2::new(7.0, 0.0), o + Vec2::new(7.0, 0.0)], Stroke::new(1.0, bimcraft_render::AXIS_X));
        painter.line_segment([o - Vec2::new(0.0, 7.0), o + Vec2::new(0.0, 7.0)], Stroke::new(1.0, bimcraft_render::AXIS_Y));
    }
}

fn draw_preview_segment(app: &crate::BimcraftApp, painter: &egui::Painter, rect: Rect, a: GVec2, b: GVec2, color: Color32) {
    let sa = plan_to_screen(app, a.to3(0.0), rect);
    let sb = plan_to_screen(app, b.to3(0.0), rect);
    painter.line_segment([sa, sb], Stroke::new(1.6, color));
    let len = a.dist(b);
    painter.text(
        Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5) + Vec2::new(8.0, -8.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{len:.2}"),
        theme::font_mono(10.0),
        color,
    );
}
