//! BimCraft chrome: app bar, left tool rail, right dock, status bar, dialogs.
//! Layout follows the VectorCraft pattern: 32 px app bar, 48 px single-column
//! tool rail, tabbed right dock, 24 px status bar. Icon buttons come from the
//! adopted `craft-ui` crate (storytold craft-libs, MIT OR Apache-2.0).

use crate::theme::{self, Theme, Tokens};
use bimcraft_bim::{ElementData, ElementKind, FoundationKind, GridAxis, ItemKind};
use bimcraft_engine::{Selection, Tool, ViewMode};
use egui::{Color32, CornerRadius, Rect, Sense, Stroke, Vec2};

// ------------------------------------------------------------- app bar ----

pub fn app_bar(ui: &mut egui::Ui, app: &mut crate::BimcraftApp) {
    let t = theme::tokens_for(app.theme);
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
                theme_picker(ui, app, &t);
                ui.separator();
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
    let modes = [
        ("2D Plan", ViewMode::Plan2D, "Top-down plan view (Tab)"),
        ("3D View", ViewMode::Perspective3D, "Orbiting 3D view (Tab)"),
        ("Walk", ViewMode::Walk, "First-person walkthrough (V)"),
    ];
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

/// VectorCraft brightness levels: Dark, Charcoal (default), Slate, Light.
fn theme_picker(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    let style = theme::icon_button_style(t);
    use craft_ui::buttons::IconButton;
    let tip = "UI theme (VectorCraft brightness levels: Dark · Charcoal · Slate · Light)";
    let resp = IconButton::new(tip, Vec2::new(64.0, 22.0), &style).show(ui, |ui, rect, color| {
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, app.theme.label(), theme::font_ui(11.5), color);
        // Tiny sun/moon mark: dark themes get a moon, light themes a sun.
        let mark_c = rect.left_center() + egui::vec2(9.0, 0.0);
        if app.theme.is_light() {
            ui.painter().circle_filled(mark_c, 3.0, color);
            for k in 0..8 {
                let a = k as f32 / 8.0 * std::f32::consts::TAU;
                let d = egui::vec2(a.cos(), a.sin()) * 5.0;
                ui.painter().line_segment([mark_c + d * 0.7, mark_c + d], egui::Stroke::new(1.0, color));
            }
        } else {
            ui.painter().circle_filled(mark_c, 3.2, color);
            ui.painter().circle_filled(mark_c + egui::vec2(1.8, -1.2), 2.8, t.app_bar);
        }
    });
    let resp = resp.on_hover_text(tip);
    if resp.clicked() {
        let idx = Theme::ALL.iter().position(|x| *x == app.theme).unwrap_or(1);
        app.theme = Theme::ALL[(idx + 1) % Theme::ALL.len()];
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
    let t = theme::tokens_for(app.theme);
    egui::Panel::left("tool_rail").exact_size(TOOLBAR_W).resizable(false).frame(egui::Frame::NONE.fill(t.app_bar)).show(ui, |ui| {
        ui.add_space(6.0);
        let style = theme::icon_button_style(&t);
        use craft_ui::buttons::IconButton;
        let tools: [(Tool, &str); 12] = [
            (Tool::Select, "Select / move (S)"),
            (Tool::Wall, "Wall — chain polyline (W)"),
            (Tool::Door, "Place door (D)"),
            (Tool::Window, "Place window (N)"),
            (Tool::Floor, "Floor slab polygon (F)"),
            (Tool::Ceiling, "Ceiling polygon (C)"),
            (Tool::Foundation, "Foundation under a building outline (B)"),
            (Tool::Item, "Place item (I)"),
            (Tool::Site, "Landscaping — trees, fences, lawns, beds (L)"),
            (Tool::Measure, "Measure distance (M)"),
            (Tool::SectionLine, "Section line — two clicks (X)"),
            (Tool::Paint, "Paint elements — pick a color in Properties (P)"),
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
        Tool::Paint => {
            // Paint bucket with a drip.
            p.line_segment([c + Vec2::new(-5.0, -4.0), c + Vec2::new(5.0, -4.0)], s);
            p.line_segment([c + Vec2::new(-5.0, -4.0), c + Vec2::new(-3.5, 5.0)], s);
            p.line_segment([c + Vec2::new(5.0, -4.0), c + Vec2::new(3.5, 5.0)], s);
            p.line_segment([c + Vec2::new(-3.5, 5.0), c + Vec2::new(3.5, 5.0)], s);
            p.line_segment([c + Vec2::new(1.5, 1.0), c + Vec2::new(1.5, 6.0)], fill);
            p.circle_filled(c + Vec2::new(1.5, 7.2), 1.3, color);
        }
        Tool::Site => {
            // Tree glyph: trunk + canopy arcs + ground line.
            p.line_segment([c + Vec2::new(-6.0, 5.0), c + Vec2::new(6.0, 5.0)], fill);
            p.line_segment([c + Vec2::new(0.0, 5.0), c + Vec2::new(0.0, 0.0)], s);
            p.circle_filled(c + Vec2::new(0.0, -2.6), 3.4, color.gamma_multiply(0.55));
            p.circle_stroke(c + Vec2::new(0.0, -2.6), 3.4, s);
        }
        Tool::Foundation => {
            // Ground slab (edge-on line pair) + footing trapezoid + wall stub.
            p.line_segment([c + Vec2::new(-6.0, 1.0), c + Vec2::new(6.0, 1.0)], s);
            p.line_segment([c + Vec2::new(-6.0, 3.0), c + Vec2::new(6.0, 3.0)], s);
            p.line_segment([c + Vec2::new(-1.5, -6.0), c + Vec2::new(-1.5, 1.0)], fill);
            p.line_segment([c + Vec2::new(1.5, -6.0), c + Vec2::new(1.5, 1.0)], fill);
            p.line_segment([c + Vec2::new(-2.0, 3.0), c + Vec2::new(-3.5, 6.0)], fill);
            p.line_segment([c + Vec2::new(2.0, 3.0), c + Vec2::new(3.5, 6.0)], fill);
            p.line_segment([c + Vec2::new(-3.5, 6.0), c + Vec2::new(3.5, 6.0)], fill);
        }
    }
}

// ----------------------------------------------------------------- dock ---

pub fn dock(ui: &mut egui::Ui, app: &mut crate::BimcraftApp) {
    let t = theme::tokens_for(app.theme);
    egui::Panel::right("dock").exact_size(304.0).resizable(true).size_range(240.0..=460.0).frame(egui::Frame::NONE.fill(t.panel)).show(ui, |ui| {
        // Tab row (wrapped: every tab stays reachable at 240 px width).
        ui.horizontal_wrapped(|ui| {
            for tab in crate::DockTab::ALL {
                let active = app.dock_tab == tab;
                let txt = egui::RichText::new(tab.label()).font(theme::font_ui(11.5)).color(if active { t.text_strong } else { t.text_dim });
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
            crate::DockTab::Landscaping => landscaping_ui(ui, app, &t),
            crate::DockTab::Schedule => schedule_ui(ui, app, &t),
            crate::DockTab::Foundation => foundation_ui(ui, app, &t),
            crate::DockTab::Model => model_ui(ui, app, &t),
        });
    });
}

fn section(ui: &mut egui::Ui, title: &str, t: &Tokens) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new(title).font(theme::font_ui(11.0)).strong().color(t.text_dim));
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
            if app.session.tool == Tool::Paint {
                section(ui, "Paint", t);
                ui.label(
                    egui::RichText::new("Pick a color, then click elements in the plan or 3D view.").font(theme::font_ui(11.0)).color(t.text_dim),
                );
                ui.add_space(4.0);
                paint_palette(ui, app, t);
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Double-click / Enter: paint every element of the same kind.").font(theme::font_ui(10.5)).color(t.text_dim),
                );
                return;
            }
            ui.label(egui::RichText::new("Nothing selected").font(theme::font_ui(12.0)).color(t.text_dim));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Pick an element in the plan:\n· walls, doors, windows, floors, ceilings, items, foundations\n· or a datum grid line",
                )
                .font(theme::font_ui(11.0))
                .color(t.text_dim),
            );
        }
        Selection::Level(id) => level_editor(ui, app, t, id),
        Selection::Grid(id) => grid_editor(ui, app, t, id),
        Selection::Element(id) => element_editor(ui, app, t, id),
        Selection::Section(id) => section_editor(ui, app, t, id),
        Selection::Foundation(id) => foundation_editor(ui, app, t, id),
        Selection::Site(id) => site_editor(ui, app, t, id),
    }
}

/// Landscaping dock tab: the site palette (library conventions — entourage
/// on top, hard landscape below) + view toggles for derived layers.
fn landscaping_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Entourage", t);
    ui.label(
        egui::RichText::new("Pick a kind, then click in the plan (or the 3D ground) to place. R rotates.")
            .font(theme::font_ui(10.5))
            .color(t.text_dim),
    );
    ui.add_space(4.0);
    egui::Grid::new("site_palette_point").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        for kind in bimcraft_bim::SiteKind::ALL.into_iter().filter(|k| k.shape() == bimcraft_bim::SiteShape::Point) {
            let (w, _d, h) = bimcraft_bim::preset_size(kind);
            let active = app.session.site_kind == kind && app.session.tool == Tool::Site;
            let txt = format!("{}  ⌀{:.1}×{:.1}", kind.name(), w, h);
            let rich = egui::RichText::new(txt).font(theme::font_ui(11.5)).color(if active { t.text_strong } else { t.text });
            if ui.add(egui::Button::new(rich).fill(if active { t.tool_active } else { t.input }).min_size(Vec2::new(134.0, 0.0))).clicked() {
                app.session.site_kind = kind;
                app.session.set_tool(Tool::Site);
            }
        }
    });
    ui.add_space(8.0);
    section(ui, "Site work", t);
    egui::Grid::new("site_palette_linear").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        for kind in bimcraft_bim::SiteKind::ALL.into_iter().filter(|k| k.shape() != bimcraft_bim::SiteShape::Point) {
            let active = app.session.site_kind == kind && app.session.tool == Tool::Site;
            let how = if kind.shape() == bimcraft_bim::SiteShape::Chain { "chain" } else { "polygon" };
            let txt = format!("{} ({})", kind.name(), how);
            let rich = egui::RichText::new(txt).font(theme::font_ui(11.5)).color(if active { t.text_strong } else { t.text });
            if ui.add(egui::Button::new(rich).fill(if active { t.tool_active } else { t.input }).min_size(Vec2::new(134.0, 0.0))).clicked() {
                app.session.site_kind = kind;
                app.session.set_tool(Tool::Site);
            }
        }
    });
    ui.add_space(10.0);
    section(ui, "Derived layers", t);
    ui.checkbox(&mut app.session.show_formwork, "Formwork (sheets + corners)");
    ui.checkbox(&mut app.session.show_rebar, "Rebar (footings + corner starters)");
    ui.add_space(6.0);
    let n = app.session.model.site.len();
    caption(ui, t, format!("{} landscaping elements on site", n));
}

/// Selected landscaping element editor.
fn site_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64) {
    let Some(el) = app.session.model.site_element(id).cloned() else {
        app.session.select(Selection::None);
        return;
    };
    section(ui, "Landscaping", t);
    caption(ui, t, format!("{} · {}", el.kind.name(), el.kind.group()));
    let (w, d, h) = bimcraft_bim::preset_size(el.kind);
    match el.kind.shape() {
        bimcraft_bim::SiteShape::Point => caption(ui, t, format!("⌀ {:.1} × {:.1} m · h {:.1} m", w, d, h)),
        bimcraft_bim::SiteShape::Chain => caption(ui, t, format!("run length {:.1} m · h {:.1} m", el.chain_length(), h)),
        bimcraft_bim::SiteShape::Polygon => caption(ui, t, format!("area {:.1} m²", el.area())),
    }
    let mut px = el.pos.x;
    let mut py = el.pos.y;
    let mut rot_deg = el.rotation.to_degrees();
    let mut changed = false;
    changed |= drag_f64(ui, "X", &mut px, 0.1, -1000.0..=1000.0, " m");
    changed |= drag_f64(ui, "Y", &mut py, 0.1, -1000.0..=1000.0, " m");
    if el.kind.shape() == bimcraft_bim::SiteShape::Point {
        changed |= drag_f64(ui, "Rotation", &mut rot_deg, 5.0, -180.0..=180.0, "°");
    }
    if changed {
        app.session.begin_edit();
        if let Some(s) = app.session.model.site_element_mut(id) {
            s.pos = bimcraft_geom::Vec2::new(px, py);
            if el.kind.shape() == bimcraft_bim::SiteShape::Point {
                s.rotation = rot_deg.to_radians();
            }
        }
    }
    ui.add_space(4.0);
    if ui.add(egui::Button::new(egui::RichText::new("Delete").color(t.danger))).clicked() {
        app.session.delete_selected();
    }
}

/// BIM paint palette for the Paint tool (session.paint is 0xRRGGBB).
const PAINTS: [u32; 16] = [
    0xd9472f, 0xe8874a, 0xf0c05a, 0x9fbf6a, 0x45af63, 0x3f8f7a, 0x5bc8d8, 0x4f80ff, 0x7a6ff0, 0xc06ad0, 0xe06a9a, 0xf06a52, 0xc8724a, 0x8a6f52,
    0xe4e9f1, 0x3d4553,
];

fn paint_palette(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    egui::Grid::new("paint_palette").num_columns(8).spacing([4.0, 4.0]).show(ui, |ui| {
        for (i, &c) in PAINTS.iter().enumerate() {
            let col = Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
            let active = app.session.paint == c;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(26.0, 22.0), egui::Sense::click());
            ui.painter().rect_filled(rect, CornerRadius::same(4), col);
            if active {
                ui.painter().rect_stroke(rect.expand(2.5), CornerRadius::same(6), Stroke::new(2.0, t.text_strong), egui::StrokeKind::Outside);
            } else {
                ui.painter().rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
            }
            if resp.clicked() {
                app.session.paint = c;
                app.session.status = format!("Paint #{c:06x} — click elements to apply");
            }
            resp.on_hover_text(format!("#{c:06x}"));
            if (i + 1) % 8 == 0 {
                ui.end_row();
            }
        }
    });
    ui.add_space(2.0);
    let cur = app.session.paint;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 10.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(3), Color32::from_rgb((cur >> 16) as u8, (cur >> 8) as u8, cur as u8));
    ui.label(egui::RichText::new(format!("Active paint  #{cur:06x}")).font(theme::font_mono(10.5)).color(t.text_dim));
}

fn section_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64) {
    let Some(sec) = app.session.model.section(id).cloned() else {
        ui.label("Section not found");
        return;
    };
    section(ui, "Section", t);
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
    section(ui, "Level", t);
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
    let mut gsx = level.grid_shift[0];
    let mut gsy = level.grid_shift[1];
    let ch_elev = drag_f64(ui, "Elevation", &mut elev, 0.05, -100.0..=1000.0, " m");
    let ch_h = drag_f64(ui, "Height", &mut height, 0.05, 1.0..=20.0, " m");
    // Level grid shift (aedifex parity): slide this level's datum grid only.
    let ch_gs = drag_f64(ui, "Grid shift X", &mut gsx, 0.1, -50.0..=50.0, " m") | drag_f64(ui, "Grid shift Y", &mut gsy, 0.1, -50.0..=50.0, " m");
    if ch_elev || ch_h || ch_gs {
        app.session.begin_edit();
        if let Some(l) = app.session.model.level_mut(id) {
            l.elevation = elev;
            l.height = height;
            l.grid_shift = [gsx, gsy];
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
    section(ui, "Datum grid", t);
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
    // Full properties: one editor block per element kind (JS propsBlock parity).
    let kind_label = match el.kind {
        ElementKind::Wall => "Wall",
        ElementKind::Door => "Door",
        ElementKind::Window => "Window",
        ElementKind::Slab => "Floor",
        ElementKind::Ceiling => "Ceiling",
        ElementKind::Item => "Item",
    };
    section(ui, kind_label, t);
    // Name (every element is renamable).
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Name").font(theme::font_ui(12.0)));
        let mut name = el.name.clone();
        if ui.text_edit_singleline(&mut name).lost_focus() && name != el.name {
            app.session.begin_edit();
            if let Some(e) = app.session.model.element_mut(id) {
                e.name = name;
            }
        }
    });
    match &el.data {
        ElementData::Wall(w) => wall_editor(ui, app, t, id, w),
        ElementData::Opening(o) => opening_editor(ui, app, t, id, el.kind, o),
        ElementData::Slab(s) => slab_editor(ui, app, t, id, s),
        ElementData::Ceiling(c) => ceiling_editor(ui, app, t, id, c),
        ElementData::Item(i) => item_editor(ui, app, t, id, i),
    }
    // Paint section (Paint tool + properties: full color control).
    ui.add_space(4.0);
    section(ui, "Paint", t);
    paint_palette(ui, app, t);
    ui.horizontal(|ui| {
        if ui.button("Eyedropper").on_hover_text("Take this element's paint as the active color").clicked() {
            app.session.pick_paint(id);
        }
        if ui.button("Clear").on_hover_text("Back to the palette default").clicked() {
            app.session.clear_paint(id);
        }
    });
    ui.add_space(4.0);
    if ui.add(egui::Button::new(egui::RichText::new("Delete").color(t.danger))).clicked() {
        app.session.delete_selected();
    }
}

fn caption(ui: &mut egui::Ui, t: &Tokens, text: String) {
    ui.label(egui::RichText::new(text).font(theme::font_mono(11.0)).color(t.text_dim));
}

/// The next level strictly above `level_id` (by elevation), for "up to level" tops.
fn next_level_above(model: &bimcraft_bim::Model, level_id: u64) -> Option<u64> {
    let base = model.level(level_id)?.elevation;
    model.levels.iter().filter(|l| l.id != level_id && l.elevation > base).min_by(|a, b| a.elevation.total_cmp(&b.elevation)).map(|l| l.id)
}

fn wall_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64, w: &bimcraft_bim::Wall) {
    let len = w.total_length();
    caption(ui, t, format!("{} segments · length {:.2} m", w.pts.len().saturating_sub(1), len));

    // Type: assembly presets (thickness drives the geometry) + Custom.
    let matched = bimcraft_bim::wall_type(&w.type_key).filter(|pt| (pt.thickness - w.thickness).abs() < 1e-3);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Type").font(theme::font_ui(12.0)));
        egui::ComboBox::from_id_salt("wall_type").selected_text(matched.map(|pt| pt.name).unwrap_or("Custom")).show_ui(ui, |ui| {
            for pt in bimcraft_bim::WALL_TYPES {
                let on = matched.map(|cur| cur.key == pt.key).unwrap_or(false);
                if ui.selectable_label(on, pt.name).clicked() {
                    app.session.begin_edit();
                    if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                        w.type_key = pt.key.to_string();
                        w.thickness = pt.thickness;
                    }
                }
            }
            if ui.selectable_label(matched.is_none(), "Custom").clicked() {
                app.session.begin_edit();
                if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                    w.type_key = String::new();
                }
            }
        });
    });

    // Brick finish: paint another brick type on the wall faces (an empty
    // finish keeps the plain assembly surface).
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Brick").font(theme::font_ui(12.0)));
        let current = bimcraft_bim::brick_type(&w.finish).map(|b| b.name).unwrap_or("None");
        egui::ComboBox::from_id_salt("wall_brick").selected_text(current).show_ui(ui, |ui| {
            if ui.selectable_label(w.finish.is_empty(), "None").clicked() {
                app.session.begin_edit();
                if let Some(ElementData::Wall(ww)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                    ww.finish.clear();
                }
            }
            for b in bimcraft_bim::BRICK_TYPES {
                let on = w.finish == b.key;
                if ui.selectable_label(on, b.name).clicked() {
                    app.session.begin_edit();
                    if let Some(ElementData::Wall(ww)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                        ww.finish = b.key.to_string();
                    }
                }
            }
        });
    });
    if let Some(b) = bimcraft_bim::brick_type(&w.finish) {
        let face_area = len * w.effective_height(&app.session.model) * 2.0;
        caption(
            ui,
            t,
            format!(
                "{} · {} × {} × {} mm · ≈{:.0} bricks (both faces)",
                b.name,
                (b.length * 1000.0) as i64,
                (b.width * 1000.0) as i64,
                (b.height * 1000.0) as i64,
                bimcraft_bim::brick::bricks_per_m2(b) * face_area
            ),
        );
    }

    // Thickness: keep the assembly key in sync (no preset at that thickness = custom).
    let mut thk = w.thickness;
    if drag_f64(ui, "Thickness", &mut thk, 0.01, 0.05..=2.0, " m") {
        app.session.begin_edit();
        if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
            w.thickness = thk;
            w.type_key = bimcraft_bim::wall_type_for_thickness(thk).map(|pt| pt.key).unwrap_or("").to_string();
        }
    }

    // Length: editing moves the last point along the last segment's direction.
    let mut new_len = len;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Length").font(theme::font_ui(12.0)));
        if ui.add(egui::DragValue::new(&mut new_len).speed(0.1).range(0.1..=1000.0).suffix(" m")).changed() {
            app.session.begin_edit();
            if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                let n = w.pts.len();
                if n >= 2 {
                    if n == 2 {
                        let dir = (w.pts[1] - w.pts[0]).normalize();
                        w.pts[1] = w.pts[0] + dir * new_len;
                    } else {
                        let prev = w.pts[n - 2];
                        let dir = (w.pts[n - 1] - prev).normalize();
                        let up_to: f64 = w.pts.windows(2).take(n - 2).map(|s| (s[1] - s[0]).length()).sum();
                        let rest = (new_len - up_to).max(0.05);
                        w.pts[n - 1] = prev + dir * rest;
                    }
                }
            }
        }
    });

    // Justification: where the body sits relative to the location line.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Justification").font(theme::font_ui(12.0)));
        egui::ComboBox::from_id_salt("wall_just")
            .selected_text(match w.justification {
                bimcraft_bim::Justification::Left => "Left",
                bimcraft_bim::Justification::Center => "Center",
                bimcraft_bim::Justification::Right => "Right",
            })
            .show_ui(ui, |ui| {
                for (j, name) in [
                    (bimcraft_bim::Justification::Left, "Left"),
                    (bimcraft_bim::Justification::Center, "Center"),
                    (bimcraft_bim::Justification::Right, "Right"),
                ] {
                    if ui.selectable_label(w.justification == j, name).clicked() {
                        app.session.begin_edit();
                        if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                            w.justification = j;
                        }
                    }
                }
            });
    });

    // Top: fixed height or following a level above.
    let up_to_level = w.top_level.is_some();
    let mut top_mode: Option<bool> = None;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Top").font(theme::font_ui(12.0)));
        egui::ComboBox::from_id_salt("wall_top").selected_text(if up_to_level { "Up to level" } else { "Fixed height" }).show_ui(ui, |ui| {
            if ui.selectable_label(!up_to_level, "Fixed height").clicked() {
                top_mode = Some(false);
            }
            if ui.selectable_label(up_to_level, "Up to level").clicked() {
                top_mode = Some(true);
            }
        });
    });
    if top_mode == Some(true) && !up_to_level {
        match next_level_above(&app.session.model, w.level) {
            Some(lid) => {
                app.session.begin_edit();
                if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                    w.top_level = Some(lid);
                }
            }
            None => app.session.status = "No level above — keeping fixed height".into(),
        }
    }
    if top_mode == Some(false) && up_to_level {
        // Freeze the current effective height so the wall does not jump.
        let h = w.effective_height(&app.session.model);
        app.session.begin_edit();
        if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
            w.top_level = None;
            w.height = h;
        }
    }
    if up_to_level {
        // Levels strictly above the wall's level, by elevation.
        let base_elev = app.session.model.level(w.level).map(|l| l.elevation).unwrap_or(0.0);
        let mut entries: Vec<(u64, String)> =
            app.session.model.levels.iter().filter(|l| l.elevation > base_elev).map(|l| (l.id, l.name.clone())).collect();
        if let Some(tid) = w.top_level {
            if !entries.iter().any(|(lid, _)| *lid == tid) {
                if let Some(l) = app.session.model.level(tid) {
                    entries.push((l.id, l.name.clone()));
                }
            }
        }
        let sel_name = w.top_level.and_then(|tid| app.session.model.level(tid).map(|l| l.name.clone())).unwrap_or_else(|| "?".into());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Level").font(theme::font_ui(12.0)));
            egui::ComboBox::from_id_salt("wall_top_level").selected_text(sel_name).show_ui(ui, |ui| {
                for (lid, name) in &entries {
                    if ui.selectable_label(w.top_level == Some(*lid), name).clicked() {
                        app.session.begin_edit();
                        if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                            w.top_level = Some(*lid);
                        }
                    }
                }
            });
        });
        let mut top_off = w.top_offset;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Top offset").font(theme::font_ui(12.0)));
            if ui.add(egui::DragValue::new(&mut top_off).speed(0.05).suffix(" m")).changed() {
                app.session.begin_edit();
                if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                    w.top_offset = top_off;
                }
            }
        });
    } else {
        let mut h = w.height;
        if drag_f64(ui, "Height", &mut h, 0.05, 0.5..=20.0, " m") {
            app.session.begin_edit();
            if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                w.height = h;
            }
        }
    }

    // Base offset above the level floor (e.g. on a foundation stem).
    let mut base = w.base_offset;
    if drag_f64(ui, "Base offset", &mut base, 0.05, -2.0..=10.0, " m") {
        app.session.begin_edit();
        if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
            w.base_offset = base;
        }
    }

    // Outside face: which side of the location line is the exterior.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Outside face").font(theme::font_ui(12.0)));
        ui.label(egui::RichText::new(w.ext_side.label()).font(theme::font_ui(12.0)).color(t.text_dim));
        if ui.button("Flip").on_hover_text("Swap the outside face to the other side").clicked() {
            app.session.begin_edit();
            if let Some(ElementData::Wall(w)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                w.ext_side = w.ext_side.flip();
            }
        }
    });

    // Level: read-only (move walls by selecting them on the level).
    if let Some(name) = app.session.model.level(w.level).map(|l| l.name.clone()) {
        ui.label(egui::RichText::new(format!("Level: {name}")).font(theme::font_ui(11.0)).color(t.text_dim));
    }
}

fn opening_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64, kind: ElementKind, o: &bimcraft_bim::Opening) {
    let family = if kind == ElementKind::Door { bimcraft_bim::OpeningFamily::Door } else { bimcraft_bim::OpeningFamily::Window };
    let kind_label = if kind == ElementKind::Door { "Door" } else { "Window" };
    let host = app.session.model.element(o.wall);
    let wall_len = app.session.model.wall(o.wall).map(|w| w.total_length()).unwrap_or(0.0);
    ui.label(
        egui::RichText::new(format!("{kind_label} hosted on {}", host.map(|w| w.name.as_str()).unwrap_or("a wall")))
            .font(theme::font_ui(11.0))
            .color(t.text_dim),
    );

    // Type: family presets (width + height come from the preset) + Custom.
    let preset = o.opening_type();
    let in_family = preset.map(|pt| pt.family == family).unwrap_or(false);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Type").font(theme::font_ui(12.0)));
        egui::ComboBox::from_id_salt("opening_type")
            .selected_text(if in_family { preset.map(|pt| pt.name).unwrap_or("Custom") } else { "Custom" })
            .show_ui(ui, |ui| {
                for pt in bimcraft_bim::opening_types_of(family) {
                    let on = preset.map(|cur| cur.key == pt.key).unwrap_or(false);
                    if ui.selectable_label(on, pt.name).clicked() {
                        app.session.begin_edit();
                        if let Some(ElementData::Opening(o)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                            o.type_key = pt.key.to_string();
                            o.width = pt.width;
                            o.height = pt.height;
                        }
                        // Re-clamp the center position after the width change.
                        let s = app.session.model.element(id).and_then(|e| match &e.data {
                            ElementData::Opening(o) => Some(o.s),
                            _ => None,
                        });
                        if let Some(s) = s {
                            app.session.model.shift_opening(id, s);
                        }
                    }
                }
                if ui.selectable_label(!in_family, "Custom").clicked() {
                    app.session.begin_edit();
                    if let Some(ElementData::Opening(o)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                        o.type_key = String::new();
                    }
                }
            });
    });

    // Dimensions.
    let mut width = o.width;
    let mut height = o.height;
    let mut sill = o.sill;
    let mut dims = false;
    dims |= drag_f64(ui, "Width", &mut width, 0.01, 0.3..=4.0, " m");
    dims |= drag_f64(ui, "Height", &mut height, 0.01, 0.3..=4.0, " m");
    if kind == ElementKind::Window {
        dims |= drag_f64(ui, "Sill", &mut sill, 0.01, 0.0..=2.5, " m");
    }

    // Position: `s` is the center arclength; from start / from end read the jambs.
    let mut from_start = (o.s - o.width / 2.0).max(0.0);
    let mut from_end = (wall_len - o.s - o.width / 2.0).max(0.0);
    let mut fs = false;
    let mut fe = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("From start").font(theme::font_ui(12.0)));
        fs |= ui.add(egui::DragValue::new(&mut from_start).speed(0.05).suffix(" m")).changed();
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("From end").font(theme::font_ui(12.0)));
        fe |= ui.add(egui::DragValue::new(&mut from_end).speed(0.05).suffix(" m")).changed();
    });

    // Shift along host wall (the "windows to be shifted" feature, kept).
    let mut s = o.s;
    let mut shifted = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Shift").font(theme::font_ui(12.0)));
        shifted |= ui.add(egui::Slider::new(&mut s, 0.0..=wall_len.max(0.1)).text("along wall").suffix(" m")).changed();
    });

    if dims || fs || fe || shifted {
        app.session.begin_edit();
        if let Some(ElementData::Opening(o)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
            o.width = width;
            o.height = height;
            o.sill = sill;
            if fs {
                o.s = from_start + o.width / 2.0;
            } else if fe {
                o.s = wall_len - from_end - o.width / 2.0;
            } else if shifted {
                o.s = s;
            }
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

    // Hand (hinge side) only for handed presets; swing face for everything.
    if preset.map(|pt| pt.handed).unwrap_or(false) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Hinges").font(theme::font_ui(12.0)));
            ui.label(egui::RichText::new(if o.hand == 'a' { "A" } else { "B" }).font(theme::font_ui(12.0)).color(t.text_dim));
            if ui.button("Flip").on_hover_text("Swap the hinge side").clicked() {
                app.session.begin_edit();
                if let Some(ElementData::Opening(o)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                    o.hand = if o.hand == 'a' { 'b' } else { 'a' };
                }
            }
        });
    }
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Swing faces").font(theme::font_ui(12.0)));
        ui.label(egui::RichText::new(if o.face == 'a' { "A" } else { "B" }).font(theme::font_ui(12.0)).color(t.text_dim));
        if ui.button("Flip").on_hover_text("Mirror the swing to the other face").clicked() {
            app.session.begin_edit();
            if let Some(ElementData::Opening(o)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                o.face = if o.face == 'a' { 'b' } else { 'a' };
            }
        }
    });

    caption(ui, t, format!("Host wall length {:.2} m", wall_len));
    ui.label(egui::RichText::new("Tip: drag the opening in the plan to slide it along the wall.").font(theme::font_ui(10.5)).color(t.text_dim));
}

fn slab_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64, s: &bimcraft_bim::Slab) {
    caption(ui, t, format!("Polygon · {} points · area {:.2} m²", s.pts.len(), s.area()));
    // Kind: floor finish preset.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Kind").font(theme::font_ui(12.0)));
        let cur = s.kind.clone();
        egui::ComboBox::from_id_salt("floor_kind").selected_text(cur.clone()).show_ui(ui, |ui| {
            for k in bimcraft_bim::FLOOR_KINDS {
                if ui.selectable_label(cur == *k, *k).clicked() {
                    app.session.begin_edit();
                    if let Some(ElementData::Slab(sl)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
                        sl.kind = (*k).to_string();
                    }
                }
            }
        });
    });
    let mut thk = s.thickness;
    let mut elev = s.elevation;
    let mut changed = false;
    changed |= drag_f64(ui, "Thickness", &mut thk, 0.01, 0.02..=2.0, " m");
    changed |= drag_f64(ui, "Elevation", &mut elev, 0.02, -2.0..=30.0, " m");
    if changed {
        app.session.begin_edit();
        if let Some(ElementData::Slab(sl)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
            sl.thickness = thk;
            sl.elevation = elev;
        }
    }
}

fn ceiling_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64, c: &bimcraft_bim::Ceiling) {
    caption(ui, t, format!("Polygon · {} points · area {:.2} m²", c.pts.len(), c.area()));
    let mut elev = c.elevation;
    let mut thk = c.thickness;
    let mut changed = false;
    changed |= drag_f64(ui, "Height", &mut elev, 0.05, 0.5..=20.0, " m");
    changed |= drag_f64(ui, "Thickness", &mut thk, 0.01, 0.01..=2.0, " m");
    // Pattern: plan hatch (none / grid / plank).
    let mut pattern = c.pattern.clone();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Pattern").font(theme::font_ui(12.0)));
        egui::ComboBox::from_id_salt("ceiling_pattern").selected_text(pattern.clone()).show_ui(ui, |ui| {
            for p in bimcraft_bim::CEILING_PATTERNS {
                if ui.selectable_label(pattern == *p, *p).clicked() {
                    pattern = (*p).to_string();
                }
            }
        });
    });
    changed |= pattern != c.pattern;
    if changed {
        app.session.begin_edit();
        if let Some(ElementData::Ceiling(cl)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
            cl.elevation = elev;
            cl.thickness = thk;
            cl.pattern = pattern;
        }
    }
}

fn item_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64, i: &bimcraft_bim::Item) {
    caption(ui, t, format!("{} · footprint {:.2} × {:.2} × {:.2} m", i.kind.name(), i.kind.preset().0, i.kind.preset().1, i.kind.preset().2));
    // Full placement properties: position on the plan.
    let (mut px, mut py) = (i.pos.x, i.pos.y);
    let mut changed = false;
    changed |= drag_f64(ui, "Pos X", &mut px, 0.05, -200.0..=200.0, " m");
    changed |= drag_f64(ui, "Pos Y", &mut py, 0.05, -200.0..=200.0, " m");
    let mut rot_deg = i.rotation.to_degrees();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Rotate").font(theme::font_ui(12.0)));
        let sl = ui.add(egui::Slider::new(&mut rot_deg, -180.0..=180.0).suffix("°"));
        changed |= sl.changed();
    });
    if changed {
        app.session.begin_edit();
        if let Some(ElementData::Item(it)) = app.session.model.element_mut(id).map(|e| &mut e.data) {
            it.pos = bimcraft_geom::Vec2::new(px, py);
            it.rotation = rot_deg.to_radians();
        }
    }
    ui.label(egui::RichText::new("R rotates 90° · Shift+R rotates 15°").font(theme::font_ui(10.5)).color(t.text_dim));
}

fn foundation_editor(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens, id: u64) {
    let Some(mut f) = app.session.model.foundation(id).cloned() else {
        app.session.select(Selection::None);
        return;
    };
    section(ui, "Foundation", t);
    // Name.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Name").font(theme::font_ui(12.0)));
        let mut name = f.name.clone();
        if ui.text_edit_singleline(&mut name).lost_focus() && name != f.name {
            app.session.begin_edit();
            if let Some(fo) = app.session.model.foundation_mut(id) {
                fo.name = name;
            }
        }
    });

    // Kind: switching resets the depth-related sizes (fresh depths per kind).
    let mut kind_pick: Option<FoundationKind> = None;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Kind").font(theme::font_ui(12.0)));
        egui::ComboBox::from_id_salt("foundation_kind").selected_text(f.kind.label()).show_ui(ui, |ui| {
            for k in FoundationKind::ALL {
                if ui.selectable_label(f.kind == k, k.label()).clicked() {
                    kind_pick = Some(k);
                }
            }
        });
    });
    if let Some(k) = kind_pick {
        if k != f.kind {
            app.session.begin_edit();
            if let Some(fo) = app.session.model.foundation_mut(id) {
                fo.kind = k;
                fo.sizes.bottom = 0.0;
                fo.sizes.stem_drop = 0.0;
            }
            app.session.status = format!("Kind set to {} — depths reset to automatic", k.label());
        }
    }

    // Reinforcement intent (bars derive from the geometry on every build).
    section(ui, "Rebar", t);
    let mut rb = f.rebar;
    let mut rb_changed = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Bar").font(theme::font_ui(12.0)));
        egui::ComboBox::from_id_salt("rebar_bar").selected_text(bimcraft_bim::rebar::bar(rb.bar).label).show_ui(ui, |ui| {
            for (i, bs) in bimcraft_bim::BARS.iter().enumerate() {
                if ui.selectable_label(rb.bar == i, format!("{} (⌀{} mm)", bs.label, (bs.db * 1000.0) as i64)).clicked() {
                    rb.bar = i;
                    rb_changed = true;
                }
            }
        });
    });
    rb_changed |= drag_f64(ui, "Spacing", &mut rb.spacing, 0.025, 0.05..=0.6, " m");
    rb_changed |= drag_f64(ui, "Cover", &mut rb.cover, 0.005, 0.02..=0.15, " m");
    rb_changed |= ui.checkbox(&mut rb.hooks, "Hooked ends").changed();
    rb_changed |= ui.checkbox(&mut rb.starters, "Starters").changed();
    if rb_changed {
        app.session.begin_edit();
        if let Some(fo) = app.session.model.foundation_mut(id) {
            fo.rebar = rb;
        }
        f = app.session.model.foundation(id).cloned().unwrap_or(f);
    }
    // Derived summary: bar count + steel mass + warnings (DetailCraft checks).
    if let Some((p, outline)) = app.session.model.foundation_resolved(&f) {
        let parts = bimcraft_bim::foundation::foundation_geometry(&outline, &p);
        let set = bimcraft_bim::footing_rebar(&parts, &p, &f.rebar, app.session.model.level(f.level).map(|l| l.elevation).unwrap_or(0.0));
        if !set.bars.is_empty() {
            caption(ui, t, format!("{} bars · {:.0} kg steel", set.bars.len(), set.total_mass()));
        }
        for warn in &set.warnings {
            caption(ui, t, format!("⚠ {warn}"));
        }
        for prob in &set.problems {
            caption(ui, t, format!("✗ {prob}"));
        }
    }

    // Stored sizes (0 = automatic). Only the kind's relevant fields are shown.
    ui.label(egui::RichText::new("0 = automatic size").font(theme::font_ui(10.5)).color(t.text_dim));
    let mut sizes = f.sizes;
    let mut changed = false;
    let stem = [(&mut sizes.stem_t, "Stem thickness"), (&mut sizes.foot_w, "Footing width"), (&mut sizes.foot_d, "Footing depth")];
    match f.kind {
        FoundationKind::Slab => {
            changed |= drag_f64(ui, "Slab thickness", &mut sizes.slab_t, 0.01, 0.0..=2.0, " m");
            changed |= drag_f64(ui, "Edge width", &mut sizes.edge_w, 0.01, 0.0..=2.0, " m");
            changed |= drag_f64(ui, "Edge depth", &mut sizes.edge_d, 0.01, 0.0..=2.0, " m");
        }
        FoundationKind::StemSlab => {
            for (v, label) in stem {
                changed |= drag_f64(ui, label, v, 0.01, 0.0..=6.0, " m");
            }
            changed |= drag_f64(ui, "Below floor", &mut sizes.bottom, 0.01, 0.0..=6.0, " m");
            changed |= drag_f64(ui, "Stem drop", &mut sizes.stem_drop, 0.01, 0.0..=6.0, " m");
            changed |= drag_f64(ui, "Slab thickness", &mut sizes.slab_t, 0.01, 0.0..=2.0, " m");
        }
        FoundationKind::Crawl => {
            for (v, label) in stem {
                changed |= drag_f64(ui, label, v, 0.01, 0.0..=6.0, " m");
            }
            changed |= drag_f64(ui, "Below floor", &mut sizes.bottom, 0.01, 0.0..=6.0, " m");
            changed |= drag_f64(ui, "Stem drop", &mut sizes.stem_drop, 0.01, 0.0..=6.0, " m");
        }
        FoundationKind::Basement => {
            for (v, label) in stem {
                changed |= drag_f64(ui, label, v, 0.01, 0.0..=6.0, " m");
            }
            changed |= drag_f64(ui, "Basement height", &mut sizes.basement_h, 0.01, 0.0..=6.0, " m");
            changed |= drag_f64(ui, "Slab thickness", &mut sizes.slab_t, 0.01, 0.0..=2.0, " m");
        }
    }
    if changed {
        app.session.begin_edit();
        if let Some(fo) = app.session.model.foundation_mut(id) {
            fo.sizes = sizes;
        }
    }

    // Computed block: outline, loads, footing, concrete volume, part list.
    if let Some((p, outline)) = app.session.model.foundation_resolved(&f) {
        let mut perimeter: f64 = outline.windows(2).map(|w| (w[1] - w[0]).length()).sum();
        if outline.len() >= 2 {
            perimeter += (outline[0] - outline[outline.len() - 1]).length();
        }
        let (stories, heavy) = app.session.model.loads_on_level(f.level);
        ui.add_space(4.0);
        caption(ui, t, format!("Outline {:.1} m · {} {}", perimeter, stories, if stories == 1 { "story" } else { "stories" }));
        if heavy {
            caption(ui, t, "heavy walls — footings widened".into());
        }
        caption(ui, t, format!("Footing {:.2} × {:.2} m ({})", p.foot_w, p.foot_d, if f.sizes.foot_w > 0.0 { "set" } else { "auto" }));
        let parts = bimcraft_bim::foundation::foundation_geometry(&outline, &p);
        let volume = bimcraft_bim::foundation::foundation_concrete_volume(&parts);
        caption(ui, t, format!("Concrete {volume:.2} m³ · {} parts", parts.len()));
        for part in &parts {
            caption(ui, t, format!("{}  z {:.2}..{:.2}", part.role.label(), part.z0, part.z1));
        }
    } else {
        ui.label(egui::RichText::new("No building outline under the seed — sizes stay automatic.").font(theme::font_ui(11.0)).color(t.warning));
    }

    ui.add_space(4.0);
    if ui.add(egui::Button::new(egui::RichText::new("Delete").color(t.danger))).clicked() {
        app.session.begin_edit();
        app.session.model.delete_foundation(id);
        app.session.select(Selection::None);
    }
}

fn levels_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Levels", t);
    let levels = app.session.model.levels.clone();
    for l in &levels {
        let is_active = Some(l.id) == app.session.active_level;
        let txt = format!("{}   elev {:.2} m   h {:.1} m", l.name, l.elevation, l.height);
        let rich = egui::RichText::new(txt).font(theme::font_ui(12.0)).color(if is_active { t.text_strong } else { t.text_dim });
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
    section(ui, "View toggles", t);
    ui.checkbox(&mut app.show_items_2d, "Show items in plan");
    ui.checkbox(&mut app.show_ceilings_3d, "Show ceilings in 3D");
    ui.checkbox(&mut app.session.solo_level, "Solo level (hide other storeys)");
    ui.checkbox(&mut app.session.xray, "Wall x-ray (translucent walls)");
    ui.checkbox(&mut app.session.show_rebar, "Rebar (footings + corner starters)");
    ui.add_space(10.0);
    section(ui, "Snapping", t);
    ui.checkbox(&mut app.session.snap_grid, "Grid snap (0.5 m, G)");
    ui.checkbox(&mut app.session.snap_endpoints, "Endpoint snap");
}

/// Foundation dock tab: list, next placement kind, formwork summary.
fn foundation_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Foundations", t);
    let founds = app.session.model.foundations.clone();
    if founds.is_empty() {
        ui.label(egui::RichText::new("Click inside a building outline with the Foundation tool (B).").font(theme::font_ui(11.0)).color(t.text_dim));
    }
    for f in &founds {
        let vol =
            app.session.model.foundation_resolved(f).map(|(p, outline)| {
                bimcraft_bim::foundation::foundation_concrete_volume(&bimcraft_bim::foundation::foundation_geometry(&outline, &p))
            });
        let vol_txt = vol.map(|v| format!("{v:.1} m³")).unwrap_or_else(|| "—".into());
        let selected = app.session.selection == Selection::Foundation(f.id);
        let rich = egui::RichText::new(format!("{} · {} · {}", f.name, f.kind.label(), vol_txt)).font(theme::font_ui(12.0)).color(if selected {
            t.text_strong
        } else {
            t.text
        });
        let resp = ui
            .add(egui::Button::new(rich).fill(if selected { t.tool_active } else { t.input }).min_size(Vec2::new(ui.available_width(), 0.0)))
            .on_hover_text("Click to select · edit sizes in Properties");
        if resp.clicked() {
            app.session.select(Selection::Foundation(f.id));
            app.dock_tab = crate::DockTab::Properties;
        }
    }
    ui.horizontal(|ui| {
        if ui.button("+ Place foundation").on_hover_text("Foundation tool: click inside a building outline (B)").clicked() {
            app.session.set_tool(Tool::Foundation);
            app.session.status = "Click inside a building outline to place the foundation".into();
        }
        let can_del = matches!(app.session.selection, Selection::Foundation(_));
        if ui.add_enabled(can_del, egui::Button::new(egui::RichText::new("Delete selected").color(t.danger))).clicked() {
            app.session.delete_selected();
        }
    });
    ui.add_space(10.0);
    section(ui, "Next placement", t);
    egui::ComboBox::from_id_salt("next_foundation_kind").selected_text(app.session.foundation_kind.label()).show_ui(ui, |ui| {
        for k in FoundationKind::ALL {
            let on = app.session.foundation_kind == k;
            if ui.selectable_label(on, k.label()).clicked() {
                app.session.foundation_kind = k;
            }
        }
    });
    ui.label(egui::RichText::new("Sizes are automatic (0) until you override them in Properties.").font(theme::font_ui(10.5)).color(t.text_dim));
    ui.add_space(10.0);
    section(ui, "Formwork", t);
    ui.checkbox(&mut app.session.show_formwork, "Show wall formwork (sheets + corners)");
    if app.session.show_formwork {
        if let Some(elid) = app.session.selected_element() {
            if let Some(w) = app.session.model.wall(elid).cloned() {
                let base_z = w.base_z(&app.session.model);
                let top_z = w.top_z(&app.session.model);
                let panels = bimcraft_bim::formwork::wall_formwork(&w, base_z, top_z);
                let posts = bimcraft_bim::formwork::wall_corner_posts(&w, base_z, top_z);
                let area = bimcraft_bim::formwork::formwork_area(&panels);
                caption(ui, t, format!("Panels {} · sheathing {:.1} m² · corner posts {}", panels.len(), area, posts.len()));
            }
        }
    }
}

fn library_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Item library", t);
    ui.label(egui::RichText::new("Pick a kind, then click in the plan to place. R rotates.").font(theme::font_ui(11.0)).color(t.text_dim));
    ui.add_space(4.0);
    egui::Grid::new("item_library").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
        for kind in ItemKind::ALL {
            let (w, d, h) = kind.preset();
            let active = app.session.item_kind == kind && app.session.tool == Tool::Item;
            let txt = format!("{}  {:.1}×{:.1}×{:.1}", kind.name(), w, d, h);
            let rich = egui::RichText::new(txt).font(theme::font_ui(11.5)).color(if active { t.text_strong } else { t.text });
            if ui.add(egui::Button::new(rich).fill(if active { t.tool_active } else { t.input }).min_size(Vec2::new(128.0, 0.0))).clicked() {
                app.session.item_kind = kind;
                app.session.set_tool(Tool::Item);
            }
        }
    });
}

fn schedule_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Quantity schedule", t);
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
                    ui.label(egui::RichText::new(&r.group).font(theme::font_ui(11.0)).color(t.accent));
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
    ui.label(egui::RichText::new("CSV export: bimcraft-cli schedule <file> --csv").font(theme::font_ui(10.5)).color(t.text_dim));
}

fn model_ui(ui: &mut egui::Ui, app: &mut crate::BimcraftApp, t: &Tokens) {
    section(ui, "Datum grids", t);
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
    section(ui, "Model tree", t);
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
    let t = theme::tokens_for(app.theme);
    egui::Panel::bottom("status_bar").exact_size(24.0).resizable(false).frame(egui::Frame::NONE.fill(t.app_bar)).show(ui, |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            let mode = match app.session.view {
                ViewMode::Plan2D => "2D Plan",
                ViewMode::Perspective3D => "3D View",
                ViewMode::Walk => "Walk (WASD + look)",
                ViewMode::Section => "Section",
            };
            ui.label(egui::RichText::new(mode).font(theme::font_ui(11.0)).color(t.accent).strong());
            ui.separator();
            ui.label(egui::RichText::new(app.session.tool.label()).font(theme::font_ui(11.0)).color(t.text));
            ui.separator();
            // Current paint chip + formwork state chip.
            let pc = app.session.paint;
            let (prect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), egui::Sense::hover());
            ui.painter().rect_filled(prect, CornerRadius::same(3), Color32::from_rgb((pc >> 16) as u8, (pc >> 8) as u8, pc as u8));
            ui.painter().rect_stroke(prect, CornerRadius::same(3), Stroke::new(1.0, t.input_border), egui::StrokeKind::Inside);
            if app.session.show_formwork {
                ui.label(egui::RichText::new("formwork").font(theme::font_ui(11.0)).color(t.accent));
            }
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
    let t = theme::tokens_for(app.theme);
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
