# v0.10 spec review (opus, pass 1)

**Date:** 2026-04-29
**Spec:** design/SPEC_v0_10_per_at_N_paths.md (commit 81745e6)
**Reviewer:** opus-4.7

## Summary

The spec faithfully encodes all 13 LOCKED brainstorm decisions and the tag-allocation, header-bit, encoder-dispatch, and migration claims hold up against the current codebase. However, two issues block "ship as-is": **F1** is a wrong byte sequence in the Example B walkthrough (multiple miscalculations of the explicit-path encoding), and **F2** is a missing piece of round-trip machinery (a `decoded_origin_paths` analogue of `decoded_shared_path`) that the encoder design glosses over. Both are easy to fix at spec-revision time. A handful of medium and small findings follow. **Verdict: needs-fixes-then-proceed.** Apply the §2 byte fix, add a §4 round-trip-stability paragraph, and the spec is ready to base a plan on.

## Findings

### F1: Example B byte sequence is mathematically wrong
**Severity:** strong
**Location:** SPEC_v0_10_per_at_N_paths.md §2 "Wire-format examples", Example B and Example C
**Issue:** The spec encodes `m/48'/0'/0'/100'` as `FE 04 60 00 00 C9`. Three errors:
1. The reviewer's own check in the briefing is correct: `48'` is hardened, so it must encode as `2*48 + 1 = 97 = 0x61`, not `0x60` (which would be `48` unhardened). The same applies to `0'` and `0'` — they must be `0x01`, not `0x00`.
2. `100'` encodes to `2*100 + 1 = 201`. 201 is `0xC9` as a *raw integer*, but the wire format uses LEB128 per `bytecode/path.rs::encode_path` (line 80) and `varint::encode_u64`. 201 has its high bit set, so LEB128(201) = `[0xC9, 0x01]` — two bytes, not one. Cross-check `encode_explicit_large_child_number` test in path.rs line 661: `m/100` (200 unhardened) encodes to `[0xFE, 0x01, 0xC8, 0x01]`.
3. The total byte count for the path is 6, not 5: `04 61 01 01 C9 01`.
**Recommendation:** Replace `FE 04 60 00 00 C9` (and the same in Example C) with `FE 04 61 01 01 C9 01`. Update the §2 prose and any byte-counting commentary accordingly. Re-verify against `path.rs::encode_path` byte-for-byte before approving the next pass.

### F2: Encoder/decoder round-trip stability needs a `decoded_origin_paths` field
**Severity:** strong
**Location:** SPEC_v0_10_per_at_N_paths.md §4 "Encoder Design", §3 "decode_origin_paths", and the implicit assumption in `placeholder_paths_in_index_order()`
**Issue:** The current `WalletPolicy` carries a `decoded_shared_path: Option<DerivationPath>` field (`crates/md-codec/src/policy.rs:197`) populated by `from_bytecode` so a `decode → encode` round-trip is first-pass byte-identical. The spec's encoder sketch says "for a `WalletPolicy`, this walks the key information vector and extracts the origin path for each placeholder" — but a template-only policy decoded from a v0.10 bytecode (which is the universal case, since the wire form has no concrete xpubs) has no key information vector to walk. Without a stashed `decoded_origin_paths: Option<Vec<DerivationPath>>` on `WalletPolicy`, a decode→encode round-trip would lose the per-`@N` divergence and re-emit `Tag::SharedPath`, breaking byte-stability.
**Recommendation:** Add a §4 paragraph explicitly:

> v0.10's `WalletPolicy` gains a `decoded_origin_paths: Option<Vec<DerivationPath>>` field, populated by `from_bytecode` when the bytecode used `Tag::OriginPaths`. The encoder's source-of-truth precedence chain (§4 "encoder dispatch") consults this field as Tier 1, parallel to the existing `decoded_shared_path` Tier 1. `placeholder_paths_in_index_order()` returns `decoded_origin_paths.clone()` when present, falling through to the existing tier chain for shared paths only when absent.

This also resolves Open Implementer Question 2 partially (the inheritance precedence chain).

### F3: `n_orig_paths_truncated` uses wrong error variant
**Severity:** nice-to-have
**Location:** SPEC §5 "Negative vectors", `n_orig_paths_truncated` description
**Issue:** The spec says the truncated case rejects with `BytecodeErrorKind::Truncated`. But `decode_path` (path.rs:104) and `decode_declaration` (path.rs:205) consistently emit `BytecodeErrorKind::UnexpectedEnd` for cursor exhaustion. `Truncated` exists in the enum (error.rs:377) but is used for length-prefix-declares-more-than-buffer cases, not cursor-mid-read exhaustion. Mixing the two introduces an inconsistency: the OriginPaths decoder reuses `decode_path` per path-decl, so each path-decl-internal truncation will surface as `UnexpectedEnd`, while the proposed top-level "ran out before reading N paths" case is stated as `Truncated`. The actual code path will produce `UnexpectedEnd`.
**Recommendation:** Update §5 to specify `BytecodeErrorKind::UnexpectedEnd` for `n_orig_paths_truncated`, matching the existing convention. (Alternatively, decide a uniform convention and apply both ways; but the conservative move is to match what the cursor primitives already produce.)

### F4: `count > 32` rejection variant is wrong-shaped
**Severity:** strong
**Location:** SPEC §3 "decode_origin_paths" code sketch, lines 202–205
**Issue:** The sketch returns `Err(Error::OriginPathsCountMismatch { expected: 0, got: count as usize })` when `count > 32`, with a comment that "the encoder-vs-decoder count consistency check happens at a higher layer." But `expected: 0` is misleading — the BIP 388 cap is 32, not 0, and the count-mismatch error implies a comparison against the tree placeholder count. Two distinct error conditions are being conflated:
1. `count` exceeds the BIP 388 32-placeholder cap (structural — caught at the bytecode layer).
2. `count` doesn't match the tree's actual placeholder count (semantic — caught at the policy-construction layer).

For (1), the right error is something like `Error::InvalidBytecode { kind: BytecodeErrorKind::OriginPathsCountTooLarge { count, max: 32 } }` or reuse of an existing variant. Stuffing it into `OriginPathsCountMismatch { expected: 0, got: 33 }` is confusing for the consumer.
**Recommendation:** Either (a) introduce a separate `BytecodeErrorKind::OriginPathsCountTooLarge` variant, or (b) reuse the structure of `Error::FingerprintsCountMismatch` and report `expected: 32` (or document that "expected 0" is sentinel for "not yet known," which is fragile). I prefer (a). This also aligns with the v0.6 strip-Layer-3 pattern of using `BytecodeErrorKind::*` for bytecode-layer structural errors and `Error::*` for higher-layer semantic errors.

### F5: `Error::ConflictingPathDeclarations` placement vs. existing `BytecodeErrorKind::UnexpectedTag`
**Severity:** nice-to-have
**Location:** SPEC §3 "Path-decl dispatch" and §4 "Error updates"
**Issue:** The spec proposes `Error::ConflictingPathDeclarations` (top-level Error variant) for the `(false, 0x36) | (true, 0x34)` case but `Error::InvalidBytecode { kind: BytecodeErrorKind::UnexpectedTag { expected, got } }` for arbitrary unknown bytes at offset 1. The reviewer's framing question — should both be the same variant? — pattern-matches with the existing v0.6 stripping logic where structural mismatches use `BytecodeErrorKind::UnexpectedTag` (see error.rs:434-447 "A tag byte was valid but not the tag expected at this position"). The conflicting-path case IS exactly that: a defined tag (`0x34` or `0x36`) appearing where the *other* one was expected per the header. So the existing `BytecodeErrorKind::UnexpectedTag` machinery already covers this; introducing `ConflictingPathDeclarations` as a peer top-level variant is a redundancy.
**Recommendation:** Replace `Error::ConflictingPathDeclarations` with `Error::InvalidBytecode { offset: 1, kind: BytecodeErrorKind::UnexpectedTag { expected: <0x34 or 0x36 per header bit>, got: <other tag byte> } }`. The diagnostic is sharper too: a CLI shows "expected 0x36 (OriginPaths), got 0x34 (SharedPath)" rather than "header bit and tag disagree" which makes the user hunt for which is wrong. Keep `ConflictingPathDeclarations` only if the team wants a dedicated string for this very specific error class — but my strong preference is the existing variant.

### F6: `count = 0` semantics undefined
**Severity:** nice-to-have
**Location:** SPEC §2 `Tag::OriginPaths` block, §5 negative vectors
**Issue:** What happens if `Tag::OriginPaths` declares `count = 0` (zero placeholders)? The spec says count must equal `max(@i) + 1` over tree placeholders. A wallet policy with zero placeholders is structurally invalid (BIP 388 requires `@N` references), so the decoder will detect the inconsistency at policy-construction time as `OriginPathsCountMismatch`. But there's no explicit MUST clause on the encoded count being ≥ 1, and an attacker could synthesize a bytestream with `0x36 0x00 <tree>` to test the boundary.
**Recommendation:** Add a one-sentence clarification under "0x36 | count: u8" in §2:

> `count` MUST be in `1..=32`. Zero is rejected at the bytecode layer (no valid wallet policy has zero placeholders); the decoder reports `Error::InvalidBytecode { kind: BytecodeErrorKind::OriginPathsCountTooSmall }` (or equivalent) before tree-walk.

And add a corresponding negative vector `n_orig_paths_count_zero`. Counter-argument: maybe drop the test in favor of relying on the higher-layer `OriginPathsCountMismatch` to fire when `count=0` doesn't match any positive tree placeholder count. Either is fine; just make the choice and document.

### F7: `walker_reports_first_violation` test claim is unfounded
**Severity:** nice-to-have
**Location:** SPEC §5 "Defensive-corpus byte-literal pinning", `origin_paths_walker_reports_first_violation`
**Issue:** The "walker" pattern in `crates/md-codec/src/bytecode/hand_ast_coverage.rs:486` (`walker_reports_deepest_violation_first`) is specifically for tap-leaf subset violations, where a depth-first AST walk visits leaves in DFS pre-order and the deepest violation is reported. The OriginPaths decoder is a flat sequential read of N paths — there's no "walker" in the same sense, just a `for _ in 0..count { decode_path(cursor)? }` loop that returns early on the first error. So the claimed test ("given multiple violations, decoder reports the first encountered") is trivially true by construction (early return on `?`); pinning it as a separate test is premature.
**Recommendation:** Drop `origin_paths_walker_reports_first_violation` from §5 unless there's a specific scenario where the decoder might process out of order (none exists in the proposed encoding). Or rename the assertion to `origin_paths_decoder_returns_on_first_error` and frame it as a defensive-corpus pin rather than a walker semantics test.

### F8: BIP 388 cap is 32 but encoder code uses `u8::try_from`
**Severity:** nice-to-have
**Location:** SPEC §4 encoder code sketch, lines 261–266
**Issue:** The encoder dispatches `u8::try_from(placeholder_paths.len())` and *then* checks `count_u8 > 32`. The `u8::try_from` only fires for `len > 255`; the 32-cap check is the meaningful one. A policy could have 33–255 placeholders and pass `u8::try_from` but fail the 32 check — that's fine but the error path is then `OriginPathsCountMismatch` with `expected: ...` being ambiguous (see F4). The current code in `policy.rs::to_bytecode` does the same dance for fingerprints (line 423–427 uses `u8::try_from` followed by the count match check earlier). Consistency with that pattern is OK.
**Recommendation:** Simplify the spec's encoder sketch to match the existing fingerprints-block emission pattern: validate the count (already done via the BIP-388 cap of `MAX_DUMMY_KEYS = 32` upstream in `to_bytecode`), then `u8::try_from(...).expect(...)` since BIP 388 caps placeholder count at 32. Or keep `try_from` defensive but elide the redundant `> 32` check at the encoder side. The decoder still validates `count > 32` independently.

### F9: `MAX_PATH_COMPONENTS = 10` rejection — encoder emits or rejects?
**Severity:** nice-to-have
**Location:** SPEC §2 "Path component count cap", §4 "encoder dispatch"
**Issue:** Spec says "Encoder rejects symmetrically before serialization" for `> 10` components. Good. But the existing `encode_path` in path.rs has *no* cap check — it just calls `encode_u64(path.len() as u64, &mut out)`. The §4 sketch says "Each path is validated for `component_count <= MAX_PATH_COMPONENTS` before emission" but doesn't specify *where*. Two design choices:
1. Add the cap check inside `encode_path` (affects both `Tag::SharedPath` and `Tag::OriginPaths` paths uniformly); or
2. Add the check at each caller (encoder dispatch loop and the SharedPath encoder).
**Recommendation:** Pick (1) — put the cap in `encode_path`. This automatically closes the SharedPath case (Q8 says cap applies to both), and the §3 sketch's "decode_path enforces MAX_PATH_COMPONENTS = 10 internally" parallel is symmetric. Update §4 to clarify: "Both `encode_path` and `decode_path` enforce the cap; the new `Tag::OriginPaths` block inherits this for free via re-use."

### F10: `BytecodeHeader` needs `#[non_exhaustive]` consideration
**Severity:** nice-to-have
**Location:** SPEC §4 "Type updates" — `BytecodeHeader::new_v0` signature change
**Issue:** The spec proposes `new_v0(bool)` → `new_v0(bool, bool)`, calling it a public-API break. But `BytecodeHeader` is `#[non_exhaustive]` (header.rs:31) precisely so that v1+ fields can be added without breakage. The `#[non_exhaustive]` only covers struct-field additions, not constructor signature changes — so the API break is real. Worth noting in the migration table that this is a deliberate signature revision.
**Recommendation:** Acceptable as-is, but consider an alternative builder API for v0.10 to avoid future churn:

```rust
impl BytecodeHeader {
    pub const fn new_v0() -> Self { ... }  // all flags false
    pub const fn with_fingerprints(self, on: bool) -> Self { ... }
    pub const fn with_origin_paths(self, on: bool) -> Self { ... }
}
```

Old `new_v0(bool)` deprecates; new API is forward-compatible. This costs a tiny amount of churn now and saves another bump every time a new flag bit is reclaimed. Probably overkill for v0.10's scope; mention as a v2-design-questions item if not adopted.

### F11: `decoded_shared_path` ↔ `decoded_origin_paths` mutual exclusion
**Severity:** nice-to-have
**Location:** Implicit — not in the spec
**Issue:** The "strict mutual exclusion at the path-decl slot" (Q3-A) is wire-level, but on the in-memory `WalletPolicy` side, if both `decoded_shared_path: Option<...>` and `decoded_origin_paths: Option<Vec<...>>` are independent fields, the encoder needs to handle the (impossible-but-defense-in-depth) case where both are `Some`. Could be enforced at construction site or papered over with a precedence rule in `to_bytecode`.
**Recommendation:** Add a one-line constraint in §4: "Invariant: at most one of `decoded_shared_path` and `decoded_origin_paths` is `Some`. `from_bytecode` populates exactly one based on which path-decl tag was on wire." Document this on the field rustdoc.

### F12: `o3_pkh_divergent_paths_n4` is a category error
**Severity:** nice-to-have
**Location:** SPEC §5 positive vectors, line 381
**Issue:** A `pkh()` policy is a single-key descriptor — it has exactly one `@N` placeholder (`@0`). A "4-`@N` policy exercising count=4 boundary" cannot be `pkh`. The natural shape is `wsh(multi(k, @0, @1, @2, @3))` or similar.
**Recommendation:** Rename and reshape — e.g., `o3_wsh_multi_4of4_divergent_paths` or `o3_sortedmulti_2of4_divergent_paths`.

### F13: PolicyId Type 0 / Type 1 typology link in §6 is correct
**Severity:** confirmation (no change)
**Location:** SPEC §6 "PolicyId Type 0 / Type 1 typology"
**Issue:** The reviewer's question — "is the prose 'two wallets with the same template and same path layout but *different* concrete cosigner xpubs share a `PolicyId`' still true under v0.10's Route X?" — yes, confirmed. `compute_policy_id_for_policy` (policy_id.rs:201) calls `policy.to_bytecode(&EncodeOptions::default())`, which under v0.10 will include the OriginPaths block (per §4 auto-detect) but still excludes xpubs (xpubs aren't part of the bytecode). So PolicyId now distinguishes per-`@N` path layouts but still collapses across xpub sets. WalletInstanceId remains the xpub-distinguishing identifier. The §6 prose is accurate.

### F14: Open implementer question 2 is partially answered by F2
**Severity:** nice-to-have
**Location:** SPEC Appendix A, question 2
**Issue:** "Per-`@N` path inheritance from key-information-vector?" is framed as deferred. But applying F2's recommendation pins the precedence chain at spec time, not plan time:
- Tier 0: `opts.origin_paths` override (parallel to existing `opts.shared_path`)
- Tier 1: `decoded_origin_paths` (new)
- Tier 2: walk key-info-vector and extract per-key origin (parallel to existing `shared_path()`)
- Tier 3: fall through to single shared-path tier chain (existing).
**Recommendation:** Either lift this into §4 explicitly (preferred, keeps spec self-contained) or note it as "spec-deferred but probable shape" in Appendix A so the plan-writer has a clear default.

### F15: Explicit-form path with N >= 128 components edge case
**Severity:** nice-to-have
**Location:** SPEC §2 implicitly
**Issue:** The Q8 cap of `MAX_PATH_COMPONENTS = 10` makes this moot, but worth noting: `decode_path_round_trip_multi_byte_component_count` (path.rs:686) tests 128-component paths explicitly. With the new cap, that test will need an `#[ignore]` or rewriting against a smaller bound, and the test's value (multi-byte LEB128 round-trip) survives at component count 10. Doesn't surface unless a careful reader notices.
**Recommendation:** Add to migration notes in §6: "The test `decode_path_round_trip_multi_byte_component_count` becomes inactive under v0.10's cap; rewrite to use multi-byte LEB128 in the *child index* dimension (e.g., `m/16384`) rather than the *component count* dimension."

## Confirmations

- **All 13 brainstorm questions are present in §1 decision matrix and faithfully reflect the LOCKED choices.** Q4 → bit 3, Q1 → 0x36, Q3 → strict mutual exclusion, Q7 → Route X, Q8 → 10, Q9 → auto-detect, Q12-Light, Q13-Light bundle. No drift.
- **`Tag::OriginPaths = 0x36` is free in current `Tag` enum.** Verified `tag.rs:188` (`0x36` returns `None` from `from_byte`). The high-bytes-unallocated test `tag_v0_6_high_bytes_unallocated` (line 296) explicitly covers `0x36..=0xFF`.
- **`RESERVED_MASK` change `0x0B` → `0x03` is backwards-compatible.** Confirmed via spot-check of `tests/vectors/v0.1.json` and `v0.2.json`: all `expected_bytecode_hex` values start with `00` or `04`. Both pass `0x03` mask.
- **Pre-v0.10 decoders correctly reject v0.10 OriginPaths-using encodings via `Error::ReservedBitsSet`.** Confirmed via `header.rs::reserved_bit_3_set` test (line 194).
- **`Tag::SharedPath = 0x34` and `Tag::Fingerprints = 0x35` byte values match.** Verified `tag.rs:122,127`.
- **`encode_path` / `decode_path` machinery is ready for OriginPaths re-use.** The LEB128-component-count form (`0xFE` + LEB128 count + per-component LEB128) is already implemented in `path.rs:65–164`. No structural change needed; just compose into the new tag wrapper.
- **`Error` is `#[non_exhaustive]`.** Confirmed `error.rs:47`. Adding new variants is API-additive.
- **`BytecodeErrorKind::UnexpectedTag` is reusable for header-bit-vs-tag conflicts.** See F5.
- **mk1 BIP §"Authority precedence" cross-reference is accurate.** Verified `bip-mnemonic-key.mediawiki:362–369`. The proposed md1 cross-reference text in §6 captures the normative claim ("MK authoritative; MD descriptive; orchestrator rejects mismatch; per-format decoders not required to be aware of cross-format context"). One small polish: mk1 BIP additionally requires "Implementations MUST surface a precise error identifying both the policy-side expected path and the key-side actual path" — the md1 cross-reference could mention this orchestrator-side error-surfacing duty for symmetry, but it's the orchestrator's job, not md1's, so acceptable to elide.
- **PolicyId remains xpub-invariant under Route X.** Verified `policy_id.rs:201` (`compute_policy_id_for_policy` calls `to_bytecode` which never embeds xpubs).
- **mk1 SPEC §5.1 normative text matches the proposed md1 cross-reference.** Verified `mk1/design/SPEC_mk_v0_1.md:318–326`. Identical normative claim, includes the orchestrator-error-surfacing requirement.
- **Test vector family-token roll is documented and matches the v0.9 pattern.** §6 "Family-token roll" subsection is correctly framed.

## Open questions for the implementer

1. **Should `decoded_origin_paths` be a `Vec` or a more structured type?** A bare `Vec<DerivationPath>` works but loses information at the type level (it's "indexed by placeholder position"). A wrapper `OriginPaths(Vec<DerivationPath>)` could carry an invariant. Probably YAGNI; flag if a use case appears.

2. **Should `compute_policy_id_for_policy` for v0.10 take an `EncodeOptions` parameter?** Currently it always uses `default()`. If a caller wants the v0.9-style PolicyId (lossy SharedPath form) for backwards-compat hashing, they have no way to ask for it. Probably WAI for v0.10 (Route X is intentional and the v0.9 PolicyIds for divergent-path policies were "wrong" by design); but document the migration impact.

3. **Encoder behavior when `decoded_origin_paths.len() != key_count()`?** Decode→edit→encode workflows could in principle change the placeholder count (adding or removing keys via `set_key_info`). The encoder needs a robust answer: reject? clear `decoded_origin_paths` and re-derive? Defer to plan.

4. **Should the spec mandate a stable iteration order for OriginPaths even when the decoder is permissive?** The wire-level order is fixed (placeholder-index order), but the in-memory `decoded_origin_paths` could in principle be reordered. Plan-time concern; flag if the round-trip stability hinges on it.

5. **CHANGELOG framing (§6) — is this a "0.10.0" or "0.10.0-rc.1"?** The wire-format break warrants extra caution; some projects ship an RC for the wire-level change. Probably overkill given the v0.x context, but worth a quick decision.
