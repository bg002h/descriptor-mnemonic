//! CLI integration test for v0.5 multi-leaf TapTree.
//! Uses assert_cmd to invoke `md encode` and `md decode` end-to-end.

use assert_cmd::Command;

#[test]
fn cli_encode_decode_multi_leaf_taptree() {
    // 3-placeholder, 2-leaf symmetric taproot policy.  No fingerprints needed;
    // the encode command accepts the BIP 388 template notation directly.
    let policy = "tr(@0/**,{pk(@1/**),pk(@2/**)})";

    let output = Command::cargo_bin("md")
        .unwrap()
        .args(["encode", policy])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "encode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let md_string = String::from_utf8(output.stdout).unwrap();
    let md_string = md_string
        .lines()
        .find(|l| l.starts_with("md1"))
        .expect("at least one md1 chunk on stdout")
        .trim()
        .to_owned();
    assert!(
        md_string.starts_with("md1"),
        "expected md1 prefix, got {md_string:?}"
    );

    let decode_out = Command::cargo_bin("md")
        .unwrap()
        .args(["decode", &md_string])
        .output()
        .unwrap();
    assert!(
        decode_out.status.success(),
        "decode failed: {}",
        String::from_utf8_lossy(&decode_out.stderr)
    );

    let stdout = String::from_utf8(decode_out.stdout).unwrap();
    assert!(
        stdout.contains("tr("),
        "decode output missing tr() form: {stdout}"
    );
    // Verify all three placeholders survived the round-trip.
    assert!(
        stdout.contains("@0") && stdout.contains("@1") && stdout.contains("@2"),
        "decode missing placeholder references: {stdout}"
    );
    // Verify the two tap-leaf branches are present.
    assert!(
        stdout.contains("pk(@1") && stdout.contains("pk(@2"),
        "decode output missing multi-leaf pk() branches: {stdout}"
    );
}
