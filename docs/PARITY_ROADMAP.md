# BimCraft Rust Edition — Crate Parity Roadmap

> **Goal:** a Rust/egui BIM authoring app in the same shape as the storytold
> Craft workspaces (MIT OR Apache-2.0), with the **VectorCraft look** (charcoal
> chrome, 48 px tool rail, right dock, coral accent) and a **CADCraft-grade
> engine**, while keeping BIM semantics: levels, grids, parametric walls,
> doors, windows, floor slabs, ceilings, items, schedules.
>
> **Status: Phase 3 landed + v0.5.0 UI parity wave 2 (full properties panel, foundations, formwork, light mode)** — on top of Phase 2 (polyline walls, shiftable
> openings, free polygon floors/ceilings, item library, measure, snapping,
> schedule, native JSON I/O, CLI, craft-libs adoption (`craft-ui`), egui 0.36,
> Rust-only repo with the JavaScript edition as the `clone/bimcraft-js`
> submodule) Phase 3 adds **room detection** (plan loops with holes, area
> labels, named annotations, schedule), **section/elevation views** (section
> line tool, projected cut view with punched openings) and **DXF R12
> exchange** (clean-room layered wireframe export + plan-sketch import).

Last updated: 2026-10-22 · Target: v1.0.0 Rust edition parity

---

## 0. Repository layout (since the 0.2.0 restructure)

The main repository carries **Rust only**. The original JavaScript edition
(behavioural reference, Apache-2.0) is pinned as a git submodule:

```
clone/bimcraft-js/   → https://github.com/getcadcraft/bimcraft (submodule)
crates/  apps/  docs/  .github/   ← Rust workspace (this roadmap)
```

See README.md and PROVENANCE.md for the acknowledgment and provenance
statements.

## 1. Source map — what each storytold crate contributes

### 1.1 CADCraft (`storytold/cadcraft`, v0.10.0) — the *domain/engine* model

| CADCraft crate | Role for BIMCraft | Parity action | Phase |
|---|---|---|---|
| `geom` | f64 drafting math, arcs, offsets, intersections | Adopt interface shape; extend `bimcraft-geom` with arcs/offsets | P3 |
| `color` | ACI + true colour, ByLayer/ByBlock | Reuse directly (git dep or vendor) | P3 |
| `dxf` | DXF tag reader/writer | Depend as-is for DXF exchange | P3 |
| `dwg` | DWG bridge (MPL-2.0 isolation) | Optional integration, isolated feature flag | P4 |
| `doc` | Entity store, symbol tables, blocks, COW undo | Model BIM elements the same way: COW store, handles | P3 |
| `constraints` | Parametric constraint solver | Host dimension/alignment constraints on walls/openings | P4 |
| `fonts` | Drafting font + shaping | Adopt for plan annotations | P4 |
| `render` | DisplayList + rasteriser | Merge `bimcraft-render` toward this API | P3 |
| `io` | DXF/DWG/SVG/PDF I/O | Reuse for import/export once doc converges | P3 |
| `engine` | Session, commands, snaps, undo, scripting | Follow its Session/command design; add BIM commands | P3 |
| `ui-egui` | egui shell over Session | Follow structure; BimCraft uses VectorCraft theme | P3 |
| `mcp` | JSON-RPC automation server | Ship `bimcraft-mcp` for headless automation | P4 |
| `cadcraft-cli` | Headless CLI | **Done in Phase 2** (`bimcraft-cli`) | **P2 ✅** |
| `cadcraft-web` | Browser shell (eframe web) | Done in Phase 1 (`bimcraft-web`, trunk) | **P1 ✅** |
| `xtask` | wasm/ci/layers tooling | Adopt `cargo xtask` once crate count grows | P4 |

### 1.2 VectorCraft (`storytold/vectorcraft`, v0.14.0) — the *UI/UX* template

| VectorCraft crate | Role for BIMCraft | Parity action | Phase |
|---|---|---|---|
| `ui-egui` | Illustrator-style chrome (theme tokens, app bar, 48 px rail, dock, status bar) | Look & layout landed in P1; icon buttons/tabs/theme palette via `craft-ui` landed in P2 | **P2 ✅ → P3** |
| `theme.rs` tokens | Design-token system (Brightness/Accent/Scheme) | Full 1:1 token table + 4-brightness switcher (Dark/Charcoal/Slate/Light) landed | **P3 ✅ (v0.4.0)** |
| `tools` | Pointer → Action tool state machines | Refactor BIM tools into the same Action model | P3 |
| `engine` | Session façade, commands + JSON, history/journal | Adopt command-with-JSON-params seam for automation | P3 |
| `doc` | Arc<Node> COW document tree | Mirror COW discipline in BIM model | P3 |
| `three-d` | Extrude/bevel/revolve effect geometry | Reuse for roof/body massing later | P4 |
| `format` | Versioned native file format | `.bimcraft.model/v2` JSON shipped in P2; binary container later | **P2 ✅ → P4** |
| `plugins` | wasmi sandboxed plug-ins | BIM plug-ins (schedules, checks) | P4 |
| `mcp`, `cli`, `web` | Automation/headless/browser shells | Same shape as CADCraft row above | P3–P4 |
| `text`, `trace`, `pdf`, `svg`, `eps`, `metafile`, `affinity`, `brush`, `pathops`, `effects`, `scriptcheck` | 2D-vector domain | **Out of scope** for BIMCraft (vector apps only) | — |

### 1.3 Shared foundation (`storytold/craft-libs`)

| Crate | Role | Action |
|---|---|---|
| `craft-ui` | Icon buttons, tabs, widget-palette theming | **Adopted in Phase 2** as a git dependency (`rev 5c874d51`) — drives the app bar, tool rail, mode switch and the widget palette in `crates/ui-egui/src/theme.rs` |
| `craft-docpath` | External reference path policy | Adopt when BIMCraft gains links/xrefs (P3) |
| `craft-window` | Shared native titlebar | Evaluate with P3 chrome work |

Dependency pattern (as used by cadcraft itself):

```toml
craft-ui = { git = "https://github.com/storytold/craft-libs", rev = "5c874d51eeb3de70e735c8ce87a59a7dedd47675" }
```

---

## 2. BIMCraft Rust workspace today (Phase 2, shipped)

| Crate / app | Purpose | Status |
|---|---|---|
| `crates/geom` | f64 Vec2/Vec3 (Z-up), serde, polygon area/centroid, **ear-clip triangulation**, rotated-box bounds, ray/ground, segment math | ✅ P2 |
| `crates/bim` | Levels, grids, **polyline walls with mitered outlines**, hosted openings (**shiftable along host wall**, clamped), **free polygon** slabs + ceilings with elevation offsets, **item library (12 presets)**, schedule rows, sample house, full serde | ✅ P2 |
| `crates/engine` | Session, snapshot undo/redo, **8 tools** (Select/Wall/Door/Window/Floor/Ceiling/Item/Measure), **drag-to-move + drag-to-shift openings**, **grid + endpoint snapping**, polygon drafting, level/grid editing commands, 2D plan + orbit 3D camera | ✅ P2 |
| `crates/render` | DisplayList; plan with wall fills/jamb ticks/door swings/window mullions/item symbols/polygon hatches; 3D with **real opening cuts** (sub-quad subtraction), window glass, door leaves, item boxes with backrests, ear-clipped concave tops, Blender grid + XYZ axes, labelled datum grids | ✅ P2 |
| `crates/io` | Native `.bimcraft.model/v2` JSON + schedule CSV + summary/validate | ✅ P2 (new) |
| `crates/ui-egui` | Craft look on **egui 0.36 + craft-ui**: app bar icon buttons, 48 px tool rail (8 tools), right dock (Properties/Levels/Library/Schedule/Model), status bar, file dialogs, persistence, full shortcut set (W/D/N/F/C/I/M, Tab, G, R, Home, Enter, Esc, Ctrl+Z/Y) | ✅ P2 |
| `apps/bimcraft` | Desktop (eframe/glow) with autosave via eframe storage + native file open/save | ✅ P2 |
| `apps/bimcraft-web` | Browser edition (trunk → wasm32) + GitHub Pages preview + localStorage autosave | ✅ P2 |
| `apps/bimcraft-cli` | Headless `sample` / `info` / `validate` / `schedule [--csv]` | ✅ P2 (new) |
| `.github/workflows` | `rust.yml` (fmt/clippy/test/wasm-check — YAML corruption fixed), `wasm.yml` (Pages preview), `release.yml` (3-OS desktop + CLI + browser zip on `bimcraft-rs-v*` tags) | ✅ P2 |

## 3. BIM feature parity with the JS edition

| Feature | JS edition (v0.6) | Rust edition |
|---|---|---|
| Parametric walls on levels | ✅ `model-walls.js` | ✅ P2 (polyline, mitered joins, thickness/height) |
| Doors / windows in walls | ✅ `model-openings.js` | ✅ P2 (hosting, dims, sill, **shift along wall**: drag + slider + clamp) |
| Floor slabs | ✅ `model-slabs.js` | ✅ P2 (free polygon, thickness + elevation) |
| Ceilings | ✅ (ceilings) | ✅ P2 (free polygon, thickness + elevation, plan + 3D + toggles) |
| Items / furnishings | ✅ `libitem.js`, `assets-bim.js` | ✅ P2 (12 presets, place/move/rotate, plan symbols + 3D boxes) |
| Levels | ✅ `model-levels.js` | ✅ P2 (add/select/rename/elevation/height editing) |
| Grids | ✅ `model-grids.js` | ✅ P2 (V/H datum lines with bubbles + add/delete) |
| 2D plan view | ✅ | ✅ P2 (fills, symbols, drafts previews, snap dot) |
| 3D view | ✅ `viewer3d.js` | ✅ P2 (orbit/pan/zoom, real cuts, glass, Blender grid + XYZ) |
| Measure | ✅ `measure.js` | ✅ P2 (two-point readout) |
| Snapping | ✅ `drafting.js` | ✅ P2 (0.5 m grid + endpoints) |
| Schedule | ✅ `model-schedule.js` | ✅ P2 (dock table + CLI table/CSV) |
| Native format + autosave | ✅ | ✅ P2 (`.bimcraft.model/v2`, eframe storage autosave, file dialogs) |
| CLI | — (differentiator) | ✅ P2 |
| Rooms / finishes | ✅ `model-rooms.js` | ✅ P3 (`rooms.rs` plan loops + holes, annotations, schedule) |
| Sections / elevations | ✅ `views-bim.js` | ✅ P3 (SectionLine tool + projected section view); sheets → P4 |
| Roofs / stairs / railings | ✅ `model-roof.js`, `res-*` | P4 |
| IFC round-trip | ✅ `ifc/` | P3–P4 |
| DXF exchange | ✅ `dxf-model.js` | ✅ P3 (clean-room ASCII DXF R12 writer + minimal reader; blocks → P4) |
| MEP, clash, BCF | ✅ `mep/`, `clash.js`, `bcf.js` | P4 |
| Point clouds / terrain / takeoff | ✅ | P4 |
| Family editor | ✅ `family-editor.js` | P4 |
| Live collaboration, XR | ✅ `live.js`, `xr/` | P4+ |

## 4. Phase gates

- **P1 (done):** workspace + core BIM objects + 2D/3D editing + CI + wasm preview + tagged releases.
- **P2 (done — this release):** polyline walls, shiftable openings, polygon floors/ceilings, items, measure, snapping, schedule, native JSON I/O, CLI, `craft-ui` adoption, egui 0.36 upgrade, Rust-only repo restructure with JS submodule.
- **P3 (engine & exchange parity):** rooms ✅, sections/elevation views ✅, DXF R12 export/import ✅, walk mode ✅ (first-person WASD + mouse look), paint tool ✅ (per-element color override, eyedropper, paint-to-kind), theme brightness switcher ✅ (VectorCraft Dark/Charcoal/Slate/Light, **Light is the default**), foundations ✅ (v0.5.0: slab/stem+slab/crawlspace/basement seeded to the wall outline, light-frame footing table, concrete volumes), wall formwork ✅ (v0.5.0: sheathing sheets + corner studs, plan + 3D), full properties panel ✅ (v0.5.0: wall type/justification/top/base/outside, opening family/sill/from-start/from-end/hand/face, floor kind, ceiling pattern, foundation sizes). Remaining: COW entity store, tool Action model, JSON command seam, Accent/Scheme switchers.
- **P4 (pro):** IFC bridge, roofs/stairs, sheets/layout, MEP views, clash, plug-in host, `bimcraft-mcp`, off-thread render pipeline, binary container format.

## 5. Release scheme

- Tags: `bimcraft-rs-vX.Y.Z` → `release.yml` builds Linux/Windows/macOS desktop **+ CLI** + browser zip and attaches them to a GitHub Release.
- Every push to `main` → `wasm.yml` publishes the browser edition to GitHub Pages (live preview).
- The JS edition keeps its own release line at `getcadcraft/bimcraft` (pinned as `clone/bimcraft-js`).
