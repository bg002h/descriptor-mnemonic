//! Phase-1 scaffold smoke test. Pinned to one canonical encode and reused as
//! the TDD invariant Phase 2's source-move must restore. Renamed in Phase 3
//! once the moved CLI test suite arrives.

use assert_cmd::Command;

#[test]
fn encode_wpkh_default_phrase() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)"]);
    cmd.assert()
        .success()
        .stdout("md1qqpqqxqxkceprx7rap4t\n");
}
