# MD scope decision (2026-04-28) — strip Layer 3 (signer-compatibility curation)

**Audience:** future agents and reviewers reading the codebase chronologically. Phase D's careful work to enforce the Coldcard tap-leaf subset (`design/agent-reports/phase-v0-2-d-taproot.md`, commit `6f6eae9`) is being **undone** in v0.6. This document explains why, so the apparent regression makes sense in context.

**TL;DR.** MD's scope is reframed from "encoding + signer-compatibility curation" to "encoding only." Whether a given BIP 388 wallet policy is signable on a given hardware signer becomes a layered concern handled by tools above and below MD, not by the encoding format itself. `validate_tap_leaf_subset` is preserved as a `pub fn` for explicit-call use but is no longer invoked by default. Named signer subsets (Coldcard, Ledger, etc.) move to a separate library that md-codec consumers can compose if they want signer-aware validation.

## Background — what Phase D did, and why

Phase D (v0.2, commit `6f6eae9`) added taproot top-level descriptor support and enforced the Coldcard per-leaf miniscript subset (`pk` / `pk_h` / `multi_a` / `or_d` / `and_v` / `older` plus the `c:` / `v:` wrappers needed to spell those operators in canonical BIP 388 form) at **both** encode and decode time.

The reasoning at the time was sound on its own terms:

- BIP 388's `wsh()` policies admit the full miniscript surface, but tap-leaf miniscript has a much narrower subset that hardware signers actually sign.
- An MD-encoded backup is meant for engravable steel — it sits in a safe for years before being read back. If the encoder admits operators no signer will sign, the backup is unspendable: encoded successfully today, unrecoverable tomorrow.
- To prevent this footgun, MD's BIP draft (`bip/bip-mnemonic-descriptor.mediawiki:547`) wrote a MUST clause: implementations supporting taproot MUST enforce the per-leaf miniscript subset constraints required by deployed hardware signers. Phase D mirrored this in code: `validate_tap_leaf_subset` was called on every encode and every decode, and out-of-subset operators surfaced a `TapLeafSubsetViolation` error.

Phase D filed two FOLLOWUPS entries for the meta-question — `phase-d-tap-leaf-wrapper-subset-clarification` and `phase-d-tap-miniscript-type-check-parity` — both deferred until "evidence from real signers" was available.

## What changed (2026-04-28 design discussion)

The 2026-04-28 design discussion challenged the premise that signer-subset enforcement is MD's job. Three observations broke the original framing:

### 1. The MUST clause was ours, not the spec's

BIP 388 §"Implementation guidelines" (`bip-0388.mediawiki:216`) explicitly says:

> "It is acceptable to implement only a subset of the possible wallet policies defined by this standard. It is recommended that any limitations are clearly documented."

That's *permission to subset*, not a directive to mirror signers. The MUST clause we wrote into MD's BIP draft was a Phase D / Phase 2 design choice, not a spec inheritance. We could rewrite it. (And we will — see `md-strip-spec-and-docs` in FOLLOWUPS.)

### 2. The recovery-footgun argument lives at the wrong layer

The argument was: encode → engrave → can't sign later. Therefore MD must curate.

The same logic, applied consistently, would also require:

- The wallet software that *generates* the policy to encode-check it before passing to MD (otherwise MD is curating something the wallet shouldn't have produced).
- The user choosing to engrave to *understand* what signer they intend to recover with (otherwise no curation by MD can save them — they could change signer vendors after engraving).
- The signer firmware to be *stable* over the backup's lifetime (which it isn't — Coldcard's edge subset has already expanded since Phase D shipped).

The footgun is real, but the layer that owns it isn't the encoding format. The wallet software has the signer context; MD has only the policy. Curating at MD level is doing the wrong job at the wrong level.

### 3. Other formats don't curate

PSBT doesn't reject scripts that some signers won't sign. Bitcoin Core's RPC doesn't reject descriptors based on signer subset. Address formats encode whatever's given. The ecosystem norm is *"the format is neutral; tools above and below it apply their own constraints."* MD's curation was an outlier.

## The reframe

Three layers:

1. **Wire encoding** — bech32 + BCH error correction + chunking. Lossless serialization of bytes.
2. **Bytecode format** — tag-per-operator + LEB128 + placeholder framing. Maps to BIP 388 wallet-policy ASTs.
3. **Admit-set policy** — which BIP 388 wallet-policy subset MD accepts.

Layers 1 and 2 are *neutral*. Layer 3 was a *curation choice* — and it's the one being stripped. After v0.6, MD is purely Layers 1 and 2: a wire format for BIP 388 wallet policies, no opinions on signer compatibility.

## Citations supporting the reframe

- **BIP 388 §"Implementation guidelines"** — `bip-0388.mediawiki:216`, the explicit "it is acceptable to implement only a subset" license.
- **Coldcard `firmware/edge` `docs/taproot.md`** — proves MD's Phase D subset is *narrower* than even Coldcard's documented admit set (Coldcard admits `sortedmulti_a`, multi-leaf TapTrees, `pkh`-in-tap-leaves; MD did not). MD was protecting users from things the signer admits, which is the opposite of the original justification.
- **Ledger `LedgerHQ/vanadium` `apps/bitcoin/common/src/bip388/cleartext.rs`** — second-vendor evidence, with first-class compound-shape variants (`SortedMultisig`, `RelativeHeightlockMultiSig`, `RelativeTimelockMultiSig`, `AbsoluteHeightlockMultiSig`, `AbsoluteTimelockMultiSig`) that MD never admitted.
- **rust-miniscript** — ships `Terminal::SortedMultiA` and a `VALID_TEMPLATES` test fixture (`src/descriptor/wallet_policy/mod.rs:351`) using `sortedmulti_a` inside a `tr({...})` multi-leaf TapTree.

The vendor evidence converges: signers are *broader* than MD's Phase D subset, and the gap was widening, not narrowing.

## What survives, what goes

**Survives:**

- `validate_tap_leaf_subset` and `validate_tap_leaf_terminal` stay as `pub fn` in `crates/md-codec/src/bytecode/encode.rs`. Callers can invoke them explicitly. Encoder and decoder no longer call them by default.
- `Error::TapLeafSubsetViolation` variant stays in `crates/md-codec/src/error.rs`. The explicit-call validator path produces it.
- The vendor citations and subset definitions Phase D documented are not lost — they migrate to a separate library (`md-signer-compat-checker-separate-library` in FOLLOWUPS) where named subsets like `COLDCARD_TAP` and `LEDGER_TAP` live alongside their citation comments and update cadence (vendor doc revision → subset bump).

**Goes:**

- The default-path encoder call to `validate_tap_leaf_subset` (currently in `Miniscript<_, Tap>` `EncodeTemplate` impl).
- The default-path decoder rejection arms in `decode_tap_terminal` (catch-all `_ => Err(TapLeafSubsetViolation)`) and the matching `validate_tap_leaf_subset` calls at `decode.rs:295` (single-leaf path) and `decode.rs:802` (multi-leaf path).
- The BIP draft MUST clause at `bip-mnemonic-descriptor.mediawiki:547` — rewrites to MAY-informational, with a new §"Signer compatibility (informational)" section that explicitly delegates to layered tools.

## Implementation plan

Tracked in `design/FOLLOWUPS.md` under the master entry `md-scope-strip-layer-3-signer-curation`. Six v0.6 child entries:

1. `md-strip-validator-default-and-corpus` — encoder/decoder default flip + corpus expansion for newly-admitted shapes.
2. `md-strip-spec-and-docs` — BIP MUST → MAY-informational; new §"Signer compatibility"; README and CLI recovery-responsibility framing.
3. `md-tag-space-rework` — allocate `Tag::SortedMultiA`, reorganize the Tag enum, drop the `Reserved*` range 0x24–0x31 entirely (Option B per the 2026-04-28 discussion).
4. `md-signer-compat-checker-separate-library` — v0.6+ aspirational; named signer subsets + caller-supplied opt-in API design.
5. `md-policy-compiler-feature` — v0.7+ future release; rust-miniscript `compiler` feature + `policy_to_bytecode` API.
6. `v0-6-release-prep-revised` — release plumbing under the strip framing.

8 prior FOLLOWUPS entries are now superseded (status: `wont-fix — superseded by <id>`):
`phase-d-tap-leaf-wrapper-subset-clarification`, `phase-d-tap-miniscript-type-check-parity`, `tap-leaf-admit-sortedmulti-a`, `tap-leaf-admit-after`, `tap-leaf-corpus-timelocked-multisig-shapes`, `tap-leaf-corpus-pkh-shape`, `v0-6-release-prep`, `p2-inline-key-tags` (the last as wont-fix-per-design rather than supersession — Reserved\* tags are out of scope under MD's BIP-388 wallet-policy framing).

## What this document is NOT

- Not a spec. The actual v0.6 spec is `design/SPEC_v0_6_strip_layer_3.md` (forthcoming).
- Not an implementation plan. That's `design/IMPLEMENTATION_PLAN_v0_6_strip_layer_3.md` (forthcoming).
- Not a critique of Phase D. Phase D's work was sound under its own framing; the framing changed. The Phase D agent-report stays as-is at `design/agent-reports/phase-v0-2-d-taproot.md` — it's a faithful record of the v0.2 reasoning. A forward-pointer note added at the top of that report links here for chronological orientation.

## Open questions deferred to spec/plan

- Final Tag layout post-rework (will be drafted in `SPEC_v0_6_strip_layer_3.md`).
- Whether the `Error::TapLeafSubsetViolation` variant should stay as-is or get a more general name (`SubsetViolation` or similar) since it'll be used for arbitrary caller-supplied subsets, not just the historical Coldcard one.
- Tooling for migrating any existing v0.5.x test fixtures that depended on `TapLeafSubsetViolation` being raised on encode of an out-of-subset operator (these flip to either positive vectors or negative-with-explicit-validator-invocation).

## Decision log (working session 2026-04-28)

Recorded for traceability:

- **Layer 3 strip**: yes (user decision).
- **Reserved\* tags 0x24–0x31**: drop entirely (Option B; MD's BIP-388 framing forbids inline keys; descriptor-codec inline-key-form vendoring is dead weight).
- **Sortedmulti_a Tag allocation**: folded into `md-tag-space-rework` (no longer its own entry).
- **Opt-in API shape**: caller-supplied `SignerSubset` spec; design deferred to `md-signer-compat-checker-separate-library` (v0.6+/aspirational).
- **Compiler integration**: future release, post-strip (v0.7+).
- **Spec/plan workflow**: one stacked spec+plan with iterative review of both; agents log reports to `design/agent-reports/`; critical and important review items addressed immediately, nits and nice-to-haves go to FOLLOWUPS; final pass compares agent reports against FOLLOWUPS to catch missed items.
- **Worktree**: feature branch `feature/v0.6-strip-layer-3` in the existing repo checkout; no separate worktree dispatch (avoids the `[patch]`-block-relative-path gotcha documented in project memory).
- **v0.5 vector files**: drop entirely after the rework (no users pinning SHAs pre-1.0).
