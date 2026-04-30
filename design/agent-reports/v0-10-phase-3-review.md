# v0.10.0 Phase 3 review (opus)

**Date:** 2026-04-29
**Commit:** `6913c5e` (`feat(v0.10-p3): policy-layer integration for OriginPaths`)
**Baseline:** `aa9041b` (Phase 2 wording-fix follow-up; substantive Phase 2 at `1936b19`)
**Reviewer:** opus-4.7 (1M context)
**Branch:** `feature/v0.10-per-at-n-paths`

## Verdict

CLEAN — proceed to Phase 4.

Phase 3 wires the new `Tag::OriginPaths` block into `WalletPolicy::to_bytecode`/
`from_bytecode`, adds `decoded_origin_paths`, implements the 4-tier precedence
chain (with Tier 2 stubbed and FOLLOWUPS-tracked), enforces the count-mismatch
invariant, and threads the `EncodeOptions::origin_paths` Tier 0 override. All
9 new tests pass; build/clippy/fmt clean; the only failing workspace test is
the expected `every_error_variant_has_a_rejects_test_in_conformance` gate
that Phase 4 closes. The Tier 2 stub is well-justified by the fork's API
constraints (private `template`/`key_info`, AST-order `iter_pk()`), is
correctly documented, and is plan-authorized; no v0.x ≤ 0.9 regression results
from it. No blockers, no inline-fix items.

## Scope reviewed

- Commit `6913c5e` against baseline `aa9041b`.
- Files inspected (full diffs read):
  - `crates/md-codec/src/policy.rs` (+551 / −70 effective)
  - `crates/md-codec/src/options.rs` (+39)
  - `crates/md-codec/src/bytecode/path.rs` (-6: removal of `#[allow(dead_code)]` and its 5-line preamble comment)
  - `design/FOLLOWUPS.md` (+44 — new entry `v010-p3-tier-2-kiv-walk-deferred`)
  - `design/agent-reports/v0-10-phase-3-implementer.md` (+104)
- Cross-references checked:
  - `design/IMPLEMENTATION_PLAN_v0_10_per_at_N_paths.md` Phase 3 (steps 3.1–3.10).
  - `design/SPEC_v0_10_per_at_N_paths.md` §3 (Path-decl dispatch) and §4 (4-tier precedence).
  - Phase 2 review (`design/agent-reports/v0-10-phase-2-review.md`).
  - Fork's `WalletPolicy` (`/scratch/code/shibboleth/rust-miniscript-fork/src/descriptor/wallet_policy/mod.rs`) to verify Tier 2 stub justification.
- Build/test verification:
  - `cargo build --workspace --all-features`: clean.
  - `cargo test --workspace --all-features --no-fail-fast`: **701 ok / 1 failed** (matches implementer's claim; the one failure is the expected conformance gate).
  - `cargo test --package md-codec --lib policy::`: 51/51 passing — confirms all 9 new tests + all pre-existing v0.9 round-trip tests still green.
  - `cargo clippy --workspace --all-features --all-targets -- -D warnings`: clean.
  - `cargo fmt --all -- --check`: clean.

## Findings

### 1 — Stale inline comments `// 0x33` near `Tag::SharedPath.as_byte()` (pre-existing; already FOLLOWUPS-tracked)

- **Severity:** Minor (cosmetic/comment drift)
- **Disposition:** Already covered by existing FOLLOWUPS entry `tag-sharedpath-rustdoc-stale-0x33`; no new entry needed
- **Description:** `crates/md-codec/src/policy.rs:1455` (in the **pre-existing** `wsh_no_origin_default_unchanged_from_v0_3` test, around bytes-vec construction) reads `Tag::SharedPath.as_byte(),  // 0x33`. Actual `Tag::SharedPath = 0x34` (verified at `bytecode/tag.rs:122`); `0x33` is `Tag::Placeholder`. Line 1098 has the same stale annotation in a docstring `"byte[1] must be Tag::SharedPath (0x33)"`. Both predate v0.10 (v0.5→v0.6 byte-shift sweep was incomplete; FOLLOWUPS entry `tag-sharedpath-rustdoc-stale-0x33` already tracks the broader class of stale `// 0x33`/`// 0x32` annotations across `path.rs` and `policy.rs`).
- **Why mention now:** Phase 3 adds a sibling test (`round_trip_shared_path_byte_identical_to_v0_9`) that uses `Tag::SharedPath.as_byte()` WITHOUT a stale numeric annotation. Reading the two tests in adjacent screens makes the drift visible. Not Phase 3-introduced; not a Phase 3 deliverable.
- **Suggested fix:** None for Phase 3. The existing `tag-sharedpath-rustdoc-stale-0x33` FOLLOWUPS entry (FOLLOWUPS.md:130) is the correct tracker; sweep `policy.rs:1098, 1455` (and any other `// 0x33` near `SharedPath` / `// 0x32` near `Placeholder`) when that entry is acted on. Suitable for a v0.10.x or v0.11 housekeeping window.

### 2 — Direct unit test for `OriginPathsCountMismatch` is absent (deferred to Phase 4)

- **Severity:** Minor (covered, just not directly)
- **Disposition:** Acknowledge-only — Phase 4 owns this
- **Description:** The new check at `policy.rs:750-758` raises `Error::OriginPathsCountMismatch` when the decoded `paths.len() != policy.key_count()`. No Phase-3 test directly exercises this — the closest thing is the conformance-gate failure (`every_error_variant_has_a_rejects_test_in_conformance`) which is intentionally left as the Phase 4 contract (the implementer's claim).
  Constructing such a test by hand is non-trivial: you need a synthetic bytecode whose `Tag::OriginPaths` block carries N paths but whose tree decodes to M placeholders ≠ N. The structural cap is enforced upstream (`OriginPathsCountTooLarge` for count=0 or count>32), so the only way to land this state is to hand-mint bytes — exactly the conformance vector that Phase 4 plans to add. No-action-needed for Phase 3.
- **Suggested fix:** None; verify the Phase 4 plan adds a `rejects_origin_paths_count_mismatch` test in `conformance.rs` (per `error_coverage.rs:65`).

### 3 — Tier 2 stub is correctly authorized and well-documented

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** `try_extract_paths_from_kiv` returns `Ok(None)` with a clear docstring (`policy.rs:491-510`) and a `// TODO(v0.10-followup):` comment pointing at the FOLLOWUPS entry. The justification — fork's `WalletPolicy::template` and `key_info` fields are private, and `descriptor.iter_pk()` returns AST-order — is verified directly against the fork source: `/scratch/code/shibboleth/rust-miniscript-fork/src/descriptor/wallet_policy/mod.rs:43-48` confirms both fields are private with no public accessors; `from_descriptor_unchecked` (line 132) builds `key_info` via `descriptor.iter_pk().collect()`, locking it into AST order even after a round-trip. There is no clean way to land a correct Tier 2 walk without either (a) upstreaming a fork accessor or (b) refactoring the policy-construction layer to capture per-`@N` paths during ingest. Either approach justifies a separate design pass, so the stub is the right call for v0.10.0.
- See "Tier 2 stub assessment" section below for the full verdict.

### 4 — `placeholder_paths_in_index_order` early-returns at first `Some`; tier ordering correct

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** Read `policy.rs:461-489`. Logic:
  - Tier 0: `if let Some(ref paths) = opts.origin_paths { return Ok(paths.clone()); }`
  - Tier 1: `if let Some(ref paths) = self.decoded_origin_paths { return Ok(paths.clone()); }`
  - Tier 2: `if let Some(paths) = self.try_extract_paths_from_kiv()? { return Ok(paths); }`
  - Tier 3: `Ok(vec![shared; count])`
  Each tier early-returns at the first `Some`. Tier 0 wins over Tier 1 → confirmed. The four-tier chain is also correctly ordered relative to the spec §4 ordering. The `debug_assert_eq!(placeholder_paths.len(), count, …)` at line 407 is sound defense-in-depth (Tier 0 callers could supply a wrong-length slice; the divergence-detection windows would still produce correct output, but `count_u8` arithmetic in Step 6 might mismatch). The plan's defense-in-depth note is followed.

### 5 — Divergence-detection edge cases (`count == 0`, `count == 1`)

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** `windows(2).all(|w| w[0] == w[1])` returns `true` for slices of length 0 or 1 (no adjacent pairs to compare → `all` is vacuously true). For `count == 1`, this trivially selects SharedPath, which is correct (single placeholder cannot diverge from itself). For `count == 0`, the policy would be malformed anyway (BIP 388 requires ≥1 placeholder), but the encoder would still produce a valid SharedPath wire form.
  However: `key_count()` for a malformed (zero-placeholder) policy returns 0; if such a policy ever made it past `from_str`, the encoder would hit Tier 3 and return `vec![shared; 0]` (empty vec), then `windows(2).all` would return true, header bit 3 = 0, and `encode_declaration(&placeholder_paths[0])` would **panic** with index-out-of-bounds. The test corpus does not exercise this path because `from_str` rejects malformed policies upstream. Not a blocker; recording as a future-proofing question only if Phase 4/5 tests ever stress-load synthetic zero-placeholder policies.

### 6 — Mutual exclusion invariant is structurally guaranteed, not just tested

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** Read `from_bytecode_with_fingerprints` at `policy.rs:597-618`. The branch:
  ```rust
  let (decoded_shared_path, decoded_origin_paths) = if header.origin_paths() {
      // … decode_origin_paths
      (None, Some(paths))
  } else {
      // … decode_declaration
      (Some(path), None)
  };
  ```
  Both arms unconditionally produce one `Some` and one `None`; the invariant "at most one of the two is `Some`" is structurally guaranteed by Rust's type system at this construction site. No test is needed to enforce it (the `decoded_shared_path_and_decoded_origin_paths_mutually_exclusive_after_decode` test is a verification, not the only line of defense).

### 7 — `EncodeOptions::origin_paths` Tier 0 override is non-mutating + builder is correct

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** Read `options.rs:84` (field declaration), `options.rs:171-174` (`with_origin_paths` builder), and `policy.rs:469-471` (Tier 0 lookup). Field is `Option<Vec<DerivationPath>>`; `with_origin_paths` consumes self and returns self (idiomatic builder); the Tier 0 lookup uses `paths.clone()` (does not move from `opts`), so callers can re-use the same `EncodeOptions` for multiple `to_bytecode` calls without surprise. `Default for EncodeOptions` correctly initializes the new field to `None` (verified by `encode_options_default_is_all_off` at `options.rs:230`).

### 8 — `from_bytecode` cursor pattern is consistent with existing style

- **Severity:** N/A (positive confirmation)
- **Disposition:** Acknowledge-only
- **Description:** The implementer notes the cursor is `Cursor::new(&bytes[1..])` instead of full-stream + offset bookkeeper. Verified at `policy.rs:596`. This matches the pre-existing pattern (see `path_consumed = cursor.offset()` at line 619) and uses the cursor's offset directly to track how many path-declaration bytes were consumed. The fingerprints block at lines 633-687 then uses `cursor_offset = 1 + path_consumed` and continues with raw byte indexing (mirroring the pre-Phase-3 style). The OriginPaths arm reads its tag byte via `cursor.read_byte()` (line 600), then validates against `Tag::OriginPaths.as_byte()` — matching the existing Fingerprints-block tag-validation pattern at lines 642-650. Style consistent; no regression.

## Spec-compliance verification

Mapping plan steps 3.1–3.10 to landed code:

| Step | Status | Notes |
|---|---|---|
| 3.1: Add `decoded_origin_paths: Option<Vec<DerivationPath>>` to WalletPolicy | ✅ | At `policy.rs:200-214` with full docstring including the mutual-exclusion invariant. |
| 3.2: Implement `placeholder_paths_in_index_order` 4-tier chain | ✅ | At `policy.rs:461-489`; tiers 0/1/2/3 in correct order with early-return. |
| 3.3: Stub Tier 2 `try_extract_paths_from_kiv` returning `Ok(None)` | ✅ | At `policy.rs:511-515` with TODO comment + docstring referencing FOLLOWUPS entry. |
| 3.4: Extract `resolve_shared_path_fallback` for Tier 3 | ✅ | At `policy.rs:522-537`; preserves the existing 4-step Phase B chain (`opts.shared_path` → `decoded_shared_path` → `shared_path()` → `default_path_for_v0_4_types` → BIP 84). Refactor of existing inline chain (architectural surprise #1 in implementer report). |
| 3.5: Rewrite `to_bytecode` to dispatch on auto-detected divergence | ✅ | At `policy.rs:355-449`; resolves per-`@N` paths, applies `windows(2).all(eq)` divergence test, emits SharedPath or OriginPaths accordingly, header bit 3 reflects the choice. |
| 3.6: Rewrite `from_bytecode` to dispatch on header bit 3 | ✅ | At `policy.rs:576-761`; mutually-exclusive branch populates exactly one of `decoded_shared_path` / `decoded_origin_paths`. |
| 3.6.5: Count-consistency check post-tree-walk | ✅ | At `policy.rs:743-758`; returns `Error::OriginPathsCountMismatch { expected, got }` on mismatch (direct test deferred to Phase 4). |
| 3.7: Add `EncodeOptions::origin_paths` Tier 0 override + `with_origin_paths` builder | ✅ | At `options.rs:70-84` (field) + `:171-174` (builder); `Default` updated; new test `encode_options_with_origin_paths_sets_field`. |
| 3.8: Run all tests; verify pass | ✅ | Verified by re-running: 701 ok / 1 failed (expected conformance gate). |
| 3.9: Commit Phase 3 | ✅ | Commit `6913c5e` with detailed message. |

All 9 new tests verified against their stated assertions:

- `round_trip_shared_path_byte_identical_to_v0_9` ✅ — verifies header `0x00` + tag `0x34`, `decoded_shared_path == Some` AND `decoded_origin_paths == None` after decode.
- `round_trip_divergent_paths_via_origin_paths_override` ✅ — verifies header `0x08` + tag `0x36`, decode→re-encode is byte-identical.
- `tier_0_origin_paths_override_wins_over_tier_1` ✅ — encodes divergent → decodes (populates Tier 1) → re-encodes with all-shared override → expects header `0x00` + tag `0x34` (proves Tier 0 cleared header bit 3 even though Tier 1 carries divergent paths). Test asserts BOTH conditions.
- `tier_3_shared_fallback_for_template_only_policy` ✅ — bare BIP 388 template emits SharedPath.
- `double_round_trip_origin_paths_byte_identical` ✅ — three rounds of encode → bytes1 == bytes2 == bytes3.
- `decoded_shared_path_and_decoded_origin_paths_mutually_exclusive_after_decode` ✅ — exactly one is `Some` per case.
- `from_bytecode_rejects_header_bit_3_set_with_shared_path_tag` ✅ — surfaces `UnexpectedTag { expected: 0x36, got: 0x34 }`.
- `from_bytecode_rejects_header_bit_3_clear_with_origin_paths_tag` ✅ — surfaces `UnexpectedTag { expected: 0x34, got: 0x36 }`.
- `encode_options_with_origin_paths_sets_field` ✅ (in `options.rs`).

Wire-byte sanity check (test outputs verified):
- Header `0x00` for shared paths (bit 3 clear): ✅ (verified by all v0.9-shape tests + new `round_trip_shared_path_byte_identical_to_v0_9`).
- Header `0x08` for divergent paths (bit 3 set): ✅ (verified by `round_trip_divergent_paths_via_origin_paths_override`).
- `windows(2).all(|w| w[0] == w[1])` correctly handles `count == 1` (vacuously true → SharedPath) and `count >= 2` divergence detection.

Dropped test `tier_1_decoded_wins_over_tier_2_kiv_walk`: the implementer's reasoning ("Tier 2 stubbed → test moot") is sound. Without a working Tier 2, there is no T1 vs T2 collision to verify. The Tier 0 vs Tier 1 collision is covered by `tier_0_origin_paths_override_wins_over_tier_1`. No coverage gap.

## Tier 2 stub assessment

**Verdict: ACCEPTABLE for v0.10.0 ship.**

1. **Is the rationale sound?** YES. Verified directly against `/scratch/code/shibboleth/rust-miniscript-fork/src/descriptor/wallet_policy/mod.rs`:
   - `WalletPolicy::template: Descriptor<KeyExpression>` is **private** (line 45).
   - `WalletPolicy::key_info: Vec<DescriptorPublicKey>` is **private** (line 47).
   - No public accessor for either field; the only escape hatches are `into_descriptor()` (which performs translation back to `Descriptor<DescriptorPublicKey>`, losing the `KeyExpression::index` field that maps AST-position → placeholder-index) and `set_key_info` (write-only).
   - `from_descriptor_unchecked` populates `key_info` via `descriptor.iter_pk().collect()` (line 132), which traverses in AST order. This means even a `to_descriptor → from_descriptor` round-trip would lock the order to AST, not placeholder-index.
   - For `sortedmulti(...)`, miniscript reorders keys lex-by-pubkey-bytes during normalization, so AST order ≠ placeholder-index order.
   The implementer did not miss an obvious accessor. Either upstreaming a fork API change (template walker or `key_info()` getter) or refactoring the codec's policy-construction layer (capture per-`@N` paths during ingest, store in a new struct field) is required for a correct walk. The implementer's "non-trivial" assessment is conservative and accurate.

2. **Is the stub correctly documented?** YES.
   - `try_extract_paths_from_kiv` has a 20-line rustdoc explaining (a) the intent, (b) the architectural ambiguity, (c) the FOLLOWUPS pointer, (d) the Tier 0/1 production-path workarounds available today.
   - `// TODO(v0.10-followup):` comment present at `policy.rs:511-512`.
   - FOLLOWUPS entry `v010-p3-tier-2-kiv-walk-deferred` exists at `design/FOLLOWUPS.md:717-759`, captures both design alternatives (template walk via private-field accessor request upstream OR per-`@N` path capture at `from_descriptor` ingestion), and explicitly documents the no-regression / no-progression behavior. Tier `v0.11`. Status `open`. Well-written.

3. **Is the behavior implication accurate?** YES. Read Tier 3 logic in `placeholder_paths_in_index_order` (line 487) and `resolve_shared_path_fallback` (line 522). For a freshly-parsed concrete-key descriptor with divergent `@N` paths:
   - Tier 0 (`opts.origin_paths`) is `None` for production callers.
   - Tier 1 (`decoded_origin_paths`) is `None` (not from a decode).
   - Tier 2 stub returns `Ok(None)`.
   - Tier 3 fires: `resolve_shared_path_fallback` walks its 4-step chain and returns ONE path (typically `shared_path()` of the first key), which is broadcast across all placeholders → all paths agree → `windows(2).all(eq)` → encoder emits `Tag::SharedPath`, header bit 3 = 0.
   This is **byte-identical to v0.9** for such inputs. Implementer's claim "v0.x ≤ 0.9 silent-flatten on freshly-parsed-from-string concrete-key descriptors with divergent `@N` paths is NOT regressed and NOT progressed in v0.10.0" is verified accurate.

4. **Is this acceptable for v0.10.0 ship?** YES.
   - The headline of v0.10 is per-`@N` origin path **encoding** (the wire format work). That ships.
   - The Tier 0 override path (test-vector generation) works → all conformance vectors / hand-AST tests for the new wire form will be generatable in Phase 4.
   - The Tier 1 round-trip path (decode→re-encode stability) works → any third-party tool emitting `Tag::OriginPaths` bytecode round-trips correctly through this codec.
   - What's lost is only the niche "freshly-parsed-from-string concrete-key descriptor with divergent paths automatically gets emitted as OriginPaths" path — a path that **doesn't currently work in any v0.x ≤ 0.9** either, so v0.10.0 is a strict superset. The plan explicitly authorized this stub ("If the KIV walk would land a non-trivial new helper (>30 lines), consider whether the simpler stub `Ok(None)` is acceptable for v0.10.0").
   - Pre-v1.0 break freedom (per `feedback_pre_v1_break_freedom`) further reduces the cost of revisiting in v0.11/v0.10.1.

The stub is **the right call** for v0.10.0; closing it via API redesign in v0.11 is the right disposition.

## Test coverage assessment

- **Round-trip stability:** Strong. Two flavors covered:
  - SharedPath (v0.9-shape): `round_trip_shared_path_byte_identical_to_v0_9` + all pre-existing v0.9-shape tests still passing → wire-format byte-stability for v0.9-shaped inputs is confirmed (no regression).
  - OriginPaths (v0.10 new): `round_trip_divergent_paths_via_origin_paths_override` (single round-trip) + `double_round_trip_origin_paths_byte_identical` (triple-encode for Tier 1 stability).
- **Tier precedence:** Two of the four collisions covered:
  - Tier 0 vs Tier 1: `tier_0_origin_paths_override_wins_over_tier_1` — non-trivial because the override is "all paths agree" while Tier 1 is "paths diverge", so the test asserts BOTH that header bit 3 was cleared (proving Tier 0 won) AND that the encoder picked SharedPath (proving the override paths were used, not the Tier 1 paths).
  - Tier 3 fallback: `tier_3_shared_fallback_for_template_only_policy` — verifies bare-template inputs land on SharedPath.
  - Tier 2 collisions: not testable while Tier 2 is stubbed (correctly elided).
  - Tier 0 vs Tier 3 (override beats fallback): not directly tested but trivially follows from the early-return logic in `placeholder_paths_in_index_order`.
- **Conflicting-path-decl rejection:** Both directions covered:
  - `from_bytecode_rejects_header_bit_3_set_with_shared_path_tag` — flips header `0x00 → 0x08` on encoded bytes; expects `UnexpectedTag { expected: 0x36, got: 0x34 }` (from the new OriginPaths-arm tag check).
  - `from_bytecode_rejects_header_bit_3_clear_with_origin_paths_tag` — flips header `0x08 → 0x00` on encoded bytes; expects `UnexpectedTag { expected: 0x34, got: 0x36 }` (from the existing `decode_declaration` validation, which already catches this).
- **Mutual exclusion invariant:** Tested AND structurally guaranteed (see Finding 6).
- **EncodeOptions builder:** `encode_options_with_origin_paths_sets_field` covers field set + non-mutation of other defaults.
- **Coverage gaps acknowledged:**
  - Direct unit test for `OriginPathsCountMismatch` is absent (Phase 4 owns this via conformance vector — see Finding 2). The conformance gate ensures the rejection vector ships before v0.10.0 release.
  - Zero-placeholder synthetic policy edge case (Finding 5): not tested, but `from_str` upstream rejects.

## Architectural notes

The implementer's report flags 5 architectural surprises vs the plan sketch. All 5 are reasonable (and unsurprising for a codebase this mature). The most notable:

1. **Existing `to_bytecode` shared-path tier chain was already a 4-step Phase B chain, not a single shared-path step.** The plan's Tier 3 sketch was simplified; the implementer correctly extracted the existing inline chain into a helper rather than rewriting it. This is the right call (preserves byte-for-byte compatibility for v0.9 inputs).

2. **`decode_declaration` already validates the SharedPath tag and emits `UnexpectedTag { expected: 0x34, got: <other> }` for any other defined tag.** The reject-test for `header bit 3 clear + 0x36 tag` is therefore handled by the *existing* `decode_declaration` logic, not a new Phase-3 code path. The test still passes; the implementer correctly noted this in the report.

3. **No public `key_info()` accessor on the fork's `WalletPolicy`.** This is the load-bearing constraint that drives the Tier 2 stub. Verified directly against the fork source.

These are surprises only relative to the plan's pre-implementation sketch — none represent post-impl deviations from spec or plan intent. The implementation is faithful.

## Recommended action

**Proceed to Phase 4 without re-review.**

### Required edits

None.

### Optional edits (non-blocking)

- Finding 1 — stale `// 0x33` annotation in `policy.rs:1098, 1455` is **already** covered by the existing FOLLOWUPS entry `tag-sharedpath-rustdoc-stale-0x33`. No Phase-3 action; sweep when that broader entry is acted on.

### Phase 4 entry conditions

After this review lands, Phase 4 may proceed. Phase 4 should:

- Add the `rejects_origin_paths_count_mismatch` conformance vector (closes the absent direct test for Finding 2 + closes the conformance-gate failure currently expected at the workspace test count).
- Add a `rejects_path_component_count_exceeded` conformance vector if not already covered (the `error_coverage.rs:66` mirror lists this variant).
- The Phase 4 plan in `IMPLEMENTATION_PLAN_v0_10_per_at_N_paths.md` already covers both per the prompt; verify before kickoff.
- Carry forward the test-count-reporting hygiene note from Phase 1/2 (the implementer's 701/1 count is correct for this round, deterministically computed via `awk` per the report).

### FOLLOWUPS entries

- `v010-p3-tier-2-kiv-walk-deferred` — already filed by implementer; verified well-written and complete. No duplication needed.
- No new entries from this review. The stale `// 0x33` comments noted in Finding 1 are already in scope of the existing `tag-sharedpath-rustdoc-stale-0x33` entry.
