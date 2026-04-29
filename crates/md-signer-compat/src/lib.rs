//! Named signer-subset validators for Mnemonic Descriptor (MD) wallet policies.
//!
//! md-codec is signer-neutral by default — per
//! `design/MD_SCOPE_DECISION_2026-04-28.md`, v0.6 removed the encoder/
//! decoder default validator gate. This crate is the layered checker
//! callers can use for explicit pre-encode validation against named
//! hardware-signer operator subsets.
//!
//! # Example
//!
//! ```no_run
//! use md_signer_compat::{COLDCARD_TAP, validate};
//! use miniscript::{DescriptorPublicKey, Miniscript, Tap};
//! use std::str::FromStr;
//!
//! let leaf: Miniscript<DescriptorPublicKey, Tap> = "pk(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)"
//!     .parse()
//!     .unwrap();
//! validate(&COLDCARD_TAP, &leaf, Some(0)).unwrap();
//! ```
//!
//! # Vendor citation discipline
//!
//! Each named subset (`COLDCARD_TAP`, `LEDGER_TAP`, ...) carries an inline
//! comment with the source URL, source repo's commit SHA (if known), and
//! last-checked date. Vendor doc revisions → subset bump → crate patch
//! release.

mod coldcard;
mod ledger;

pub use coldcard::COLDCARD_TAP;
pub use ledger::LEDGER_TAP;

/// A named subset of miniscript operators a hardware signer is documented
/// to admit. Operator names follow rust-miniscript desugared AST node
/// naming (matching md-codec's `tag_to_bip388_name` adapter output).
///
/// See module-level documentation for vendor-citation discipline.
#[derive(Debug, Clone)]
pub struct SignerSubset {
    /// Human-readable name (e.g., "Coldcard tap-leaf").
    pub name: &'static str,
    /// Operator names (rust-miniscript desugared AST node names) the signer admits.
    pub allowed_operators: &'static [&'static str],
}

/// Validate a tap-context miniscript leaf against a named signer subset.
///
/// Returns `Ok(())` if every operator in the leaf AST appears in
/// `subset.allowed_operators`. Returns
/// [`md_codec::Error::SubsetViolation`] with the offending operator
/// name and `leaf_index` on the first out-of-subset operator.
///
/// `leaf_index` is the DFS pre-order index of this leaf within the
/// containing tap tree. Pass `Some(0)` for single-leaf, `Some(n)` for
/// the n-th leaf in DFS pre-order, or `None` for callers without
/// leaf-index context.
///
/// For multi-leaf trees, see [`validate_tap_tree`] which walks every
/// leaf and threads a derived DFS pre-order index through to each
/// per-leaf call.
pub fn validate(
    subset: &SignerSubset,
    ms: &miniscript::Miniscript<miniscript::DescriptorPublicKey, miniscript::Tap>,
    leaf_index: Option<usize>,
) -> Result<(), md_codec::Error> {
    md_codec::bytecode::encode::validate_tap_leaf_subset_with_allowlist(
        ms,
        subset.allowed_operators,
        leaf_index,
    )
}

/// Validate every leaf of a tap tree against a named signer subset,
/// threading a DFS pre-order leaf index through each per-leaf call.
///
/// On the first violation, returns
/// [`md_codec::Error::SubsetViolation`] with the offending operator
/// name and the *DFS pre-order index of the leaf* that contained it.
/// Returns `Ok(())` if every leaf is in-subset.
///
/// Tap trees with no script leaves (key-path-only `tr(K)`) are accepted
/// trivially — `Ok(())` with no validator calls.
pub fn validate_tap_tree(
    subset: &SignerSubset,
    tap_tree: &miniscript::descriptor::TapTree<miniscript::DescriptorPublicKey>,
) -> Result<(), md_codec::Error> {
    // Upstream `TapTree::leaves()` returns leaves in DFS pre-order via a
    // `TapTreeIterItem` accessor (`leaf.miniscript()` / `leaf.depth()`).
    // `enumerate()` re-indexes that order to derive `leaf_index`.
    for (idx, leaf) in tap_tree.leaves().enumerate() {
        validate(subset, leaf.miniscript(), Some(idx))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
