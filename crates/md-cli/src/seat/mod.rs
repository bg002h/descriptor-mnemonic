//! The seating engine (P2/A1-A5, B1-B2).
//!
//! C2 lands the engine under this doc comment; C0 landed the comment alone,
//! embedding plan section 1's
//! goal-and-gaps matrix per the operator's MATRIX-TRAVELS directive
//! (2026-08-30): the table below is byte-identical to
//! `design/IMPLEMENTATION_PLAN_wallet_form_converter.md` section 1, and
//! to `design/SPEC_wallet_form_converter.md`'s copy.
//!
//! ## Where each normative rule lives
//!
//! | SPEC rule | module |
//! | --- | --- |
//! | P2 input pipeline (A3(a): dedupe → group → reassemble) | [`input`] |
//! | A2 satisfaction, the two door checks, the two card-set checks | [`satisfy`] |
//! | A3 matchings + compose-canonicalise-compare + cap + tie-break | [`matching`] |
//! | A4 completeness (unfilled slots, leftover cards) | [`complete`] |
//! | composition, the comparison form, the spend-equality checker | [`compose`] |
//!
//! ## 1. The surface: one matrix
//!
//! **THE MATRIX TRAVELS (operator directive, 2026-08-30): this table is
//! the cycle's goal-and-gaps statement and is embedded, cells kept
//! current, in EVERY artifact — brainstorm, spec, this plan, and the
//! seating engine's module doc comment in the code. A document or module
//! missing it is incomplete.**
//!
//! Input forms (any COMPLETE wallet expression):
//!
//! - **D** — concrete descriptor (miniscript or plain), keys + origins inline
//! - **T** — BIP-388 template + per-slot keys/origins as flags
//! - **S** — the split card set: keyless md1 phrases + mk1 strings
//! - **K** — keyed md1 phrases (Pubkeys TLV)
//!
//! Output forms: concrete descriptor · addresses · keyed card (via the
//! existing `md encode --key` bridge) · template + origin-notated key lines.
//!
//! | in \ out | concrete descriptor | addresses | keyed card | keyless + mk1 cards |
//! | --- | --- | --- | --- | --- |
//! | **D** concrete descriptor | — | ✗ P3 | ✗ P3+bridge | ✗ P3 (the decomposer) |
//! | **T** template + key flags | ⚠ P1 (flag-form gap; inline template origins already work — r1 I8) | ⚠ P1 | ✓ `md encode --key` (Divergent) | ✓ |
//! | **S** keyless card + mk1 strings | ✗ P2 (the seating engine) | ✗ P2 | ✗ P2+bridge | — |
//! | **K** keyed card phrases | ✓ (round-tripped live) | ✓ | — | ✗ non-goal (first real need files it) |
//!
//! ✓ measured working; ⚠/✗ the gaps, tagged with the piece that closes
//! them. On C4 close, the ⚠/✗ cells this cycle owns flip to ✓ in every
//! embedded copy in the same commit as the acceptance walk that proves
//! them.

// The engine's modules are reached from `main` only once step 7 lands the
// `--from-mk1` surface (plan §3 C2). Until then the allow keeps
// `clippy -D warnings` honest without hiding anything else; step 7's commit
// removes it, exactly as C1 removed C0's.
#[allow(dead_code)]
pub mod complete;
#[allow(dead_code)]
pub mod compose;
#[allow(dead_code)]
pub mod input;
#[allow(dead_code)]
pub mod matching;
#[allow(dead_code)]
pub mod satisfy;
