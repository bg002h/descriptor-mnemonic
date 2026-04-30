//! Bytecode tag enum.

/// Single-byte tag identifying an operator in the canonical bytecode.
///
/// v0.6 layout: descriptor-codec-vendored allocations dropped in favor
/// of a coherent grouping. `Reserved*` range 0x24-0x31 dropped entirely
/// (MD's BIP-388 framing forbids inline keys; see
/// `design/MD_SCOPE_DECISION_2026-04-28.md`). `Tag::Bare` dropped
/// (never used as inner; encoder rejects `Descriptor::Bare` via
/// `PolicyScopeViolation`). `Tag::SortedMultiA` (0x0B) NEW; needed for
/// tap-context sorted multisig shapes documented by Coldcard and Ledger.
///
/// Byte 0x32 is intentionally left unallocated to surface v0.5→v0.6
/// transcoder mistakes as clean `from_byte=None` rather than data
/// corruption (v0.5 emitted `Placeholder=0x32` in every encoded MD string).
///
/// Marked `#[non_exhaustive]` so adding new variants in v0.7+ does not
/// break downstream `match` consumers.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    // Constants
    /// `0` — miniscript terminal always-false fragment.
    False = 0x00,
    /// `1` — miniscript terminal always-true fragment.
    True = 0x01,

    // Top-level descriptor wrappers
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

    // Tap-tree framing
    /// Taproot script tree inner-node framing (inside `tr(KEY, TREE)`).
    TapTree = 0x07,

    // Multisig family
    /// `multi(k, ...)` — k-of-n multisig (P2WSH-only by miniscript typing).
    Multi = 0x08,
    /// `sortedmulti(k, ...)` — sorted multisig (P2WSH-only by miniscript typing).
    SortedMulti = 0x09,
    /// `multi_a(k, ...)` — taproot k-of-n multisig (Tapscript-only by miniscript typing).
    MultiA = 0x0A,
    /// `sortedmulti_a(k, ...)` — taproot sorted multisig (Tapscript-only).
    /// NEW in v0.6.
    SortedMultiA = 0x0B,

    // Wrappers
    /// `a:` wrapper — toaltstack/fromaltstack.
    Alt = 0x0C,
    /// `s:` wrapper — swap.
    Swap = 0x0D,
    /// `c:` wrapper — checksig.
    Check = 0x0E,
    /// `d:` wrapper — dup-if.
    DupIf = 0x0F,
    /// `v:` wrapper — verify.
    Verify = 0x10,
    /// `j:` wrapper — non-zero.
    NonZero = 0x11,
    /// `n:` wrapper — zero-not-equal.
    ZeroNotEqual = 0x12,

    // Logical operators
    /// `and_v(X, Y)` — verify-and conjunction.
    AndV = 0x13,
    /// `and_b(X, Y)` — boolean-and conjunction.
    AndB = 0x14,
    /// `andor(X, Y, Z)` — if X then Y else Z.
    AndOr = 0x15,
    /// `or_b(X, Z)` — boolean-or disjunction.
    OrB = 0x16,
    /// `or_c(X, Z)` — or-continue disjunction.
    OrC = 0x17,
    /// `or_d(X, Z)` — or-dup disjunction.
    OrD = 0x18,
    /// `or_i(X, Z)` — or-if disjunction.
    OrI = 0x19,
    /// `thresh(k, ...)` — k-of-n threshold over fragments.
    Thresh = 0x1A,

    // Keys (byte values unchanged from v0.5)
    /// `pk_k(K)` — bare-key key script.
    PkK = 0x1B,
    /// `pk_h(K)` — keyhash key script.
    PkH = 0x1C,
    /// `pk_h(<20-byte hash>)` — raw-pubkeyhash key script.
    RawPkH = 0x1D,

    // Timelocks (byte values unchanged from v0.5)
    /// `after(n)` — absolute timelock.
    After = 0x1E,
    /// `older(n)` — relative timelock.
    Older = 0x1F,

    // Hashes (byte values unchanged from v0.5)
    /// `sha256(h)` — SHA-256 preimage commitment.
    Sha256 = 0x20,
    /// `hash256(h)` — double-SHA-256 preimage commitment.
    Hash256 = 0x21,
    /// `ripemd160(h)` — RIPEMD-160 preimage commitment.
    Ripemd160 = 0x22,
    /// `hash160(h)` — RIPEMD-160 of SHA-256 preimage commitment.
    Hash160 = 0x23,

    // Reserved (0x24-0x31): DROPPED in v0.6 — see crate-level rationale.
    // Byte 0x32: DROPPED — was v0.5 Placeholder; intentionally unallocated.

    // MD-specific framing (Placeholder + SharedPath bytes shifted +1 from v0.5
    // to leave 0x32 unallocated; Fingerprints byte unchanged from v0.5)
    /// MD extension: BIP 388 key placeholder (`@i/<a;b>/*`).
    Placeholder = 0x33,
    /// MD extension: shared-path declaration for placeholder framing.
    SharedPath = 0x34,
    /// MD extension: fingerprints block (Phase E v0.2).
    ///
    /// Byte value preserved across v0.5→v0.6 for wire-format continuity
    /// of the v0.2-shipped fingerprints framing.
    Fingerprints = 0x35,
    /// MD extension: per-`@N` origin paths block. NEW in v0.10.
    ///
    /// Present only when at least one placeholder's origin path diverges
    /// from the implicit shared-path framing; signaled by header bit 3.
    OriginPaths = 0x36,
}

impl Tag {
    /// Convert a raw byte into a `Tag`. Returns `None` for unknown values.
    ///
    /// Implemented as an exhaustive `match` (rather than an unsafe transmute)
    /// so adding a future non-contiguous variant cannot introduce UB.
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            // Constants
            0x00 => Some(Tag::False),
            0x01 => Some(Tag::True),
            // Top-level descriptor wrappers
            0x02 => Some(Tag::Pkh),
            0x03 => Some(Tag::Sh),
            0x04 => Some(Tag::Wpkh),
            0x05 => Some(Tag::Wsh),
            0x06 => Some(Tag::Tr),
            // Tap-tree framing
            0x07 => Some(Tag::TapTree),
            // Multisig family
            0x08 => Some(Tag::Multi),
            0x09 => Some(Tag::SortedMulti),
            0x0A => Some(Tag::MultiA),
            0x0B => Some(Tag::SortedMultiA),
            // Wrappers
            0x0C => Some(Tag::Alt),
            0x0D => Some(Tag::Swap),
            0x0E => Some(Tag::Check),
            0x0F => Some(Tag::DupIf),
            0x10 => Some(Tag::Verify),
            0x11 => Some(Tag::NonZero),
            0x12 => Some(Tag::ZeroNotEqual),
            // Logical operators
            0x13 => Some(Tag::AndV),
            0x14 => Some(Tag::AndB),
            0x15 => Some(Tag::AndOr),
            0x16 => Some(Tag::OrB),
            0x17 => Some(Tag::OrC),
            0x18 => Some(Tag::OrD),
            0x19 => Some(Tag::OrI),
            0x1A => Some(Tag::Thresh),
            // Keys
            0x1B => Some(Tag::PkK),
            0x1C => Some(Tag::PkH),
            0x1D => Some(Tag::RawPkH),
            // Timelocks
            0x1E => Some(Tag::After),
            0x1F => Some(Tag::Older),
            // Hashes
            0x20 => Some(Tag::Sha256),
            0x21 => Some(Tag::Hash256),
            0x22 => Some(Tag::Ripemd160),
            0x23 => Some(Tag::Hash160),
            // 0x24-0x32: unallocated (Reserved* dropped, Bare dropped, Placeholder moved)
            // MD-specific framing
            0x33 => Some(Tag::Placeholder),
            0x34 => Some(Tag::SharedPath),
            0x35 => Some(Tag::Fingerprints),
            0x36 => Some(Tag::OriginPaths),
            // 0x37-0xFF: reserved
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
    fn tag_v0_6_layout_top_level_descriptors() {
        assert_eq!(Tag::False.as_byte(), 0x00);
        assert_eq!(Tag::True.as_byte(), 0x01);
        assert_eq!(Tag::Pkh.as_byte(), 0x02);
        assert_eq!(Tag::Sh.as_byte(), 0x03);
        assert_eq!(Tag::Wpkh.as_byte(), 0x04);
        assert_eq!(Tag::Wsh.as_byte(), 0x05);
        assert_eq!(Tag::Tr.as_byte(), 0x06);
    }

    #[test]
    fn tag_v0_6_layout_taptree_framing() {
        assert_eq!(Tag::TapTree.as_byte(), 0x07);
    }

    #[test]
    fn tag_v0_6_layout_multisig_family_contiguous() {
        assert_eq!(Tag::Multi.as_byte(), 0x08);
        assert_eq!(Tag::SortedMulti.as_byte(), 0x09);
        assert_eq!(Tag::MultiA.as_byte(), 0x0A);
        assert_eq!(Tag::SortedMultiA.as_byte(), 0x0B);
    }

    #[test]
    fn tag_v0_6_layout_wrappers() {
        assert_eq!(Tag::Alt.as_byte(), 0x0C);
        assert_eq!(Tag::Swap.as_byte(), 0x0D);
        assert_eq!(Tag::Check.as_byte(), 0x0E);
        assert_eq!(Tag::DupIf.as_byte(), 0x0F);
        assert_eq!(Tag::Verify.as_byte(), 0x10);
        assert_eq!(Tag::NonZero.as_byte(), 0x11);
        assert_eq!(Tag::ZeroNotEqual.as_byte(), 0x12);
    }

    #[test]
    fn tag_v0_6_layout_logical() {
        assert_eq!(Tag::AndV.as_byte(), 0x13);
        assert_eq!(Tag::AndB.as_byte(), 0x14);
        assert_eq!(Tag::AndOr.as_byte(), 0x15);
        assert_eq!(Tag::OrB.as_byte(), 0x16);
        assert_eq!(Tag::OrC.as_byte(), 0x17);
        assert_eq!(Tag::OrD.as_byte(), 0x18);
        assert_eq!(Tag::OrI.as_byte(), 0x19);
        assert_eq!(Tag::Thresh.as_byte(), 0x1A);
    }

    #[test]
    fn tag_v0_6_layout_keys_unchanged() {
        assert_eq!(Tag::PkK.as_byte(), 0x1B);
        assert_eq!(Tag::PkH.as_byte(), 0x1C);
        assert_eq!(Tag::RawPkH.as_byte(), 0x1D);
    }

    #[test]
    fn tag_v0_6_layout_timelocks_unchanged() {
        assert_eq!(Tag::After.as_byte(), 0x1E);
        assert_eq!(Tag::Older.as_byte(), 0x1F);
    }

    #[test]
    fn tag_v0_6_layout_hashes_unchanged() {
        assert_eq!(Tag::Sha256.as_byte(), 0x20);
        assert_eq!(Tag::Hash256.as_byte(), 0x21);
        assert_eq!(Tag::Ripemd160.as_byte(), 0x22);
        assert_eq!(Tag::Hash160.as_byte(), 0x23);
    }

    #[test]
    fn tag_v0_6_layout_framing() {
        assert_eq!(Tag::Placeholder.as_byte(), 0x33);
        assert_eq!(Tag::SharedPath.as_byte(), 0x34);
        assert_eq!(Tag::Fingerprints.as_byte(), 0x35);
    }

    #[test]
    fn tag_origin_paths_byte_position() {
        assert_eq!(Tag::OriginPaths.as_byte(), 0x36);
        assert_eq!(Tag::from_byte(0x36), Some(Tag::OriginPaths));
    }

    #[test]
    fn tag_v0_10_unallocated_starts_at_0x37() {
        for b in 0x37..=0xFF_u8 {
            assert!(
                Tag::from_byte(b).is_none(),
                "byte {b:#04x} should be unallocated"
            );
        }
    }

    #[test]
    fn tag_v0_6_unallocated_bytes() {
        // Reserved* range dropped (was 0x24-0x31 in v0.5)
        for b in 0x24u8..=0x31 {
            assert!(
                Tag::from_byte(b).is_none(),
                "byte {b:#04x} should be unallocated in v0.6 (Reserved* range dropped)"
            );
        }
        // Byte 0x32 (formerly Placeholder in v0.5) intentionally vacant
        assert!(
            Tag::from_byte(0x32).is_none(),
            "byte 0x32 should be unallocated in v0.6 (formerly Placeholder, intentionally vacant per spec §2.2)"
        );
    }

    #[test]
    fn tag_v0_10_high_bytes_unallocated() {
        // 0x37..=0xFF reserved (0x36 reclaimed in v0.10 as Tag::OriginPaths)
        for b in 0x37u8..=0xFF {
            assert!(
                Tag::from_byte(b).is_none(),
                "byte {b:#04x} should be unallocated"
            );
        }
    }

    #[test]
    fn tag_round_trip_all_defined() {
        // v0.10 allocation: 0x00-0x23 contiguous, then 0x33-0x36.
        // Gap: 0x24-0x32 (Reserved* dropped + Bare dropped + Placeholder moved up).
        let v0_10_allocated: Vec<u8> = (0x00..=0x23).chain(0x33..=0x36).collect();
        for b in v0_10_allocated {
            let t = Tag::from_byte(b);
            assert!(t.is_some(), "byte {b:#04x} should be a valid v0.10 tag");
            assert_eq!(t.unwrap().as_byte(), b);
        }
    }

    #[test]
    fn tag_rejects_unknown_bytes() {
        // 0x24-0x32: dropped in v0.6 (Reserved* + Bare + Placeholder moved up)
        for b in 0x24u8..=0x32 {
            assert!(
                Tag::from_byte(b).is_none(),
                "byte {b:#04x} should be rejected (v0.10 unallocated)"
            );
        }
        // 0x37-0xFF: high range reserved (0x36 reclaimed in v0.10)
        for b in 0x37u8..=0xFF {
            assert!(
                Tag::from_byte(b).is_none(),
                "byte {b:#04x} should be rejected (high range reserved)"
            );
        }
    }

    #[test]
    fn tag_specific_values() {
        assert_eq!(Tag::Wsh.as_byte(), 0x05);
        assert_eq!(Tag::PkK.as_byte(), 0x1B);
        assert_eq!(Tag::Sha256.as_byte(), 0x20);
        assert_eq!(Tag::Placeholder.as_byte(), 0x33);
        assert_eq!(Tag::SharedPath.as_byte(), 0x34);
        assert_eq!(Tag::Fingerprints.as_byte(), 0x35);
        // v0.6 NEW
        assert_eq!(Tag::SortedMultiA.as_byte(), 0x0B);
        assert_eq!(Tag::TapTree.as_byte(), 0x07);
    }
}
