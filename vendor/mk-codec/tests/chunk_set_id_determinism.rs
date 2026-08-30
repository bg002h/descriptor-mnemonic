//! `chunk_set_id` determinism: re-encoding the same card MUST reproduce the
//! same mk1 strings, byte for byte.
//!
//! # Why this file exists
//!
//! SPEC §2.5 makes re-encode stability normative:
//!
//! > **Encoders MUST reuse the same value for all re-encodings of the same
//! > card.**
//!
//! Before 2026-08-14 that clause read "SHOULD generate it from a
//! cryptographically secure random source at first encoding and reuse the same
//! value for all subsequent re-encodings" — and a stateless encoder cannot
//! satisfy the second half of it by drawing fresh
//! entropy per call: it has nowhere to keep "the value used at first
//! encoding". `mk encode` did exactly that, so three invocations on identical
//! inputs produced three different cards on the wire — measured 2026-08-14.
//!
//! Deriving the id from the canonical bytecode satisfies the clause exactly and
//! statelessly, and it is the rule the sibling format already uses:
//! `md-codec`'s `derive_chunk_set_id` (chunk.rs) takes the top 20 bits of the
//! payload hash, MSB-first. mk1 now takes the top 20 bits of
//! `SHA-256(canonical_bytecode)` — the hash the chunk layer already computes
//! for the cross-chunk integrity suffix, so no new hashing is introduced.
//!
//! Callers that need a specific value (vector regeneration, conformance
//! fixtures) keep [`encode_with_chunk_set_id`], which is unchanged.

use std::str::FromStr;

use bitcoin::bip32::{DerivationPath, Fingerprint, Xpub};
use mk_codec::{KeyCard, decode, encode, encode_with_chunk_set_id};

/// The SeedHammer II cosigner card `A@0`, from BIP-39's own published test
/// vector `abandon abandon … about` at `m/48'/0'/0'/2'`.
///
/// PUBLIC BY CONSTRUCTION — a published BIP-39 vector. Never put funds behind
/// it.
const DEVICE_XPUB: &str = "xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf";
const DEVICE_FINGERPRINT: [u8; 4] = [0x73, 0xc5, 0xda, 0x0a];
const DEVICE_STUB: [u8; 4] = [0x5b, 0x48, 0xaf, 0x35];
const DEVICE_PATH: &str = "48'/0'/0'/2'";

/// What an INDEPENDENT implementation emits for that card: the SeedHammer
/// fork's Go port (`mk/encode.go`), whose `top20(sha256(bytecode))` is the same
/// rule this crate now applies.
///
/// These two strings are engraved on steel in the fork's committed S0 gate
/// record (`oracle/gaterecords/S0-trace-a.record.json`). They are the reason
/// this test is a CONFORMANCE vector and not merely a regression pin: they were
/// produced by code that has never seen this crate.
const DEVICE_STRINGS: [&str; 2] = [
    "mk1qpd8cwpqqsq4kj90x4eutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5av5muuc0cmfrjw2",
    "mk1qpd8cwpp806lhaeh6reknylagmwyjycf8044xtt9flsdlkvt6f6cthyl995lpm5zlrp6yv6kc36tw",
];

fn device_card() -> KeyCard {
    let path = DerivationPath::from_str(DEVICE_PATH).expect("valid path");
    let xpub = Xpub::from_str(DEVICE_XPUB).expect("valid xpub");
    KeyCard::new(
        vec![DEVICE_STUB],
        Some(Fingerprint::from(DEVICE_FINGERPRINT)),
        path,
        xpub,
    )
}

/// The SHOULD clause, as a test: two encodings of one card agree.
#[test]
fn re_encoding_a_card_reproduces_the_same_strings() {
    let card = device_card();
    let first = encode(&card).expect("encode succeeds");
    let second = encode(&card).expect("encode succeeds");
    assert!(
        first.len() > 1,
        "this card must chunk, or the test proves nothing about chunk_set_id \
         (single-string encodings carry no such field); got {} string(s)",
        first.len()
    );
    assert_eq!(
        first, second,
        "re-encoding the same card produced different strings, so the id was \
         not reused across encodings"
    );
}

/// Byte-identity with an independent implementation of the same format.
///
/// This is the property every cross-implementation byte-comparison gate rests
/// on. It cannot hold while the id is drawn from entropy, and it holds for free
/// once the id is derived from the payload both implementations already agree
/// on — proven by the fact that each decodes the other's chunks, which verifies
/// `SHA-256(canonical_bytecode)[0..4]` at reassembly.
#[test]
fn device_card_a0_matches_the_independent_go_implementation() {
    let strings = encode(&device_card()).expect("encode succeeds");
    assert_eq!(
        strings.len(),
        DEVICE_STRINGS.len(),
        "chunk count differs from the independent implementation"
    );
    for (i, (got, want)) in strings.iter().zip(DEVICE_STRINGS.iter()).enumerate() {
        // In full, both of them. A truncated mismatch message renders two
        // different strings as one.
        assert_eq!(got, want, "chunk {i} differs\n  got  {got}\n  want {want}");
    }
}

/// The explicit override is unchanged, and still beats the derived default.
/// Vector regeneration depends on this and must not become collateral damage.
#[test]
fn an_explicit_chunk_set_id_still_wins() {
    let card = device_card();
    let pinned = encode_with_chunk_set_id(&card, 0x12345).expect("encode succeeds");
    let derived = encode(&card).expect("encode succeeds");
    assert_ne!(
        pinned, derived,
        "0x12345 happens to equal the derived id; pick a different pin for this test"
    );
    // Both must still decode to the same card: the id is opaque to content.
    let refs: Vec<&str> = pinned.iter().map(String::as_str).collect();
    let a = decode(&refs).expect("pinned encoding decodes");
    let refs: Vec<&str> = derived.iter().map(String::as_str).collect();
    let b = decode(&refs).expect("derived encoding decodes");
    assert_eq!(a, b, "the two encodings must carry the same card");
}
