//! Bytecode template encoder for WDM wallet policies.
//!
//! Walks a `Descriptor<DescriptorPublicKey>` and emits the canonical bytecode
//! used by Template Cards. Key positions are replaced by `Tag::Placeholder`
//! followed by an LEB128-encoded index into the wallet policy's key
//! information vector (caller supplies the index map).
//!
//! v0.1 scope: only `wsh(...)` top-level descriptors are accepted. Sh, Pkh,
//! Wpkh, Tr, and Bare descriptors are rejected with `PolicyScopeViolation`.
//! Inline keys (any key not present in `placeholder_map`) are also rejected.
//!
//! Architecture mirrors `joshdoman/descriptor-codec` (CC0) — see
//! `design/PHASE_2_DECISIONS.md` D-4 for rationale.

use std::collections::HashMap;
use std::sync::Arc;

use bitcoin::hashes::Hash;
use miniscript::descriptor::{Descriptor, DescriptorPublicKey, SortedMultiVec, Wsh, WshInner};
use miniscript::{Miniscript, Segwitv0, Terminal, Threshold};

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
            Descriptor::Sh(_) => Err(Error::PolicyScopeViolation(
                "v0.1 does not support sh() — use wsh()".to_string(),
            )),
            Descriptor::Pkh(_) => Err(Error::PolicyScopeViolation(
                "v0.1 does not support pkh() — use wsh(pk_h(...))".to_string(),
            )),
            Descriptor::Wpkh(_) => Err(Error::PolicyScopeViolation(
                "v0.1 does not support wpkh() — use wsh(pk_h(...))".to_string(),
            )),
            Descriptor::Tr(_) => Err(Error::PolicyScopeViolation(
                "v0.1 does not support taproot tr() — deferred to v0.2".to_string(),
            )),
            Descriptor::Bare(_) => Err(Error::PolicyScopeViolation(
                "v0.1 does not support bare() descriptors".to_string(),
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

impl EncodeTemplate for WshInner<DescriptorPublicKey> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        match self {
            WshInner::Ms(ms) => ms.encode_template(out, placeholder_map),
            WshInner::SortedMulti(sortedmulti) => sortedmulti.encode_template(out, placeholder_map),
        }
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
            Terminal::MultiA(thresh) => {
                // Taproot multi-A. v0.1 doesn't support taproot, but the encoding
                // shape is the same as Multi; ship the arm in case Tr is enabled
                // later. For v0.1 it can never trigger because Descriptor::Tr is
                // rejected at the top level, so this is effectively dead code today.
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
            other => Err(Error::PolicyScopeViolation(format!(
                "unsupported Terminal fragment in v0.1 scope: {other:?}"
            ))),
        }
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
        crate::bytecode::varint::encode_u64(u64::from(index), out);
        Ok(())
    }
}

impl<T: EncodeTemplate, const MAX: usize> EncodeTemplate for Threshold<T, MAX> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        crate::bytecode::varint::encode_u64(self.k() as u64, out);
        crate::bytecode::varint::encode_u64(self.n() as u64, out);
        for elem in self.iter() {
            elem.encode_template(out, placeholder_map)?;
        }
        Ok(())
    }
}

impl EncodeTemplate for SortedMultiVec<DescriptorPublicKey, Segwitv0> {
    fn encode_template(
        &self,
        out: &mut Vec<u8>,
        placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        out.push(Tag::SortedMulti.as_byte());
        crate::bytecode::varint::encode_u64(self.k() as u64, out);
        crate::bytecode::varint::encode_u64(self.n() as u64, out);
        for pk in self.pks() {
            pk.encode_template(out, placeholder_map)?;
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
    fn rejects_wpkh_top_level() {
        let d = parse_descriptor(
            "wpkh(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(msg) if msg.contains("wpkh")),
            "expected PolicyScopeViolation mentioning wpkh"
        );
    }

    #[test]
    fn rejects_sh_top_level() {
        let d = parse_descriptor(
            "sh(wpkh(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5))",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(msg) if msg.contains("sh")),
            "expected PolicyScopeViolation mentioning sh"
        );
    }

    #[test]
    fn rejects_tr_top_level() {
        // Key-path-only taproot descriptor (no script tree).
        let d = parse_descriptor(
            "tr(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(msg) if msg.contains("taproot")),
            "expected PolicyScopeViolation mentioning taproot"
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

    #[test]
    fn encode_unsupported_terminal_returns_error() {
        // A terminal we haven't wired up yet (e.g. Verify wrapper, Task 2.10)
        // returns an out-of-v0.1-scope PolicyScopeViolation via the catch-all
        // arm. After/Older were moved out of the catch-all in Task 2.8.
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let inner: Arc<Miniscript<DescriptorPublicKey, Segwitv0>> =
            Arc::new(Miniscript::from_ast(Terminal::True).unwrap());
        let term: Terminal<DescriptorPublicKey, Segwitv0> = Terminal::Verify(inner);
        let mut out = Vec::new();
        let err = term.encode_template(&mut out, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("unsupported Terminal")),
            "expected unsupported-Terminal PolicyScopeViolation, got: {err:?}"
        );
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

        let d = parse_descriptor(&format!(
            "wsh(sortedmulti(2,{k0},{k1},{k2}))"
        ));
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
                Tag::Wsh.as_byte(),    // 0x05
                Tag::After.as_byte(),  // 0x1E
                0xE8, 0x07,            // varint(1000)
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
                Tag::Wsh.as_byte(),    // 0x05
                Tag::Older.as_byte(),  // 0x1F
                0x90, 0x01,            // varint(144)
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
                0x02, 0x03,            // k=2, n=3
                Tag::False.as_byte(),  // 0x00
                Tag::True.as_byte(),   // 0x01
                Tag::False.as_byte(),  // 0x00
            ]
        );
    }

    #[test]
    fn encode_terminal_sha256() {
        // Sha256(0x00..0x1F) — 32-byte hash with deterministic content.
        use bitcoin::hashes::{sha256, Hash};
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let bytes: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
            0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
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

        let bytes: [u8; 32] = [0xAB; 32];
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
        use bitcoin::hashes::{ripemd160, Hash};
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let bytes: [u8; 20] = [
            0xCD, 0xEF, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC,
            0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88, 0x99, 0xAA,
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
        use bitcoin::hashes::{hash160, Hash};
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let bytes: [u8; 20] = [0x42; 20];
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
}
