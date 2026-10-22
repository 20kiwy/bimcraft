# Provenance — BimCraft Rust Edition

This file records where every line of Rust in this repository comes from.

## Rust code

* All Rust sources under `crates/`, `apps/`, and the workspace manifests were
  written for this project (clean-room) against the public behaviour of the
  JavaScript BimCraft edition and the public documentation of the referenced
  Rust projects. No decompilation, no source extraction, no copying of the
  JavaScript code base.

## Referenced projects

### getcadcraft/bimcraft (the JavaScript edition)

* URL: https://github.com/getcadcraft/bimcraft
* License: Apache-2.0
* Relationship: **behavioural reference only**. Pinned as git submodule
  `clone/bimcraft-js`. Its module map drives `docs/PARITY_ROADMAP.md`.
* Code used: none (no shared code; no transpilation).

### storytold/craft-libs (adopted dependency)

* URL: https://github.com/storytold/craft-libs
* License: MIT OR Apache-2.0
* Relationship: **direct dependency**. The `craft-ui` crate (icon buttons,
  tabs, widget palette) is consumed as a git dependency by
  `crates/ui-egui` and keeps its license and attribution (see NOTICE).

### storytold/cadcraft and storytold/vectorcraft (references)

* URLs: https://github.com/storytold/cadcraft, https://github.com/storytold/vectorcraft
* License: MIT OR Apache-2.0
* Relationship: architectural references for workspace shape, crate seams
  (geom / doc / render / engine / ui-egui / io / cli), and the app chrome.
  Code used: none (clean-room).

## Verification

`git log` on this repository shows the full history: the Rust workspace was
authored in this repository; the JavaScript tree was removed from the main
branch and replaced by the `clone/bimcraft-js` submodule.
