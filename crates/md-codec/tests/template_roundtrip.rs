#![allow(missing_docs)]

mod manifest {
    include!("vectors/manifest.rs");
}

use assert_cmd::Command;

fn encode(template: &str) -> String {
    let out = Command::cargo_bin("md").unwrap().args(["encode", template]).output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

fn decode(phrase: &str) -> String {
    let out = Command::cargo_bin("md").unwrap().args(["decode", phrase]).output().unwrap();
    String::from_utf8(out.stdout).unwrap().lines().next().unwrap().to_string()
}

#[test]
fn round_trip_each_manifest_entry() {
    for v in manifest::MANIFEST {
        if v.force_chunked { continue; }   // multi-chunk handled separately
        let phrase = encode(v.template);
        let back = decode(&phrase);
        assert_eq!(back, v.template, "round-trip mismatch for {}: got {} want {}", v.name, back, v.template);
    }
}
