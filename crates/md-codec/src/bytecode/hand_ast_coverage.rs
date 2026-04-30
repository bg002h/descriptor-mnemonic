//! Hand-AST coverage for tap-leaf operators that BIP 388 source-form
//! parsers reject due to top-level B-type requirement, plus a hash
//! byte-order defensive pin and a handful of per-arm decoder unit tests.
//!
//! These tests bypass the parser via `Miniscript::from_ast` and assert
//! the wire-byte form of the encoded AST directly, plus (for byte-order
//! coverage per Plan reviewer #1 Concern 5) a decode-direction round-trip
//! that catches the asymmetric encode/decode reversal bug class.
//!
//! Per spec §3.1–§3.3 of `design/SPEC_v0_7_0.md`. The module is
//! registered as `#[cfg(test)] mod hand_ast_coverage;` in `bytecode/mod.rs`.

use std::collections::HashMap;
use std::sync::Arc;

use bitcoin::hashes::Hash as _;
use miniscript::{DescriptorPublicKey, Miniscript, Tap, Terminal};

use crate::bytecode::Tag;
use crate::bytecode::cursor::Cursor;
use crate::bytecode::decode::{decode_tap_miniscript, decode_tap_terminal};
use crate::bytecode::encode::EncodeTemplate;
use crate::test_helpers::{dummy_key_a, dummy_key_b};

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

/// Encoder wire-byte pin only. An unwrapped `or_c` at the top of a tap
/// leaf is V-typed, which the parser/typer would reject at the leaf
/// root; the decoder round-trip via `t:or_c` (= `and_v(or_c, True)`)
/// is covered by `t_or_c_tap_leaf_round_trips` below.
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
    let key_c = crate::test_helpers::dummy_key_c();
    let keys = vec![key_a, key_b, key_c];

    // [k=2, n=3, Placeholder@0, Placeholder@1, Placeholder@2, sentinel]
    let p = Tag::Placeholder.as_byte();
    let bytes = vec![0x02, 0x03, p, 0, p, 1, p, 2, 0xFF];
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &keys, Tag::MultiA, 0, None).expect("multi_a decodes");
    assert_eq!(
        cur.remaining(),
        &[0xFF],
        "decoder must consume exactly k+n+placeholders, leaving sentinel"
    );
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
    // [False, True, False, sentinel]
    let bytes = vec![
        Tag::False.as_byte(),
        Tag::True.as_byte(),
        Tag::False.as_byte(),
        0xFF,
    ];
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::AndOr, 0, None).expect("andor decodes");
    assert_eq!(
        cur.remaining(),
        &[0xFF],
        "decoder must consume exactly three children, leaving sentinel"
    );
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
    let key_c = crate::test_helpers::dummy_key_c();
    let keys = vec![key_a, key_b, key_c];

    let p = Tag::Placeholder.as_byte();
    let c = Tag::Check.as_byte();
    let s = Tag::Swap.as_byte();
    let pk_k = Tag::PkK.as_byte();
    // [k=2, n=3, c:pk_k(@0), s:c:pk_k(@1), s:c:pk_k(@2), sentinel]
    let bytes = vec![
        0x02, 0x03, c, pk_k, p, 0, s, c, pk_k, p, 1, s, c, pk_k, p, 2, 0xFF,
    ];
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &keys, Tag::Thresh, 0, None).expect("thresh decodes");
    assert_eq!(
        cur.remaining(),
        &[0xFF],
        "decoder must consume exactly k + n + 3 children, leaving sentinel"
    );
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
    let bytes = vec![0xD2, 0x09, 0xFF];
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::After, 0, None).expect("after decodes");
    assert_eq!(
        cur.remaining(),
        &[0xFF],
        "decoder must consume only the LEB128 varint, leaving sentinel"
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
    let key_c = crate::test_helpers::dummy_key_c();
    let keys = vec![key_a, key_b, key_c];

    let p = Tag::Placeholder.as_byte();
    let bytes = vec![0x02, 0x03, p, 0, p, 1, p, 2, 0xFF];
    let mut cur = Cursor::new(&bytes);
    let decoded = decode_tap_terminal(&mut cur, &keys, Tag::SortedMultiA, 0, None)
        .expect("sortedmulti_a decodes");
    assert_eq!(
        cur.remaining(),
        &[0xFF],
        "decoder must consume exactly k+n+placeholders, leaving sentinel"
    );
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
    let mut bytes = [0u8; 33];
    bytes[..32].copy_from_slice(&[0x42u8; 32]);
    bytes[32] = 0xFF; // sentinel
    let mut cur = Cursor::new(&bytes);
    let decoded =
        decode_tap_terminal(&mut cur, &no_keys, Tag::Hash256, 0, None).expect("hash256 decodes");
    assert_eq!(
        cur.remaining(),
        &[0xFF],
        "decoder must consume exactly 32 bytes, leaving sentinel"
    );
    let payload = [0x42u8; 32];
    match decoded.node {
        Terminal::Hash256(h) => assert_eq!(h.as_byte_array(), &payload),
        other => panic!("expected Hash256, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Walk-order regression pin (v0.7.1)
// ---------------------------------------------------------------------------

/// `validate_tap_leaf_subset_with_allowlist` walks operator-tree
/// children depth-first leaf-first, so the deepest violation is
/// reported. For a tree like `thresh(1, sha256(H))` with an empty
/// allowlist (so neither `thresh` nor `sha256` is admitted), the
/// walker must report `"sha256"` (the leaf) — a future regression
/// flipping back to top-down rejection would report `"thresh"`.
///
/// Pins the contract documented in
/// [`bytecode::encode::validate_tap_leaf_subset_with_allowlist`]'s
/// rustdoc.
#[test]
fn walker_reports_deepest_violation_first() {
    use bitcoin::hashes::sha256;
    use miniscript::Threshold;

    use crate::bytecode::encode::validate_tap_leaf_subset_with_allowlist;

    let h = sha256::Hash::from_byte_array([0xAA; 32]);
    let sha = Miniscript::<DescriptorPublicKey, Tap>::from_ast(Terminal::Sha256(h)).unwrap();
    let thresh_term = Terminal::Thresh(Threshold::new(1, vec![Arc::new(sha)]).unwrap());
    let ms = Miniscript::<DescriptorPublicKey, Tap>::from_ast(thresh_term).unwrap();

    // Empty allowlist: every operator is out-of-subset.
    let allowlist: &[&str] = &[];
    let err = validate_tap_leaf_subset_with_allowlist(&ms, allowlist, Some(0))
        .expect_err("empty allowlist must reject");
    match err {
        crate::Error::SubsetViolation { operator, .. } => {
            assert_eq!(
                operator, "sha256",
                "depth-first walker must report the deepest violation \
                 (got {operator:?}; if 'thresh', the walker reverted to \
                 top-down rejection — see v07-walker-deepest-violation-pin-test)"
            );
        }
        other => panic!("expected SubsetViolation, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// §3.4 — v0.10 hand-AST coverage for OriginPaths block
//
// Pin (a) header bit 3 round-trip through `BytecodeHeader::new_v0` /
// `from_byte`, (b) encoder dispatch determinism (shared paths emit
// `Tag::SharedPath=0x34`; divergent paths emit `Tag::OriginPaths=0x36`),
// (c) `MAX_PATH_COMPONENTS=10` boundary at the `encode_path` API.
// ---------------------------------------------------------------------------

/// Header bit 3 round-trips through `BytecodeHeader::new_v0` and
/// `BytecodeHeader::from_byte`. The encoded byte must equal `0x08`
/// (origin_paths flag bit only) for `new_v0(false, true)`, and the
/// origin_paths accessor must agree.
#[test]
fn header_origin_paths_flag_round_trip() {
    use crate::bytecode::header::BytecodeHeader;

    let h = BytecodeHeader::new_v0(false, true);
    let b = h.as_byte();
    assert_eq!(b, 0x08, "new_v0(false, true) must encode bit 3 only");

    let h2 = BytecodeHeader::from_byte(b).expect("0x08 must round-trip");
    assert_eq!(
        h.origin_paths(),
        h2.origin_paths(),
        "origin_paths flag must survive round-trip through as_byte/from_byte"
    );
    assert!(
        h2.origin_paths(),
        "round-tripped header must report origin_paths == true"
    );
    assert!(
        !h2.fingerprints(),
        "fingerprints flag must remain clear when only bit 3 is set"
    );
}

/// Encoder dispatch determinism — when all per-`@N` paths agree, the encoder
/// MUST emit `Tag::SharedPath=0x34` (bit 3 clear), NOT `Tag::OriginPaths=0x36`.
/// Pinned by inspecting bytecode[0] (header) and bytecode[1] (path-decl tag).
#[test]
fn encoder_emits_shared_path_when_all_paths_agree() {
    use crate::policy::WalletPolicy;
    use crate::{EncodeOptions, bytecode::Tag};

    let p: WalletPolicy = "wsh(sortedmulti(2,@0/**,@1/**,@2/**))".parse().unwrap();
    let bytes = p.to_bytecode(&EncodeOptions::default()).unwrap();
    assert_eq!(
        bytes[0] & 0x08,
        0x00,
        "header bit 3 must be clear for shared-path policy; got 0x{:02x}",
        bytes[0]
    );
    assert_eq!(
        bytes[1],
        Tag::SharedPath.as_byte(),
        "encoder must emit Tag::SharedPath (0x34) for shared-path policy; got 0x{:02x}",
        bytes[1]
    );
    assert_ne!(
        bytes[1],
        Tag::OriginPaths.as_byte(),
        "encoder must NOT emit Tag::OriginPaths (0x36) when all per-@N paths agree"
    );
}

/// Encoder dispatch determinism — with divergent per-`@N` paths supplied via
/// `EncodeOptions::with_origin_paths`, the encoder MUST emit
/// `Tag::OriginPaths=0x36` and set header bit 3 (`0x08`).
#[test]
fn encoder_emits_origin_paths_when_paths_diverge() {
    use bitcoin::bip32::DerivationPath;
    use std::str::FromStr;

    use crate::policy::WalletPolicy;
    use crate::{EncodeOptions, bytecode::Tag};

    let p: WalletPolicy = "wsh(sortedmulti(2,@0/**,@1/**,@2/**))".parse().unwrap();
    let opts = EncodeOptions::default().with_origin_paths(vec![
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/2'").unwrap(),
        DerivationPath::from_str("m/48'/0'/0'/100'").unwrap(),
    ]);
    let bytes = p.to_bytecode(&opts).unwrap();
    assert_eq!(
        bytes[0], 0x08,
        "header byte must be 0x08 (bit 3 set, no fingerprints) for divergent paths; got 0x{:02x}",
        bytes[0]
    );
    assert_eq!(
        bytes[1],
        Tag::OriginPaths.as_byte(),
        "encoder must emit Tag::OriginPaths (0x36) for divergent paths; got 0x{:02x}",
        bytes[1]
    );
}

/// `MAX_PATH_COMPONENTS=10` boundary — `encode_path` must accept exactly
/// 10 components and reject 11 with
/// `Error::PathComponentCountExceeded { got: 11, max: 10 }`.
#[test]
fn max_path_components_boundary_10_passes_11_rejects() {
    use bitcoin::bip32::DerivationPath;
    use std::str::FromStr;

    use crate::bytecode::path::encode_path;

    let path_10 =
        DerivationPath::from_str("m/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'").expect("10-component path");
    let path_11 =
        DerivationPath::from_str("m/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'/0'").expect("11-component path");

    encode_path(&path_10).expect("10 components must encode (boundary is inclusive)");

    let err = encode_path(&path_11).expect_err("11 components must reject");
    assert!(
        matches!(
            err,
            crate::Error::PathComponentCountExceeded { got: 11, max: 10 }
        ),
        "expected PathComponentCountExceeded {{ got: 11, max: 10 }}, got {:?}",
        err
    );
}
