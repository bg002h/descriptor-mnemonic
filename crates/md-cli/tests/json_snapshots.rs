#![allow(missing_docs)]

#![cfg(feature = "json")]
use assert_cmd::Command;

mod manifest {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../md-codec/tests/vectors/manifest.rs"));
}

fn encode(template: &str) -> String {
    let out = Command::cargo_bin("md").unwrap().args(["encode", template]).output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn decode_json_snapshots() {
    for v in manifest::MANIFEST {
        if v.force_chunked { continue; }
        let phrase = encode(v.template);
        let out = Command::cargo_bin("md").unwrap().args(["decode", &phrase, "--json"]).output().unwrap();
        let body = String::from_utf8(out.stdout).unwrap();
        insta::with_settings!({ snapshot_suffix => v.name }, {
            insta::assert_snapshot!("decode", body);
        });
    }
}

#[test]
fn inspect_json_snapshots() {
    for v in manifest::MANIFEST {
        if v.force_chunked { continue; }
        let phrase = encode(v.template);
        let out = Command::cargo_bin("md").unwrap().args(["inspect", &phrase, "--json"]).output().unwrap();
        let body = String::from_utf8(out.stdout).unwrap();
        insta::with_settings!({ snapshot_suffix => v.name }, {
            insta::assert_snapshot!("inspect", body);
        });
    }
}
