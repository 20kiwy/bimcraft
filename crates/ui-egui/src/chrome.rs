//! BimCraft chrome: app bar, left tool rail, right dock, status bar, dialogs.
//! Layout follows the VectorCraft pattern: 32 px app bar, 48 px single-column
//! tool rail, tabbed right dock, 24 px status bar. Icon buttons come from the
//! adopted `craft-ui` crate (storytold craft-libs, MIT OR Apache-2.0).

use crate::theme::{self, Tokens};
use bimcraft_bim::{ElementData, ElementKind, GridAxis, ItemKind};
use bimcraft_engine::{Selection, Tool, ViewMode};
use egui::{Color32, CornerRadius, Rect, Sense, Stroke, Vec2};

// ------------------------------------------------------------- app bar ----

pub fn app_bar(ui: &mut egui::Ui, app: &mut crate::BimcraftApp) {
    let t = theme::tokens();
    egui::Panel::top("app_bar").exact_size(34.0).resizable(false).frame(egui::Frame::NONE.fill(t.app_bar)).show(ui, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            brand_mark(ui, &t);
            ui.separator();
            let style = theme::icon_button_style(&t);
            bar_button(
                ui,
                &style,
                "New empty model",
                |ui, rect, color| file_icon(ui.painter(), rect.center(), color),
                || {
                    app.session_reset();
                },
            );
            bar_button(
                ui,
                &style,
                "Reload the sample house",
                |ui, rect, color| sample_icon(ui.painter(), rect.center(), color),
                || {
                    app.session_sample();
                },
            );
            if !cfg!(target_arch = "wasm32") {
                bar_button(
                    ui,
                    &style,
                    "Open a native .bimcraft.json file",
                    |ui, rect, color| open_icon(ui.painter(), rect.center(), color),
                    || {
                        app.show_open_dialog = true;
                    },
                );
                bar_button(
                    ui,
                    &style,
                    "Save the model to a native .bimcraft.json file",
                    |ui, rect, color| save_icon(ui.painter(), rect.center(), color),
                    || {
                        app.show_save_dialog = true;
                    },
                );
            }
            ui.separator();
            bar_button(
                ui,
                &style,
                "Undo (Ctrl+Z)",
                |ui, rect, color| undo_icon(ui.painter(), rect.center(), color),
                || {
                    app.session.undo();
                },
            );
            bar_button(
                ui,
                &style,
                "Redo (Ctrl+Y)",
                |ui, rect, color| redo_icon(ui.painter(), rect.center(), color),
                || {
                    app.session.redo();
                },
            );
            bar_button(
                ui,
                &style,
                "Delete selection (Del)",
                |ui, rect, color| trash_icon(ui.painter(), rect.center(), color),
                || {
                    app.session.delete_selected();
                },
            );
            bar_button(
                ui,
                &style,
                "Zoom to fit (Home)",
                |ui, rect, color| fit_icon(ui.painter(), rect.center(), color),
                || {
                    app.zoom_to_fit();
                },
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                mode_switch(ui, app, &t);
                ui.separator();
                snap_toggle(ui, app, &t);
            });
        });
    });
}

fn bar_button(
    ui: &mut egui::Ui,
    style: &craft_ui::buttons::IconButtonStyle,
    tip: &str,
    paint: impl FnOnce(&egui::Ui, Rect, Color32),
    mut action: impl FnMut(),
) {
    use craft_ui::buttons::IconButton;
    let resp = IconButton::new(tip, Vec2::new(24.0, 24.0), style).show(ui, paint);
    let resp = resp.on_hover_text(tip);
    if resp.clicked() {
        action();
    }
}

fn brand_mark(ui: &mut egui::Ui, t: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(20.0, 20.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(5), t.accent);
    p.line_segment([rect.center() - Vec2::new(5.0, -4.0), rect.center() + Vec2::new(5.0, 4.0)], Stroke::new(2.2, Color32::WHITE));
    p.line_segment([rect.center() - Vec2::new(0.0, -6.5), rect.center() - Vec2::new(0.0, 1.5)], Stroke::new(1.4, Color32::WHITE));
    ui.label(egui::RichText::new("BimCraft").font(theme::font_ui(14.0)).strong());
}

fn mode_switch(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    let style = theme::icon_button_style(t);
    use craft_ui::buttons::IconButton;
    let modes = [("2D Plan", ViewMode::Plan2D, "Top-down plan view (Tab)"), ("3D View", ViewMode::Perspective3D, "Orbiting 3D view (Tab)")];
    for (label, mode, tip) in modes {
        let active = app.session.view == mode;
        let resp = IconButton::new(tip, Vec2::new(64.0, 22.0), &style).selected(active).show(ui, |ui, rect, color| {
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, theme::font_ui(12.0), color);
        });
        let resp = resp.on_hover_text(tip);
        if resp.clicked() {
            app.session.set_view(mode);
        }
    }
}

fn snap_toggle(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    let style = theme::icon_button_style(t);
    use craft_ui::buttons::IconButton;
    let on = app.session.snap_grid;
    let tip = "Grid snap 0.5 m (G)";
    let resp = IconButton::new(tip, Vec2::new(28.0, 22.0), &style).selected(on).show(ui, |ui, rect, color| {
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "⌖ 0.5", theme::font_ui(11.0), color);
    });
    let resp = resp.on_hover_text(tip);
    if resp.clicked() {
        app.session.snap_grid = !on;
    }
}

// -------------------------------------------------------------- toolbar ---

pub const TOOLBAR_W: f32 = 48.0;

pub fn toolbar(ui: &mut egui::Ui, app: &mut crate::BimcraftApp) {
    let t = theme::tokens();
    egui::Panel::left("tool_rail").exact_size(TOOLBAR_W).resizable(false).frame(egui::Frame::NONE.fill(t.app_bar)).show(ui, |ui| {
        ui.add_space(6.0);
        let style = theme::icon_button_style(&t);
        use craft_ui::buttons::IconButton;
        let tools: [(Tool, &str); 9] = [
            (Tool::Select, "Select / move (S)"),
            (Tool::Wall, "Wall — chain polyline (W)"),
            (Tool::Door, "Place door (D)"),
            (Tool::Window, "Place window (N)"),
            (Tool::Floor, "Floor slab polygon (F)"),
            (Tool::Ceiling, "Ceiling polygon (C)"),
            (Tool::Item, "Place item (I)"),
            (Tool::Measure, "Measure distance (M)"),
            (Tool::SectionLine, "Section line — two clicks (X)"),
        ];
        for (tool, tip) in tools {
            let active = app.session.tool == tool;
            let resp = IconButton::new(tip, Vec2::new(34.0, 34.0), &style).selected(active).show(ui, |ui, rect, color| {
                draw_tool_icon(ui.painter(), rect.center(), tool, color);
            });
            let resp = resp.on_hover_text(tip);
            if resp.clicked() {
                app.session.set_tool(tool);
            }
        }
    });
}

/// Small vector glyphs for each tool, painted directly (no icon assets needed).
pub fn draw_tool_icon(p: &egui::Painter, c: egui::Pos2, tool: Tool, color: Color32) {
    let s = Stroke::new(1.6, color);
    let fill = Stroke::new(1.0, color);
    match tool {
        Tool::Select => {
            let pts = [
                c + Vec2::new(-4.0, -6.0),
                c + Vec2::new(-4.0, 5.0),
                c + Vec2::new(-1.0, 2.5),
                c + Vec2::new(1.5, 6.0),
                c + Vec2::new(3.0, 5.0),
                c + Vec2::new(0.5, 1.8),
                c + Vec2::new(4.0, 1.0),
            ];
            p.add(egui::Shape::convex_polygon(pts.to_vec(), color, Stroke::NONE));
        }
        Tool::Wall => {
            p.line_segment([c + Vec2::new(-6.0, -3.0), c + Vec2::new(6.0, -3.0)], s);
            p.line_segment([c + Vec2::new(-6.0, 3.0), c + Vec2::new(6.0, 3.0)], s);
            p.line_segment([c + Vec2::new(-6.0, -3.0), c + Vec2::new(-6.0, 3.0)], fill);
            p.line_segment([c + Vec2::new(6.0, -3.0), c + Vec2::new(6.0, 3.0)], fill);
        }
        Tool::Door => {
            p.line_segment([c + Vec2::new(-6.0, 5.0), c + Vec2::new(6.0, 5.0)], s);
            p.line_segment([c + Vec2::new(-6.0, 5.0), c + Vec2::new(-6.0, -5.0)], s);
            let mut prev = c + Vec2::new(-6.0, -5.0);
            for k in 1..=6 {
                let ang = std::f32::consts::FRAC_PI_2 * (k as f32 / 6.0);
                let pt = c + Vec2::new(-6.0 + 12.0 * (1.0 - ang.cos()), 5.0 - 10.0 * ang.sin());
                p.line_segment([prev, pt], fill);
                prev = pt;
            }
        }
        Tool::Window => {
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(12.0, 8.0)), CornerRadius::same(1), s, egui::StrokeKind::Inside);
            p.line_segment([c + Vec2::new(-6.0, 0.0), c + Vec2::new(6.0, 0.0)], fill);
            p.line_segment([c, c + Vec2::new(0.0, 4.0)], fill);
        }
        Tool::Floor => {
            let pts = [c + Vec2::new(-6.0, 2.0), c + Vec2::new(-1.0, -5.0), c + Vec2::new(6.0, -2.0), c + Vec2::new(1.0, 5.0)];
            p.add(egui::Shape::convex_polygon(pts.to_vec(), color.gamma_multiply(0.35), s));
        }
        Tool::Ceiling => {
            let pts = [c + Vec2::new(-6.0, 2.0), c + Vec2::new(-1.0, -5.0), c + Vec2::new(6.0, -2.0), c + Vec2::new(1.0, 5.0)];
            p.add(egui::Shape::convex_polygon(pts.to_vec(), Color32::TRANSPARENT, s));
        }
        Tool::Item => {
            p.line_segment([c + Vec2::new(-4.0, 5.0), c + Vec2::new(-4.0, -2.0)], s);
            p.line_segment([c + Vec2::new(-4.0, 0.0), c + Vec2::new(2.0, 0.0)], s);
            p.line_segment([c + Vec2::new(2.0, 0.0), c + Vec2::new(2.0, 5.0)], s);
            p.line_segment([c + Vec2::new(-4.0, 5.0), c + Vec2::new(2.0, 5.0)], s);
            p.line_segment([c + Vec2::new(-4.0, -2.0), c + Vec2::new(2.0, -5.0)], s);
        }
        Tool::Measure => {
            p.line_segment([c + Vec2::new(-6.0, 4.0), c + Vec2::new(6.0, -4.0)], s);
            p.line_segment([c + Vec2::new(-6.0, 4.0), c + Vec2::new(-4.5, 5.0)], fill);
            p.line_segment([c + Vec2::new(-2.0, 2.0), c + Vec2::new(-0.5, 3.0)], fill);
            p.line_segment([c + Vec2::new(2.0, 0.0), c + Vec2::new(3.5, 1.0)], fill);
            p.line_segment([c + Vec2::new(6.0, -4.0), c + Vec2::new(4.5, -5.0)], fill);
        }
        Tool::SectionLine => {
            // Section marker: cut line with end flags.
            p.line_segment([c + Vec2::new(-6.0, 0.0), c + Vec2::new(6.0, 0.0)], s);
            p.line_segment([c + Vec2::new(-6.0, 0.0), c + Vec2::new(-6.0, -5.0)], fill);
            p.line_segment([c + Vec2::new(-6.0, -5.0), c + Vec2::new(-3.0, -5.0)], fill);
            p.line_segment([c + Vec2::new(6.0, 0.0), c + Vec2::new(6.0, 5.0)], fill);
            p.line_segment([c + Vec2::new(6.0, 5.0), c + Vec2::new(3.0, 5.0)], fill);
        }
    }
}

// ----------------------------------------------------------------- dock ---

pub fn dock(ui: &mut egui::Ui, app: &mut crate::BimcraftApp) {
    let t = theme::tokens();
    egui::Panel::right("dock").exact_size(288.0).resizable(false).frame(egui::Frame::NONE.fill(t.panel)).show(ui, |ui| {
        // Tab row.
        ui.horizontal(|ui| {
            for tab in crate::DockTab::ALL {
                let active = app.dock_tab == tab;
                let txt = egui::RichText::new(tab.label()).font(theme::font_ui(11.5)).color(if active { Color32::WHITE } else { t.text_dim });
                if ui.add(egui::Button::new(txt).fill(if active { t.tool_active } else { Color32::TRANSPARENT })).clicked() {
                    app.dock_tab = tab;
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match app.dock_tab {
            crate::DockTab::Properties => properties_ui(ui, app, &t),
            crate::DockTab::Levels => levels_ui(ui, app, &t),
            crate::DockTab::Library => library_ui(ui, app, &t),
            crate::DockTab::Schedule => schedule_ui(ui, app, &t),
            crate::DockTab::Model => model_ui(ui, app, &t),
        });
    });
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new(title).font(theme::font_ui(11.0)).strong().color(theme::tokens().text_dim));
    ui.separator();
}

fn drag_f64(ui: &mut egui::Ui, label: &str, value: &mut f64, speed: f64, range: std::ops::RangeInclusive<f64>, suffix: &str) -> bool {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).font(theme::font_ui(12.0)));
        let r = ui.add(egui::DragValue::new(value).speed(speed).range(range).suffix(suffix));
        r.changed()
    })
    .inner
}

fn properties_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    match app.session.selection {
        Selection::None => {
            ui.label(egui::RichText::new("Nothing selected").font(theme::font_ui(12.0)).color(t.text_dim));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Pick an element in the plan:\n· walls, doors, windows, floors, ceilings, items\n· or a datum grid line")
                    .font(theme::font_ui(11.0))
                    .color(t.text_dim),
            );
        }
        Selection::Level(id) => level_editor(ui, app, t, id),
        Selection::Grid(id) => grid_editor(ui, app, t, id),
        Selection::Element(id) => element_editor(ui, app, t, id),
        Selection::Section(id) => section_editor(ui, app, t, id),
    }
}

fn section_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64) {
    let Some(sec) = app.session.model.section(id).cloned() else {
        ui.label("Section not found");
        return;
    };
    let _ = t;
    section(ui, "Section");
    ui.horizontal(|ui| {
        ui.label("Name");
        let mut name = sec.name.clone();
        if ui.text_edit_singleline(&mut name).lost_focus() && name != sec.name {
            app.session.begin_edit();
            if let Some(s) = app.session.model.section_mut(id) {
                s.name = name;
            }
        }
    });
    let mut depth = sec.depth;
    let ch_depth = drag_f64(ui, "View depth", &mut depth, 0.25, 0.5..=30.0, " m");
    if ch_depth {
        app.session.begin_edit();
        if let Some(s) = app.session.model.section_mut(id) {
            s.depth = depth;
        }
    }
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(format!(
            "Length {:.2} m · level {}",
            sec.length(),
            app.session.model.level(sec.level).map(|l| l.name.as_str()).unwrap_or("?")
        ))
        .font(theme::font_ui(11.0))
        .color(t.text_dim),
    );
    ui.add_space(4.0);
    if ui.button("Open section view").clicked() {
        app.session.active_section = Some(id);
        app.session.set_view(ViewMode::Section);
    }
    ui.label(egui::RichText::new("Del: delete section").font(theme::font_ui(11.0)).color(t.text_dim));
}

fn level_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64) {
    let Some(level) = app.session.model.level(id).cloned() else {
        ui.label("Level not found");
        return;
    };
    section(ui, "Level");
    ui.horizontal(|ui| {
        ui.label("Name");
        let mut name = level.name.clone();
        if ui.text_edit_singleline(&mut name).lost_focus() && name != level.name {
            app.session.begin_edit();
            if let Some(l) = app.session.model.level_mut(id) {
                l.name = name;
            }
        }
    });
    let mut elev = level.elevation;
    let mut height = level.height;
    let ch_elev = drag_f64(ui, "Elevation", &mut elev, 0.05, -100.0..=1000.0, " m");
    let ch_h = drag_f64(ui, "Height", &mut height, 0.05, 1.0..=20.0, " m");
    if ch_elev || ch_h {
        app.session.begin_edit();
        if let Some(l) = app.session.model.level_mut(id) {
            l.elevation = elev;
            l.height = height;
        }
    }
    if ui.add(egui::Button::new(egui::RichText::new("Set as active level")).min_size(Vec2::new(ui.available_width(), 0.0))).clicked() {
        app.session.set_active_level(id);
    }
    if ui.add(egui::Button::new(egui::RichText::new("Delete").color(t.danger))).clicked() && app.session.model.levels.len() > 1 {
        app.session.begin_edit();
        app.session.model.levels.retain(|l| l.id != id);
        app.session.model.elements.retain(|e| e.level_id() != Some(id));
        if app.session.active_level == Some(id) {
            app.session.active_level = app.session.model.levels.first().map(|l| l.id);
        }
        app.session.select(Selection::None);
    }
}

fn grid_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64) {
    let Some(idx) = app.session.model.grids.iter().position(|g| g.id == id) else {
        ui.label("Grid not found");
        return;
    };
    let g = app.session.model.grids[idx].clone();
    section(ui, "Datum grid");
    ui.horizontal(|ui| {
        ui.label("Name");
        let mut name = g.name.clone();
        if ui.text_edit_singleline(&mut name).lost_focus() && name != g.name {
            app.session.begin_edit();
            app.session.model.grids[idx].name = name;
        }
    });
    ui.label(format!("Axis: {}", if g.axis == GridAxis::V { "V (runs along Y)" } else { "H (runs along X)" }));
    let mut pos = g.position;
    if drag_f64(ui, "Position", &mut pos, 0.1, -200.0..=200.0, " m") {
        app.session.begin_edit();
        app.session.model.grids[idx].position = pos;
    }
    if ui.add(egui::Button::new(egui::RichText::new("Delete").color(t.danger))).clicked() {
        app.session.delete_selected();
    }
}

fn element_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64) {
    let Some(el) = app.session.model.element(id).cloned() else {
        app.session.select(Selection::None);
        return;
    };
    section(ui, &el.name);
    let mut changed = false;
    match &el.data {
        ElementData::Wall(w) => {
            let len = w.total_length();
            ui.label(
                egui::RichText::new(format!("{} segments · length {:.2} m", w.pts.len().saturating_sub(1), len))
                    .font(theme::font_mono(11.0))
                    .color(t.text_dim),
            );
            let mut thk = w.thickness;
            let mut h = w.height;
            changed |= drag_f64(ui, "Thickness", &mut thk, 0.01, 0.05..=2.0, " m");
            changed |= drag_f64(ui, "Height", &mut h, 0.05, 0.5..=20.0, " m");
            if changed {
                app.session.begin_edit();
                if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                    w.thickness = thk;
                    w.height = h;
                }
            }
        }
        ElementData::Opening(o) => {
            let wall_len = app.session.model.wall(o.wall).map(|w| w.total_length()).unwrap_or(0.0);
            let kind_label = if el.kind == ElementKind::Door { "Door" } else { "Window" };
            ui.label(egui::RichText::new(format!("{kind_label} hosted on a wall")).font(theme::font_ui(11.0)).color(t.text_dim));
            let mut width = o.width;
            let mut height = o.height;
            let mut sill = o.sill;
            changed |= drag_f64(ui, "Width", &mut width, 0.01, 0.3..=4.0, " m");
            changed |= drag_f64(ui, "Height", &mut height, 0.01, 0.3..=4.0, " m");
            if el.kind == ElementKind::Window {
                changed |= drag_f64(ui, "Sill", &mut sill, 0.01, 0.0..=2.5, " m");
            }
            // Shift along host wall (the "windows to be shifted" feature).
            let mut s = o.s;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Shift").font(theme::font_ui(12.0)));
                let slider = ui.add(egui::Slider::new(&mut s, 0.0..=wall_len.max(0.1)).text("along wall").suffix(" m"));
                changed |= slider.changed();
            });
            if changed {
                app.session.begin_edit();
                if let Some(ElementData::Opening(o)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                    o.width = width;
                    o.height = height;
                    o.sill = sill;
                    o.s = s;
                }
                // Re-clamp via the model API.
                let s_final = app.session.model.element(id).and_then(|e| match &e.data {
                    ElementData::Opening(o) => Some(o.s),
                    _ => None,
                });
                if let Some(s) = s_final {
                    app.session.model.shift_opening(id, s);
                }
            }
            ui.label(
                egui::RichText::new("Tip: drag the opening in the plan to slide it along the wall.").font(theme::font_ui(10.5)).color(t.text_dim),
            );
        }
        ElementData::Slab(_) | ElementData::Ceiling(_) => {
            let (pts, mut thk, mut elev, area) = match &el.data {
                ElementData::Slab(s) => (s.pts.clone(), s.thickness, s.elevation, s.area()),
                ElementData::Ceiling(c) => (c.pts.clone(), c.thickness, c.elevation, c.area()),
                _ => unreachable!(),
            };
            ui.label(
                egui::RichText::new(format!("Polygon · {} points · area {:.2} m²", pts.len(), area)).font(theme::font_mono(11.0)).color(t.text_dim),
            );
            changed |= drag_f64(ui, "Thickness", &mut thk, 0.01, 0.02..=2.0, " m");
            changed |= drag_f64(ui, "Elevation", &mut elev, 0.02, -2.0..=30.0, " m");
            if changed {
                app.session.begin_edit();
                match app.session.model.element_mut(id).map(|e| &mut e.data) {
                    Some(ElementData::Slab(s)) => {
                        s.thickness = thk;
                        s.elevation = elev;
                    }
                    Some(ElementData::Ceiling(c)) => {
                        c.thickness = thk;
                        c.elevation = elev;
                    }
                    _ => {}
                }
            }
        }
        ElementData::Item(i) => {
            ui.label(
                egui::RichText::new(format!(
                    "{} · footprint {:.2} × {:.2} × {:.2} m",
                    i.kind.name(),
                    i.kind.preset().0,
                    i.kind.preset().1,
                    i.kind.preset().2
                ))
                .font(theme::font_mono(11.0))
                .color(t.text_dim),
            );
            let mut rot_deg = i.rotation.to_degrees();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Rotate").font(theme::font_ui(12.0)));
                let sl = ui.add(egui::Slider::new(&mut rot_deg, -180.0..=180.0).suffix("°"));
                changed |= sl.changed();
            });
            if changed {
                app.session.begin_edit();
                if let Some(ElementData::Item(it)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                    it.rotation = rot_deg.to_radians();
                }
            }
            ui.label(egui::RichText::new("R rotates 90° · Shift+R rotates 15°").font(theme::font_ui(10.5)).color(t.text_dim));
        }
    }
    ui.add_space(4.0);
    if ui.add(egui::Button::new(egui::RichText::new("Delete").color(t.danger))).clicked() {
        app.session.delete_selected();
    }
}

fn levels_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Levels");
    let levels = app.session.model.levels.clone();
    for l in &levels {
        let is_active = Some(l.id) == app.session.active_level;
        let txt = format!("{}   elev {:.2} m   h {:.1} m", l.name, l.elevation, l.height);
        let rich = egui::RichText::new(txt).font(theme::font_ui(12.0)).color(if is_active { Color32::WHITE } else { t.text_dim });
        let resp =
            ui.add(egui::Button::new(rich).fill(if is_active { t.tool_active } else { t.input }).min_size(Vec2::new(ui.available_width(), 0.0)));
        if resp.clicked() {
            app.session.set_active_level(l.id);
            app.session.select(Selection::Level(l.id));
            app.dock_tab = crate::DockTab::Properties;
        }
        resp.on_hover_text("Click to activate · edit in Properties");
    }
    if ui.add(egui::Button::new("+ Add level").min_size(Vec2::new(ui.available_width(), 0.0))).clicked() {
        app.session.add_level();
    }
    ui.add_space(10.0);
    section(ui, "View toggles");
    ui.checkbox(&mut app.show_items_2d, "Show items in plan");
    ui.checkbox(&mut app.show_ceilings_3d, "Show ceilings in 3D");
    ui.add_space(10.0);
    section(ui, "Snapping");
    ui.checkbox(&mut app.session.snap_grid, "Grid snap (0.5 m, G)");
    ui.checkbox(&mut app.session.snap_endpoints, "Endpoint snap");
}

fn library_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Item library");
    ui.label(egui::RichText::new("Pick a kind, then click in the plan to place. R rotates.").font(theme::font_ui(11.0)).color(t.text_dim));
    ui.add_space(4.0);
    egui::Grid::new("item_library").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        for kind in ItemKind::ALL {
            let (w, d, h) = kind.preset();
            let active = app.session.item_kind == kind && app.session.tool == Tool::Item;
            let txt = format!("{}  {:.1}×{:.1}×{:.1}", kind.name(), w, d, h);
            let rich = egui::RichText::new(txt).font(theme::font_ui(11.5)).color(if active { Color32::WHITE } else { t.text });
            if ui.add(egui::Button::new(rich).fill(if active { t.tool_active } else { t.input }).min_size(Vec2::new(128.0, 0.0))).clicked() {
                app.session.item_kind = kind;
                app.session.set_tool(Tool::Item);
            }
        }
    });
    let _ = t;
}

fn schedule_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, _t: &Tokens) {
    section(ui, "Quantity schedule");
    let rows = app.session.model.schedule();
    if rows.is_empty() {
        ui.label("Nothing to schedule yet.");
        return;
    }
    ui.push_id("schedule_table", |ui| {
        egui::Grid::new("schedule_rows").num_columns(3).spacing([12.0, 2.0]).striped(true).show(ui, |ui| {
            ui.label(egui::RichText::new("element").font(theme::font_ui(11.0)).strong());
            ui.label(egui::RichText::new("count").font(theme::font_ui(11.0)).strong());
            ui.label(egui::RichText::new("quantity").font(theme::font_ui(11.0)).strong());
            ui.end_row();
            let mut last_group = String::new();
            for r in &rows {
                if r.group != last_group {
                    ui.label(egui::RichText::new(&r.group).font(theme::font_ui(11.0)).color(theme::tokens().accent));
                    ui.label("");
                    ui.label("");
                    ui.end_row();
                    last_group = r.group.clone();
                }
                ui.label(egui::RichText::new(&r.name).font(theme::font_ui(11.5)));
                ui.label(egui::RichText::new(format!("{}", r.count)).font(theme::font_mono(11.5)));
                ui.label(egui::RichText::new(format!("{:.2} {}", r.quantity, r.unit)).font(theme::font_mono(11.5)));
                ui.end_row();
            }
        });
    });
    ui.add_space(6.0);
    ui.label(egui::RichText::new("CSV export: bimcraft-cli schedule <file> --csv").font(theme::font_ui(10.5)).color(theme::tokens().text_dim));
}

fn model_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Datum grids");
    let grids = app.session.model.grids.clone();
    for g in &grids {
        let label = format!("{}  {}  @ {:.1} m", g.name, if g.axis == GridAxis::V { "V" } else { "H" }, g.position);
        if ui.selectable_label(app.session.selection == Selection::Grid(g.id), label).clicked() {
            app.session.select(Selection::Grid(g.id));
            app.dock_tab = crate::DockTab::Properties;
        }
    }
    ui.horizontal(|ui| {
        let cx = (app.session.cursor.x * 2.0).round() / 2.0;
        let cy = (app.session.cursor.y * 2.0).round() / 2.0;
        if ui.add(egui::Button::new("+ V grid")).on_hover_text(format!("Add vertical grid at X = {cx:.1} m (cursor)")).clicked() {
            let n = app.session.model.grids.iter().filter(|g| g.axis == GridAxis::V).count();
            app.session.add_grid(&letter_name(n), GridAxis::V, cx);
        }
        if ui.add(egui::Button::new("+ H grid")).on_hover_text(format!("Add horizontal grid at Y = {cy:.1} m (cursor)")).clicked() {
            let n = app.session.model.grids.iter().filter(|g| g.axis == GridAxis::H).count();
            app.session.add_grid(&(n + 1).to_string(), GridAxis::H, cy);
        }
    });

    let kinds = [
        (ElementKind::Wall, "Walls"),
        (ElementKind::Door, "Doors"),
        (ElementKind::Window, "Windows"),
        (ElementKind::Slab, "Floors"),
        (ElementKind::Ceiling, "Ceilings"),
        (ElementKind::Item, "Items"),
    ];
    ui.add_space(10.0);
    section(ui, "Model tree");
    for (kind, title) in kinds {
        let count = app.session.model.elements.iter().filter(|e| e.kind == kind).count();
        egui::CollapsingHeader::new(egui::RichText::new(format!("{title} ({count})")).font(theme::font_ui(12.0))).default_open(false).show(
            ui,
            |ui| {
                let elements = app.session.model.elements.clone();
                for e in elements.iter().filter(|e| e.kind == kind) {
                    let selected = app.session.selection == Selection::Element(e.id);
                    if ui.selectable_label(selected, e.name.clone()).clicked() {
                        app.session.select(Selection::Element(e.id));
                        app.dock_tab = crate::DockTab::Properties;
                    }
                }
            },
        );
    }
    let _ = t;
}

fn letter_name(mut n: usize) -> String {
    let mut out = String::new();
    loop {
        out.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out
}

// ----------------------------------------------------------- status bar ---

pub fn status_bar(ui: &mut egui::Ui, app: &mut crate::BimcraftApp) {
    let t = theme::tokens();
    egui::Panel::bottom("status_bar").exact_size(24.0).resizable(false).frame(egui::Frame::NONE.fill(t.app_bar)).show(ui, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            let mode = match app.session.view {
                ViewMode::Plan2D => "2D Plan",
                ViewMode::Perspective3D => "3D View",
                ViewMode::Section => "Section",
            };
            ui.label(egui::RichText::new(mode).font(theme::font_ui(11.0)).color(t.accent).strong());
            ui.separator();
            ui.label(egui::RichText::new(app.session.tool.label()).font(theme::font_ui(11.0)).color(t.text));
            ui.separator();
            ui.label(egui::RichText::new(app.session.tool.hint()).font(theme::font_ui(11.0)).color(t.text_dim));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(app.session.status.clone()).font(theme::font_ui(11.0)).color(t.text));
                ui.separator();
                let n = app.session.model.elements.len();
                ui.label(egui::RichText::new(format!("{n} elements")).font(theme::font_ui(11.0)).color(t.text_dim));
                ui.separator();
                let snap = if app.session.snap_grid { "snap 0.5" } else { "free" };
                ui.label(egui::RichText::new(snap).font(theme::font_ui(11.0)).color(if app.session.snap_grid { t.accent } else { t.text_dim }));
                ui.separator();
                let (x, y, z) = (app.session.cursor.x, app.session.cursor.y, app.session.cursor.z);
                ui.label(egui::RichText::new(format!("X {x:6.2}  Y {y:6.2}  Z {z:5.2}")).font(theme::font_mono(11.0)).color(t.text_dim));
            });
        });
    });
}

// -------------------------------------------------------------- dialogs ---

pub fn file_dialogs(ctx: &egui::Context, app: &mut crate::BimcraftApp) {
    let t = theme::tokens();
    if app.show_save_dialog {
        let mut open = std::mem::take(&mut app.show_save_dialog);
        egui::Window::new("Save model").open(&mut open).resizable(false).show(ctx, |ui| {
            ui.label(egui::RichText::new("Native JSON format (.bimcraft.json)").font(theme::font_ui(11.0)).color(t.text_dim));
            ui.horizontal(|ui| {
                ui.label("Path");
                ui.add(egui::TextEdit::singleline(&mut app.save_path).desired_width(220.0));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("Save")).clicked() {
                    let path = app.save_path.clone();
                    app.save_file(&path);
                }
                if ui.button("Cancel").clicked() {
                    app.show_save_dialog = false;
                }
            });
        });
        app.show_save_dialog = open && app.show_save_dialog;
    }
    if app.show_open_dialog {
        let mut open = std::mem::take(&mut app.show_open_dialog);
        egui::Window::new("Open model").open(&mut open).resizable(false).show(ctx, |ui| {
            ui.label(egui::RichText::new("Native JSON format (.bimcraft.json)").font(theme::font_ui(11.0)).color(t.text_dim));
            ui.horizontal(|ui| {
                ui.label("Path");
                ui.add(egui::TextEdit::singleline(&mut app.save_path).desired_width(220.0));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("Open")).clicked() {
                    let path = app.save_path.clone();
                    app.open_file(&path);
                }
                if ui.button("Cancel").clicked() {
                    app.show_open_dialog = false;
                }
            });
        });
        app.show_open_dialog = open && app.show_open_dialog;
    }
}

// --------------------------------------------------------- bar icons ------

fn file_icon(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.5, color);
    p.add(egui::Shape::line(
        vec![
            c + Vec2::new(-4.0, -6.0),
            c + Vec2::new(2.0, -6.0),
            c + Vec2::new(4.0, -4.0),
            c + Vec2::new(4.0, 6.0),
            c + Vec2::new(-4.0, 6.0),
            c + Vec2::new(-4.0, -6.0),
        ],
        s,
    ));
    p.add(egui::Shape::line(vec![c + Vec2::new(2.0, -6.0), c + Vec2::new(2.0, -4.0), c + Vec2::new(4.0, -4.0)], s));
}

fn sample_icon(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.4, color);
    let house = [c + Vec2::new(-6.0, 2.0), c + Vec2::new(0.0, -5.0), c + Vec2::new(6.0, 2.0)];
    p.line_segment([house[0], house[1]], s);
    p.line_segment([house[1], house[2]], s);
    p.line_segment([c + Vec2::new(-4.0, 2.0), c + Vec2::new(-4.0, 6.0)], s);
    p.line_segment([c + Vec2::new(4.0, 2.0), c + Vec2::new(4.0, 6.0)], s);
    p.line_segment([c + Vec2::new(-4.0, 6.0), c + Vec2::new(4.0, 6.0)], s);
}

fn open_icon(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.5, color);
    p.add(egui::Shape::line(vec![c + Vec2::new(-6.0, -4.0), c + Vec2::new(-2.0, -4.0), c + Vec2::new(0.0, -2.0), c + Vec2::new(6.0, -2.0)], s));
    p.add(egui::Shape::line(vec![c + Vec2::new(-6.0, -4.0), c + Vec2::new(-6.0, 5.0), c + Vec2::new(-4.0, 5.0)], s));
    p.add(egui::Shape::line(
        vec![c + Vec2::new(-4.0, 5.0), c + Vec2::new(-1.0, 0.0), c + Vec2::new(6.0, 0.0), c + Vec2::new(3.0, 5.0), c + Vec2::new(-4.0, 5.0)],
        s,
    ));
}

fn save_icon(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.5, color);
    p.add(egui::Shape::line(
        vec![
            c + Vec2::new(-5.0, -5.0),
            c + Vec2::new(3.0, -5.0),
            c + Vec2::new(5.0, -3.0),
            c + Vec2::new(5.0, 5.0),
            c + Vec2::new(-5.0, 5.0),
            c + Vec2::new(-5.0, -5.0),
        ],
        s,
    ));
    p.add(egui::Shape::line(vec![c + Vec2::new(-3.0, -5.0), c + Vec2::new(-3.0, -1.0), c + Vec2::new(3.0, -1.0), c + Vec2::new(3.0, -5.0)], s));
    p.add(egui::Shape::line(vec![c + Vec2::new(-3.0, 5.0), c + Vec2::new(-3.0, 1.0), c + Vec2::new(3.0, 1.0), c + Vec2::new(3.0, 5.0)], s));
}

fn undo_icon(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.6, color);
    p.line_segment([c + Vec2::new(-5.0, -2.0), c + Vec2::new(2.0, -2.0)], s);
    p.line_segment([c + Vec2::new(-5.0, -2.0), c + Vec2::new(-1.5, -5.5)], s);
    p.line_segment([c + Vec2::new(-5.0, -2.0), c + Vec2::new(-1.5, 1.5)], s);
    p.add(egui::Shape::line(vec![c + Vec2::new(2.0, -2.0), c + Vec2::new(5.0, 1.0), c + Vec2::new(5.0, 4.0)], s));
}

fn redo_icon(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.6, color);
    p.line_segment([c + Vec2::new(5.0, -2.0), c + Vec2::new(-2.0, -2.0)], s);
    p.line_segment([c + Vec2::new(5.0, -2.0), c + Vec2::new(1.5, -5.5)], s);
    p.line_segment([c + Vec2::new(5.0, -2.0), c + Vec2::new(1.5, 1.5)], s);
    p.add(egui::Shape::line(vec![c + Vec2::new(-2.0, -2.0), c + Vec2::new(-5.0, 1.0), c + Vec2::new(-5.0, 4.0)], s));
}

fn trash_icon(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.5, color);
    p.line_segment([c + Vec2::new(-4.0, -4.0), c + Vec2::new(4.0, -4.0)], s);
    p.add(egui::Shape::line(vec![c + Vec2::new(-2.0, -4.0), c + Vec2::new(-2.0, -6.0), c + Vec2::new(2.0, -6.0), c + Vec2::new(2.0, -4.0)], s));
    p.add(egui::Shape::line(vec![c + Vec2::new(-4.0, -4.0), c + Vec2::new(-3.0, 6.0), c + Vec2::new(3.0, 6.0), c + Vec2::new(4.0, -4.0)], s));
    p.line_segment([c + Vec2::new(-1.0, -1.0), c + Vec2::new(-1.0, 3.0)], s);
    p.line_segment([c + Vec2::new(1.0, -1.0), c + Vec2::new(1.0, 3.0)], s);
}

fn fit_icon(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.5, color);
    let corners = [c + Vec2::new(-6.0, -5.0), c + Vec2::new(6.0, -5.0), c + Vec2::new(6.0, 5.0), c + Vec2::new(-6.0, 5.0)];
    for (i, corner) in corners.iter().enumerate() {
        let next = corners[(i + 1) % 4];
        let mid = *corner + (next - *corner) * 0.5;
        p.line_segment([*corner + (mid - *corner) * 0.2, mid - (mid - *corner) * 0.2], s);
    }
    p.line_segment([c + Vec2::new(-2.0, 2.0), c + Vec2::new(2.0, -2.0)], s);
}
