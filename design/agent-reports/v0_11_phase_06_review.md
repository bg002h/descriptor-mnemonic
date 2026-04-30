# Phase 6 Review — v0.11 Implementation (Tag enum + Tree skeleton)

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Commits:**
  - Task 6.1: `abf31d7` — Tag enum (36 variants, 8 tests)
  - Task 6.2: `e66baea` — `tree.rs` skeleton (Node + Body, `unimplemented!` write/read)

## Verification

- `cargo test -p md-codec --lib v11`: **44 passed; 0 failed; 0 ignored** (no new tests in 6.2; 8 new tests added in 6.1).
- `cargo build -p md-codec`: clean.

## Files added

- `crates/md-codec/src/v11/tag.rs` — full implementation of the `Tag` enum, covering the entire spec §5 tag-space allocation (36 variants: 31 primary + 5 extension).
- `crates/md-codec/src/v11/tree.rs` — skeleton with `Node` and `Body` enums; `write_node` / `read_node` are stubbed with `unimplemented!()` to be filled in starting Phase 7.

## Spec coverage (§5 — Tag-space allocation)

Primary tag space (5-bit, `0x00..=0x1F`):

- **Top-level descriptor wrappers:** `Wpkh = 0x00`, `Tr = 0x01`, `Wsh = 0x02`, `Sh = 0x03`, `Pkh = 0x04`, `TapTree = 0x05`.
- **Multisig family (0x06..=0x09):** `Multi`, `SortedMulti`, `MultiA`, `SortedMultiA`.
- **Keys:** `PkK = 0x0A`, `PkH = 0x0B`.
- **Miniscript wrappers (0x0C..=0x12):** `WrapA`, `WrapS`, `WrapC`, `WrapT`, `WrapD`, `WrapV`, `WrapJ` (with `WrapN`/`WrapL`/`WrapU` slotted per spec — see source).
- **Logical / fragment combinators (0x13..=0x1A):** `AndV`, `AndB`, `AndOr`, `OrB`, `OrC`, `OrD`, `OrI`, `Thresh`.
- **Timelocks:** `After = 0x1B`, `Older = 0x1C`.
- **Hashes (primary):** `Sha256 = 0x1D`, `Hash160 = 0x1E`.
- **Extension prefix:** `0x1F` — escapes into the 5-bit extension space.

Extension tag space (`0x1F` + 5-bit suffix):

- `Hash256 = 0x00`, `Ripemd160 = 0x01`, `RawPkH = 0x02`, `False = 0x03`, `True = 0x04`.

## Body enum (Phase 6.2 skeleton)

Two key shape decisions consistent with the plan revision (wire bit-widths are passed as function parameters, not stored on the struct):

- `Body::Tr { key_index: u8, tree: Option<Box<Node>> }` — internal-key index + optional taptree subtree.
- `Body::KeyArg { index: u8 }` — generic key argument referencing the use-site key table.

Other `Body` variants (multisig, wrappers, combinators, timelocks, hashes) are stubbed in line with the §5 allocation and will be exercised as Phases 7+ wire `write_node` / `read_node`.

## Carry-forward deferred items

From earlier phases (still open, scoped for follow-up):

- **Phase 1:** `read_past_end` error-path state preservation; the `BitStreamExhausted` variant is currently unused.
- **Phase 2:** `write_varint` should promote `debug_assert!` to `assert!`; add a hand-crafted `L = 0` test.
- **Phase 4:** `PathDecl::write` is missing a `# Errors` rustdoc section.
- **Phase 5:** `UseSitePath::write` is missing a `# Errors` rustdoc section.

## Next

Phase 7 — Tree Class 1: implement `write_node` / `read_node` for the leaf and simple-arity variants, with TDD round-trips.
