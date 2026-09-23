//! Coordinator-compat plan 1b, the host half (design §4, "`md` on the host"):
//! `md shape-key`, and the verdict notice on `md compose` and `md descriptor`.
//!
//! Every test names the mutation that reds it.

#![allow(missing_docs)]

use std::collections::BTreeSet;
use std::process::Command as StdCommand;

use md_codec::chunk::split;
use md_codec::descriptor_route::{descriptor_from_text, skeleton_key_of_text};

/// `(stdout, stderr, exit code)`; the same helper `cli_compose_unspendable.rs`
/// and `liana_input_side.rs` define.
fn md(args: &[&str]) -> (String, String, i32) {
    let out = StdCommand::new(assert_cmd::cargo::cargo_bin("md"))
        .args(args)
        .output()
        .expect("invoke md");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().expect("md exited normally"),
    )
}

fn evidence() -> Vec<serde_json::Value> {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../md-codec/tests/fixtures/coordinator/evidence.jsonl"
    );
    std::fs::read_to_string(p)
        .unwrap_or_else(|e| panic!("{p}: {e}"))
        .lines()
        .map(|l| serde_json::from_str(l).expect("row parses"))
        .collect()
}

fn descriptor_named(name: &str) -> String {
    evidence()
        .into_iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no evidence row named {name}"))["descriptor"]
        .as_str()
        .unwrap()
        .to_string()
}

/// The md1 chunks of a keyed card, minted by the library from a descriptor.
fn card_of(descriptor: &str) -> Vec<String> {
    split(&descriptor_from_text(descriptor).unwrap()).unwrap()
}

/// Design §5 step 1: "`md shape-key` is a thin CLI over the same library
/// function, and a test asserts the two agree" -- over every distinct
/// keyable evidence descriptor, i.e. every key the table was built from.
/// Mutation: print the key with its U+001F separators replaced by spaces ->
/// every row reds.
#[test]
fn shape_key_agrees_with_the_library_on_every_evidence_descriptor() {
    let descriptors: BTreeSet<String> = evidence()
        .into_iter()
        .filter(|r| r.get("unkeyable").is_none())
        .map(|r| r["descriptor"].as_str().unwrap().to_string())
        .collect();
    assert!(descriptors.len() >= 73, "{} descriptors", descriptors.len());
    for d in &descriptors {
        let (out, err, code) = md(&["shape-key", "--descriptor", d]);
        assert_eq!(code, 0, "{err}");
        let want = skeleton_key_of_text(d).unwrap();
        assert_eq!(out.strip_suffix('\n'), Some(want.as_str()), "{d}");
    }
}

/// A card and its descriptor are one key. Mutation: key the card through
/// `DecodeOpts::partial()` with its TLV dropped -> the partitions render
/// empty and this reds.
#[test]
fn shape_key_of_a_card_equals_the_key_of_its_descriptor() {
    for name in [
        "CONTROL-accept-kofn-recovery-flat",
        "X24-wsh-2of3-1of1-unlocked-plus-rec",
    ] {
        let d = descriptor_named(name);
        let card = card_of(&d);
        let mut args = vec!["shape-key"];
        args.extend(card.iter().map(String::as_str));
        let (from_card, err, code) = md(&args);
        assert_eq!(code, 0, "{name}: {err}");
        let (from_text, _, _) = md(&["shape-key", "--descriptor", &d]);
        assert_eq!(from_card, from_text, "{name}");
    }
}

/// A single-chain descriptor cannot be keyed: the key keeps `<0;1>`.
/// Mutation: accept single-chain text by guessing `<0;1>` -> exit 0, red.
#[test]
fn shape_key_refuses_a_single_chain_descriptor() {
    let d = descriptor_named("X24-wsh-2of3-1of1-unlocked-plus-rec").replace("<0;1>", "0");
    let d = d.split('#').next().unwrap().to_string();
    let (out, err, code) = md(&["shape-key", "--descriptor", &d]);
    assert_ne!(code, 0, "{out}");
    assert!(err.contains("multipath"), "{err}");
}
