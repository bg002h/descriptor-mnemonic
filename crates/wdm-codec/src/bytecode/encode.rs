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

use miniscript::descriptor::{Descriptor, DescriptorPublicKey, Wsh, WshInner};
use miniscript::{Miniscript, Segwitv0, Terminal};

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
            WshInner::SortedMulti(_) => Err(Error::PolicyScopeViolation(
                "sortedmulti() encoding not yet implemented (Task 2.6)".to_string(),
            )),
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
                encode_key(key, out, placeholder_map)
            }
            Terminal::PkH(key) => {
                out.push(Tag::PkH.as_byte());
                encode_key(key, out, placeholder_map)
            }
            other => Err(Error::PolicyScopeViolation(format!(
                "Terminal variant not yet implemented (Task 2.6+): {other:?}"
            ))),
        }
    }
}

/// Emit a `DescriptorPublicKey` as a placeholder reference.
///
/// Looks up `key` in `placeholder_map` and writes `Tag::Placeholder`
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
/// a with-origin form (or vice versa) will see this function report
/// "inline key" even though the bare key bytes match. Real BIP 388 wallet
/// policies always carry origin metadata; ensure your map is built from
/// the same parsed descriptor instance.
fn encode_key(
    key: &DescriptorPublicKey,
    out: &mut Vec<u8>,
    placeholder_map: &HashMap<DescriptorPublicKey, u8>,
) -> Result<(), Error> {
    let &index = placeholder_map.get(key).ok_or_else(|| {
        Error::PolicyScopeViolation(format!(
            "inline key not in placeholder_map (v0.1 forbids inline keys): {key}"
        ))
    })?;
    out.push(Tag::Placeholder.as_byte());
    crate::bytecode::varint::encode_u64(u64::from(index), out);
    Ok(())
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
        // pushed Tag::PkK before encode_key returned Err, so out is non-empty.
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
}
