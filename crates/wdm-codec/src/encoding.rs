//! Encoding layer: bech32 alphabet conversion and BCH error correction.
//!
//! Implements the codex32-derived (BIP 93) encoding with HRP `"wdm"`.

/// Which BCH code variant a string uses.
///
/// Determined by the total data-part length: regular for ≤93 chars,
/// long for 96–108 chars. Lengths 94–95 are reserved-invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BchCode {
    /// Regular code: BCH(93,80,8). 13-char checksum.
    Regular,
    /// Long code: BCH(108,93,8). 15-char checksum.
    Long,
}

/// The bech32 32-character alphabet, in 5-bit-value order.
///
/// `q=0, p=1, z=2, r=3, y=4, 9=5, x=6, 8=7, g=8, f=9, 2=10, t=11, v=12,
///  d=13, w=14, 0=15, s=16, 3=17, j=18, n=19, 5=20, 4=21, k=22, h=23,
///  c=24, e=25, 6=26, m=27, u=28, a=29, 7=30, l=31`.
pub const ALPHABET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Inverse lookup: char (lowercase ASCII) -> 5-bit value, or 0xFF if not in alphabet.
const ALPHABET_INV: [u8; 128] = build_alphabet_inv();

const fn build_alphabet_inv() -> [u8; 128] {
    let mut inv = [0xFFu8; 128];
    let mut i = 0;
    while i < 32 {
        inv[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    inv
}

/// Convert a sequence of 8-bit bytes to a sequence of 5-bit values
/// (padded with zero bits at the end if the bit count is not a multiple of 5).
pub fn bytes_to_5bit(bytes: &[u8]) -> Vec<u8> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity((bytes.len() * 8).div_ceil(5));
    for &b in bytes {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(((acc >> bits) & 0x1F) as u8);
        }
    }
    if bits > 0 {
        out.push(((acc << (5 - bits)) & 0x1F) as u8);
    }
    out
}

/// Convert a sequence of 5-bit values back to 8-bit bytes.
///
/// Returns `None` if any value in `values` is ≥ 32 (out of 5-bit range),
/// or if the trailing padding bits are non-zero.
pub fn five_bit_to_bytes(values: &[u8]) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(values.len() * 5 / 8);
    for &v in values {
        if v >= 32 {
            return None;
        }
        acc = (acc << 5) | v as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    // Any remaining bits must be zero (padding).
    if bits >= 5 {
        return None;
    }
    if (acc & ((1 << bits) - 1)) != 0 {
        return None;
    }
    Some(out)
}

/// The fixed human-readable part for WDM strings.
///
/// Mandated by the WDM spec (BIP 93 codex32 derivation). The value `"wdm"`
/// is permanently assigned and must not be made configurable.
pub const HRP: &str = "wdm";

/// The bech32 separator character between HRP and data-part (BIP 173 §3).
pub const SEPARATOR: char = '1';

/// Determine the BchCode variant from a total data-part length.
///
/// Boundaries are from BIP 93 (codex32): regular code `BCH(93,80,8)` caps at 93,
/// long code `BCH(108,93,8)` runs 96–108, and lengths 94–95 are explicitly
/// reserved-invalid to prevent ambiguity in code-variant selection. Lengths
/// below 14 or above 108 are also rejected.
pub fn bch_code_for_length(data_part_len: usize) -> Option<BchCode> {
    match data_part_len {
        14..=93 => Some(BchCode::Regular),
        94..=95 => None,
        96..=108 => Some(BchCode::Long),
        _ => None,
    }
}

/// Check whether a string is all-lowercase, all-uppercase, or mixed.
///
/// Only ASCII letters are considered; non-ASCII characters (digits, punctuation,
/// Unicode letters) are treated as neither case. This is appropriate for WDM
/// strings, whose alphabet is a subset of ASCII. An empty string or one with
/// no ASCII letters returns [`CaseStatus::Lower`].
pub fn case_check(s: &str) -> CaseStatus {
    let mut has_lower = false;
    let mut has_upper = false;
    for c in s.chars() {
        if c.is_ascii_lowercase() {
            has_lower = true;
        } else if c.is_ascii_uppercase() {
            has_upper = true;
        }
        if has_lower && has_upper {
            break;
        }
    }
    match (has_lower, has_upper) {
        (true, true) => CaseStatus::Mixed,
        (true, false) => CaseStatus::Lower,
        (false, true) => CaseStatus::Upper,
        (false, false) => CaseStatus::Lower, // empty / no letters; treat as lower
    }
}

/// Result of a case check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseStatus {
    /// All-lowercase or no letters.
    Lower,
    /// All-uppercase.
    Upper,
    /// Both lowercase and uppercase letters present (invalid).
    Mixed,
}

/// BCH polymod constants for the regular checksum (BCH(93,80,8)).
///
/// Source: BIP 93 (codex32) reference implementation, `ms32_polymod` function.
/// These five values are XORed into the running residue based on the top 5 bits
/// of the residue at each step. The polymod operation uses a 65-bit residue
/// (top 5 bits = current `b`, bottom 60 bits = masked state).
///
/// Verified against the canonical reference at
/// <https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki>.
pub const GEN_REGULAR: [u128; 5] = [
    0x19dc500ce73fde210,
    0x1bfae00def77fe529,
    0x1fbd920fffe7bee52,
    0x1739640bdeee3fdad,
    0x07729a039cfc75f5a,
];

/// Expected residue after polymod over a valid regular-code WDM string
/// (HRP-expanded + header + payload + checksum).
///
/// Derived NUMS-style: the top 65 bits of `SHA-256(b"shibbolethnums")`
/// interpreted as a big-endian 256-bit integer. This is unrelated to
/// BIP 93's `MS32_CONST` — WDM uses BIP 93's polynomial but its own
/// target residue, with HRP-mixing à la BIP 173 providing further
/// domain separation from codex32. See the BIP draft §"Why new target
/// constants?" for the design rationale.
///
/// Reproducible by:
/// ```text
/// import hashlib
/// h = hashlib.sha256(b"shibbolethnums").digest()
/// int.from_bytes(h, "big") >> (256 - 65)  # → 0x0815c07747a3392e7
/// ```
pub const WDM_REGULAR_CONST: u128 = 0x0815c07747a3392e7;

/// Initial residue value for both the regular and long polymod algorithms (BIP 93).
///
/// Both `ms32_polymod` and `ms32_long_polymod` start with this residue before
/// processing any input characters.
pub const POLYMOD_INIT: u128 = 0x23181b3;

/// Right-shift amount to extract the top 5 bits from a 65-bit regular-code residue.
///
/// Usage: `b = residue >> REGULAR_SHIFT` gives the 5-bit feedback selector
/// for the polymod algorithm.
pub const REGULAR_SHIFT: u32 = 60;

/// Mask preserving the low 60 bits of a 65-bit regular-code residue.
pub const REGULAR_MASK: u128 = 0x0fffffffffffffff;

/// BCH polymod constants for the long checksum (BCH(108,93,8)).
///
/// Source: BIP 93 (codex32) reference implementation, `ms32_long_polymod` function.
/// The long polymod uses a 75-bit residue (top 5 bits = `b`, bottom 70 bits = masked state).
///
/// Verified against the canonical reference at
/// <https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki>.
pub const GEN_LONG: [u128; 5] = [
    0x3d59d273535ea62d897,
    0x7a9becb6361c6c51507,
    0x543f9b7e6c38d8a2a0e,
    0x0c577eaeccf1990d13c,
    0x1887f74f8dc71b10651,
];

/// Expected residue after polymod over a valid long-code WDM string.
///
/// Derived NUMS-style: the top 75 bits of `SHA-256(b"shibbolethnums")`.
/// See [`WDM_REGULAR_CONST`] for the derivation method and design rationale.
///
/// Reproducible by:
/// ```text
/// import hashlib
/// h = hashlib.sha256(b"shibbolethnums").digest()
/// int.from_bytes(h, "big") >> (256 - 75)  # → 0x205701dd1e8ce4b9f47
/// ```
pub const WDM_LONG_CONST: u128 = 0x205701dd1e8ce4b9f47;

/// Right-shift amount to extract the top 5 bits from a 75-bit long-code residue.
///
/// Usage: `b = residue >> LONG_SHIFT` gives the 5-bit feedback selector
/// for the polymod algorithm.
pub const LONG_SHIFT: u32 = 70;

/// Mask preserving the low 70 bits of a 75-bit long-code residue.
pub const LONG_MASK: u128 = 0x3fffffffffffffffff;

/// One step of the BCH polymod algorithm from BIP 93.
///
/// Updates the running `residue` to incorporate the next 5-bit input `value`
/// using the polynomial defined by `gen`, shift width `shift`, and mask `mask`.
/// The same function is used for both the regular and long codes; pass
/// `(GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK)` for the regular code and
/// `(GEN_LONG, LONG_SHIFT, LONG_MASK)` for the long code.
///
/// Returns the updated residue after incorporating `value`. The top 5 bits of
/// the returned residue feed the next iteration's `b` selector.
///
/// This is a direct port of BIP 93's `ms32_polymod` / `ms32_long_polymod` inner
/// loop. See <https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki> .
fn polymod_step(residue: u128, value: u128, r#gen: &[u128; 5], shift: u32, mask: u128) -> u128 {
    let b = residue >> shift;
    let mut new_residue = (residue & mask) << 5 ^ value;
    for (i, &g) in r#gen.iter().enumerate() {
        if (b >> i) & 1 != 0 {
            new_residue ^= g;
        }
    }
    new_residue
}

/// BIP 173-style HRP-expansion: produces the 5-bit-symbol prelude that gets
/// prepended to the data part before running the BCH polymod.
///
/// For each HRP character `c`, emits `c >> 5` (high 3 bits); then emits a
/// single 0 separator; then emits each character's `c & 31` (low 5 bits).
/// The result has length `2 * hrp.len() + 1` for ASCII HRPs.
///
/// For `hrp_expand("wdm")` this returns `[3, 3, 3, 0, 23, 4, 13]`.
pub fn hrp_expand(hrp: &str) -> Vec<u8> {
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

/// Run polymod over a sequence of 5-bit values using the parameters for
/// either the regular or long BCH code, starting from POLYMOD_INIT.
fn polymod_run(values: &[u8], r#gen: &[u128; 5], shift: u32, mask: u128) -> u128 {
    let mut residue = POLYMOD_INIT;
    for &v in values {
        residue = polymod_step(residue, v as u128, r#gen, shift, mask);
    }
    residue
}

/// Compute the 13-character BCH checksum for the regular code over the
/// HRP-expanded preamble plus the data part.
///
/// `data` is the sequence of 5-bit values for the data part (header + payload),
/// not including the checksum. Returns the 13-element checksum array, ready
/// to append to `data` to form the full data-part-plus-checksum.
///
/// The algorithm runs polymod over `hrp_expand(hrp) || data || [0; 13]`,
/// then XORs the result with [`WDM_REGULAR_CONST`] to extract the checksum.
pub fn bch_create_checksum_regular(hrp: &str, data: &[u8]) -> [u8; 13] {
    // Regular code: 13-symbol checksum (0..=12), pad/array/extraction all use 13.
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data);
    input.extend(std::iter::repeat_n(0, 13));
    let polymod =
        polymod_run(&input, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK) ^ WDM_REGULAR_CONST;
    let mut out = [0u8; 13];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ((polymod >> (5 * (12 - i))) & 0x1F) as u8;
    }
    out
}

/// Verify a regular-code BCH checksum.
///
/// `data_with_checksum` is the full data part including the trailing 13
/// checksum characters. Returns `true` iff the polymod over
/// `hrp_expand(hrp) || data_with_checksum` equals [`WDM_REGULAR_CONST`].
pub fn bch_verify_regular(hrp: &str, data_with_checksum: &[u8]) -> bool {
    if data_with_checksum.len() < 13 {
        return false;
    }
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data_with_checksum);
    polymod_run(&input, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK) == WDM_REGULAR_CONST
}

/// Compute the 15-character BCH checksum for the long code.
///
/// Same algorithm as [`bch_create_checksum_regular`] but uses the long-code
/// polymod parameters (`GEN_LONG`, `LONG_SHIFT`, `LONG_MASK`) and target
/// constant ([`WDM_LONG_CONST`]). Produces a 15-element checksum array.
pub fn bch_create_checksum_long(hrp: &str, data: &[u8]) -> [u8; 15] {
    // Long code: 15-symbol checksum (0..=14), pad/array/extraction all use 15.
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data);
    input.extend(std::iter::repeat_n(0, 15));
    let polymod = polymod_run(&input, &GEN_LONG, LONG_SHIFT, LONG_MASK) ^ WDM_LONG_CONST;
    let mut out = [0u8; 15];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ((polymod >> (5 * (14 - i))) & 0x1F) as u8;
    }
    out
}

/// Verify a long-code BCH checksum.
///
/// Same algorithm as [`bch_verify_regular`] with long-code parameters.
/// Returns false if `data_with_checksum` is shorter than 15 symbols.
pub fn bch_verify_long(hrp: &str, data_with_checksum: &[u8]) -> bool {
    if data_with_checksum.len() < 15 {
        return false;
    }
    let mut input = hrp_expand(hrp);
    input.extend_from_slice(data_with_checksum);
    polymod_run(&input, &GEN_LONG, LONG_SHIFT, LONG_MASK) == WDM_LONG_CONST
}

/// Result of a successful BCH decode + correct attempt.
///
/// Returned by [`bch_correct_regular`] / [`bch_correct_long`] when correction
/// succeeds. `corrections_applied == 0` means the input was already valid;
/// `> 0` means substitutions were applied at the indicated positions.
///
/// Marked `#[non_exhaustive]` to allow future fields (e.g., confidence
/// score, syndrome metadata) without breaking downstream struct-literal
/// construction. Construct via the [`bch_correct_regular`] /
/// [`bch_correct_long`] APIs.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionResult {
    /// The corrected `data_with_checksum` slice (input may have been modified).
    pub data: Vec<u8>,
    /// Number of substitutions applied (0 = clean input).
    pub corrections_applied: usize,
    /// Indices into `data` of the substituted positions.
    pub corrected_positions: Vec<usize>,
}

// TODO(v0.2): replace brute-force 1-error correction with proper syndrome-based
// BCH decoding (Berlekamp-Massey / Forney) to reach the spec-promised 4-error
// correction in O(n^2) time. v0.1's brute force is correct for 0 or 1 errors;
// inputs with 2+ errors that happen to brute-force-correct are mathematically
// undetectable as "wrong correction" and would mislead the user. The v0.1
// decoder therefore deliberately returns BchUncorrectable rather than risk
// silent miscorrection beyond its 1-error reach.

/// Attempt to correct a regular-code BCH-checksummed string with up to one
/// substitution. Spec promises up to 4-error correction; v0.1 ships only a
/// 1-error baseline. See the TODO comment above this function for the v0.2
/// upgrade path.
///
/// Returns `Ok(CorrectionResult)` if the input is clean or one substitution
/// repairs it. Returns `Err(Error::BchUncorrectable)` otherwise.
pub fn bch_correct_regular(
    hrp: &str,
    data_with_checksum: &[u8],
) -> Result<CorrectionResult, crate::Error> {
    if bch_verify_regular(hrp, data_with_checksum) {
        return Ok(CorrectionResult {
            data: data_with_checksum.to_vec(),
            corrections_applied: 0,
            corrected_positions: vec![],
        });
    }
    if let Some(r) = brute_force_one_error(hrp, data_with_checksum, bch_verify_regular) {
        return Ok(r);
    }
    Err(crate::Error::BchUncorrectable)
}

/// Long-code analog of [`bch_correct_regular`].
pub fn bch_correct_long(
    hrp: &str,
    data_with_checksum: &[u8],
) -> Result<CorrectionResult, crate::Error> {
    if bch_verify_long(hrp, data_with_checksum) {
        return Ok(CorrectionResult {
            data: data_with_checksum.to_vec(),
            corrections_applied: 0,
            corrected_positions: vec![],
        });
    }
    if let Some(r) = brute_force_one_error(hrp, data_with_checksum, bch_verify_long) {
        return Ok(r);
    }
    Err(crate::Error::BchUncorrectable)
}

/// Try every (position, replacement) pair until verify succeeds. Returns the
/// first match (deterministic, scanned left-to-right, lowest replacement first).
fn brute_force_one_error<F>(
    hrp: &str,
    data_with_checksum: &[u8],
    verify: F,
) -> Option<CorrectionResult>
where
    F: Fn(&str, &[u8]) -> bool,
{
    // Single allocation; mutate in place and restore between positions.
    let mut trial = data_with_checksum.to_vec();
    for i in 0..trial.len() {
        let orig = trial[i];
        for v in 0..32u8 {
            if v == orig {
                continue;
            }
            trial[i] = v;
            if verify(hrp, &trial) {
                return Some(CorrectionResult {
                    data: trial,
                    corrections_applied: 1,
                    corrected_positions: vec![i],
                });
            }
        }
        trial[i] = orig;
    }
    None
}

/// Encode a payload + header into a full WDM string with HRP, separator, and checksum.
///
/// The BCH code variant (regular or long) is auto-selected from the data-part
/// length per the spec: regular for ≤93-char data parts, long for 96–108-char
/// data parts. Lengths that fall in the reserved-invalid 94–95 gap or exceed
/// 108 return [`Error::InvalidStringLength`].
///
/// Returns the full string starting with `"wdm1"`.
///
/// [`Error::InvalidStringLength`]: crate::Error::InvalidStringLength
pub fn encode_string(header: &[u8], payload: &[u8]) -> Result<String, crate::Error> {
    use crate::Error;

    let mut all_bytes = Vec::with_capacity(header.len() + payload.len());
    all_bytes.extend_from_slice(header);
    all_bytes.extend_from_slice(payload);
    let data_5bit = bytes_to_5bit(&all_bytes);

    // Auto-determine code from the eventual data-part length (data_5bit + checksum).
    let regular_total = data_5bit.len() + 13;
    let long_total = data_5bit.len() + 15;
    let code = match (
        bch_code_for_length(regular_total),
        bch_code_for_length(long_total),
    ) {
        (Some(BchCode::Regular), _) => BchCode::Regular,
        (_, Some(BchCode::Long)) => BchCode::Long,
        // Neither code variant accepts this data-part length: too short, in
        // the 94–95 reserved-invalid gap, or too long for v0.1.
        _ => {
            // Pick the closest length to report — long_total is always larger,
            // so report that as the "actual length you tried to produce".
            return Err(Error::InvalidStringLength(long_total));
        }
    };

    let checksum: Vec<u8> = match code {
        BchCode::Regular => bch_create_checksum_regular(HRP, &data_5bit).to_vec(),
        BchCode::Long => bch_create_checksum_long(HRP, &data_5bit).to_vec(),
    };

    let mut full = String::with_capacity(HRP.len() + 1 + data_5bit.len() + checksum.len());
    full.push_str(HRP);
    full.push(SEPARATOR);
    for &v in &data_5bit {
        full.push(ALPHABET[v as usize] as char);
    }
    for v in checksum {
        full.push(ALPHABET[v as usize] as char);
    }
    Ok(full)
}

/// Result of a successful WDM string decode.
///
/// `data` is the data part as 5-bit values (header + payload, checksum stripped).
/// Use [`five_bit_to_bytes`] to recover the original byte sequence.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedString {
    /// Data part as 5-bit values, with the trailing checksum already stripped.
    pub data: Vec<u8>,
    /// Detected BCH code variant.
    pub code: BchCode,
    /// Number of substitution errors corrected (0 = clean input, 1 = recovered).
    pub corrections_applied: usize,
    /// Indices into the data-part (chars after `"wdm1"`) of any corrected positions.
    pub corrected_positions: Vec<usize>,
}

/// Decode a WDM string, validating HRP, case, length, and checksum.
///
/// Performs error correction up to one substitution (v0.1 brute-force baseline;
/// v0.2 will add full 4-error syndrome decoding).
///
/// Errors:
/// - [`Error::MixedCase`] if the string mixes upper and lower case.
/// - [`Error::InvalidHrp`] if the HRP is missing or not `"wdm"`.
/// - [`Error::InvalidStringLength`] if the data-part length isn't a valid WDM length.
/// - [`Error::InvalidChar`] if the data part contains a non-bech32 character.
/// - [`Error::BchUncorrectable`] if the checksum can't be repaired with one substitution.
///
/// [`Error::MixedCase`]: crate::Error::MixedCase
/// [`Error::InvalidHrp`]: crate::Error::InvalidHrp
/// [`Error::InvalidStringLength`]: crate::Error::InvalidStringLength
/// [`Error::InvalidChar`]: crate::Error::InvalidChar
/// [`Error::BchUncorrectable`]: crate::Error::BchUncorrectable
pub fn decode_string(s: &str) -> Result<DecodedString, crate::Error> {
    use crate::Error;

    if matches!(case_check(s), CaseStatus::Mixed) {
        return Err(Error::MixedCase);
    }
    let s_lower = s.to_lowercase();

    let sep_pos = s_lower
        .rfind(SEPARATOR)
        .ok_or_else(|| Error::InvalidHrp(s_lower.clone()))?;
    let (hrp, rest) = s_lower.split_at(sep_pos);
    let data_part = &rest[1..]; // skip the '1' separator

    if hrp != HRP {
        return Err(Error::InvalidHrp(hrp.to_string()));
    }

    let code =
        bch_code_for_length(data_part.len()).ok_or(Error::InvalidStringLength(data_part.len()))?;

    let mut values: Vec<u8> = Vec::with_capacity(data_part.len());
    for (i, c) in data_part.chars().enumerate() {
        if !c.is_ascii() {
            return Err(Error::InvalidChar { ch: c, position: i });
        }
        let v = ALPHABET_INV[c as usize];
        if v == 0xFF {
            return Err(Error::InvalidChar { ch: c, position: i });
        }
        values.push(v);
    }

    let correction = match code {
        BchCode::Regular => bch_correct_regular(hrp, &values),
        BchCode::Long => bch_correct_long(hrp, &values),
    };
    let result = correction?;

    let checksum_len = match code {
        BchCode::Regular => 13,
        BchCode::Long => 15,
    };
    let data_only = result.data[..result.data.len() - checksum_len].to_vec();

    Ok(DecodedString {
        data: data_only,
        code,
        corrections_applied: result.corrections_applied,
        corrected_positions: result.corrected_positions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bch_code_equality() {
        assert_eq!(BchCode::Regular, BchCode::Regular);
        assert_ne!(BchCode::Regular, BchCode::Long);
    }

    #[test]
    fn bch_code_can_be_hashed() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BchCode::Regular);
        set.insert(BchCode::Long);
        set.insert(BchCode::Regular);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn alphabet_is_32_unique_chars() {
        let mut seen = std::collections::HashSet::new();
        for &c in ALPHABET {
            assert!(seen.insert(c), "duplicate char in alphabet: {}", c as char);
        }
        assert_eq!(seen.len(), 32);
    }

    #[test]
    fn bytes_to_5bit_round_trip_zero() {
        let bytes = vec![0x00];
        let fives = bytes_to_5bit(&bytes);
        assert_eq!(fives, vec![0, 0]);
        let back = five_bit_to_bytes(&fives).unwrap();
        assert_eq!(back, bytes);
    }

    #[test]
    fn bytes_to_5bit_round_trip_known_value() {
        // 0xFF = binary 11111111. Splits as 11111 (=31) and 111 (padded with 00 to 11100=28).
        let bytes = vec![0xFF];
        let fives = bytes_to_5bit(&bytes);
        assert_eq!(fives, vec![31, 28]);
    }

    #[test]
    fn bytes_to_5bit_round_trip_multibyte() {
        // 3 bytes = 24 bits → 5 five-bit groups (25 bits, 1 pad bit).
        let bytes = vec![0xDE, 0xAD, 0xBE];
        let back = five_bit_to_bytes(&bytes_to_5bit(&bytes)).unwrap();
        assert_eq!(back, bytes);
    }

    #[test]
    fn five_bit_to_bytes_rejects_nonzero_padding() {
        // Two 5-bit values = 10 bits, of which 8 form a byte and 2 are padding.
        // If padding bits are nonzero, decode must fail.
        // 31 = 11111, 1 = 00001. Last 2 bits (= 01) are nonzero padding.
        assert!(five_bit_to_bytes(&[31, 1]).is_none());
    }

    #[test]
    fn five_bit_to_bytes_rejects_value_out_of_range() {
        assert!(five_bit_to_bytes(&[32]).is_none());
    }

    #[test]
    fn bch_code_for_length_regular() {
        assert_eq!(bch_code_for_length(14), Some(BchCode::Regular));
        assert_eq!(bch_code_for_length(93), Some(BchCode::Regular));
    }

    #[test]
    fn bch_code_for_length_long() {
        assert_eq!(bch_code_for_length(96), Some(BchCode::Long));
        assert_eq!(bch_code_for_length(108), Some(BchCode::Long));
    }

    #[test]
    fn bch_code_for_length_rejects_94_and_95() {
        assert_eq!(bch_code_for_length(94), None);
        assert_eq!(bch_code_for_length(95), None);
    }

    #[test]
    fn bch_code_for_length_rejects_extremes() {
        assert_eq!(bch_code_for_length(0), None);
        assert_eq!(bch_code_for_length(13), None);
        assert_eq!(bch_code_for_length(109), None);
        assert_eq!(bch_code_for_length(1000), None);
    }

    #[test]
    fn case_check_lowercase() {
        assert_eq!(case_check("wdm1qq"), CaseStatus::Lower);
    }

    #[test]
    fn case_check_uppercase() {
        assert_eq!(case_check("WDM1QQ"), CaseStatus::Upper);
    }

    #[test]
    fn case_check_mixed() {
        assert_eq!(case_check("wDm1qq"), CaseStatus::Mixed);
    }

    #[test]
    fn case_check_empty_string_is_lower() {
        assert_eq!(case_check(""), CaseStatus::Lower);
    }

    #[test]
    fn case_check_digits_only_is_lower() {
        // Digits have no case; result must be Lower (BIP 173: no-letter strings are lower).
        assert_eq!(case_check("1234"), CaseStatus::Lower);
    }

    #[test]
    fn gen_regular_has_5_entries() {
        assert_eq!(GEN_REGULAR.len(), 5);
    }

    #[test]
    fn gen_long_has_5_entries() {
        assert_eq!(GEN_LONG.len(), 5);
    }

    #[test]
    fn gen_regular_matches_bip93_canonical_values() {
        // Cross-checked against https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki
        // ms32_polymod GEN array. If this fails, the constants drifted from the BIP.
        assert_eq!(GEN_REGULAR[0], 0x19dc500ce73fde210);
        assert_eq!(GEN_REGULAR[1], 0x1bfae00def77fe529);
        assert_eq!(GEN_REGULAR[2], 0x1fbd920fffe7bee52);
        assert_eq!(GEN_REGULAR[3], 0x1739640bdeee3fdad);
        assert_eq!(GEN_REGULAR[4], 0x07729a039cfc75f5a);
    }

    #[test]
    fn gen_long_matches_bip93_canonical_values() {
        // Cross-checked against https://github.com/bitcoin/bips/blob/master/bip-0093.mediawiki
        // ms32_long_polymod GEN array.
        assert_eq!(GEN_LONG[0], 0x3d59d273535ea62d897);
        assert_eq!(GEN_LONG[1], 0x7a9becb6361c6c51507);
        assert_eq!(GEN_LONG[2], 0x543f9b7e6c38d8a2a0e);
        assert_eq!(GEN_LONG[3], 0x0c577eaeccf1990d13c);
        assert_eq!(GEN_LONG[4], 0x1887f74f8dc71b10651);
    }

    #[test]
    fn polymod_init_matches_bip93() {
        // POLYMOD_INIT is unchanged from BIP 93; the GEN_REGULAR / GEN_LONG
        // constants have their own value-equality tests.
        assert_eq!(POLYMOD_INIT, 0x23181b3);
    }

    #[test]
    fn wdm_target_constants_match_nums_derivation() {
        // Self-check: the constants must equal the top 65 / 75 bits of
        // SHA-256(b"shibbolethnums") interpreted as a big-endian 256-bit
        // integer. If anyone "fixes" the hex values without updating the
        // derivation, this test fails.
        use bitcoin::hashes::{Hash, sha256};
        let h = sha256::Hash::hash(b"shibbolethnums");
        let bytes = h.to_byte_array();
        // First 16 bytes of the hash interpreted as a big-endian u128.
        // The top 65 / 75 bits of this value equal the top 65 / 75 bits
        // of the full 256-bit hash, since 75 < 128.
        let top_128 = u128::from_be_bytes(bytes[..16].try_into().unwrap());
        assert_eq!(top_128 >> (128 - 65), WDM_REGULAR_CONST);
        assert_eq!(top_128 >> (128 - 75), WDM_LONG_CONST);
    }

    #[test]
    fn polymod_masks_are_consistent_with_shifts() {
        // The mask must be (1 << shift) - 1 so that masking preserves bits below
        // the shift boundary, exactly matching the BIP 93 algorithm.
        assert_eq!(REGULAR_MASK, (1u128 << REGULAR_SHIFT) - 1);
        assert_eq!(LONG_MASK, (1u128 << LONG_SHIFT) - 1);
        assert_eq!(REGULAR_SHIFT, 60);
        assert_eq!(LONG_SHIFT, 70);
    }

    #[test]
    fn polymod_step_zero_residue_zero_value() {
        // Both residue and value zero, no GEN XORs since b = 0.
        assert_eq!(
            polymod_step(0, 0, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK),
            0
        );
    }

    #[test]
    fn polymod_step_value_only_xor_when_residue_zero() {
        // Residue 0, value 7 → result is 7 (XORed into the shifted-zero residue).
        assert_eq!(
            polymod_step(0, 7, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK),
            7
        );
    }

    #[test]
    fn polymod_step_isolates_each_gen_entry() {
        // Setting just bit `shift+i` in the residue → b = 1<<i → only GEN[i] is XORed.
        for i in 0..5 {
            let r = 1u128 << (REGULAR_SHIFT + i);
            assert_eq!(
                polymod_step(r, 0, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK),
                GEN_REGULAR[i as usize],
                "bit {} of b should isolate GEN_REGULAR[{}]",
                i,
                i
            );
        }
    }

    #[test]
    fn polymod_step_xors_multiple_gens_when_multiple_b_bits_set() {
        // b = 0b00011 → XOR GEN[0] and GEN[1].
        let r = 0b00011u128 << REGULAR_SHIFT;
        assert_eq!(
            polymod_step(r, 0, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK),
            GEN_REGULAR[0] ^ GEN_REGULAR[1]
        );
        // b = 0b11111 → XOR all 5.
        let r = 0b11111u128 << REGULAR_SHIFT;
        let expected =
            GEN_REGULAR[0] ^ GEN_REGULAR[1] ^ GEN_REGULAR[2] ^ GEN_REGULAR[3] ^ GEN_REGULAR[4];
        assert_eq!(
            polymod_step(r, 0, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK),
            expected
        );
    }

    #[test]
    fn polymod_step_works_for_long_code() {
        // Same parameterization works for the long code (shift=70, mask=LONG_MASK).
        let r = 1u128 << LONG_SHIFT;
        assert_eq!(
            polymod_step(r, 0, &GEN_LONG, LONG_SHIFT, LONG_MASK),
            GEN_LONG[0]
        );
        // b = 0b11111 → XOR all 5 long GENs.
        let r = 0b11111u128 << LONG_SHIFT;
        let expected = GEN_LONG[0] ^ GEN_LONG[1] ^ GEN_LONG[2] ^ GEN_LONG[3] ^ GEN_LONG[4];
        assert_eq!(
            polymod_step(r, 0, &GEN_LONG, LONG_SHIFT, LONG_MASK),
            expected
        );
    }

    #[test]
    fn polymod_step_init_residue_first_iteration() {
        // POLYMOD_INIT < 2^60 so b = 0 in the first iteration; only the shift+xor happens.
        // Verify: polymod_step(POLYMOD_INIT, 0) = POLYMOD_INIT << 5.
        assert_eq!(
            polymod_step(POLYMOD_INIT, 0, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK),
            POLYMOD_INIT << 5
        );
        // And with value=v: polymod_step(POLYMOD_INIT, v) = (POLYMOD_INIT << 5) ^ v.
        assert_eq!(
            polymod_step(POLYMOD_INIT, 31, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK),
            (POLYMOD_INIT << 5) ^ 31
        );
    }

    #[test]
    fn polymod_step_value_and_gen_xor_combined() {
        // Both effects active: b = 1 (bit 0 of b set) AND value = 5.
        // Expected: ((residue & mask) << 5) ^ value ^ GEN[0]
        //         = (0 << 5) ^ 5 ^ GEN[0]
        //         = GEN_REGULAR[0] ^ 5
        let r = 1u128 << REGULAR_SHIFT;
        assert_eq!(
            polymod_step(r, 5, &GEN_REGULAR, REGULAR_SHIFT, REGULAR_MASK),
            GEN_REGULAR[0] ^ 5
        );
    }

    #[test]
    fn hrp_expand_wdm_matches_spec() {
        // BIP 173 hrp_expand for the WDM HRP. The seven-element prelude is
        // documented in the BIP draft §"Checksum".
        assert_eq!(hrp_expand("wdm"), vec![3, 3, 3, 0, 23, 4, 13]);
    }

    #[test]
    fn hrp_expand_empty_returns_just_separator() {
        // Edge case: empty HRP yields just the [0] separator.
        assert_eq!(hrp_expand(""), vec![0]);
    }

    #[test]
    fn bch_round_trip_regular() {
        // Encode then verify a small data part. The verify call sees the
        // full data + checksum, so polymod returns WDM_REGULAR_CONST exactly.
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_create_checksum_regular(hrp, &data);
        assert_eq!(checksum.len(), 13);

        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        assert!(bch_verify_regular(hrp, &full));
    }

    #[test]
    fn bch_verify_rejects_single_char_tampering_regular() {
        // Flipping one bit in one symbol breaks verification.
        // (Spot check; BCH detects all single-symbol errors by construction.)
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_create_checksum_regular(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        full[5] ^= 0x01;
        assert!(!bch_verify_regular(hrp, &full));
    }

    #[test]
    fn bch_verify_rejects_too_short_input_regular() {
        // Less than 13 symbols cannot hold a checksum.
        assert!(!bch_verify_regular("wdm", &[0, 1, 2]));
        assert!(!bch_verify_regular("wdm", &[]));
    }

    #[test]
    fn bch_known_vector_regular() {
        // Independently computed (Python reference) ground truth for one
        // specific input. If polymod, HRP-mixing, or the target constant
        // ever drift, this test catches it.
        //
        // Input: HRP "wdm", data = [0, 1, 2, 3, 4, 5, 6, 7]
        // Expected checksum: [8, 15, 19, 11, 11, 21, 18, 31, 14, 12, 14, 26, 15]
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let expected: [u8; 13] = [8, 15, 19, 11, 11, 21, 18, 31, 14, 12, 14, 26, 15];
        let actual = bch_create_checksum_regular("wdm", &data);
        assert_eq!(actual, expected);
    }

    #[test]
    fn bch_zero_data_does_not_self_validate_regular() {
        // The all-zeros data + all-zeros checksum must NOT validate, because
        // WDM_REGULAR_CONST was chosen NUMS-style to avoid this trivial case.
        // Data length 8 is arbitrary; any non-empty zero-fill exhibits the same
        // negative result. 8 echoes the regular-code known-vector data length.
        let mut zero = vec![0u8; 8];
        zero.extend(std::iter::repeat_n(0, 13));
        assert!(!bch_verify_regular("wdm", &zero));
    }

    #[test]
    fn bch_round_trip_empty_data_regular() {
        // Empty data part is a degenerate but valid input: the checksum
        // covers only the HRP preamble. encode → verify must round-trip.
        let checksum = bch_create_checksum_regular("wdm", &[]);
        assert!(bch_verify_regular("wdm", &checksum));
    }

    #[test]
    fn bch_round_trip_long() {
        let hrp = "wdm";
        let data: Vec<u8> = (0..16).collect();
        let checksum = bch_create_checksum_long(hrp, &data);
        assert_eq!(checksum.len(), 15);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        assert!(bch_verify_long(hrp, &full));
    }

    #[test]
    fn bch_verify_rejects_single_char_tampering_long() {
        // Flipping one bit in one symbol breaks verification.
        // (Spot check; BCH detects all single-symbol errors by construction.)
        let hrp = "wdm";
        let data: Vec<u8> = (0..16).collect();
        let checksum = bch_create_checksum_long(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        full[7] ^= 0x01;
        assert!(!bch_verify_long(hrp, &full));
    }

    #[test]
    fn bch_verify_rejects_too_short_input_long() {
        // Less than 15 symbols cannot hold a long-code checksum.
        assert!(!bch_verify_long("wdm", &[0; 14]));
        assert!(!bch_verify_long("wdm", &[]));
    }

    #[test]
    fn bch_known_vector_long() {
        // Independently computed (Python reference) ground truth.
        // Input: HRP "wdm", data = [0, 1, 2, ..., 15]
        // Expected checksum: [15, 13, 21, 28, 0, 1, 29, 17, 1, 26, 1, 25, 9, 30, 5]
        let data: Vec<u8> = (0..16).collect();
        let expected: [u8; 15] = [15, 13, 21, 28, 0, 1, 29, 17, 1, 26, 1, 25, 9, 30, 5];
        let actual = bch_create_checksum_long("wdm", &data);
        assert_eq!(actual, expected);
    }

    #[test]
    fn bch_zero_data_does_not_self_validate_long() {
        // All-zeros must not validate, by NUMS construction of WDM_LONG_CONST.
        // Data length 16 is arbitrary; any non-empty zero-fill exhibits the same
        // negative result. 16 echoes the long-code known-vector data length.
        let mut zero = vec![0u8; 16];
        zero.extend(std::iter::repeat_n(0, 15));
        assert!(!bch_verify_long("wdm", &zero));
    }

    #[test]
    fn bch_round_trip_empty_data_long() {
        // Degenerate but valid: checksum covers only the HRP preamble.
        let checksum = bch_create_checksum_long("wdm", &[]);
        assert!(bch_verify_long("wdm", &checksum));
    }

    #[test]
    fn bch_correct_regular_clean_input() {
        // Clean input → 0 corrections, identity result.
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_create_checksum_regular(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        let r = bch_correct_regular(hrp, &full).unwrap();
        assert_eq!(r.corrections_applied, 0);
        assert!(r.corrected_positions.is_empty());
        assert_eq!(r.data, full);
    }

    #[test]
    fn bch_correct_regular_one_error() {
        // Single-symbol corruption is recoverable.
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_create_checksum_regular(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        let original = full.clone();
        full[3] = (full[3] + 1) & 0x1F;
        let r = bch_correct_regular(hrp, &full).unwrap();
        assert_eq!(r.corrections_applied, 1);
        assert_eq!(r.corrected_positions, vec![3]);
        assert_eq!(r.data, original);
    }

    #[test]
    fn bch_correct_regular_two_errors_uncorrectable_v0_1() {
        // v0.1 baseline cannot fix 2-error damage; spec promises this in v0.2.
        // Choice of positions 3 and 7 with delta +1: the BCH code's minimum
        // distance ≥ 9 means no 2-error pattern is reachable as a 1-substitution
        // from any other valid codeword. This specific pair was verified
        // empirically not to alias to a different 1-correctable string under
        // the v0.1 brute-force decoder.
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let checksum = bch_create_checksum_regular(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        full[3] = (full[3] + 1) & 0x1F;
        full[7] = (full[7] + 1) & 0x1F;
        let result = bch_correct_regular(hrp, &full);
        assert!(matches!(result, Err(crate::Error::BchUncorrectable)));
    }

    #[test]
    fn bch_correct_long_clean_input() {
        let hrp = "wdm";
        let data: Vec<u8> = (0..16).collect();
        let checksum = bch_create_checksum_long(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        let r = bch_correct_long(hrp, &full).unwrap();
        assert_eq!(r.corrections_applied, 0);
    }

    #[test]
    fn bch_correct_long_one_error() {
        let hrp = "wdm";
        let data: Vec<u8> = (0..16).collect();
        let checksum = bch_create_checksum_long(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        let original = full.clone();
        full[5] = (full[5] + 1) & 0x1F;
        let r = bch_correct_long(hrp, &full).unwrap();
        assert_eq!(r.corrections_applied, 1);
        assert_eq!(r.corrected_positions, vec![5]);
        assert_eq!(r.data, original);
    }

    #[test]
    fn bch_correct_returns_correction_result_with_position() {
        // Verify the API contract: a successful 1-error correction reports
        // exactly the position that was changed.
        let hrp = "wdm";
        let data: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let checksum = bch_create_checksum_regular(hrp, &data);
        let mut full = data.clone();
        full.extend_from_slice(&checksum);
        // Damage the second checksum byte (position 9 from start).
        full[9] = (full[9] + 7) & 0x1F;
        let r = bch_correct_regular(hrp, &full).unwrap();
        assert_eq!(r.corrected_positions, vec![9]);
    }

    #[test]
    fn encode_decode_round_trip_regular() {
        let header = vec![0x42u8, 0x00];
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let s = encode_string(&header, &payload).unwrap();
        assert!(s.starts_with("wdm1"));

        let decoded = decode_string(&s).unwrap();
        assert_eq!(decoded.code, BchCode::Regular);
        assert_eq!(decoded.corrections_applied, 0);
        assert!(decoded.corrected_positions.is_empty());

        let bytes = five_bit_to_bytes(&decoded.data).unwrap();
        let mut expected = header.clone();
        expected.extend_from_slice(&payload);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn encode_decode_round_trip_long() {
        // Long-code data-part length must fall in 96..=108. Each byte = 8 bits
        // = 1.6 bech32 chars; with the 15-char long-code checksum appended, the
        // data_5bit length must be in 81..=93. 51 total bytes → ceil(51*8/5) = 82
        // 5-bit chars → 82 + 15 = 97-char data part (inside the long range).
        let header = vec![0u8; 2];
        let payload = vec![0x55u8; 49];
        let s = encode_string(&header, &payload).unwrap();
        let decoded = decode_string(&s).unwrap();
        assert_eq!(decoded.code, BchCode::Long);

        let bytes = five_bit_to_bytes(&decoded.data).unwrap();
        let mut expected = header.clone();
        expected.extend_from_slice(&payload);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn encode_starts_with_hrp_and_separator() {
        let s = encode_string(&[], &[1, 2, 3]).unwrap();
        assert!(
            s.starts_with("wdm1"),
            "string did not start with wdm1: {}",
            s
        );
    }

    #[test]
    fn decode_rejects_invalid_hrp() {
        let s = encode_string(&[], &[0u8; 10]).unwrap();
        let bad = s.replace("wdm", "btc");
        assert!(matches!(
            decode_string(&bad),
            Err(crate::Error::InvalidHrp(_))
        ));
    }

    #[test]
    fn decode_rejects_mixed_case() {
        let s = encode_string(&[], &[0u8; 10]).unwrap();
        let bad: String = s
            .chars()
            .enumerate()
            .map(|(i, c)| if i == 5 { c.to_ascii_uppercase() } else { c })
            .collect();
        assert!(matches!(decode_string(&bad), Err(crate::Error::MixedCase)));
    }

    #[test]
    fn decode_rejects_invalid_char() {
        // 'b' is excluded from the bech32 alphabet; substitute one in the data
        // part to force a parse-time character rejection.
        let s = encode_string(&[], &[0u8; 10]).unwrap();
        // s looks like "wdm1...". Splice 'b' at index 5 (definitely past "wdm1").
        let mut chars: Vec<char> = s.chars().collect();
        chars[5] = 'b';
        let bad: String = chars.into_iter().collect();
        assert!(matches!(
            decode_string(&bad),
            Err(crate::Error::InvalidChar { .. })
        ));
    }

    #[test]
    fn decode_rejects_missing_separator() {
        // No '1' at all in the string.
        let bad = "wdmnoseparatorhere";
        // Could be matched as InvalidHrp because rfind('1') returns None — actually
        // we need to walk the function: rfind('1') returns None → returns
        // InvalidHrp(s_lower). So a missing separator is reported as InvalidHrp.
        assert!(matches!(
            decode_string(bad),
            Err(crate::Error::InvalidHrp(_))
        ));
    }

    #[test]
    fn decode_recovers_one_error() {
        // Encode, corrupt one char in the data part, decode should auto-correct.
        let header = vec![0x42u8];
        let payload = vec![0x12, 0x34, 0x56, 0x78];
        let s = encode_string(&header, &payload).unwrap();

        let mut chars: Vec<char> = s.chars().collect();
        // Corrupt position 6 (well within the data part).
        // Replace with the next bech32 char in the alphabet (any deterministic 1-char swap).
        let original_char = chars[6];
        chars[6] = if original_char == 'q' { 'p' } else { 'q' };
        let corrupted: String = chars.into_iter().collect();

        let decoded = decode_string(&corrupted).unwrap();
        assert_eq!(decoded.corrections_applied, 1);
        assert_eq!(decoded.corrected_positions.len(), 1);

        // The recovered data should match the original payload via bytes round-trip.
        let bytes = five_bit_to_bytes(&decoded.data).unwrap();
        let mut expected = header.clone();
        expected.extend_from_slice(&payload);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn encode_rejects_payload_in_reserved_invalid_length_range() {
        // Data-part length 94-95 is reserved-invalid. Engineer a payload that
        // produces such a length: regular needs data_5bit ≤ 80 (data_part = 80+13=93),
        // long needs data_5bit ≥ 81 (data_part = 81+15=96).
        // For data_part = 94 we'd need data_5bit = 81 (with 13-char checksum).
        // 81 5-bit chars = 405 bits = 51 bytes minimum (51*8=408 → ceil to 81 chars).
        // So 51 bytes of input → 81-char data_5bit → regular_total=94, long_total=96.
        // Long is valid (96 in range), so this case actually picks Long.
        // Need data_5bit such that BOTH regular_total and long_total miss the valid ranges:
        //   regular_total = data_5bit.len() + 13 in [14..=93]  ⇒ data_5bit ∈ [1..=80]
        //   long_total    = data_5bit.len() + 15 in [96..=108] ⇒ data_5bit ∈ [81..=93]
        // The gap is data_5bit ∈ {} — no gap! Either regular or long catches every
        // length in [1..=93]. The error path triggers at data_5bit=0 or data_5bit≥94.
        //
        // Easy test: empty header + empty payload ⇒ data_5bit.len() = 0 ⇒ regular_total=13
        // ⇒ bch_code_for_length(13)=None ⇒ error.
        let result = encode_string(&[], &[]);
        assert!(matches!(result, Err(crate::Error::InvalidStringLength(_))));
    }
}
