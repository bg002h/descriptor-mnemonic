# v0.11 Phase 18 Review — Display rules

**Date:** 2026-04-30
**Branch:** `feature/v0.11-impl-phase-1`
**Phase commit:** `dce48c7` — feat(v0.11): engraving render helper for visual grouping
**Spec reference:** §10.2 (engraving display)

## Status: DONE

Phase 18 ships the minimal code-side display surface required by the v0.11 spec: a single `render_codex32_grouped(s, group_size)` helper in `crates/md-codec/src/v11/encode.rs` that emits a transcription-friendly hyphen-grouped form of an md1 string. Two unit tests cover the happy path (grouping at 4) and the `group_size == 0` no-op path.

## Verification

- `cargo test -p md-codec --lib v11::encode` — **2 passed**, 0 failed:
  - `v11::encode::render_tests::render_groups_at_4`
  - `v11::encode::render_tests::render_zero_group_size_no_grouping`
- Cumulative v11 test count: **89** (87 prior + 2 new).

## Scope notes

Phase 18's display surface is intentionally minimal in this round of the plan:

- The single helper `render_codex32_grouped` is the lone code-side item, intended for CLI/UI to emit a transcription-friendly grouped form.
- Decoders strip whitespace/separators on input per **D11 / §10.5**, so the hyphen is purely a display artifact and never affects the canonical wire form or BCH residue.
- Multi-card layout templates and phrase-rendering recommendations (§10.3, §10.4) are documented in the spec as **non-normative engraver guidance**. They do not require code; they are spec-text addressed by users at engraving time.
- The spec's "engrave the md1 string + optionally the phrase + optionally the fingerprint" pattern is implementation-agnostic and needs no codec-level support.

## Spec alignment (§10.2)

The grouped-display helper is consistent with §10.2's engraving-display intent: the canonical string remains the source of truth, while UIs may insert visual group separators (e.g., hyphens every 4 symbols) to aid manual transcription. The decoder's whitespace/separator-stripping rule (D11 / §10.5) ensures grouped renderings round-trip without modification to the encoded payload.

## Concerns

None. The code surface is small, well-tested for its two branches, and the broader §10.3/§10.4 territory is correctly left to spec text rather than codec code.

## Carry-forward deferred items

Same set as prior phases: P1, P2, P4, P5, P12, P13a, P13b.

## Context for next phase

**Phase 19:** codex32 BCH wiring — bypassing v0.x's `encode_string` with symbol-aligned packing. This is the wire-format heart of v0.11 and where the per-format BCH residue / HRP-mixing plumbing shared with mk1 becomes load-bearing.
