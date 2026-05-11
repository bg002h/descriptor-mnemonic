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
    // v0.30 wire-format break: phrase re-pinned post-tag-space rework
    // (`md1qqpqqxqq0zkd22pw8dmd3` v0.18 → `md1yqpqqxqq8xtwhw4xwn4qh` v0.30)
    // due to 6-bit primary tags + 4-bit version + `is_nums` flag + kiw
    // formula change to ⌈log₂(n)⌉.
    let mut cmd = Command::cargo_bin("md").unwrap();
    cmd.args(["encode", "wpkh(@0/<0;1>/*)"]);
    cmd.assert().success().stdout("md1yqpqqxqq8xtwhw4xwn4qh\n");
}
