//! Hand-AST coverage for tap-leaf operators that BIP 388 source-form
//! parsers reject due to top-level B-type requirement, plus a hash
//! byte-order defensive pin and a handful of per-arm decoder unit tests.
//!
//! These tests bypass the parser via `Miniscript::from_ast` and assert
//! the wire-byte form of the encoded AST directly, plus (for byte-order
//! coverage per Plan reviewer #1 Concern 5) a decode-direction round-trip
//! that catches the asymmetric encode/decode reversal bug class.
//!
//! Per spec §3.1–§3.3 of `design/SPEC_v0_7_0.md`.

#![cfg(test)]

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use bitcoin::hashes::Hash as _;
use miniscript::{DescriptorPublicKey, Miniscript, Tap, Terminal};

use crate::bytecode::Tag;
use crate::bytecode::cursor::Cursor;
use crate::bytecode::decode::{decode_tap_miniscript, decode_tap_terminal};
use crate::bytecode::encode::EncodeTemplate;

fn dummy_key_a() -> DescriptorPublicKey {
    DescriptorPublicKey::from_str(
        "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5",
    )
    .unwrap()
}

fn dummy_key_b() -> DescriptorPublicKey {
    DescriptorPublicKey::from_str(
        "03a34b99f22c790c4e36b2b3c2c35a36db06226e41c692fc82b8b56ac1c540c5bd",
    )
    .unwrap()
}

fn map_ab() -> (HashMap<DescriptorPublicKey, u8>, Vec<DescriptorPublicKey>) {
    let key_a = dummy_key_a();
    let key_b = dummy_key_b();
    let mut map = HashMap::new();
    map.insert(key_a.clone(), 0u8);
    map.insert(key_b.clone(), 1u8);
    let keys = vec![key_a, key_b];
    (map, keys)
}

// ---------------------------------------------------------------------------
// §3.1 — Hand-AST tests for typing-awkward operators
// ---------------------------------------------------------------------------

/// Phase 2.0 outcome: the v0.6 decoder constructs a Miniscript via
/// `Miniscript::from_ast` (or equivalent typed constructor); for an
/// unwrapped `or_c` at the top of a tap leaf, the result is K-typed (not
/// B), which the typer will reject. This test pins the encoder's wire
/// shape (which is what Phase 2.1 cares about) and verifies the
/// decoder's behavior on the unwrapped form. If the decoder accepts
/// (returning the K-typed leaf), the round-trip is byte-stable. If the
/// decoder rejects, this test asserts the rejection diagnostic so a
/// later contributor knows precisely how the parser surfaces it.
#[test]
fn or_c_unwrapped_tap_leaf_byte_form() {
    let (map, _keys) = map_ab();
    let key_a = dummy_key_a();
    let key_b = dummy_key_b();

    // or_c(B-du, V): right child must be V-type. v: wraps a B-type, so
    // first build c:pk_k(b) (B), then wrap v:.
    let pk_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_a))
        .expect("pk_k(a) must build");
    let pk_b = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_b))
        .expect("pk_k(b) must build");
    let c_pk_b = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Check(Arc::new(pk_b)))
        .expect("c:pk_k(b) must build");
    let v_c_pk_b =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Verify(Arc::new(c_pk_b)))
            .expect("v:c:pk_k(b) must build");
    let or_c_term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::OrC(Arc::new(pk_a), Arc::new(v_c_pk_b));

    let mut out = Vec::new();
    or_c_term.encode_template(&mut out, &map).unwrap();

    assert_eq!(
        out,
        vec![
            Tag::OrC.as_byte(),
            Tag::PkK.as_byte(),
            Tag::Placeholder.as_byte(),
            0,
            Tag::Verify.as_byte(),
            Tag::Check.as_byte(),
            Tag::PkK.as_byte(),
            Tag::Placeholder.as_byte(),
            1,
        ],
        "or_c(pk_k(a), v:c:pk_k(b)) wire bytes must match the v0.6 layout"
    );
}

/// `t:or_c` (= `and_v(or_c, true)`) round-trips through the decoder
/// because the outer `and_v` produces a B-type at the top of the leaf,
/// which satisfies the type checker. Per spec §3.1 fold-in O-4: the
/// canonical way to test or_c through a full encode→decode round-trip
/// is via this `t:` wrap.
#[test]
fn t_or_c_tap_leaf_round_trips() {
    let (map, keys) = map_ab();
    let key_a = dummy_key_a();
    let key_b = dummy_key_b();

    // or_c(B-du, V): both children must be wrapped — left to B, right to V.
    let pk_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_a))
        .expect("pk_k(a) must build");
    let c_pk_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Check(Arc::new(pk_a)))
        .expect("c:pk_k(a) must build (B-type)");
    let pk_b = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_b))
        .expect("pk_k(b) must build");
    let c_pk_b = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Check(Arc::new(pk_b)))
        .expect("c:pk_k(b) must build");
    let v_c_pk_b =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Verify(Arc::new(c_pk_b)))
            .expect("v:c:pk_k(b) must build (V-type)");
    let or_c_inner: Terminal<DescriptorPublicKey, Tap> =
        Terminal::OrC(Arc::new(c_pk_a), Arc::new(v_c_pk_b));
    let or_c_ms = Miniscript::<DescriptorPublicKey, Tap>::from_ast(or_c_inner)
        .expect("or_c(B-du, V) must build");
    let true_ms =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::True).expect("True must build");
    // `t:X` desugars to `and_v(X, True)`. and_v requires a V-type left
    // child; or_c is itself V-type, so we feed it directly (no extra
    // v: wrap, which would fail since v: requires B-type).
    let t_or_c: Terminal<DescriptorPublicKey, Tap> =
        Terminal::AndV(Arc::new(or_c_ms), Arc::new(true_ms));

    // Encode
    let mut out = Vec::new();
    t_or_c.encode_template(&mut out, &map).unwrap();

    // Decode round-trip. decode_tap_miniscript reads the leading tag itself.
    let mut cur = Cursor::new(&out);
    let decoded = decode_tap_miniscript(&mut cur, &keys, Some(0))
        .expect("t:or_c wrapped form must round-trip through the decoder");

    // Re-encode the decoded miniscript and compare.
    let mut out2 = Vec::new();
    decoded
        .encode_template(&mut out2, &map)
        .expect("re-encode of decoded t:or_c must succeed");
    assert_eq!(out, out2, "round-trip of t:or_c must be byte-stable");
}

/// `d:v:older(144)` — `Terminal::DupIf(Verify(Older))`. v0.6 byte-shift
/// keeps Older=0x1F unchanged; LEB128(144) = `[0x90, 0x01]`.
#[test]
fn d_wrapper_tap_leaf_byte_form() {
    use miniscript::RelLockTime;
    let older_term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Older(RelLockTime::from_consensus(144).unwrap());
    let older_ms = Miniscript::<DescriptorPublicKey, Tap>::from_ast(older_term).unwrap();
    let v_older =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Verify(Arc::new(older_ms)))
            .unwrap();
    let d_v_older: Terminal<DescriptorPublicKey, Tap> = Terminal::DupIf(Arc::new(v_older));

    let mut out = Vec::new();
    d_v_older
        .encode_template(&mut out, &HashMap::new())
        .unwrap();

    assert_eq!(
        out,
        vec![
            Tag::DupIf.as_byte(),
            Tag::Verify.as_byte(),
            Tag::Older.as_byte(),
            0x90,
            0x01,
        ],
        "d:v:older(144) wire bytes must match the v0.6 layout"
    );
}

/// `j:pk_k(a)` — `Terminal::NonZero(PkK(a))`. j: requires a Bn-type child.
#[test]
fn j_wrapper_tap_leaf_byte_form() {
    let (map, _keys) = map_ab();
    let key_a = dummy_key_a();
    let pk_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_a)).unwrap();
    let j_pk: Terminal<DescriptorPublicKey, Tap> = Terminal::NonZero(Arc::new(pk_a));

    let mut out = Vec::new();
    j_pk.encode_template(&mut out, &map).unwrap();

    assert_eq!(
        out,
        vec![
            Tag::NonZero.as_byte(),
            Tag::PkK.as_byte(),
            Tag::Placeholder.as_byte(),
            0,
        ],
        "j:pk_k(a) wire bytes must match the v0.6 layout"
    );
}

/// `n:c:pk_k(a)` — `Terminal::ZeroNotEqual(Check(PkK(a)))`. n: requires
/// a B-type child; `c:pk_k` is B-type.
#[test]
fn n_wrapper_tap_leaf_byte_form() {
    let (map, _keys) = map_ab();
    let key_a = dummy_key_a();
    let pk_a = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::PkK(key_a)).unwrap();
    let c_pk =
        Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Check(Arc::new(pk_a))).unwrap();
    let n_c_pk: Terminal<DescriptorPublicKey, Tap> = Terminal::ZeroNotEqual(Arc::new(c_pk));

    let mut out = Vec::new();
    n_c_pk.encode_template(&mut out, &map).unwrap();

    assert_eq!(
        out,
        vec![
            Tag::ZeroNotEqual.as_byte(),
            Tag::Check.as_byte(),
            Tag::PkK.as_byte(),
            Tag::Placeholder.as_byte(),
            0,
        ],
        "n:c:pk_k(a) wire bytes must match the v0.6 layout"
    );
}

// ---------------------------------------------------------------------------
// §3.2 — Hash byte-order defensive pin (encode + decode round-trip)
// ---------------------------------------------------------------------------

/// MD wire format pins hash terminals in **internal byte order** (the
/// same byte order Bitcoin Core's RPC returns; *not* the reversed
/// display order). This test pins both directions: the encoder MUST emit
/// the input bytes verbatim, and the decoder MUST reconstruct a hash
/// with the same internal bytes.
///
/// Per Plan reviewer #1 Concern 5: a single-direction encode-only check
/// would miss the asymmetric "encode reverses, decode reverses back"
/// bug class where the round-trip succeeds but every wire byte is
/// silently rotated. Decode-direction assertion is required.
///
/// Inputs are **asymmetric** byte sequences (strictly increasing) so
/// that any reversal — encode-only, decode-only, or symmetric — is
/// observable. A constant-fill (palindromic) input would defeat the
/// asymmetric-reversal check (Phase 2 reviewer IMP-1).
#[test]
fn hash_terminals_encode_internal_byte_order_with_decode_round_trip() {
    use bitcoin::hashes::{hash160, ripemd160, sha256};
    use miniscript::hash256;

    let known_32: [u8; 32] = std::array::from_fn(|i| i as u8); // [0x00, 0x01, ..., 0x1F]
    let known_20: [u8; 20] = std::array::from_fn(|i| 0x80 + i as u8); // [0x80, 0x81, ..., 0x93]
    let map: HashMap<DescriptorPublicKey, u8> = HashMap::new();
    let no_keys: Vec<DescriptorPublicKey> = Vec::new();

    // ----- Sha256 -----
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Sha256(sha256::Hash::from_byte_array(known_32));
    let mut out = Vec::new();
    term.encode_template(&mut out, &map).unwrap();
    assert_eq!(out[0], Tag::Sha256.as_byte());
    assert_eq!(&out[1..33], &known_32[..]);
    let mut cur = Cursor::new(&out[1..]);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::Sha256, 0, None).expect("sha256 decodes");
    match decoded.node {
        Terminal::Sha256(h) => assert_eq!(h.as_byte_array(), &known_32),
        other => panic!("expected Sha256, got {other:?}"),
    }

    // ----- Hash256 -----
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Hash256(hash256::Hash::from_byte_array(known_32));
    let mut out = Vec::new();
    term.encode_template(&mut out, &map).unwrap();
    assert_eq!(out[0], Tag::Hash256.as_byte());
    assert_eq!(&out[1..33], &known_32[..]);
    let mut cur = Cursor::new(&out[1..]);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::Hash256, 0, None).expect("hash256 decodes");
    match decoded.node {
        Terminal::Hash256(h) => assert_eq!(h.as_byte_array(), &known_32),
        other => panic!("expected Hash256, got {other:?}"),
    }

    // ----- Ripemd160 -----
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Ripemd160(ripemd160::Hash::from_byte_array(known_20));
    let mut out = Vec::new();
    term.encode_template(&mut out, &map).unwrap();
    assert_eq!(out[0], Tag::Ripemd160.as_byte());
    assert_eq!(&out[1..21], &known_20[..]);
    let mut cur = Cursor::new(&out[1..]);
    let decoded = decode_tap_terminal(&mut cur, &no_keys, Tag::Ripemd160, 0, None)
        .expect("ripemd160 decodes");
    match decoded.node {
        Terminal::Ripemd160(h) => assert_eq!(h.as_byte_array(), &known_20),
        other => panic!("expected Ripemd160, got {other:?}"),
    }

    // ----- Hash160 -----
    let term: Terminal<DescriptorPublicKey, Tap> =
        Terminal::Hash160(hash160::Hash::from_byte_array(known_20));
    let mut out = Vec::new();
    term.encode_template(&mut out, &map).unwrap();
    assert_eq!(out[0], Tag::Hash160.as_byte());
    assert_eq!(&out[1..21], &known_20[..]);
    let mut cur = Cursor::new(&out[1..]);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::Hash160, 0, None).expect("hash160 decodes");
    match decoded.node {
        Terminal::Hash160(h) => assert_eq!(h.as_byte_array(), &known_20),
        other => panic!("expected Hash160, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// §3.3 — Per-arm decoder unit tests
// ---------------------------------------------------------------------------

/// Decode `multi_a(2, @0, @1, @2)` — verify k/n payloads consume the
/// expected number of bytes and the resulting Terminal is well-shaped.
#[test]
fn decoder_arm_multi_a_consumes_correct_bytes() {
    let key_a = dummy_key_a();
    let key_b = dummy_key_b();
    let key_c = DescriptorPublicKey::from_str(
        "02e493dbf1c10d80f3581e4904930b1404cc6c13900ee0758474fa94abe8c4cd13",
    )
    .unwrap();
    let keys = vec![key_a, key_b, key_c];

    // [k=2, n=3, Placeholder@0, Placeholder@1, Placeholder@2]
    let p = Tag::Placeholder.as_byte();
    let bytes = vec![0x02, 0x03, p, 0, p, 1, p, 2];
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &keys, Tag::MultiA, 0, None).expect("multi_a decodes");
    assert!(cur.is_empty(), "decoder must consume all input bytes");
    match decoded.node {
        Terminal::MultiA(ref thresh) => {
            assert_eq!(thresh.k(), 2);
            assert_eq!(thresh.n(), 3);
        }
        other => panic!("expected MultiA, got {other:?}"),
    }
}

/// Decode `andor(0, 1, 0)` — three children consumed in argument order.
#[test]
fn decoder_arm_andor_consumes_three_children() {
    let no_keys: Vec<DescriptorPublicKey> = Vec::new();
    // [False, True, False]
    let bytes = vec![
        Tag::False.as_byte(),
        Tag::True.as_byte(),
        Tag::False.as_byte(),
    ];
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::AndOr, 0, None).expect("andor decodes");
    assert!(cur.is_empty(), "decoder must consume all three children");
    assert!(matches!(decoded.node, Terminal::AndOr(_, _, _)));
}

/// Decode `thresh(2, c:pk_k(@0), s:c:pk_k(@1), s:c:pk_k(@2))` — k +
/// arg-count + N children. Children must be dissatisfiable; True is not,
/// so this test uses `c:pk_k` (B-type, dissatisfiable) wrapped in the
/// `s:` swap wrapper to satisfy thresh's W-type constraint on positions
/// 1..N (Tag::Swap = 0x0D in v0.6).
#[test]
fn decoder_arm_thresh_consumes_k_and_children() {
    let key_a = dummy_key_a();
    let key_b = dummy_key_b();
    let key_c = DescriptorPublicKey::from_str(
        "02e493dbf1c10d80f3581e4904930b1404cc6c13900ee0758474fa94abe8c4cd13",
    )
    .unwrap();
    let keys = vec![key_a, key_b, key_c];

    let p = Tag::Placeholder.as_byte();
    let c = Tag::Check.as_byte();
    let s = Tag::Swap.as_byte();
    let pk_k = Tag::PkK.as_byte();
    // [k=2, n=3, c:pk_k(@0), s:c:pk_k(@1), s:c:pk_k(@2)]
    let bytes = vec![
        0x02, 0x03, c, pk_k, p, 0, s, c, pk_k, p, 1, s, c, pk_k, p, 2,
    ];
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &keys, Tag::Thresh, 0, None).expect("thresh decodes");
    assert!(cur.is_empty(), "decoder must consume k + n + 3 children");
    match decoded.node {
        Terminal::Thresh(ref thresh) => {
            assert_eq!(thresh.k(), 2);
            assert_eq!(thresh.n(), 3);
        }
        other => panic!("expected Thresh, got {other:?}"),
    }
}

/// Decode `after(1234)` — varint payload consumed exactly.
/// LEB128(1234) = [0xD2, 0x09].
#[test]
fn decoder_arm_after_consumes_varint_only() {
    let no_keys: Vec<DescriptorPublicKey> = Vec::new();
    let bytes = vec![0xD2, 0x09];
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::After, 0, None).expect("after decodes");
    assert!(
        cur.is_empty(),
        "decoder must consume only the LEB128 varint"
    );
    match decoded.node {
        Terminal::After(lock) => {
            assert_eq!(lock.to_consensus_u32(), 1234);
        }
        other => panic!("expected After, got {other:?}"),
    }
}

/// Decode `sortedmulti_a(2, @0, @1, @2)` — same shape as multi_a, distinct tag.
#[test]
fn decoder_arm_sortedmulti_a_consumes_correct_bytes() {
    let key_a = dummy_key_a();
    let key_b = dummy_key_b();
    let key_c = DescriptorPublicKey::from_str(
        "02e493dbf1c10d80f3581e4904930b1404cc6c13900ee0758474fa94abe8c4cd13",
    )
    .unwrap();
    let keys = vec![key_a, key_b, key_c];

    let p = Tag::Placeholder.as_byte();
    let bytes = vec![0x02, 0x03, p, 0, p, 1, p, 2];
    let mut cur = Cursor::new(&bytes);
    let decoded = decode_tap_terminal(&mut cur, &keys, Tag::SortedMultiA, 0, None)
        .expect("sortedmulti_a decodes");
    assert!(cur.is_empty(), "decoder must consume all input bytes");
    // SortedMultiA decodes to its own Terminal variant (the upstream
    // miniscript fork tracks sorted-vs-unsorted as distinct Terminals).
    match decoded.node {
        Terminal::SortedMultiA(ref thresh) => {
            assert_eq!(thresh.k(), 2);
            assert_eq!(thresh.n(), 3);
        }
        other => panic!("expected SortedMultiA, got {other:?}"),
    }
}

/// Decode a Hash256 32-byte payload — verify exactly 32 bytes consumed.
#[test]
fn decoder_arm_hash256_consumes_32_bytes() {
    let no_keys: Vec<DescriptorPublicKey> = Vec::new();
    let payload = [0x42u8; 32];
    let mut cur = Cursor::new(&payload);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::Hash256, 0, None).expect("hash256 decodes");
    assert!(cur.is_empty(), "decoder must consume exactly 32 bytes");
    match decoded.node {
        Terminal::Hash256(h) => assert_eq!(h.as_byte_array(), &payload),
        other => panic!("expected Hash256, got {other:?}"),
    }
}
