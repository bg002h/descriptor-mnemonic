#![allow(missing_docs)]
//! P3 row 1 — the pin, asserted by a CALL rather than by a manifest line.
//!
//! `mnemonic-io-lib` is pinned by exact git rev in `Cargo.toml`. A manifest
//! entry proves the dependency resolves; it does not prove the pinned rev
//! exposes the one item `md` adopts. This file is the compile-level assertion
//! that it does, written at the module-qualified path — `write` is a `pub mod`
//! with NO root re-export, so `mnemonic_io_lib::write_private` is an E0425 and
//! a test written that way would fail to build for a reason that has nothing
//! to do with the pin.

use mnemonic_io_lib::write::write_private;

#[test]
fn the_pinned_rev_exposes_write_private_and_it_creates_owner_only() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("artifact");
    write_private(&p, b"md1yqpqqxqq8xtwhw4xwn4qh\n").unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"md1yqpqqxqq8xtwhw4xwn4qh\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "write_private must create 0600, got {mode:o}");
    }
}
