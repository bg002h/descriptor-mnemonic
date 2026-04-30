//! v0.11 Tag enum per spec §5.
//!
//! 31 ops in primary 5-bit space (0x00..0x1E) + extension prefix at 0x1F.
//! 5 ops in extension 10-bit space.

use crate::v11::bitstream::{BitReader, BitWriter};
use crate::v11::error::V11Error;

/// Operator tag identifying a descriptor/Miniscript fragment kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    /// `wpkh` — P2WPKH descriptor.
    Wpkh,
    /// `tr` — Taproot descriptor.
    Tr,
    /// `wsh` — P2WSH descriptor.
    Wsh,
    /// `sh` — P2SH descriptor.
    Sh,
    /// `pkh` — P2PKH descriptor.
    Pkh,
    /// Taproot tree node.
    TapTree,
    /// `multi` — k-of-n multisig.
    Multi,
    /// `sortedmulti` — sorted-key multisig.
    SortedMulti,
    /// `multi_a` — Tapscript multisig with `OP_CHECKSIGADD`.
    MultiA,
    /// `sortedmulti_a` — sorted-key Tapscript multisig.
    SortedMultiA,
    /// Miniscript `pk_k` — bare public key check.
    PkK,
    /// Miniscript `pk_h` — public-key-hash check.
    PkH,
    /// Miniscript `c:` wrapper (CHECKSIG).
    Check,
    /// Miniscript `v:` wrapper (VERIFY).
    Verify,
    /// Miniscript `s:` wrapper (SWAP).
    Swap,
    /// Miniscript `a:` wrapper (TOALTSTACK).
    Alt,
    /// Miniscript `d:` wrapper (DUPIF).
    DupIf,
    /// Miniscript `j:` wrapper (NONZERO).
    NonZero,
    /// Miniscript `n:` wrapper (ZERONOTEQUAL).
    ZeroNotEqual,
    /// Miniscript `and_v`.
    AndV,
    /// Miniscript `and_b`.
    AndB,
    /// Miniscript `andor`.
    AndOr,
    /// Miniscript `or_b`.
    OrB,
    /// Miniscript `or_c`.
    OrC,
    /// Miniscript `or_d`.
    OrD,
    /// Miniscript `or_i`.
    OrI,
    /// Miniscript `thresh`.
    Thresh,
    /// Miniscript `after` — absolute timelock.
    After,
    /// Miniscript `older` — relative timelock.
    Older,
    /// Miniscript `sha256`.
    Sha256,
    /// Miniscript `hash160`.
    Hash160,

    /// Miniscript `hash256` (extension space).
    Hash256,
    /// Miniscript `ripemd160` (extension space).
    Ripemd160,
    /// Raw public-key hash variant (extension space).
    RawPkH,
    /// Miniscript `0` literal (extension space).
    False,
    /// Miniscript `1` literal (extension space).
    True,
}

const EXTENSION_PREFIX: u8 = 0x1F;

impl Tag {
    /// Returns `(primary_code, extension_code_opt)`. If `primary_code` is
    /// `0x1F`, the second value is the 5-bit extension code; else it is `None`.
    pub(crate) fn codes(&self) -> (u8, Option<u8>) {
        match self {
            Tag::Wpkh => (0x00, None),
            Tag::Tr => (0x01, None),
            Tag::Wsh => (0x02, None),
            Tag::Sh => (0x03, None),
            Tag::Pkh => (0x04, None),
            Tag::TapTree => (0x05, None),
            Tag::Multi => (0x06, None),
            Tag::SortedMulti => (0x07, None),
            Tag::MultiA => (0x08, None),
            Tag::SortedMultiA => (0x09, None),
            Tag::PkK => (0x0A, None),
            Tag::PkH => (0x0B, None),
            Tag::Check => (0x0C, None),
            Tag::Verify => (0x0D, None),
            Tag::Swap => (0x0E, None),
            Tag::Alt => (0x0F, None),
            Tag::DupIf => (0x10, None),
            Tag::NonZero => (0x11, None),
            Tag::ZeroNotEqual => (0x12, None),
            Tag::AndV => (0x13, None),
            Tag::AndB => (0x14, None),
            Tag::AndOr => (0x15, None),
            Tag::OrB => (0x16, None),
            Tag::OrC => (0x17, None),
            Tag::OrD => (0x18, None),
            Tag::OrI => (0x19, None),
            Tag::Thresh => (0x1A, None),
            Tag::After => (0x1B, None),
            Tag::Older => (0x1C, None),
            Tag::Sha256 => (0x1D, None),
            Tag::Hash160 => (0x1E, None),
            Tag::Hash256 => (EXTENSION_PREFIX, Some(0x00)),
            Tag::Ripemd160 => (EXTENSION_PREFIX, Some(0x01)),
            Tag::RawPkH => (EXTENSION_PREFIX, Some(0x02)),
            Tag::False => (EXTENSION_PREFIX, Some(0x03)),
            Tag::True => (EXTENSION_PREFIX, Some(0x04)),
        }
    }

    /// Encode this tag (5 bits, plus 5 more if extension) into `w`.
    pub fn write(&self, w: &mut BitWriter) {
        let (primary, ext) = self.codes();
        w.write_bits(u64::from(primary), 5);
        if let Some(e) = ext {
            w.write_bits(u64::from(e), 5);
        }
    }

    /// Decode a tag from `r`, consuming 5 bits (or 10 for extension).
    pub fn read(r: &mut BitReader) -> Result<Self, V11Error> {
        let primary = r.read_bits(5)? as u8;
        if primary == EXTENSION_PREFIX {
            let ext = r.read_bits(5)? as u8;
            match ext {
                0x00 => Ok(Tag::Hash256),
                0x01 => Ok(Tag::Ripemd160),
                0x02 => Ok(Tag::RawPkH),
                0x03 => Ok(Tag::False),
                0x04 => Ok(Tag::True),
                _ => Err(V11Error::UnknownExtensionTag(ext)),
            }
        } else {
            match primary {
                0x00 => Ok(Tag::Wpkh),
                0x01 => Ok(Tag::Tr),
                0x02 => Ok(Tag::Wsh),
                0x03 => Ok(Tag::Sh),
                0x04 => Ok(Tag::Pkh),
                0x05 => Ok(Tag::TapTree),
                0x06 => Ok(Tag::Multi),
                0x07 => Ok(Tag::SortedMulti),
                0x08 => Ok(Tag::MultiA),
                0x09 => Ok(Tag::SortedMultiA),
                0x0A => Ok(Tag::PkK),
                0x0B => Ok(Tag::PkH),
                0x0C => Ok(Tag::Check),
                0x0D => Ok(Tag::Verify),
                0x0E => Ok(Tag::Swap),
                0x0F => Ok(Tag::Alt),
                0x10 => Ok(Tag::DupIf),
                0x11 => Ok(Tag::NonZero),
                0x12 => Ok(Tag::ZeroNotEqual),
                0x13 => Ok(Tag::AndV),
                0x14 => Ok(Tag::AndB),
                0x15 => Ok(Tag::AndOr),
                0x16 => Ok(Tag::OrB),
                0x17 => Ok(Tag::OrC),
                0x18 => Ok(Tag::OrD),
                0x19 => Ok(Tag::OrI),
                0x1A => Ok(Tag::Thresh),
                0x1B => Ok(Tag::After),
                0x1C => Ok(Tag::Older),
                0x1D => Ok(Tag::Sha256),
                0x1E => Ok(Tag::Hash160),
                _ => Err(V11Error::UnknownPrimaryTag(primary)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(t: Tag) {
        let mut w = BitWriter::new();
        t.write(&mut w);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert_eq!(Tag::read(&mut r).unwrap(), t);
    }

    #[test] fn tag_wpkh() { round_trip(Tag::Wpkh); }
    #[test] fn tag_tr() { round_trip(Tag::Tr); }
    #[test] fn tag_taptree() { round_trip(Tag::TapTree); }
    #[test] fn tag_thresh() { round_trip(Tag::Thresh); }
    #[test] fn tag_hash256_extension() { round_trip(Tag::Hash256); }
    #[test] fn tag_false_extension() { round_trip(Tag::False); }
    #[test] fn tag_true_extension() { round_trip(Tag::True); }

    #[test]
    fn tag_unknown_extension_rejected() {
        let mut w = BitWriter::new();
        w.write_bits(0x1F, 5);
        w.write_bits(0x05, 5);
        let bytes = w.into_bytes();
        let mut r = BitReader::new(&bytes);
        assert!(matches!(Tag::read(&mut r), Err(V11Error::UnknownExtensionTag(0x05))));
    }
}
