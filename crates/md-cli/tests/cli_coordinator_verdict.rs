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

// ---------------------------------------------------------------------------
// The verdict on `md compose` and `md descriptor` (Task 8).
// ---------------------------------------------------------------------------

const H: &str = "a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8";

/// F-644's md-cli half, re-owned by plan 1b: each shape it names now carries
/// Liana's refusal, keyed on the composed shape, with or without the flag.
/// Mutation: remove the `verdict::notice` call from `compose::run` -> red.
#[test]
fn compose_names_liana_refusal_for_every_f644_shape() {
    for spelling in [
        &["--path", "2of3,unsorted", "--path", "2of2"][..],
        &["--path", "2of2", "--path", "1of1,after=800000"][..],
        &[
            "--path",
            "2of2",
            "--path",
            "1of1,older=100",
            "--path",
            "1of1,older=100",
        ][..],
        &["--path", "2of3,unsorted", "--experimental"][..],
    ] {
        for flag in [&["--unspendable", "liana"][..], &[][..]] {
            let mut args = vec!["compose", "--wrapper", "tr"];
            args.extend_from_slice(spelling);
            args.extend_from_slice(flag);
            let (out, err, code) = md(&args);
            assert_eq!(code, 0, "{args:?}: {err}");
            assert!(!out.is_empty(), "{args:?}");
            assert!(err.contains("Liana 8.0-15.0: refuses ("), "{args:?}: {err}");
        }
    }
}

/// A template never claims an import (design §1 (a2)); Core's structural
/// refusal before 26.0 still prints. Mutation: drop the `KeysAbsent` return
/// in `md_codec::coordinator::at_version` -> the lines change and this reds.
#[test]
fn compose_prints_refusals_and_silences_never_imports() {
    let (_, err, code) = md(&[
        "compose",
        "--wrapper",
        "tr",
        "--preset",
        "kofn-recovery,2of3,older=26280",
    ]);
    assert_eq!(code, 0, "{err}");
    assert!(
        err.contains("Bitcoin Core 24.2-25.2: refuses (miniscript under tr)"),
        "{err}"
    );
    assert!(
        err.contains("Bitcoin Core 26.0-31.1: unproven (a template has no keys"),
        "{err}"
    );
    assert!(!err.contains(": imports"), "{err}");
}

/// Ruling 2 and design §4: compose refuses the none case, exits non-zero,
/// names `--md-only`, and emits nothing; `--md-only` proceeds. Mutation:
/// drop `&& !md_only` from the refusal -> the second half reds.
#[test]
fn compose_refuses_the_none_case_and_md_only_proceeds() {
    let keyless = format!("keyless,sha256={H}");
    let base = [
        "compose",
        "--wrapper",
        "wsh",
        "--path",
        "2of3",
        "--path",
        &keyless,
        "--experimental",
    ];
    let (out, err, code) = md(&base);
    assert_eq!(code, 1, "{err}");
    assert!(out.is_empty(), "a refusal printed a template: {out}");
    assert!(err.contains("--md-only"), "{err}");

    let mut args = base.to_vec();
    args.push("--md-only");
    let (out, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("wsh("), "{out}");
}

/// `md descriptor` READS a card, so it never refuses on a verdict -- not
/// even the none case -- and it names the spelling it printed. Mutation:
/// route `md descriptor` through compose's none-case refusal -> the keyless
/// card exits 1 and this reds.
#[test]
fn descriptor_names_the_form_and_never_refuses() {
    let card = card_of(&descriptor_named("CONTROL-accept-kofn-recovery-flat"));
    let mut args = vec!["descriptor"];
    args.extend(card.iter().map(String::as_str));
    let (_, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("multipath form"), "{err}");
    assert!(
        err.contains("Bitcoin Core 24.2-28.4: refuses (the <0;1> multipath spelling"),
        "{err}"
    );
    args.extend(["--chain", "0"]);
    let (_, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    assert!(
        err.contains("Bitcoin Core 26.0-31.1: imports the chain0 form"),
        "{err}"
    );

    let card = card_of(&descriptor_named("keyless-hash-path-wsh"));
    let mut args = vec!["descriptor", "--experimental"];
    args.extend(card.iter().map(String::as_str));
    let (out, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    assert!(out.starts_with("wsh("), "{out}");
    assert!(
        err.contains("Liana 8.0-15.0: refuses (a path with no key)"),
        "{err}"
    );
}

/// F-674 item 3, as the user sees it: `md descriptor` on one of the composer
/// tr wallets the Core e2e run imported (agent-reports/e2e-live-site-wallets.md,
/// "Date 2026-09-23", mainnet) dates the Core import 2026-09-23, the day it
/// was measured in local time. It printed 2026-09-24, the UTC date of the
/// engrave commit that recorded it.
/// Mutation: vendor with the old git-blame rule -> this reds.
#[test]
fn descriptor_dates_the_core_import_the_day_it_was_measured() {
    let card = card_of(&descriptor_named("kofn-nums"));
    let mut args = vec!["descriptor"];
    args.extend(card.iter().map(String::as_str));
    let (_, err, code) = md(&args);
    assert_eq!(code, 0, "{err}");
    let core = err
        .lines()
        .find(|l| l.contains("Bitcoin Core") && l.contains("imports the multipath form"))
        .unwrap_or_else(|| panic!("no Core multipath import line: {err}"));
    assert!(core.ends_with("(measured 2026-09-23)"), "{core}");
    assert!(!err.contains("2026-09-24"), "{err}");
}
