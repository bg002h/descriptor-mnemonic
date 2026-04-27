//! WdmKey — the v0.1 representation of a key reference inside the canonical
//! bytecode.
//!
//! In v0.1 every key in a WDM-encoded BIP 388 wallet policy is a
//! [`WdmKey::Placeholder(u8)`] referencing the policy's key information vector
//! at that index. The [`WdmKey::Key`] variant is reserved for v1+ inline-key
//! support; v0.1 encoders MUST NOT emit it and v0.1 decoders MUST reject any
//! bytecode that would deserialize to it (those tags 0x24..=0x31 are the
//! `Reserved*` set in [`crate::bytecode::Tag`]).

use miniscript::descriptor::DescriptorPublicKey;

/// A key reference appearing in the canonical bytecode of a WDM wallet policy.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WdmKey {
    /// BIP 388 placeholder reference (`@i`) into the wallet policy's key
    /// information vector at index `i`.
    Placeholder(u8),
    /// Inline descriptor public key. Reserved for v1+; v0.1 encoders MUST NOT
    /// emit and v0.1 decoders MUST reject.
    Key(DescriptorPublicKey),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_equality_and_inequality() {
        let k = WdmKey::Placeholder(0);
        assert_eq!(k, WdmKey::Placeholder(0));
        assert_ne!(k, WdmKey::Placeholder(1));
    }

    #[test]
    fn placeholder_clone_round_trip() {
        // Sanity check that the Clone derive does the right thing.
        let k = WdmKey::Placeholder(42);
        let copy = k.clone();
        assert_eq!(k, copy);
    }
}
