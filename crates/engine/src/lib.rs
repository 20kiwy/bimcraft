//! BimCraft engine: session state, tools, snapping, undo/redo, cameras.
//!
//! Modeled on the same seams as the storytold Craft apps (MIT OR Apache-2.0):
//! a UI-agnostic `Session` owning the model, command entry points, and
//! snapshot-based history. The parity roadmap (docs/PARITY_ROADMAP.md) maps
//! how this grows toward `cadcraft-engine`'s role.

use bimcraft_bim::{ElementData, ElementKind, GridAxis, Id, ItemKind, Model, Opening};
use bimcraft_geom::{dist_point_segment_xy, ray_ground, Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    /// Orthographic top-down plan.
    Plan2D,
    /// Orbiting perspective view.
    Perspective3D,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Wall,
    Door,
    Window,
    Floor,
    Ceiling,
    Item,
    Measure,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Wall => "Wall",
            Tool::Door => "Door",
            Tool::Window => "Window",
            Tool::Floor => "Floor Slab",
            Tool::Ceiling => "Ceiling",
            Tool::Item => "Item",
            Tool::Measure => "Measure",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Tool::Select => "Click: select · drag: move · drag opening: shift along wall · Esc: deselect",
            Tool::Wall => "Click: chain points · Enter / double-click: finish · Esc: cancel",
            Tool::Door | Tool::Window => "Click on a wall to place",
            Tool::Floor | Tool::Ceiling => "Click polygon points · Enter / double-click: close",
            Tool::Item => "Pick a kind in Library, then click to place · R: rotate",
            Tool::Measure => "Click two points to measure",
        }
    }
}

/// Orbit camera for the 3D view (Z-up).
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub target: Vec3,
    pub distance: f64,
    pub yaw: f64,
    pub pitch: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self { target: Vec3::new(4.5, 3.5, 1.0), distance: 26.0, yaw: 0.8, pitch: 0.7 }
    }
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let p = self.pitch.clamp(0.05, 1.55);
        Vec3::new(
            self.target.x + self.distance * p.cos() * self.yaw.cos(),
            self.target.y + self.distance * p.cos() * self.yaw.sin(),
            self.target.z + self.distance * p.sin(),
        )
    }

    /// View basis: (right, up, forward) with forward pointing at the target.
    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let eye = self.eye();
        let f = (self.target - eye).normalize();
        let up = Vec3::new(0.0, 0.0, 1.0);
        let mut r = f.cross(up).normalize();
        if r.length() < 1e-6 {
            r = Vec3::new(1.0, 0.0, 0.0);
        }
        let u = r.cross(f).normalize();
        (r, u, f)
    }

    /// Unproject a screen point (px, with origin at rect top-left) onto the Z=0 plane.
    pub fn pick_ground(&self, sx: f64, sy: f64, rect_w: f64, rect_h: f64, fov_y: f64) -> Option<Vec3> {
        let (r, u, f) = self.basis();
        let eye = self.eye();
        let focal = rect_h / (2.0 * (fov_y / 2.0).tan());
        let ndc_x = sx - rect_w / 2.0;
        let ndc_y = rect_h / 2.0 - sy;
        let d = (f * focal + r * ndc_x + u * ndc_y).normalize();
        ray_ground(eye, d)
    }
}

/// Plan view state for the 2D mode.
#[derive(Clone, Copy, Debug)]
pub struct Plan {
    pub center: Vec2,
    /// Pixels per meter.
    pub zoom: f64,
}

impl Default for Plan {
    fn default() -> Self {
        Self { center: Vec2::new(4.5, 3.5), zoom: 48.0 }
    }
}

/// What is currently selected. Openings select as (wall, opening) pairs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Element(Id),
    Grid(Id),
    Level(Id),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measure {
    pub a: Vec2,
    pub b: Vec2,
}

impl Measure {
    pub fn length(&self) -> f64 {
        (self.b - self.a).length()
    }
}

/// Interactive session: model + history + view + tool state.
#[derive(Clone, Debug)]
pub struct Session {
    pub model: Model,
    undo_stack: Vec<Model>,
    redo_stack: Vec<Model>,
    pub selection: Selection,
    pub tool: Tool,
    pub view: ViewMode,
    pub camera: Camera,
    pub plan: Plan,
    pub active_level: Option<Id>,
    /// Pending clicks of the active tool (wall chain points, polygon points).
    pub pending: Vec<Vec2>,
    /// Last known world-space cursor position (status bar + tool preview).
    pub cursor: Vec3,
    /// Snapped plan cursor for tool previews (2D only).
    pub cursor_plan_snapped: Option<Vec2>,
    /// Snapping flags.
    pub snap_grid: bool,
    pub snap_endpoints: bool,
    /// Active item kind for the Item tool (set from the Library dock tab).
    pub item_kind: ItemKind,
    /// Placement rotation for new items (radians).
    pub item_rotation: f64,
    /// Finished measurement (measure tool).
    pub measure: Option<Measure>,
    /// Element dragged with the Select tool: records drag start.
    pub drag_anchor: Option<Vec2>,
    pub status: String,
}

impl Default for Session {
    fn default() -> Self {
        let model = bimcraft_bim::sample();
        let active_level = model.levels.first().map(|l| l.id);
        Self {
            model,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selection: Selection::None,
            tool: Tool::Select,
            view: ViewMode::Plan2D,
            camera: Camera::default(),
            plan: Plan::default(),
            active_level,
            pending: Vec::new(),
            cursor: Vec3::ZERO,
            cursor_plan_snapped: None,
            snap_grid: true,
            snap_endpoints: true,
            item_kind: ItemKind::Table,
            item_rotation: 0.0,
            measure: None,
            drag_anchor: None,
            status: "Sample house loaded — New Model starts empty".into(),
        }
    }
}

impl Session {
    /// A session over an existing model (e.g. loaded from disk).
    pub fn with_model(model: Model) -> Self {
        let active_level = model.levels.first().map(|l| l.id);
        Self { model, active_level, ..Default::default() }
    }

    /// Open an undo snapshot; use before direct model mutations (e.g. from the
    /// properties dock) so the change can be undone.
    pub fn begin_edit(&mut self) {
        self.snapshot();
    }

    fn snapshot(&mut self) {
        self.undo_stack.push(self.model.clone());
        if self.undo_stack.len() > 128 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(m) = self.undo_stack.pop() {
            self.redo_stack.push(std::mem::replace(&mut self.model, m));
            self.prune_selection();
            self.status = "Undo".into();
        }
    }

    pub fn redo(&mut self) {
        if let Some(m) = self.redo_stack.pop() {
            self.undo_stack.push(std::mem::replace(&mut self.model, m));
            self.prune_selection();
            self.status = "Redo".into();
        }
    }

    fn prune_selection(&mut self) {
        if let Selection::Element(id) = self.selection {
            if self.model.element(id).is_none() {
                self.selection = Selection::None;
            }
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.pending.clear();
        self.drag_anchor = None;
        if tool != Tool::Measure {
            // Keep the last measurement visible until re-measured.
        }
        self.status = tool.label().into();
    }

    pub fn set_view(&mut self, view: ViewMode) {
        self.view = view;
        self.pending.clear();
        self.drag_anchor = None;
    }

    pub fn select(&mut self, sel: Selection) {
        self.selection = sel;
        if let Selection::Level(id) = sel {
            self.active_level = Some(id);
        }
    }

    pub fn selected_element(&self) -> Option<Id> {
        match self.selection {
            Selection::Element(id) => Some(id),
            _ => None,
        }
    }

    pub fn cancel_pending(&mut self) {
        let mut cancelled = false;
        if !self.pending.is_empty() {
            self.pending.clear();
            cancelled = true;
        }
        if self.tool == Tool::Measure {
            self.measure = None;
            cancelled = true;
        }
        if cancelled {
            self.status = "Cancelled".into();
        }
    }

    pub fn snap(&self, p: Vec2) -> Vec2 {
        // Endpoint snap wins over grid snap.
        if self.snap_endpoints {
            let tol = 0.35;
            let mut best: Option<(Vec2, f64)> = None;
            for e in &self.model.elements {
                let candidates: &[Vec2] = match &e.data {
                    ElementData::Wall(w) => &w.pts,
                    ElementData::Slab(s) => &s.pts,
                    ElementData::Ceiling(c) => &c.pts,
                    _ => &[],
                };
                for q in candidates {
                    let d = p.dist(*q);
                    if d < tol && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                        best = Some((*q, d));
                    }
                }
            }
            if let Some((q, _)) = best {
                return q;
            }
        }
        if self.snap_grid {
            let g = 0.5;
            return Vec2::new((p.x / g).round() * g, (p.y / g).round() * g);
        }
        p
    }

    pub fn add_wall(&mut self, pts: Vec<Vec2>) {
        if pts.len() < 2 {
            return;
        }
        let lvl = self.active_level_id();
        self.snapshot();
        let id = self.model.add_wall(lvl, pts, 0.2, self.level_height());
        self.selection = Selection::Element(id);
        self.status = format!("Wall {id} added");
    }

    pub fn place_opening(&mut self, kind: ElementKind, world: Vec2) {
        let Some((wall, s)) = self.model.host_wall_at(world, 1.0) else {
            self.status = "Click on a wall".into();
            return;
        };
        self.snapshot();
        let o = match kind {
            ElementKind::Door => Opening { wall, s, width: 0.9, height: 2.1, sill: 0.0 },
            _ => Opening { wall, s, width: 1.2, height: 1.4, sill: 0.9 },
        };
        let id = self.model.add_opening(kind, o);
        self.selection = Selection::Element(id);
        self.status = format!("{} {id} placed", if kind == ElementKind::Door { "Door" } else { "Window" });
    }

    /// Shift the selected opening along its host wall (drag or slider).
    pub fn shift_selected_opening(&mut self, new_s: f64) {
        let Some(id) = self.selected_element() else { return };
        if self.drag_anchor.is_none() {
            self.snapshot();
        }
        if self.model.shift_opening(id, new_s).is_some() {
            let name = self.model.element(id).map(|e| e.name.clone()).unwrap_or_default();
            if let Some(ElementData::Opening(o)) = self.model.element(id).map(|e| e.data.clone()) {
                self.status = format!("{name} shifted to s = {:.2} m", o.s);
            }
        }
    }

    pub fn add_plate(&mut self, kind: ElementKind, pts: Vec<Vec2>) {
        if !Model::poly_valid(&pts) {
            self.status = "Degenerate polygon".into();
            return;
        }
        let lvl = self.active_level_id();
        self.snapshot();
        let id = match kind {
            ElementKind::Slab => self.model.add_slab(lvl, pts, 0.25, 0.0),
            _ => self.model.add_ceiling(lvl, pts, 0.1, self.level_height() - 0.1),
        };
        self.selection = Selection::Element(id);
        self.status = format!("{} added", self.model.element(id).map(|e| e.name.clone()).unwrap_or_default());
    }

    pub fn place_item(&mut self, world: Vec2) {
        let lvl = self.active_level_id();
        self.snapshot();
        let id = self.model.add_item(lvl, self.item_kind, world, self.item_rotation);
        self.selection = Selection::Element(id);
        self.status = format!("{} placed", self.model.element(id).map(|e| e.name.clone()).unwrap_or_default());
    }

    pub fn rotate_item(&mut self, id: Id, by: f64) {
        if self.drag_anchor.is_none() {
            self.snapshot();
        }
        if let Some(ElementData::Item(i)) = self.model.element_mut(id).map(|e| &mut e.data) {
            i.rotation += by;
        }
    }

    pub fn delete_selected(&mut self) {
        let id = match self.selection {
            Selection::Element(id) => id,
            Selection::Grid(id) => {
                self.snapshot();
                self.model.grids.retain(|g| g.id != id);
                self.selection = Selection::None;
                self.status = "Grid deleted".into();
                return;
            }
            _ => return,
        };
        self.snapshot();
        self.model.delete(id);
        self.selection = Selection::None;
        self.status = "Deleted".into();
    }

    pub fn add_level(&mut self) {
        self.snapshot();
        let idx = self.model.levels.len();
        let elev = self.model.levels.last().map(|l| l.elevation + l.height).unwrap_or(0.0);
        let id = self.model.add_level(&format!("Level {idx}"), elev, 3.0);
        self.active_level = Some(id);
        self.status = format!("Level {idx} added");
    }

    pub fn set_active_level(&mut self, id: Id) {
        self.active_level = Some(id);
    }

    pub fn active_level_id(&self) -> Id {
        self.model.active_or_first_level(self.active_level).id
    }

    pub fn level_height(&self) -> f64 {
        self.model.active_or_first_level(self.active_level).height.max(0.5)
    }

    pub fn level_elevation(&self) -> f64 {
        self.model.active_or_first_level(self.active_level).elevation
    }

    pub fn add_grid(&mut self, name: &str, axis: GridAxis, position: f64) {
        self.snapshot();
        let id = self.model.add_grid(name, axis, position);
        self.selection = Selection::Grid(id);
        self.status = format!("Grid {name} added");
    }

    /// Feed a plan-space click into the active tool. `p` is in world XY meters
    /// (already snapped by the canvas).
    pub fn click_plan(&mut self, p: Vec2) {
        match self.tool {
            Tool::Select => {
                let hit = self.pick_plan(p);
                self.select(hit);
                self.status = match hit {
                    Selection::Element(id) => format!("Selected {}", self.model.element(id).map(|e| e.name.clone()).unwrap_or_default()),
                    Selection::Grid(id) => {
                        format!("Selected grid {}", self.model.grids.iter().find(|g| g.id == id).map(|g| g.name.clone()).unwrap_or_default())
                    }
                    Selection::Level(id) => format!("Selected level {}", self.model.level(id).map(|l| l.name.clone()).unwrap_or_default()),
                    Selection::None => "Nothing selected".into(),
                };
            }
            Tool::Wall => {
                self.pending.push(p);
                self.status = format!("Wall: {} points — Enter to finish", self.pending.len());
            }
            Tool::Door => self.place_opening(ElementKind::Door, p),
            Tool::Window => self.place_opening(ElementKind::Window, p),
            Tool::Floor | Tool::Ceiling => {
                self.pending.push(p);
                self.status =
                    format!("{}: {} points — Enter to close", if self.tool == Tool::Floor { "Floor" } else { "Ceiling" }, self.pending.len());
            }
            Tool::Item => self.place_item(p),
            Tool::Measure => match self.measure {
                None => {
                    self.measure = Some(Measure { a: p, b: p });
                    self.status = "Measure: pick second point".into();
                }
                Some(m) if m.a == m.b => {
                    self.measure = Some(Measure { a: m.a, b: p });
                    self.status = format!("Measured {:.2} m — Esc to clear", m.a.dist(p));
                }
                Some(_) => {
                    self.measure = None;
                    self.status = "Measure: pick first point".into();
                }
            },
        }
    }

    /// Finish the current draft (Enter / double-click): wall chain or polygon.
    pub fn finish_draft(&mut self) {
        match self.tool {
            Tool::Wall if self.pending.len() >= 2 => {
                let pts = std::mem::take(&mut self.pending);
                self.add_wall(pts);
            }
            Tool::Floor | Tool::Ceiling if self.pending.len() >= 3 => {
                let pts = std::mem::take(&mut self.pending);
                let kind = if self.tool == Tool::Floor { ElementKind::Slab } else { ElementKind::Ceiling };
                self.add_plate(kind, pts);
            }
            _ => {}
        }
    }

    /// Begin dragging the selected element (called on drag start over it).
    pub fn begin_drag(&mut self, world: Vec2) -> bool {
        if self.tool != Tool::Select {
            return false;
        }
        match self.selection {
            Selection::Element(id) => {
                // Openings shift along their host wall; everything else moves.
                self.snapshot();
                self.drag_anchor = Some(world);
                let _ = id;
                true
            }
            _ => false,
        }
    }

    /// Drag update: translate the selected element, or shift an opening along
    /// its host wall ("windows to be shifted").
    pub fn drag_to(&mut self, world: Vec2) {
        let Some(anchor) = self.drag_anchor else { return };
        let Some(id) = self.selected_element() else { return };
        match self.model.element(id).map(|e| e.data.clone()) {
            Some(ElementData::Opening(o)) => {
                if let Some(w) = self.model.wall(o.wall) {
                    let (s, _) = w.project(world);
                    self.shift_selected_opening(s);
                }
                self.drag_anchor = Some(world);
            }
            Some(_) => {
                let delta = world - anchor;
                if delta.length() > 1e-9 {
                    self.model.translate(id, delta);
                    self.drag_anchor = Some(world);
                    self.status = "Moved".into();
                }
            }
            None => {}
        }
    }

    pub fn end_drag(&mut self) {
        self.drag_anchor = None;
    }

    pub fn dragging(&self) -> bool {
        self.drag_anchor.is_some()
    }

    /// Nearest plan element at `p` (walls, openings, plates, items, grids).
    pub fn pick_plan(&self, p: Vec2) -> Selection {
        let tol = 0.35;
        let mut best: Option<(Id, f64)> = None;
        for e in &self.model.elements {
            let d = match &e.data {
                ElementData::Wall(w) => w.segments().map(|(a, b)| dist_point_segment_xy(p, a, b)).fold(f64::INFINITY, f64::min),
                ElementData::Opening(o) => match self.model.opening_rect(o) {
                    Some(q) => {
                        let c = (q[0] + q[2]) * 0.5;
                        if point_in_poly(p, &[q[0].xy(), q[1].xy(), q[2].xy(), q[3].xy()]) {
                            0.05
                        } else {
                            p.dist(c.xy())
                        }
                    }
                    None => f64::INFINITY,
                },
                ElementData::Slab(_) | ElementData::Ceiling(_) => {
                    let pts: &[Vec2] = match &e.data {
                        ElementData::Slab(s) => &s.pts,
                        ElementData::Ceiling(c) => &c.pts,
                        _ => unreachable!(),
                    };
                    if point_in_poly(p, pts) {
                        0.3
                    } else {
                        pts.windows(2)
                            .map(|w| dist_point_segment_xy(p, w[0], w[1]))
                            .chain(std::iter::once(dist_point_segment_xy(p, pts[pts.len() - 1], pts[0])))
                            .fold(f64::INFINITY, f64::min)
                    }
                }
                ElementData::Item(i) => {
                    let bb = i.footprint();
                    if bb.contains(p) {
                        0.25
                    } else {
                        p.dist(i.pos)
                    }
                }
            };
            // Openings win over walls (hosted in them): bias wall distances up.
            let d = if matches!(&e.data, ElementData::Wall(_)) { d + 0.12 } else { d };
            if d < tol && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((e.id, d));
            }
        }
        if let Some((id, _)) = best {
            return Selection::Element(id);
        }
        // Datum grids (near the dashed line).
        for g in &self.model.grids {
            let d = match g.axis {
                GridAxis::V => (p.x - g.position).abs(),
                GridAxis::H => (p.y - g.position).abs(),
            };
            if d < 0.15 {
                return Selection::Grid(g.id);
            }
        }
        Selection::None
    }
}

pub fn point_in_poly(p: Vec2, pts: &[Vec2]) -> bool {
    let mut inside = false;
    let n = pts.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let a = pts[j];
        let b = pts[i];
        if (b.y > p.y) != (a.y > p.y) && p.x < (a.x - b.x) * (p.y - b.y) / (a.y - b.y) + b.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_restores_model() {
        let mut s = Session::default();
        let n0 = s.model.elements.len();
        s.set_tool(Tool::Wall);
        s.click_plan(Vec2::ZERO);
        s.click_plan(Vec2::new(4.0, 0.0));
        s.finish_draft();
        assert_eq!(s.model.elements.len(), n0 + 1);
        s.undo();
        assert_eq!(s.model.elements.len(), n0);
        s.redo();
        assert_eq!(s.model.elements.len(), n0 + 1);
    }

    #[test]
    fn wall_chain_creates_one_polyline() {
        let mut s = Session::default();
        let n0 = s.model.walls().count();
        s.set_tool(Tool::Wall);
        s.click_plan(Vec2::ZERO);
        s.click_plan(Vec2::new(5.0, 0.0));
        s.click_plan(Vec2::new(5.0, 3.0));
        s.finish_draft();
        assert_eq!(s.model.walls().count(), n0 + 1);
        let w = s.model.walls().last().unwrap();
        assert!(matches!(&w.data, ElementData::Wall(w) if w.pts.len() == 3));
    }

    #[test]
    fn door_places_on_wall() {
        let mut s = Session::default();
        let n0 = s.model.elements.iter().filter(|e| e.kind == ElementKind::Door).count();
        s.set_tool(Tool::Door);
        s.click_plan(Vec2::new(2.0, 0.05));
        assert_eq!(s.model.elements.iter().filter(|e| e.kind == ElementKind::Door).count(), n0 + 1);
    }

    #[test]
    fn window_drags_along_wall() {
        let mut s = Session::default();
        s.set_tool(Tool::Window);
        s.click_plan(Vec2::new(2.0, 0.02)); // sample south wall at y=0
        let id = s.selected_element().unwrap();
        let s0 = match s.model.element(id).unwrap().data.clone() {
            ElementData::Opening(o) => o.s,
            _ => panic!(),
        };
        // Drag to x=7 on the same wall (dragging is a Select-tool interaction).
        s.set_tool(Tool::Select);
        s.begin_drag(Vec2::new(2.0, 0.0));
        s.drag_to(Vec2::new(7.0, 0.0));
        s.end_drag();
        let s1 = match s.model.element(id).unwrap().data.clone() {
            ElementData::Opening(o) => o.s,
            _ => panic!(),
        };
        assert!(s1 > s0 + 2.0, "window should shift along the wall");
        assert!(s1 <= 9.0 - 0.65);
    }

    #[test]
    fn item_placement_and_rotate() {
        let mut s = Session::default();
        s.set_tool(Tool::Item);
        s.click_plan(Vec2::new(4.0, 4.0));
        let id = s.selected_element().unwrap();
        let rot0 = match s.model.element(id).unwrap().data.clone() {
            ElementData::Item(i) => i.rotation,
            _ => panic!(),
        };
        s.rotate_item(id, std::f64::consts::FRAC_PI_2);
        let rot1 = match s.model.element(id).unwrap().data.clone() {
            ElementData::Item(i) => i.rotation,
            _ => panic!(),
        };
        assert!((rot1 - rot0 - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }

    #[test]
    fn measure_tool_two_clicks() {
        let mut s = Session::default();
        s.set_tool(Tool::Measure);
        s.click_plan(Vec2::ZERO);
        assert!(s.measure.is_some());
        s.click_plan(Vec2::new(3.0, 4.0));
        let m = s.measure.unwrap();
        assert!((m.length() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn polygon_floor_closes() {
        let mut s = Session::default();
        let n0 = s.model.elements.iter().filter(|e| e.kind == ElementKind::Slab).count();
        s.set_tool(Tool::Floor);
        s.click_plan(Vec2::ZERO);
        s.click_plan(Vec2::new(4.0, 0.0));
        s.click_plan(Vec2::new(4.0, 3.0));
        s.click_plan(Vec2::new(0.0, 3.0));
        s.finish_draft();
        assert_eq!(s.model.elements.iter().filter(|e| e.kind == ElementKind::Slab).count(), n0 + 1);
    }

    #[test]
    fn snap_prefers_endpoints_over_grid() {
        let s = Session::default();
        let p = s.snap(Vec2::new(0.13, 0.02)); // near sample wall vertex (0,0)
        assert_eq!(p, Vec2::ZERO);
        let q = s.snap(Vec2::new(1.13, 1.87));
        assert_eq!(q, Vec2::new(1.0, 2.0));
    }
}
