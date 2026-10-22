//! BimCraft egui frontend: a thin shell over the engine Session, following the
//! Craft-app architecture (app bar + 48 px tool rail + central canvas + right
//! dock + status bar). Icon buttons and theming use the adopted `craft-ui`
//! crate from storytold craft-libs (MIT OR Apache-2.0).

mod canvas;
mod chrome;
pub mod theme;

use bimcraft_engine::{Session, Tool, ViewMode};
use bimcraft_geom::{Vec2, Vec3};
use theme::Theme;

/// Top-level app state. Platform shells implement `eframe::App` and delegate
/// to [`BimcraftApp::update`]; persistence goes through `bimcraft-io` JSON.
pub struct BimcraftApp {
    pub session: Session,
    /// Path text used by the desktop Open/Save dialogs.
    pub save_path: String,
    pub show_open_dialog: bool,
    pub show_save_dialog: bool,
    /// View toggles.
    pub show_items_2d: bool,
    pub show_ceilings_3d: bool,
    /// Active VectorCraft brightness theme.
    pub theme: Theme,
    /// Dock tab (persisted per session only).
    pub dock_tab: DockTab,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockTab {
    Properties,
    Levels,
    Library,
    Schedule,
    Model,
}

impl DockTab {
    pub const ALL: [DockTab; 5] = [DockTab::Properties, DockTab::Levels, DockTab::Library, DockTab::Schedule, DockTab::Model];
    pub fn label(self) -> &'static str {
        match self {
            DockTab::Properties => "Properties",
            DockTab::Levels => "Levels",
            DockTab::Library => "Library",
            DockTab::Schedule => "Schedule",
            DockTab::Model => "Model",
        }
    }
}

impl BimcraftApp {
    pub fn new() -> Self {
        Self {
            session: Session::default(),
            save_path: "model.bimcraft.json".into(),
            show_open_dialog: false,
            show_save_dialog: false,
            show_items_2d: true,
            show_ceilings_3d: true,
            theme: Theme::Charcoal,
            dock_tab: DockTab::Properties,
        }
    }

    pub fn with_model(model: bimcraft_bim::Model) -> Self {
        let mut app = Self::new();
        let active = model.levels.first().map(|l| l.id);
        app.session.model = model;
        app.session.active_level = active;
        app.session.status = "Model loaded".into();
        app
    }

    pub fn session_reset(&mut self) {
        self.session = Session::with_model(bimcraft_bim::Model::new());
        self.session.status = "New empty model".into();
    }

    pub fn session_sample(&mut self) {
        self.session = Session::default();
        self.session.status = "Sample house loaded".into();
    }

    pub fn zoom_to_fit(&mut self) {
        match self.session.model.bounds() {
            Some((min, max)) => {
                let cx = (min.x + max.x) / 2.0;
                let cy = (min.y + max.y) / 2.0;
                let span = (max.x - min.x).max(max.y - min.y).max(4.0);
                match self.session.view {
                    ViewMode::Plan2D => {
                        self.session.plan.center = Vec2::new(cx, cy);
                        self.session.plan.zoom = (900.0 / span).clamp(6.0, 400.0);
                    }
                    ViewMode::Perspective3D | ViewMode::Walk => {
                        self.session.camera.target = Vec3::new(cx, cy, (min.z + max.z) / 2.0);
                        self.session.camera.distance = (span * 1.6).clamp(6.0, 200.0);
                    }
                    ViewMode::Section => {
                        // Fit the u range of the active section (fallback: model span).
                        let len = self
                            .session
                            .active_or_first_section()
                            .and_then(|sid| self.session.model.section(sid))
                            .map(|s| s.length())
                            .unwrap_or(span)
                            .max(2.0);
                        self.session.plan.center = Vec2::new(len / 2.0, 1.5);
                        self.session.plan.zoom = (900.0 / len).clamp(6.0, 400.0);
                    }
                }
                self.session.status = "Zoom to fit".into();
            }
            None => self.session.status = "Nothing to fit".into(),
        }
    }

    /// Open a native file by path (desktop; the web edition autosaves to
    /// localStorage via eframe storage instead).
    pub fn open_file(&mut self, path: &str) {
        match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|text| bimcraft_io::load_json(&text).map_err(|e| e.0)) {
            Ok(model) => {
                self.show_open_dialog = false;
                let active = model.levels.first().map(|l| l.id);
                self.session.model = model;
                self.session.active_level = active;
                self.save_path = path.to_string();
                self.session.status = format!("Opened {path}");
            }
            Err(e) => self.session.status = format!("Open failed: {e}"),
        }
    }

    pub fn save_file(&mut self, path: &str) {
        let json = bimcraft_io::save_json(&self.session.model);
        match std::fs::write(path, json) {
            Ok(()) => {
                self.show_save_dialog = false;
                self.save_path = path.to_string();
                self.session.status = format!("Saved {path}");
            }
            Err(e) => self.session.status = format!("Save failed: {e}"),
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::Key;
        let (w, d, n, f, c, s, i, m, x, p, v, esc, enter, tab, del, z, y, r, g, home, ctrl, shift) = ctx.input(|i| {
            (
                i.key_pressed(Key::W),
                i.key_pressed(Key::D),
                i.key_pressed(Key::N),
                i.key_pressed(Key::F),
                i.key_pressed(Key::C),
                i.key_pressed(Key::S),
                i.key_pressed(Key::I),
                i.key_pressed(Key::M),
                i.key_pressed(Key::X),
                i.key_pressed(Key::P),
                i.key_pressed(Key::V),
                i.key_pressed(Key::Escape),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Tab),
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Z),
                i.key_pressed(Key::Y),
                i.key_pressed(Key::R),
                i.key_pressed(Key::G),
                i.key_pressed(Key::Home),
                i.modifiers.ctrl,
                i.modifiers.shift,
            )
        });
        if ctrl && z {
            self.session.undo();
        } else if ctrl && (y || (z && shift)) {
            self.session.redo();
        }
        // Walk mode: V toggles, Esc leaves (back to the orbit view).
        if v {
            let next = if self.session.view == ViewMode::Walk { ViewMode::Perspective3D } else { ViewMode::Walk };
            self.session.set_view(next);
        }
        if self.session.view == ViewMode::Walk {
            if esc {
                self.session.set_view(ViewMode::Perspective3D);
            }
            return;
        }
        if !ctrl {
            if w {
                self.session.set_tool(Tool::Wall);
            }
            if d {
                self.session.set_tool(Tool::Door);
            }
            if n {
                self.session.set_tool(Tool::Window);
            }
            if f {
                self.session.set_tool(Tool::Floor);
            }
            if c {
                self.session.set_tool(Tool::Ceiling);
            }
            if i {
                self.session.set_tool(Tool::Item);
                self.dock_tab = DockTab::Library;
            }
            if m {
                self.session.set_tool(Tool::Measure);
            }
            if x {
                self.session.set_tool(Tool::SectionLine);
            }
            if p {
                self.session.set_tool(Tool::Paint);
                self.dock_tab = DockTab::Properties;
            }
            if s {
                if esc && !self.session.pending.is_empty() {
                    // fall through to Esc handling below
                } else {
                    self.session.set_tool(Tool::Select);
                }
            }
            if r {
                // Rotate selected item 90° (Shift: 15°).
                if let Some(id) = self.session.selected_element() {
                    self.session.rotate_item(id, if shift { 15.0_f64.to_radians() } else { std::f64::consts::FRAC_PI_2 });
                }
            }
            if g {
                self.session.snap_grid = !self.session.snap_grid;
                self.session.status = format!("Grid snap {}", if self.session.snap_grid { "on" } else { "off" });
            }
        }
        if esc {
            if self.session.tool == Tool::Paint {
                self.session.set_tool(Tool::Select);
            } else if !self.session.pending.is_empty() || self.session.measure.is_some() {
                self.session.cancel_pending();
            } else {
                self.session.select(bimcraft_engine::Selection::None);
            }
        }
        if enter {
            self.session.finish_draft();
        }
        if tab {
            let next = match self.session.view {
                ViewMode::Plan2D => ViewMode::Perspective3D,
                ViewMode::Perspective3D => ViewMode::Section,
                ViewMode::Section => ViewMode::Plan2D,
                ViewMode::Walk => ViewMode::Plan2D,
            };
            self.session.set_view(next);
        }
        if del && !ctrl {
            self.session.delete_selected();
        }
        if home {
            self.zoom_to_fit();
        }
    }
}

impl Default for BimcraftApp {
    fn default() -> Self {
        Self::new()
    }
}

impl BimcraftApp {
    /// Draw one frame. Platform shells (apps/bimcraft, apps/bimcraft-web)
    /// implement `eframe::App::ui` and delegate here, keeping this crate
    /// a thin egui layer over the engine session.
    pub fn ui(&mut self, root: &mut egui::Ui) {
        let ctx = root.ctx().clone();
        theme::apply(&ctx, self.theme);
        self.handle_shortcuts(&ctx);

        chrome::status_bar(root, self);
        chrome::app_bar(root, self);
        chrome::toolbar(root, self);
        chrome::dock(root, self);
        chrome::file_dialogs(&ctx, self);

        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(theme::tokens_for(self.theme).pasteboard)).show(root, |ui| {
            canvas::show(ui, self);
        });

        // Continuously repaint while a tool has pending input (rubber band)
        // or the walkthrough is active (WASD movement).
        if !self.session.pending.is_empty() || self.session.dragging() || self.session.tool == Tool::Measure || self.session.view == ViewMode::Walk {
            ctx.request_repaint();
        }
    }
}
