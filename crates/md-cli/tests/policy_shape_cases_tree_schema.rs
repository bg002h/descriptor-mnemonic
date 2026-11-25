//! md thresh / seedhammer I-1: md-codec's shared shape-vector file
//! (`crates/md-codec/tests/fixtures/policy_shape/cases.json`) writes each
//! tree in the node schema of the vendored `.descriptor.json` files, with
//! its own small serializer. This checks, for every case that has a card,
//! that `md decode --json` reads that card to the very same `tree`, so the
//! two serializers cannot drift apart.

use std::process::Command;

#[test]
fn md_cli_reads_the_same_tree() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../md-codec/tests/fixtures/policy_shape/cases.json"
    );
    let file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut checked = 0;
    for c in file["cases"].as_array().unwrap() {
        let Some(chunks) = c["md1"].as_array() else {
            continue;
        };
        let chunks: Vec<&str> = chunks.iter().map(|x| x.as_str().unwrap()).collect();
        let out = Command::new(assert_cmd::cargo::cargo_bin("md"))
            .arg("decode")
            .arg("--json")
            .args(&chunks)
            .output()
            .unwrap();
        let name = c["name"].as_str().unwrap();
        assert!(
            out.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let j: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(j["descriptor"]["tree"], c["tree"], "{name}");
        checked += 1;
    }
    assert_eq!(checked, 81, "every case with a card");
}
