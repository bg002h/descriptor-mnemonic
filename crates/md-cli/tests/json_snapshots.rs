#![allow(missing_docs)]
#![cfg(feature = "json")]
use assert_cmd::Command;

use md_codec::test_vectors as manifest;

fn encode(template: &str) -> String {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", template])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
}

/// Encode a path-carrying (non-canonical) manifest vector with its explicit
/// origin, so the emitted card decodes instead of tripping the origin gate.
fn encode_with_path(template: &str, path: &str) -> String {
    let out = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", template, "--path", path])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string()
}

/// Encode a manifest vector honoring its optional explicit `path`.
fn encode_vector(v: &manifest::Vector) -> String {
    match v.path {
        Some(p) => encode_with_path(v.template, p),
        None => encode(v.template),
    }
}

#[test]
fn decode_json_snapshots() {
    for v in manifest::MANIFEST {
        if v.force_chunked {
            continue;
        }
        let phrase = encode_vector(v);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["decode", &phrase, "--json"])
            .output()
            .unwrap();
        let body = String::from_utf8(out.stdout).unwrap();
        insta::with_settings!({ snapshot_suffix => v.name }, {
            insta::assert_snapshot!("decode", body);
        });
    }
}

#[test]
fn inspect_json_snapshots() {
    for v in manifest::MANIFEST {
        if v.force_chunked {
            continue;
        }
        let phrase = encode_vector(v);
        let out = Command::cargo_bin("md")
            .unwrap()
            .args(["inspect", &phrase, "--json"])
            .output()
            .unwrap();
        let body = String::from_utf8(out.stdout).unwrap();
        insta::with_settings!({ snapshot_suffix => v.name }, {
            insta::assert_snapshot!("inspect", body);
        });
    }
}
