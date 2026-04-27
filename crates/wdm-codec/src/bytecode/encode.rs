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
            other => Err(Error::PolicyScopeViolation(format!(
                "Terminal variant not yet implemented (Task 2.6+): {other:?}"
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
        // A terminal we haven't wired up yet (e.g. After) returns an
        // explicit "not yet implemented" PolicyScopeViolation.
        use miniscript::AbsLockTime;
        use miniscript::Segwitv0;
        use miniscript::Terminal;

        let term: Terminal<DescriptorPublicKey, Segwitv0> =
            Terminal::After(AbsLockTime::from_consensus(1234).unwrap());
        let mut out = Vec::new();
        let err = term.encode_template(&mut out, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(ref msg) if msg.contains("not yet implemented")),
            "expected not-yet-implemented PolicyScopeViolation, got: {err:?}"
        );
    }

    #[test]
    fn encode_wsh_sortedmulti_2_of_3() {
        // Three distinct keys, indexed 0, 1, 2 in placeholder_map.
        // wsh(sortedmulti(2, K0, K1, K2)) -> [Wsh, SortedMulti, varint(2),
        // varint(3), Placeholder, idx(0), Placeholder, idx(1), Placeholder, idx(2)]
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

        // sortedmulti() reorders keys lexicographically. Construct the descriptor
        // string with keys in the original order; the Descriptor parser will
        // sort them. The placeholder map must contain all three regardless.
        let d = parse_descriptor(&format!(
            "wsh(sortedmulti(2,{k0},{k1},{k2}))"
        ));
        let bytes = encode_template(&d, &map).unwrap();

        // Manually compute expected: tag bytes + varints. SortedMultiVec emits
        // keys in canonical order (lexicographic by the inner key bytes), so
        // we don't know the index order without inspecting the keys' sorted
        // order. We assert structural shape instead: starts with Wsh +
        // SortedMulti + varint(2) + varint(3), then 3 placeholder records.
        assert_eq!(bytes[0], Tag::Wsh.as_byte()); // 0x05
        assert_eq!(bytes[1], Tag::SortedMulti.as_byte()); // 0x09
        assert_eq!(bytes[2], 0x02); // varint(2)
        assert_eq!(bytes[3], 0x03); // varint(3)
        // Three placeholder records of 2 bytes each (Tag::Placeholder + idx).
        // 4 + 3*2 = 10 bytes total.
        assert_eq!(bytes.len(), 10);
        for i in 0..3 {
            assert_eq!(bytes[4 + 2 * i], Tag::Placeholder.as_byte()); // 0x32
            // bytes[5 + 2*i] is the LEB128-encoded index (0, 1, or 2).
            assert!(bytes[5 + 2 * i] <= 2);
        }
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
}
