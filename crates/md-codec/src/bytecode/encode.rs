//! Bytecode template encoder for MD wallet policies.
//!
//! Walks a `Descriptor<DescriptorPublicKey>` and emits the canonical bytecode
//! used by Template Cards. Key positions are replaced by `Tag::Placeholder`
//! followed by an LEB128-encoded index into the wallet policy's key
//! information vector (caller supplies the index map).
//!
//! v0.4 scope: `wsh(...)`, `tr(...)` (single-leaf), `wpkh(...)`, `sh(wpkh(...))`,
//! and `sh(wsh(...))` top-level descriptors are accepted. Pkh and Bare are rejected
//! with `PolicyScopeViolation`. Sh-inner restricted to Wpkh/Wsh per BIP §"Sh wrapper restriction matrix" (Ms rejected — legacy P2SH out of scope).
//! Inline keys (any key not present in `placeholder_map`) are also rejected.
//!
//! v0.2 (Phase D): single-leaf taproot `tr()` top-level descriptors are also
//! accepted, with the per-leaf miniscript subset enforced per BIP §"Taproot
//! tree" (Coldcard subset: `pk_k`, `pk_h`, `multi_a`, `or_d`, `and_v`,
//! `older` plus the `c:` / `v:` wrappers required to spell them through the
//! BIP 388 string form).
//!
//! v0.5: multi-leaf TapTree `tr()` is admitted via DFS pre-order traversal of
//! `TapTree::leaves()`, emitting `Tag::TapTree` (0x08) inner-node framings.
//! KeyOnly `tr(KEY)` and single-leaf `tr(KEY, leaf)` paths are preserved
//! byte-identically from v0.4.x.
//!
//! Architecture mirrors `joshdoman/descriptor-codec` (CC0) — see
//! `design/PHASE_2_DECISIONS.md` D-4 for rationale.

use std::collections::HashMap;
use std::sync::Arc;

use bitcoin::hashes::Hash;
use miniscript::descriptor::{Descriptor, DescriptorPublicKey, ShInner, Wsh};
use miniscript::{Miniscript, Segwitv0, Tap, Terminal, Threshold};

use crate::Error;
use crate::bytecode::Tag;

/// Encode a wallet-policy descriptor into canonical Template Card bytecode.
///
/// `placeholder_map` maps each public key in the descriptor to its index in
/// the wallet policy's key information vector (`@i` placeholder index).
///
/// Returns the encoded byte stream on success. Returns
/// [`Error::PolicyScopeViolation`] if the descriptor uses a top-level form
/// not supported in v0.1, or if any leaf key is not present in
/// `placeholder_map` (inline keys are forbidden in v0.1's wallet-policy
/// framing).
pub fn encode_template(
    descriptor: &Descriptor<DescriptorPublicKey>,
    placeholder_map: &HashMap<DescriptorPublicKey, u8>,
) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    descriptor.encode_template(&mut out, placeholder_map)?;
    Ok(out)
}

/// Internal walker trait. Each implementation appends its bytecode encoding
/// to `out`. Mirrors `joshdoman/descriptor-codec`'s `EncodeTemplate` trait
/// but emits only the template byte stream (no payload) and returns errors
/// instead of panicking on out-of-scope inputs.
///
/// **`out` mutation contract**: implementations may have appended bytes to
/// `out` before returning `Err(...)`. Callers must not assume `out` is
/// unchanged on error. The public [`encode_template`] function shields
/// callers by always allocating a fresh `Vec`, so the dirty-on-error state
/// is only visible to internal recursive walkers.
trait EncodeTemplate {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error>;
}

/// Forward `EncodeTemplate` through `Arc<T>`.
///
/// Miniscript's `Terminal` AST stores child fragments as
/// `Arc<Miniscript<...>>`, so logical operators (and_v / and_b / and_or /
/// or_*) recurse through `Arc` references. Without this impl, calling
/// `child.encode_template(...)` on an `&Arc<Miniscript<...>>` would not
/// resolve to the trait method directly. This blanket impl makes any
/// `Arc<T>` where `T: EncodeTemplate` itself implement `EncodeTemplate`
/// by dereferencing once.
impl<T: EncodeTemplate> EncodeTemplate for Arc<T> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        (**self).encode_template(out, placeholder_map)
    }
}

impl EncodeTemplate for Descriptor<DescriptorPublicKey> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        match self {
            Descriptor::Wsh(wsh) => {
                out.push(Tag::Wsh.as_byte());
                wsh.encode_template(out, placeholder_map)
            }
            Descriptor::Wpkh(wpkh) => {
                out.push(Tag::Wpkh.as_byte());
                wpkh.as_inner().encode_template(out, placeholder_map)
            }
            Descriptor::Sh(sh) => {
                out.push(Tag::Sh.as_byte());
                match sh.as_inner() {
                    ShInner::Wpkh(wpkh) => {
                        out.push(Tag::Wpkh.as_byte());
                        wpkh.as_inner().encode_template(out, placeholder_map)
                    }
                    ShInner::Wsh(wsh) => {
                        out.push(Tag::Wsh.as_byte());
                        // Reuse Wsh::encode_template which only emits the inner script.
                        wsh.encode_template(out, placeholder_map)
                    }
                    ShInner::Ms(_) => Err(Error::PolicyScopeViolation(
                        "sh(<legacy P2SH>) including sh(multi/sortedmulti) is permanently \
                         rejected (legacy non-segwit out of scope per design); use \
                         sh(wsh(sortedmulti(...))) for modern nested-segwit multisig"
                            .to_string(),
                    )),
                }
            }
            Descriptor::Pkh(_) => Err(Error::PolicyScopeViolation(
                "top-level pkh() is permanently rejected (legacy non-segwit out of scope per design)"
                    .to_string(),
            )),
            Descriptor::Tr(tr) => {
                // BIP §"Taproot tree" + Phase D D-1: emit Tag::Tr (0x06)
                // followed by the internal-key placeholder reference and an
                // optional script-tree subtree. v0.5 admits multi-leaf trees
                // via Tag::TapTree (0x08) inner-node framing; KeyOnly and
                // single-leaf paths preserved byte-identically from v0.4.x.
                out.push(Tag::Tr.as_byte());
                tr.internal_key().encode_template(out, placeholder_map)?;
                if let Some(tap_tree) = tr.tap_tree() {
                    let leaves: Vec<(u8, &Arc<Miniscript<DescriptorPublicKey, Tap>>)> = tap_tree
                        .leaves()
                        .map(|item| (item.depth(), item.miniscript()))
                        .collect();
                    if leaves.is_empty() {
                        return Err(Error::PolicyScopeViolation(
                            "tap_tree present but contains no leaves".to_string(),
                        ));
                    }
                    if leaves.len() == 1 && leaves[0].0 == 0 {
                        // Single-leaf path — byte-identical to v0.4.x.
                        let leaf_ms = leaves[0].1;
                        validate_tap_leaf_subset(leaf_ms, Some(0))?;
                        leaf_ms.encode_template(out, placeholder_map)?;
                    } else {
                        // Multi-leaf path — emit 0x08 framings.
                        // (Encoder relies on rust-miniscript TapTree::combine's
                        //  upstream depth-128 invariant; no defensive check needed.)
                        //
                        // Entry target_depth=0 (the implicit "tree root" level):
                        // any leaf at depth >= 1 triggers a 0x08 framing and the
                        // recursion descends until the leaf depths match the
                        // target depth. Mirrors the decoder's depth==0 entry
                        // into `decode_tap_subtree`.
                        let mut cursor: usize = 0;
                        encode_tap_subtree(&leaves, &mut cursor, 0, out, placeholder_map)?;
                        debug_assert_eq!(
                            cursor,
                            leaves.len(),
                            "encode_tap_subtree must consume all leaves"
                        );
                    }
                }
                Ok(())
            }
            Descriptor::Bare(_) => Err(Error::PolicyScopeViolation(
                "top-level bare() is permanently rejected (legacy non-segwit out of scope per design)"
                    .to_string(),
            )),
        }
    }
}

impl EncodeTemplate for Wsh<DescriptorPublicKey> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        self.as_inner().encode_template(out, placeholder_map)
    }
}

impl EncodeTemplate for Miniscript<DescriptorPublicKey, Segwitv0> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        self.node.encode_template(out, placeholder_map)
    }
}

impl EncodeTemplate for Terminal<DescriptorPublicKey, Segwitv0> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        match self {
            Terminal::True => {
                out.push(Tag::True.as_byte());
                Ok(())
            }
            Terminal::False => {
                out.push(Tag::False.as_byte());
                Ok(())
            }
            Terminal::PkK(key) => {
                out.push(Tag::PkK.as_byte());
                key.encode_template(out, placeholder_map)
            }
            Terminal::PkH(key) => {
                out.push(Tag::PkH.as_byte());
                key.encode_template(out, placeholder_map)
            }
            Terminal::Multi(thresh) => {
                out.push(Tag::Multi.as_byte());
                thresh.encode_template(out, placeholder_map)
            }
            Terminal::SortedMulti(thresh) => {
                out.push(Tag::SortedMulti.as_byte());
                out.push(u8::try_from(thresh.k()).map_err(|_| {
                    Error::PolicyScopeViolation(format!(
                        "threshold k={} exceeds single-byte width (255)",
                        thresh.k()
                    ))
                })?);
                out.push(u8::try_from(thresh.n()).map_err(|_| {
                    Error::PolicyScopeViolation(format!(
                        "threshold n={} exceeds single-byte width (255)",
                        thresh.n()
                    ))
                })?);
                for pk in thresh.iter() {
                    pk.encode_template(out, placeholder_map)?;
                }
                Ok(())
            }
            Terminal::MultiA(thresh) => {
                // Taproot multi-A. Unreachable through the Segwitv0 context
                // (miniscript's typing rules forbid `multi_a` inside `wsh()`)
                // but kept here so the Segwitv0 dispatch stays exhaustive.
                // The Tap-context dispatch below has its own MultiA arm
                // that is the live path for Phase D.
                out.push(Tag::MultiA.as_byte());
                thresh.encode_template(out, placeholder_map)
            }
            Terminal::AndV(left, right) => {
                out.push(Tag::AndV.as_byte());
                left.encode_template(out, placeholder_map)?;
                right.encode_template(out, placeholder_map)
            }
            Terminal::AndB(left, right) => {
                out.push(Tag::AndB.as_byte());
                left.encode_template(out, placeholder_map)?;
                right.encode_template(out, placeholder_map)
            }
            Terminal::AndOr(a, b, c) => {
                out.push(Tag::AndOr.as_byte());
                a.encode_template(out, placeholder_map)?;
                b.encode_template(out, placeholder_map)?;
                c.encode_template(out, placeholder_map)
            }
            Terminal::OrB(left, right) => {
                out.push(Tag::OrB.as_byte());
                left.encode_template(out, placeholder_map)?;
                right.encode_template(out, placeholder_map)
            }
            Terminal::OrC(left, right) => {
                out.push(Tag::OrC.as_byte());
                left.encode_template(out, placeholder_map)?;
                right.encode_template(out, placeholder_map)
            }
            Terminal::OrD(left, right) => {
                out.push(Tag::OrD.as_byte());
                left.encode_template(out, placeholder_map)?;
                right.encode_template(out, placeholder_map)
            }
            Terminal::OrI(left, right) => {
                out.push(Tag::OrI.as_byte());
                left.encode_template(out, placeholder_map)?;
                right.encode_template(out, placeholder_map)
            }
            Terminal::Thresh(thresh) => {
                out.push(Tag::Thresh.as_byte());
                thresh.encode_template(out, placeholder_map)
            }
            Terminal::After(after) => {
                out.push(Tag::After.as_byte());
                crate::bytecode::varint::encode_u64(u64::from(after.to_consensus_u32()), out);
                Ok(())
            }
            Terminal::Older(older) => {
                out.push(Tag::Older.as_byte());
                crate::bytecode::varint::encode_u64(u64::from(older.to_consensus_u32()), out);
                Ok(())
            }
            Terminal::Sha256(h) => {
                out.push(Tag::Sha256.as_byte());
                out.extend_from_slice(h.as_byte_array());
                Ok(())
            }
            Terminal::Hash256(h) => {
                // h is miniscript::hash256::Hash — a forward-display newtype
                // around sha256d::Hash. as_byte_array() returns the *internal*
                // byte order (NOT the reversed display order of sha256d).
                // Decoders MUST round-trip via hash256::Hash::from_byte_array,
                // not sha256d::Hash::from_hex.
                out.push(Tag::Hash256.as_byte());
                out.extend_from_slice(h.as_byte_array());
                Ok(())
            }
            Terminal::Ripemd160(h) => {
                out.push(Tag::Ripemd160.as_byte());
                out.extend_from_slice(h.as_byte_array());
                Ok(())
            }
            Terminal::Hash160(h) => {
                out.push(Tag::Hash160.as_byte());
                out.extend_from_slice(h.as_byte_array());
                Ok(())
            }
            Terminal::Alt(child) => {
                out.push(Tag::Alt.as_byte());
                child.encode_template(out, placeholder_map)
            }
            Terminal::Swap(child) => {
                out.push(Tag::Swap.as_byte());
                child.encode_template(out, placeholder_map)
            }
            Terminal::Check(child) => {
                out.push(Tag::Check.as_byte());
                child.encode_template(out, placeholder_map)
            }
            Terminal::DupIf(child) => {
                out.push(Tag::DupIf.as_byte());
                child.encode_template(out, placeholder_map)
            }
            Terminal::Verify(child) => {
                out.push(Tag::Verify.as_byte());
                child.encode_template(out, placeholder_map)
            }
            Terminal::NonZero(child) => {
                out.push(Tag::NonZero.as_byte());
                child.encode_template(out, placeholder_map)
            }
            Terminal::ZeroNotEqual(child) => {
                out.push(Tag::ZeroNotEqual.as_byte());
                child.encode_template(out, placeholder_map)
            }
            Terminal::RawPkH(h) => {
                // RawPkH encodes a 20-byte pubkey hash literal embedded directly
                // in the miniscript fragment (no key info vector lookup). This is
                // distinct from Terminal::Hash160 which encodes a hash-preimage
                // commitment under the same 20-byte width. Distinct tags
                // (RawPkH = 0x1D vs Hash160 = 0x23) keep the wire format unambiguous.
                out.push(Tag::RawPkH.as_byte());
                out.extend_from_slice(h.as_byte_array());
                Ok(())
            }
            // Defensive catch-all for any Terminal variant added in a future
            // miniscript version. v0.1's Segwitv0 scope is fully covered above;
            // this arm is unreachable today (Terminal is not #[non_exhaustive]
            // in the workspace-pinned miniscript v12, so the compiler proves
            // unreachability — the #[allow] keeps the guard in place against
            // a future upgrade that adds new variants).
            #[allow(unreachable_patterns)]
            other => Err(Error::PolicyScopeViolation(format!(
                "unsupported Terminal fragment in v0.1 scope: {other:?}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Tap-context encoder (Phase D)
// ---------------------------------------------------------------------------
//
// Forwarding impls so `Miniscript<DescriptorPublicKey, Tap>` and its inner
// `Terminal<DescriptorPublicKey, Tap>` flow through the same encoding shape
// the Segwitv0 path uses. The wire format for shared operators (e.g. `pk_k`,
// `multi_a`, `or_d`, `and_v`, `older`, the `c:` / `v:` wrappers) is identical
// across contexts: the per-leaf subset validator (`validate_tap_leaf_subset`)
// is what prevents the encoder from emitting any tap-illegal terminal.

impl EncodeTemplate for Miniscript<DescriptorPublicKey, Tap> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        self.node.encode_template(out, placeholder_map)
    }
}

impl EncodeTemplate for Terminal<DescriptorPublicKey, Tap> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        match self {
            Terminal::PkK(key) => {
                out.push(Tag::PkK.as_byte());
                key.encode_template(out, placeholder_map)
            }
            Terminal::PkH(key) => {
                out.push(Tag::PkH.as_byte());
                key.encode_template(out, placeholder_map)
            }
            Terminal::MultiA(thresh) => {
                out.push(Tag::MultiA.as_byte());
                thresh.encode_template(out, placeholder_map)
            }
            Terminal::AndV(left, right) => {
                out.push(Tag::AndV.as_byte());
                left.encode_template(out, placeholder_map)?;
                right.encode_template(out, placeholder_map)
            }
            Terminal::OrD(left, right) => {
                out.push(Tag::OrD.as_byte());
                left.encode_template(out, placeholder_map)?;
                right.encode_template(out, placeholder_map)
            }
            Terminal::Older(older) => {
                out.push(Tag::Older.as_byte());
                crate::bytecode::varint::encode_u64(u64::from(older.to_consensus_u32()), out);
                Ok(())
            }
            Terminal::Check(child) => {
                // `c:` wrapper. Emitted by the BIP 388 parser when the user
                // writes `pk(K)` (a tap-illegal bare K-type fragment is
                // wrapped as `c:pk_k(K)` to lift it to B-type).
                out.push(Tag::Check.as_byte());
                child.encode_template(out, placeholder_map)
            }
            Terminal::Verify(child) => {
                // `v:` wrapper. Required to drive `and_v(v:..., ...)`, the
                // canonical way to write `and(...)` with timelocks under
                // tapscript per BIP §"Taproot tree" (Coldcard subset).
                out.push(Tag::Verify.as_byte());
                child.encode_template(out, placeholder_map)
            }
            // Anything else is outside the Phase D Coldcard subset.
            // `validate_tap_leaf_subset` is the gate that catches these
            // before the encoder is invoked; if a caller bypasses validation
            // and emits an out-of-subset terminal we still surface a precise
            // error rather than silently writing it.
            other => Err(Error::TapLeafSubsetViolation {
                operator: tap_terminal_name(other).to_string(),
                leaf_index: None, // Terminal-encoder catch-all has no leaf-index context;
                                  // the outer validate_tap_leaf_subset call site supplies index.
            }),
        }
    }
}

/// Validate that a tap-leaf miniscript fragment uses only operators in the
/// BIP §"Taproot tree" Coldcard subset (`pk_k`, `pk_h`, `multi_a`, `or_d`,
/// `and_v`, `older`, plus the `c:` and `v:` wrappers needed to spell them
/// through the BIP 388 parser).
///
/// Walks the AST recursively. On the first violation, returns
/// [`Error::TapLeafSubsetViolation`] with the offending operator name. See
/// `design/PHASE_v0_2_D_DECISIONS.md` D-2.
///
/// **Wrapper-terminal handling** (D-2 narrowing): `Terminal::Check` (`c:`)
/// and `Terminal::Verify` (`v:`) are allowed because the BIP 388 parser
/// emits them implicitly when the user writes `pk(K)` (= `c:pk_k`) or
/// `and_v(v:..., ...)`, both of which are explicitly named in the
/// Coldcard subset. Every other miniscript wrapper (`a:` / `s:` / `d:` /
/// `j:` / `n:`) is rejected: none appear in the Coldcard tap-leaf
/// vocabulary documented as of edge firmware. v0.3 may relax this if a
/// signer documents a wider safe wrapper set; tracked as
/// `phase-d-tap-leaf-wrapper-subset-clarification` in `FOLLOWUPS.md`.
///
/// # Parameters
///
/// `leaf_index` is the DFS pre-order index of this leaf within the
/// containing tap tree. The value is propagated into
/// [`Error::TapLeafSubsetViolation`] to enrich diagnostics for multi-leaf
/// decode/encode paths. Pass `Some(0)` for single-leaf `tr(KEY, leaf)`,
/// `Some(n)` for the n-th leaf in DFS pre-order traversal of a multi-leaf
/// tree, or `None` for callers without leaf-index context (currently no
/// in-tree caller passes `None`; reserved for external callers).
pub fn validate_tap_leaf_subset(
    ms: &Miniscript<DescriptorPublicKey, Tap>,
    leaf_index: Option<usize>,
) -> Result<(), Error> {
    validate_tap_leaf_terminal(&ms.node).map_err(|e| match e {
        Error::TapLeafSubsetViolation { operator, .. } => Error::TapLeafSubsetViolation {
            operator,
            leaf_index,
        },
        other => other,
    })
}

fn validate_tap_leaf_terminal(term: &Terminal<DescriptorPublicKey, Tap>) -> Result<(), Error> {
    match term {
        // Allowed leaves (per BIP §"Taproot tree" / Coldcard subset).
        Terminal::PkK(_) | Terminal::PkH(_) => Ok(()),
        Terminal::MultiA(_) => Ok(()),
        Terminal::Older(_) => Ok(()),
        // Allowed compositions — recurse into children.
        Terminal::AndV(a, b) | Terminal::OrD(a, b) => {
            validate_tap_leaf_terminal(&a.node)?;
            validate_tap_leaf_terminal(&b.node)
        }
        // Allowed safe wrappers (used by the BIP 388 parser when spelling
        // `pk(...)` and `and_v(v:..., ...)`).
        Terminal::Check(child) | Terminal::Verify(child) => validate_tap_leaf_terminal(&child.node),
        // Everything else is out-of-subset for v0.2.
        other => Err(Error::TapLeafSubsetViolation {
            operator: tap_terminal_name(other).to_string(),
            leaf_index: None, // Sub-helper of validate_tap_leaf_subset; outer caller
                              // re-wraps the error with the correct leaf_index via
                              // map_err in Task 2.3.
        }),
    }
}

/// Recursive encoder helper for v0.5 multi-leaf TapTree emission.
///
/// Walks a depth-annotated leaf slice (DFS pre-order, from
/// `TapTree::leaves()`) and emits `Tag::TapTree (0x08)` framings as the
/// target depth dictates. Leaves are encoded inline once the leaf's depth
/// matches the current target depth; otherwise emit a `0x08` framing and
/// recurse with `target_depth + 1` for both children.
///
/// **Invariant**: `leaves[*cursor].0 >= target_depth` is upheld by the DFS
/// pre-order from upstream `TapTree::leaves()`. The `else` arm exists for
/// completeness but is unreachable on inputs constructed via rust-miniscript
/// `TapTree::combine`.
fn encode_tap_subtree(
    leaves: &[(u8, &Arc<Miniscript<DescriptorPublicKey, Tap>>)],
    cursor: &mut usize,
    target_depth: u8,
    out: &mut Vec<u8>,
    placeholder_map: &HashMap<DescriptorPublicKey, u8>,
) -> Result<(), Error> {
    use std::cmp::Ordering;
    let leaf_depth = leaves[*cursor].0;
    match leaf_depth.cmp(&target_depth) {
        Ordering::Equal => {
            let leaf_index = *cursor;
            let ms = leaves[*cursor].1;
            validate_tap_leaf_subset(ms, Some(leaf_index))?;
            ms.encode_template(out, placeholder_map)?;
            *cursor += 1;
        }
        Ordering::Greater => {
            out.push(Tag::TapTree.as_byte());
            encode_tap_subtree(leaves, cursor, target_depth + 1, out, placeholder_map)?;
            encode_tap_subtree(leaves, cursor, target_depth + 1, out, placeholder_map)?;
        }
        // `leaf_depth < target_depth` is unreachable given DFS pre-order from
        // upstream `TapTree::leaves()`; intentionally a no-op rather than panic
        // (helper is internal; ill-formed input would surface elsewhere first).
        Ordering::Less => {}
    }
    Ok(())
}

/// Human-readable name for a tap-context Terminal variant, used in error
/// messages. Mirrors the operator names that appear in BIP 388 / miniscript
/// source form so caller-facing diagnostics are precise.
fn tap_terminal_name(term: &Terminal<DescriptorPublicKey, Tap>) -> &'static str {
    match term {
        Terminal::True => "1",
        Terminal::False => "0",
        Terminal::PkK(_) => "pk_k",
        Terminal::PkH(_) => "pk_h",
        Terminal::RawPkH(_) => "raw_pk_h",
        Terminal::Multi(_) => "multi",
        Terminal::SortedMulti(_) => "sortedmulti",
        Terminal::MultiA(_) => "multi_a",
        Terminal::SortedMultiA(_) => "sortedmulti_a",
        Terminal::After(_) => "after",
        Terminal::Older(_) => "older",
        Terminal::Sha256(_) => "sha256",
        Terminal::Hash256(_) => "hash256",
        Terminal::Ripemd160(_) => "ripemd160",
        Terminal::Hash160(_) => "hash160",
        Terminal::AndV(..) => "and_v",
        Terminal::AndB(..) => "and_b",
        Terminal::AndOr(..) => "andor",
        Terminal::OrB(..) => "or_b",
        Terminal::OrC(..) => "or_c",
        Terminal::OrD(..) => "or_d",
        Terminal::OrI(..) => "or_i",
        Terminal::Thresh(_) => "thresh",
        Terminal::Alt(_) => "a:",
        Terminal::Swap(_) => "s:",
        Terminal::Check(_) => "c:",
        Terminal::DupIf(_) => "d:",
        Terminal::Verify(_) => "v:",
        Terminal::NonZero(_) => "j:",
        Terminal::ZeroNotEqual(_) => "n:",
        // Defensive catch-all — Terminal isn't `#[non_exhaustive]` in the
        // pinned miniscript v13, so this is unreachable today; kept against
        // a future upgrade adding new variants.
        #[allow(unreachable_patterns)]
        _ => "<unknown-terminal>",
    }
}

/// Emit a `DescriptorPublicKey` as a placeholder reference.
///
/// Looks up `self` in `placeholder_map` and writes `Tag::Placeholder`
/// (`0x32`) followed by the LEB128-encoded index. Returns
/// [`Error::PolicyScopeViolation`] if the key is not present in the map
/// (v0.1 forbids inline keys; every leaf key must come through the
/// wallet-policy key information vector).
///
/// **Equality semantics**: `DescriptorPublicKey`'s derived `PartialEq`
/// includes the `origin` field (BIP 32 fingerprint + derivation path).
/// Callers that build `placeholder_map` from one parse of the descriptor
/// and re-parse the descriptor for encoding will get matching keys. But
/// callers that build the map from a stripped-origin form and then encode
/// a with-origin form (or vice versa) will see this impl report
/// "inline key" even though the bare key bytes match. Real BIP 388 wallet
/// policies always carry origin metadata; ensure your map is built from
/// the same parsed descriptor instance.
impl EncodeTemplate for DescriptorPublicKey {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        // Equality semantics: see module-level note about origin fields.
        let &index = placeholder_map.get(self).ok_or_else(|| {
            Error::PolicyScopeViolation(format!(
                "inline key not in placeholder_map (v0.1 forbids inline keys): {self}"
            ))
        })?;
        out.push(Tag::Placeholder.as_byte());
        out.push(index);
        Ok(())
    }
}

impl<T: EncodeTemplate, const MAX: usize> EncodeTemplate for Threshold<T, MAX> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        out.push(u8::try_from(self.k()).map_err(|_| {
            Error::PolicyScopeViolation(format!(
                "threshold k={} exceeds single-byte width (255)",
                self.k()
            ))
        })?);
        out.push(u8::try_from(self.n()).map_err(|_| {
            Error::PolicyScopeViolation(format!(
                "threshold n={} exceeds single-byte width (255)",
                self.n()
            ))
        })?);
        for elem in self.iter() {
            elem.encode_template(out, placeholder_map)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn empty_map() -> HashMap<DescriptorPublicKey, u8> {
        HashMap::new()
    }

    fn parse_descriptor(s: &str) -> Descriptor<DescriptorPublicKey> {
        Descriptor::from_str(s).expect("test fixture should parse")
    }

    #[test]
    fn rejects_pkh_top_level() {
        let d = parse_descriptor(
            "pkh(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(msg) if msg.contains("pkh")),
            "expected PolicyScopeViolation mentioning pkh"
        );
    }

    #[test]
    fn rejects_wpkh_inline_key() {
        // v0.4 accepts wpkh() at the top level, but inline keys (not in
        // placeholder_map) are still rejected with PolicyScopeViolation.
        let d = parse_descriptor(
            "wpkh(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("inline key")),
            "expected PolicyScopeViolation about inline key, got: {err:?}"
        );
    }

    #[test]
    fn rejects_sh_wpkh_inline_key() {
        // v0.4 accepts sh(wpkh()) at the top level, but inline keys are
        // still rejected with PolicyScopeViolation.
        let d = parse_descriptor(
            "sh(wpkh(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5))",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("inline key")),
            "expected PolicyScopeViolation about inline key, got: {err:?}"
        );
    }

    #[test]
    fn rejects_tr_inline_internal_key() {
        // Key-path-only taproot descriptor parsed with a raw key. v0.2
        // accepts top-level `tr()`, but rejects the internal key as an
        // inline key (no placeholder_map entry) — the same rule that
        // forbids inline keys inside `wsh()`.
        let d = parse_descriptor(
            "tr(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("inline key")),
            "expected PolicyScopeViolation about inline key, got: {err:?}"
        );
    }

    #[test]
    fn rejects_bare_top_level() {
        // Bare miniscript at the top level (no top-level wrapper).
        let d = parse_descriptor(
            "pk(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(msg) if msg.contains("bare")),
            "expected PolicyScopeViolation mentioning bare"
        );
    }

    #[test]
    fn encode_wsh_false() {
        // wsh(0) = top-level Wsh wrapping the False (always-false) terminal.
        let d = parse_descriptor("wsh(0)");
        let bytes = encode_template(&d, &empty_map()).unwrap();
        // Tag::Wsh = 0x05, Tag::False = 0x00.
        assert_eq!(bytes, vec![0x05, 0x00]);
    }

    #[test]
    fn encode_wsh_true() {
        // wsh(1) = top-level Wsh wrapping the True (always-true) terminal.
        let d = parse_descriptor("wsh(1)");
        let bytes = encode_template(&d, &empty_map()).unwrap();
        assert_eq!(bytes, vec![0x05, 0x01]);
    }

    #[test]
    fn encode_terminal_pk_k_with_placeholder() {
        // Construct a Terminal::PkK directly and verify encoding.
        // Going through the parser would require c:pk_k wrapping (Task 2.10),
        // so we drive the Terminal-level encoder directly.
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 0u8);

        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::PkK(key);
        let mut out = Vec::new();
        term.encode_template(&mut out, &map).unwrap();

        // Expected: Tag::PkK (0x1B), Tag::Placeholder (0x32), varint(0) (0x00).
        assert_eq!(out, vec![0x1B, 0x32, 0x00]);
    }

    #[test]
    fn encode_terminal_pk_h_with_placeholder() {
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let key = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 7u8);

        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::PkH(key);
        let mut out = Vec::new();
        term.encode_template(&mut out, &map).unwrap();

        // Expected: Tag::PkH (0x1C), Tag::Placeholder (0x32), varint(7) (0x07).
        assert_eq!(out, vec![0x1C, 0x32, 0x07]);
    }

    #[test]
    fn encode_pk_k_rejects_inline_key() {
        // A key not in the placeholder_map must produce PolicyScopeViolation.
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::PkK(key);
        let mut out = Vec::new();
        let err = term.encode_template(&mut out, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("inline key")),
            "expected PolicyScopeViolation about inline key, got: {err:?}"
        );
        // Pin the documented dirty-buffer-on-error contract: the PkK arm
        // pushed Tag::PkK before encode_template on the key returned Err,
        // so out is non-empty.
        assert_eq!(out, vec![Tag::PkK.as_byte()]);
    }

    // ---- v0.4 positive encode tests (Tasks 1.1-1.5) ----------------------

    #[test]
    fn encode_wpkh_single_key() {
        // wpkh(@0) encodes as: Tag::Wpkh (0x04), Tag::Placeholder (0x32), varint(0) (0x00).
        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 0u8);

        let d = parse_descriptor(&format!("wpkh({key})"));
        let bytes = encode_template(&d, &map).unwrap();

        // Expected: [0x04, 0x32, 0x00]
        assert_eq!(
            bytes,
            vec![Tag::Wpkh.as_byte(), Tag::Placeholder.as_byte(), 0x00]
        );
    }

    #[test]
    fn encode_sh_wpkh_single_key() {
        // sh(wpkh(@0)) encodes as: Tag::Sh (0x03), Tag::Wpkh (0x04), Tag::Placeholder (0x32), varint(0) (0x00).
        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 0u8);

        let d = parse_descriptor(&format!("sh(wpkh({key}))"));
        let bytes = encode_template(&d, &map).unwrap();

        // Expected: [0x03, 0x04, 0x32, 0x00]
        assert_eq!(
            bytes,
            vec![
                Tag::Sh.as_byte(),
                Tag::Wpkh.as_byte(),
                Tag::Placeholder.as_byte(),
                0x00,
            ]
        );
    }

    #[test]
    fn encode_sh_wsh_sortedmulti_2_of_3() {
        // sh(wsh(sortedmulti(2, @0, @1, @2))) encodes as:
        //   Tag::Sh (0x03), Tag::Wsh (0x05), Tag::SortedMulti (0x09),
        //   varint(2), varint(3), then 3 placeholder records.
        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let k2 = DescriptorPublicKey::from_str(
            "0395bcfdb728e8b1f0eda94f0db26d4ee3eebca73d11611ace1c0e4eed1bdc0e8a",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);
        map.insert(k2.clone(), 2u8);

        let d = parse_descriptor(&format!("sh(wsh(sortedmulti(2,{k0},{k1},{k2})))"));
        let bytes = encode_template(&d, &map).unwrap();

        // Structure: Sh + Wsh + SortedMulti + k + n + 3*(Placeholder + idx) = 11 bytes
        assert_eq!(bytes[0], Tag::Sh.as_byte()); // 0x03
        assert_eq!(bytes[1], Tag::Wsh.as_byte()); // 0x05
        assert_eq!(bytes[2], Tag::SortedMulti.as_byte()); // 0x09
        assert_eq!(bytes[3], 0x02); // k=2
        assert_eq!(bytes[4], 0x03); // n=3
        assert_eq!(bytes.len(), 11);
        let mut indices: Vec<u8> = Vec::with_capacity(3);
        for i in 0..3 {
            assert_eq!(bytes[5 + 2 * i], Tag::Placeholder.as_byte());
            indices.push(bytes[6 + 2 * i]);
        }
        indices.sort_unstable();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn encode_wsh_sortedmulti_2_of_3() {
        // Three distinct keys with placeholder indices 0, 1, 2.
        // wsh(sortedmulti(2, K0, K1, K2)) emits structurally:
        //   [Wsh, SortedMulti, varint(2), varint(3), <3 placeholder records>]
        // (See the comment near the assertions for why we don't pin a specific
        // per-record index order.)
        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let k2 = DescriptorPublicKey::from_str(
            "0395bcfdb728e8b1f0eda94f0db26d4ee3eebca73d11611ace1c0e4eed1bdc0e8a",
        )
        .unwrap();

        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);
        map.insert(k2.clone(), 2u8);

        let d = parse_descriptor(&format!("wsh(sortedmulti(2,{k0},{k1},{k2}))"));
        let bytes = encode_template(&d, &map).unwrap();

        // SortedMultiVec::pks() returns keys in parse order (insertion into
        // the descriptor string), not in BIP 67 lexicographic order — sorting
        // is deferred to witness construction via sorted_node(). Since our
        // three test keys' parse-order-vs-placeholder-index mapping is not
        // load-bearing, we assert structural shape (Wsh + SortedMulti + the
        // two varints + 3 placeholder records of 2 bytes each = 10 bytes)
        // and that each emitted index is in the valid range [0, 2].
        assert_eq!(bytes[0], Tag::Wsh.as_byte()); // 0x05
        assert_eq!(bytes[1], Tag::SortedMulti.as_byte()); // 0x09
        assert_eq!(bytes[2], 0x02); // varint(2)
        assert_eq!(bytes[3], 0x03); // varint(3)
        // Three placeholder records of 2 bytes each (Tag::Placeholder + idx).
        // 4 + 3*2 = 10 bytes total.
        assert_eq!(bytes.len(), 10);
        let mut indices: Vec<u8> = Vec::with_capacity(3);
        for i in 0..3 {
            assert_eq!(bytes[4 + 2 * i], Tag::Placeholder.as_byte()); // 0x32
            indices.push(bytes[5 + 2 * i]);
        }
        // The three emitted indices must be exactly {0, 1, 2} regardless of
        // emission order — every original key found its placeholder.
        indices.sort_unstable();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn encode_terminal_multi_2_of_3() {
        // Construct Terminal::Multi directly (the descriptor parser handles
        // multi() inside wsh() too; we drive the lower level here to keep
        // the test compact and to verify Threshold encoding directly).
        use miniscript::Threshold;

        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let k2 = DescriptorPublicKey::from_str(
            "0395bcfdb728e8b1f0eda94f0db26d4ee3eebca73d11611ace1c0e4eed1bdc0e8a",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);
        map.insert(k2.clone(), 2u8);

        let thresh = Threshold::new(2, vec![k0, k1, k2]).unwrap();
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Multi(thresh);
        let mut out = Vec::new();
        term.encode_template(&mut out, &map).unwrap();

        // Expected: Tag::Multi (0x19), varint(2) (0x02), varint(3) (0x03),
        // then for each of three keys: Placeholder (0x32) + varint(idx).
        // Multi preserves key order, so indices appear 0, 1, 2.
        assert_eq!(
            out,
            vec![
                0x19, // Tag::Multi
                0x02, // varint k=2
                0x03, // varint n=3
                0x32, 0x00, // Placeholder, idx 0
                0x32, 0x01, // Placeholder, idx 1
                0x32, 0x02, // Placeholder, idx 2
            ]
        );
    }

    #[test]
    fn encode_wsh_multi_via_descriptor_parser() {
        // wsh(multi(...)) parses through WshInner::Ms because multi is a
        // miniscript fragment, not a descriptor-level wrapper.
        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);

        let d = parse_descriptor(&format!("wsh(multi(1,{k0},{k1}))"));
        let bytes = encode_template(&d, &map).unwrap();

        // Wsh + Multi + varint(1) + varint(2) + 2 placeholder records (4 bytes).
        assert_eq!(bytes[0], Tag::Wsh.as_byte()); // 0x05
        assert_eq!(bytes[1], Tag::Multi.as_byte()); // 0x19
        assert_eq!(bytes[2], 0x01); // k=1
        assert_eq!(bytes[3], 0x02); // n=2
        assert_eq!(bytes[4], Tag::Placeholder.as_byte()); // 0x32
        assert_eq!(bytes[5], 0x00); // idx 0
        assert_eq!(bytes[6], Tag::Placeholder.as_byte()); // 0x32
        assert_eq!(bytes[7], 0x01); // idx 1
        assert_eq!(bytes.len(), 8);
    }

    // ---- Logical operators (Task 2.7) -------------------------------------
    //
    // Test fixtures for and_v / and_b / and_or / or_b / or_c / or_d / or_i.
    //
    // Type-system constraints (miniscript correctness rules) limit which
    // combinations of `0` (False, B-type, dissatisfiable+unit) and `1`
    // (True, B-type, non-dissatisfiable+unit) parse cleanly:
    //
    //   - and_or(B-du-unit, B, B): `andor(0,1,0)` parses (False is dis+unit)
    //   - or_d(B-du-unit, B):      `or_d(0,1)`     parses
    //   - or_i(B,B / V,V / K,K):   `or_i(0,1)`     parses
    //
    //   - and_v(V, B/V/K): needs V-type left (v: wrapper, Task 2.10)
    //   - and_b(B, W):     needs W-type right (a:/s: wrapper, Task 2.10)
    //   - or_b(E, W):      same — needs W-type
    //   - or_c(B-du, V):   needs V-type right
    //
    // For the four arms not reachable through the parser at this phase,
    // we drive the encoder directly with hand-built `Terminal::*` nodes
    // whose children are `Arc<Miniscript<_, _>>` built from `True`/`False`
    // via `Miniscript::from_ast`. This bypasses miniscript's typing
    // validator (since we never wrap the outer logical-op `Terminal` in
    // `Miniscript::from_ast`) and exercises only the encoder, which is
    // what these tests are about.

    fn make_true_arc() -> Arc<Miniscript<DescriptorPublicKey, Segwitv0>> {
        // True is B/zudemsx — accepted by `from_ast` unconditionally.
        Arc::new(Miniscript::from_ast(Terminal::True).unwrap())
    }

    fn make_false_arc() -> Arc<Miniscript<DescriptorPublicKey, Segwitv0>> {
        Arc::new(Miniscript::from_ast(Terminal::False).unwrap())
    }

    #[test]
    fn encode_wsh_or_d_with_constants() {
        // or_d(0, 1) parses: False is B-dissat-unit (left requirement),
        // True is B (right). Emits Wsh, OrD, False, True.
        let d = parse_descriptor("wsh(or_d(0,1))");
        let bytes = encode_template(&d, &empty_map()).unwrap();
        assert_eq!(
            bytes,
            vec![
                Tag::Wsh.as_byte(),   // 0x05
                Tag::OrD.as_byte(),   // 0x16
                Tag::False.as_byte(), // 0x00
                Tag::True.as_byte(),  // 0x01
            ]
        );
    }

    #[test]
    fn encode_wsh_or_i_with_constants() {
        // or_i(0, 1) parses: both children B-type. Emits Wsh, OrI, False, True.
        let d = parse_descriptor("wsh(or_i(0,1))");
        let bytes = encode_template(&d, &empty_map()).unwrap();
        assert_eq!(
            bytes,
            vec![
                Tag::Wsh.as_byte(),   // 0x05
                Tag::OrI.as_byte(),   // 0x17
                Tag::False.as_byte(), // 0x00
                Tag::True.as_byte(),  // 0x01
            ]
        );
    }

    #[test]
    fn encode_wsh_andor_with_constants() {
        // andor(0, 1, 0) parses: False is B-dissat-unit (the `a` branch
        // requirement), and all three children are B-type. Encoding emits
        // children in argument order: a, b, c.
        let d = parse_descriptor("wsh(andor(0,1,0))");
        let bytes = encode_template(&d, &empty_map()).unwrap();
        assert_eq!(
            bytes,
            vec![
                Tag::Wsh.as_byte(),   // 0x05
                Tag::AndOr.as_byte(), // 0x13
                Tag::False.as_byte(), // 0x00 (a)
                Tag::True.as_byte(),  // 0x01 (b)
                Tag::False.as_byte(), // 0x00 (c)
            ]
        );
    }

    #[test]
    fn encode_terminal_and_v_direct() {
        // and_v needs a V-type left, which requires a v: wrapper (Task 2.10).
        // Drive the encoder directly with True/False children — the encoder
        // is type-blind and just emits tag + children in order.
        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::AndV(make_true_arc(), make_false_arc());
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(
            out,
            vec![
                Tag::AndV.as_byte(),  // 0x11
                Tag::True.as_byte(),  // 0x01
                Tag::False.as_byte(), // 0x00
            ]
        );
    }

    #[test]
    fn encode_terminal_and_b_direct() {
        // and_b needs B + W children; W requires a:/s: wrappers (Task 2.10).
        // Encoder is type-blind, so we drive it directly with True/False.
        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::AndB(make_true_arc(), make_false_arc());
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(
            out,
            vec![
                Tag::AndB.as_byte(),  // 0x12
                Tag::True.as_byte(),  // 0x01
                Tag::False.as_byte(), // 0x00
            ]
        );
    }

    #[test]
    fn encode_terminal_or_b_direct() {
        // or_b needs E + W; W requires wrapping (Task 2.10). Drive directly.
        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::OrB(make_false_arc(), make_true_arc());
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(
            out,
            vec![
                Tag::OrB.as_byte(),   // 0x14
                Tag::False.as_byte(), // 0x00
                Tag::True.as_byte(),  // 0x01
            ]
        );
    }

    #[test]
    fn encode_terminal_or_c_direct() {
        // or_c needs B-du-unit + V; V needs v: wrapper (Task 2.10). Drive directly.
        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::OrC(make_false_arc(), make_true_arc());
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(
            out,
            vec![
                Tag::OrC.as_byte(),   // 0x15
                Tag::False.as_byte(), // 0x00
                Tag::True.as_byte(),  // 0x01
            ]
        );
    }

    #[test]
    fn encode_terminal_and_or_direct_three_children() {
        // and_or via direct construction. Confirms three children encode
        // in argument order (a, b, c) with the AndOr tag in front.
        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::AndOr(make_false_arc(), make_true_arc(), make_false_arc());
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(
            out,
            vec![
                Tag::AndOr.as_byte(), // 0x13
                Tag::False.as_byte(), // 0x00 (a)
                Tag::True.as_byte(),  // 0x01 (b)
                Tag::False.as_byte(), // 0x00 (c)
            ]
        );
    }

    #[test]
    fn encode_terminal_or_d_recurses_into_pk_k_children() {
        // Exercises the Arc forwarding impl on the immediate children of a
        // logical operator: each `Arc<Miniscript<...>>` child resolves
        // through Arc → Miniscript → Terminal → PkK with placeholder lookup.
        // (Arc forwarding is invoked once per child arc, not in a deeper
        // recursive chain — that depth comes from the trait dispatch chain
        // below it.) or_d(pk_k(K0), pk_k(K1)) — neither pk_k branch is
        // actually B-du-unit on its own (pk_k is K-type), so this would
        // not parse, but the encoder is type-blind.
        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);

        let left: Arc<Miniscript<DescriptorPublicKey, Segwitv0>> =
            Arc::new(Miniscript::from_ast(Terminal::PkK(k0)).unwrap());
        let right: Arc<Miniscript<DescriptorPublicKey, Segwitv0>> =
            Arc::new(Miniscript::from_ast(Terminal::PkK(k1)).unwrap());

        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::OrD(left, right);
        let mut out = Vec::new();
        term.encode_template(&mut out, &map).unwrap();

        // Expected: OrD, then for each PkK child:
        //   PkK tag, Placeholder tag, varint(idx).
        assert_eq!(
            out,
            vec![
                Tag::OrD.as_byte(),         // 0x16
                Tag::PkK.as_byte(),         // 0x1B
                Tag::Placeholder.as_byte(), // 0x32
                0x00,                       // idx 0
                Tag::PkK.as_byte(),         // 0x1B
                Tag::Placeholder.as_byte(), // 0x32
                0x01,                       // idx 1
            ]
        );
    }

    #[test]
    fn encode_terminal_after() {
        // After(LockTime) emits Tag::After + varint(consensus_u32).
        // 1234 in LEB128: 1234 = 0x4D2 = 0b100_1101_0010
        //   low 7 bits = 0b101_0010 = 0xD2, top bit set (continuation)
        //   high 7 bits = 0b000_1001 = 0x09, top bit clear (last)
        // → bytes [0xD2, 0x09]
        use miniscript::AbsLockTime;
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::After(AbsLockTime::from_consensus(1234).unwrap());
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(out, vec![Tag::After.as_byte(), 0xD2, 0x09]);
    }

    #[test]
    fn encode_terminal_after_small() {
        // After(127) — single LEB128 byte (0x7F).
        use miniscript::AbsLockTime;
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::After(AbsLockTime::from_consensus(127).unwrap());
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(out, vec![Tag::After.as_byte(), 0x7F]);
    }

    #[test]
    fn encode_terminal_older() {
        // Older(4032) — 28 days in blocks.
        // 4032 = 0xFC0 = 0b1111_1100_0000
        //   low 7 bits = 0b100_0000 = 0x40, continuation
        //   high 7 bits = 0b001_1111 = 0x1F, last
        // → bytes [0xC0, 0x1F]
        use miniscript::RelLockTime;
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::Older(RelLockTime::from_consensus(4032).unwrap());
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(out, vec![Tag::Older.as_byte(), 0xC0, 0x1F]);
    }

    #[test]
    fn encode_wsh_after_via_parser() {
        // wsh(after(1000)) — full pipeline.
        // 1000 LEB128: 1000 = 0x3E8, low 7 bits = 0x68 + continuation = 0xE8, high = 0x07.
        let d = parse_descriptor("wsh(after(1000))");
        let bytes = encode_template(&d, &empty_map()).unwrap();
        assert_eq!(
            bytes,
            vec![
                Tag::Wsh.as_byte(),   // 0x05
                Tag::After.as_byte(), // 0x1E
                0xE8,
                0x07, // varint(1000)
            ]
        );
    }

    #[test]
    fn encode_wsh_older_via_parser() {
        // wsh(older(144)) — 144 blocks (1 day).
        // 144 LEB128: 144 = 0x90, low 7 bits = 0x10 + continuation = 0x90, high = 0x01.
        let d = parse_descriptor("wsh(older(144))");
        let bytes = encode_template(&d, &empty_map()).unwrap();
        assert_eq!(
            bytes,
            vec![
                Tag::Wsh.as_byte(),   // 0x05
                Tag::Older.as_byte(), // 0x1F
                0x90,
                0x01, // varint(144)
            ]
        );
    }

    #[test]
    fn encode_terminal_thresh_2_of_3_with_constants() {
        // thresh(2, 0, 1, 0) -> Tag::Thresh + varint(2) + varint(3) + 3 children.
        // Threshold<Arc<Miniscript>, MAX> reuses the generic Threshold impl.
        use miniscript::Threshold;

        let zero: Arc<Miniscript<DescriptorPublicKey, Segwitv0>> =
            Arc::new(Miniscript::from_ast(Terminal::False).unwrap());
        let one: Arc<Miniscript<DescriptorPublicKey, Segwitv0>> =
            Arc::new(Miniscript::from_ast(Terminal::True).unwrap());

        let thresh = Threshold::new(2, vec![zero.clone(), one.clone(), zero]).unwrap();
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Thresh(thresh);
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();

        assert_eq!(
            out,
            vec![
                Tag::Thresh.as_byte(), // 0x18
                0x02,
                0x03,                 // k=2, n=3
                Tag::False.as_byte(), // 0x00
                Tag::True.as_byte(),  // 0x01
                Tag::False.as_byte(), // 0x00
            ]
        );
    }

    #[test]
    fn encode_terminal_sha256() {
        // Sha256(0x00..0x1F) — 32-byte hash with deterministic content.
        use bitcoin::hashes::{Hash, sha256};
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let bytes: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let h = sha256::Hash::from_byte_array(bytes);
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Sha256(h);
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();

        let mut expected = vec![Tag::Sha256.as_byte()];
        expected.extend_from_slice(&bytes);
        assert_eq!(out, expected);
        assert_eq!(out.len(), 33); // 1 tag + 32 hash bytes
    }

    #[test]
    fn encode_terminal_hash256() {
        // Terminal::Hash256 expects miniscript's hash256::Hash newtype
        // (a forwarded wrapper around bitcoin::hashes::sha256d::Hash), not
        // sha256d::Hash directly. The MiniscriptKey impl for
        // DescriptorPublicKey sets `type Hash256 = miniscript::hash256::Hash`.
        use bitcoin::hashes::Hash;
        use miniscript::Segwitv0;
        use miniscript::Terminal;
        use miniscript::hash256;

        // Asymmetric byte pattern (descending then ascending) — exposes any
        // accidental byte-order reversal that a [0xAA; 32] palindrome would mask.
        let bytes: [u8; 32] = [
            0x1f, 0x1e, 0x1d, 0x1c, 0x1b, 0x1a, 0x19, 0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12,
            0x11, 0x10, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
            0x0c, 0x0d, 0x0e, 0x0f,
        ];
        let h = hash256::Hash::from_byte_array(bytes);
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Hash256(h);
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();

        let mut expected = vec![Tag::Hash256.as_byte()];
        expected.extend_from_slice(&bytes);
        assert_eq!(out, expected);
        assert_eq!(out.len(), 33);
    }

    #[test]
    fn encode_terminal_ripemd160() {
        use bitcoin::hashes::{Hash, ripemd160};
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let bytes: [u8; 20] = [
            0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44,
            0x55, 0x66, 0x77, 0x88, 0x99, 0xAA,
        ];
        let h = ripemd160::Hash::from_byte_array(bytes);
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Ripemd160(h);
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();

        let mut expected = vec![Tag::Ripemd160.as_byte()];
        expected.extend_from_slice(&bytes);
        assert_eq!(out, expected);
        assert_eq!(out.len(), 21); // 1 tag + 20 hash bytes
    }

    #[test]
    fn encode_terminal_hash160() {
        use bitcoin::hashes::{Hash, hash160};
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        // Asymmetric pattern (matches the convention in the sha256/hash256
        // tests) — exposes any accidental byte-order reversal that a uniform
        // palindrome like [0x42; 20] would mask.
        let bytes: [u8; 20] = [
            0x13, 0x12, 0x11, 0x10, 0x0F, 0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 0x00, 0x01, 0x02, 0x03,
            0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
        ];
        let h = hash160::Hash::from_byte_array(bytes);
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Hash160(h);
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();

        let mut expected = vec![Tag::Hash160.as_byte()];
        expected.extend_from_slice(&bytes);
        assert_eq!(out, expected);
        assert_eq!(out.len(), 21);
    }

    #[test]
    fn encode_wsh_sha256_via_parser() {
        // wsh(sha256(<32-byte hex>)) — full pipeline. The hash literal is
        // a B-type fragment so wsh accepts it directly.
        let hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let d = parse_descriptor(&format!("wsh(sha256({hex}))"));
        let bytes = encode_template(&d, &empty_map()).unwrap();

        // Expected: Wsh + Sha256 + 32 bytes (last byte = 0x01, rest 0x00).
        let mut expected = vec![Tag::Wsh.as_byte(), Tag::Sha256.as_byte()];
        expected.extend(std::iter::repeat_n(0u8, 31));
        expected.push(0x01);
        assert_eq!(bytes, expected);
        assert_eq!(bytes.len(), 34); // Wsh + Sha256 + 32 bytes
    }

    #[test]
    fn encode_terminal_check() {
        // c:pk_k(K) — Check wrapping a PkK leaf. The 'pk(K)' descriptor parses
        // to Wsh -> Ms -> Check(PkK), so we exercise the parser path here.
        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 0u8);

        let d = parse_descriptor(&format!("wsh(pk({key}))"));
        let bytes = encode_template(&d, &map).unwrap();

        // Expected: Wsh + Check + PkK + Placeholder + idx(0)
        assert_eq!(
            bytes,
            vec![
                Tag::Wsh.as_byte(),         // 0x05
                Tag::Check.as_byte(),       // 0x0C
                Tag::PkK.as_byte(),         // 0x1B
                Tag::Placeholder.as_byte(), // 0x32
                0x00,                       // idx 0
            ]
        );
    }

    #[test]
    fn encode_terminal_verify_via_parser() {
        // v:pk(K) — Verify(Check(PkK(K))). Parser exposes via wsh(and_v(v:pk(K), 1)).
        // Since and_v(V, *) is B-typed, this parses as a wsh inner.
        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 0u8);

        let d = parse_descriptor(&format!("wsh(and_v(v:pk({key}),1))"));
        let bytes = encode_template(&d, &map).unwrap();

        // Expected: Wsh + AndV + Verify + Check + PkK + Placeholder + idx(0) + True
        assert_eq!(
            bytes,
            vec![
                Tag::Wsh.as_byte(),         // 0x05
                Tag::AndV.as_byte(),        // 0x11
                Tag::Verify.as_byte(),      // 0x0E
                Tag::Check.as_byte(),       // 0x0C
                Tag::PkK.as_byte(),         // 0x1B
                Tag::Placeholder.as_byte(), // 0x32
                0x00,                       // idx 0
                Tag::True.as_byte(),        // 0x01
            ]
        );
    }

    #[test]
    fn encode_terminal_alt_swap_directly() {
        // Direct construction tests for two wrappers that are harder to
        // parse-drive without a specific typing context.
        let true_ms: Arc<Miniscript<DescriptorPublicKey, Segwitv0>> =
            Arc::new(Miniscript::from_ast(Terminal::True).unwrap());

        let alt: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Alt(true_ms.clone());
        let mut out = Vec::new();
        alt.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(out, vec![Tag::Alt.as_byte(), Tag::True.as_byte()]);

        let swap: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Swap(true_ms.clone());
        let mut out = Vec::new();
        swap.encode_template(&mut out, &empty_map()).unwrap();
        assert_eq!(out, vec![Tag::Swap.as_byte(), Tag::True.as_byte()]);
    }

    #[test]
    fn encode_terminal_dup_if_non_zero_zero_not_equal_directly() {
        let true_ms: Arc<Miniscript<DescriptorPublicKey, Segwitv0>> =
            Arc::new(Miniscript::from_ast(Terminal::True).unwrap());

        for (term_ctor, tag) in [
            (Terminal::DupIf as fn(_) -> _, Tag::DupIf),
            (Terminal::NonZero as fn(_) -> _, Tag::NonZero),
            (Terminal::ZeroNotEqual as fn(_) -> _, Tag::ZeroNotEqual),
        ] {
            let term: Terminal<DescriptorPublicKey, Segwitv0> = term_ctor(true_ms.clone());
            let mut out = Vec::new();
            term.encode_template(&mut out, &empty_map()).unwrap();
            assert_eq!(out, vec![tag.as_byte(), Tag::True.as_byte()]);
        }
    }

    #[test]
    fn encode_terminal_raw_pk_h() {
        use bitcoin::hashes::{Hash, hash160};
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let bytes: [u8; 20] = [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
            0xFF, 0x00, 0x01, 0x02, 0x03, 0x04,
        ];
        let h = hash160::Hash::from_byte_array(bytes);
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::RawPkH(h);
        let mut out = Vec::new();
        term.encode_template(&mut out, &empty_map()).unwrap();

        let mut expected = vec![Tag::RawPkH.as_byte()]; // 0x1D
        expected.extend_from_slice(&bytes);
        assert_eq!(out, expected);
        assert_eq!(out.len(), 21); // 1 tag + 20 hash bytes
    }

    #[test]
    fn encode_placeholder_index_above_127_uses_single_byte() {
        // BIP §"LEB128 encoding": placeholder index is a 1-byte field,
        // not LEB128. For index < 128 the wire forms coincide; for ≥128
        // they diverge. Pin the single-byte form here so a regression
        // back to LEB128 (which would emit 2 bytes for 200) fails the test.
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let key = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(key.clone(), 200u8);

        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::PkK(key);
        let mut out = Vec::new();
        term.encode_template(&mut out, &map).unwrap();

        // Expected: Tag::PkK (0x1B), Tag::Placeholder (0x32), 0xC8 (= 200).
        // Under LEB128, 200 would emit as [0xC8, 0x01] — total 4 bytes.
        // Under single-byte, 200 emits as [0xC8] — total 3 bytes.
        assert_eq!(out, vec![0x1B, 0x32, 0xC8]);
        assert_eq!(
            out.len(),
            3,
            "single-byte placeholder index must be exactly 1 byte"
        );
    }

    // ---- Task 1.6: Encode-side restriction-matrix tests -------------------

    #[test]
    fn encode_rejects_sh_multi_legacy_p2sh() {
        // sh(multi(...)) uses ShInner::Ms — must be rejected with a message
        // mentioning "legacy P2SH".
        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let k2 = DescriptorPublicKey::from_str(
            "0395bcfdb728e8b1f0eda94f0db26d4ee3eebca73d11611ace1c0e4eed1bdc0e8a",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);
        map.insert(k2.clone(), 2u8);

        let d = parse_descriptor(&format!("sh(multi(2,{k0},{k1},{k2}))"));
        let err = encode_template(&d, &map).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("legacy P2SH")),
            "expected PolicyScopeViolation mentioning legacy P2SH, got: {err:?}"
        );
    }

    #[test]
    fn encode_rejects_sh_sortedmulti_legacy_p2sh() {
        // sh(sortedmulti(...)) also uses ShInner::Ms — must be rejected with
        // a message mentioning "legacy P2SH".
        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let k2 = DescriptorPublicKey::from_str(
            "0395bcfdb728e8b1f0eda94f0db26d4ee3eebca73d11611ace1c0e4eed1bdc0e8a",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);
        map.insert(k2.clone(), 2u8);

        let d = parse_descriptor(&format!("sh(sortedmulti(2,{k0},{k1},{k2}))"));
        let err = encode_template(&d, &map).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("legacy P2SH")),
            "expected PolicyScopeViolation mentioning legacy P2SH, got: {err:?}"
        );
    }

    #[test]
    fn encode_rejects_top_level_pkh() {
        // pkh() at the top level must be rejected with a message mentioning
        // "top-level pkh()".
        let d = parse_descriptor(
            "pkh(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("top-level pkh()")),
            "expected PolicyScopeViolation mentioning top-level pkh(), got: {err:?}"
        );
    }

    #[test]
    fn encode_rejects_top_level_bare() {
        // bare() at the top level must be rejected with a message mentioning
        // "bare()". Note: `pk(K)` parses as a Bare descriptor.
        let d = parse_descriptor(
            "pk(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("bare()")),
            "expected PolicyScopeViolation mentioning bare(), got: {err:?}"
        );
    }

    #[test]
    fn encode_rejects_sh_via_inner_ms_arbitrary_miniscript() {
        // sh(and_v(...)) — arbitrary non-multi sh-wrapped miniscript — uses
        // ShInner::Ms and must be rejected with a message mentioning "legacy P2SH".
        let k0 = DescriptorPublicKey::from_str(
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
        )
        .unwrap();
        let k1 = DescriptorPublicKey::from_str(
            "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
        )
        .unwrap();
        let mut map = HashMap::new();
        map.insert(k0.clone(), 0u8);
        map.insert(k1.clone(), 1u8);

        let d = parse_descriptor(&format!("sh(and_v(v:pk({k0}),pk({k1})))"));
        let err = encode_template(&d, &map).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("legacy P2SH")),
            "expected PolicyScopeViolation mentioning legacy P2SH, got: {err:?}"
        );
    }
}
