#![allow(missing_docs)]
//! The corpus may not pin a wallet that cannot exist (F-217).
//!
//! BIP-32 is deterministic: a `(master fingerprint, derivation path)` pair
//! identifies exactly ONE extended key. A vector whose descriptor binds one such
//! pair to two different xpubs describes an impossible wallet — and it did, in
//! **9 of 9** multi-key keyed vectors, with zero consistent, until this gate and
//! the regeneration that came with it.
//!
//! WHY A CORPUS GATE AND NOT ONLY THE ENCODER CHECK. The encoder refuses to MINT
//! one now, which stops the defect at its source. This is the other half: the
//! vectors are *files*, vendored into the Go port and compared byte for byte, so
//! a hand-edit or a bad regeneration could reintroduce the shape without going
//! through the encoder at all. The gate reads what is committed.
//!
//! It is deliberately written against the RENDERED descriptor rather than the
//! wire bytes: that is the form a coordinator imports, and the form in which the
//! contradiction is legible.

use std::collections::HashMap;
use std::path::PathBuf;

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("md-codec/tests/vectors")
}

/// Every `[fingerprint/path]xpub` pair in a descriptor string.
fn key_origins(desc: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = desc.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        let Some(close) = desc[i..].find(']').map(|o| i + o) else {
            break;
        };
        let origin = &desc[i + 1..close];
        let rest = &desc[close + 1..];
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        let key = &rest[..end];
        if key.starts_with("xpub") || key.starts_with("tpub") {
            out.push((origin.to_owned(), key.to_owned()));
        }
        i = close + 1;
    }
    out
}

#[test]
fn no_vector_declares_one_origin_for_two_different_keys() {
    let mut checked = 0;
    let mut multikey = 0;
    let mut offences: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(vectors_dir()).unwrap() {
        let path = entry.unwrap().path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.ends_with(".conformance.json") {
            continue;
        }
        let rec: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let vname = rec["name"].as_str().unwrap_or(name).to_owned();
        let Some(chains) = rec["chains"].as_object() else {
            continue;
        };
        for (chain, body) in chains {
            let Some(desc) = body["descriptor"].as_str() else {
                continue;
            };
            checked += 1;
            let pairs = key_origins(desc);
            if pairs.len() < 2 {
                continue;
            }
            multikey += 1;
            let mut by_origin: HashMap<&str, Vec<&str>> = HashMap::new();
            for (o, k) in &pairs {
                by_origin.entry(o).or_default().push(k);
            }
            for (origin, keys) in by_origin {
                let distinct: std::collections::HashSet<&&str> = keys.iter().collect();
                if distinct.len() > 1 {
                    offences.push(format!(
                        "{vname} chain {chain}: origin [{origin}] is bound to {} DIFFERENT keys",
                        distinct.len()
                    ));
                }
            }
        }
    }

    assert!(
        checked > 0,
        "no descriptors examined — this gate is checking NOTHING"
    );
    // Non-vacuous in the way that matters: the defect can only appear where two
    // keys share an origin, so a corpus with no multi-key descriptor would pass
    // this gate while proving nothing about it.
    assert!(
        multikey > 0,
        "examined {checked} descriptors and not one had two keys — the shape this gate \
         exists for cannot occur in this corpus"
    );
    assert!(
        offences.is_empty(),
        "{} impossible key origin(s) in the corpus:\n  {}",
        offences.len(),
        offences.join("\n  ")
    );
}
