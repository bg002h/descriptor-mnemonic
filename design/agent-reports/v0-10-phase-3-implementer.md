# v0.10 Phase 3 implementer report — policy-layer integration (decoded_origin_paths, to_bytecode dispatch)

**Status:** DONE_WITH_CONCERNS — Phase 3 ships per the plan; Tier 2 KIV walk is stubbed (FOLLOWUPS entry filed).

**Working directory:** `/scratch/code/shibboleth/descriptor-mnemonic`
**Branch:** `feature/v0.10-per-at-n-paths`
**Commit:** TBD (see end of report; updated after `git commit`)

## Files changed (absolute paths)

- `/scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/src/policy.rs` — `WalletPolicy::decoded_origin_paths` field, `placeholder_paths_in_index_order`, `try_extract_paths_from_kiv` (Tier 2 stub), `resolve_shared_path_fallback` extracted helper, `to_bytecode` rewritten to dispatch on auto-detected divergence, `from_bytecode_with_fingerprints` rewritten to dispatch on header bit 3, count-mismatch validation post-tree-walk, 8 new tests.
- `/scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/src/options.rs` — `EncodeOptions::origin_paths` Tier 0 override field, `with_origin_paths` builder, default test updated, 1 new test for the builder.
- `/scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec/src/bytecode/path.rs` — removed `#[allow(dead_code)]` from `decode_origin_paths` (now wired into `from_bytecode`).
- `/scratch/code/shibboleth/descriptor-mnemonic/design/FOLLOWUPS.md` — appended `v010-p3-tier-2-kiv-walk-deferred` entry under Open items, tier `v0.11`.

## Tier 2 KIV walk: stubbed in v0.10.0

`WalletPolicy::try_extract_paths_from_kiv` returns `Ok(None)` and is documented with a `// TODO(v0.10-followup):` comment pointing to the FOLLOWUPS entry.

**Rationale (also captured in FOLLOWUPS):** the natural input — `descriptor.iter_pk()` post-`set_key_info(&dummies).into_descriptor()` — traverses in AST order. For `sortedmulti(...)` this yields lex-sorted-by-pubkey-bytes order, not placeholder-index order. A correct walk needs either (a) access to the inner `WalletPolicy.template: Descriptor<KeyExpression>` (currently private in the fork; no public accessor) so that each `KeyExpression`'s `.index` field can map AST position → placeholder index, or (b) a refactor that captures per-`@N` paths during `from_descriptor` ingestion and stores them on the `WalletPolicy`. Either approach is non-trivial and warrants a separate design pass — outside Phase 3 scope.

**Behavior implication:** the v0.x ≤ 0.9 silent-flatten bug for freshly-parsed-from-string concrete-key descriptors with divergent per-`@N` paths is NOT fully fixed in v0.10.0; Tier 3 fires and the encoder emits SharedPath for those policies (identical to v0.9 — no regression, no progression). Production callers wanting per-`@N` divergence today have the Tier 0 `EncodeOptions::origin_paths` override (test-vector generation) and the Tier 1 `decoded_origin_paths` round-trip path (any policy decoded from a `Tag::OriginPaths`-bearing bytecode). Both are fully wired and tested.

## Test counts

- Workspace (`cargo test --workspace --all-features --no-fail-fast`): **701 ok / 1 failed**.
- The single failed test is the expected `every_error_variant_has_a_rejects_test_in_conformance` conformance gate — Phase 4 fixes it (covered in plan).
- Pre-Phase-3 baseline was 692 ok / 1 failed → Phase 3 adds **+9 tests**, all passing.

### New test names (9)

`crates/md-codec/src/options.rs`:

- `encode_options_with_origin_paths_sets_field`

`crates/md-codec/src/policy.rs` (Phase 3 block at end of `mod tests`):

- `round_trip_shared_path_byte_identical_to_v0_9` — Step 3.6
- `round_trip_divergent_paths_via_origin_paths_override` — Step 3.6
- `tier_0_origin_paths_override_wins_over_tier_1` — Step 3.6.5 (F8/F9)
- `tier_3_shared_fallback_for_template_only_policy` — Step 3.6.5
- `double_round_trip_origin_paths_byte_identical` — Step 3.6.5
- `decoded_shared_path_and_decoded_origin_paths_mutually_exclusive_after_decode` — Step 3.6.5
- `from_bytecode_rejects_header_bit_3_set_with_shared_path_tag` — Step 3.7
- `from_bytecode_rejects_header_bit_3_clear_with_origin_paths_tag` — Step 3.7

(Per the prompt: the `tier_1_decoded_wins_over_tier_2_kiv_walk` test from the plan is dropped because Tier 2 is stubbed.)

## Self-review findings

- All 4 round-trip tests pass.
- All 3 tier-precedence tests pass (Tier 0 over Tier 1, Tier 3 fallback, double round-trip stability).
- All 2 conflicting-path-decl rejection tests pass.
  - Header bit 3 SET + Tag::SharedPath wire byte: surfaces `UnexpectedTag { expected: 0x36, got: 0x34 }` from the new origin-paths arm in `from_bytecode_with_fingerprints`.
  - Header bit 3 CLEAR + Tag::OriginPaths wire byte: surfaces `UnexpectedTag { expected: 0x34, got: 0x36 }` from the existing `decode_declaration` (its existing tag-validation logic catches it correctly without any change).
- `decode_origin_paths` no longer marked `#[allow(dead_code)]` (it's now called by `from_bytecode_with_fingerprints` on the OriginPaths arm).
- Build clean; clippy `-D warnings` clean; `fmt --check` clean (one auto-format adjustment was applied to a multi-line `to_bytecode` call inside the new mutual-exclusion test).
- Workspace test count 701 ok / 1 failed (= 692 prior + 9 new); the one failure is the expected `every_error_variant_has_a_rejects_test_in_conformance` Phase 4 gate.

## FOLLOWUPS entries appended

- **`v010-p3-tier-2-kiv-walk-deferred`** (tier `v0.11`, status `open`) — captures the architectural ambiguity of the KIV walk (sortedmulti AST-order vs placeholder-index order; private `WalletPolicy.template` field with no public accessor in the fork) and pins the v0.11 follow-up plan with two design alternatives (template walk via private-field accessor request upstream, or per-`@N` path capture at `from_descriptor` ingestion). Cites `WalletPolicy::try_extract_paths_from_kiv` and `placeholder_paths_in_index_order` as the relevant code surfaces. Documents the no-regression / no-progression behavior for v0.x ≤ 0.9-shaped concrete-key inputs.

## Architectural surprises in `policy.rs` shape vs. plan sketch

1. **Existing `to_bytecode` shared-path tier chain was Phase B (4-tier) not "single shared_path"** — the existing v0.x logic already had a 4-tier precedence chain (`opts.shared_path` → `decoded_shared_path` → `shared_path()` → `default_path_for_v0_4_types` → BIP 84). The plan's sketch showed a simpler "Tier 3 shared fallback" but in practice this is already a 5-step chain. Resolved by extracting the existing chain into a `resolve_shared_path_fallback` helper and broadcasting its single output across all placeholders for Tier 3 use. Wire format for v0.9-shaped inputs is byte-identical (verified by all pre-existing wsh-no-origin / BIP 49/84/48-default tests still passing).

2. **`from_bytecode_with_fingerprints` already accepted `Cursor` via `Cursor::new(&bytes[1..])`** — the plan sketched a `Cursor` constructed from the full byte stream with an explicit offset bookkeeper. Adapted to use the existing slice-from-offset-1 + cursor pattern, saving an offset translation. The OriginPaths arm reads its tag byte directly from `cursor.read_byte()` and validates it against `Tag::OriginPaths.as_byte()` (matching the existing Fingerprints-block tag-validation pattern at lines 517–525).

3. **`UnexpectedTag` variant exists at `BytecodeErrorKind::UnexpectedTag { expected: u8, got: u8 }`** — exact match to the plan's expectation. No new variant introduced; Phase 3 reuses the existing variant for both rejection paths.

4. **No public `key_info()` accessor on the fork's `WalletPolicy`** — the inner type's `key_info: Vec<DescriptorPublicKey>` field is private, with no getter. This drove the Tier 2 stub decision; the `iter_pk()` workaround would surface AST-order keys (which sortedmulti reorders), not placeholder-index-order keys. Captured in the FOLLOWUPS entry.

5. **`default_path_for_v0_4_types` lives at module scope, not as an `impl` method** — the plan's sketch placed the precedence-chain helper inside `impl WalletPolicy`. The existing helper is a free function at module scope (line ~640 of `policy.rs`). I kept it where it was and call it from the new `resolve_shared_path_fallback` method; no need to move it.

## Verification commands run (Step 3.8)

```bash
$ cargo build --workspace --all-features 2>&1 | tail -3
   Compiling md-codec v0.10.0 (/scratch/code/shibboleth/descriptor-mnemonic/crates/md-codec)
   Compiling md-signer-compat v0.1.1 (/scratch/code/shibboleth/descriptor-mnemonic/crates/md-signer-compat)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.08s

$ cargo test --package md-codec policy 2>&1 | grep '^test result'
test result: ok. 81 passed; 0 failed; 0 ignored; 0 measured; 391 filtered out; finished in 0.05s

$ cargo test --workspace --all-features --no-fail-fast 2>&1 | grep '^test result' | awk '{ok+=$4; failed+=$6} END {print "ok="ok" failed="failed}'
ok=701 failed=1

$ cargo +stable clippy --workspace --all-features --all-targets -- -D warnings 2>&1 | tail -3
    Checking md-codec v0.10.0 (...)
    Checking md-signer-compat v0.1.1 (...)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.63s

$ cargo +stable fmt --all -- --check 2>&1 | head -3
(no output → clean)
```

## Outstanding scope check

- All Step 3.1–3.7 deliverables: DONE.
- Step 3.8 build/test/clippy/fmt gate: PASSED.
- Step 3.9 commit: PENDING (this is the final action).
- Step 3.10 (opus reviewer gate): outside Phase 3 implementer scope; controller dispatches.
