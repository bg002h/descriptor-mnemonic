#![allow(missing_docs)]
//! The keyed conformance records must survive a round trip through the WIRE.
//!
//! R3's point is to give the Go port something keyed to conform to. A record
//! that only agrees with the function that produced it is not that: it would
//! pin a bug as faithfully as a fix. So every field that can be re-derived is
//! re-derived here through an INDEPENDENT route -- the emitted md1 chunks, fed
//! back through `md address`, which shares only the codec with the exporter.
//!
//! What this would catch: an exporter that wrote the template's addresses while
//! the card encoded something else; a `--path` applied on one side and not the
//! other (the exact asymmetry R4 fixed); a chunked payload that reassembles to
//! a different descriptor than it was built from.

use assert_cmd::Command;
use std::path::Path;

fn md() -> Command {
    Command::cargo_bin("md").unwrap()
}

/// Regenerate the corpus into a temp dir and hand back its path.
fn generate(dir: &Path) {
    let out = md()
        .args(["vectors", "--out", dir.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "md vectors failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn keyed_conformance_records_match_the_phrase_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    generate(tmp.path());

    let mut checked = 0;
    for entry in std::fs::read_dir(tmp.path()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(stem) = name.strip_suffix(".conformance.json") else {
            continue;
        };

        let rec: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        // The card as emitted, separators stripped exactly as an operator's
        // re-typed card would be.
        let phrase_file = tmp.path().join(format!("{stem}.phrase.txt"));
        let phrase = std::fs::read_to_string(&phrase_file).unwrap();
        let chunks: Vec<String> = phrase
            .lines()
            .filter(|l| l.starts_with("md1"))
            .map(|l| l.replace(' ', ""))
            .collect();
        assert!(
            !chunks.is_empty(),
            "{stem}: no md1 chunks in {phrase_file:?}"
        );

        for (chain_str, chain_rec) in rec["chains"].as_object().unwrap() {
            let chain: u32 = chain_str.parse().unwrap();
            let want: Vec<String> = chain_rec["addresses"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap().to_string())
                .collect();
            if want.is_empty() {
                continue;
            }

            let mut cmd = md();
            cmd.arg("address");
            for c in &chunks {
                cmd.arg(c);
            }
            let out = cmd
                .args([
                    "--chain",
                    &chain.to_string(),
                    "--count",
                    &want.len().to_string(),
                ])
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{stem} chain {chain}: deriving from the CARD failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let got: Vec<String> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| l.starts_with("bc1") || l.starts_with('1') || l.starts_with('3'))
                .map(str::to_string)
                .collect();

            assert_eq!(
                got, want,
                "{stem} chain {chain}: the conformance record and the emitted card \
                 disagree about addresses.\n  record: {want:?}\n  card:   {got:?}"
            );
            checked += 1;
        }
    }

    // A conformance suite that checked nothing would pass silently, which is
    // the failure this whole vector effort exists to remove.
    assert!(
        checked >= 6,
        "only {checked} chain(s) cross-checked — are the keyed vectors present?"
    );
    eprintln!("cross-checked {checked} vector-chains against the emitted cards");
}
