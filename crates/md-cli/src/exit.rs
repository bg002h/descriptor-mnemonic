//! The constellation exit codes (`SPEC_constellation_exit_codes.md` §3, in
//! mnemonic-engrave's `design/`).
//!
//! A local copy: it mirrors the `exit` constants and bit-set of
//! `mnemonic-io-lib` until that crate is published with them (spec decision
//! 4); then this module gives way to the shared one. The test below pins the
//! numbers to the spec.
//!
//! Codes 1 to 63 are sums of the six bits; 64 and 101 never combine; 130 is an
//! operator stop and 128+N a signal (the shell's, not md's).

/// Everything requested was done and every check passed.
pub const OK: u8 = 0;
/// The input is well-formed, and a comparison against something else came
/// out no (a verify mismatch, NOT spend-equal).
pub const NO: u8 = 1;
/// An input is wrong for this verb whatever the flags.
pub const INVALID: u8 = 2;
/// The tool checked part of the result and could not confirm it.
pub const UNVERIFIED: u8 = 4;
/// The tool declined under a safety or policy rule a flag, network, channel
/// or destination would waive, or a network mismatch.
pub const REFUSED: u8 = 8;
/// A checksum correction was applied to an input and the corrected value used.
pub const REPAIRED: u8 = 16;
/// Something needed from the system failed or is missing (a file, stdout, a
/// feature this build lacks).
pub const IO: u8 = 32;
/// The command line alone is wrong; nothing ran. Never combines.
pub const USAGE: u8 = 64;
/// A bug, or any failure no class covers. Never combines.
pub const INTERNAL: u8 = 101;

/// The root `--help` legend (spec R11), word for word.
pub const LEGEND: &str = "Exit status: 0 = done, every check passed. 64 = bad command line, \
nothing ran. 101 = internal error. 130 = stopped by the operator. Other codes from 128 up = \
killed by a signal. Otherwise the sum (1 to 63) of: 1 a check said no; 2 an input is invalid; \
4 verify the result before trusting it; 8 refused (network mismatch, secret on the command \
line, unsafe output); 16 a correction was applied; 32 a file, device or helper failed.";

/// A set of the six combinable bits (1 to 32).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Bits(u8);

impl Bits {
    /// No bit set (exit 0).
    #[cfg_attr(not(test), allow(dead_code))]
    pub const NONE: Bits = Bits(0);

    /// The set holding `bit` (one of [`NO`] .. [`IO`], or a sum of them).
    pub const fn of(bit: u8) -> Bits {
        Bits(bit & 63)
    }

    /// True when every bit of `bit` is set.
    #[cfg_attr(not(test), allow(dead_code))]
    pub const fn contains(self, bit: u8) -> bool {
        self.0 & bit == bit
    }

    /// The process exit status: the sum of the bits.
    pub const fn to_exit_code(self) -> u8 {
        self.0
    }
}

impl std::ops::BitOr for Bits {
    type Output = Bits;
    fn bitor(self, rhs: Bits) -> Bits {
        Bits(self.0 | rhs.0)
    }
}

impl std::ops::BitOr<u8> for Bits {
    type Output = Bits;
    fn bitor(self, rhs: u8) -> Bits {
        Bits(self.0 | (rhs & 63))
    }
}

impl std::ops::BitOrAssign<u8> for Bits {
    fn bitor_assign(&mut self, rhs: u8) {
        self.0 |= rhs & 63;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The numbers of spec §3. A change here is a change to the constellation
    /// contract, not to md alone.
    #[test]
    fn the_codes_are_the_specs() {
        assert_eq!(
            [
                OK, NO, INVALID, UNVERIFIED, REFUSED, REPAIRED, IO, USAGE, INTERNAL
            ],
            [0, 1, 2, 4, 8, 16, 32, 64, 101]
        );
    }

    #[test]
    fn bits_sum_and_never_reach_the_single_codes() {
        let b = Bits::of(REPAIRED) | UNVERIFIED | Bits::of(INVALID);
        assert_eq!(b.to_exit_code(), 22);
        assert!(b.contains(REPAIRED) && b.contains(UNVERIFIED) && b.contains(INVALID));
        assert!(!b.contains(NO));
        let mut all = Bits::NONE;
        for bit in [NO, INVALID, UNVERIFIED, REFUSED, REPAIRED, IO] {
            all |= bit;
        }
        assert_eq!(all.to_exit_code(), 63);
        // 64 is not a bit: `of` cannot smuggle it into a sum.
        assert_eq!(Bits::of(USAGE).to_exit_code(), 0);
    }

    #[test]
    fn the_legend_is_the_specs_text() {
        // The spec's quoted block (§4, R11), word for word.
        assert_eq!(
            LEGEND,
            [
                "Exit status: 0 = done, every check passed. 64 = bad command line, nothing ran.",
                "101 = internal error. 130 = stopped by the operator. Other codes from 128 up =",
                "killed by a signal. Otherwise the sum (1 to 63) of: 1 a check said no; 2 an",
                "input is invalid; 4 verify the result before trusting it; 8 refused (network",
                "mismatch, secret on the command line, unsafe output); 16 a correction was",
                "applied; 32 a file, device or helper failed.",
            ]
            .join(" ")
        );
    }
}
