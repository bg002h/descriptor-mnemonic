# v0.18 Phase 7 — Whole-PR architect review (per-phase report)

**Date:** 2026-05-09
**Branch:** `feat/v0.18-full-tap-and-nums-engraving`

## Scope

Final architect pass over the v0.18 cycle (7 commits, 47 files changed, ~2000 net insertions). Per-phase reviews already SHIP'd at each gate (0C/0I after inline fixes); this is the integration-level pass before the v0.18 release tag.

## Architect findings

**SHIP** with 2 pre-tag actions:

1. **Required before tag** — Open Phase 8 companion PR against `mnemonic-toolkit/docs/manual/src/40-cli-reference/42-md.md` per CLAUDE.md `manual-cli-surface-mirror` invariant.
2. **Recommended before tag** — File `v0.18-render-wrapper-chain-empty-prefix-defensibility` FOLLOWUP. **Done** (also added a `debug_assert!` guard in `render_wrapper_chain` for explicit invariant).

### All 8 architect-round-1 findings verified resolved

- **C1** (decode.rs lockstep formula): `decode.rs:24` matches `encode.rs::key_index_width`. Cross-reference comments in both. Clean.
- **C2** (canonicalize.rs `> n`): `check_placeholder_bounds` line 241 + `validate.rs::walk_for_placeholders` line 72 both have `> n` semantics. Clean.
- **I1** (v017_v1_c → v018_v1_c rename): present at line 61 of `v017_v1_encode_acceptance.rs` (architect's grep was scoped wrong; manually verified).
- **I2** (render_node n-threading): cascaded to all call sites; sentinel check at `text.rs:41`. Clean.
- **I3** (Thresh and AndOr standalone tests): both Phase 4a walker tests present.
- **I4** (round-trip path assertion): structural assertions (NUMS hex contains, multi_a contains, etc.) appropriate for integration layer.
- **L1** (tag.rs comment): module doc updated with v0.17→v0.18 transition.
- **L2** (MIGRATION exact error): `Error::UnknownExtensionTag(0x05)` present.

### Latent gap addressed inline

**`render_wrapper_chain` empty-prefix path** (`text.rs`). The function loops collecting wrapper letters; if called with a non-wrapper tag, the loop breaks immediately producing a bare `:` followed by inner render (malformed miniscript). Currently unreachable because the dispatch arm at `render_node` is restricted to the 6 wrapper tags. Phase 7 added a `debug_assert!` at function entry to make the invariant explicit. Filed `v0.18-render-wrapper-chain-empty-prefix-defensibility` FOLLOWUP for a future structural restructure.

### Documentation consistency fix

CHANGELOG headline previously said "17 new walker arms" but body description listed 17 + True/False. MIGRATION listed 19. Reconciled CHANGELOG to "17 new walker arms ... plus 2 boolean-literal arms (True, False — reachable via miniscript's `t:` sugar = `and_v(X, 1)`)" — both docs now align on 17 + 2 = 19.

### Wire-format break completeness

Confirmed within workspace: `encode.rs`, `decode.rs`, `validate.rs`, `canonicalize.rs`, `tree.rs`, `text.rs` all updated in lockstep. Architect noted a theoretical concern about external consumers reimplementing `key_index_width` independently; pre-1.0 with no known external consumers, this is non-blocking.

### Cross-phase coupling correctness

Architect verified Phase 1 → Phase 5 dependency (--path enables round-trip canonicity), Phase 4b refactor of Phase 4a additions (no orphaned arms), and the wire-format break flow through validate/canonicalize/encode/decode/walker/renderer. No regressions or dead code.

### Test coverage

420 tests for a wire-format break + 17 walker arms + 4 CLI surface changes. Architect verdict: "defensible for a pre-1.0 cycle with no phrases in the wild." 4 sentinel boundary tests at n=1,2,3,4 + 2 round-trip integration tests + 6 Phase 4a/4b round-trips + 6 walker shape tests + 6 CLI rejection/acceptance tests.

### Strict break vs additive judgment

Architect verdict: "structurally incompatible with v0.17; same-day ship with zero phrases in the wild means the migration cost is exactly zero. The break is correct and well-scoped."

### CLI UX coherence

Architect verdict: "coherent in isolation." One latent confusion (`--unspendable-key` accepts only the default value the auto-NUMS path provides, making it a no-op when supplied) is documented in MIGRATION and scoped to v0.19+ for caller-supplied internal-key support.

### FOLLOWUP hygiene

Filed during cycle: `v0.18-phase-1-low-2-cli-path-non-from-policy-test-gate`, `v0.18-phase-4a-build-multi-node-k-bounds-parity`, `v0.18-render-wrapper-chain-empty-prefix-defensibility` (Phase 7).

Resolved during cycle: `v0.17.1-from-policy-round-trip-integration` (carryover from canceled v0.17.1).

Net cycle FOLLOWUP delta: +2.

### Manual mirror invariant (Phase 8 prerequisite)

CLAUDE.md `manual-cli-surface-mirror` requires the companion PR in `mnemonic-toolkit` to open BEFORE the v0.18 tag is pushed. The user-visible flag deltas in v0.18:

- `--path` flag is now functional (was wired but silently dropped pre-v0.18). If the manual described it as silently ignored, an update is required.
- `--unspendable-key` narrowed (xpub-form rejected).
- All other flag surfaces unchanged.

Phase 8 must verify the manual covers the new behavior before tagging.

## Verification (final pre-tag)

- `cargo test --workspace --all-features` → 420 pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- `cargo fmt` (was applied during v0.17 cleanup; no further drift).

## Exit gate

- ✅ All 8 architect-round-1 findings verified resolved.
- ✅ One latent gap addressed inline (render_wrapper_chain debug_assert).
- ✅ Documentation count discrepancy fixed (17 + 2 = 19, both docs aligned).
- ✅ Two new FOLLOWUPS filed (4a-parity, wrapper-chain-defensibility).
- ✅ One v0.17.1 carryover resolved.
- ✅ Wire-format break verified complete within workspace.
- ✅ Workspace tests + clippy clean (420 tests).
- ✅ Architect SHIP with 2 pre-tag actions identified.

Phase 7 closed; Phase 8 (release tagging + manual mirror PR) up next. Manual mirror PR must open BEFORE the tag push.
