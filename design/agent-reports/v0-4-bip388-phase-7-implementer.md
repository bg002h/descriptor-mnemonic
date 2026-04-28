# Phase 7 implementer report — Cargo bump + vector regen + SHA pin update

**Date:** 2026-04-27  
**Branch:** feature/v0.4-bip388-modern-surface  
**Commit:** 313a790

## Status

COMPLETE — all tasks executed in order, all gates green.

## New SHAs (verbatim — needed for Phase 9 CHANGELOG/MIGRATION)

- **v0.1.json:** `bb2bcc78835d519c7f7595994c6113ef62c379cee99e4d62288772834d4f1c26`
- **v0.2.json:** `caddad36ecc3893e3aae87a6bb57ff1928ed9d8b8710d05a78a6501dbd1e5770`

## Tasks executed

| Task | Action | Result |
|------|--------|--------|
| 7.1 | Cargo.toml version 0.3.0 → 0.4.0 | Done |
| 7.2 | Regenerate v0.1.json (schema 1) | Done; family generator = "md-codec 0.4" |
| 7.3 | Regenerate v0.2.json (schema 2, 22 pos + 43 neg) | Done |
| 7.4 | Update V0_2_SHA256 constant in vectors_schema.rs | Done |
| 7.5 | Re-enable 2 `#[ignore]` tests (removed TODO Phase 7 comments) | Done |
| 7.6 | gen_vectors --verify: PASS for both v0.1.json and v0.2.json | PASS |
| 7.7 | BIP mediawiki: updated SHA + family-stable note; removed TODO markers | Done |
| 7.8 | build PASS; 609 tests passing + 0 ignored; clippy clean; fmt clean | PASS |
| 7.9 | Commit + push | 313a790 pushed |

## Gates

- `cargo build --workspace --all-targets`: PASS
- `cargo test -p md-codec`: 609 passing, 0 failed, 0 ignored
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --check`: clean

## Concerns

None. The SHA change on v0.1.json is expected (only the `generator` field changes from `"md-codec 0.3"` to `"md-codec 0.4"`; no fixture changes). The v0.2.json SHA changes for the same reason plus the new Phase 6 v0.4 fixtures. Both `--verify` passes confirm codec determinism.
