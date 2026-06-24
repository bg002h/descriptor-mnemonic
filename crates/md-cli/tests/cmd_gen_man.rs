//! Integration test for `md gen-man --out <DIR>` — spawns the built binary and
//! verifies it emits a non-empty, help-shadow-free set of roff man pages.
//!
//! SPEC §8 P1 (constellation man pages): non-empty `*.1` set, root page bears a
//! `.TH` header, the new `md-gen-man.1` page exists, and the NEGATIVE canary —
//! ZERO `*-help*.1` pages (the tripwire for an accidental pre-`.build()`, C-1).

#![allow(missing_docs)]

use assert_cmd::Command;
use std::collections::BTreeSet;

fn gen_into(dir: &std::path::Path) {
    Command::cargo_bin("md")
        .unwrap()
        .arg("gen-man")
        .arg("--out")
        .arg(dir)
        .assert()
        .success();
}

fn pages(dir: &std::path::Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".1"))
        .collect()
}

#[test]
fn gen_man_produces_nonempty_page_set() {
    let dir = tempfile::tempdir().unwrap();
    gen_into(dir.path());
    let p = pages(dir.path());
    assert!(!p.is_empty(), "no *.1 pages produced");
    assert!(p.contains("md.1"), "root md.1 missing: {p:?}");
    assert!(p.contains("md-gen-man.1"), "md-gen-man.1 missing: {p:?}");
    // One page per top-level subcommand (md is a flat tree).
    for expected in [
        "md-encode.1",
        "md-decode.1",
        "md-verify.1",
        "md-inspect.1",
        "md-bytecode.1",
        "md-vectors.1",
        "md-address.1",
        "md-repair.1",
    ] {
        assert!(p.contains(expected), "{expected} missing: {p:?}");
    }
}

#[test]
fn gen_man_root_page_has_th_header() {
    let dir = tempfile::tempdir().unwrap();
    gen_into(dir.path());
    let root = std::fs::read_to_string(dir.path().join("md.1")).unwrap();
    assert!(root.contains(".TH"), "root md.1 missing roff .TH header");
    assert!(!root.is_empty());
}

#[test]
fn gen_man_negative_canary_zero_help_pages() {
    let dir = tempfile::tempdir().unwrap();
    gen_into(dir.path());
    for n in pages(dir.path()) {
        assert!(
            !(n == "md-help.1" || n.contains("-help-") || n.ends_with("-help.1")),
            "spurious help shadow page (accidental pre-build?): {n}"
        );
    }
}

#[test]
fn gen_man_creates_missing_out_dir() {
    let base = tempfile::tempdir().unwrap();
    let nested = base.path().join("a").join("b").join("man1");
    gen_into(&nested);
    assert!(
        pages(&nested).contains("md.1"),
        "did not create missing out dir"
    );
}
