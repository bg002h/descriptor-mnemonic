//! The ONE place BIP 388's pairwise-distinctness rule is quoted.
//!
//! Three md surfaces refuse the same wallet shape — a keyed template
//! (`cmd::build`, the T row), a set of mk1 key cards (`seat::satisfy`, the S
//! row) and a concrete descriptor (`decompose`, the D row) — and SPEC A3
//! requires all three to say the same thing: cite BIP 388, say UNSUPPORTED,
//! never call the input invalid.
//!
//! Three transcriptions of one quotation is exactly the shape that drifts, and
//! a drifted quotation is a false record about a normative document. The
//! citation lives here so the two host-side compose refusals share bytes rather
//! than a promise. (`decompose` quotes the rule TOGETHER with its security note
//! and the BIP's own invalid example, so its longer text stays in place; the
//! sentence below is the fragment all three have in common.)

/// BIP 388, "Additional rules", rule (1) — verbatim.
pub const PAIRWISE_DISTINCT_RULE: &str = "the public keys obtained by deserializing elements of \
     the key information vector must be pairwise distinct";

/// BIP 388, "Additional rules", rule (2) — the DISJOINTNESS rule, verbatim
/// modulo stripping the mediawiki `<tt>` markup (fetched from
/// `bitcoin/bips` master, `bip-0388.mediawiki` line 195, 2026-08-31, whole-
/// diff review r1 I1). This is the rule two KEY expressions on the SAME
/// placeholder actually break — including two IDENTICAL expressions, whose
/// multipath sets are then the same set compared with itself. Rule (1)
/// above (pairwise distinctness of the key information VECTOR) is a
/// different rule, about a different vector, and does not apply when that
/// vector holds only one element.
pub const DISJOINTNESS_RULE: &str = "if two KEY are KP/<M;N>/* and KP/<P;Q>/* for the same key \
     placeholder KP, then the sets {M, N} and {P, Q} must be disjoint";

/// The security note BIP 388 attaches to rule (1), paraphrased as the two
/// compose-side refusals state it.
pub const REUSE_SECURITY_NOTE: &str =
    "its security note adds that reusing pubkeys can be insecure in miniscript wallet policies";
