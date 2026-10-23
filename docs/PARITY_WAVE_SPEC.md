# BimCraft Rust — UI parity wave spec (v0.5.0)

Goal: match https://getcadcraft.com/use/bimcraft (the JS edition) for (1) the full
properties panel, (2) the panel/tab settings, (3) DetailCraft-style foundations
(slab / stem+slab / crawlspace / basement) and wall formwork, (4) light mode default.

This file is the **API contract** between the four tasks. Signatures here are final;
implement them exactly. All lengths are meters, heights z-up, plan XY.

Ported-from references (Apache-2.0, clean-room): JS `res-foundation.js`
(FOUNDATION_KINDS slab|stemslab|crawl|basement, footingSize, foundationParams,
insetPoly, foundationGeometry), `model-ui.js propsBlock()` (properties fields),
`ui.inc` (right panel tabs).

## 1. bim crate (crates/bim/src)

### 1.1 New module `foundation.rs`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundationKind { Slab, StemSlab, Crawl, Basement }
impl FoundationKind {
    pub const ALL: [FoundationKind; 4] = [...];
    pub fn label(self) -> &'static str { "Slab on grade" | "Stem walls + slab" | "Crawlspace" | "Basement" }
    pub fn key(self) -> &'static str { "slab" | "stemslab" | "crawl" | "basement" }
}

/// Stored sizes (meters). 0.0 = automatic (default for the kind + loads).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct FoundationSizes { pub slab_t: f64, pub edge_w: f64, pub edge_d: f64,
    pub stem_t: f64, pub foot_w: f64, pub foot_d: f64, pub bottom: f64,
    pub stem_drop: f64, pub basement_h: f64 }
impl Default -> all zeros.

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FoundationRole { Slab, Edge, Footing, Stem }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FoundationPart { pub role: FoundationRole,
    pub outer: Vec<Vec2>, pub inner: Option<Vec<Vec2>>, pub z0: f64, pub z1: f64 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Foundation {
    pub id: Id, pub level: Id,
    pub kind: FoundationKind,
    /// A point inside the building outline this foundation serves.
    pub seed: Vec2,
    #[serde(default)] pub sizes: FoundationSizes,
    #[serde(default)] pub name: String,   // empty => UI auto-names
}

pub fn footing_size(kind: FoundationKind, stories: u32, heavy: bool) -> (f64, f64) // (width, depth)
   // base inches [12,15,23] (slab|stemslab) or [18,22,27] (crawl|basement), +4" if heavy; depth 8", 10" when width >= 24"
pub fn foundation_params(kind, stored: &FoundationSizes, stories: u32, heavy: bool) -> ResolvedSizes
   // ResolvedSizes = all 9 fields f64 (defaults where stored==0) + pub auto: [bool; 9]-style knowledge not needed
   // defaults (m): slab_t 0.1016, edge_w 0.3048, edge_d 0.3048,
   //   stem_t (heavy? 0.254 : 0.2032), foot_w/foot_d from footing_size,
   //   bottom (crawl 0.9144 else 0.6096), stem_drop (crawl|basement 0.3048 else 0.0), basement_h 2.4384
pub fn inset_poly(poly: &[Vec2], d: f64) -> Vec<Vec2>
   // anticlockwise polygon offset inward by d (d<0 => outward), corners mitred
   // (port of the JS insetPoly edge-intersection math; keep vertex count == poly.len())
pub fn foundation_geometry(outline: &[Vec2], p: &ResolvedSizes) -> Vec<FoundationPart>
   // heights measured from the level floor (z=0):
   // Slab:     slab(0, -slab_t..0); ring(Edge, outer 0 / inner edge_w, -edge_d..-slab_t)
   // StemSlab: foot_top = -bottom + foot_d; ring(Stem, 0/stem_t, foot_top..stem_top);
   //           ring(Footing, (stem_t-foot_w)/2 / (stem_t+foot_w)/2, foot_top-foot_d..foot_top); slab(stem_t, -slab_t..0)
   // Crawl:    stem + footing only (stem_top = -stem_drop)
   // Basement: slab_top = stem_top - basement_h; ring(Stem, 0/stem_t, slab_top-slab_t..stem_top);
   //           footing under it; slab(stem_t, slab_top-slab_t..slab_top)
   // ring() keeps a part only if inner area > 1e-4 and z1 > z0; slab only if area > 1e-4.
pub fn foundation_concrete_volume(parts: &[FoundationPart]) -> f64  // sum |area| * (z1-z0), holes subtracted
```

### 1.2 New module `formwork.rs`

```rust
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormworkPanel { pub side: f64,  // -1 left face / +1 right face of the segment
    pub a: Vec2, pub b: Vec2,              // panel base edge on the plan (face line, NOT offset)
    pub z0: f64, pub z1: f64, pub row: usize, pub col: usize }

pub const FORMWORK_PANEL_W: f64 = 1.2;   // sheet width (m)
pub const FORMWORK_PANEL_H: f64 = 2.4;   // sheet height (m)
pub const FORMWORK_OFFSET: f64 = 0.06;   // sheathing standoff from the wall face

/// Sheathing grid on both faces of every wall segment + corner posts at each
/// interior vertex (the "corners" ask). `base_z` = level elevation + wall base
/// offset; `top_z` = wall top (effective). Panels run along each face, split
/// into FORMWORK_PANEL_W columns (last one cut) and FORMWORK_PANEL_H rows.
pub fn wall_formwork(wall: &Wall, base_z: f64, top_z: f64) -> Vec<FormworkPanel>
/// Corner posts (plan square, height base_z..top_z) at each wall vertex, both faces.
pub fn wall_corner_posts(wall: &Wall, base_z: f64, top_z: f64, size: f64) -> Vec<(Vec2, f64, f64)> // (center, half, z-span)
pub fn formwork_area(panels: &[FormworkPanel]) -> f64 // sum (b-a).length * (z1-z0)
```

### 1.3 Model extensions (all new fields `#[serde(default)]`, old files load)

- `Wall { level, pts, thickness, height }` gains:
  - `type_key: String` (default `"ext-300"`)
  - `justification: Justification` — enum `Left | Center | Right` (default Center)
  - `top_level: Option<Id>`, `top_offset: f64` (default 0) — Some => top follows that level
  - `base_offset: f64` (default 0)
  - `ext_side: Side` — enum `Left | Right` (default Right), the "outside" face
  - methods:
    - `pub fn effective_height(&self, model: &Model) -> f64` — top_level? (level(top).elevation - level(self.level).elevation) + top_offset, else self.height; fallback height if levels missing; min 0.1
    - `pub fn base_z(&self, model: &Model) -> f64` — level(self.level).elevation + base_offset
    - `pub fn top_z(&self, model: &Model) -> f64` — base_z + effective_height
    - rework private `offset_pts(half)` -> `offset_sides(&self, left_off: f64, right_off: f64) -> (Vec<Vec2>, Vec<Vec2>)` (same miter math, separate offsets); `outline()` maps Center=(t/2,t/2), Left=(0,t), Right=(t,0)
- `Opening { wall, s, width, height, sill }` gains `type_key: String` (default `""`), `hand: char` (`'a'|'b'`, default 'a'), `face: char` (default 'a').
- `Slab { level, pts, thickness, elevation }` gains `kind: String` (default `"Concrete"`).
- `Ceiling` gains `pattern: String` (default `"none"`; `none|grid|plank`).
- `Room` gains `number: String` (default `""`).

### 1.4 New type tables (in lib.rs or `types.rs`)

```rust
pub struct WallType { pub key: &'static str, pub name: &'static str, pub thickness: f64, pub exterior: bool }
pub const WALL_TYPES: &[WallType] = &[ ext-300 "Exterior 300" 0.30 true, ext-250 "Exterior 250" 0.25 true,
  brick-230 "Brick 230" 0.23 true, cmu-200 "CMU 200" 0.20 true, concrete-250 "Concrete 250" 0.25 true,
  int-150 "Interior 150" 0.15 false, int-120 "Interior 120" 0.12 false, int-100 "Partition 100" 0.10 false ];
pub fn wall_type(key: &str) -> Option<&'static WallType>;
pub fn wall_type_for_thickness(t: f64) -> Option<&'static WallType>; // exact 1e-3 match

pub struct OpeningType { pub key: &'static str, pub name: &'static str, pub kind: OpeningFamily, pub width: f64, pub height: f64, pub handed: bool }
pub enum OpeningFamily { Door, Window }
pub const OPENING_TYPES: &[OpeningType] = &[
  door-single "Single door" 0.90 2.10 handed, door-double "Double door" 1.80 2.10 handed,
  door-sliding "Sliding door" 1.50 2.10 !handed, door-garage "Garage door" 2.40 2.10 !handed,
  win-single "Single window" 1.20 1.20 !handed, win-double "Double window" 1.80 1.20 !handed,
  win-picture "Picture window" 2.40 1.40 !handed, win-hung "Double-hung" 0.90 1.40 !handed ];
pub fn opening_type(key: &str) -> Option<&'static OpeningType>;
pub fn opening_types_of(family: OpeningFamily) -> impl Iterator<Item=&'static OpeningType>;

pub const FLOOR_KINDS: &[&str] = &["Concrete", "Wood", "Tile", "Stone", "Carpet", "Vinyl"];
pub const CEILING_PATTERNS: &[&str] = &["none", "grid", "plank"];
```

### 1.5 Model storage + API

- `Model` gains `#[serde(default)] pub foundations: Vec<Foundation>`.
- `Model::add_foundation(&mut self, f: Foundation) -> Id` (allocates id if 0, pushes, returns id)
- `Model::foundation(&self, id) -> Option<&Foundation>` / `foundation_mut`
- `Model::delete_foundation(&mut self, id)`
- `Model::outline_loops_for_level(&self, level: Id) -> Vec<Vec<Vec2>>` — union outline loops of
  the level's walls (reuse rooms::plan_loops outlines; see rooms.rs `PlanLoops::outlines`).
- `Model::foundation_outline(&self, f: &Foundation) -> Option<Vec<Vec2>>` — the outline loop
  of `f.level` that contains `f.seed` (point-in-poly), else None.
- `Model::formwork_visible_default()` not needed.
- `Model::schedule()` gains foundation rows: one row per foundation (group "Foundations",
  name = kind label, count 1, quantity = concrete volume m³).
- `Model::bounds()` includes foundation parts (top z up to 0 only needs min side; include
  outline extent in XY and z from -bottom to 0 relative to level elevation).

### 1.6 Tests (bim)

- inset_poly: area shrinks monotonically, vertex count preserved, mitre corner exact for a square.
- foundation_geometry per kind: expected roles present, z ranges correct, slab-of-stemslab inset by stem_t.
- footing_size table: (Slab,1,false)=(0.3048,0.2032); (Crawl,2,true)=(0.6096,0.254).
- formwork: 6 m straight wall, 3.0 m height, base 0 => 5 columns x 2 rows x 2 faces = 20 panels; corner posts = 2 per vertex.
- wall justification: Left outline keeps left face on the centerline.
- serde: old JSON without new fields loads (defaults) — round-trip a sample through save/load strings in io tests if simpler there.

## 2. engine crate (crates/engine/src/lib.rs)

- `Tool::Foundation` variant; label "Foundation", hint "Click inside a building outline: foundation under the walls · Esc: back".
- `Selection::Foundation(u64)` variant (update every match).
- `Session` gains:
  - `pub foundation_kind: FoundationKind` (default Slab)
  - `pub foundation_sizes: FoundationSizes` (default zeros)
  - `pub show_formwork: bool` (default true)
  - `pub formwork_selected_only: bool` (default false)
- `Session::place_foundation(&mut self, seed: Vec2)` — find outline loop containing seed on the
  active level (via Model::outline_loops_for_level + point-in-poly); none => status "No building outline here —
  draw walls first"; else snapshot, add_foundation(kind, level, seed, sizes), select it, status with kind label.
- `click_plan` Foundation arm => `self.place_foundation(p)`.
- `pick_plan` (priority after sections, before elements): any foundation whose foundation_outline
  contains p (use outline poly area centroid test via the stored outline loop) => Selection::Foundation.
- `delete_selected` handles Selection::Foundation (delete_foundation).
- `apply_paint` family: foundations paintable? NO (keep scope tight).
- tests: place on the sample house outline => 1 foundation; pick_plan hits; delete works; place outside walls fails.

## 3. render crate (crates/render/src/lib.rs)

- `ViewInfo` gains `pub light: bool` and `pub show_formwork: bool` (canvas fills them).
- Palette: when `info.light`, swap the dark-tuned plan helper colors: GRID_LINE/GRID_STRONG =>
  0xc6cdd8/0xb3bac7, datum text 0x8a93a3 => 0x5a6472; wall 2D fill stays (works on light);
  section background colors unchanged.
- Plan:
  - Foundations: for each foundation with an outline: parts drawn — Slab: fill 0x9ab4c8 alpha 45 +
    dashed outline; Edge/Footing/Stem: solid light fill (0xb8c2cf alpha 70) + edges; seed marker
    "F" label + kind initial. Selected => SELECT accent edges. Also part edges at ring boundaries.
  - Formwork (if info.show_formwork): per wall — panel joint ticks on both faces (thin lines
    across the face every panel column, row lines along), corner posts as small squares at vertices;
    draw only when plan zoom > 12 px/m to avoid noise; color 0xc98a4a alpha ~140.
  - Walls: respect justification (outline() already does), draw wall base line dashed when base_offset > 0.
  - Openings: door swing arc flips by `hand`, swing/face side by `face` (extend the existing swing
    drawing to honor them).
- 3D:
  - Foundations: solid concrete (0xb8bcc4 side / 0xd0d4da top via shade()) extrusions per part:
    ring parts => outer side quads + inner side quads + top ring band (outer[i],outer[i+1],inner[i+1])
    + (outer[i],inner[i+1],inner[i]); slab => extrude poly. Selected => SELECT tint.
  - Formwork (if show_formwork): sheathing quads at ±FORMWORK_OFFSET from each face, amber
    0xd9a05a alpha 120, edge lines darker; corner posts as thin boxes (4 side quads), color 0xa87c3f.
  - Walls: z from `wall.base_z(model)` to `wall.top_z(model)` (respect top_level/top_offset,
    base_offset). Openings sill measured from the wall base (existing behavior relative to level
    floor — rebase to base_z).
- `build_3d` walls/openings read heights through the model-levels; keep opening cutting behavior.

## 4. ui-egui crate (crates/ui-egui/src)

- `BimcraftApp::new()` => `theme: Theme::Light` (light mode default, user ask).
- `canvas.rs` + `chrome.rs` stop using the hard-coded `theme::tokens()`; thread
  `theme::tokens_for(app.theme)` through (canvas::show computes once and passes &Tokens to
  draw_plan/draw_3d/draw_section/handle_input if needed; ViewInfo.light = t.is_light()).
- Dock: `Panel::right("dock").exact_size(304.0).resizable(true).width_range(240.0..=460.0)`;
  tab row uses `ui.horizontal_wrapped` so every tab stays reachable at 240 px.
- `DockTab` gains `Foundation` (order: Properties | Levels | Library | Schedule | Foundation | Model).
- Foundation tab content:
  - "Foundations" section: list (name + kind label); click => select + switch to Properties.
  - Buttons: "+ Place foundation" (activates Tool::Foundation), "Delete" (when selected).
  - Kind picker for the next placement (FoundationKind::ALL combo) writing session.foundation_kind.
  - "Formwork" section: checkbox `session.show_formwork` ("Show wall formwork (sheets + corners)"),
    checkbox `session.formwork_selected_only`.
  - Formwork summary when a wall is selected: panel count + total area for that wall.
- Properties panel (full parity with the JS propsBlock):
  - Wall: Name · Type combo (WALL_TYPES + "Custom") · Thickness (custom editable) · Length
    (edit moves end point: recompute last pt along current direction) · Justification combo
    (Left/Center/Right) · Top combo (Up to level / Fixed) — when "Up to level": level combo of
    levels above + top offset field · Height (fixed mode) · Base offset · Outside face
    (Left/Right + Flip) · Level (read-only) · area/length caption.
  - Door/Window: Name · Type combo (family's OPENING_TYPES) · Width · Height · Sill (windows) ·
    From start (edit => s) · From end (edit => s) · Hand flip button (handed types) · Face flip ·
    host wall caption.
  - Floor slab: Name · Kind combo (FLOOR_KINDS) · Thickness · Elevation (offset) · area caption.
  - Ceiling: Name · Height (elevation) · Thickness · Pattern combo (CEILING_PATTERNS) · area.
  - Item: unchanged (pos x/y + rotate) + Z not needed.
  - Room (Selection::Room if it exists? rooms are annotations — keep existing handling; add
    Number field where the room editor lives; if no room editor exists yet, add one in Properties
    for Selection::Element only — skip if rooms aren't selectable).
  - Foundation editor (Selection::Foundation): Name · Kind combo (switch resets bottom/stem_drop
    stored to 0) · size fields grouped: Slab thickness, Edge width, Edge depth, Stem thickness,
    Footing width, Footing depth, Bottom, Stem drop, Basement height — each 0 = automatic, only
    show fields relevant to the kind · computed: outline length, stories (walls on level), heavy
    (any brick/cmu/concrete wall), footing used (w×d), concrete volume, part count · Delete.
- Level editor: keep + show room count/elevation hint (no change required).
- Status bar: add formwork toggle chip? Not needed.

## 5. io / cli (crates/io, crates/cli)

- serde defaults cover loading. `validate()` gains: foundations whose seed has no containing
  outline => warning "foundation outside any building outline".
- `summary_text` counts foundations.
- CLI: no new commands required; `info` prints foundations count.

## 6. Verification gate (all tasks)

`cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo check -p bimcraft-web --target wasm32-unknown-unknown`.
cargo lives at /home/z/.cargo/bin (export PATH).
