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

use miniscript::descriptor::{Descriptor, DescriptorPublicKey};

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

impl EncodeTemplate for miniscript::descriptor::Wsh<DescriptorPublicKey> {
    fn encode_template(
        &self,
        _out: &mut Vec<u8>,
        _placeholder_map: &HashMap<DescriptorPublicKey, u8>,
    ) -> Result<(), Error> {
        // TODO Task 2.5+: dispatch into the inner WshInner (SortedMulti or Ms).
        Err(Error::PolicyScopeViolation(
            "wsh() inner encoding not yet implemented (Task 2.5+)".to_string(),
        ))
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
    fn wsh_skeleton_returns_inner_not_implemented_error() {
        // Wsh top-level dispatches to the inner walker, which currently returns
        // the Task 2.5+ stub error. The Wsh tag byte is still pushed before
        // the inner returns Err — that's the controller's choice here, since
        // the next tasks will replace the inner stub.
        let d = parse_descriptor(
            "wsh(pk(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5))",
        );
        let err = encode_template(&d, &empty_map()).unwrap_err();
        assert!(
            matches!(err, Error::PolicyScopeViolation(msg) if msg.contains("wsh") || msg.contains("Task 2.5")),
            "expected PolicyScopeViolation mentioning wsh inner or Task 2.5"
        );
    }
}
