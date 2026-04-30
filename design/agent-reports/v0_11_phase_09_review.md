# v0.11 Phase 9 Review Report

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Commits:**
  - Task 9.1: `280de57` — Tr dispatch (BIP 86 no-tree case), 1 test (`tr_bip86_no_tree`, asserts 6-bit cost)
  - Task 9.2: `72b33af` — Tr with single tap-script-tree leaf using MultiA, 1 test (`tr_with_single_leaf`)

## Files modified

- `crates/md-codec/src/v11/tree.rs` — filled in Class 3 dispatch arm in `write_node`/`read_node` for the `Tr` tag (taproot internal KeyArg + optional tap-script-tree); added 2 unit tests covering both the BIP 86 keypath-only shape and the single-leaf tap-script-tree shape.

## Test results

- `cargo test -p md-codec --lib v11::tree` → **9 passed** (7 from Phase 8 + 2 from Phase 9: `tr_bip86_no_tree`, `tr_with_single_leaf`).
- Cumulative `cargo test -p md-codec --lib v11` → **53 passed** (51 prior + 2 from Phase 9).

## Spec coverage (§6.3, Class 3: Tr)

Class 3 tag now dispatched in `tree.rs`:

- **Tr (taproot):** internal KeyArg + 1-bit "has-tree" flag + optional tap-script-tree (leaf is a sub-Node restricted to tapscript-legal fragments per §6.3.1).

### Bit-cost confirmation (BIP 86 no-tree case)

For `tr(@1)` with no tap-script-tree (per §6.3):

- 5 bits Tr tag
- 0 bits internal KeyArg payload (`@1` is N=1, zero-width index)
- 1 bit "has-tree" flag = 0

Total = **6 bits**, asserted by `tr_bip86_no_tree`.

### Tap-script-tree leaf-restriction validation — deferred

§6.3.1 restricts tap-script-tree leaves to tapscript-legal fragments (forbidding non-tapscript leaf tags). Phase 9 implements the structural encode/decode and exercises one valid leaf shape (MultiA), but does **not** enforce the forbidden-leaf-tag whitelist on read or reject malformed leaves on write. That validation is deferred to Phase 13 (validation pass).

## Deferred to later phases

The `_ => unimplemented!()` arms in `write_node`/`read_node` continue to close out as:

- **Phase 10:** Terminals — After, Older, Sha256, Hash160
- **Phase 11:** Extension space — Hash256, Ripemd160, RawPkH, False, True

## Carry-forward deferred items (Phases 1–9)

- **Phase 1:** `read_past_end_errors` state-preservation; unused `BitStreamExhausted` variant
- **Phase 2:** `write_varint` `debug_assert!` → `assert!` upgrade; L=0 hand-crafted decode test
- **Phase 4:** `PathDecl::write` `# Errors` rustdoc gap
- **Phase 5:** `UseSitePath::write` `# Errors` rustdoc gap
- **Phase 7:** arity-2/3 explicit unit-test coverage (defer to Phase 14 smoke)
- **Phase 9 (new):** `Body::Tr` tap-script-tree leaf validation — enforce §6.3.1 forbidden-leaf-tag whitelist on encode and decode (defer to Phase 13)

## Next

Phase 10 — Tree Class 4 (terminals: After, Older, Sha256, Hash160).
