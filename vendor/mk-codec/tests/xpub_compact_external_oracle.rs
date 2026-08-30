//! T4-b — spec-EXTERNAL field oracle for `XpubCompact::from_xpub`
//! (constellation-eval §2 item #9 Layer-A residual;
//! `design/SPEC_test_hardening_T4_mk_external_oracle.md`).
//!
//! `xpub_compact.rs`'s field mapping is otherwise tested only
//! self-referentially (`round_trip_full_xpub_depth_4`, `reconstruct_depth0_empty_path`
//! — both compare mutated code's output to a struct produced by the SAME
//! mutated code). A coordinated field-swap (e.g. `chain_code`↔`public_key`)
//! would survive both self-round-trip tests.
//!
//! This test decodes the **published BIP-32 test-vector-1 master xpub**
//! with a from-scratch base58check field-extractor sharing **zero code**
//! with `mk_codec` or `bitcoin::bip32::Xpub` on the verify side (only
//! mk-codec's existing `sha2` dev-dep for the double-SHA256 checksum — no
//! `bs58`, zero `Cargo.lock` delta), and asserts the independently-extracted
//! fields byte-for-byte against `XpubCompact::from_xpub`'s **compact-form**
//! output (never the reconstructed `Xpub` — a coordinated
//! `from_xpub`+`reconstruct_xpub` swap would cancel there, per R0 I3).

use std::str::FromStr;

use bitcoin::bip32::Xpub;
use mk_codec::bytecode::XpubCompact;
use sha2::{Digest, Sha256};

// BIP-32 "Test vector 1", seed 000102030405060708090a0b0c0d0e0f, "Chain m"
// (master, depth 0). Source:
// https://raw.githubusercontent.com/bitcoin/bips/master/bip-0032.mediawiki
// Re-verified character-for-character against the live BIP text 2026-07-10.
const TV1_MASTER_XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";

const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// From-scratch base58 decode (Bitcoin Core `DecodeBase58` algorithm:
/// repeated multiply-by-58-and-add over a big-endian byte buffer). Shares no
/// code with `bitcoin::base58` or any `m*-codec` crate.
fn base58_decode(s: &str) -> Vec<u8> {
    let zeroes = s.bytes().take_while(|&b| b == b'1').count();
    let digits: Vec<u32> = s
        .bytes()
        .map(|b| {
            BASE58_ALPHABET
                .iter()
                .position(|&a| a == b)
                .unwrap_or_else(|| panic!("invalid base58 character: {}", b as char))
                as u32
        })
        .collect();
    // log(58)/log(256) ~= 0.732; upper bound on the base-256 byte length.
    let size = digits.len() * 733 / 1000 + 1;
    let mut b256 = vec![0u8; size];
    let mut length = 0usize;
    for d in digits {
        let mut carry = d;
        let mut i = 0usize;
        for byte in b256.iter_mut().rev() {
            if carry == 0 && i >= length {
                break;
            }
            carry += 58 * (*byte as u32);
            *byte = (carry % 256) as u8;
            carry /= 256;
            i += 1;
        }
        length = i;
    }
    let first_nonzero = b256.iter().position(|&b| b != 0).unwrap_or(b256.len());
    let mut out = vec![0u8; zeroes];
    out.extend_from_slice(&b256[first_nonzero..]);
    out
}

/// Independently-extracted fields of a base58check-encoded extended public
/// key (BIP-32 §"Serialization format"): 78-byte payload
/// `version[4] || depth[1] || parent_fingerprint[4] || child_number[4] ||
/// chain_code[32] || public_key[33]` followed by a 4-byte double-SHA256
/// checksum.
struct DecodedXpubFields {
    version: [u8; 4],
    depth: u8,
    parent_fingerprint: [u8; 4],
    child_number: [u8; 4],
    chain_code: [u8; 32],
    public_key: [u8; 33],
}

/// Base58check-decode `s` and slice out the fixed-offset xpub fields,
/// verifying the double-SHA256 checksum. Uses only `sha2` (mk-codec's
/// existing dev-dep) — no `bitcoin::base58`, no `mk_codec` decode path.
fn decode_xpub_fields(s: &str) -> DecodedXpubFields {
    let raw = base58_decode(s);
    assert!(
        raw.len() >= 4,
        "base58check-decoded payload too short: {} bytes",
        raw.len()
    );
    let (payload, checksum) = raw.split_at(raw.len() - 4);

    let round1 = Sha256::digest(payload);
    let round2 = Sha256::digest(round1);
    assert_eq!(
        &round2[0..4],
        checksum,
        "base58check checksum verification failed"
    );

    assert_eq!(
        payload.len(),
        78,
        "xpub payload must be exactly 78 bytes, got {}",
        payload.len()
    );
    DecodedXpubFields {
        version: payload[0..4].try_into().unwrap(),
        depth: payload[4],
        parent_fingerprint: payload[5..9].try_into().unwrap(),
        child_number: payload[9..13].try_into().unwrap(),
        chain_code: payload[13..45].try_into().unwrap(),
        public_key: payload[45..78].try_into().unwrap(),
    }
}

/// `XpubCompact::from_xpub`'s compact-form fields must byte-for-byte match
/// an independently-extracted base58check decode of the SAME published
/// BIP-32 tv1 master xpub. Depth-0 / `parent_fingerprint = 00000000` /
/// `child_number = 00000000` (the master-key case) exercises the no-path
/// reconstruction branch's input shape (`xpub_compact.rs:89-97`).
#[test]
fn xpub_compact_fields_match_independent_base58check_decode() {
    let expected = decode_xpub_fields(TV1_MASTER_XPUB);
    assert_eq!(expected.depth, 0, "tv1 Chain m is depth 0");
    assert_eq!(
        expected.parent_fingerprint, [0u8; 4],
        "tv1 master has no parent"
    );
    assert_eq!(
        expected.child_number, [0u8; 4],
        "tv1 master child_number is the BIP-32 convention 0"
    );

    // Input side ONLY: parse via bitcoin::bip32::Xpub to obtain a typed value
    // to feed into the crate under test. The verification side above never
    // touches bitcoin::bip32 or mk_codec.
    let xpub = Xpub::from_str(TV1_MASTER_XPUB).unwrap();
    let compact = XpubCompact::from_xpub(&xpub);

    assert_eq!(
        compact.version, expected.version,
        "XpubCompact.version must match the independently-decoded version bytes"
    );
    assert_eq!(
        compact.parent_fingerprint, expected.parent_fingerprint,
        "XpubCompact.parent_fingerprint must match the independently-decoded field"
    );
    assert_eq!(
        compact.chain_code, expected.chain_code,
        "XpubCompact.chain_code must match the independently-decoded field"
    );
    assert_eq!(
        compact.public_key, expected.public_key,
        "XpubCompact.public_key must match the independently-decoded field"
    );
}
