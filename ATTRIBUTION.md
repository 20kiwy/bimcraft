# Attribution

## BimCraft (JavaScript edition) — the original

The BimCraft product concept — parametric walls, doors and windows on levels
and grids, with views, schedules and clash checks from one model — was
created as a JavaScript application by the CadCraft project:

* Repository: <https://github.com/getcadcraft/bimcraft>
* License: Apache-2.0
* Upstream apps & docs: <https://getcadcraft.com>

This repository (the **Rust edition**) re-implements that product in Rust +
egui. The JavaScript edition is *acknowledged and referenced* (git submodule
`clone/bimcraft-js`); it is not included in this repository's build and no
code is shared between the two.

## storytold (ArtCraft suite) — adopted crates & templates

* `craft-libs` / **craft-ui** (<https://github.com/storytold/craft-libs>,
  MIT OR Apache-2.0) — adopted as a dependency: icon buttons, tab widgets and
  the widget-palette theming mechanism used by `crates/ui-egui`.
* **cadcraft** (<https://github.com/storytold/cadcraft>, MIT OR Apache-2.0) —
  clean-room reference for the crate seams of a CAD-grade Rust workspace
  (geom / doc / render / engine / ui-egui / io / cli).
* **vectorcraft** (<https://github.com/storytold/vectorcraft>, MIT OR
  Apache-2.0) — clean-room reference for the app chrome: charcoal theme
  tokens, app bar + 48 px tool rail + right dock + status bar layout.

## Foundation libraries

* **egui / eframe** (<https://github.com/emilk/egui>, MIT OR Apache-2.0) —
  immediate-mode GUI used for every pixel of the UI.
* Other crates.io dependencies keep their own licenses; the full resolved set
  and versions are pinned in `Cargo.lock`.

## Trademarks

BimCraft, CadCraft, CADCraft, VectorCraft and related names may be trademarks
of their respective owners. This repository is an independent implementation
and is not affiliated with, endorsed by, or sponsored by the upstream
projects.
