# Phase 4 — NAME_TABLE addition for `bip48-nested` — Implementer Report

**Status:** COMPLETE

## Files changed

- `crates/md-codec/src/bin/md/main.rs` — added 2 entries to `NAME_TABLE`:
  - `("bip48-nested", 0x06)` after `("bip48", 0x05)`
  - `("bip48-nestedt", 0x16)` after `("bip48t", 0x15)`
- `crates/md-codec/tests/cli.rs` — added 1 new test:
  - `md_encode_path_bip48_nested_resolves_to_indicator_0x06`

## TDD sequence

1. Test written → FAILED (`invalid child number format`)
2. NAME_TABLE entries added → test PASSED

## Gates

| Gate | Result |
|------|--------|
| `cargo test --workspace` | All passed (19 CLI tests including new one) |
| `cargo clippy -- -D warnings` | Clean (stable toolchain) |
| `cargo fmt -- --check` | Clean |
| Full workspace test count | 0 failures across all suites |

## Commit

SHA: `45f6736`
Branch: `feature/v0.4-bip388-modern-surface`
Pushed: yes

## Notes

Wire format unchanged. Indicators 0x06 and 0x16 were already in
`bytecode/path.rs::DICT` since v0.1. This phase adds only the
CLI name-to-indicator mapping as a pure ergonomics affordance.
