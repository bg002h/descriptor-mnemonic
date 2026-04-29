# v0.7.0 Phase 1 — `bytecode::{decode,encode,path}::tests` rebaseline

**Branch:** `feature/v0.7.0-development`
**Predecessor commit:** `35caa24` (policy::tests rebaseline)
**Scope:** the 27 enumerated failing unit tests across `bytecode::decode::tests`, `bytecode::encode::tests`, `bytecode::path::tests`.

## Summary

- 26 of the 27 enumerated tests rebaselined using symbolic `Tag::Foo.as_byte()` references where the byte represents a tag.
- 1 test deleted as redundant: `decode_rejects_top_bare_legacy` (rationale below).
- 1 incidental source-file change: a one-string update to a diagnostic message in `decode_descriptor`'s `Tag::TapTree` arm to preserve test intent.
- Final unit-test run: **`cargo test -p md-codec --lib` → 431 passed; 0 failed.**

## Files touched

- `crates/md-codec/src/bytecode/decode.rs` — 14 unit-test rebaselines + 1 deletion + 1 production-code message tweak.
- `crates/md-codec/src/bytecode/encode.rs` — 4 unit-test rebaselines (`super::*` already imports `Tag`, no module-level import added).
- `crates/md-codec/src/bytecode/path.rs` — 7 unit-test rebaselines + new `use crate::bytecode::Tag;` in test mod.
- `design/FOLLOWUPS.md` — appended `v0-7-phase-1-integration-test-rebaseline`.

## Per-test changes

### `bytecode::decode::tests` (16 enumerated → 15 fixed + 1 deleted)

| Test | Action | Notes |
|---|---|---|
| `decode_andor_known_vector` | rebaselined | `[0x05, 0x13, 0x00, 0x01, 0x00]` → 5×`Tag::*.as_byte()` |
| `decode_andor_rejects_truncated_after_two_children` | rebaselined | `[0x05, 0x13, 0x00, 0x01]` → 4×`Tag::*.as_byte()` |
| `decode_logical_op_rejects_truncated_after_first_child` | rebaselined | `[0x05, 0x11, 0x01]` → `[Wsh, AndV, True]` symbolic |
| `decode_multi_rejects_truncated_after_k` | rebaselined | `[0x05, 0x19, 0x02]` → `[Wsh, Multi, 0x02]` (0x02=k payload, kept literal) |
| `decode_multi_rejects_truncated_mid_keys` | rebaselined | placeholder index bytes (`0x00`, `0x01`) kept literal as payload values |
| `decode_or_d_known_vector` | rebaselined | `[0x05, 0x16, 0x00, 0x01]` → 4×`Tag::*.as_byte()` |
| `decode_rejects_reserved_inline_key_tag` | semantics-preserving rewrite | In v0.5, byte `0x24` was `ReservedOrigin` and the decoder emitted a `PolicyScopeViolation` mentioning "inline-key". In v0.6 the entire `0x24..=0x32` Reserved* range was DROPPED (per `tag.rs` crate-level rationale: BIP-388 framing forbids inline keys). The byte now produces a generic `UnknownTag(0x24)` diagnostic. The test's intent — "v0.5's Reserved* sub-range is rejected" — is preserved by asserting the v0.6 unknown-tag path on the same input byte. The rewrite includes a comment block documenting why the assertion shape changed. **Note: test intent preserved but assertion shape changed; flagged here as the only unit test where the rebaseline went beyond a pure byte rename.** |
| `decode_rejects_sh_key_slot_placeholder` | rebaselined | `[Sh, Placeholder, 0x00]` symbolic |
| `decode_rejects_top_bare_legacy` | **DELETED** | Redundant with `taptree_at_top_level_produces_specific_diagnostic`. In v0.5 the test fed byte `0x07` (Bare) and asserted "bare" appears in the rejection message. In v0.6, byte `0x07` is `Tag::TapTree`; rejection at the top level is now via the explicit `Tag::TapTree` arm in `decode_descriptor`, which produces a TapTree-specific diagnostic. There is no longer a separate code path that emits a "bare" message at the top level. The semantically-equivalent "0x07 at top is rejected" coverage lives in `taptree_at_top_level_produces_specific_diagnostic`. Deletion replaced with an explanatory comment block at the same location for future readers. |
| `decode_sh_wpkh_round_trip` | rebaselined | `[Sh, Wpkh, Placeholder, 0x00]` symbolic; placeholder index payload kept literal |
| `decode_sh_wsh_sortedmulti_round_trip` | rebaselined | full byte vec rewritten with named tag refs for each tag byte; k/n/index payloads kept literal |
| `decode_terminal_alt_swap_directly` | rebaselined | both `alt_bytes` and `swap_bytes` rewritten (`Tag::Alt`, `Tag::Swap`, `Tag::True`) |
| `decode_thresh_with_constants_round_trip` | rebaselined | `[Wsh, Thresh, k, n, False, True, False]` |
| `decode_wpkh_round_trip` | rebaselined | `[Wpkh, Placeholder, 0x00]` |
| `decode_wsh_body_returns_inner_wsh_not_descriptor` | rebaselined | `[SortedMulti, k, n, Placeholder×3]` symbolic |
| `taptree_at_top_level_produces_specific_diagnostic` | rebaselined + production-code tweak | The test asserts the diagnostic message contains `"TapTree"` and `"0x08"` (v0.5). In v0.6 TapTree=`0x07`, but the production-code diagnostic message in `decode_descriptor` did not include the byte at all — so the test would fail under v0.6 even with the literal updated. **Production-code change:** the message string was updated from `"TapTree is not a valid top-level descriptor; ..."` to `"TapTree (0x07) is not a valid top-level descriptor; ..."` (one inline byte mention added for diagnostic completeness, parallel to the v0.5 phrasing). The test was then updated to assert `"0x07"` and the rustdoc comment was updated to note both bytes. This was the single source-file change in the rebaseline pass and was driven by preserving test intent. |

### `bytecode::encode::tests` (4 enumerated)

| Test | Action |
|---|---|
| `encode_placeholder_index_above_127_uses_single_byte` | rebaselined: expected `[Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 0xC8]` (200=`0xC8` is the placeholder index payload, kept literal) |
| `encode_terminal_multi_2_of_3` | rebaselined: 9-byte expected vector rewritten with `Tag::Multi`, `Tag::Placeholder` symbolic refs; k/n/index payloads (`0x02`, `0x03`, `0x00`/`0x01`/`0x02`) kept literal |
| `encode_terminal_pk_h_with_placeholder` | rebaselined: `[Tag::PkH.as_byte(), Tag::Placeholder.as_byte(), 0x07]` |
| `encode_terminal_pk_k_with_placeholder` | rebaselined: `[Tag::PkK.as_byte(), Tag::Placeholder.as_byte(), 0x00]` |

### `bytecode::path::tests` (7 enumerated)

The path tests had a single dominant shift: `Tag::SharedPath` byte `0x33`→`0x34`. Added `use crate::bytecode::Tag;` to the test mod (was not previously imported via `super::*`).

| Test | Action |
|---|---|
| `declaration_round_trip_dict` | `0x33` → `Tag::SharedPath.as_byte()` |
| `declaration_round_trip_explicit` | `0x33` → `Tag::SharedPath.as_byte()` |
| `decode_declaration_rejects_truncated` | input `[0x33]` → `[Tag::SharedPath.as_byte()]` |
| `decode_declaration_rejects_wrong_tag` | wrong-tag input `[0x05, 0x01]` → `[Tag::Wsh.as_byte(), 0x01]`; `expected: 0x33` and `got: 0x05` in the assertion's match arm replaced with a guard clause comparing to `Tag::SharedPath.as_byte()` and `Tag::Wsh.as_byte()` (struct-pattern positional matching can't bind dynamic values, so this is the idiomatic symbolic form) |
| `encode_declaration_dictionary_byte_layout` | `[0x33, 0x01]` → `[Tag::SharedPath.as_byte(), 0x01]` |
| `encode_declaration_explicit_byte_layout` | `[0x33, 0xFE, 0x02, 0x59, 0x00]` — first byte symbolic; explicit-form marker `0xFE`, count `0x02`, hardened `0x59`, unhardened `0x00` kept literal as payload values |
| `from_bytes_propagates_errors` | wrong-tag and truncated cases — same pattern as `decode_declaration_rejects_wrong_tag` |

## Symbolic-vs-literal policy applied

Per the user's "symbolic Tag refs" preference, replaced byte literals with `Tag::Foo.as_byte()` ONLY when the byte represents a tag (i.e., is read by `Tag::from_byte`). Kept raw literals for:

- LEB128 varint payloads (e.g., timelock values)
- Threshold k/n bytes
- Placeholder index bytes (now single-byte 0..=255 per D-7)
- Path-encoding payloads (explicit-marker `0xFE`, hardened-encoded child numbers, count bytes)
- Hash payload bytes (32-byte sha256/hash256, 20-byte hash160/ripemd160)

This matches the `35caa24` policy::tests precedent.

## Acceptance criteria

| # | Criterion | Status |
|---|---|---|
| 1 | `cargo test -p md-codec --lib` 0 failures | **PASS** (431 passed; 0 failed) |
| 2 | Each fixed test preserves intent (same scenario, same assertion semantics) | **PASS** with one annotated exception: `decode_rejects_reserved_inline_key_tag` — the v0.6 codebase no longer emits a "inline-key" diagnostic for `0x24` (Reserved* range was structurally dropped); the test now asserts the v0.6-correct `UnknownTag(0x24)` path on the same input. The rebaseline includes an in-test comment explaining the change. |
| 3 | No new tests added; 1 test deleted with rationale | **PASS** — `decode_rejects_top_bare_legacy` deleted as redundant with `taptree_at_top_level_produces_specific_diagnostic` (see table above). |
| 4 | Symbolic Tag refs where byte is a tag; literals otherwise | **PASS** |
| 5 | Commit with the mandated message format | (executed at the end of this report) |

## Anomalies / risk surfaces

1. **Production-code message change in `decode_descriptor`**: I added `"(0x07)"` to the `Tag::TapTree` arm's diagnostic message to preserve test intent for `taptree_at_top_level_produces_specific_diagnostic`. This is a semantic change to a user-facing error message. Justification: (a) the v0.5 message included `"(0x08)"` per commit `59797ef`; the byte mention was lost in the v0.6 reorganization, and (b) including the byte in the diagnostic is genuinely helpful for users debugging encode/decode mistakes. The test assertion shape (must contain `"TapTree"` AND `"0x07"`) is preserved.

2. **Test deletion** (`decode_rejects_top_bare_legacy`): documented in the table above. Replaced with an explanatory comment block in-place so future readers understand the v0.5→v0.6 transition.

3. **Pre-existing failures outside scope**: `cargo test -p md-codec --no-fail-fast` after this commit still has ~17 failures across `tests/cli.rs`, `tests/conformance.rs`, `tests/vectors_*.rs`, and `tests/build_test_vectors.rs`. These are the same v0.5→v0.6 byte-shift class as the unit tests but were NOT in the enumerated 27. They were not in the original Phase 1 plan failing-test inventory (plan §1.1.2 estimated ~38 across all modules but the actual count is higher). Filed as `v0-7-phase-1-integration-test-rebaseline` in `design/FOLLOWUPS.md` (tier: v0.7-blocker — must close before Phase 6 release plumbing).

## FOLLOWUPS appended

- `v0-7-phase-1-integration-test-rebaseline` — open, v0.7-blocker

## No anomalies in the strict sense

No tests revealed a real bug requiring deeper changes. Every rebaselined test passes after the byte shift; no new bytecode pathways exposed; no upstream miniscript surprises.
