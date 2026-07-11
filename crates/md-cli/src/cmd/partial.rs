//! P1.1 (pathless/dead-card partial-decode) — shared render helpers for
//! `md decode` and `md inspect`.
//!
//! Hoisted here (out of the two command modules where they were previously
//! duplicated verbatim) so the marker text and the stderr note cannot drift
//! between the two commands — the byte-identical cross-binary contract with
//! the toolkit's `mnemonic inspect` (SPEC "cross-binary parity") depends on
//! both surfaces emitting exactly the same bytes on a partial decode.

/// The text-form marker printed on stdout (in addition to the
/// always-renderable template) when the decoded descriptor carries at least
/// one unresolved-origin `@N`. Verbatim per SPEC/plan — the origin is elided
/// (`m`) on the wire and has no canonical default for this shape, so it stays
/// genuinely unspecified.
pub(crate) const ORIGIN_UNSPECIFIED_MARKER: &str =
    "origin: \u{ab}unspecified \u{2014} supply on restore\u{bb}";

/// Emit the partial-decode stderr note (partial case only). Never lands on
/// stdout. `unres` is the ascending set of unresolved-origin `@N` indices
/// (always non-empty when this is called). The wording is number-agnostic —
/// "the origin(s) for … are unspecified" reads correctly for one OR several
/// indices (no `has`/`have` agreement bug).
pub(crate) fn emit_partial_stderr_note<W: std::io::Write>(unres: &[u8], w: &mut W) {
    let idxs = unres
        .iter()
        .map(|i| format!("@{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(
        w,
        "note: the origin(s) for {idxs} are unspecified \u{2014} this card shape has no canonical \
         default derivation path and none was supplied explicitly; exit 4 (VERIFY-ME): confirm \
         the intended path out-of-band before restoring funds from this backup"
    );
}
