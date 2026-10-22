//! BimCraft building information model: levels, grids, parametric walls,
//! hosted openings (doors + windows), floor slabs, ceilings, items, rooms,
//! section/elevation views, schedule.
//!
//! Clean-room BIM data model for the BIMCraft Rust edition, mapped to the
//! JavaScript edition's modules in docs/PARITY_ROADMAP.md:
//! `model-levels` → [Level], `model-grids` → [Grid], `model-walls` → [Wall],
//! `model-openings`/`opening-types` → [Opening], `model-floors`/`model-slabs`
//! → [Slab], ceilings → [Ceiling], `libitem`/`assets-bim` → [Item],
//! `model-rooms` → [rooms] + [Room], section/elevation views → [Section],
//! `model-schedule` → [Model::schedule].

pub mod dxf;
pub mod rooms;

use bimcraft_geom::{dist_point_segment_xy, poly_area, poly_centroid, triangulate, Aabb2, Vec2, Vec3};
pub use rooms::{plan_loops, PlanLoops, RoomLoop};
use serde::{Deserialize, Serialize};

pub type Id = u64;

/// A storey of the building. Elevation and height are in meters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Level {
    pub id: Id,
    pub name: String,
    pub elevation: f64,
    pub height: f64,
}

/// A datum line on the plan grid (structural grid, Blender-like reference).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridAxis {
    /// Line runs along Y at x = `position` (vertical on plan).
    V,
    /// Line runs along X at y = `position` (horizontal on plan).
    H,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grid {
    pub id: Id,
    pub name: String,
    pub axis: GridAxis,
    pub position: f64,
}

/// Kind tag for fast picking/filtering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementKind {
    Wall,
    Door,
    Window,
    Slab,
    Ceiling,
    Item,
}

/// A section / elevation view: a line on the plan of one level with a view
/// depth band. The projected view is built by `bimcraft-render::build_section`
/// (parity: the JS edition's section/elevation views, `res-views.js` /
/// `section-volume.js`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Section {
    pub id: Id,
    pub level: Id,
    pub name: String,
    /// Section line start point on the plan (meters).
    pub a: Vec2,
    /// Section line end point on the plan (meters).
    pub b: Vec2,
    /// Half-depth of the view band on each side of the line (meters).
    pub depth: f64,
}

impl Section {
    /// Plan-space direction along the section line (normalized).
    pub fn dir(&self) -> Vec2 {
        (self.b - self.a).normalize()
    }

    pub fn length(&self) -> f64 {
        (self.b - self.a).length()
    }

    /// Signed perpendicular distance of a plan point from the section line.
    pub fn signed_dist(&self, p: Vec2) -> f64 {
        let d = self.dir();
        (p - self.a).x * d.y - (p - self.a).y * d.x
    }

    /// Projection parameter `u` (meters along the line) of a plan point.
    pub fn along(&self, p: Vec2) -> f64 {
        (p - self.a).dot(self.dir())
    }

    /// Convert (u, z) section-space coordinates back to plan (XY) space:
    /// the point on the section line at distance `u` from [Section::a].
    pub fn plan_point(&self, u: f64) -> Vec2 {
        self.a + self.dir() * u
    }
}

/// A named room annotation. Geometry is *derived* from the walls on the level
/// ([rooms::plan_loops]); the annotation attaches a name and finishes to the
/// loop containing [Room::label] (parity: `model-rooms.js` rooms + names).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: Id,
    pub level: Id,
    pub name: String,
    pub floor_finish: String,
    pub ceiling_finish: String,
    /// Plan point inside the room this annotation names.
    pub label: Vec2,
}

/// Parametric wall: polyline centerline on a level, constant thickness/height.
/// Multi-segment walls join with miters (parity: `model-walls.js`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wall {
    pub level: Id,
    pub pts: Vec<Vec2>,
    pub thickness: f64,
    pub height: f64,
}

impl Wall {
    pub fn segments(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.pts.windows(2).map(|w| (w[0], w[1]))
    }

    pub fn total_length(&self) -> f64 {
        self.segments().map(|(a, b)| (b - a).length()).sum()
    }

    /// Walk `s` meters along the polyline: returns (position, segment index).
    pub fn point_at(&self, mut s: f64) -> (Vec2, usize) {
        for (i, (a, b)) in self.segments().enumerate() {
            let len = (b - a).length();
            if s <= len || i == self.pts.len() - 2 {
                return (a + (b - a) * if len > 1e-9 { (s / len).clamp(0.0, 1.0) } else { 0.0 }, i);
            }
            s -= len;
        }
        (self.pts[self.pts.len() - 1], self.pts.len().saturating_sub(2))
    }

    /// Locate an arclength: returns (segment index, local arclength).
    pub fn locate(&self, mut s: f64) -> (usize, f64) {
        for (i, (a, b)) in self.segments().enumerate() {
            let len = (b - a).length();
            if s <= len || i == self.pts.len() - 2 {
                return (i, s.clamp(0.0, len));
            }
            s -= len;
        }
        (self.pts.len().saturating_sub(2), 0.0)
    }

    /// Project a world point onto the polyline: arclength `s` + distance.
    pub fn project(&self, p: Vec2) -> (f64, f64) {
        let mut best = (f64::INFINITY, 0.0);
        let mut acc = 0.0;
        for (a, b) in self.segments() {
            let d = dist_point_segment_xy(p, a, b);
            if d < best.0 {
                let t = bimcraft_geom::project_on_segment_xy(p, a, b);
                best = (d, acc + t * (b - a).length());
            }
            acc += (b - a).length();
        }
        (best.1, best.0)
    }

    /// Mitered outline offsets. For each vertex returns its offset point for
    /// the left side (+) and right side (-) at `half` thickness.
    fn offset_pts(&self, half: f64) -> (Vec<Vec2>, Vec<Vec2>) {
        let n = self.pts.len();
        let mut left = Vec::with_capacity(n);
        let mut right = Vec::with_capacity(n);
        for (j, &v) in self.pts.iter().enumerate() {
            let d_in = if j > 0 { (self.pts[j] - self.pts[j - 1]).normalize() } else { (self.pts[1] - self.pts[0]).normalize() };
            let d_out = if j + 1 < n { (self.pts[j + 1] - self.pts[j]).normalize() } else { d_in };
            let n_in = d_in.perp();
            let n_out = d_out.perp();
            let m = (n_in + n_out).normalize();
            let cos_half = m.dot(n_in).abs().max(0.35);
            let scale = half / cos_half;
            let (l, r) = if m.dot(n_in) >= 0.0 { (m * scale, m * -scale) } else { (m * -scale, m * scale) };
            left.push(v + l);
            right.push(v + r);
        }
        (left, right)
    }

    /// Wall body outline polygon (left side forward + right side back).
    pub fn outline(&self) -> Vec<Vec2> {
        let (left, right) = self.offset_pts(self.thickness / 2.0);
        let mut out = left;
        out.extend(right.iter().rev().copied());
        out
    }
}

/// Door or window hosted in a wall. `s` is the arclength position (meters)
/// along the wall centerline; `sill` is the height above the level floor
/// (0 for doors). Windows shift along the host wall (parity: the user asked
/// for "windows to be shifted" — drag in plan or use the properties slider).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Opening {
    pub wall: Id,
    pub s: f64,
    pub width: f64,
    pub height: f64,
    pub sill: f64,
}

/// Horizontal floor slab bound to a level. `elevation` is the top-surface
/// offset above the level elevation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Slab {
    pub level: Id,
    pub pts: Vec<Vec2>,
    pub thickness: f64,
    pub elevation: f64,
}

impl Slab {
    pub fn area(&self) -> f64 {
        poly_area(&self.pts).abs()
    }
}

/// Ceiling bound to a level. `elevation` is the underside height above the
/// level elevation (defaults to the level height).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ceiling {
    pub level: Id,
    pub pts: Vec<Vec2>,
    pub thickness: f64,
    pub elevation: f64,
}

impl Ceiling {
    pub fn area(&self) -> f64 {
        poly_area(&self.pts).abs()
    }
}

/// Furniture / equipment item (parity: `libitem.js`, `assets-bim.js`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub level: Id,
    pub kind: ItemKind,
    pub pos: Vec2,
    /// Rotation in radians (plan, around the footprint center).
    pub rotation: f64,
}

impl Item {
    pub fn footprint(&self) -> Aabb2 {
        let (w, d, _) = self.kind.preset();
        Aabb2::from_rotated_box(self.pos, w, d, self.rotation)
    }
}

/// Preset item kinds with fixed footprints (w × d × h in meters).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Table,
    Chair,
    Sofa,
    Bed,
    Wardrobe,
    KitchenUnit,
    Fridge,
    Stove,
    Sink,
    Toilet,
    Bathtub,
    Plant,
}

impl ItemKind {
    /// (width, depth, height) in meters.
    pub fn preset(self) -> (f64, f64, f64) {
        match self {
            ItemKind::Table => (1.6, 0.9, 0.75),
            ItemKind::Chair => (0.45, 0.45, 0.9),
            ItemKind::Sofa => (2.1, 0.9, 0.8),
            ItemKind::Bed => (1.6, 2.0, 0.6),
            ItemKind::Wardrobe => (1.2, 0.6, 2.1),
            ItemKind::KitchenUnit => (1.2, 0.6, 0.9),
            ItemKind::Fridge => (0.7, 0.7, 1.8),
            ItemKind::Stove => (0.6, 0.6, 0.9),
            ItemKind::Sink => (0.6, 0.5, 0.9),
            ItemKind::Toilet => (0.4, 0.65, 0.75),
            ItemKind::Bathtub => (1.7, 0.75, 0.55),
            ItemKind::Plant => (0.5, 0.5, 1.2),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Table => "Table",
            ItemKind::Chair => "Chair",
            ItemKind::Sofa => "Sofa",
            ItemKind::Bed => "Bed",
            ItemKind::Wardrobe => "Wardrobe",
            ItemKind::KitchenUnit => "Kitchen unit",
            ItemKind::Fridge => "Fridge",
            ItemKind::Stove => "Stove",
            ItemKind::Sink => "Sink",
            ItemKind::Toilet => "Toilet",
            ItemKind::Bathtub => "Bathtub",
            ItemKind::Plant => "Plant",
        }
    }

    pub const ALL: [ItemKind; 12] = [
        ItemKind::Table,
        ItemKind::Chair,
        ItemKind::Sofa,
        ItemKind::Bed,
        ItemKind::Wardrobe,
        ItemKind::KitchenUnit,
        ItemKind::Fridge,
        ItemKind::Stove,
        ItemKind::Sink,
        ItemKind::Toilet,
        ItemKind::Bathtub,
        ItemKind::Plant,
    ];
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ElementData {
    Wall(Wall),
    Opening(Opening),
    Slab(Slab),
    Ceiling(Ceiling),
    Item(Item),
}

impl ElementData {
    /// Shared view over plan-polygon elements (slabs and ceilings).
    /// Returns (level id, polygon points, thickness, elevation).
    pub fn plan_poly(&self) -> Option<(Id, &[Vec2], f64, f64)> {
        match self {
            ElementData::Slab(s) => Some((s.level, &s.pts, s.thickness, s.elevation)),
            ElementData::Ceiling(s) => Some((s.level, &s.pts, s.thickness, s.elevation)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Element {
    pub id: Id,
    pub kind: ElementKind,
    pub name: String,
    pub data: ElementData,
    /// Paint override as 0xRRGGBB (the Paint tool). `None` = palette default.
    #[serde(default)]
    pub paint: Option<u32>,
}

impl Element {
    /// The paint override, if any.
    pub fn paint(&self) -> Option<u32> {
        self.paint
    }
}

/// The whole building model. Cheap clone enables snapshot undo.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Model {
    pub levels: Vec<Level>,
    pub grids: Vec<Grid>,
    pub elements: Vec<Element>,
    #[serde(default)]
    pub rooms: Vec<Room>,
    #[serde(default)]
    pub sections: Vec<Section>,
    next_id: Id,
}

impl Model {
    pub fn new() -> Self {
        let mut m = Self { next_id: 1, ..Default::default() };
        let l0 = m.alloc();
        m.levels.push(Level { id: l0, name: "Level 0".into(), elevation: 0.0, height: 3.0 });
        let g0 = m.alloc();
        m.grids.push(Grid { id: g0, name: "A".into(), axis: GridAxis::V, position: 0.0 });
        let g1 = m.alloc();
        m.grids.push(Grid { id: g1, name: "1".into(), axis: GridAxis::H, position: 0.0 });
        m
    }

    fn alloc(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Next id that would be allocated (for io round-trip tests and UI).
    pub fn next_id_peek(&self) -> Id {
        self.next_id
    }

    pub fn level(&self, id: Id) -> Option<&Level> {
        self.levels.iter().find(|l| l.id == id)
    }

    pub fn level_mut(&mut self, id: Id) -> Option<&mut Level> {
        self.levels.iter_mut().find(|l| l.id == id)
    }

    pub fn active_or_first_level(&self, id: Option<Id>) -> &Level {
        self.levels.iter().find(|l| Some(l.id) == id).unwrap_or(&self.levels[0])
    }

    pub fn element(&self, id: Id) -> Option<&Element> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn element_mut(&mut self, id: Id) -> Option<&mut Element> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    pub fn wall(&self, id: Id) -> Option<&Wall> {
        self.element(id).and_then(|e| match &e.data {
            ElementData::Wall(w) => Some(w),
            _ => None,
        })
    }

    pub fn walls(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter().filter(|e| e.kind == ElementKind::Wall)
    }

    /// Openings hosted by the given wall.
    pub fn openings_for_wall(&self, wall_id: Id) -> Vec<&Element> {
        self.elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Door | ElementKind::Window))
            .filter(|e| match &e.data {
                ElementData::Opening(o) => o.wall == wall_id,
                _ => false,
            })
            .collect()
    }

    pub fn items(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter().filter(|e| e.kind == ElementKind::Item)
    }

    pub fn section(&self, id: Id) -> Option<&Section> {
        self.sections.iter().find(|s| s.id == id)
    }

    pub fn section_mut(&mut self, id: Id) -> Option<&mut Section> {
        self.sections.iter_mut().find(|s| s.id == id)
    }

    pub fn add_section(&mut self, level: Id, name: &str, a: Vec2, b: Vec2, depth: f64) -> Id {
        let id = self.alloc();
        self.sections.push(Section { id, level, name: name.into(), a, b, depth });
        id
    }

    /// Detected room loops for one level (derived from the wall outlines).
    pub fn detect_rooms(&self, level: Id) -> PlanLoops {
        let walls: Vec<&Wall> = self
            .elements
            .iter()
            .filter_map(|e| match &e.data {
                ElementData::Wall(w) if w.level == level => Some(w),
                _ => None,
            })
            .collect();
        rooms::plan_loops(&rooms::wall_joined_outlines(walls))
    }

    /// The room annotation whose label sits inside a detected loop.
    pub fn room_annotation_at(&self, level: Id, p: Vec2) -> Option<&Room> {
        let loops = self.detect_rooms(level);
        let hit = rooms::room_at(&loops.rooms, p)?;
        self.rooms
            .iter()
            .filter(|r| r.level == level && rooms::in_poly(r.label.x, r.label.y, &hit.poly))
            .min_by(|a, b| (a.label - p).length().total_cmp(&(b.label - p).length()))
            .or_else(|| self.rooms.iter().filter(|r| r.level == level).min_by(|a, b| (a.label - p).length().total_cmp(&(b.label - p).length())))
    }

    pub fn add_room(&mut self, level: Id, name: &str, label: Vec2) -> Id {
        let id = self.alloc();
        self.rooms.push(Room { id, level, name: name.into(), floor_finish: "—".into(), ceiling_finish: "—".into(), label });
        id
    }

    pub fn delete_level(&mut self, id: Id) {
        self.levels.retain(|l| l.id != id);
        self.elements.retain(|e| e.level_id() != Some(id));
        self.rooms.retain(|r| r.level != id);
        self.sections.retain(|s| s.level != id);
    }

    pub fn add_level(&mut self, name: &str, elevation: f64, height: f64) -> Id {
        let id = self.alloc();
        self.levels.push(Level { id, name: name.into(), elevation, height });
        id
    }

    pub fn add_grid(&mut self, name: &str, axis: GridAxis, position: f64) -> Id {
        let id = self.alloc();
        self.grids.push(Grid { id, name: name.into(), axis, position });
        id
    }

    pub fn add_wall(&mut self, level: Id, pts: Vec<Vec2>, thickness: f64, height: f64) -> Id {
        let id = self.alloc();
        self.elements.push(Element {
            id,
            kind: ElementKind::Wall,
            name: format!("Wall {id}"),
            data: ElementData::Wall(Wall { level, pts, thickness, height }),
            paint: None,
        });
        id
    }

    pub fn add_opening(&mut self, kind: ElementKind, o: Opening) -> Id {
        let id = self.alloc();
        let name = match kind {
            ElementKind::Door => format!("Door {id}"),
            ElementKind::Window => format!("Window {id}"),
            _ => format!("Opening {id}"),
        };
        self.elements.push(Element { id, kind, name, data: ElementData::Opening(o), paint: None });
        id
    }

    pub fn add_slab(&mut self, level: Id, pts: Vec<Vec2>, thickness: f64, elevation: f64) -> Id {
        let id = self.alloc();
        self.elements.push(Element {
            id,
            kind: ElementKind::Slab,
            name: format!("Floor {id}"),
            data: ElementData::Slab(Slab { level, pts, thickness, elevation }),
            paint: None,
        });
        id
    }

    pub fn add_ceiling(&mut self, level: Id, pts: Vec<Vec2>, thickness: f64, elevation: f64) -> Id {
        let id = self.alloc();
        self.elements.push(Element {
            id,
            kind: ElementKind::Ceiling,
            name: format!("Ceiling {id}"),
            data: ElementData::Ceiling(Ceiling { level, pts, thickness, elevation }),
            paint: None,
        });
        id
    }

    pub fn add_item(&mut self, level: Id, kind: ItemKind, pos: Vec2, rotation: f64) -> Id {
        let id = self.alloc();
        self.elements.push(Element {
            id,
            kind: ElementKind::Item,
            name: format!("{} {id}", kind.name()),
            data: ElementData::Item(Item { level, kind, pos, rotation }),
            paint: None,
        });
        id
    }

    pub fn delete(&mut self, id: Id) {
        self.elements.retain(|e| e.id != id);
        // Cascade: openings die with their host wall.
        self.elements.retain(|e| match &e.data {
            ElementData::Opening(o) => o.wall != id,
            _ => true,
        });
        // Sections keep working; nothing cascades from plain elements.
    }

    pub fn delete_section(&mut self, id: Id) {
        self.sections.retain(|s| s.id != id);
    }

    pub fn delete_room(&mut self, id: Id) {
        self.rooms.retain(|r| r.id != id);
    }

    /// Move an element by `delta` in plan space (walls, plates, items).
    pub fn translate(&mut self, id: Id, delta: Vec2) {
        if let Some(e) = self.element_mut(id) {
            match &mut e.data {
                ElementData::Wall(w) => w.pts.iter_mut().for_each(|p| *p = *p + delta),
                ElementData::Slab(s) => s.pts.iter_mut().for_each(|p| *p = *p + delta),
                ElementData::Ceiling(c) => c.pts.iter_mut().for_each(|p| *p = *p + delta),
                ElementData::Item(i) => i.pos = i.pos + delta,
                ElementData::Opening(_) => {}
            }
        }
    }

    /// Shift an opening along its host wall, clamped to the wall body.
    /// Returns the new `s` when the host wall exists.
    pub fn shift_opening(&mut self, opening_id: Id, new_s: f64) -> Option<f64> {
        let o = self.element(opening_id)?;
        let Opening { wall: wall_id, width, .. } = match &o.data {
            ElementData::Opening(o) => o.clone(),
            _ => return None,
        };
        let len = self.wall(wall_id)?.total_length();
        let margin = width / 2.0 + 0.05;
        let s = new_s.clamp(margin.min(len / 2.0), (len - margin).max(margin.min(len / 2.0)));
        if let Some(e) = self.element_mut(opening_id) {
            if let ElementData::Opening(o) = &mut e.data {
                o.s = s;
            }
        }
        Some(s)
    }

    /// World-space rectangle for an opening, on the host wall centerline:
    /// [left-jamb bottom, right-jamb bottom, right-jamb top, left-jamb top].
    pub fn opening_rect(&self, o: &Opening) -> Option<[Vec3; 4]> {
        let w = self.wall(o.wall)?;
        let z0 = self.level(w.level).map(|l| l.elevation).unwrap_or(0.0);
        let (c, seg) = w.point_at(o.s);
        let segs: Vec<(Vec2, Vec2)> = w.segments().collect();
        let dir = segs.get(seg).map(|(a, b)| (*b - *a).normalize()).unwrap_or_else(|| Vec2::new(1.0, 0.0));
        let half = (o.width / 2.0).min(w.total_length() / 2.0);
        let p0 = c - dir * half;
        let p1 = c + dir * half;
        let zb = z0 + o.sill;
        let zt = zb + o.height;
        Some([p0.to3(zb), p1.to3(zb), p1.to3(zt), p0.to3(zt)])
    }

    /// Try to host an opening on the nearest wall at world point `p` (XY).
    /// Returns (wall id, arclength `s`).
    pub fn host_wall_at(&self, p: Vec2, max_dist: f64) -> Option<(Id, f64)> {
        let mut best: Option<(Id, f64, f64)> = None;
        for e in self.walls() {
            if let ElementData::Wall(w) = &e.data {
                let (s, d) = w.project(p);
                if d <= max_dist && best.map(|(_, bd, _)| d < bd).unwrap_or(true) {
                    best = Some((e.id, d, s));
                }
            }
        }
        best.map(|(id, _, s)| (id, s))
    }

    /// Closed polygon from two opposite corners (rectangular quick-draw).
    pub fn rect_poly(a: Vec2, b: Vec2) -> Vec<Vec2> {
        vec![Vec2::new(a.x, a.y), Vec2::new(b.x, a.y), Vec2::new(b.x, b.y), Vec2::new(a.x, b.y)]
    }

    pub fn poly_valid(pts: &[Vec2]) -> bool {
        pts.len() >= 3 && poly_area(pts).abs() > 1e-9
    }

    /// Schedule rows (parity: `model-schedule.js`).
    pub fn schedule(&self) -> Vec<ScheduleRow> {
        let mut rows = Vec::new();
        let walls: Vec<&Wall> = self
            .elements
            .iter()
            .filter_map(|e| match &e.data {
                ElementData::Wall(w) => Some(w),
                _ => None,
            })
            .collect();
        let total_len: f64 = walls.iter().map(|w| w.total_length()).sum();
        let face_area: f64 = walls.iter().map(|w| w.total_length() * w.height * 2.0).sum();
        rows.push(ScheduleRow { group: "Walls".into(), name: "Walls".into(), count: walls.len(), unit: "m".into(), quantity: total_len });
        rows.push(ScheduleRow { group: "Walls".into(), name: "Wall face area".into(), count: walls.len(), unit: "m²".into(), quantity: face_area });
        for (kind, label) in [(ElementKind::Door, "Doors"), (ElementKind::Window, "Windows")] {
            let n = self.elements.iter().filter(|e| e.kind == kind).count();
            rows.push(ScheduleRow { group: "Openings".into(), name: label.into(), count: n, unit: "pcs".into(), quantity: n as f64 });
        }
        let slabs: Vec<&Slab> = self
            .elements
            .iter()
            .filter_map(|e| match &e.data {
                ElementData::Slab(s) => Some(s),
                _ => None,
            })
            .collect();
        let slab_area: f64 = slabs.iter().map(|s| s.area()).sum();
        rows.push(ScheduleRow { group: "Floors".into(), name: "Floor slabs".into(), count: slabs.len(), unit: "m²".into(), quantity: slab_area });
        let ceils: Vec<&Ceiling> = self
            .elements
            .iter()
            .filter_map(|e| match &e.data {
                ElementData::Ceiling(c) => Some(c),
                _ => None,
            })
            .collect();
        let ceil_area: f64 = ceils.iter().map(|c| c.area()).sum();
        rows.push(ScheduleRow { group: "Ceilings".into(), name: "Ceilings".into(), count: ceils.len(), unit: "m²".into(), quantity: ceil_area });
        for kind in ItemKind::ALL {
            let n = self.elements.iter().filter(|e| matches!(&e.data, ElementData::Item(i) if i.kind == kind)).count();
            if n > 0 {
                rows.push(ScheduleRow { group: "Items".into(), name: kind.name().into(), count: n, unit: "pcs".into(), quantity: n as f64 });
            }
        }
        // Rooms: detected loops carry the areas; annotations carry the names.
        let mut room_area = 0.0;
        let mut room_count = 0usize;
        for l in &self.levels {
            let loops = self.detect_rooms(l.id);
            room_count += loops.rooms.len();
            room_area += loops.rooms.iter().map(|r| r.area).sum::<f64>();
        }
        if room_count > 0 {
            rows.push(ScheduleRow { group: "Rooms".into(), name: "Rooms (net)".into(), count: room_count, unit: "m²".into(), quantity: room_area });
        }
        rows
    }

    /// Bounding box of every element (walls + plates + items), Z-up. Used by "zoom to fit".
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        let mut min = Vec3::splat(f64::INFINITY);
        let mut max = Vec3::splat(f64::NEG_INFINITY);
        let mut any = false;
        let grow = |p: Vec2, z0: f64, z1: f64, min: &mut Vec3, max: &mut Vec3| {
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(z0.min(z1));
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(z0.max(z1));
        };
        for e in &self.elements {
            match &e.data {
                ElementData::Wall(w) => {
                    let z = self.level(w.level).map(|l| l.elevation).unwrap_or(0.0);
                    for p in &w.pts {
                        grow(*p, z, z + w.height, &mut min, &mut max);
                        any = true;
                    }
                }
                ElementData::Slab(_) | ElementData::Ceiling(_) => {
                    if let Some((lvl, pts, thk, elev)) = e.data.plan_poly() {
                        let z = self.level(lvl).map(|l| l.elevation).unwrap_or(0.0);
                        let (z0, z1) = if e.kind == ElementKind::Slab { (z + elev - thk, z + elev) } else { (z + elev, z + elev + thk) };
                        for p in pts {
                            grow(*p, z0, z1, &mut min, &mut max);
                            any = true;
                        }
                    }
                }
                ElementData::Item(i) => {
                    let z = self.level(i.level).map(|l| l.elevation).unwrap_or(0.0);
                    let bb = i.footprint();
                    grow(bb.min, z, z + i.kind.preset().2, &mut min, &mut max);
                    grow(bb.max, z, z + i.kind.preset().2, &mut min, &mut max);
                    any = true;
                }
                ElementData::Opening(_) => {}
            }
        }
        any.then_some((min, max))
    }
}

impl Element {
    pub fn level_id(&self) -> Option<Id> {
        match &self.data {
            ElementData::Wall(w) => Some(w.level),
            ElementData::Slab(s) => Some(s.level),
            ElementData::Ceiling(c) => Some(c.level),
            ElementData::Item(i) => Some(i.level),
            ElementData::Opening(o) => Some(o.wall),
        }
    }
}

/// One schedule table row.
#[derive(Clone, Debug)]
pub struct ScheduleRow {
    pub group: String,
    pub name: String,
    pub count: usize,
    pub unit: String,
    pub quantity: f64,
}

/// Triangulated top face of a plan polygon (for concave slab fills in 3D).
pub fn poly_tris_world(pts: &[Vec2], z: f64) -> Vec<[Vec3; 3]> {
    triangulate(pts).into_iter().map(|t| [pts[t[0]].to3(z), pts[t[1]].to3(z), pts[t[2]].to3(z)]).collect()
}

/// Centroid of a polygon (used for item/slab labels).
pub fn poly_center(pts: &[Vec2]) -> Vec2 {
    poly_centroid(pts)
}

/// The sample house shown on first run (parity: `new-model.js`).
pub fn sample() -> Model {
    let mut m = Model::new();
    let l0 = m.levels[0].id;
    let l1 = m.add_level("Level 1", 3.0, 3.0);

    // Datum grids A–E / 1–4.
    let grid_names = ["A", "B", "C", "D"];
    for (i, x) in [0.0, 3.0, 6.0, 9.0].into_iter().enumerate() {
        m.add_grid(grid_names[i], GridAxis::V, x);
    }
    for (i, y) in [0.0, 3.5, 7.0].into_iter().enumerate() {
        m.add_grid(&(i + 1).to_string(), GridAxis::H, y);
    }

    let thk = 0.2;
    let h = 3.0;
    // Outer walls (polyline rectangles).
    let outer = vec![Vec2::new(0.0, 0.0), Vec2::new(9.0, 0.0), Vec2::new(9.0, 7.0), Vec2::new(0.0, 7.0), Vec2::new(0.0, 0.0)];
    let south = m.add_wall(l0, vec![Vec2::new(0.0, 0.0), Vec2::new(9.0, 0.0)], thk, h);
    let east = m.add_wall(l0, vec![Vec2::new(9.0, 0.0), Vec2::new(9.0, 7.0)], thk, h);
    let north = m.add_wall(l0, vec![Vec2::new(9.0, 7.0), Vec2::new(0.0, 7.0)], thk, h);
    let west = m.add_wall(l0, vec![Vec2::new(0.0, 7.0), Vec2::new(0.0, 0.0)], thk, h);
    let _ = outer;
    // Interior wall with door gap nearby.
    let int1 = m.add_wall(l0, vec![Vec2::new(6.0, 0.0), Vec2::new(6.0, 4.2)], thk, h);
    let int2 = m.add_wall(l0, vec![Vec2::new(6.0, 4.2), Vec2::new(0.0, 4.2)], thk, h);
    let _ = int1;
    let _ = int2;

    // Openings: front door + windows.
    m.add_opening(ElementKind::Door, Opening { wall: south, s: 1.4, width: 1.0, height: 2.1, sill: 0.0 });
    m.add_opening(ElementKind::Door, Opening { wall: int2, s: 1.2, width: 0.9, height: 2.1, sill: 0.0 });
    m.add_opening(ElementKind::Window, Opening { wall: south, s: 4.5, width: 1.6, height: 1.4, sill: 0.9 });
    m.add_opening(ElementKind::Window, Opening { wall: south, s: 7.6, width: 1.2, height: 1.4, sill: 0.9 });
    m.add_opening(ElementKind::Window, Opening { wall: east, s: 2.0, width: 1.4, height: 1.4, sill: 0.9 });
    m.add_opening(ElementKind::Window, Opening { wall: east, s: 5.2, width: 1.4, height: 1.4, sill: 0.9 });
    m.add_opening(ElementKind::Window, Opening { wall: west, s: 5.0, width: 1.4, height: 1.4, sill: 0.9 });

    // Floor slab + ceiling on level 0; slab on level 1.
    let footprint = Model::rect_poly(Vec2::new(-0.1, -0.1), Vec2::new(9.1, 7.1));
    m.add_slab(l0, footprint.clone(), 0.25, 0.0);
    m.add_ceiling(l0, footprint.clone(), 0.1, 2.8);
    m.add_slab(l1, footprint, 0.25, 0.0);

    // Upper floor bedroom walls.
    m.add_wall(l1, vec![Vec2::new(0.0, 0.0), Vec2::new(9.0, 0.0)], thk, h);
    m.add_wall(l1, vec![Vec2::new(9.0, 0.0), Vec2::new(9.0, 7.0)], thk, h);
    m.add_wall(l1, vec![Vec2::new(9.0, 7.0), Vec2::new(0.0, 7.0)], thk, h);
    m.add_wall(l1, vec![Vec2::new(0.0, 7.0), Vec2::new(0.0, 0.0)], thk, h);
    let w_up = m.add_wall(l1, vec![Vec2::new(4.5, 0.0), Vec2::new(4.5, 7.0)], thk, h);
    m.add_opening(ElementKind::Door, Opening { wall: w_up, s: 2.2, width: 0.9, height: 2.1, sill: 0.0 });
    m.add_opening(ElementKind::Window, Opening { wall: north, s: 2.0, width: 1.4, height: 1.4, sill: 0.9 });

    // Items: living room + kitchen + bedroom.
    m.add_item(l0, ItemKind::Sofa, Vec2::new(2.2, 5.6), 0.0);
    m.add_item(l0, ItemKind::Table, Vec2::new(2.2, 2.2), 0.0);
    m.add_item(l0, ItemKind::Chair, Vec2::new(1.4, 2.2), std::f64::consts::FRAC_PI_2);
    m.add_item(l0, ItemKind::Chair, Vec2::new(3.0, 2.2), -std::f64::consts::FRAC_PI_2);
    m.add_item(l0, ItemKind::KitchenUnit, Vec2::new(7.9, 1.2), 0.0);
    m.add_item(l0, ItemKind::Fridge, Vec2::new(7.9, 2.4), 0.0);
    m.add_item(l0, ItemKind::Stove, Vec2::new(7.9, 0.4), 0.0);
    m.add_item(l0, ItemKind::Sink, Vec2::new(8.4, 6.4), std::f64::consts::FRAC_PI_2);
    m.add_item(l0, ItemKind::Toilet, Vec2::new(1.0, 6.3), 0.0);
    m.add_item(l0, ItemKind::Bathtub, Vec2::new(2.6, 6.4), 0.0);
    m.add_item(l0, ItemKind::Plant, Vec2::new(8.4, 5.2), 0.0);
    m.add_item(l1, ItemKind::Bed, Vec2::new(6.8, 5.6), 0.0);
    m.add_item(l1, ItemKind::Wardrobe, Vec2::new(6.8, 1.0), 0.0);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_has_seed_data() {
        let m = Model::new();
        assert_eq!(m.levels.len(), 1);
        assert_eq!(m.grids.len(), 2);
    }

    #[test]
    fn wall_opening_cascade() {
        let mut m = Model::new();
        let lvl = m.levels[0].id;
        let w = m.add_wall(lvl, vec![Vec2::ZERO, Vec2::new(6.0, 0.0)], 0.2, 3.0);
        let d = m.add_opening(ElementKind::Door, Opening { wall: w, s: 3.0, width: 0.9, height: 2.1, sill: 0.0 });
        assert_eq!(m.elements.len(), 2);
        m.delete(w);
        assert!(m.element(d).is_none());
        assert!(m.element(w).is_none());
    }

    #[test]
    fn host_wall_picks_nearest() {
        let mut m = Model::new();
        let lvl = m.levels[0].id;
        m.add_wall(lvl, vec![Vec2::ZERO, Vec2::new(6.0, 0.0)], 0.2, 3.0);
        let (id, s) = m.host_wall_at(Vec2::new(3.0, 0.05), 1.0).unwrap();
        assert!((s - 3.0).abs() < 0.1);
        assert!(m.wall(id).is_some());
    }

    #[test]
    fn windows_shift_and_clamp() {
        let mut m = Model::new();
        let lvl = m.levels[0].id;
        let w = m.add_wall(lvl, vec![Vec2::ZERO, Vec2::new(6.0, 0.0)], 0.2, 3.0);
        let win = m.add_opening(ElementKind::Window, Opening { wall: w, s: 3.0, width: 1.2, height: 1.4, sill: 0.9 });
        m.shift_opening(win, 5.9).unwrap(); // clamped below 6.0
        let o = m.element(win).unwrap();
        match &o.data {
            ElementData::Opening(o) => assert!(o.s <= 6.0 - 0.65 + 1e-9),
            _ => panic!("expected opening"),
        }
        m.shift_opening(win, -5.0).unwrap();
        let o = m.element(win).unwrap();
        match &o.data {
            ElementData::Opening(o) => assert!(o.s >= 0.65 - 1e-9),
            _ => panic!("expected opening"),
        }
    }

    #[test]
    fn polyline_wall_length_and_point() {
        let w = Wall { level: 0, pts: vec![Vec2::ZERO, Vec2::new(3.0, 0.0), Vec2::new(3.0, 4.0)], thickness: 0.2, height: 3.0 };
        assert!((w.total_length() - 7.0).abs() < 1e-9);
        let (p, seg) = w.point_at(5.0);
        assert_eq!(seg, 1);
        assert!((p.y - 2.0).abs() < 1e-9 && (p.x - 3.0).abs() < 1e-9);
    }

    #[test]
    fn sample_house_is_rich() {
        let m = sample();
        assert!(m.levels.len() >= 2);
        assert!(m.walls().count() >= 8);
        assert!(m.items().count() >= 10);
        let rows = m.schedule();
        assert!(rows.iter().any(|r| r.name == "Windows" && r.count >= 5));
        let (min, max) = m.bounds().unwrap();
        assert!(max.x > min.x && max.z > min.z);
    }

    #[test]
    fn model_json_roundtrip() {
        let m = sample();
        let json = serde_json::to_string(&m).unwrap();
        let back: Model = serde_json::from_str(&json).unwrap();
        assert_eq!(back.elements.len(), m.elements.len());
        assert_eq!(back.levels.len(), m.levels.len());
    }
}
