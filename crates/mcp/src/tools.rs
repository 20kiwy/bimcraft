//! The automation tool catalog: every tool with its permission group and a
//! JSON-schema-ish `inputSchema` served by `tools/list` (MCP style).
//!
//! Groups follow the cadcraft permission model: `read` is safe, `edit`
//! mutates the model (every edit = one undo step), `save` writes native
//! files, `export` writes exchange formats (DXF).

use serde_json::{json, Value};

use crate::Group;

/// One automatable tool.
pub struct ToolSpec {
    /// Snake-case tool name used in `tools/call`.
    pub name: &'static str,
    /// Permission group the caller must have enabled via `--allow`.
    pub group: Group,
    /// One-line description served by `tools/list`.
    pub description: &'static str,
    /// JSON schema of the `arguments` object.
    pub input_schema: Value,
}

fn schema(props: Value, required: &[&str]) -> Value {
    let mut s = json!({ "type": "object", "properties": props, "additionalProperties": false });
    if !required.is_empty() {
        s["required"] = json!(required);
    }
    s
}

/// All tools, in listing order (read, edit, save, export).
pub fn catalog() -> Vec<ToolSpec> {
    let kind_enum = |values: &[&str]| json!({ "type": "string", "enum": values });
    vec![
        // ------------------------------------------------------------- read --
        ToolSpec {
            name: "model_describe",
            group: Group::Read,
            description: "Describe the whole model: units, levels, element counts, foundations, site objects and bounds.",
            input_schema: schema(json!({}), &[]),
        },
        ToolSpec {
            name: "elements_list",
            group: Group::Read,
            description: "List elements with short summaries, optionally filtered by kind and level, with offset/limit paging.",
            input_schema: schema(
                json!({
                    "kind": kind_enum(&["Wall", "Door", "Window", "Slab", "Ceiling", "Item"]),
                    "level": { "type": ["string", "number"], "description": "level name or level id" },
                    "offset": { "type": "integer", "minimum": 0, "description": "skip the first N matches (default 0)" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "description": "page size (default 100, max 1000)" }
                }),
                &[],
            ),
        },
        ToolSpec {
            name: "element_get",
            group: Group::Read,
            description: "Fetch one element's full data (walls, openings, slabs, ceilings, items).",
            input_schema: schema(json!({ "id": { "type": "integer", "description": "element id" } }), &["id"]),
        },
        ToolSpec {
            name: "site_list",
            group: Group::Read,
            description: "List every landscaping element (trees, shrubs, hedges, fences, lawns, beds, paths, driveways).",
            input_schema: schema(json!({}), &[]),
        },
        ToolSpec {
            name: "foundation_list",
            group: Group::Read,
            description: "List foundations with resolved sizes, concrete volume and derived rebar steel mass.",
            input_schema: schema(json!({}), &[]),
        },
        // ------------------------------------------------------------- edit --
        ToolSpec {
            name: "level_add",
            group: Group::Edit,
            description: "Add a storey and make it the active level.",
            input_schema: schema(
                json!({
                    "name": { "type": "string", "description": "default 'Level <n>'" },
                    "elevation": { "type": "number", "description": "floor elevation above project zero, meters" },
                    "height": { "type": "number", "description": "storey height, meters (> 0)" }
                }),
                &["elevation", "height"],
            ),
        },
        ToolSpec {
            name: "wall_add",
            group: Group::Edit,
            description: "Add a parametric wall from a polyline (auto-foundation + brick finish aware). One undo step.",
            input_schema: schema(
                json!({
                    "level": { "type": ["string", "number"], "description": "level name or id (default: active level)" },
                    "points": { "type": "array", "items": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 }, "minItems": 2, "description": "location line vertices [[x, y], …] in meters" },
                    "thickness": { "type": "number", "description": "meters (default 0.2)" },
                    "height": { "type": "number", "description": "meters (default: level height)" },
                    "brick": { "type": "string", "description": "brick finish key, e.g. 'brick-red'" }
                }),
                &["points"],
            ),
        },
        ToolSpec {
            name: "opening_add",
            group: Group::Edit,
            description: "Host a door or window in a wall at arclength `s` (clamped into the wall body). One undo step.",
            input_schema: schema(
                json!({
                    "kind": kind_enum(&["door", "window"]),
                    "wall": { "type": "integer", "description": "host wall id" },
                    "s": { "type": "number", "description": "position along the wall location line, meters" },
                    "width": { "type": "number" },
                    "height": { "type": "number" },
                    "sill": { "type": "number", "description": "height above the wall base (doors 0, windows 0.9)" },
                    "type_key": { "type": "string", "description": "preset key, e.g. 'door-single' / 'win-picture'" }
                }),
                &["kind", "wall", "s"],
            ),
        },
        ToolSpec {
            name: "slab_add",
            group: Group::Edit,
            description: "Add a floor slab over a closed polygon. One undo step.",
            input_schema: schema(
                json!({
                    "level": { "type": ["string", "number"], "description": "level name or id (default: active level)" },
                    "points": { "type": "array", "items": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 }, "minItems": 3, "description": "closed polygon [[x, y], …]" },
                    "thickness": { "type": "number", "description": "meters (default 0.25)" },
                    "elevation": { "type": "number", "description": "top surface above the level floor (default 0)" }
                }),
                &["points"],
            ),
        },
        ToolSpec {
            name: "item_place",
            group: Group::Edit,
            description: "Place a furniture item (Table, Chair, Sofa, Bed, Wardrobe, KitchenUnit, Fridge, Stove, Sink, Toilet, Bathtub, Plant). One undo step.",
            input_schema: schema(
                json!({
                    "kind": { "type": "string", "description": "item kind name, e.g. 'Table' or 'kitchen_unit'" },
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "level": { "type": ["string", "number"], "description": "level name or id (default: active level)" },
                    "rotation_deg": { "type": "number", "description": "plan rotation (default 0)" }
                }),
                &["kind", "x", "y"],
            ),
        },
        ToolSpec {
            name: "site_object_add",
            group: Group::Edit,
            description: "Place landscaping: point kinds (trees, shrubs, hedge, potted plant) at x/y; fence needs a point chain; grass/flower bed/path/driveway need a closed polygon. One undo step.",
            input_schema: schema(
                json!({
                    "kind": { "type": "string", "description": "site kind name or key, e.g. 'TreeDeciduous6' / 'deciduous_tree_6_m' / 'Fence'" },
                    "x": { "type": "number", "description": "point kinds only" },
                    "y": { "type": "number", "description": "point kinds only" },
                    "rotation_deg": { "type": "number", "description": "point kinds only (default 0)" },
                    "points": { "type": "array", "items": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 }, "description": "chain / polygon kinds only" }
                }),
                &["kind"],
            ),
        },
        ToolSpec {
            name: "corner_detail",
            group: Group::Edit,
            description: "Set a wall-corner detailing override (formwork post size + L starter bars + note) at a plan vertex. One undo step.",
            input_schema: schema(
                json!({
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "post": { "type": "number", "description": "corner post square side in meters (0 = formwork default)" },
                    "starters": { "type": "boolean", "description": "draw L starter bars (default true)" },
                    "note": { "type": "string", "description": "e.g. 'Brick quoin corner'" }
                }),
                &["x", "y"],
            ),
        },
        ToolSpec {
            name: "section_add",
            group: Group::Edit,
            description: "Add a section/elevation view line on a level's plan. One undo step.",
            input_schema: schema(
                json!({
                    "level": { "type": ["string", "number"], "description": "level name or id (default: active level)" },
                    "a": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2, "description": "line start [x, y]" },
                    "b": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2, "description": "line end [x, y]" },
                    "depth": { "type": "number", "description": "half view depth each side, meters (default 4)" }
                }),
                &["a", "b"],
            ),
        },
        ToolSpec {
            name: "undo",
            group: Group::Edit,
            description: "Undo the last edit (each edit tool is exactly one undo step).",
            input_schema: schema(json!({}), &[]),
        },
        ToolSpec {
            name: "redo",
            group: Group::Edit,
            description: "Redo the last undone edit.",
            input_schema: schema(json!({}), &[]),
        },
        // ------------------------------------------------------------- save --
        ToolSpec {
            name: "model_save",
            group: Group::Save,
            description: "Save the model as native bimcraft JSON (same format the app uses). Path must stay inside --root when set.",
            input_schema: schema(json!({ "path": { "type": "string" } }), &["path"]),
        },
        ToolSpec {
            name: "model_load",
            group: Group::Save,
            description: "Replace the session model with a native JSON file (clears undo history). Path must stay inside --root when set.",
            input_schema: schema(json!({ "path": { "type": "string" } }), &["path"]),
        },
        ToolSpec {
            name: "model_new",
            group: Group::Save,
            description: "Start an empty model (one level + datum grids, no elements).",
            input_schema: schema(json!({}), &[]),
        },
        // ----------------------------------------------------------- export --
        ToolSpec {
            name: "dxf_export",
            group: Group::Export,
            description: "Export the model as ASCII DXF R12 (meters). Path must stay inside --root when set.",
            input_schema: schema(json!({ "path": { "type": "string" } }), &["path"]),
        },
    ]
}
