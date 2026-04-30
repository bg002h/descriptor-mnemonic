# Phase 23 Final Review — v0.11 Wire Format Implementation

**Date:** 2026-04-30
**Branch:** `feature/v0.11-impl-phase-1`
**Final cutover commit:** `5e7cddc`
**Commits ahead of `main`:** 69

---

## Executive summary

The v0.11 wire-format redesign for `md-codec` is **complete**. The work spans 23 implementation phases plus one consolidated phase-polish commit, executed under per-phase TDD discipline with opus reviews persisted under `design/agent-reports/`.

The final cutover (Task 23.1, commit `5e7cddc`) re-exports the `v11` module at the crate root as the canonical md-codec API, alongside the legacy v0.x surface. To avoid name collision with v0.x types still living at the root (notably `ChunkHeader` and `Tag`), v11 exposes `V11ChunkHeader` and `V11Tag` aliases.

- **Total v11 test count:** 106
  - v11 lib (`cargo test -p md-codec --lib v11`): 94 passed
  - `v11_smoke` integration: 8 passed (BIP 84 / BIP 86 single-sig; BIP 48 2-of-3 multisig; vault `or_d/older`)
  - `v11_forward_compat`: 1 passed
  - `v11_chunking`: 3 passed (single-chunk split; chunk_set_id derivation; split→reassemble round-trip)
- **Workspace tests:** 578 passed, 0 failed (`cargo test --workspace`)
- **Clippy:** `cargo clippy -p md-codec --all-targets -- -D warnings` clean
- **Deferred items:** all 7 cleared in the phase-polish commit pair (`38fe372` + `dcaea5f`); none remain.

---

## Architecture & implementation milestones

### 1. Bit-aligned wire-format primitives (Phases 1–2)

`BitWriter` / `BitReader` provide MSB-first bit-level packing/unpacking with `with_bit_limit` scoping for nested length-prefixed regions and save/restore cursor checkpoints for speculative-decode rollback. The LP4-ext varint encodes lengths in 4 bits with a single 8-bit extension prefix (covering values up to 2^29 − 1); recursive extension is deferred to v0.12+.

### 2. Header & path declarations (Phases 3–5)

The 5-bit fixed header encodes `version` + reserved bits + `divergent_paths` flag (Decision D9, D29). Origin-path declarations cover both shared and divergent paths; use-site declarations override per-`@N` placeholder bindings.

### 3. Tag space & operator tree (Phases 6–11)

The 36-operator tag space packs 31 primary 5-bit tags plus a single 5-bit extension prefix expanding into 5 extension 10-bit tags. Operator dispatch covers `KeyArg`, `Children`, `Variable`, `Tr`, terminals, hash literals, and `False`/`True`.

### 4. TLV section (Phase 12)

Carries `UseSitePathOverrides`, `Fingerprints`, and preserves unknown TLV tags through round-trip for forward compatibility.

### 5. Validation (Phase 13)

BIP 388 placeholder usage rules, multipath consistency across `@N` placeholders, and tap-script-tree leaf-only restriction.

### 6. End-to-end encode/decode (Phases 14, 19, 20)

`encode_payload` / `decode_payload` plus the codex32-wrapped `encode_md1_string` / `decode_md1_string`. Symbol-aligned BCH wiring (Phase 19) saved one character per encoding versus byte-alignment. Phase 20 round-tripped four representative real-world wallets end-to-end.

### 7. Identity & display (Phases 15, 18)

`Md1EncodingId` (engraving-specific 128-bit hash) and `WalletDescriptorTemplateId` (γ-flavor template hash). BIP-39 12-word phrase rendering and `render_codex32_grouped` for human-readable output.

### 8. Chunking (Phases 16, 21)

`ChunkHeader` (37 bits) plus split/reassemble with codex32 integration; cross-chunk integrity is anchored via `Md1EncodingId`.

### 9. Forward-compat (Phase 17)

Unknown TLV tags survive a full encode→decode→encode cycle, enabling additive future evolution.

### 10. Final cutover (Phase 23)

Public API surface re-exports v11 as canonical, with `V11Tag` and `V11ChunkHeader` aliases for collision avoidance with the still-present v0.x types at crate root.

---

## Spec coverage

The v0.11 specification sections fully covered:

- §1, §2, §3 — wire layout
- §4 — encoding primitives
- §5 — tag space
- §6 — operator bodies
- §7 — validation
- §8 — identity
- §9 — chunking
- §10 — display
- §11 — forward-compat
- §13 — decoder/encoder
- §14 — worked examples

All brainstorm decisions D1–D39 are reflected in the implementation.

---

## Test summary

| Suite | Count | Status |
|---|---|---|
| md-codec v11 lib (`--lib v11`) | 94 | pass |
| `v11_smoke` integration | 8 | pass |
| `v11_forward_compat` | 1 | pass |
| `v11_chunking` | 3 | pass |
| **v11 cumulative** | **106** | **pass** |
| Workspace cumulative (`cargo test --workspace`) | 578 | pass |
| `cargo clippy -p md-codec --all-targets -- -D warnings` | — | clean |

`v11_smoke` covers BIP 84 single-sig, BIP 86 taproot single-sig, BIP 48 2-of-3 sortedmulti, and a `or_d`/`older` vault descriptor.

---

## Authorized deviations from plan (all resolved)

- **Phase 1:** `BitWriter` `Default` derive added during Task 1.3 to satisfy `clippy::new_without_default`.
- **Phase 2:** `varint_max_u31` test value adjusted from 2^31 − 1 to 2^29 − 1 to match the LP4-ext single-extension cap; recursive extension deferred to v0.12+.
- **Phase 3:** `clippy::identity_op` cleanup (`| 0u64` removed).
- **Phase 7:** TDD red→green discipline relaxed for some tasks where the spec was prescriptive enough that drafting a failing test added no information.
- **Phase 12:** TLV decoder loop `while remaining_bits >= 5` workaround added; replaced in Phase 19 with proper rollback via `BitReader` save/restore.
- **Phase 13:** `ForbiddenTapTreeLeaf` payload changed from `String` to `u8` in phase-polish for stable error semantics.
- **Phase 23:** `V11ChunkHeader` alias introduced to avoid collision with v0.x's `ChunkHeader` at crate root; same treatment for `V11Tag`.

---

## Phase-polish commits (`38fe372` + `dcaea5f`)

All 7 deferred items cleared:

- **P1.a** state-preservation across BitWriter/BitReader operations
- **P1.b** unused `BitStreamExhausted` variant dropped
- **P2.a** `write_varint` `debug_assert` upgraded to `assert`
- **P2.b** `L = 0` hand-crafted varint test added
- **P4** `PathDecl::write` `# Errors` rustdoc added
- **P5** `UseSitePath::write` `# Errors` rustdoc added
- **P12** ✅ resolved earlier in Phase 19 (rollback decoder)
- **P13a** `u8` discriminant for `ForbiddenTapTreeLeaf`
- **P13b** `debug_assert` on internal bounds
- *(Bonus)* `clippy::manual_div_ceil` in `chunk.rs` cleaned up

---

## Wire-format size summary

Common-case engraving lengths (codex32 chars including `md1` HRP + BCH checksum):

| Wallet shape | Length | Notes |
|---|---|---|
| BIP 84 single-sig | 28 chars | symbol-aligned packing saved 1 char vs byte-alignment |
| BIP 86 taproot single-sig | ~28 chars | similar savings |
| BIP 48 2-of-3 sortedmulti | ~37 chars | |
| Vault (`or_d` / `older`) | ~50 chars | |

---

## Next steps

1. Apply `superpowers:finishing-a-development-branch` to merge / open PR for `feature/v0.11-impl-phase-1`.
2. Future v0.12 will ship the Xpubs TLV (tag 0x02) — purely additive per Decision D31′; no v0.11 wire-format break.
3. Future v0.12+ / vendor extensions may re-introduce dictionaries (path, use-site, shape) per §6 Future Considerations — see `design/REJECTEDFORNOW.md`.

---

## Final status

**v0.11 IMPLEMENTATION: COMPLETE.**

- Final cutover: `5e7cddc`
- 106 v11 tests passing; 578 workspace tests passing
- Clippy clean under `-D warnings`
- 0 deferred items
- 69 commits ahead of `main`, ready for branch finishing
