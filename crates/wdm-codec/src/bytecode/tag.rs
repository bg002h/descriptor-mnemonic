//! Bytecode tag enum.

/// Single-byte tag identifying an operator in the canonical bytecode.
///
/// Values 0x00–0x31 are vendored verbatim from the descriptor-codec project
/// (joshdoman, CC0). Values 0x32–0x33 are WDM-specific extensions for BIP 388
/// placeholder framing and shared-path declarations.
///
/// Tag 0x35 (fingerprints block) is implemented in v0.2 (Phase E); the
/// fingerprints block follows the path declaration when the bytecode header's
/// fingerprints flag (bit 2 = 1) is set. See BIP §"Fingerprints block".
///
/// Marked `#[non_exhaustive]` so adding new variants in v0.2+ does not break
/// downstream `match` consumers. See PHASE_2_DECISIONS.md.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    /// `0` — miniscript terminal always-false fragment.
    False = 0x00,
    /// `1` — miniscript terminal always-true fragment.
    True = 0x01,
    /// `pkh(K)` — pay-to-pubkey-hash top-level descriptor.
    Pkh = 0x02,
    /// `sh(...)` — pay-to-script-hash top-level descriptor.
    Sh = 0x03,
    /// `wpkh(K)` — pay-to-witness-pubkey-hash top-level descriptor.
    Wpkh = 0x04,
    /// `wsh(...)` — pay-to-witness-script-hash top-level descriptor.
    Wsh = 0x05,
    /// `tr(...)` — taproot top-level descriptor.
    Tr = 0x06,
    /// `bare(...)` — bare miniscript top-level descriptor.
    Bare = 0x07,
    /// Taproot script tree node.
    TapTree = 0x08,
    /// `sorted_multi(k, ...)` — sorted multisig.
    SortedMulti = 0x09,
    /// `a:` wrapper — toaltstack/fromaltstack.
    Alt = 0x0A,
    /// `s:` wrapper — swap.
    Swap = 0x0B,
    /// `c:` wrapper — checksig.
    Check = 0x0C,
    /// `d:` wrapper — dup-if.
    DupIf = 0x0D,
    /// `v:` wrapper — verify.
    Verify = 0x0E,
    /// `j:` wrapper — non-zero.
    NonZero = 0x0F,
    /// `n:` wrapper — zero-not-equal.
    ZeroNotEqual = 0x10,
    /// `and_v(X, Y)` — verify-and conjunction.
    AndV = 0x11,
    /// `and_b(X, Y)` — boolean-and conjunction.
    AndB = 0x12,
    /// `andor(X, Y, Z)` — if X then Y else Z.
    AndOr = 0x13,
    /// `or_b(X, Z)` — boolean-or disjunction.
    OrB = 0x14,
    /// `or_c(X, Z)` — or-continue disjunction.
    OrC = 0x15,
    /// `or_d(X, Z)` — or-dup disjunction.
    OrD = 0x16,
    /// `or_i(X, Z)` — or-if disjunction.
    OrI = 0x17,
    /// `thresh(k, ...)` — k-of-n threshold over fragments.
    Thresh = 0x18,
    /// `multi(k, ...)` — k-of-n multisig.
    Multi = 0x19,
    /// `multi_a(k, ...)` — taproot k-of-n multisig.
    MultiA = 0x1A,
    /// `pk_k(K)` — bare-key key script.
    PkK = 0x1B,
    /// `pk_h(K)` — keyhash key script.
    PkH = 0x1C,
    /// `pk_h(<20-byte hash>)` — raw-pubkeyhash key script.
    RawPkH = 0x1D,
    /// `after(n)` — absolute timelock.
    After = 0x1E,
    /// `older(n)` — relative timelock.
    Older = 0x1F,
    /// `sha256(h)` — SHA-256 preimage commitment.
    Sha256 = 0x20,
    /// `hash256(h)` — double-SHA-256 preimage commitment.
    Hash256 = 0x21,
    /// `ripemd160(h)` — RIPEMD-160 preimage commitment.
    Ripemd160 = 0x22,
    /// `hash160(h)` — RIPEMD-160 of SHA-256 preimage commitment.
    Hash160 = 0x23,
    /// Reserved (descriptor-codec): key with origin info. Unused in v0.1.
    ReservedOrigin = 0x24,
    /// Reserved (descriptor-codec): key without origin info. Unused in v0.1.
    ReservedNoOrigin = 0x25,
    /// Reserved (descriptor-codec): uncompressed full public key. Unused in v0.1.
    ReservedUncompressedFullKey = 0x26,
    /// Reserved (descriptor-codec): compressed full public key. Unused in v0.1.
    ReservedCompressedFullKey = 0x27,
    /// Reserved (descriptor-codec): x-only public key. Unused in v0.1.
    ReservedXOnly = 0x28,
    /// Reserved (descriptor-codec): xpub. Unused in v0.1.
    ReservedXPub = 0x29,
    /// Reserved (descriptor-codec): multipath xpub. Unused in v0.1.
    ReservedMultiXPub = 0x2A,
    /// Reserved (descriptor-codec): uncompressed single private key. Unused in v0.1.
    ReservedUncompressedSinglePriv = 0x2B,
    /// Reserved (descriptor-codec): compressed single private key. Unused in v0.1.
    ReservedCompressedSinglePriv = 0x2C,
    /// Reserved (descriptor-codec): xpriv. Unused in v0.1.
    ReservedXPriv = 0x2D,
    /// Reserved (descriptor-codec): multipath xpriv. Unused in v0.1.
    ReservedMultiXPriv = 0x2E,
    /// Reserved (descriptor-codec): no-wildcard derivation suffix. Unused in v0.1.
    ReservedNoWildcard = 0x2F,
    /// Reserved (descriptor-codec): unhardened wildcard `/*`. Unused in v0.1.
    ReservedUnhardenedWildcard = 0x30,
    /// Reserved (descriptor-codec): hardened wildcard `/*'`. Unused in v0.1.
    ReservedHardenedWildcard = 0x31,
    /// WDM extension: BIP 388 key placeholder (`@i/<a;b>/*`).
    Placeholder = 0x32,
    /// WDM extension: shared-path declaration for placeholder framing.
    SharedPath = 0x33,
    /// WDM extension: fingerprints block (Phase E, v0.2).
    ///
    /// When the bytecode header's fingerprints flag (bit 2) is set, a
    /// fingerprints block of the form `[Tag::Fingerprints][count][4*count
    /// fingerprint bytes]` follows the path declaration and precedes the
    /// tree operators. See BIP §"Fingerprints block".
    Fingerprints = 0x35,
}

impl Tag {
    /// Convert a raw byte into a `Tag`. Returns `None` for unknown values.
    ///
    /// Implemented as an exhaustive `match` (rather than an unsafe transmute)
    /// so adding a future non-contiguous variant cannot introduce UB. See
    /// `design/PHASE_2_DECISIONS.md` D-1.
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(Tag::False),
            0x01 => Some(Tag::True),
            0x02 => Some(Tag::Pkh),
            0x03 => Some(Tag::Sh),
            0x04 => Some(Tag::Wpkh),
            0x05 => Some(Tag::Wsh),
            0x06 => Some(Tag::Tr),
            0x07 => Some(Tag::Bare),
            0x08 => Some(Tag::TapTree),
            0x09 => Some(Tag::SortedMulti),
            0x0A => Some(Tag::Alt),
            0x0B => Some(Tag::Swap),
            0x0C => Some(Tag::Check),
            0x0D => Some(Tag::DupIf),
            0x0E => Some(Tag::Verify),
            0x0F => Some(Tag::NonZero),
            0x10 => Some(Tag::ZeroNotEqual),
            0x11 => Some(Tag::AndV),
            0x12 => Some(Tag::AndB),
            0x13 => Some(Tag::AndOr),
            0x14 => Some(Tag::OrB),
            0x15 => Some(Tag::OrC),
            0x16 => Some(Tag::OrD),
            0x17 => Some(Tag::OrI),
            0x18 => Some(Tag::Thresh),
            0x19 => Some(Tag::Multi),
            0x1A => Some(Tag::MultiA),
            0x1B => Some(Tag::PkK),
            0x1C => Some(Tag::PkH),
            0x1D => Some(Tag::RawPkH),
            0x1E => Some(Tag::After),
            0x1F => Some(Tag::Older),
            0x20 => Some(Tag::Sha256),
            0x21 => Some(Tag::Hash256),
            0x22 => Some(Tag::Ripemd160),
            0x23 => Some(Tag::Hash160),
            0x24 => Some(Tag::ReservedOrigin),
            0x25 => Some(Tag::ReservedNoOrigin),
            0x26 => Some(Tag::ReservedUncompressedFullKey),
            0x27 => Some(Tag::ReservedCompressedFullKey),
            0x28 => Some(Tag::ReservedXOnly),
            0x29 => Some(Tag::ReservedXPub),
            0x2A => Some(Tag::ReservedMultiXPub),
            0x2B => Some(Tag::ReservedUncompressedSinglePriv),
            0x2C => Some(Tag::ReservedCompressedSinglePriv),
            0x2D => Some(Tag::ReservedXPriv),
            0x2E => Some(Tag::ReservedMultiXPriv),
            0x2F => Some(Tag::ReservedNoWildcard),
            0x30 => Some(Tag::ReservedUnhardenedWildcard),
            0x31 => Some(Tag::ReservedHardenedWildcard),
            0x32 => Some(Tag::Placeholder),
            0x33 => Some(Tag::SharedPath),
            0x35 => Some(Tag::Fingerprints),
            _ => None,
        }
    }

    /// The byte value of this tag.
    pub fn as_byte(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_round_trip_all_defined() {
        // 0x00–0x33 plus 0x35 are defined; 0x34 is reserved.
        for b in 0u8..=0x33 {
            let t = Tag::from_byte(b);
            assert!(t.is_some(), "byte {b:#04x} should be a valid tag");
            assert_eq!(t.unwrap().as_byte(), b);
        }
        let t = Tag::from_byte(0x35);
        assert!(
            t.is_some(),
            "byte 0x35 should be a valid tag (Fingerprints)"
        );
        assert_eq!(t.unwrap().as_byte(), 0x35);
    }

    #[test]
    fn tag_rejects_unknown_bytes() {
        // 0x34 is reserved; 0x35 is now Tag::Fingerprints (Phase E).
        // 0x36..=0xFF are reserved.
        assert!(
            Tag::from_byte(0x34).is_none(),
            "byte 0x34 should be rejected (reserved)"
        );
        for b in 0x36u8..=0xFF {
            assert!(
                Tag::from_byte(b).is_none(),
                "byte {b:#04x} should be rejected"
            );
        }
    }

    #[test]
    fn tag_specific_values() {
        assert_eq!(Tag::Wsh.as_byte(), 0x05);
        assert_eq!(Tag::PkK.as_byte(), 0x1B);
        assert_eq!(Tag::Sha256.as_byte(), 0x20);
        assert_eq!(Tag::Placeholder.as_byte(), 0x32);
        assert_eq!(Tag::SharedPath.as_byte(), 0x33);
        assert_eq!(Tag::Fingerprints.as_byte(), 0x35);
    }
}
