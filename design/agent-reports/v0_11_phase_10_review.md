# v0.11 Phase 10 Review Report

- **Date:** 2026-04-30
- **Branch:** `feature/v0.11-impl-phase-1`
- **Commits:**
  - Task 10.1: `aa90ab0` — timelocks (After, Older), 2 tests (`after_700_000_round_trip`, `older_144_round_trip`; each asserts 37 bits = 5-bit tag + 32-bit raw u32)
  - Task 10.2: `e2ecc7c` — primary-tag hash literals (Sha256, Hash160), 2 tests (`sha256_round_trip` asserts 261 bits = 5 + 256; `hash160_round_trip` asserts 165 bits = 5 + 160)

## Files modified

- `crates/md-codec/src/v11/tree.rs` — filled in Class 4 dispatch arms in `write_node`/`read_node` for the four terminal/literal tags currently in primary-tag space (After, Older, Sha256, Hash160); added 4 unit tests covering bit-cost and round-trip behavior.

## Test results

- `cargo test -p md-codec --lib v11::tree` → **13 passed** (9 from Phase 9 + 4 from Phase 10).
- Cumulative `cargo test -p md-codec --lib v11` → **57 passed** (53 prior + 4 from Phase 10).

## Spec coverage (§6.4, Class 4: terminals & literals)

Class 4 tags now dispatched in `tree.rs`:

- **After / Older (§4.3, Bitcoin-native u32 timelock):** share a single dispatch arm; payload is the raw absolute/relative locktime u32 emitted MSB-first as 32 bits with no further compression. Total cost per node = 5-bit primary tag + 32-bit raw u32 = **37 bits**, confirmed by both timelock round-trip tests.
- **Sha256 / Hash160 (§4.4, hash literals byte-aligned):** payload is the raw digest bytes wrapped via `Body::Hash256Body` / `Body::Hash160Body`. Encoding is byte-aligned per §4.4 (no compression, no varint length — the tag fixes the digest width). Costs:
  - `sha256(<32 B>)` = 5 + 256 = **261 bits**
  - `hash160(<20 B>)` = 5 + 160 = **165 bits**

  Both confirmed by their respective round-trip tests.

### Note on shared timelock dispatch

After and Older share the same dispatch arm in `write_node`/`read_node` because the wire shape is identical (raw u32, MSB-first); the tag itself disambiguates semantics. This matches §6.4 and §4.3, and keeps the dispatch table compact.

### Note on hash-literal Body wrappers

Sha256 and Hash160 use distinct `Body::Hash256Body` / `Body::Hash160Body` newtype wrappers around fixed-width byte arrays so that encode/decode width is statically determined by tag (no on-wire length field needed). Phase 11 will introduce additional hash variants (Hash256, Ripemd160) in the extension space.

## Deferred to later phases

The `_ => unimplemented!()` arms in `write_node`/`read_node` continue to close out as:

- **Phase 11:** Extension space — Hash256, Ripemd160, RawPkH, False, True

## Carry-forward deferred items (Phases 1–10)

- **Phase 1:** `read_past_end_errors` state-preservation; unused `BitStreamExhausted` variant
- **Phase 2:** `write_varint` `debug_assert!` → `assert!` upgrade; L=0 hand-crafted decode test
- **Phase 4:** `PathDecl::write` `# Errors` rustdoc gap
- **Phase 5:** `UseSitePath::write` `# Errors` rustdoc gap
- **Phase 7:** arity-2/3 explicit unit-test coverage (defer to Phase 14 smoke)
- **Phase 9:** `Body::Tr` tap-script-tree leaf validation — enforce §6.3.1 forbidden-leaf-tag whitelist on encode and decode (defer to Phase 13)
- **Phase 10 (new):** _none_

## Next

Phase 11 — Extension-space ops (Hash256, Ripemd160, RawPkH, False, True).
