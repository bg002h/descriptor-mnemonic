//! #file location crates/md-codec/src/derive.rs

//! Address derivation per md1 v0.14.
//!
//! Given a decoded [`Descriptor`] in wallet-policy mode (`Pubkeys` TLV
//! populated), derive the receive or change address at a given
//! `(chain, index)` pair.
//!
//! ### What this module does
//!
//! - Classifies the descriptor's wrapper shape against the BIP 388
//!   "default" wallet-policy set (the same five shapes
//!   [`crate::canonical_origin::canonical_origin`] recognizes).
//! - Reconstructs an [`Xpub`] per `@N` from the 65-byte
//!   `chain_code || compressed_pubkey` Pubkeys TLV payload (BIP 32
//!   metadata fields like `depth`/`parent_fingerprint`/`child_number`
//!   are not used by `CKDpub` and are filled with placeholders).
//! - Walks the use-site path forward — the multipath alternative for
//!   `chain`, then `address_index` as the trailing wildcard child —
//!   to produce a per-`@N` derived public key.
//! - Renders the final address by wrapper shape: P2PKH, P2WPKH, P2TR
//!   (BIP 86 NUMS tweak), P2WSH, or P2SH-P2WSH (with hand-rolled
//!   multisig script for the multi/sortedmulti cases).
//!
//! ### What this module does NOT do
//!
//! - Origin path is not consulted. Origin is the path *to* the xpub
//!   from the master seed; address derivation starts at the xpub. The
//!   recorded origin matters for signing flows (PSBT key-source
//!   metadata), not for getting an address.
//! - Master fingerprint (`Fingerprints` TLV) is unused for the same
//!   reason — it identifies the master, not the derivation root.
//! - Hardened use-site components are rejected. Hardened public
//!   derivation is forbidden by BIP 32; an xpub-only restore cannot
//!   produce addresses for a wallet whose use-site path has a
//!   hardened alternative or hardened wildcard.
//! - Non-default wrapper shapes (`tr(@N, TapTree)`, `sh(sortedmulti)`
//!   legacy, miniscript bodies, bare `wsh(@N)`/`sh(@N)`) are not
//!   supported. They round-trip through encode/decode fine; address
//!   derivation just refuses them with
//!   [`Error::UnsupportedDerivationShape`].

use crate::canonical_origin::is_wsh_inner_multi;
use crate::canonicalize::expand_per_at_n;
use crate::encode::Descriptor;
use crate::error::Error;
use crate::tag::Tag;
use crate::tree::{Body, Node};
use crate::use_site_path::UseSitePath;
use bitcoin::address::NetworkUnchecked;
use bitcoin::bip32::{ChainCode, ChildNumber, Fingerprint, Xpub};
use bitcoin::secp256k1::{PublicKey, Secp256k1, Verification};
use bitcoin::{Address, CompressedPublicKey, Network, NetworkKind, ScriptBuf};

/// Wrapper shapes the v0.14 derivation API supports. Mirrors
/// [`crate::canonical_origin::canonical_origin`]'s classification but
/// extracts threshold + sorted bits so the script builder doesn't
/// re-walk the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DerivableShape {
    /// `pkh(@0)` single-key.
    Pkh,
    /// `wpkh(@0)` single-key.
    Wpkh,
    /// `tr(@0)` key-path only (no `TapTree`); BIP 86 NUMS tweak.
    TrKeyPathOnly,
    /// `wsh(multi)` or `wsh(sortedmulti)`.
    WshMulti {
        /// Multisig threshold.
        k: u8,
        /// `true` for `sortedmulti`, `false` for `multi`.
        sorted: bool,
    },
    /// `sh(wsh(multi))` or `sh(wsh(sortedmulti))`.
    ShWshMulti {
        /// Multisig threshold.
        k: u8,
        /// `true` for `sortedmulti`, `false` for `multi`.
        sorted: bool,
    },
}

/// Classify a tree's top-level wrapper into a [`DerivableShape`].
///
/// Returns [`Error::UnsupportedDerivationShape`] for any shape not in
/// the v0.14 supported set (including `tr(@N, TapTree)`, legacy
/// `sh(sortedmulti)`, miniscript bodies, and bare `wsh(@N)`/`sh(@N)`).
fn classify_derivable_shape(tree: &Node) -> Result<DerivableShape, Error> {
    match (&tree.tag, &tree.body) {
        (Tag::Pkh, Body::KeyArg { .. }) => Ok(DerivableShape::Pkh),
        (Tag::Wpkh, Body::KeyArg { .. }) => Ok(DerivableShape::Wpkh),
        (Tag::Tr, Body::Tr { tree: None, .. }) => Ok(DerivableShape::TrKeyPathOnly),
        (Tag::Wsh, Body::Children(children))
            if children.len() == 1 && is_wsh_inner_multi(children[0].tag) =>
        {
            let (k, sorted) = multi_threshold_and_sort(&children[0])?;
            Ok(DerivableShape::WshMulti { k, sorted })
        }
        (Tag::Sh, Body::Children(children)) if children.len() == 1 => {
            let inner = &children[0];
            if inner.tag == Tag::Wsh {
                if let Body::Children(grand) = &inner.body {
                    if grand.len() == 1 && is_wsh_inner_multi(grand[0].tag) {
                        let (k, sorted) = multi_threshold_and_sort(&grand[0])?;
                        return Ok(DerivableShape::ShWshMulti { k, sorted });
                    }
                }
            }
            Err(Error::UnsupportedDerivationShape)
        }
        _ => Err(Error::UnsupportedDerivationShape),
    }
}

/// Extract `(k, sorted)` from a `multi` or `sortedmulti` node.
fn multi_threshold_and_sort(node: &Node) -> Result<(u8, bool), Error> {
    let sorted = match node.tag {
        Tag::Multi => false,
        Tag::SortedMulti => true,
        _ => return Err(Error::UnsupportedDerivationShape),
    };
    match &node.body {
        Body::Variable { k, .. } => Ok((*k, sorted)),
        _ => Err(Error::UnsupportedDerivationShape),
    }
}

/// Reconstruct an [`Xpub`] from a 65-byte `Pubkeys` TLV payload.
///
/// Layout: `bytes[0..32]` = chain code; `bytes[32..65]` = compressed
/// public key. The four BIP 32 metadata fields (`network`, `depth`,
/// `parent_fingerprint`, `child_number`) are not used by
/// [`Xpub::derive_pub`] (only `chain_code` and `public_key` participate
/// in `CKDpub`); they are filled with safe placeholders.
fn xpub_from_tlv_bytes(idx: u8, bytes: &[u8; 65]) -> Result<Xpub, Error> {
    let chain_code_bytes: [u8; 32] = bytes[0..32]
        .try_into()
        .expect("32-byte slice is statically sized");
    let chain_code = ChainCode::from(chain_code_bytes);
    let public_key =
        PublicKey::from_slice(&bytes[32..65]).map_err(|_| Error::InvalidXpubBytes { idx })?;
    Ok(Xpub {
        network: NetworkKind::Main,
        depth: 0,
        parent_fingerprint: Fingerprint::default(),
        child_number: ChildNumber::Normal { index: 0 },
        public_key,
        chain_code,
    })
}

/// Walk the use-site path forward from `xpub`: pick the multipath
/// alternative at index `chain`, then derive the wildcard child at
/// `address_index`. Both steps must be non-hardened (BIP 32
/// constraint on public-key derivation).
fn derive_use_site_pubkey<C: Verification>(
    xpub: &Xpub,
    use_site: &UseSitePath,
    chain: u32,
    address_index: u32,
    secp: &Secp256k1<C>,
) -> Result<PublicKey, Error> {
    let mut intermediate = *xpub;
    if let Some(alts) = &use_site.multipath {
        if (chain as usize) >= alts.len() {
            return Err(Error::ChainIndexOutOfRange {
                chain,
                alt_count: alts.len(),
            });
        }
        let alt = alts[chain as usize];
        if alt.hardened {
            return Err(Error::HardenedPublicDerivation);
        }
        intermediate = intermediate
            .derive_pub(secp, &[ChildNumber::Normal { index: alt.value }])
            .map_err(|_| Error::HardenedPublicDerivation)?;
    } else if chain != 0 {
        return Err(Error::ChainIndexOutOfRange {
            chain,
            alt_count: 0,
        });
    }
    if use_site.wildcard_hardened {
        return Err(Error::HardenedPublicDerivation);
    }
    let leaf = intermediate
        .derive_pub(secp, &[ChildNumber::Normal { index: address_index }])
        .map_err(|_| Error::HardenedPublicDerivation)?;
    Ok(leaf.public_key)
}

/// Build the redeem script for `multi(k, ...)` / `sortedmulti(k, ...)`:
/// `<k> <pk1> <pk2> ... <pkn> <n> OP_CHECKMULTISIG`.
///
/// When `sorted` is true, public keys are sorted lexicographically by
/// their compressed (33-byte) serialization per BIP 67.
fn build_multi_script(k: u8, pubkeys: &[PublicKey], sorted: bool) -> ScriptBuf {
    use bitcoin::opcodes::all::OP_CHECKMULTISIG;
    let mut keys: Vec<CompressedPublicKey> =
        pubkeys.iter().copied().map(CompressedPublicKey).collect();
    if sorted {
        keys.sort_by_key(|c| c.0.serialize());
    }
    let mut b = bitcoin::blockdata::script::Builder::new().push_int(k as i64);
    for key in &keys {
        b = b.push_key(&bitcoin::PublicKey::from(*key));
    }
    b.push_int(keys.len() as i64)
        .push_opcode(OP_CHECKMULTISIG)
        .into_script()
}

impl Descriptor {
    /// Derive the address at `(chain, index)` for this descriptor on
    /// `network`.
    ///
    /// `chain` selects the use-site multipath alternative (e.g. `0` =
    /// receive, `1` = change for the standard `<0;1>/*` form).
    /// `index` is the trailing wildcard child number.
    ///
    /// Returns an [`Address<NetworkUnchecked>`]; callers can
    /// `.assume_checked()` (when they trust the network parameter) or
    /// `.require_network(network)` to lock it down.
    ///
    /// # Errors
    ///
    /// - [`Error::UnsupportedDerivationShape`] when the wrapper shape
    ///   is outside the v0.14 supported set (see module docs).
    /// - [`Error::MissingPubkey`] when any `@N` lacks an xpub.
    /// - [`Error::InvalidXpubBytes`] when an xpub's 33-byte pubkey
    ///   field doesn't parse as a valid secp256k1 point.
    /// - [`Error::ChainIndexOutOfRange`] when `chain` is out of range
    ///   for the use-site multipath.
    /// - [`Error::HardenedPublicDerivation`] when the use-site path
    ///   requires a hardened derivation step.
    /// - [`Error::MissingExplicitOrigin`] propagated from
    ///   [`expand_per_at_n`] for non-canonical wrappers without an
    ///   explicit origin path. (Such descriptors should already have
    ///   been rejected by [`crate::decode::decode_payload`]; this is
    ///   defense in depth for hand-built `Descriptor` values.)
    ///
    /// # Origin path
    ///
    /// The recorded origin path is **not** consulted by this method.
    /// Origin describes how the xpub was derived from master; address
    /// derivation starts at the xpub itself. Origin matters for
    /// signing flows (PSBT key-source metadata) but not for rendering
    /// an address.
    pub fn derive_address(
        &self,
        chain: u32,
        index: u32,
        network: Network,
    ) -> Result<Address<NetworkUnchecked>, Error> {
        let shape = classify_derivable_shape(&self.tree)?;
        let expanded = expand_per_at_n(self)?;

        let secp = Secp256k1::verification_only();
        let mut pubkeys: Vec<PublicKey> = Vec::with_capacity(expanded.len());
        for e in &expanded {
            let xpub_bytes = e.xpub.ok_or(Error::MissingPubkey { idx: e.idx })?;
            let xpub = xpub_from_tlv_bytes(e.idx, &xpub_bytes)?;
            let pk = derive_use_site_pubkey(&xpub, &e.use_site_path, chain, index, &secp)?;
            pubkeys.push(pk);
        }

        let addr = match shape {
            DerivableShape::Pkh => {
                let cpk = CompressedPublicKey(pubkeys[0]);
                Address::p2pkh(cpk, network).into_unchecked()
            }
            DerivableShape::Wpkh => {
                let cpk = CompressedPublicKey(pubkeys[0]);
                Address::p2wpkh(&cpk, network).into_unchecked()
            }
            DerivableShape::TrKeyPathOnly => {
                let xonly = bitcoin::secp256k1::XOnlyPublicKey::from(pubkeys[0]);
                Address::p2tr(&secp, xonly, None, network).into_unchecked()
            }
            DerivableShape::WshMulti { k, sorted } => {
                let script = build_multi_script(k, &pubkeys, sorted);
                Address::p2wsh(&script, network).into_unchecked()
            }
            DerivableShape::ShWshMulti { k, sorted } => {
                let script = build_multi_script(k, &pubkeys, sorted);
                Address::p2shwsh(&script, network).into_unchecked()
            }
        };
        Ok(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::origin_path::{OriginPath, PathComponent, PathDecl, PathDeclPaths};
    use crate::tag::Tag;
    use crate::tlv::TlvSection;
    use crate::tree::{Body, Node};
    use crate::use_site_path::{Alternative, UseSitePath};

    fn pkk(index: u8) -> Node {
        Node {
            tag: Tag::PkK,
            body: Body::KeyArg { index },
        }
    }

    // ─── classify_derivable_shape ─────────────────────────────────────

    #[test]
    fn classify_pkh() {
        let n = Node {
            tag: Tag::Pkh,
            body: Body::KeyArg { index: 0 },
        };
        assert_eq!(classify_derivable_shape(&n).unwrap(), DerivableShape::Pkh);
    }

    #[test]
    fn classify_wpkh() {
        let n = Node {
            tag: Tag::Wpkh,
            body: Body::KeyArg { index: 0 },
        };
        assert_eq!(classify_derivable_shape(&n).unwrap(), DerivableShape::Wpkh);
    }

    #[test]
    fn classify_tr_keypath_only() {
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr { key_index: 0, tree: None },
        };
        assert_eq!(
            classify_derivable_shape(&n).unwrap(),
            DerivableShape::TrKeyPathOnly
        );
    }

    #[test]
    fn classify_tr_with_taptree_rejected() {
        let n = Node {
            tag: Tag::Tr,
            body: Body::Tr {
                key_index: 0,
                tree: Some(Box::new(pkk(1))),
            },
        };
        assert!(matches!(
            classify_derivable_shape(&n),
            Err(Error::UnsupportedDerivationShape)
        ));
    }

    #[test]
    fn classify_wsh_multi() {
        let n = Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::Multi,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(0), pkk(1), pkk(2)],
                },
            }]),
        };
        assert_eq!(
            classify_derivable_shape(&n).unwrap(),
            DerivableShape::WshMulti { k: 2, sorted: false }
        );
    }

    #[test]
    fn classify_wsh_sortedmulti() {
        let n = Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(0), pkk(1), pkk(2)],
                },
            }]),
        };
        assert_eq!(
            classify_derivable_shape(&n).unwrap(),
            DerivableShape::WshMulti { k: 2, sorted: true }
        );
    }

    #[test]
    fn classify_sh_wsh_sortedmulti() {
        let n = Node {
            tag: Tag::Sh,
            body: Body::Children(vec![Node {
                tag: Tag::Wsh,
                body: Body::Children(vec![Node {
                    tag: Tag::SortedMulti,
                    body: Body::Variable {
                        k: 2,
                        children: vec![pkk(0), pkk(1), pkk(2)],
                    },
                }]),
            }]),
        };
        assert_eq!(
            classify_derivable_shape(&n).unwrap(),
            DerivableShape::ShWshMulti { k: 2, sorted: true }
        );
    }

    #[test]
    fn classify_legacy_sh_sortedmulti_rejected() {
        let n = Node {
            tag: Tag::Sh,
            body: Body::Children(vec![Node {
                tag: Tag::SortedMulti,
                body: Body::Variable {
                    k: 2,
                    children: vec![pkk(0), pkk(1)],
                },
            }]),
        };
        assert!(matches!(
            classify_derivable_shape(&n),
            Err(Error::UnsupportedDerivationShape)
        ));
    }

    #[test]
    fn classify_bare_wsh_at_n_rejected() {
        let n = Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![pkk(0)]),
        };
        assert!(matches!(
            classify_derivable_shape(&n),
            Err(Error::UnsupportedDerivationShape)
        ));
    }

    #[test]
    fn classify_miniscript_body_rejected() {
        let n = Node {
            tag: Tag::Wsh,
            body: Body::Children(vec![Node {
                tag: Tag::OrD,
                body: Body::Children(vec![pkk(0), pkk(1)]),
            }]),
        };
        assert!(matches!(
            classify_derivable_shape(&n),
            Err(Error::UnsupportedDerivationShape)
        ));
    }

    #[test]
    fn classify_bare_pkk_root_rejected() {
        let n = pkk(0);
        assert!(matches!(
            classify_derivable_shape(&n),
            Err(Error::UnsupportedDerivationShape)
        ));
    }

    // ─── xpub_from_tlv_bytes ─────────────────────────────────────────

    #[test]
    fn xpub_from_tlv_bytes_rejects_invalid_pubkey() {
        // 33 zero bytes is not a valid compressed pubkey.
        let bytes = [0u8; 65];
        assert!(matches!(
            xpub_from_tlv_bytes(7, &bytes),
            Err(Error::InvalidXpubBytes { idx: 7 })
        ));
    }

    // ─── derive_use_site_pubkey ──────────────────────────────────────

    /// Round-trip: derive a child via `derive_use_site_pubkey` and via
    /// raw `Xpub::derive_pub`; assert equality. Uses G as a placeholder
    /// pubkey + zero chain code (still parses).
    fn test_xpub() -> Xpub {
        // Use the secp256k1 generator G as a known-valid compressed point.
        // 0x02 || x(G) (compressed, even-Y prefix).
        let mut pubkey_bytes = [0u8; 33];
        pubkey_bytes[0] = 0x02;
        let g_x: [u8; 32] = [
            0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87,
            0x0B, 0x07, 0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B,
            0x16, 0xF8, 0x17, 0x98,
        ];
        pubkey_bytes[1..].copy_from_slice(&g_x);
        let mut bytes = [0u8; 65];
        bytes[32..].copy_from_slice(&pubkey_bytes);
        // Non-zero chain code so derive_pub produces meaningful output.
        bytes[..32].copy_from_slice(&[0x42; 32]);
        xpub_from_tlv_bytes(0, &bytes).unwrap()
    }

    #[test]
    fn derive_rejects_hardened_alt() {
        let xpub = test_xpub();
        let secp = Secp256k1::verification_only();
        let usp = UseSitePath {
            multipath: Some(vec![
                Alternative { hardened: true, value: 0 },
                Alternative { hardened: false, value: 1 },
            ]),
            wildcard_hardened: false,
        };
        assert!(matches!(
            derive_use_site_pubkey(&xpub, &usp, 0, 0, &secp),
            Err(Error::HardenedPublicDerivation)
        ));
    }

    #[test]
    fn derive_rejects_hardened_wildcard() {
        let xpub = test_xpub();
        let secp = Secp256k1::verification_only();
        let usp = UseSitePath {
            multipath: Some(vec![
                Alternative { hardened: false, value: 0 },
                Alternative { hardened: false, value: 1 },
            ]),
            wildcard_hardened: true,
        };
        assert!(matches!(
            derive_use_site_pubkey(&xpub, &usp, 0, 0, &secp),
            Err(Error::HardenedPublicDerivation)
        ));
    }

    #[test]
    fn derive_rejects_chain_out_of_range() {
        let xpub = test_xpub();
        let secp = Secp256k1::verification_only();
        let usp = UseSitePath::standard_multipath();
        assert!(matches!(
            derive_use_site_pubkey(&xpub, &usp, 5, 0, &secp),
            Err(Error::ChainIndexOutOfRange { chain: 5, alt_count: 2 })
        ));
    }

    #[test]
    fn derive_no_multipath_only_chain_zero() {
        let xpub = test_xpub();
        let secp = Secp256k1::verification_only();
        let usp = UseSitePath {
            multipath: None,
            wildcard_hardened: false,
        };
        // chain=0 ok
        assert!(derive_use_site_pubkey(&xpub, &usp, 0, 0, &secp).is_ok());
        // chain=1 rejected
        assert!(matches!(
            derive_use_site_pubkey(&xpub, &usp, 1, 0, &secp),
            Err(Error::ChainIndexOutOfRange { chain: 1, alt_count: 0 })
        ));
    }

    // ─── build_multi_script ──────────────────────────────────────────

    /// Build two distinguishable test pubkeys (G and 2*G effectively —
    /// just two distinct valid points).
    fn two_distinct_pubkeys() -> (PublicKey, PublicKey) {
        let secp = Secp256k1::verification_only();
        let mut bytes_a = [0u8; 65];
        bytes_a[0..32].copy_from_slice(&[0x11; 32]);
        bytes_a[32] = 0x02;
        bytes_a[33..].copy_from_slice(&[
            0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87,
            0x0B, 0x07, 0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B,
            0x16, 0xF8, 0x17, 0x98,
        ]);
        let xpub_a = xpub_from_tlv_bytes(0, &bytes_a).unwrap();
        let pk_a = xpub_a
            .derive_pub(&secp, &[ChildNumber::Normal { index: 0 }])
            .unwrap()
            .public_key;
        let pk_b = xpub_a
            .derive_pub(&secp, &[ChildNumber::Normal { index: 1 }])
            .unwrap()
            .public_key;
        (pk_a, pk_b)
    }

    #[test]
    fn multi_script_unsorted_preserves_input_order() {
        let (a, b) = two_distinct_pubkeys();
        let s1 = build_multi_script(2, &[a, b], false);
        let s2 = build_multi_script(2, &[b, a], false);
        // Different input order, different unsorted scripts.
        assert_ne!(s1, s2);
    }

    #[test]
    fn multi_script_sorted_is_input_order_independent() {
        let (a, b) = two_distinct_pubkeys();
        let s1 = build_multi_script(2, &[a, b], true);
        let s2 = build_multi_script(2, &[b, a], true);
        // Sorted form is input-order-independent.
        assert_eq!(s1, s2);
    }

    #[test]
    fn multi_script_sorted_matches_bip67_order() {
        let (a, b) = two_distinct_pubkeys();
        let sorted_pks = {
            let mut v = vec![a, b];
            v.sort_by_key(|p| p.serialize());
            v
        };
        let from_unsorted = build_multi_script(2, &sorted_pks, false);
        let from_sorted = build_multi_script(2, &[a, b], true);
        assert_eq!(from_unsorted, from_sorted);
    }

    // ─── Descriptor::derive_address — error paths ────────────────────

    fn bip84_origin() -> OriginPath {
        OriginPath {
            components: vec![
                PathComponent { hardened: true, value: 84 },
                PathComponent { hardened: true, value: 0 },
                PathComponent { hardened: true, value: 0 },
            ],
        }
    }

    fn one_test_xpub_bytes() -> [u8; 65] {
        let mut bytes = [0u8; 65];
        bytes[0..32].copy_from_slice(&[0x42; 32]);
        bytes[32] = 0x02;
        bytes[33..].copy_from_slice(&[
            0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87,
            0x0B, 0x07, 0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B,
            0x16, 0xF8, 0x17, 0x98,
        ]);
        bytes
    }

    #[test]
    fn derive_address_missing_pubkey_for_partial_keys() {
        // 2-of-2 wsh-sortedmulti with only @0 populated.
        let d = Descriptor {
            n: 2,
            path_decl: PathDecl {
                n: 2,
                paths: PathDeclPaths::Shared(OriginPath {
                    components: vec![
                        PathComponent { hardened: true, value: 48 },
                        PathComponent { hardened: true, value: 0 },
                        PathComponent { hardened: true, value: 0 },
                        PathComponent { hardened: true, value: 2 },
                    ],
                }),
            },
            use_site_path: UseSitePath::standard_multipath(),
            tree: Node {
                tag: Tag::Wsh,
                body: Body::Children(vec![Node {
                    tag: Tag::SortedMulti,
                    body: Body::Variable {
                        k: 2,
                        children: vec![pkk(0), pkk(1)],
                    },
                }]),
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.pubkeys = Some(vec![(0u8, one_test_xpub_bytes())]);
                t
            },
        };
        let err = d.derive_address(0, 0, Network::Bitcoin).unwrap_err();
        assert!(matches!(err, Error::MissingPubkey { idx: 1 }));
    }

    #[test]
    fn derive_address_unsupported_shape() {
        // tr(@0, TapTree) — has script tree, derivation rejects.
        let d = Descriptor {
            n: 1,
            path_decl: PathDecl {
                n: 1,
                paths: PathDeclPaths::Shared(OriginPath {
                    components: vec![PathComponent { hardened: true, value: 99 }],
                }),
            },
            use_site_path: UseSitePath::standard_multipath(),
            tree: Node {
                tag: Tag::Tr,
                body: Body::Tr {
                    key_index: 0,
                    tree: Some(Box::new(pkk(0))),
                },
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.pubkeys = Some(vec![(0u8, one_test_xpub_bytes())]);
                t
            },
        };
        let err = d.derive_address(0, 0, Network::Bitcoin).unwrap_err();
        assert!(matches!(err, Error::UnsupportedDerivationShape));
    }

    #[test]
    fn derive_address_chain_out_of_range() {
        let d = Descriptor {
            n: 1,
            path_decl: PathDecl {
                n: 1,
                paths: PathDeclPaths::Shared(bip84_origin()),
            },
            use_site_path: UseSitePath::standard_multipath(), // alt-count=2
            tree: Node {
                tag: Tag::Wpkh,
                body: Body::KeyArg { index: 0 },
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.pubkeys = Some(vec![(0u8, one_test_xpub_bytes())]);
                t
            },
        };
        let err = d.derive_address(5, 0, Network::Bitcoin).unwrap_err();
        assert!(matches!(
            err,
            Error::ChainIndexOutOfRange { chain: 5, alt_count: 2 }
        ));
    }

    #[test]
    fn derive_address_hardened_wildcard_rejected() {
        let d = Descriptor {
            n: 1,
            path_decl: PathDecl {
                n: 1,
                paths: PathDeclPaths::Shared(bip84_origin()),
            },
            use_site_path: UseSitePath {
                multipath: Some(vec![
                    Alternative { hardened: false, value: 0 },
                    Alternative { hardened: false, value: 1 },
                ]),
                wildcard_hardened: true,
            },
            tree: Node {
                tag: Tag::Wpkh,
                body: Body::KeyArg { index: 0 },
            },
            tlv: {
                let mut t = TlvSection::new_empty();
                t.pubkeys = Some(vec![(0u8, one_test_xpub_bytes())]);
                t
            },
        };
        let err = d.derive_address(0, 0, Network::Bitcoin).unwrap_err();
        assert!(matches!(err, Error::HardenedPublicDerivation));
    }
}
