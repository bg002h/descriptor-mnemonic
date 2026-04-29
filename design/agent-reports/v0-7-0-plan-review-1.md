# v0.7.0 plan review #1 — pre-implementation

**Status:** DONE_WITH_CONCERNS
**Reviewer:** Claude Opus 4.7 (1M context)
**Date:** 2026-04-28
**Plan commit:** 54819b6 (on `feature/v0.7.0-development`)
**File(s):**
- `design/IMPLEMENTATION_PLAN_v0_7_0.md` (under review)
- `design/SPEC_v0_7_0.md` (companion)
- `design/SPEC_v0_6_strip_layer_3.md` §2.3 (byte-shift truth source)
- `crates/md-codec/src/bytecode/encode.rs` lines 644–679 (`validate_tap_leaf_subset` source)
- `design/agent-reports/v0-7-0-spec-review-1.md` (spec round-1 review)
**Role:** reviewer (plan)

## Summary

The plan is solid and faithfully implements the spec. Verified across 6 specific concerns and found **no Critical issues**, **2 Important issues** (Phase 5 internal-key open question; Phase 2.2 round-trip coverage gap), and **4 Nits** (allowlist/rustdoc drift; validate API generic-vs-Tap-only; missing-from-table mapping; missing all-features acceptance gate). Structural ordering (Phase 3 before Phase 4) is correct; the byte-shift mapping table is accurate against SPEC v0.6 §2.3; the `HISTORICAL_COLDCARD_TAP_OPERATORS` constant exactly matches the existing v0.6 hardcoded match arms.

The controller can safely proceed with Phase 1 in parallel. The Important issues land in Phase 2 and Phase 5; the controller will reach those after Phase 1 lands.

## Verification of the 6 specific concerns

### Concern 1 — Byte-shift mapping table accuracy: PASS (high confidence)

Cross-checked every row of Plan §1.2's mapping table against SPEC v0.6 §2.3 (canonical source at `design/SPEC_v0_6_strip_layer_3.md` lines 128–170). All 20 rows match exactly. No swapped values; no transcription errors. (`Tag::TapTree` 0x08→0x07, `Tag::Multi` 0x19→0x08, `Tag::MultiA` 0x1A→0x0A, all wrappers/operators shifted by 2, `Placeholder` and `SharedPath` shifted by 1 — all match.)

### Concern 2 — Phase 5 internal-key open question: IMPORTANT (confidence 90)

`design/IMPLEMENTATION_PLAN_v0_7_0.md` lines 953–966 leaves three unresolved questions for the Tap branch:

```rust
ScriptContext::Tap => {
    let ms = concrete.compile::<miniscript::Tap>()...;
    todo!("wrap in tr + to_bytecode")
}
```

The plan acknowledges this with "NOTE: This implementation has open questions" but defers the decision to Phase 5 execution. The implementer will hit this and have to decide ad-hoc. The three options are mutually exclusive at the API level:

- (a) `policy_to_bytecode` adds a 4th parameter `internal_key: Option<DescriptorPublicKey>` — caller-supplies.
- (b) Default-to-NUMS-unspendable internal key when the policy is leaf-only.
- (c) Reject Tap-context compilation entirely until v0.7.1 (compiler-feature ships Segwitv0-only at v0.7.0).

**Recommendation: pin (a) explicitly in the plan before Phase 5 starts.** Adding a parameter is the most flexible, and matches the spec §5.2's signature in spirit (a 3-param API can grow to 4). Add to the plan's §5.2.1 sketch:

```rust
pub fn policy_to_bytecode(
    policy: &str,
    options: &EncodeOptions,
    script_context: ScriptContext,
    internal_key: Option<DescriptorPublicKey>,  // None → use NUMS for Tap
) -> Result<Vec<u8>, Error>
```

This also affects spec §5.2 — the spec also shows the 3-param signature without addressing this. Worth raising before Phase 5.

### Concern 3 — Phase 4 → Phase 3 dependency: PASS (high confidence)

Phase 4 imports `md_codec::bytecode::encode::validate_tap_leaf_subset_with_allowlist` (plan line 669) which doesn't exist until Phase 3 lands (plan line 493). Plan Phase ordering is correct: Phase 3 (Task 3.2) creates `pub fn validate_tap_leaf_subset_with_allowlist` before Phase 4 (Task 4.2) consumes it.

There's one subtlety: workspace `Cargo.toml` `members = ["crates/md-codec", "crates/md-signer-compat"]` change in Task 4.1.3 adds the new crate to the workspace; if Phase 4 tasks are partially started before Phase 3 lands, then `cargo check --workspace` will fail.

**Recommendation:** Add a dependency note at top of Phase 4: "Prerequisite: Phase 3 must be committed (Step 3.4.2) before Phase 4 begins."

### Concern 4 — Phase 1 exact failing-test count: PASS (high confidence)

Plan Step 1.1.1 produces an exact count via `wc -l /tmp/v0-7-baseline-failures.txt`. The "off-by-one" (10+8+10+6+5=39 vs ~38) is a spec-level approximation, not a plan defect. The plan correctly resolves this empirically.

### Concern 5 — Phase 2 byte-order test round-trip coverage: IMPORTANT (confidence 85)

Spec §3.2 says the test "catches the round-trip-stable-but-format-changed regression class that the corpus alone cannot detect." This implies the test should pin BOTH directions:

1. Encoder emits internal-byte-order (currently in plan Step 2.2.1).
2. **Decoder, on input bytes in internal-byte-order, reconstructs the same hash.** (NOT currently in plan.)

The plan's test only checks half the loop — asserting `out[1..33] == known_32` catches encoder-side accidents but does NOT catch asymmetric encode/decode bugs where both sides reverse, producing `encoded_hash == decoded_hash` even though the wire format silently rotated.

**Recommendation:** Extend Step 2.2.1's test body to add a decode pass after each encode:

```rust
// Decode and assert reconstructed hash equals input (catches asymmetric
// reverse-on-encode + reverse-on-decode bug class).
let decoded: Terminal<DescriptorPublicKey, Tap> = decode_tap_terminal(&out, ...).unwrap();
match decoded {
    Terminal::Sha256(h) => assert_eq!(h.as_byte_array(), &known_32),
    _ => panic!("expected Sha256, got {decoded:?}"),
}
// Repeat for Hash256, Ripemd160, Hash160.
```

The exact API name (`decode_tap_terminal`) needs verification at execution time.

### Concern 6 — `HISTORICAL_COLDCARD_TAP_OPERATORS` correctness: PASS (high confidence)

Verified against `crates/md-codec/src/bytecode/encode.rs` lines 657–678. Source admits: `PkK`, `PkH`, `MultiA`, `Older`, `AndV`, `OrD`, `Check`, `Verify`. Names from `tag_to_bip388_name`: `pk_k`, `pk_h`, `multi_a`, `older`, `and_v`, `or_d`, `c:`, `v:`.

Plan's constant `&["pk_k", "pk_h", "multi_a", "or_d", "and_v", "older", "c:", "v:"]` — exact 8-name match. Order differs but is irrelevant for `.contains()` semantics. **The shim is byte-identically equivalent to the v0.6 hardcoded match arms.**

## Additional findings

### Finding A — LEDGER_TAP rustdoc/array drift on `pk_h`: nit (confidence 80)

SPEC_v0_7_0.md lines 348–349 docstring text says 8 operators (no `pk_h`); constant body at line 354 has 9 (includes `pk_h`). The plan inherits this drift in Plan §4.4.

**Recommendation:** verify whether Ledger's `cleartext.rs` actually admits `pk_h` and either remove from array or add to rustdoc text.

### Finding B — Plan §4.2.1 `validate` API generic-vs-Tap-only drift from spec §4.2: nit (confidence 80)

Spec §4.2 line 297 has `<C: ScriptContext>` generic; Plan §4.2.1 lines 664–674 pins to `Tap`. The Tap-only path is simpler and matches spec scope (`tap-leaf miniscript subsets`). Pick one and align spec+plan.

### Finding C — SortedMulti / SortedMultiA missing from Plan §1.2 mapping table: nit (confidence 80)

Plan §1.2's table correctly omits `Tag::SortedMulti` (0x09 unchanged from v0.5) and `Tag::SortedMultiA` (NEW at 0x0B). A 1-line note would help: "(Tags whose byte is unchanged, or that are NEW in v0.6, are excluded from this table.)"

### Finding D — Phase 6 acceptance gate missing all-features test: nit (confidence 80)

Phase 5 gates compiler/cli-compiler tests behind `--features cli-compiler`. `cargo test -p md-codec` (default features) won't exercise them. Phase 6 acceptance should include both `--no-default-features` AND `--all-features` test runs.

## Items for FOLLOWUPS

If not addressed inline, track in `design/FOLLOWUPS.md`:

1. Add SortedMulti / SortedMultiA explanatory note to Plan §1.2 mapping table.
2. Reconcile spec §4.2 generic `<C>` vs plan §4.2.1 Tap-only validate signature.
3. Reconcile LEDGER_TAP rustdoc operator list vs constant body (`pk_h` omitted in text, present in array).
4. Add `cargo test -p md-codec --no-default-features` and `--all-features` to Phase 6 acceptance gate.

## Verdict

**DONE_WITH_CONCERNS.** Phase 1 may proceed without modification. Two Important issues to address before reaching their phases:

- **Phase 2:** add decode-direction round-trip assertion to the byte-order pin test (Concern 5).
- **Phase 5:** pin the internal-key strategy for Tap-context compilation explicitly in the plan before Phase 5 starts (Concern 2).

Two nits worth addressing inline if cheap (Findings A, B), or otherwise track in FOLLOWUPS.
