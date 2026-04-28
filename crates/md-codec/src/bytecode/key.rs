//! MdKey — the v0.1 representation of a key reference inside the canonical
//! bytecode.
//!
//! In v0.1 every key in a WDM-encoded BIP 388 wallet policy is a
//! [`MdKey::Placeholder`] referencing the policy's key information vector
//! at that index. The [`MdKey::Key`] variant is reserved for v1+ inline-key
//! support; v0.1 encoders MUST NOT emit it and v0.1 decoders MUST reject any
//! bytecode that would deserialize to it (those tags 0x24..=0x31 are the
//! `Reserved*` set in [`crate::bytecode::Tag`]).

use miniscript::descriptor::DescriptorPublicKey;

/// A key reference appearing in the canonical bytecode of a WDM wallet policy.
///
/// Marked `#[non_exhaustive]` so v1+ variants (e.g. inline-key extensions
/// or origin/fingerprint metadata) can be added without breaking downstream
/// `match` consumers. See decision D-2 in `design/PHASE_2_DECISIONS.md`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MdKey {
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
        let k = MdKey::Placeholder(0);
        assert_eq!(k, MdKey::Placeholder(0));
        assert_ne!(k, MdKey::Placeholder(1));
    }

    #[test]
    fn placeholder_clone_round_trip() {
        // Sanity check that the Clone derive does the right thing.
        let k = MdKey::Placeholder(42);
        let copy = k.clone();
        assert_eq!(k, copy);
    }
}
