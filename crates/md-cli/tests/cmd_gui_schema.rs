//! Integration test for `md gui-schema` — exercises the `assert_cmd`-spawned
//! binary end-to-end and verifies SPEC §7 envelope, expected subcommand names,
//! and the canonical `encode --context` dropdown shape consumed by the
//! `mnemonic-gui` schema-mirror CI gate.

#![allow(missing_docs)]
#![cfg(feature = "json")]

use assert_cmd::Command;
use serde_json::Value;

fn run_schema() -> Value {
    let out = Command::cargo_bin("md")
        .unwrap()
        .arg("gui-schema")
        .output()
        .expect("md gui-schema spawn");
    assert!(
        out.status.success(),
        "exit={:?} stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("stdout is valid JSON")
}

#[test]
fn gui_schema_exits_zero_and_outputs_envelope() {
    let v = run_schema();
    assert_eq!(v["version"], Value::from(1i64));
    assert_eq!(v["cli"], Value::from("md"));
    assert!(v["subcommands"].is_array());
    assert!(!v["subcommands"].as_array().unwrap().is_empty());
}

#[test]
fn gui_schema_lists_all_documented_subcommands() {
    let v = run_schema();
    let arr = v["subcommands"].as_array().unwrap();
    let names: Vec<&str> = arr.iter().map(|s| s["name"].as_str().unwrap()).collect();
    for expected in [
        "encode", "decode", "verify", "inspect", "bytecode", "vectors", "compile", "address",
    ] {
        assert!(
            names.contains(&expected),
            "subcommand {expected} missing from {names:?}"
        );
    }
    // gui-schema must NOT appear in the schema it emits.
    assert!(
        !names.contains(&"gui-schema"),
        "gui-schema must omit itself from the schema"
    );
    // Built-in `help` must be filtered out.
    assert!(!names.contains(&"help"), "clap's built-in `help` must be filtered");
}

#[test]
fn encode_context_is_dropdown_tap_segwitv0() {
    let v = run_schema();
    let arr = v["subcommands"].as_array().unwrap();
    let encode = arr.iter().find(|s| s["name"] == "encode").unwrap();
    let flags = encode["flags"].as_array().unwrap();
    let ctx = flags
        .iter()
        .find(|f| f["name"] == "--context")
        .expect("--context flag on encode");
    assert_eq!(ctx["kind"], "dropdown");
    assert_eq!(ctx["choices"], serde_json::json!(["tap", "segwitv0"]));
    assert_eq!(ctx["required"], false);
}

#[test]
fn decode_strings_positional_is_required_repeating() {
    let v = run_schema();
    let arr = v["subcommands"].as_array().unwrap();
    let decode = arr.iter().find(|s| s["name"] == "decode").unwrap();
    let positionals = decode["positionals"].as_array().unwrap();
    assert_eq!(positionals.len(), 1);
    let p = &positionals[0];
    assert_eq!(p["required"], true);
    assert_eq!(p["repeating"], true);
}

#[test]
fn flag_kind_and_choices_invariants() {
    // SPEC §7 invariant: only "dropdown" carries non-null choices; every
    // other kind sets choices to null.
    let v = run_schema();
    for sub in v["subcommands"].as_array().unwrap() {
        for f in sub["flags"].as_array().unwrap() {
            let kind = f["kind"].as_str().unwrap();
            let choices = &f["choices"];
            if kind == "dropdown" {
                assert!(
                    choices.is_array() && !choices.as_array().unwrap().is_empty(),
                    "dropdown flag {} must have non-empty choices",
                    f["name"]
                );
            } else {
                assert!(
                    choices.is_null(),
                    "non-dropdown flag {} ({}) must have null choices, got {:?}",
                    f["name"],
                    kind,
                    choices
                );
            }
            assert!(
                ["text", "boolean", "number", "dropdown", "path"].contains(&kind),
                "flag {} has unknown kind {}",
                f["name"],
                kind
            );
        }
    }
}
