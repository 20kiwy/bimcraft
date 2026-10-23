//! Dispatcher unit tests: protocol behavior, permission gating, tool
//! semantics (undo steps, auto-foundation, clamping) and root restriction.

use super::*;
use serde_json::json;
use std::path::PathBuf;

// ------------------------------------------------------------- helpers --

fn call(d: &mut Dispatcher, method: &str, params: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }).to_string();
    let raw = d.handle_line(&line).expect("request must get a response");
    serde_json::from_str(&raw).expect("response must be valid JSON")
}

/// Call a tool; returns (isError, parsed payload from the text content).
fn tool(d: &mut Dispatcher, name: &str, args: Value) -> (bool, Value) {
    let resp = call(d, "tools/call", json!({ "name": name, "arguments": args }));
    let result = resp.get("result").expect("tools/call must return a result");
    let is_err = result["isError"].as_bool().unwrap_or(false);
    let text = result["content"][0]["text"].as_str().expect("content[0].text");
    (is_err, serde_json::from_str(text).expect("content text must be JSON"))
}

fn tool_ok(d: &mut Dispatcher, name: &str, args: Value) -> Value {
    let (is_err, v) = tool(d, name, args);
    assert!(!is_err, "tool {name} failed: {v}");
    v
}

fn tool_err(d: &mut Dispatcher, name: &str, args: Value) -> String {
    let (is_err, v) = tool(d, name, args);
    assert!(is_err, "tool {name} was expected to fail but returned {v}");
    v["error"].as_str().expect("error message").to_string()
}

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bimcraft-mcp-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create tempdir");
    dir
}

fn empty_model() -> bimcraft_bim::Model {
    bimcraft_bim::Model::new()
}

// ------------------------------------------------------------ protocol --

#[test]
fn initialize_reports_server_info_and_ping_is_empty() {
    let mut d = Dispatcher::new(Groups::all());
    let resp = call(&mut d, "initialize", json!({}));
    let result = resp.get("result").expect("initialize result");
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "bimcraft-mcp");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    let ping = call(&mut d, "ping", json!({}));
    assert_eq!(ping["result"], json!({}));
}

#[test]
fn notifications_and_blank_lines_get_no_response() {
    let mut d = Dispatcher::new(Groups::all());
    assert!(d.handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    assert!(d.handle_line(r#"{"jsonrpc":"2.0","method":"no/such/notification"}"#).is_none());
    assert!(d.handle_line("   ").is_none());
    assert!(d.handle_line("").is_none());
}

#[test]
fn unknown_method_is_32601_and_parse_error_is_32700() {
    let mut d = Dispatcher::new(Groups::all());
    let resp = call(&mut d, "no/such/method", json!({}));
    assert_eq!(resp["error"]["code"], -32601);
    let raw = d.handle_line("{not json").expect("parse errors are answered");
    let resp: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(resp["error"]["code"], -32700);
    assert_eq!(resp["id"], Value::Null);
}

#[test]
fn tools_list_respects_allow_groups() {
    let read_only = Groups::parse("read").unwrap();
    let mut d = Dispatcher::new(read_only);
    let resp = call(&mut d, "tools/list", json!({}));
    let names: Vec<&str> = resp["result"]["tools"].as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"model_describe"));
    assert!(names.contains(&"elements_list"));
    assert!(!names.contains(&"wall_add"), "edit tools hidden without the edit group");
    assert!(!names.contains(&"model_save"));
    assert!(!names.contains(&"dxf_export"));
    for t in resp["result"]["tools"].as_array().unwrap() {
        assert_eq!(t["inputSchema"]["type"], "object");
    }

    let mut full = Dispatcher::new(Groups::all());
    let resp = call(&mut full, "tools/list", json!({}));
    let names: Vec<&str> = resp["result"]["tools"].as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(names.len(), 19);
    for expected in [
        "wall_add",
        "opening_add",
        "slab_add",
        "item_place",
        "site_object_add",
        "corner_detail",
        "section_add",
        "undo",
        "redo",
        "model_save",
        "model_load",
        "model_new",
        "dxf_export",
        "site_list",
        "foundation_list",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }
}

#[test]
fn tools_call_rejects_unknown_tool_and_missing_group() {
    let mut d = Dispatcher::new(Groups::parse("read").unwrap());
    let resp = call(&mut d, "tools/call", json!({ "name": "nope", "arguments": {} }));
    assert_eq!(resp["error"]["code"], -32602);
    let resp = call(&mut d, "tools/call", json!({ "name": "wall_add", "arguments": { "points": [[0.0, 0.0], [1.0, 0.0]] } }));
    assert_eq!(resp["error"]["code"], -32602);
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("'edit'"), "message should name the missing group: {msg}");
    // Missing name entirely → invalid params.
    let resp = call(&mut d, "tools/call", json!({}));
    assert_eq!(resp["error"]["code"], -32602);
}

// --------------------------------------------------------------- reads --

#[test]
fn model_describe_lists_levels_counts_and_bounds() {
    let mut d = Dispatcher::new(Groups::all()); // sample house
    let v = tool_ok(&mut d, "model_describe", json!({}));
    assert_eq!(v["units"], "meters");
    assert_eq!(v["levels"].as_array().unwrap().len(), 3);
    let l0 = &v["levels"][0];
    assert_eq!(l0["name"], "Level 0");
    assert_eq!(l0["grid_shift"], json!([0.0, 0.0]));
    assert!(v["counts"]["walls"].as_u64().unwrap() >= 8);
    assert!(v["counts"]["doors"].as_u64().unwrap() >= 1);
    assert_eq!(v["counts"]["foundations"], v["foundations"]);
    assert!(v["site_objects"].as_u64().unwrap() >= 5);
    assert!(v["bounds"]["max"]["x"].as_f64().unwrap() > v["bounds"]["min"]["x"].as_f64().unwrap());
}

#[test]
fn elements_list_filters_kind_level_and_pages() {
    let mut d = Dispatcher::new(Groups::all());
    let v = tool_ok(&mut d, "elements_list", json!({ "kind": "Wall", "level": "Level 1" }));
    assert!(v["total"].as_u64().unwrap() >= 4, "upper storey has its envelope + partitions");
    for e in v["elements"].as_array().unwrap() {
        assert_eq!(e["kind"], "Wall");
        assert_eq!(e["level"], "Level 1");
        assert!(e["summary"].as_str().unwrap().contains("len"));
    }
    // Level ids work as references too; windows resolve their host wall's level.
    let lvl1_id = tool_ok(&mut d, "model_describe", json!({}))["levels"][2]["id"].clone();
    let v = tool_ok(&mut d, "elements_list", json!({ "kind": "Window", "level": lvl1_id }));
    assert_eq!(v["total"].as_u64().unwrap(), v["elements"].as_array().unwrap().len() as u64);
    // Paging.
    let all = tool_ok(&mut d, "elements_list", json!({ "kind": "Wall" }));
    let page = tool_ok(&mut d, "elements_list", json!({ "kind": "Wall", "offset": 1, "limit": 2 }));
    assert_eq!(page["elements"].as_array().unwrap().len(), 2);
    assert_eq!(page["total"], all["total"]);
    // Unknown kind and unknown level are tool errors (isError content).
    assert!(tool_err(&mut d, "elements_list", json!({ "kind": "Roof" })).contains("unknown kind"));
    assert!(tool_err(&mut d, "elements_list", json!({ "level": "Attic" })).contains("no level named"));
}

#[test]
fn element_get_returns_full_data_and_reports_missing() {
    let mut d = Dispatcher::new(Groups::all());
    let list = tool_ok(&mut d, "elements_list", json!({ "kind": "Wall", "limit": 1 }));
    let id = list["elements"][0]["id"].clone();
    let e = tool_ok(&mut d, "element_get", json!({ "id": id }));
    assert_eq!(e["id"], id);
    assert!(e["data"]["Wall"].is_object(), "serde-serialized ElementData: {e}");
    assert!(e["data"]["Wall"]["pts"].is_array());
    let msg = tool_err(&mut d, "element_get", json!({ "id": 424242 }));
    assert!(msg.contains("no element with id 424242"));
}

#[test]
fn site_list_covers_the_sample_and_foundation_list_resolves() {
    let mut d = Dispatcher::new(Groups::all());
    let v = tool_ok(&mut d, "site_list", json!({}));
    assert!(v["count"].as_u64().unwrap() >= 5);
    let fence = v["site_objects"].as_array().unwrap().iter().find(|s| s["key"] == "fence_run").expect("sample has fences");
    assert_eq!(fence["shape"], "chain");
    assert!(fence["length_m"].as_f64().unwrap() > 0.0);
    let lawn = v["site_objects"].as_array().unwrap().iter().find(|s| s["key"] == "grass_lawn").expect("sample has a lawn");
    assert_eq!(lawn["shape"], "polygon");
    assert!(lawn["area_m2"].as_f64().unwrap() > 10.0);

    // The sample starts without stored foundations (they are derived on the
    // first edit); grounding one storey makes foundation_list resolve sizes
    // and the derived rebar set.
    tool_ok(&mut d, "wall_add", json!({ "level": "Level 0", "points": [[14.0, 0.0], [14.0, 5.0]] }));
    let v = tool_ok(&mut d, "foundation_list", json!({}));
    assert!(v["count"].as_u64().unwrap() >= 1, "lowest wall-carrying level is grounded");
    let f = v["foundations"].as_array().unwrap().iter().find(|f| f["resolved"] == true).expect("resolved foundation");
    assert!(f["sizes_m"]["foot_w"].as_f64().unwrap() > 0.0);
    assert!(f["rebar"]["bars"].as_u64().unwrap() > 0);
    assert!(f["concrete_volume_m3"].as_f64().unwrap() > 0.0);
}

// --------------------------------------------------------------- edits --

#[test]
fn wall_add_creates_wall_and_auto_foundation() {
    let mut d = Dispatcher::with_model(empty_model(), Groups::all());
    let v = tool_ok(
        &mut d,
        "wall_add",
        json!({
            "points": [[0.0, 0.0], [6.0, 0.0], [6.0, 4.0], [0.0, 4.0], [0.0, 0.0]],
            "thickness": 0.23,
            "brick": "brick-red"
        }),
    );
    let id = v["id"].as_u64().expect("wall id");
    assert_eq!(v["foundations_added"].as_array().unwrap().len(), 1, "closed outline gets one slab-on-grade");
    let e = tool_ok(&mut d, "element_get", json!({ "id": id }));
    let wall = &e["data"]["Wall"];
    assert_eq!(wall["thickness"], 0.23);
    assert_eq!(wall["finish"], "brick-red");
    assert!(d.session().model.foundations.len() == 1);
    // Bad brick + short polylines are refused without touching the model.
    assert!(tool_err(&mut d, "wall_add", json!({ "points": [[0.0, 0.0], [1.0, 0.0]], "brick": "gold" })).contains("unknown brick"));
    assert!(tool_err(&mut d, "wall_add", json!({ "points": [[2.0, 2.0]] })).contains("at least 2"));
}

#[test]
fn undo_reverts_a_wall_and_redo_restores_it() {
    let mut d = Dispatcher::with_model(empty_model(), Groups::all());
    let before_walls = d.session().model.walls().count();
    tool_ok(&mut d, "wall_add", json!({ "points": [[0.0, 0.0], [6.0, 0.0], [6.0, 4.0], [0.0, 4.0], [0.0, 0.0]] }));
    assert_eq!(d.session().model.walls().count(), before_walls + 1);
    assert_eq!(d.session().model.foundations.len(), 1);

    let v = tool_ok(&mut d, "undo", json!({}));
    assert_eq!(v["applied"], true);
    assert_eq!(d.session().model.walls().count(), before_walls, "one tool call = one undo step");
    assert_eq!(d.session().model.foundations.len(), 0, "auto-foundation rides the same undo step");

    tool_ok(&mut d, "redo", json!({}));
    assert_eq!(d.session().model.walls().count(), before_walls + 1);
    assert_eq!(d.session().model.foundations.len(), 1);
}

#[test]
fn opening_add_uses_presets_and_clamps_into_the_wall() {
    let mut d = Dispatcher::with_model(empty_model(), Groups::all());
    let wall = tool_ok(&mut d, "wall_add", json!({ "points": [[0.0, 0.0], [6.0, 0.0]] }));
    let wall_id = wall["id"].as_u64().unwrap();
    let v = tool_ok(&mut d, "opening_add", json!({ "kind": "door", "wall": wall_id, "s": 100.0 }));
    assert_eq!(v["width"], 0.9);
    let s = v["s"].as_f64().unwrap();
    assert!(s <= 6.0 - 0.5, "clamped inside the wall body: {s}");
    let v = tool_ok(&mut d, "opening_add", json!({ "kind": "window", "wall": wall_id, "s": -10.0, "type_key": "win-picture" }));
    assert_eq!(v["width"], 2.4, "preset width used when not overridden");
    assert_eq!(v["height"], 1.4);
    assert!(v["s"].as_f64().unwrap() >= 0.0);
    assert!(tool_err(&mut d, "opening_add", json!({ "kind": "hatch", "wall": wall_id, "s": 1.0 })).contains("\"door\" or \"window\""));
    assert!(tool_err(&mut d, "opening_add", json!({ "kind": "door", "wall": 999, "s": 1.0 })).contains("no wall with id 999"));
}

#[test]
fn edits_build_levels_items_site_corners_and_sections() {
    let mut d = Dispatcher::with_model(empty_model(), Groups::all());
    let lvl = tool_ok(&mut d, "level_add", json!({ "elevation": 0.0, "height": 3.0 }));
    let lvl_id = lvl["id"].as_u64().unwrap();

    let item = tool_ok(&mut d, "item_place", json!({ "kind": "kitchen_unit", "x": 2.0, "y": 1.0, "level": lvl_id, "rotation_deg": 90.0 }));
    assert_eq!(item["kind"], "Kitchen unit");
    assert!(tool_err(&mut d, "item_place", json!({ "kind": "Jacuzzi", "x": 0.0, "y": 0.0 })).contains("unknown item kind"));

    let fence = tool_ok(&mut d, "site_object_add", json!({ "kind": "Fence", "points": [[0.0, 0.0], [5.0, 0.0]] }));
    assert!(fence["length_m"].as_f64().unwrap() > 4.9);
    let tree = tool_ok(&mut d, "site_object_add", json!({ "kind": "TreeDeciduous6", "x": 8.0, "y": -2.0, "rotation_deg": 15.0 }));
    assert_eq!(tree["key"], "deciduous_tree_6_m");
    assert_eq!(d.session().model.site.len(), 2);
    assert!(tool_err(&mut d, "site_object_add", json!({ "kind": "Volcano", "x": 0.0, "y": 0.0 })).contains("unknown site kind"));
    assert!(tool_err(&mut d, "site_object_add", json!({ "kind": "Grass", "x": 0.0, "y": 0.0 })).contains("'points' is required"));
    assert!(tool_err(&mut d, "site_object_add", json!({ "kind": "Grass", "points": [[0.0, 0.0], [1.0, 0.0], [0.5, 0.0]] }))
        .contains("valid closed polygon"));

    let corner = tool_ok(&mut d, "corner_detail", json!({ "x": 0.0, "y": 0.0, "post": 0.12, "note": "Quoin" }));
    assert_eq!(d.session().model.corner_details.len(), 1);
    assert_eq!(corner["starters"], true);

    let section = tool_ok(&mut d, "section_add", json!({ "a": [0.0, 2.0], "b": [6.0, 2.0] }));
    assert_eq!(section["name"], "Section 1");
    assert!(tool_err(&mut d, "section_add", json!({ "a": [0.0, 0.0], "b": [0.1, 0.0] })).contains("too short"));
}

#[test]
fn slab_add_validates_polygon_and_levels_add_names() {
    let mut d = Dispatcher::with_model(empty_model(), Groups::all());
    let lvl = tool_ok(&mut d, "level_add", json!({ "elevation": 3.0, "height": 2.8 }));
    assert_eq!(lvl["name"], "Level 1", "auto name after Model::new's Level 0");
    let slab = tool_ok(
        &mut d,
        "slab_add",
        json!({
            "level": lvl["id"],
            "points": [[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]],
            "thickness": 0.2
        }),
    );
    assert!(slab["area_m2"].as_f64().unwrap() > 11.0);
    assert!(tool_err(&mut d, "slab_add", json!({ "points": [[0.0, 0.0], [1.0, 0.0]] })).contains("valid polygon"));
    assert!(tool_err(&mut d, "slab_add", json!({ "points": [[0.0, 0.0], [1.0, 0.0], [0.5, 0.0]] })).contains("valid polygon"), "degenerate");
    assert!(tool_err(&mut d, "level_add", json!({ "elevation": 0.0, "height": 0.0 })).contains("'height' must be > 0"));
}

// ---------------------------------------------------------------- save --

#[test]
fn save_load_new_roundtrip_inside_root() {
    let root = tempdir("roundtrip");
    let mut d = Dispatcher::new(Groups::all()).with_root(root.clone()).expect("root exists");
    let (is_err, v) = tool(&mut d, "model_save", json!({ "path": "house.json" }));
    assert!(!is_err, "{v}");
    let saved = v["saved"].as_str().unwrap().to_string();
    assert!(saved.starts_with(root.to_str().unwrap()), "saved inside root: {saved}");
    let elements = v["elements"].as_u64().unwrap();
    assert!(std::path::Path::new(&saved).exists());

    tool_ok(&mut d, "model_new", json!({}));
    assert_eq!(d.session().model.elements.len(), 0);
    let v = tool_ok(&mut d, "model_load", json!({ "path": "house.json" }));
    assert_eq!(v["elements"], elements);
    assert_eq!(d.session().model.elements.len() as u64, elements);

    let v = tool_ok(&mut d, "dxf_export", json!({ "path": "house.dxf" }));
    let dxf = std::fs::read_to_string(root.join("house.dxf")).unwrap();
    assert!(dxf.contains("AC1009"));
    assert!(v["bytes"].as_u64().unwrap() > 100);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn root_restriction_refuses_outside_paths() {
    let root = tempdir("root");
    let mut d = Dispatcher::new(Groups::all()).with_root(root.clone()).expect("root exists");
    // Relative escape …
    let msg = tool_err(&mut d, "model_save", json!({ "path": "../escape.json" }));
    assert!(msg.contains("outside --root"), "{msg}");
    // … absolute outside (unique per process so parallel/old runs can't collide) …
    let escape = std::env::temp_dir().join(format!("bimcraft-mcp-escape-{}.dxf", std::process::id()));
    let msg = tool_err(&mut d, "dxf_export", json!({ "path": escape.to_str().unwrap() }));
    assert!(msg.contains("outside --root"), "{msg}");
    // … and a read from outside.
    let msg = tool_err(&mut d, "model_load", json!({ "path": "/etc/hostname" }));
    assert!(msg.contains("outside --root"), "{msg}");
    assert!(!root.join("../escape.json").exists());
    assert!(!escape.exists());

    // Without --root there is no restriction (default permissive host).
    let mut open = Dispatcher::new(Groups::all());
    let out = tempdir("unrooted");
    let v = tool_ok(&mut open, "model_save", json!({ "path": out.join("anywhere.json").to_str().unwrap() }));
    assert!(v["saved"].as_str().unwrap().ends_with("anywhere.json"));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn groups_parse_is_strict() {
    assert!(Groups::parse("read,edit,save,export").unwrap().keys() == vec!["read", "edit", "save", "export"]);
    assert!(Groups::parse(" read , edit ").unwrap().allows(Group::Edit));
    assert!(!Groups::parse("read").unwrap().allows(Group::Export));
    assert!(Groups::parse("admin").is_err(), "typo must not widen permissions");
    assert!(Groups::parse("").unwrap() == Groups::none());
}
