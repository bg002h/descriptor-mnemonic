//! F-449 stage 2 Task 4: C3's deferred evidence legs (SPEC §8.2 through the
//! CLI, and §8.3's md-vs-Liana address leg), as committed tests.
//!
//! Stage 1b proved these at the codec level and deferred the CLI legs because
//! they need the binary (`crates/md-codec/tests/liana_unspendable.rs`, the C3
//! ruling). The whole-branch review measured them passing -- in a transcript,
//! which dies with it. These are that measurement as assertions.
//!
//! THE ORACLE IS LIANA'S RECORD, not md: every expected value below is the
//! vendored evidence (`descriptor_with_checksum`, `liana_receive`,
//! `liana_change`, written by Liana v15.0), never md recomputing from the
//! descriptor under test, which would assert only that md agrees with itself.
//!
//! THE PATH IS THE WIRE: each case is decomposed, minted into a keyed md1 card
//! with `md encode`, and read back with `md descriptor` / `md address` -- so a
//! pass means the kind-1 recogniser, the version-8 wire and the renderer all
//! reproduce Liana's own strings.
//!
//! Counts are DERIVED from the corpus (R1 I-e): the loops run over whatever
//! `cases.json` marks ACCEPT and assert only that the set is non-empty.

#![allow(missing_docs)]

// Shared with `liana_input_side.rs`; this file uses only the corpus reader.
#[allow(dead_code)]
#[path = "liana_cases.rs"]
mod liana_cases;
use liana_cases::{Case, all_cases};

use std::process::Command as StdCommand;

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

fn accepted() -> Vec<Case> {
    let cases: Vec<Case> = all_cases().into_iter().filter(|c| c.accepted).collect();
    assert!(!cases.is_empty(), "the corpus has no ACCEPT case to check");
    cases
}

/// Decompose the evidence descriptor and mint it into a keyed md1 card:
/// `md decompose --emit template|keys|fingerprints` -> `md encode`.
fn mint(c: &Case) -> Vec<String> {
    let emit = |what: &str| -> String {
        let (out, err, code) = md(&["decompose", &c.descriptor_with_checksum, "--emit", what]);
        assert_eq!(code, 0, "{}: decompose --emit {what}: {err}", c.name);
        out
    };
    let template = emit("template");
    let keys = emit("keys");
    let fps = emit("fingerprints");
    let fp_flags: Vec<&str> = fps
        .lines()
        .map(|l| l.strip_prefix("--fingerprint ").expect("fingerprint flag"))
        .collect();
    let mut args: Vec<String> = vec!["encode".into(), template.trim_end().into()];
    for (i, line) in keys.lines().enumerate() {
        // `[fp/path]xpub` -- slot order is line order; cross-check the fp.
        let (origin, xpub) = line
            .strip_prefix('[')
            .and_then(|r| r.split_once(']'))
            .unwrap_or_else(|| panic!("{}: key line {line}", c.name));
        let fp = origin.split('/').next().unwrap_or_default();
        assert_eq!(fp_flags[i], format!("@{i}={fp}"), "{}: slot order", c.name);
        args.push("--key".into());
        args.push(format!("@{i}={xpub}"));
        args.push("--fingerprint".into());
        args.push(fp_flags[i].into());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (out, err, code) = md(&refs);
    assert_eq!(code, 0, "{}: encode: {err}", c.name);
    let strings: Vec<String> = out.lines().map(str::to_string).collect();
    assert!(!strings.is_empty(), "{}: encode printed no md1", c.name);
    strings
}

/// §8.2: md renders every ACCEPT case's descriptor byte-exact, INCLUDING the
/// BIP-380 checksum Liana recorded.
#[test]
fn every_accept_case_renders_lianas_descriptor_byte_exact() {
    for c in accepted() {
        let strings = mint(&c);
        let mut args = vec!["descriptor"];
        args.extend(strings.iter().map(String::as_str));
        let (out, err, code) = md(&args);
        assert_eq!(code, 0, "{}: descriptor: {err}", c.name);
        assert_eq!(
            out.trim_end(),
            c.descriptor_with_checksum,
            "{}: md's descriptor differs from Liana's",
            c.name
        );
    }
}

/// §8.3's md-vs-Liana leg: the first receive and change addresses md derives
/// from the minted card are exactly the ones Liana recorded -- as many as the
/// evidence records per chain (three today), read from the corpus.
#[test]
fn every_accept_case_derives_lianas_addresses() {
    for c in accepted() {
        let strings = mint(&c);
        for (chain, expected) in [("receive", &c.liana_receive), ("change", &c.liana_change)] {
            assert!(
                !expected.is_empty(),
                "{}: no recorded {chain} addresses",
                c.name
            );
            let count = expected.len().to_string();
            let mut args = vec!["address", "--count", &count];
            if chain == "change" {
                args.push("--change");
            }
            args.extend(strings.iter().map(String::as_str));
            let (out, err, code) = md(&args);
            assert_eq!(code, 0, "{} {chain}: {err}", c.name);
            let got: Vec<&str> = out.lines().collect();
            assert_eq!(
                &got, expected,
                "{}: {chain} addresses differ from Liana's",
                c.name
            );
        }
    }
}
