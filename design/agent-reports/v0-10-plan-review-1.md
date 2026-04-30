# v0.10 plan review (opus, pass 1)

**Date:** 2026-04-29
**Plan:** design/IMPLEMENTATION_PLAN_v0_10_per_at_N_paths.md (commit 2a9c969)
**Reviewer:** opus-4.7

## Summary

**Verdict: needs-fixes-then-proceed.** The plan is structurally sound — the seven-phase decomposition (foundation types → bytecode helpers → policy-layer integration → corpus → fingerprint API → docs → release) is the right ordering, the per-phase opus-review gates are in place, and the TDD step pattern is consistent with v0.7 / v0.9 precedent. Spec coverage of §1–§6 plus appendix is essentially complete, with only minor inline-prose-vs-back-reference questions in Phase 6.

However, eight findings should be addressed before implementation begins. Three are **blockers** (existing tests deliberately written against v0.9 behavior that the plan misses; a missing `ErrorVariantName` enum update that will silently break the Phase 1 conformance-gate; a critical wire-format example divergence between spec Example B and corpus o1). Five more are **strong** (a pure-helper test can avoid infrastructure ambiguity; round-trip stability needs broader coverage; Phase 7 sibling-repo coordination is too thin compared to v0.9's pattern; etc.). The remaining are nice-to-have polish.

After fixes, proceed to phased implementation with the existing per-phase reviewer gates.

## Findings

### F1: Existing header tests `reserved_bit_3_set` and `all_reserved_bits_set_no_fingerprints` will fail after Phase 1.3
**Severity:** blocker
**Location:** Phase 1, Step 1.3 (header.rs implementation) and Step 1.5 (call-site sweep)
**Issue:** `crates/md-codec/src/bytecode/header.rs` already contains two tests that explicitly assert `0x08` and `0x0B` are reserved-bit-set rejections (`reserved_bit_3_set` at line 194, `all_reserved_bits_set_no_fingerprints` at line 209). The plan only adds new `header_byte_0x08_decodes_with_origin_paths_flag_set` etc. tests; it does not delete or rewrite the existing tests. After Step 1.3 lands `RESERVED_MASK = 0x03` and the `origin_paths` flag, both existing tests fail because:
- `reserved_bit_3_set` expects `from_byte(0x08) → Err`, but `0x08` will now be valid.
- `all_reserved_bits_set_no_fingerprints` tests `0x0B` (bits 3+1+0); under the new mask `0x03`, bit 3 is no longer reserved, so the byte still has bits 1+0 set and would still error — but the comment "bits 3, 1, 0 all set" is now stale. The test would coincidentally pass but with misleading prose.
- The `RESERVED_MASK` import at line 170 in the test module may also need an update.

**Recommendation:** Add an explicit step between 1.3 and 1.4 to:
1. **Delete** `reserved_bit_3_set` (it's the inversion of the new behavior).
2. **Rewrite** `all_reserved_bits_set_no_fingerprints` to test the new `0x03` mask boundary — e.g. exercise `0x03` (bits 1+0 set, no flags) or rename to `all_reserved_bits_set_in_v0_10` with `byte = 0x03` and the same matching assertion.
3. Update the `RESERVED_MASK` constant binding in test imports if needed.

This is also a TDD-discipline finding: the plan's Step 1.2 ("verify they fail") asserts only the *new* tests fail. After 1.3, two *existing* tests will also fail — those existing-test failures should be predicted in 1.2 and explicitly resolved in 1.3 (not surface unexpectedly).

### F2: `ErrorVariantName` enum in `tests/error_coverage.rs` not updated; conformance gate will pass spuriously
**Severity:** blocker
**Location:** Phase 1, Step 1.8 (error variants) and Phase 4, Step 4.4 (conformance tests)
**Issue:** `crates/md-codec/tests/error_coverage.rs` line 37–65 maintains a hand-mirrored `ErrorVariantName` enum that drives the `every_error_variant_has_a_rejects_test_in_conformance` check. The plan references this gate in Step 1.10 ("Phase 5 will add: `rejects_origin_paths_count_too_large`, `rejects_origin_paths_count_mismatch`, `rejects_path_component_count_exceeded`") and Phase 4 ships those tests. But the plan never says to extend `ErrorVariantName` itself with the new entries (`OriginPathsCountMismatch`, `PathComponentCountExceeded`).

Without the enum extension:
- After Phase 1.8 + Phase 4.4, the conformance gate test passes — but only because it checks variants in the hand-written enum, not the actual `Error` enum. New variants are invisible to the gate.
- The "every variant has a rejects test" guarantee is silently broken.

Note: `BytecodeErrorKind::OriginPathsCountTooLarge` is fine — `error_coverage.rs` only mirrors the top-level `Error` enum, and its `InvalidBytecode` entry covers all `BytecodeErrorKind` sub-variants via the `INVALID_BYTECODE_PREFIX`.

**Recommendation:** In Phase 1, Step 1.8, add an explicit substep:
> Update `crates/md-codec/tests/error_coverage.rs::ErrorVariantName` enum: add `OriginPathsCountMismatch`, `PathComponentCountExceeded`. Per the file's own header comment (line 6–22), the enum is hand-mirrored and adding a new `Error` variant requires extending it.

The plan's Step 1.10 then becomes accurate: the gate "fails because we added 3 new variants without conformance test coverage" — but without F2's fix, the gate would not fail at all, and Phase 5 would land vacuous coverage.

### F3: Phase 4 `o1` corpus vector path layout differs from spec Example B; lose mutual validation
**Severity:** blocker (per task review item #6)
**Location:** Phase 4, Step 4.1 (build_v0_10_origin_paths_vectors)
**Issue:** Spec §2 Example B is a 3-cosigner divergent-path multisig with paths `{m/48'/0'/0'/2', m/48'/0'/0'/2', m/48'/0'/0'/100'}` and explicit-form bytes `FE 04 61 01 01 C9 01` (header `0x0C` because fingerprints are also present). The OriginPaths block bytes are pinned to `36 03 05 05 FE 04 61 01 01 C9 01`.

The plan's `o1` vector uses the *same* `{mainnet, mainnet, custom}` paths but with `fps: None` (matching spec Example C, the no-fingerprints variant — header `0x08`). The plan's `o2` vector uses these paths *with* fingerprints, matching spec Example B (header `0x0C`).

This near-miss is fine on its own — but the spec uses *example-specific* test values for fingerprints (`deadbeef`, `cafebabe`, `d00df00d`) that the plan correctly mirrors in `o2`'s fingerprint synthesis. Cross-verification works.

**However:** the plan does not state that `o1` / `o2` are *intended to mirror spec Examples B/C byte-for-byte*. If a future implementer rewrites the spec example values without updating the corpus (or vice versa), the spec-vs-corpus invariant degrades silently.

**Recommendation:** In Step 4.1, add a doc comment to `build_v0_10_origin_paths_vectors`:
```rust
// Vectors o1 and o2 mirror SPEC §2 Example C (header 0x08, no fps) and
// Example B (header 0x0C, fps {deadbeef, cafebabe, d00df00d}) respectively.
// The OriginPaths block bytes 36 03 05 05 FE 04 61 01 01 C9 01 are
// pinned in the spec; corpus regen MUST produce these same bytes. If
// the spec example values change, both must update in lockstep.
```
Add an inline test in Phase 4 Step 4.1 that asserts `o2`'s `expected_bytecode_hex` *contains* the substring `36030505fe046101 01c901` (spec Example B's OriginPaths bytes). This makes the spec-corpus correspondence checked automatically rather than by hand.

### F4: Tag-byte unallocated tests are at TWO locations in tag.rs, not one
**Severity:** strong
**Location:** Phase 1, Step 1.6 (final paragraph — "Also update `tag_v0_6_high_bytes_unallocated` test")
**Issue:** `crates/md-codec/src/bytecode/tag.rs` has TWO tests that loop `0x36..=0xFF`:
- `tag_v0_6_high_bytes_unallocated` (line 294)
- `tag_rejects_unknown_bytes` (line 326, second loop in the same fn)

The plan only mentions the first. The second would silently fail when `0x36 → Some(Tag::OriginPaths)`.

Additionally, `tag_round_trip_all_defined` (line 305) builds `let v0_6_allocated: Vec<u8> = (0x00..=0x23).chain(0x33..=0x35).collect();` — this needs `0x36` added.

**Recommendation:** Phase 1, Step 1.6 should explicitly call out all three tests for adjustment:
- `tag_v0_6_high_bytes_unallocated` (line 294): loop `0x37..=0xFF`.
- `tag_rejects_unknown_bytes` (line 317): second loop `0x37..=0xFF`.
- `tag_round_trip_all_defined` (line 305): extend to `(0x00..=0x23).chain(0x33..=0x36)`.

Also rename `tag_v0_6_*` → `tag_v0_10_*` for the affected tests where the test name pins a specific version, OR keep names for traceability and just update the loop bounds. Rename is closer to project convention (the v0.6 test names date to that version's tag-renumber).

### F5: Phase 2 test `decode_path_rejects_11_components_in_explicit_form` synthesizes incorrect bytes
**Severity:** strong
**Location:** Phase 2, Step 2.1
**Issue:** The test synthesizes `let mut bytes = vec![0xFE, 0x0B];` and pushes 11 `0x01`s. The interpretation:
- `0xFE` indicator → explicit form
- `0x0B` LEB128 → count = 11
- 11 components, each `0x01` (LEB128 1, decoded via `n = 1`, `n & 1 == 1` → hardened, index `1 >> 1 = 0`) → all `m/0'`.

That's correct in principle. But after Step 2.3 lands the cap, `decode_path` should reject *before* component decoding (it knows the count is 11). The test synthesizes 11 actual component bytes — defense-in-depth — so it works either way. Note the cursor enters via `Cursor::new(&bytes)` and the `0xFE` is consumed first. OK.

**However:** the cap check belongs at the count-read site, before allocating the `components` Vec. The plan's encode_path/decode_path sketches in Step 2.3 are correct (`if count > MAX_PATH_COMPONENTS as u64 { ... }`) but the test should still exercise the bound at exactly 11.

**Recommendation:** Add an additional test variant that synthesizes only `[0xFE, 0x0B]` (count = 11, no components after) and verifies the cap check fires *before* `UnexpectedEnd` from missing components:
```rust
#[test]
fn decode_path_cap_check_fires_before_component_decode() {
    // count=11 with no component bytes — cap rejection MUST surface, not UnexpectedEnd.
    let bytes = vec![0xFE, 0x0B];
    let mut cur = Cursor::new(&bytes);
    let err = decode_path(&mut cur).unwrap_err();
    assert!(matches!(err, Error::PathComponentCountExceeded { got: 11, max: 10 }));
}
```
This pins error-priority ordering and prevents a future refactor from re-ordering the checks.

### F6: `encode_path` signature change from `Vec<u8>` to `Result<Vec<u8>, Error>` is a public-API break that the plan understates
**Severity:** strong
**Location:** Phase 2, Step 2.3 (parenthetical "if it was infallible `-> Vec<u8>`, this becomes a signature change — update call sites")
**Issue:** `encode_path` is currently `pub fn encode_path(path: &DerivationPath) -> Vec<u8>` (path.rs line 65). Making it `Result`-returning is a public-API break — every consumer (including `encode_declaration` at line 178, plus 6+ test sites in the same file) needs `?` or `unwrap()` propagation.

This is also a **migration table omission**: the v0.9 → v0.10 MIGRATION.md table (Phase 6, Step 6.9) only lists `BytecodeHeader::new_v0` as the forced edit. Adding `encode_path` as a now-fallible function is a second forced edit, omitted from MIGRATION.

**Recommendation:** Either:
- **(A)** keep `encode_path` infallible by validating the cap *only* in `encode_declaration` and the new `encode_origin_paths` (the public-facing wrappers). The unwrapped `encode_path` then becomes a low-level helper that callers must guard. This is brittle.
- **(B)** explicitly bump `encode_path` to `Result<Vec<u8>, Error>` as Phase 2 plans, and ALSO:
  - Add to MIGRATION.md (Phase 6, Step 6.9): `encode_path` signature changes; update call sites with `?` or `.expect("validated upstream")`.
  - Update Phase 2 Step 2.10 commit message to enumerate the public-API break explicitly.

Recommend (B) — symmetric with `decode_path`'s already-fallible signature, simpler invariants, fewer hidden footguns. But the plan must own the break.

### F7: Phase 7 sibling-repo coordination is too thin compared to v0.9.0 release pattern
**Severity:** strong
**Location:** Phase 7, Step 7.11 (mk1 cross-update)
**Issue:** v0.9.0's release plan had explicit Phase 0 (open coordinated mk1 draft PR before any md1 changes) and Step 10 (update sibling FOLLOWUPS in a separate sibling-repo commit on its own branch). v0.10's plan reduces this to one bullet at Phase 7, Step 7.11: "Light cross-update may be a no-op or a one-line note. Audit `/scratch/code/shibboleth/mnemonic-key/` for any forward-references that resolve."

Specifically missing from Phase 7:
1. **No mk1 BIP coordination check.** Spec §6 says "mk1 BIP §"Authority precedence" prose stays unchanged across v0.10" — but md1's new BIP §"Per-`@N` path declaration" + §"Authority precedence with MK" cross-references mk1's BIP. If mk1's BIP changes between brainstorm-time and v0.10 ship, the cross-reference breaks. Phase 7 should `gh pr` or `gh api` check for any mk1 BIP edits.
2. **No companion-FOLLOWUPS update step.** The mk1-side `md-per-N-path-tag-allocation` companion entry (cited in the FOLLOWUPS entry at line 99 of design/FOLLOWUPS.md) needs `Status: resolved by md-codec-v0.10.0 (commit ...)`. v0.9's plan did this in Step 10 as a separate sibling-repo commit; v0.10's plan does not.
3. **No CLAUDE.md crosspointer maintenance trigger.** RELEASE_PROCESS.md §"CLAUDE.md crosspointer maintenance" requires updating both CLAUDE.md and the mk1 companion. v0.10's plan addresses CLAUDE.md (Step 7.4) but not the mk1-side update.

**Recommendation:** Expand Phase 7, Step 7.11 to:
```
- [ ] **Step 7.11: Cross-update sibling mnemonic-key repo**

Per design/RELEASE_PROCESS.md §"CLAUDE.md crosspointer maintenance":

a. cd /scratch/code/shibboleth/mnemonic-key
b. Check for any forward-reference text that becomes resolvable post-v0.10
   (search: `git grep -i 'md-per-at-N\|0x36\|OriginPaths\|md1.*path-tag'`).
   If hits, update them to point at v0.10.0 release-tag prose.
c. Update mk1's design/FOLLOWUPS.md companion entry (`md-per-N-path-tag-allocation`):
   Status: resolved by md-codec-v0.10.0 (commit <md1-merge-sha>).
d. Open a small mk1 PR for these updates; cross-link to md1's v0.10 PR.
e. Audit mk1 BIP for any post-brainstorm edits that would break md1's
   §"Authority precedence with MK" cross-reference. If hits, either
   update md1's reference prose OR push back to user.
```

This brings Phase 7 in line with v0.9.0's discipline.

### F8: Tier-0 / Tier-1 / Tier-2 precedence verification has uneven test coverage
**Severity:** strong
**Location:** Phase 3, Steps 3.6–3.7 (round-trip tests)
**Issue:** Per task review item #4, the spec's 4-tier precedence chain (`opts.origin_paths` → `decoded_origin_paths` → KIV walk → shared fallback) needs explicit precedence-collision tests. The plan covers:
- Tier 0 winning (Step 3.6 `round_trip_divergent_paths_via_origin_paths_override`).
- Tier 1 wins (implicit in `round_trip_divergent_paths_via_origin_paths_override`'s decode-then-encode subtest).
- Tier 2 (KIV walk): NOT tested explicitly. The plan adds `try_extract_paths_from_kiv` as a new helper but no test exercises it.
- Tier 3 (shared fallback): tested via `round_trip_shared_path_byte_identical_to_v0_9`.

**Critical missing tests:**
1. **Tier 0 overrides Tier 1.** Decode an OriginPaths-bearing bytecode (populating `decoded_origin_paths`), then re-encode with `opts.origin_paths = Some(different paths)`. Expect bytecode reflects opts override, not decoded.
2. **Tier 1 wins over Tier 2.** Decode an OriginPaths bytecode (populating Tier 1), then re-encode without opts override (Tier 1 should win, even if KIV walk would yield different paths).
3. **Tier 2 fires for full-descriptor parses.** Construct a `WalletPolicy` from a full descriptor string (via `FromStr`) with divergent origins per cosigner. Expect `to_bytecode` walks the KIV and emits OriginPaths.

**Recommendation:** Add three tests to Phase 3, Step 3.6, after `round_trip_divergent_paths_via_origin_paths_override`:
```rust
#[test]
fn tier_0_overrides_tier_1() { ... }
#[test]
fn tier_1_wins_over_tier_2() { ... }
#[test]
fn tier_2_kiv_walk_extracts_divergent_paths() { ... }
```

Note: Tier 2 test depends on the descriptor parser being able to produce a `WalletPolicy` with per-key origin paths. If `WalletPolicy::FromStr` only accepts BIP 388 templates (which carry `@N` placeholders, not concrete keys), Tier 2 may need a different constructor. Plan implementer should investigate during P3 — the plan's `try_extract_paths_from_kiv` returning `None` for "no KIV present" suggests Tier 2 is feasible, but the test path needs validation.

### F9: Round-trip stability test in Phase 3 is too thin (one happy-path test)
**Severity:** strong
**Location:** Phase 3, Step 3.6
**Issue:** Per task review item #4, the plan tests round-trip stability with a single test (`round_trip_divergent_paths_via_origin_paths_override`). For a wire-format-breaking release with a new field (`decoded_origin_paths`), one test is thin.

**Specific gaps:**
1. No double-round-trip test (`encode → decode → encode → decode → encode` is byte-stable, not just the first encode). This catches bugs where Tier 1 ↔ Tier 0 priority is asymmetric.
2. No test confirming that `WalletPolicy::PartialEq` invariant from existing decoded_shared_path docs (line 190–196 of policy.rs) extends correctly to the new field. Two policies from `parse()` (Tier-0/1 None) vs `from_bytecode` (Tier 1 Some) should still compare unequal even though byte-equivalent.
3. No test that the `decoded_shared_path` / `decoded_origin_paths` mutual-exclusion invariant (spec §4) is enforced — a defensively-misconstructed `WalletPolicy` with both fields set should be unreachable in practice but the invariant should be documented and assertable.

**Recommendation:** Add to Phase 3, Step 3.6:
```rust
#[test]
fn double_round_trip_origin_paths() {
    let bytes1 = ...;     // synthesize divergent OriginPaths bytecode
    let p1 = WalletPolicy::from_bytecode(&bytes1).unwrap();
    let bytes2 = p1.to_bytecode(&EncodeOptions::default()).unwrap();
    let p2 = WalletPolicy::from_bytecode(&bytes2).unwrap();
    let bytes3 = p2.to_bytecode(&EncodeOptions::default()).unwrap();
    assert_eq!(bytes1, bytes2, "first round-trip");
    assert_eq!(bytes2, bytes3, "second round-trip — Tier 1 stability");
    assert_eq!(p1, p2);
}

#[test]
fn decoded_shared_path_and_decoded_origin_paths_mutually_exclusive() {
    // After from_bytecode, exactly one of the two fields is Some.
    let shared_bytes = vec![0x00, 0x34, 0x05];   // SharedPath
    let p_shared = WalletPolicy::from_bytecode(&shared_bytes).unwrap();
    assert!(p_shared.decoded_shared_path.is_some() ^ p_shared.decoded_origin_paths.is_some(),
            "exactly one of the two decoded-path fields must be Some after from_bytecode");
    // Same check for an OriginPaths-bearing bytecode...
}
```

### F10: Phase 6 BIP prose is back-referenced rather than inlined; reference brittleness risk
**Severity:** nice-to-have
**Location:** Phase 6, Steps 6.1, 6.2, 6.4
**Issue:** Per task review item #7, the plan says `[... full prose per spec §2-§3 ...]` and `Per spec §6 prose for Type 0 / Type 1 typology`. This is fine when the spec is treated as authoritative source of truth — but the BIP draft IS user-facing prose, not a derivative of the spec. Spec §6 has substantial prose (lines 519–569 of SPEC_v0_10) including the exact ":Type 1 — `PolicyId`" framing, the "MAY engrave" softening, and the three-sentence mk1 cross-reference.

**Risk:** an implementer reading only the plan won't know whether to copy spec prose verbatim or paraphrase. The spec prose is already polished after 3 review passes; verbatim copy is the safer move.

**Recommendation:** In Phase 6 Steps 6.1–6.5, replace `[... full prose per spec ...]` with explicit "copy verbatim from SPEC_v0_10 §6 prose blocks" instructions. Specifically:
- Step 6.2 (PolicyId types): "Copy the three-paragraph prose at SPEC_v0_10 §6 lines 524–529 verbatim as the BIP §"PolicyId types" subsection body, then localize URL references."
- Step 6.3 (engraving softening): "Copy the two-paragraph prose at SPEC_v0_10 §6 lines 535–539."
- Step 6.4 (Authority precedence with MK): "Copy the four-line block at SPEC_v0_10 §6 lines 545–547."

This nails down the source-of-truth ambiguity and reduces the per-phase reviewer's surface area.

### F11: Phase 5 conformance gate gap — `PolicyId::fingerprint` is pure-additive but no test for fingerprint stability across decode→encode
**Severity:** nice-to-have
**Location:** Phase 5, Step 5.2
**Issue:** Phase 5 tests `policy_id_fingerprint_is_first_4_bytes` and `policy_id_fingerprint_deterministic_from_policy`. Both use a hand-constructed PolicyId or a same-policy doubly-computed PolicyId. Missing: a test confirming that `decode → re-encode → recompute fingerprint` yields the same fingerprint as the original. This is implied by the canonical-bytecode invariant but worth pinning explicitly given the v0.10 wire-format break.

**Recommendation:** Add to Phase 5, Step 5.2:
```rust
#[test]
fn policy_id_fingerprint_stable_across_round_trip() {
    let p1: WalletPolicy = "wsh(sortedmulti(2, @0/**, @1/**, @2/**))".parse().unwrap();
    let bytes = p1.to_bytecode(&EncodeOptions::default()).unwrap();
    let p2 = WalletPolicy::from_bytecode(&bytes).unwrap();
    let id1 = compute_policy_id_for_policy(&p1).unwrap();
    let id2 = compute_policy_id_for_policy(&p2).unwrap();
    assert_eq!(id1.fingerprint(), id2.fingerprint(),
               "fingerprint must survive decode→re-encode→re-id");
}
```
For a divergent-path policy, this would also verify that v0.10's PolicyId encoding (per Q7 Route X) is deterministic.

### F12: Phase 7 step 7.1 audit confirmed: md-signer-compat has zero hits (no version bump needed)
**Severity:** confirmation
**Location:** Phase 7, Step 7.1
**Issue:** Per task review item #9: ran `rg 'BytecodeHeader|Tag::|MAX_PATH_COMPONENTS|OriginPaths|policy_id_seed|encode_path|decode_path|new_v0' crates/md-signer-compat/`. **Zero hits.** md-signer-compat has no public-API surface that touches the renamed/changed symbols.

**Recommendation:** Plan's assumption is correct — md-signer-compat stays at current version (0.1.1). However, per RELEASE_PROCESS.md the workspace `Cargo.toml` lockstep convention is that any md-codec minor bump implies regenerating dependents. Verify Phase 7 Step 7.5's `cargo build --workspace` includes md-signer-compat — it does (since the workspace covers it). Confirmed.

### F13: Phase 4 step 4.2 negative vector enumeration mismatches Phase 1 step 1.10 list
**Severity:** nice-to-have
**Location:** Phase 1, Step 1.10 vs. Phase 4, Step 4.2
**Issue:** Phase 1 Step 1.10 says "Phase 5 will add: `rejects_origin_paths_count_too_large`, `rejects_origin_paths_count_mismatch`, `rejects_path_component_count_exceeded`" — three tests. Phase 4 Step 4.2 lists six negative vectors plus Step 4.4 lists three rejects_*. The negative vector `n_path_components_too_long` and `n_conflicting_path_declarations_bit_set_tag_shared` are present in Step 4.2 but the plan is unclear whether they map to the rejects_* tests (some negative vectors share rejects_* tests in v0.x precedent).

**Recommendation:** In Phase 4 Step 4.2 add a mapping comment:
```
- n_orig_paths_count_zero      → rejects_origin_paths_count_too_large
- n_orig_paths_count_too_large → rejects_origin_paths_count_too_large
- n_orig_paths_truncated       → no dedicated rejects_*; covered by inline test in Phase 2
- n_orig_paths_count_mismatch  → rejects_origin_paths_count_mismatch
- n_path_components_too_long   → rejects_path_component_count_exceeded
- n_conflicting_path_declarations_bit_set_tag_shared → no dedicated rejects_*; covered by Phase 3 from_bytecode_rejects_* tests
```

Also: `n_orig_paths_count_zero` and `n_orig_paths_count_too_large` both surface `BytecodeErrorKind::OriginPathsCountTooLarge` (the variant covers both bounds per spec F4). The negative-vector schema's `expected_error_variant` field (line 125 of vectors.rs) is a String — both vectors should set it to `"InvalidBytecode"` (the wrapping top-level variant), not `OriginPathsCountTooLarge`. The plan doesn't make this explicit; an implementer might confuse the two.

### F14: Pre-Phase-0 baseline test count is vague ("678 tests pass or whatever")
**Severity:** nice-to-have
**Location:** Pre-Phase-0, Step 3
**Issue:** The pre-phase test-count baseline is `# Expect: 678 tests pass (or whatever the v0.9.1 baseline is)`. This is unactionable — an implementer who sees 700 tests pass might second-guess whether something already broke. Worse, if a test was lost between v0.9.1 ship and v0.10 work, the baseline drifts silently.

**Recommendation:** Pin the actual number by running `cargo test --workspace --all-features` against `main` once before plan-finalize. Replace the comment with `# Expect: 720 tests pass (verified against main commit 2a9c969 on 2026-04-29).`

### F15: Cargo.toml family-token roll for vectors regen needs version-bump-first ordering
**Severity:** nice-to-have
**Location:** Phase 4, Step 4.6 (regen) vs. Phase 7, Step 7.2 (version bump)
**Issue:** `GENERATOR_FAMILY` at vectors.rs line 818 reads `concat!("md-codec ", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"))`. The family token is computed from `Cargo.toml` at build time. If Phase 4 regenerates vectors *before* Phase 7's version bump, the regen produces `"md-codec 0.9"` strings, not `"md-codec 0.10"` — and Phase 7's regen-after-version-bump would produce yet another corpus.

**Recommendation:** Move the version bump (Step 7.2) earlier — into Pre-Phase-0 or the start of Phase 4 — so that all vector regens during the plan occur with the correct family token. v0.9.0's plan handled this by having the version bump in Phase 0 (the rename phase, before any vector-touching). v0.10's plan should do the same:

```
- [ ] **Pre-Phase-0, Step 4 (NEW): Bump version to 0.10.0**

Update crates/md-codec/Cargo.toml: 0.9.1 → 0.10.0.
This ensures GENERATOR_FAMILY = "md-codec 0.10" for all subsequent
vector regenerations during this plan. The version bump is committed
as part of Pre-Phase-0 so it lives at the foundation of the feature
branch; final release commit (Step 7.6) doesn't re-bump.
```

Then drop Step 7.2 or rename it "Confirm version is 0.10.0".

## Confirmations

- **Spec coverage of §1–§6 + appendix is complete.** Spot-checked:
  - §1 Decision matrix Q1–Q13 all addressed in plan phases.
  - §2 Wire Format → Phase 1 (header) + Phase 2 (path encoding) + Phase 3 (dispatch).
  - §3 Decoder Design → Phase 1 (header parse) + Phase 2 (decode_origin_paths) + Phase 3 (from_bytecode).
  - §4 Encoder Design + Type/Error Updates → Phase 1 (errors) + Phase 3 (precedence chain + dispatch).
  - §5 Test Corpus → Phase 4 (positive + negative + hand-AST + cursor sentinel).
  - §6 Migration + Release Framing → Phase 6 (docs) + Phase 7 (release).
  - Appendix A open implementer questions → annotated under "Open implementer questions" at plan tail.

- **Phase ordering is correct.** Phase 1 (foundation types isolated from policy layer) → Phase 2 (bytecode helpers as standalone fns) → Phase 3 (policy-layer wiring) is a clean dependency progression. The user's concern about Phase 3 breaking Phase 1 baseline is addressed: Phase 1 Step 1.5 sweeps existing call sites and Phase 1 Step 1.9 confirms a clean compile before commit.

- **TDD discipline is consistent across phases.** Step N.1 (write failing tests) → N.2 (verify failure) → N.3 (implement) → N.4 (verify pass) appears in Phases 1, 2, 3, 5. Phases 4 (corpus) and 6 (docs) are TDD-light by their nature — Phase 4's test-driven posture is to pin byte-literals via Phase 1/2/3 hand-AST tests. This matches v0.9.0 plan precedent.

- **md-signer-compat is unaffected** (per F12 confirmation).

- **Commit messages are accurate, complete, consistent with project's git log convention** (spot-checked Phase 1 and Phase 3 messages — both follow `refactor(v0.10-pN): ...` / `feat(v0.10-pN): ...` style with multi-paragraph rationale and `Co-Authored-By:` trailer per CLAUDE.md and prior release commits).

- **Per-phase opus-review gates are in place** (Step N.12 / N.11 / N.10 etc.) with persistent reports at `design/agent-reports/v0-10-phase-N-review.md`.

- **mk1-cross-format tests are correctly out of scope** (per task review item #4) — mk1 has its own crate at `/scratch/code/shibboleth/mnemonic-key/`. Phase 7 Step 7.11 covers cross-update lightly (but per F7, needs strengthening).

## Open questions for the implementer

1. **Tier 2 (KIV walk) feasibility for `WalletPolicy::FromStr` outputs.** The current `WalletPolicy::FromStr` parses BIP 388 templates that use `@N` placeholders (no concrete origins). Tier 2's "walk key information vector" only fires when the policy has concrete-key descriptor data. The plan's `try_extract_paths_from_kiv` returning `None` for template-only policies is the correct fallback, but the implementer should verify: does a `parse("wsh(pk(...))")` from a full-descriptor string (with `[fp/m/48'/0'/0'/2']xpub...`) produce a `WalletPolicy` with KIV data? If not, Tier 2 is dead code in practice and Phase 3 step 3.3 simplifies. If yes, the test in F8 recommendation is reachable.

2. **`encode_path` signature: fallible or infallible?** Per F6 — recommend fallible (option B). Confirm before P2 lands.

3. **Order of `o1`/`o2`/`o3` in the corpus.** Plan tail says "place after the v0.9 T1 vector." The `build_v0_9_testnet_p2sh_p2wsh_vector` in vectors.rs at line 871 is currently the last positive vector before `out` returns. Confirm placement is post-T1 and pre-return.

4. **Conformance test substring matching for negative vectors.** As noted in F13, the negative vector schema's `expected_error_variant` is a String. For `n_orig_paths_count_zero` and `n_orig_paths_count_too_large` (both surfacing `BytecodeErrorKind::OriginPathsCountTooLarge` under top-level `Error::InvalidBytecode`), the value should be `"InvalidBytecode"` — not the inner kind. Confirm the v0.x convention and pin in plan.

5. **Spec version "Wire format break" CHANGELOG framing — does it conflict with v0.6's "wire-format break" framing?** v0.6 was a wire-format break at the policy layer (Layer 3 strip); v0.10 is a wire-format break at the bytecode header layer. Same magnitude, different surface. CHANGELOG framing should clarify "v0.10 is the second wire-format break in the v0.x series" or similar, not just "Why a wire-format break?" in isolation.

---

## Subtle / freelance findings

- **`shared_path` field in `EncodeOptions` overlaps with `origin_paths`.** Currently `EncodeOptions::shared_path: Option<DerivationPath>` is the v0.4+ "Tier 0 override for shared-path" mechanism. With Phase 3 adding `EncodeOptions::origin_paths: Option<Vec<DerivationPath>>` as Tier 0 for OriginPaths, what happens if both are `Some`? Spec §4 implies origin_paths takes precedence (it's listed as Tier 0 in the per-`@N` chain), but the spec is silent on the bystander behavior of `shared_path` when origin_paths is also set. Recommend adding a `debug_assert!(opts.shared_path.is_none() || opts.origin_paths.is_none())` in `placeholder_paths_in_index_order` and documenting the ambiguity rejection in the field rustdoc.

- **`WalletPolicy::PartialEq` semantics expand across the new field.** Phase 3 adds `decoded_origin_paths: Option<Vec<DerivationPath>>`. Two policies that compare unequal under `PartialEq` (one from `parse()`, one from `from_bytecode`) is the v0.x precedent. The new field doubles the surface — no logic changes needed but the existing rustdoc on `decoded_shared_path` (lines 190–196 of policy.rs) should be amended to mention `decoded_origin_paths` parallel-equality.

- **Vector schema-1 regen check.** Plan's Phase 4 Step 4.6 regenerates BOTH schemas. Per RELEASE_PROCESS.md §"Wire-format SHA pin", schema-1's "byte-identical regen across patch versions" invariant is load-bearing for downstream consumers. Schema-1 regenerates under family-token-roll on minor bumps, which matches plan expectations. But the plan's negative vectors are schema-2-only; schema-1 should regen byte-identically modulo family token. Verify post-regen that `expected_error_variant` strings in schema-1 are unchanged (none of the new error variants are filed there).

- **Plan does NOT include opus-review gate after Pre-Phase-0.** Pre-Phase-0 is purely setup (branch cut, baseline test). No reviewer gate is needed there — confirmed acceptable.

- **Plan does NOT mention bumping `walletinstanceid-rendering-parity` FOLLOWUPS entry status.** Spec §1 lists this as filed-during-brainstorm carry-forward. Per design/FOLLOWUPS.md tracking convention, since this is *not* resolved at v0.10.0 ship, no action is needed — the entry remains open, and FOLLOWUPS.md doesn't require an "untouched" annotation. Confirmed acceptable.

- **No FOLLOWUPS entries surfaced during this review.** All findings are plan-internal corrections, not deferred items.
