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
    // v0.18 wire-format break: phrase shifted by one bech32 char
    // (`md1qqpqqxqxkceprx7rap4t` v0.17 → `md1qqpqqxqq0zkd22pw8dmd3` v0.18) due
    // to the key_index_width formula moving to ⌈log₂(n+1)⌉ — at n=1 the width
    // grew 0→1, adding one bit to the descriptor body.
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)"]);
    cmd.assert().success().stdout("md1qqpqqxqq0zkd22pw8dmd3\n");
}
