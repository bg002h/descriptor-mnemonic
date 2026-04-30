//! BIP 93 codex32 BCH primitives for HRP `"md"` (regular code only).
//!
//! Extracted from the v0.x `encoding` module; v0.11 needs only the regular-code
//! checksum + verify (long code dropped along with v0.x).

/// BCH(93,80,8) generator polynomial coefficients (5 × 65-bit).
const GEN_REGULAR: [u128; 5] = [
    0x19dc500ce73fde210,
    0x1bfae00def77fe529,
    0x1fbd920fffe7bee52,
    0x1739640bdeee3fdad,
    0x07729a039cfc75f5a,
];

/// MD-domain target residue (NUMS-style, top 65 bits of
/// `SHA-256("shibbolethnums")`).
const MD_REGULAR_CONST: u128 = 0x0815c07747a3392e7;

const POLYMOD_INIT: u128 = 0x23181b3;
const REGULAR_SHIFT: u32 = 60;
const REGULAR_MASK: u128 = 0x0fffffffffffffff;

fn polymod_step(residue: u128, value: u128) -> u128 {
    let b = residue >> REGULAR_SHIFT;
    let mut new_residue = ((residue & REGULAR_MASK) << 5) ^ value;
    for (i, &g) in GEN_REGULAR.iter().enumerate() {
        if (b >> i) & 1 != 0 {
            new_residue ^= g;
        }
    }
    new_residue
}

fn polymod_run(values: &[u8]) -> u128 {
    let mut residue = POLYMOD_INIT;
    for &v in values {
        residue = polymod_step(residue, v as u128);
    }
    residue
}

/// BIP 173-style HRP expansion: `[c >> 5 for c in hrp] ++ [0] ++ [c & 31 for c in hrp]`.
fn hrp_expand(hrp: &str) -> Vec<u8> {
    let bytes = hrp.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 2 + 1);
    for &c in bytes {
        out.push(c >> 5);
    }
    out.push(0);
    for &c in bytes {
        out.push(c & 31);
    }
    out
}

/// 13-symbol regular-code BCH checksum over `hrp_expand(hrp) || data || [0; 13]`.
pub(crate) fn bch_create_checksum_regular(hrp: &str, data: &[u8]) -> [u8; 13] {
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data);
    input.extend(std::iter::repeat_n(0, 13));
    let polymod = polymod_run(&input) ^ MD_REGULAR_CONST;
    let mut out = [0u8; 13];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ((polymod >> (5 * (12 - i))) & 0x1F) as u8;
    }
    out
}

/// Verify a regular-code BCH checksum over the data-part-with-checksum.
pub(crate) fn bch_verify_regular(hrp: &str, data_with_checksum: &[u8]) -> bool {
    if data_with_checksum.len() < 13 {
        return false;
    }
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data_with_checksum);
    polymod_run(&input) == MD_REGULAR_CONST
}
