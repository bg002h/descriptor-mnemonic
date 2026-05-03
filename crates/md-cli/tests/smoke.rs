//! CLI canary smoke test. Pinned to one canonical encode round-trip; runs
//! the `md` binary via `assert_cmd::Command::cargo_bin("md")`. Run with
//! `cargo test -p md-cli --test smoke` (or `cargo test -p md-cli`) to ensure
//! the test target is built against md-cli's `[[bin]]`; a workspace-wide
//! `cargo test --workspace` works today because md-cli is the only crate
//! defining `[[bin]] name = "md"`, but pinning `-p md-cli` is robust against
//! future workspace additions.

use assert_cmd::Command;

#[test]
fn encode_wpkh_default_phrase() {
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)"]);
    cmd.assert()
        .success()
        .stdout("md1qqpqqxqxkceprx7rap4t\n");
}
