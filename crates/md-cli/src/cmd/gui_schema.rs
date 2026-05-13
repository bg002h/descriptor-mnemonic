//! `md gui-schema` — emit SPEC §7 JSON describing this CLI's flag surface.
//!
//! The output is consumed by the `mnemonic-gui` overlay (see the
//! `mnemonic-gui-schema-mirror` follow-up entry) to bootstrap and drift-check
//! its per-subcommand widget schemas. The format is intentionally lossy:
//! complex GUI variants (composite/tagged-or-indexed/range/timestamp) collapse
//! to `"text"` upstream and are hand-overridden GUI-side after import.
//!
//! The shape is documented in the mnemonic-gui v0.2 plan SPEC §7:
//!
//! ```json
//! {
//!   "version": 1,
//!   "cli": "md",
//!   "subcommands": [
//!     {
//!       "name": "encode",
//!       "flags": [
//!         { "name": "--context", "required": false, "kind": "dropdown",
//!           "choices": ["tap", "segwitv0"] }
//!       ],
//!       "positionals": [
//!         { "name": "template", "required": false, "repeating": false }
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! `gui-schema` itself is NOT included in the `subcommands` array (the GUI
//! never surfaces a `Run gui-schema` form); built-in `help` is filtered too.

use crate::error::CliError;
use clap::{Arg, ArgAction, Command, CommandFactory, ValueHint};
use serde_json::{Map, Value, json};

/// SPEC §7 schema version. Bump only on a breaking JSON shape change; the
/// GUI rejects unknown versions and falls back to regex extraction.
const SCHEMA_VERSION: i64 = 1;

/// `cli` field value. Matches the binary name (`md`).
const CLI_NAME: &str = "md";

/// Names of subcommands hidden from the schema. `gui-schema` is omitted by
/// design (the GUI never renders it as a form); `help` is clap's built-in.
const HIDDEN_SUBCOMMANDS: &[&str] = &["gui-schema", "help"];

/// Entry point: print SPEC §7 JSON for every visible subcommand.
pub fn run() -> Result<(), CliError> {
    let cmd = crate::Cli::command();
    let value = build_schema(&cmd);
    // C.2 R1 I-1 fold: emit compact single-line JSON to match the
    // mk-cli / ms-cli / mnemonic-toolkit sibling implementations.
    // Multi-line pretty output diverges from the cross-repo convention
    // and breaks line-count / byte-length CI scripts.
    println!("{}", serde_json::to_string(&value).unwrap());
    Ok(())
}

/// Walk the root `Command` and produce the SPEC §7 JSON `Value`. Pure
/// function for testability (no I/O).
pub(crate) fn build_schema(root: &Command) -> Value {
    let mut subcommands: Vec<Value> = Vec::new();
    for sub in root.get_subcommands() {
        let name = sub.get_name();
        if HIDDEN_SUBCOMMANDS.contains(&name) {
            continue;
        }
        subcommands.push(subcommand_to_json(sub));
    }
    json!({
        "version": SCHEMA_VERSION,
        "cli": CLI_NAME,
        "subcommands": subcommands,
    })
}

fn subcommand_to_json(sub: &Command) -> Value {
    let mut flags: Vec<Value> = Vec::new();
    let mut positionals: Vec<Value> = Vec::new();
    for arg in sub.get_arguments() {
        // Skip the auto-generated --help / --version flags clap synthesises.
        if matches!(
            arg.get_action(),
            ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
        ) {
            continue;
        }
        if arg.is_positional() {
            positionals.push(positional_to_json(arg));
        } else {
            flags.push(flag_to_json(arg));
        }
    }
    let mut obj = Map::new();
    obj.insert("name".into(), Value::String(sub.get_name().to_owned()));
    obj.insert("flags".into(), Value::Array(flags));
    obj.insert("positionals".into(), Value::Array(positionals));
    Value::Object(obj)
}

fn flag_to_json(arg: &Arg) -> Value {
    let name = flag_name(arg);
    let required = arg.is_required_set();
    let (kind, choices) = classify_kind(arg);
    let mut obj = Map::new();
    obj.insert("name".into(), Value::String(name));
    obj.insert("required".into(), Value::Bool(required));
    obj.insert("kind".into(), Value::String(kind.into()));
    obj.insert("choices".into(), choices);
    Value::Object(obj)
}

fn positional_to_json(arg: &Arg) -> Value {
    // Positionals don't carry a `--` prefix; use the value-name or id.
    let name = arg
        .get_value_names()
        .and_then(|v| v.first())
        .map(|s| s.as_str().to_owned())
        .unwrap_or_else(|| arg.get_id().as_str().to_owned());
    let required = arg.is_required_set();
    // Repeating = takes multiple values OR action is Append.
    let repeating = matches!(arg.get_action(), ArgAction::Append)
        || arg
            .get_num_args()
            .map(|r| r.max_values() > 1)
            .unwrap_or(false);
    let mut obj = Map::new();
    obj.insert("name".into(), Value::String(name));
    obj.insert("required".into(), Value::Bool(required));
    obj.insert("repeating".into(), Value::Bool(repeating));
    Value::Object(obj)
}

/// Prefer the long flag name (`--from-policy`); fall back to short or id.
fn flag_name(arg: &Arg) -> String {
    if let Some(long) = arg.get_long() {
        return format!("--{long}");
    }
    if let Some(short) = arg.get_short() {
        return format!("-{short}");
    }
    arg.get_id().as_str().to_owned()
}

/// Map an `Arg` to a SPEC §7 `kind` plus matching `choices` (Value::Array for
/// dropdowns, Value::Null otherwise). Ordering of the kind-detection rules is
/// important; see inline comments.
fn classify_kind(arg: &Arg) -> (&'static str, Value) {
    // 1. Booleans: clap flags use SetTrue / SetFalse (and Count for repeated).
    if matches!(
        arg.get_action(),
        ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count
    ) {
        return ("boolean", Value::Null);
    }

    // 2. Restricted-choice (value_enum, value_parser = [...] literal):
    //    `Arg::get_possible_values()` returns the canonical names.
    let possible = arg.get_possible_values();
    if !possible.is_empty() {
        let choices: Vec<Value> = possible
            .iter()
            .map(|pv| Value::String(pv.get_name().to_owned()))
            .collect();
        return ("dropdown", Value::Array(choices));
    }

    // 3. Path-shaped values: either a ValueHint or a PathBuf parser.
    if matches!(
        arg.get_value_hint(),
        ValueHint::FilePath | ValueHint::DirPath | ValueHint::AnyPath | ValueHint::ExecutablePath
    ) {
        return ("path", Value::Null);
    }
    // C.2 R1 I-2 fold: replace fragile Debug-string heuristics with
    // stable TypeId comparisons matching the mk-cli / mnemonic-toolkit
    // sibling pattern. `ValueParser::type_id()` is a documented public
    // API; the Debug output for ValueParser is not contract-stable
    // across clap minor releases.
    let tid = arg.get_value_parser().type_id();
    if tid == std::any::TypeId::of::<std::path::PathBuf>() {
        return ("path", Value::Null);
    }
    let is_numeric = tid == std::any::TypeId::of::<u8>()
        || tid == std::any::TypeId::of::<u16>()
        || tid == std::any::TypeId::of::<u32>()
        || tid == std::any::TypeId::of::<u64>()
        || tid == std::any::TypeId::of::<i8>()
        || tid == std::any::TypeId::of::<i16>()
        || tid == std::any::TypeId::of::<i32>()
        || tid == std::any::TypeId::of::<i64>()
        || tid == std::any::TypeId::of::<usize>()
        || tid == std::any::TypeId::of::<isize>();
    if is_numeric {
        return ("number", Value::Null);
    }

    // 5. Default: text. Includes ValueParser::string / os_string and any
    //    custom parser whose TypeId is not in the path/number set above.
    ("text", Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn schema() -> Value {
        build_schema(&crate::Cli::command())
    }

    #[test]
    fn schema_envelope_fields() {
        let v = schema();
        assert_eq!(v["version"], json!(1));
        assert_eq!(v["cli"], json!("md"));
        assert!(v["subcommands"].is_array());
        let arr = v["subcommands"].as_array().unwrap();
        assert!(!arr.is_empty(), "subcommands array must not be empty");
    }

    #[test]
    fn gui_schema_subcommand_is_hidden() {
        let v = schema();
        let arr = v["subcommands"].as_array().unwrap();
        for sub in arr {
            let name = sub["name"].as_str().unwrap();
            assert_ne!(
                name, "gui-schema",
                "gui-schema must not include itself in the schema"
            );
            assert_ne!(name, "help");
        }
    }

    #[test]
    fn expected_subcommands_present() {
        let v = schema();
        let arr = v["subcommands"].as_array().unwrap();
        let names: Vec<&str> = arr.iter().map(|s| s["name"].as_str().unwrap()).collect();
        for expected in [
            "encode", "decode", "verify", "inspect", "bytecode", "vectors", "address",
        ] {
            assert!(
                names.contains(&expected),
                "missing subcommand {expected} in {names:?}"
            );
        }
    }

    #[test]
    fn encode_context_is_dropdown_with_tap_segwitv0() {
        let v = schema();
        let arr = v["subcommands"].as_array().unwrap();
        let encode = arr
            .iter()
            .find(|s| s["name"] == json!("encode"))
            .expect("encode subcommand");
        let flags = encode["flags"].as_array().unwrap();
        let ctx = flags
            .iter()
            .find(|f| f["name"] == json!("--context"))
            .expect("--context flag on encode");
        assert_eq!(ctx["kind"], json!("dropdown"));
        assert_eq!(ctx["choices"], json!(["tap", "segwitv0"]));
        assert_eq!(ctx["required"], json!(false));
    }

    #[test]
    fn encode_force_chunked_is_boolean() {
        let v = schema();
        let arr = v["subcommands"].as_array().unwrap();
        let encode = arr.iter().find(|s| s["name"] == json!("encode")).unwrap();
        let flags = encode["flags"].as_array().unwrap();
        let fc = flags
            .iter()
            .find(|f| f["name"] == json!("--force-chunked"))
            .expect("--force-chunked flag");
        assert_eq!(fc["kind"], json!("boolean"));
        assert_eq!(fc["choices"], Value::Null);
    }

    #[test]
    fn encode_network_is_dropdown_with_four_choices() {
        let v = schema();
        let arr = v["subcommands"].as_array().unwrap();
        let encode = arr.iter().find(|s| s["name"] == json!("encode")).unwrap();
        let flags = encode["flags"].as_array().unwrap();
        let net = flags
            .iter()
            .find(|f| f["name"] == json!("--network"))
            .expect("--network flag");
        assert_eq!(net["kind"], json!("dropdown"));
        assert_eq!(
            net["choices"],
            json!(["mainnet", "testnet", "signet", "regtest"])
        );
    }

    #[test]
    fn address_count_is_number() {
        let v = schema();
        let arr = v["subcommands"].as_array().unwrap();
        let address = arr
            .iter()
            .find(|s| s["name"] == json!("address"))
            .expect("address subcommand");
        let flags = address["flags"].as_array().unwrap();
        let count = flags
            .iter()
            .find(|f| f["name"] == json!("--count"))
            .expect("--count flag");
        assert_eq!(count["kind"], json!("number"));
        assert_eq!(count["choices"], Value::Null);
    }

    #[test]
    fn encode_template_positional_present() {
        let v = schema();
        let arr = v["subcommands"].as_array().unwrap();
        let encode = arr.iter().find(|s| s["name"] == json!("encode")).unwrap();
        let positionals = encode["positionals"].as_array().unwrap();
        assert!(
            !positionals.is_empty(),
            "encode should have a template positional"
        );
        let p = &positionals[0];
        assert_eq!(p["repeating"], json!(false));
        assert_eq!(p["required"], json!(false));
    }

    #[test]
    fn decode_strings_positional_is_repeating_and_required() {
        let v = schema();
        let arr = v["subcommands"].as_array().unwrap();
        let decode = arr
            .iter()
            .find(|s| s["name"] == json!("decode"))
            .expect("decode subcommand");
        let positionals = decode["positionals"].as_array().unwrap();
        assert_eq!(positionals.len(), 1);
        assert_eq!(positionals[0]["required"], json!(true));
        assert_eq!(positionals[0]["repeating"], json!(true));
    }
}
