# bimcraft-mcp

Headless automation server for BimCraft: JSON-RPC 2.0 over stdio, one
newline-delimited message per line. Clean-room port of the cadcraft MCP
dispatcher design (see `docs/PARITY_ROADMAP.md`, phase 4) — no DOM, no UI:
one [`bimcraft_engine::Session`] in memory, permission groups, and **every
edit tool = exactly one undo step**.

## Running

```text
bimcraft-mcp --allow read,edit,save,export [--root DIR] [--log FILE]
```

* `--allow` — comma-separated permission groups to enable (default `read`).
  Unknown groups are rejected so a typo never widens permissions.
* `--root DIR` — when set, `model_save` / `model_load` / `dxf_export` paths
  must resolve inside `DIR` (canonicalized; `..` and absolute escapes are
  refused).
* `--log FILE` — append a `< request` / `> response` trace.

Protocol: `initialize` (protocol `2024-11-05`), `notifications/initialized`,
`ping`, `tools/list`, `tools/call`. Unknown methods → JSON-RPC `-32601`, bad
params → `-32602`, tool failures → `isError: true` content with the message
inside, so an agent can self-correct.

## Tools

| group | tool | notes |
|-------|------|-------|
| read | `model_describe` | units, levels (id/name/elevation/height/grid_shift), counts, foundations, site objects, bounds |
| read | `elements_list` | filter by kind + level (name or id), `offset`/`limit` paging (default 100, max 1000), short summaries |
| read | `element_get` | full serde-serialized `Element` |
| read | `site_list` | all landscaping with kind/pos/pts (+ chain length / polygon area) |
| read | `foundation_list` | resolved sizes, concrete volume, rebar steel mass |
| edit | `level_add` | name?, elevation, height → id (becomes the active level) |
| edit | `wall_add` | level?, points, thickness?, height?, brick? → id (auto-foundation) |
| edit | `opening_add` | door/window in a wall at arclength `s`, clamped into the wall; presets from `type_key` |
| edit | `slab_add` | closed polygon; validates before snapshot |
| edit | `item_place` | 12 preset kinds (`Table` … `Plant`), rotation in degrees |
| edit | `site_object_add` | point kinds at x/y; fence = point chain; lawn/bed/path/drive = closed polygon |
| edit | `corner_detail` | wall-corner post size + L starter bars + note |
| edit | `section_add` | section line a→b on a level's plan |
| edit | `undo` / `redo` | one tool call = one undo step |
| save | `model_save` | native bimcraft JSON (same format as the app) |
| save | `model_load` | replaces the session model (clears undo history) |
| save | `model_new` | empty model (one level + datum grids) |
| export | `dxf_export` | ASCII DXF R12, meters |

Level references accept a numeric id or a name (`"Level 1"`), kinds accept
display names or normalized keys (`"Kitchen unit"`, `"kitchen_unit"`).
Coordinates are meters, z-up.

## Library use

The dispatcher is a plain library too:

```rust
use bimcraft_mcp::{Dispatcher, Groups};

let mut d = Dispatcher::new(Groups::parse("read,edit")?);
let reply = d.handle_line(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#);
```

`bimcraft_mcp::run_stdio(&mut d)` runs the stdio loop; embedders can call
`handle_line` directly and wire any transport they like.
