#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn vectors_output_matches_committed_corpus() {
    let tmp = tempdir().unwrap();
    Command::cargo_bin("md").unwrap()
        .args(["vectors", "--out", tmp.path().to_str().unwrap()])
        .assert().success();
    let committed = format!("{}/tests/vectors", env!("CARGO_MANIFEST_DIR"));
    // Compare every committed corpus file against the freshly-generated tmp tree.
    // Use --exclude to skip the manifest (source-of-truth, not a generated artifact)
    // and the .gitkeep marker.
    let status = StdCommand::new("diff")
        .args(["-r", "--exclude=manifest.rs", "--exclude=.gitkeep",
               tmp.path().to_str().unwrap(), &committed])
        .status().unwrap();
    assert!(status.success(), "vectors corpus drift detected");
}
