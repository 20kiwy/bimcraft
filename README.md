<img src="docs/bimcraft-mark.svg" alt="" width="64" height="64">

# BimCraft — Rust Edition

One model. Every plan, section and schedule. **Now in Rust.**

BimCraft is the BIM authoring app in the CadCraft family. This edition is a
clean-room **Rust + egui** implementation of the BimCraft model — parametric
walls, doors and windows on levels and grids, floor slabs, ceilings, items,
schedules — with a built-in **2D plan + 3D perspective** workspace, a
Blender-style grid canvas with XYZ axes, a scriptable CLI, a browser (WASM)
edition and native desktop builds.

> **Acknowledging the JavaScript original** — BimCraft began life as a
> JavaScript application (`app/*.js`, ~350 modules) at
> [getcadcraft/bimcraft](https://github.com/getcadcraft/bimcraft), Apache-2.0.
> That repository remains the canonical **JavaScript edition** and the feature
> reference for the parity roadmap. It is pinned here as the git submodule
> [`clone/bimcraft-js`](clone/bimcraft-js) so the original is always one
> command away — while this main repo carries **Rust only**
> ([see provenance & attribution](PROVENANCE.md)).

[![Rust CI](https://github.com/20kiwy/bimcraft/actions/workflows/rust.yml/badge.svg)](https://github.com/20kiwy/bimcraft/actions/workflows/rust.yml)
[![WASM preview](https://github.com/20kiwy/bimcraft/actions/workflows/wasm.yml/badge.svg)](https://github.com/20kiwy/bimcraft/actions/workflows/wasm.yml)

## Why a Rust edition

| | JavaScript edition | Rust edition (this repo) |
|---|---|---|
| Language | Plain ES modules, no bundler | Rust 2021 workspace, 6 crates + 3 apps |
| UI | Hand-rolled DOM canvas | egui 0.36 + `craft-ui` (storytold craft-libs) |
| Views | Browser-first | Desktop (Windows/macOS/Linux) + browser WASM + CLI |
| 3D | DOM canvas projection | Painter-based Z-up perspective with depth sort, real opening cuts |
| Automation | none | `bimcraft-cli` (validate / schedule / sample) |
| CI | manual | GitHub Actions: fmt/clippy/test + WASM Pages preview + tagged releases |

The two editions share **no code**. The JavaScript edition is the behavioural
reference; parity is tracked module-for-module in
[docs/PARITY_ROADMAP.md](docs/PARITY_ROADMAP.md).

## Status — parity at a glance

Full parity map: [docs/PARITY_ROADMAP.md](docs/PARITY_ROADMAP.md).

| Subsystem | JS module(s) | Rust crate(s) | State |
|---|---|---|---|
| Levels & storeys | `model-levels.js` | `bimcraft-bim` | ✅ edit elevation/height, per-level drafting & display |
| Structural grids | `model-grids.js` | `bimcraft-bim` | ✅ labelled datum lines A–E / 1–4, bubbles |
| Parametric walls | `model-walls.js` | `bimcraft-bim` | ✅ multi-segment polylines, mitered joins, thickness/height |
| Doors & windows | `model-openings.js`, `opening-types.js` | `bimcraft-bim` | ✅ hosted openings, **windows shift along the host wall** (drag or slider), real 3D cuts |
| Floor slabs | `model-floors.js`, `model-slabs.js` | `bimcraft-bim` | ✅ free polygon sketch, thickness + elevation offset |
| Ceilings | `model-floors.js` (ceilings) | `bimcraft-bim` | ✅ free polygon sketch, thickness + elevation offset, dashed plan style |
| Items / furnishings | `libitem.js`, `assets-bim.js` | `bimcraft-bim` | ✅ preset library (table, chair, sofa, bed, …), place / move / rotate |
| 2D plan view | `model-planview.js`, `views-model.js` | `bimcraft-render` | ✅ metric grid, datum bubbles, wall fills, door swings, selection grips |
| 3D view | `viewer3d.js` | `bimcraft-render` | ✅ orbit/pan/zoom, Blender grid + XYZ axes, shaded extrusions |
| Measure | `measure.js` | `bimcraft-engine` | ✅ two-point distance readout |
| Snapping | `drafting.js` | `bimcraft-engine` | ✅ grid snap + endpoint snap |
| Schedule | `model-schedule.js` | `bimcraft-bim`, `bimcraft-cli` | ✅ quantities table + CSV export |
| Undo / history | `model-ops.js` | `bimcraft-engine` | ✅ snapshot undo/redo (Ctrl+Z / Ctrl+Y) |
| Native file format | `app-autosave.js`, `model-write.js` | `bimcraft-io` | ✅ versioned JSON (`.bimcraft.json`), autosave on web & desktop |
| Sample model | `new-model.js` | `bimcraft-bim` | ✅ two-storey sample house on first run |
| CLI | — (differentiator) | `bimcraft-cli` | ✅ info / validate / schedule / sample |
| Browser edition | `web/` | `apps/bimcraft-web` | ✅ trunk → WASM, GitHub Pages preview |
| Release automation | manual | `.github/workflows` | ✅ tag `bimcraft-rs-v*` → desktop bundles + browser zip |
| Sections / elevations | `views-bim.js` | — | 📋 Phase 3 |
| Rooms & finishes | `model-rooms.js` | — | 📋 Phase 3 |
| Roofs, columns, ramps | `model-roof.js`, `model-columns.js`, `model-ramps.js` | — | 📋 Phase 3 |
| IFC import/export | `ifc/` | — | 📋 Phase 3 |
| DWG/DXF exchange | `dwg/`, `dxf-model.js` | — | 📋 Phase 4 (via `cadcraft-dxf/dwg`) |
| MEP systems | `mep/` | — | 📋 Phase 4 |
| Clash detection | `clash.js`, `clash-worker.js` | — | 📋 Phase 4 |
| Sheets & layout | `layout/`, `res-sheets.js` | — | 📋 Phase 4 |
| BCF / collaboration | `bcf.js`, `live.js` | — | 📋 Phase 4 |
| Point clouds / terrain | `pointcloud.js`, `terrain.js` | — | 📋 Phase 4 |
| Family editor | `family-editor.js` | — | 📋 Phase 4 |
| XR | `xr/` | — | 📋 Phase 5 |

## Try it

**Desktop** (Windows / macOS / Linux):

```sh
cargo run -p bimcraft --release
```

**Browser** (WASM): the `wasm` workflow publishes a live preview on GitHub
Pages on every push to `main`; or build locally:

```sh
cargo install trunk
trunk serve --config apps/bimcraft-web/Trunk.toml
```

**CLI**:

```sh
cargo run -p bimcraft-cli -- sample /tmp/house.bimcraft.json
cargo run -p bimcraft-cli -- schedule /tmp/house.bimcraft.json
cargo run -p bimcraft-cli -- validate /tmp/house.bimcraft.json
```

Shortcuts: `W` wall · `D` door · `N` window · `F` floor · `C` ceiling ·
`S` select · `Tab` 2D↔3D · `G` grid snap · `R` rotate item · `M` measure ·
`Enter` finish polyline · `Esc` cancel · `Del` delete · `Ctrl+Z`/`Ctrl+Y` undo/redo.

## Repository layout

```
crates/
  geom/        bimcraft-geom       f64 math, polygons, ear-clip triangulation (Z-up)
  bim/         bimcraft-bim        the BIM model: levels, grids, walls, openings,
                                   slabs, ceilings, items, schedule, sample house
  render/      bimcraft-render     display-list renderer for plan & 3D views
  engine/      bimcraft-engine     session, tools, snapping, undo, cameras
  io/          bimcraft-io         versioned JSON + schedule CSV
  ui-egui/     bimcraft-ui-egui    VectorCraft-style chrome (craft-ui buttons/tabs)
apps/
  bimcraft/    desktop shell (eframe/glow, Windows/macOS/Linux)
  bimcraft-web/ browser shell (trunk → wasm32)
  bimcraft-cli/ headless automation
clone/
  bimcraft-js/ → git submodule: the JavaScript edition (feature reference)
docs/          parity roadmap & provenance
.github/       CI: rust / wasm-preview / release
```

## Work on it

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
rustup target add wasm32-unknown-unknown
cargo check -p bimcraft-web --target wasm32-unknown-unknown
```

`clone/bimcraft-js` is the JavaScript reference implementation; to run or
inspect it:

```sh
git submodule update --init clone/bimcraft-js
node devshell/serve.mjs   # inside clone/bimcraft-js (Node 20.19+)
```

## Releases

Every release is tagged `bimcraft-rs-v*`; the `release` workflow then attaches
desktop bundles (Linux `.tar.gz`, Windows `.zip`, macOS `.tar.gz`) and the
browser edition `.zip` to the GitHub release:

```sh
git tag -a bimcraft-rs-v0.2.0 -m "BimCraft Rust edition v0.2.0 — full Phase 2 parity"
git push origin bimcraft-rs-v0.2.0
```

## License

Apache License 2.0 ([LICENSE](LICENSE)). The JavaScript edition is Apache-2.0
by [getcadcraft](https://github.com/getcadcraft/bimcraft) and is acknowledged
— not copied — here ([NOTICE](NOTICE), [ATTRIBUTION.md](ATTRIBUTION.md),
[PROVENANCE.md](PROVENANCE.md)). Adopted storytold crates (`craft-ui`,
MIT OR Apache-2.0) keep their license and attribution.
