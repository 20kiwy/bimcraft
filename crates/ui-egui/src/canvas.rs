//! BimCraft canvas: 2D plan and 3D perspective viewports with a Blender-style
//! ground grid, XYZ axes and tool interaction. Everything is drawn through the
//! egui painter with hand-rolled projection math (no extra 3D deps).

use crate::theme::{self, Tokens};
use bimcraft_engine::{Selection, Tool, ViewMode};
use bimcraft_geom::Vec2 as GVec2;
use bimcraft_geom::Vec3;
use bimcraft_render::{build, ViewInfo, ViewModeKind};
use egui::{Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Vec2};

pub fn show(ui: &mut egui::Ui, app: &mut crate::BimcraftApp) {
    let t = theme::tokens_for(app.theme);
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
            let info = {
                // Ghost host for the 3D opening placeholder: wall under the
                // cursor + arclength (only when placing doors/windows).
                let ghost_host = match app.session.tool {
                    bimcraft_engine::Tool::Door | bimcraft_engine::Tool::Window => {
                        let p = app.session.cursor.xy();
                        app.session.model.host_wall_at(p, 1.0).map(|(id, _)| {
                            let (s, _) = app.session.model.wall(id).map(|w| w.project(p)).unwrap_or((0.0, 0.0));
                            (id, s)
                        })
                    }
                    _ => None,
                };
                let pending_pts = if app.session.tool == bimcraft_engine::Tool::Site { app.session.pending.clone() } else { Vec::new() };
                ViewInfo {
                    mode,
                    active_level: app.session.active_level_id(),
                    selection: match app.session.selection {
                        Selection::Element(id) => vec![id],
                        Selection::Site(id) => vec![id],
                        Selection::Foundation(id) => vec![id],
                        _ => vec![],
                    },
                    half_extent: plan_half_extent(app.session.plan.zoom, rect.width()),
                    ground_extent: 40.0,
                    show_ceilings_3d: app.show_ceilings_3d,
                    show_items_2d: app.show_items_2d,
                    light: t.is_light(),
                    show_formwork: app.session.show_formwork,
                    eye,
                    solo: app.session.solo_level,
                    xray: app.session.xray,
                    show_rebar: app.session.show_rebar,
                    tool: match app.session.tool {
                        bimcraft_engine::Tool::Select => bimcraft_render::PreviewTool::Select,
                        bimcraft_engine::Tool::Wall => bimcraft_render::PreviewTool::Wall,
                        bimcraft_engine::Tool::Door => bimcraft_render::PreviewTool::Door,
                        bimcraft_engine::Tool::Window => bimcraft_render::PreviewTool::Window,
                        bimcraft_engine::Tool::Floor => bimcraft_render::PreviewTool::Floor,
                        bimcraft_engine::Tool::Ceiling => bimcraft_render::PreviewTool::Ceiling,
                        bimcraft_engine::Tool::Item => bimcraft_render::PreviewTool::Item,
                        bimcraft_engine::Tool::Measure => bimcraft_render::PreviewTool::Measure,
                        bimcraft_engine::Tool::SectionLine => bimcraft_render::PreviewTool::SectionLine,
                        bimcraft_engine::Tool::Paint => bimcraft_render::PreviewTool::Paint,
                        bimcraft_engine::Tool::Foundation => bimcraft_render::PreviewTool::Foundation,
                        bimcraft_engine::Tool::Site => bimcraft_render::PreviewTool::Site,
                    },
                    cursor: app.session.cursor,
                    site_kind: app.session.site_kind,
                    item_kind: app.session.item_kind,
                    pending: app.session.pending.len(),
                    pending_pts,
                    ghost_host,
                    rotation: app.session.item_rotation,
                }
            };
            let dl = build(&app.session.model, &info);
            match app.session.view {
                ViewMode::Plan2D => draw_plan(ui, app, &painter, rect, &dl, &t),
                ViewMode::Perspective3D | ViewMode::Walk => draw_3d(ui, app, &painter, rect, &dl, &t),
                ViewMode::Section => unreachable!(),
            }
        }
        ViewMode::Section => {
            if let Some(sid) = app.session.active_or_first_section() {
                if let Some(sec) = app.session.model.section(sid).cloned() {
                    let dl = bimcraft_render::build_section(&app.session.model, &sec);
                    draw_section(ui, app, &painter, rect, &dl, &sec, &t);
                }
            } else {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No sections yet — pick the Section tool and click two points in the plan",
                    theme::font_ui(13.0),
                    t.text_dim,
                );
            }
        }
    }

    // Tool overlays (pending rubber-band etc.).
    draw_overlays(ui, app, &painter, rect, &t);
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

fn draw_plan(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, painter: &egui::Painter, rect: Rect, dl: &bimcraft_render::DisplayList, t: &Tokens) {
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
        t.text_dim,
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
    t: &Tokens,
) {
    let _ = ui;
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
        t.text_dim,
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
    /// Project a view-space point (x right, y up, z forward). Used by the line
    /// and triangle paths after explicit near-plane handling; no screen-bounds
    /// rejection here (egui clips to the viewport rect).
    fn project_view(&self, x: f64, y: f64, z: f64) -> (Pos2, f64) {
        (Pos2::new((self.cx + x / z * self.focal) as f32, (self.cy - y / z * self.focal) as f32), z)
    }

    fn point(&self, p: Vec3, eye: Vec3, r: Vec3, u: Vec3, f: Vec3) -> Option<(Pos2, f64)> {
        let d = p - eye;
        let x = d.dot(r);
        let y = d.dot(u);
        let z = d.dot(f);
        if z < 0.08 {
            return None;
        }
        let (sp, z) = self.project_view(x, y, z);
        // Generous cull for isolated points (texts): 3x viewport window.
        let (wf, hf) = (self.w as f32, self.h as f32);
        if sp.x < -wf || sp.x > wf * 2.0 || sp.y < -hf || sp.y > hf * 2.0 {
            return None;
        }
        Some((sp, z))
    }
}

/// Sutherland–Hodgman clip of a view-space polygon against z >= `near`.
fn clip_near(poly: &[(f64, f64, f64)], near: f64) -> Vec<(f64, f64, f64)> {
    let mut out = Vec::with_capacity(poly.len() + 1);
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        let a_in = a.2 >= near;
        let b_in = b.2 >= near;
        if a_in {
            out.push(a);
        }
        if a_in != b_in && b.2 != a.2 {
            let t = (near - a.2) / (b.2 - a.2);
            out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, near));
        }
    }
    out
}

fn draw_3d(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, painter: &egui::Painter, rect: Rect, dl: &bimcraft_render::DisplayList, t: &Tokens) {
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

    // Near-plane clip in view space (no screen-bounds rejection: the painter
    // clips to the viewport, and dropping whole triangles because one vertex
    // leaves the window made walls vanish).
    const NEAR: f64 = 0.08;
    let mut opaque: Vec<(Vec<Pos2>, Vec<f64>, f64, Color32)> = Vec::with_capacity(dl.tris.len());
    let mut translucent: Vec<(Vec<Pos2>, Vec<f64>, f64, Color32)> = Vec::new();
    for tri in &dl.tris {
        let mut poly: Vec<(f64, f64, f64)> = Vec::with_capacity(3);
        for p in &tri.pts {
            let d = *p - eye;
            poly.push((d.dot(r), d.dot(u), d.dot(f)));
        }
        let clipped = clip_near(&poly, NEAR);
        if clipped.len() < 3 {
            continue;
        }
        let pts: Vec<Pos2> = clipped.iter().map(|v| proj.project_view(v.0, v.1, v.2).0).collect();
        let zs: Vec<f64> = clipped.iter().map(|v| v.2).collect();
        let zavg = zs.iter().sum::<f64>() / zs.len() as f64;
        if tri.color.a() == 255 {
            opaque.push((pts, zs, zavg, tri.color));
        } else {
            translucent.push((pts, zs, zavg, tri.color));
        }
    }

    // Software z-buffer rasterization (parity with the JS edition's WebGL
    // depth buffer). The old painter sort by per-triangle AVERAGE depth
    // mis-ordered large wall quads against roof/floor slabs, so slab
    // triangulation fans bled through facades as diagonal shading bands.
    let frame = &mut app.viewport_frame;
    frame.ensure(rect, ui.ctx().pixels_per_point(), t.viewport_bg);
    for (pts, zs, _, color) in &opaque {
        for k in 1..pts.len().saturating_sub(1) {
            frame.raster([pts[0], pts[k], pts[k + 1]], [zs[0], zs[k], zs[k + 1]], *color, false);
        }
    }
    // Translucent faces (glass, formwork sheathing) blend over the opaque
    // scene, far to near, and do not occlude later depth tests.
    translucent.sort_by(|a, b| b.2.total_cmp(&a.2));
    for (pts, zs, _, color) in &translucent {
        for k in 1..pts.len().saturating_sub(1) {
            frame.raster([pts[0], pts[k], pts[k + 1]], [zs[0], zs[k], zs[k + 1]], *color, true);
        }
    }

    // Blit the finished frame as a texture at device resolution.
    let image = frame.image();
    let tex = app.viewport_texture.get_or_insert_with(|| ui.ctx().load_texture("bimcraft-3d-frame", image.clone(), egui::TextureOptions::LINEAR));
    tex.set(image, egui::TextureOptions::LINEAR);
    painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);

    // Lines with occlusion against the same depth buffer.
    for l in &dl.lines {
        draw_line_occluded(painter, &proj, frame, eye, r, u, f, l.a, l.b, l.color, l.width);
    }

    // Texts (projected; skip when behind camera or hidden by the model).
    for tx in &dl.texts {
        if let Some((sp, z)) = proj.point(tx.pos, eye, r, u, f) {
            if frame.visible_logical(sp.x, sp.y, z + 0.25) {
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
    painter.text(rect.right_top() + Vec2::new(-14.0, 14.0), egui::Align2::RIGHT_TOP, hint, theme::font_ui(11.0), t.text_dim);
}

/// Software color+depth framebuffer for the 3D viewport — the parity
/// equivalent of the JS edition's WebGL per-pixel z-buffer. Triangles pass an
/// exact per-pixel depth test, so large coplanar faces can no longer
/// interleave by average depth and bleed through each other.
///
/// Buffer pixels are DEVICE pixels (logical egui pixels × pixels-per-point);
/// the public raster/visible APIs take logical coordinates and scale
/// internally. The color buffer starts filled with the viewport background,
/// so the final image is fully opaque and blits without alpha surprises.
pub struct Frame3D {
    w: usize,
    h: usize,
    scale: f32,
    ox: f32,
    oy: f32,
    color: Vec<Color32>,
    depth: Vec<f32>,
}

impl Default for Frame3D {
    fn default() -> Self {
        Self { w: 0, h: 0, scale: 1.0, ox: 0.0, oy: 0.0, color: Vec::new(), depth: Vec::new() }
    }
}

impl Frame3D {
    /// Hard pixel budget (device px) so huge 4K/HiDPI viewports degrade
    /// gracefully instead of exploding fill cost.
    const MAX_PIXELS: usize = 9_000_000;

    fn ensure(&mut self, rect: Rect, ppp: f32, bg: Color32) {
        let mut scale = ppp.max(0.5);
        let mut w = ((rect.width() * scale).ceil() as usize).max(1);
        let mut h = ((rect.height() * scale).ceil() as usize).max(1);
        if w.saturating_mul(h) > Self::MAX_PIXELS {
            let k = ((Self::MAX_PIXELS as f64) / ((w * h) as f64)).sqrt();
            scale = (scale as f64 * k).max(0.25) as f32;
            w = ((rect.width() * scale).ceil() as usize).max(1);
            h = ((rect.height() * scale).ceil() as usize).max(1);
        }
        if self.w != w || self.h != h || self.color.len() != w * h {
            self.w = w;
            self.h = h;
            self.color = vec![bg; w * h];
            self.depth = vec![f32::INFINITY; w * h];
        } else {
            self.color.fill(bg);
            self.depth.fill(f32::INFINITY);
        }
        self.scale = scale;
        self.ox = rect.left();
        self.oy = rect.top();
    }

    /// Rasterize one triangle with an exact depth test. Screen points are
    /// logical egui pixels; depths are view-space meters. `blend` does a
    /// straight src-over onto the opaque buffer (for glass/formwork) and
    /// leaves the depth buffer untouched; opaque writes both.
    fn raster(&mut self, pts: [Pos2; 3], zs: [f64; 3], color: Color32, blend: bool) {
        let p0 = Pos2::new((pts[0].x - self.ox) * self.scale, (pts[0].y - self.oy) * self.scale);
        let p1 = Pos2::new((pts[1].x - self.ox) * self.scale, (pts[1].y - self.oy) * self.scale);
        let p2 = Pos2::new((pts[2].x - self.ox) * self.scale, (pts[2].y - self.oy) * self.scale);
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
        // Perspective-correct depth: interpolate 1/z (screen-linear), invert.
        let inv_z = [1.0 / zs[0], 1.0 / zs[1], 1.0 / zs[2]];
        // Incremental barycentric edge values, normalized by the signed area
        // so the inside test works in either winding.
        let w0_row = ((p1.y - p2.y) * (minx as f32 + 0.5 - p2.x) + (p2.x - p1.x) * (miny as f32 + 0.5 - p2.y)) * inv;
        let w1_row = ((p2.y - p0.y) * (minx as f32 + 0.5 - p2.x) + (p0.x - p2.x) * (miny as f32 + 0.5 - p2.y)) * inv;
        let w0_dx = (p1.y - p2.y) * inv;
        let w0_dy = (p2.x - p1.x) * inv;
        let w1_dx = (p2.y - p0.y) * inv;
        let w1_dy = (p0.x - p2.x) * inv;
        let (sr, sg, sb, sa) = (color.r() as u16, color.g() as u16, color.b() as u16, color.a() as u16);
        for y in miny..=maxy {
            let row = y * self.w;
            let mut w0 = w0_row + w0_dy * (y - miny) as f32;
            let mut w1 = w1_row + w1_dy * (y - miny) as f32;
            for x in minx..=maxx {
                let w2 = 1.0 - w0 - w1;
                let inside = (w0 >= -1e-4 && w1 >= -1e-4 && w2 >= -1e-4) || (w0 <= 1e-4 && w1 <= 1e-4 && w2 <= 1e-4);
                if inside {
                    let wi = (w0 as f64) * inv_z[0] + (w1 as f64) * inv_z[1] + (w2 as f64) * inv_z[2];
                    let z = if wi > 1e-9 { 1.0 / wi } else { f64::INFINITY };
                    let slot = &mut self.depth[row + x];
                    if (z as f32) < *slot {
                        let px = &mut self.color[row + x];
                        if blend && sa < 255 {
                            // Straight src-over over an opaque destination:
                            // out = src*a + dst*(1-a).
                            let ia = 255 - sa;
                            let mix = |d: u8, s: u16| ((d as u16 * ia + s * sa + 127) / 255) as u8;
                            *px = Color32::from_rgb(mix(px.r(), sr), mix(px.g(), sg), mix(px.b(), sb));
                        } else {
                            *px = color;
                        }
                        if !blend {
                            *slot = z as f32;
                        }
                    }
                }
                w0 += w0_dx;
                w1 += w1_dx;
            }
        }
    }

    /// Depth test at a logical-pixel position (lines, labels).
    fn visible_logical(&self, x: f32, y: f32, z: f64) -> bool {
        let xr = (x - self.ox) * self.scale;
        let yr = (y - self.oy) * self.scale;
        if xr < 0.0 || yr < 0.0 {
            return false;
        }
        let xi = (xr as usize).min(self.w - 1);
        let yi = (yr as usize).min(self.h - 1);
        z <= self.depth[yi * self.w + xi] as f64 + 0.045
    }

    fn image(&self) -> egui::ColorImage {
        egui::ColorImage::new([self.w, self.h], self.color.clone())
    }
}

/// Draw a 3D line, clipped at the near plane and occluded where the model's
/// depth buffer is in front of it (sampled every few screen pixels).
fn draw_line_occluded(
    painter: &egui::Painter,
    proj: &Proj,
    frame: &Frame3D,
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
    // Screen span decides the sample count (about one sample per 3 px).
    let (sa, _) = proj.project_view(ax, ay, az);
    let (sb, _) = proj.project_view(bx, by, bz);
    let span = ((sb.x - sa.x).abs().max((sb.y - sa.y).abs()) as usize).clamp(2, 512);
    let mut prev: Option<(Pos2, bool)> = None;
    for k in 0..=span {
        let t = k as f64 / span as f64;
        let vx = ax + (bx - ax) * t;
        let vy = ay + (by - ay) * t;
        let vz = az + (bz - az) * t;
        let (sp, z) = proj.project_view(vx, vy, vz);
        let vis = frame.visible_logical(sp.x, sp.y, z);
        if let Some((last, last_vis)) = prev {
            if vis && last_vis {
                painter.line_segment([last, sp], Stroke::new(width, color));
            }
        }
        prev = Some((sp, vis));
    }
}

// ------------------------------------------------------------ overlays ---

fn draw_overlays(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, painter: &egui::Painter, rect: Rect, t: &Tokens) {
    let _ = ui;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_near_keeps_straddling_triangle() {
        // One vertex behind the camera: the clip yields a quad in front.
        let tri = [(0.0, 0.0, 1.0), (1.0, 0.0, -1.0), (0.0, 1.0, 2.0)];
        let out = clip_near(&tri, 0.08);
        assert_eq!(out.len(), 4);
        assert!(out.iter().all(|v| v.2 >= 0.08));
        // Fully in front: unchanged; fully behind: empty.
        assert_eq!(clip_near(&[(0.0, 0.0, 1.0), (1.0, 0.0, 2.0), (0.0, 1.0, 3.0)], 0.08).len(), 3);
        assert!(clip_near(&[(0.0, 0.0, -1.0), (1.0, 0.0, -2.0), (0.0, 1.0, -3.0)], 0.08).is_empty());
    }

    #[test]
    fn frame3d_is_rect_relative_and_depth_sorts() {
        // The viewport sits offset from the window origin (app bar + dock):
        // occlusion must still index the right cells (the streak bug).
        let rect = Rect::from_min_size(Pos2::new(300.0, 50.0), egui::vec2(64.0, 64.0));
        let mut f = Frame3D::default();
        f.ensure(rect, 1.0, Color32::from_rgb(10, 20, 30));
        let (cx, cy) = (rect.center().x, rect.center().y);
        let near = [Pos2::new(cx - 10.0, cy + 10.0), Pos2::new(cx + 10.0, cy + 10.0), Pos2::new(cx, cy - 10.0)];
        // Near triangle (red), then a far triangle (green) covering the same
        // pixels: the z-buffer must keep the near color.
        f.raster(near, [2.0, 2.0, 2.0], Color32::from_rgb(200, 0, 0), false);
        f.raster(near, [6.0, 6.0, 6.0], Color32::from_rgb(0, 200, 0), false);
        assert_eq!(f.image().pixels[32 * 64 + 32], Color32::from_rgb(200, 0, 0));
        // Center: hidden behind the surface, visible at/near it.
        assert!(!f.visible_logical(cx, cy, 5.0));
        assert!(f.visible_logical(cx, cy, 2.04));
        // An offset-buffer bug would read the clamped edge instead.
        assert!(f.visible_logical(cx - 20.0, cy, 50.0));
    }

    #[test]
    fn frame3d_blends_translucent_without_depth_write() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(32.0, 32.0));
        let mut f = Frame3D::default();
        f.ensure(rect, 1.0, Color32::from_rgb(0, 0, 0));
        let tri = [Pos2::new(4.0, 24.0), Pos2::new(24.0, 24.0), Pos2::new(14.0, 6.0)];
        // Opaque white wall at 2 m, then translucent gray glass at 1 m.
        f.raster(tri, [2.0, 2.0, 2.0], Color32::from_rgb(255, 255, 255), false);
        f.raster(tri, [1.0, 1.0, 1.0], Color32::from_rgba_unmultiplied(0, 0, 0, 128), true);
        let px = f.image().pixels[16 * 32 + 16];
        // 255*(1-0.502) rounds to 127 with the +127 rounding bias.
        assert_eq!(px, Color32::from_rgb(127, 127, 127));
        // Glass must not write depth: an edge behind the glass stays visible.
        assert!(f.visible_logical(16.0, 16.0, 1.5));
        // …but a line behind the opaque wall is hidden.
        assert!(!f.visible_logical(16.0, 16.0, 2.5));
    }
}
