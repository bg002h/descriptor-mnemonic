#![allow(missing_docs)]

use assert_cmd::Command;
use std::process::Command as StdCommand;
use tempfile::tempdir;

// Unix-only: this test shells out to the system `diff` command which is not
// standard on Windows. The corpus is platform-invariant (regenerated vectors
// are byte-identical regardless of OS), so Unix CI coverage is sufficient.
#[cfg(unix)]
#[test]
fn vectors_output_matches_committed_corpus() {
    let tmp = tempdir().unwrap();
    Command::cargo_bin("md")
        .unwrap()
        .args(["vectors", "--out", tmp.path().to_str().unwrap()])
        .assert()
        .success();
    let committed = format!("{}/../md-codec/tests/vectors", env!("CARGO_MANIFEST_DIR"));
    // Compare every committed corpus file against the freshly-generated tmp tree.
    // Use --exclude to skip the manifest (source-of-truth, not a generated artifact)
    // and the .gitkeep marker.
    let status = StdCommand::new("diff")
        .args([
            "-r",
            "--exclude=manifest.rs",
            "--exclude=.gitkeep",
            tmp.path().to_str().unwrap(),
            &committed,
        ])
        .status()
        .unwrap();
    assert!(status.success(), "vectors corpus drift detected");
}
