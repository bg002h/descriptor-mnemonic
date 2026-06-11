//! Cycle B (stress program) — P6/P7/P8 + anti-vacuity over the typed (T)
//! and wire (W) strategies. Spec:
//! design/BRAINSTORM_proptest_fragment_domain_expansion.md (R4 GREEN).
//!
//! P6: typed descriptors render through `to_miniscript_descriptor`, wire
//!     round-trip exactly, reparse to an EQUAL `miniscript::Descriptor`
//!     (rust-miniscript's parser is the genuinely independent oracle), and
//!     derive an address end-to-end.
//! P7: wire-valid-but-miniscript-invalid inputs refuse CLEANLY (Err, never
//!     a panic) while the wire round-trip stays exact.
//! P8: encoder-side clean errors on out-of-range k/n/children, plus the
//!     loud k>n engrave-but-can't-restore characterization (FOLLOWUP
//!     `encode-accepts-k-greater-than-n`).
#![cfg(feature = "derive")]

mod common;

use bitcoin::Network;
use common::{
    W_BOUNDARY_TIMELOCKS, canon, collect_tags_and_locks, descriptor_from_tree,
    descriptor_with_pubkeys, hash32, keyarg, multikeys, node2, node3, thresh_node, timelock,
    tr_node, typed_descriptor_strategy, wire_descriptor_strategy, wrap,
};
use md_codec::chunk::{reassemble, split};
use md_codec::decode::{decode_md1_string, decode_payload};
use md_codec::encode::{encode_md1_string, encode_payload};
use md_codec::to_miniscript::to_miniscript_descriptor;
use md_codec::tree::{Body, Node};
use md_codec::{Descriptor, Error, Tag};
use miniscript::DescriptorPublicKey;
use proptest::prelude::*;
use std::collections::HashSet;
use std::str::FromStr;

// ─── P6 oracle chain (shared by the property and the golden cells) ──────

/// Run the full P6 chain on `d` and return the derived mainnet receive
/// address string:
/// 1. `to_miniscript_descriptor(&canon(d), 0)` succeeds (failure is RED,
///    never filtered);
/// 2. wire round-trip: encode→string→chunks→decode == canon(d);
/// 3. reparse fixed-point: `Descriptor::from_str(rendered.to_string())`
///    succeeds AND == the constructed Descriptor (PartialEq);
/// 4. `derive_address(0, 0, Bitcoin)` succeeds and equals the reparsed
///    descriptor's `at_derivation_index(0)` address. (Given step 3 the
///    equality is implied; the marginal value is that the full derivation
///    pipeline errors nowhere. Address-oracle independence is anchored by
///    the golden literals in the self-test cells below.)
fn p6_chain(d: &Descriptor) -> String {
    let c = canon(d);
    // Step 1 — converter must succeed.
    let rendered = to_miniscript_descriptor(&c, 0).unwrap_or_else(|e| {
        panic!("P6 step 1: to_miniscript_descriptor must succeed, got {e:?}\ninput: {c:?}")
    });
    // Step 2 — wire round-trip (payload, string, chunks).
    let (bytes, bits) = encode_payload(&c).expect("P6 step 2: canonical encodes");
    assert_eq!(
        decode_payload(&bytes, bits).expect("P6 step 2: payload decodes"),
        c,
        "P6 step 2: payload round-trip must be exact"
    );
    let s = encode_md1_string(&c).expect("P6 step 2: string encodes");
    assert_eq!(
        decode_md1_string(&s).expect("P6 step 2: string decodes"),
        c,
        "P6 step 2: string round-trip must be exact"
    );
    let chunks = split(&c).expect("P6 step 2: splits");
    let refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
    assert_eq!(
        reassemble(&refs).expect("P6 step 2: reassembles"),
        c,
        "P6 step 2: chunk round-trip must be exact"
    );
    // Step 3 — reparse fixed-point via rust-miniscript's own parser.
    let rendered_str = rendered.to_string();
    let reparsed = miniscript::Descriptor::<DescriptorPublicKey>::from_str(&rendered_str)
        .unwrap_or_else(|e| panic!("P6 step 3: reparse must succeed, got {e:?}\n{rendered_str}"));
    assert_eq!(
        reparsed, rendered,
        "P6 step 3: reparse must be a fixed point"
    );
    // Step 4 — end-to-end address derivation.
    let got = c
        .derive_address(0, 0, Network::Bitcoin)
        .expect("P6 step 4: derive_address succeeds")
        .assume_checked()
        .to_string();
    let expected = reparsed
        .at_derivation_index(0)
        .expect("P6 step 4: at_derivation_index")
        .address(Network::Bitcoin)
        .expect("P6 step 4: address")
        .to_string();
    assert_eq!(got, expected, "P6 step 4: address differential");
    got
}

/// P7 oracle: `to_miniscript_descriptor` returns a clean `Err` (never a
/// panic, never a wrong descriptor) AND the wire round-trip stays exact.
fn assert_p7_clean_refusal(d: &Descriptor) {
    let c = canon(d);
    let res = to_miniscript_descriptor(&c, 0);
    assert!(
        res.is_err(),
        "P7: expected clean refusal, got Ok({})",
        res.unwrap()
    );
    let (bytes, bits) = encode_payload(&c).expect("P7: wire-valid input encodes");
    assert_eq!(
        decode_payload(&bytes, bits).expect("P7: payload decodes"),
        c,
        "P7: payload round-trip must stay exact"
    );
    let s = encode_md1_string(&c).expect("P7: string encodes");
    assert_eq!(
        decode_md1_string(&s).expect("P7: string decodes"),
        c,
        "P7: string round-trip must stay exact"
    );
    let chunks = split(&c).expect("P7: splits");
    let refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
    assert_eq!(
        reassemble(&refs).expect("P7: reassembles"),
        c,
        "P7: chunk round-trip must stay exact"
    );
}

// ─── Permanent oracle self-test cells: known-good through P6 ────────────
// Each pins a GOLDEN ADDRESS LITERAL (derived once, prefix-verified, then
// hard-coded) — this anchors the address oracle independently of the
// converter under test.

#[test]
fn self_test_wsh_and_v_pk_older_144() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
            timelock(Tag::Older, 144),
        ),
    ));
    let addr = p6_chain(&d);
    assert!(addr.starts_with("bc1q"), "expected P2WSH, got {addr}");
    assert_eq!(
        addr,
        "bc1qjrek53xfxcz9epmg7teke3qh0sgs4za8zgnaf8kzr62rd7gp5nrq6xs44a"
    );
}

#[test]
fn self_test_wsh_andor_pk_older_4096_pk() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node3(
            Tag::AndOr,
            keyarg(Tag::PkK, 0),
            timelock(Tag::Older, 4096),
            keyarg(Tag::PkK, 1),
        ),
    ));
    let addr = p6_chain(&d);
    assert!(addr.starts_with("bc1q"), "expected P2WSH, got {addr}");
    assert_eq!(
        addr,
        "bc1qg0snqkymvvd0s4pusv2humsdj2t5yf5as4e5sk9w8zl0quqrj4rqr406t2"
    );
}

#[test]
fn self_test_tr_nums_and_v_sha256_pk() {
    let d = descriptor_with_pubkeys(tr_node(
        true,
        0,
        Some(node2(
            Tag::AndV,
            wrap(Tag::Verify, hash32(Tag::Sha256, [0x11; 32])),
            keyarg(Tag::PkK, 0),
        )),
    ));
    let addr = p6_chain(&d);
    assert!(addr.starts_with("bc1p"), "expected P2TR, got {addr}");
    assert_eq!(
        addr,
        "bc1psldl66p3tqj0lxcl7zm4eclrxaet4vz5ppqa6sxt5az8u4a6ef2qp5l03l"
    );
}

/// Miniscript-leniency pin: `older(0x10000)` is OUT of the BIP-68 mask
/// (low 16 bits zero — consensus treats it as no-op) yet rust-miniscript
/// 13.0.0 ACCEPTS it. This is the known leniency that motivated the
/// toolkit's own mask gate (toolkit v0.53.9). Pinned Ok LOUDLY: if a
/// future miniscript starts rejecting it, this cell goes red and the
/// P6/P7 class split must be re-derived.
#[test]
fn self_test_older_0x10000_miniscript_leniency() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
            timelock(Tag::Older, 0x0001_0000),
        ),
    ));
    let addr = p6_chain(&d);
    assert!(addr.starts_with("bc1q"), "expected P2WSH, got {addr}");
    assert_eq!(
        addr,
        "bc1qcj2atyh7su8wnqn3ew4drtxmfh3tl5y3n6uwvw38jep83xfy0alspfzaze"
    );
}

/// Same leniency pin for the time-class out-of-mask value `older(0x00410000)`
/// (bit 22 set, low 16 bits zero). See toolkit v0.53.9.
#[test]
fn self_test_older_0x00410000_miniscript_leniency() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
            timelock(Tag::Older, 0x0041_0000),
        ),
    ));
    let addr = p6_chain(&d);
    assert!(addr.starts_with("bc1q"), "expected P2WSH, got {addr}");
    assert_eq!(
        addr,
        "bc1qznrwq5w3wmzhlhjkazz4zqlhc092f79x6622uun9fxset0zeld9sqk8djl"
    );
}

/// Empirical sanity proof for the T-tier tap thresh production
/// (`thresh(1, pk_h(@1), s:pk(@2))` as a tap leaf) — round-3 evidence
/// proved the shape via from_str; this cell proves it through the FULL
/// P6 chain including the Tr-only reparse sanity branch.
#[test]
fn self_test_tr_thresh_pkh_swap_pk_leaf() {
    let d = descriptor_with_pubkeys(tr_node(
        false,
        0,
        Some(thresh_node(
            1,
            vec![keyarg(Tag::PkH, 1), wrap(Tag::Swap, keyarg(Tag::PkK, 2))],
        )),
    ));
    let addr = p6_chain(&d);
    assert!(addr.starts_with("bc1p"), "expected P2TR, got {addr}");
    assert_eq!(
        addr,
        "bc1pm0mejtph5njw5lespxmn2y3t3fxk9c3maku84llw2s7rhqa6kansahz935"
    );
}

/// Empirical sanity proof for the T-tier tap `a:` W-production
/// (`and_b(pk(@1), a:pk_h(@2))` as a tap leaf): round-3 proved a:pkh
/// type-valid; this cell proves tap-context sanity through the full chain.
#[test]
fn self_test_tr_and_b_pk_alt_pkh_leaf() {
    let d = descriptor_with_pubkeys(tr_node(
        false,
        0,
        Some(node2(
            Tag::AndB,
            keyarg(Tag::PkK, 1),
            wrap(Tag::Alt, keyarg(Tag::PkH, 2)),
        )),
    ));
    let addr = p6_chain(&d);
    assert!(addr.starts_with("bc1p"), "expected P2TR, got {addr}");
    assert_eq!(
        addr,
        "bc1pqf9kp8ehq9dn8at3wzp2m76pkc7eq902y73dupl5p02dpzemvk5q88urry"
    );
}

/// Legacy context through the full chain: `sh(or_d(multi(1,@0,@1), pk(@2)))`.
#[test]
fn self_test_sh_or_d_multi_pk() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Sh,
        node2(
            Tag::OrD,
            multikeys(Tag::Multi, 1, vec![0, 1]),
            keyarg(Tag::PkK, 2),
        ),
    ));
    let addr = p6_chain(&d);
    assert!(addr.starts_with('3'), "expected P2SH, got {addr}");
    assert_eq!(addr, "3HVMGTDDMN9FBw8QByVehgNb52m8k4WmwW");
}

/// LOUD characterization of an UPSTREAM rust-miniscript 13.0.0
/// Display/parse asymmetry that P6 found during bring-up (NOT an md-codec
/// bug — reproduced with pure miniscript, no md-codec involvement):
/// a DEPTH-2 taptree built via `TapTree::combine(combine(a,b),c)` Displays
/// as the malformed `{{a,b,c}}` instead of `{{a,b},c}`, and miniscript's
/// OWN `Descriptor::from_str` rejects that output
/// (`IncorrectNumberOfChildren { description: "taptree branch", .. }`).
/// A correctly-written depth-2 string PARSES Ok but re-Displays broken
/// (same checksum — Display is the faulty side).
///
/// md-codec's wire round-trip, converter, and address derivation are all
/// unaffected (none go through the string form) — asserted below. The
/// T-tier generator constrains taptrees to depth ≤ 1 because of this
/// (see common/mod.rs::t_tr_tree). If a future miniscript bump fixes
/// Display, the final assertion flips: restore the depth-2 generator arm
/// and invert this cell.
#[test]
fn upstream_taptree_depth2_display_asymmetry() {
    let d = descriptor_with_pubkeys(tr_node(
        false,
        0,
        Some(common::taptree2(
            common::taptree2(keyarg(Tag::PkK, 1), keyarg(Tag::PkK, 2)),
            keyarg(Tag::PkK, 3),
        )),
    ));
    let c = canon(&d);
    // Converter + derivation + wire all work…
    let rendered = to_miniscript_descriptor(&c, 0).expect("depth-2 taptree converts fine");
    let addr = c
        .derive_address(0, 0, Network::Bitcoin)
        .expect("depth-2 taptree derives fine")
        .assume_checked()
        .to_string();
    assert!(addr.starts_with("bc1p"), "expected P2TR, got {addr}");
    let s = encode_md1_string(&c).expect("encodes");
    assert_eq!(decode_md1_string(&s).expect("decodes"), c);
    // …but the rendered STRING is not reparseable under pinned 13.0.0.
    let rendered_str = rendered.to_string();
    assert!(
        miniscript::Descriptor::<DescriptorPublicKey>::from_str(&rendered_str).is_err(),
        "UPSTREAM FIXED? miniscript now reparses its own depth-2 taptree \
         Display ({rendered_str}); restore the t_tr_tree depth-2 arm and \
         invert this cell"
    );
}

// ─── Permanent oracle self-test cells: known-bad through P7 ─────────────

#[test]
fn self_test_bad_sortedmultia_wsh_leaf() {
    // SortedMultiA anywhere — rust-miniscript v13 has no Terminal
    // (FOLLOWUP `md-codec-sortedmulti-a-to-miniscript-rendering-gap`).
    let d = descriptor_with_pubkeys(wrap(Tag::Wsh, multikeys(Tag::SortedMultiA, 1, vec![0, 1])));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_sortedmultia_tap_leaf() {
    let d = descriptor_with_pubkeys(tr_node(
        false,
        0,
        Some(multikeys(Tag::SortedMultiA, 2, vec![1, 2])),
    ));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_rawpkh_leaf() {
    // RawPkH is not constructible via miniscript's public API.
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
            Node {
                tag: Tag::RawPkH,
                body: Body::Hash160Body([0x22; 20]),
            },
        ),
    ));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_sortedmulti_under_combinator() {
    // The Cycle-A engrave-but-can't-restore shape: SortedMulti must be the
    // sole child of wsh/sh; under a combinator it wire-round-trips but
    // refuses to render.
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, multikeys(Tag::SortedMulti, 1, vec![0, 1])),
            timelock(Tag::Older, 1),
        ),
    ));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_shape_c_check_over_or_i() {
    // Shape C: Check over a NON-bare-key child double-wraps and errors
    // (`c:` over type B). The 0.35.1 idempotence arm only collapses
    // Check(bare PkK/PkH).
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        wrap(
            Tag::Check,
            node2(Tag::OrI, keyarg(Tag::PkK, 0), keyarg(Tag::PkK, 1)),
        ),
    ));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_after_zero() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
            timelock(Tag::After, 0),
        ),
    ));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_after_bit31() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
            timelock(Tag::After, 0x8000_0000),
        ),
    ));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_older_zero() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
            timelock(Tag::Older, 0),
        ),
    ));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_older_bit31() {
    let d = descriptor_with_pubkeys(wrap(
        Tag::Wsh,
        node2(
            Tag::AndV,
            wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
            timelock(Tag::Older, 0x8000_0000),
        ),
    ));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_wsh_multi_21_keys() {
    let d = descriptor_with_pubkeys(wrap(Tag::Wsh, multikeys(Tag::Multi, 2, (0..21).collect())));
    assert_p7_clean_refusal(&d);
}

#[test]
fn self_test_bad_sh_multi_21_keys() {
    let d = descriptor_with_pubkeys(wrap(Tag::Sh, multikeys(Tag::Multi, 2, (0..21).collect())));
    assert_p7_clean_refusal(&d);
}

// ─── P6 / P7 properties ─────────────────────────────────────────────────

proptest! {
    // P6 — typed strategy renders, wire-round-trips, reparses to a fixed
    // point, and derives end-to-end. NO filtering: any failure is a
    // generator bug or a codec bug, both RED.
    #[test]
    fn p6_typed_to_miniscript_round_trip(d in typed_descriptor_strategy()) {
        p6_chain(&d);
    }

    // P7 (parametrized classes) — consensus-invalid `after` values refuse
    // cleanly and stay wire-exact.
    #[test]
    fn p7_bad_after_refuses_cleanly(v in prop_oneof![Just(0u32), 0x8000_0000u32..=u32::MAX]) {
        let d = descriptor_with_pubkeys(wrap(
            Tag::Wsh,
            node2(
                Tag::AndV,
                wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
                timelock(Tag::After, v),
            ),
        ));
        assert_p7_clean_refusal(&d);
    }

    // P7 — `older(0)` / `older(bit-31-set)` refuse cleanly. (NOT in this
    // set: out-of-BIP-68-mask values like 0x10000 — miniscript accepts
    // them; pinned Ok in the leniency cells above.)
    #[test]
    fn p7_bad_older_refuses_cleanly(v in prop_oneof![Just(0u32), 0x8000_0000u32..=u32::MAX]) {
        let d = descriptor_with_pubkeys(wrap(
            Tag::Wsh,
            node2(
                Tag::AndV,
                wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
                timelock(Tag::Older, v),
            ),
        ));
        assert_p7_clean_refusal(&d);
    }

    // P7 — Segwitv0/Legacy multi with 21..=32 keys exceeds
    // MAX_PUBKEYS_PER_MULTISIG and refuses cleanly in both contexts.
    #[test]
    fn p7_oversize_multi_refuses_cleanly(
        n in 21u8..=32,
        k in 1u8..=20,
        legacy in any::<bool>(),
    ) {
        let root = if legacy { Tag::Sh } else { Tag::Wsh };
        let d = descriptor_with_pubkeys(wrap(root, multikeys(Tag::Multi, k, (0..n).collect())));
        assert_p7_clean_refusal(&d);
    }
}

// ─── P8 — encoder-side clean errors + the k>n gap pin ───────────────────

proptest! {
    // P8 — out-of-range multi-family threshold k (0 or 33..=255) is a clean
    // encoder Err, never a panic.
    #[test]
    fn p8_encode_rejects_out_of_range_multi_k(
        k in prop_oneof![Just(0u8), 33u8..],
        len in 1usize..=8,
        tag in prop::sample::select(vec![
            Tag::Multi, Tag::SortedMulti, Tag::MultiA, Tag::SortedMultiA
        ]),
    ) {
        let d = descriptor_from_tree(
            wrap(Tag::Wsh, multikeys(tag, k, (0..len as u8).collect())),
            true,
        );
        let err = encode_payload(&d).expect_err("k out of 1..=32 must not encode");
        prop_assert!(
            matches!(err, Error::ThresholdOutOfRange { .. }),
            "expected ThresholdOutOfRange, got {err:?}"
        );
    }

    // P8 — out-of-range thresh k is a clean encoder Err.
    #[test]
    fn p8_encode_rejects_out_of_range_thresh_k(k in 33u8..) {
        let d = descriptor_from_tree(
            wrap(
                Tag::Wsh,
                thresh_node(k, vec![keyarg(Tag::PkK, 0), keyarg(Tag::PkK, 1)]),
            ),
            true,
        );
        let err = encode_payload(&d).expect_err("thresh k out of 1..=32 must not encode");
        prop_assert!(
            matches!(err, Error::ThresholdOutOfRange { .. }),
            "expected ThresholdOutOfRange, got {err:?}"
        );
    }
}

#[test]
fn p8_encode_rejects_empty_multi_indices() {
    // 0 keys in a multi-family body: clean ChildCountOutOfRange. The tree
    // still references @0 elsewhere so n ≥ 1 (a keyless tree would fail at
    // PathDecl instead).
    let d = descriptor_from_tree(
        wrap(
            Tag::Wsh,
            node2(
                Tag::AndV,
                wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
                multikeys(Tag::Multi, 1, vec![]),
            ),
        ),
        true,
    );
    let err = encode_payload(&d).expect_err("empty multi must not encode");
    assert!(
        matches!(err, Error::ChildCountOutOfRange { count: 0 }),
        "expected ChildCountOutOfRange, got {err:?}"
    );
}

#[test]
fn p8_encode_rejects_empty_thresh_children() {
    let d = descriptor_from_tree(
        wrap(
            Tag::Wsh,
            node2(
                Tag::AndV,
                wrap(Tag::Verify, keyarg(Tag::PkK, 0)),
                thresh_node(1, vec![]),
            ),
        ),
        true,
    );
    let err = encode_payload(&d).expect_err("empty thresh must not encode");
    assert!(
        matches!(err, Error::ChildCountOutOfRange { count: 0 }),
        "expected ChildCountOutOfRange, got {err:?}"
    );
}

#[test]
fn p8_encode_rejects_more_than_32_multi_indices() {
    // 33 repeated key slots in one multi body (n stays small — duplicates):
    // clean ChildCountOutOfRange at write_node.
    let d = descriptor_from_tree(wrap(Tag::Wsh, multikeys(Tag::Multi, 1, vec![0; 33])), true);
    let err = encode_payload(&d).expect_err("33-slot multi must not encode");
    assert!(
        matches!(err, Error::ChildCountOutOfRange { count: 33 }),
        "expected ChildCountOutOfRange, got {err:?}"
    );
}

#[test]
fn p8_encode_rejects_more_than_32_distinct_keys() {
    // 33 DISTINCT keys → n = 33 > 32: clean KeyCountOutOfRange at the
    // PathDecl (written before the tree).
    let d = descriptor_from_tree(
        wrap(Tag::Wsh, multikeys(Tag::Multi, 1, (0..33).collect())),
        true,
    );
    let err = encode_payload(&d).expect_err("n = 33 must not encode");
    assert!(
        matches!(err, Error::KeyCountOutOfRange { .. }),
        "expected KeyCountOutOfRange, got {err:?}"
    );
}

/// LOUD characterization of the encoder-side k>n gap: a multi with k > n
/// (both ≤ 32) ENCODES successfully — and the resulting wire payload is
/// then REJECTED at decode with `KGreaterThanN`. This is an
/// engrave-but-can't-restore gap in the same family as the Cycle-A find
/// (`bundle-accepts-sortedmulti-in-combinator-restore-cannot`).
///
/// FOLLOWUP: `encode-accepts-k-greater-than-n` (design/FOLLOWUPS.md;
/// companion entry in mnemonic-toolkit). Fixing the encoder gate is
/// library code — its own cycle. If this cell starts failing because
/// encode_payload begins REJECTING k > n, the gap was closed: resolve the
/// FOLLOWUP and invert this cell.
#[test]
fn p8_encode_accepts_k_greater_than_n_decode_rejects() {
    let d = descriptor_from_tree(wrap(Tag::Wsh, multikeys(Tag::Multi, 3, vec![0, 1])), true);
    let (bytes, bits) =
        encode_payload(&d).expect("CHARACTERIZATION: k=3-of-n=2 currently ENCODES (the gap)");
    let err = decode_payload(&bytes, bits).expect_err("decode must reject k > n");
    assert!(
        matches!(err, Error::KGreaterThanN { k: 3, n: 2 }),
        "expected KGreaterThanN, got {err:?}"
    );
    // The string form engraves too — same trap end-to-end.
    let s = encode_md1_string(&d).expect("k>n string-encodes (the gap)");
    assert!(decode_md1_string(&s).is_err(), "string decode must reject");
}

// ─── Anti-vacuity: generator coverage (fixed-seed TestRunner) ───────────

const W_TARGET_TAGS: [Tag; 34] = [
    Tag::Wsh,
    Tag::Sh,
    Tag::Tr,
    Tag::TapTree,
    Tag::PkK,
    Tag::PkH,
    Tag::Multi,
    Tag::SortedMulti,
    Tag::MultiA,
    Tag::SortedMultiA,
    Tag::After,
    Tag::Older,
    Tag::Sha256,
    Tag::Hash256,
    Tag::Ripemd160,
    Tag::Hash160,
    Tag::RawPkH,
    Tag::True,
    Tag::False,
    Tag::Check,
    Tag::Verify,
    Tag::Swap,
    Tag::Alt,
    Tag::DupIf,
    Tag::NonZero,
    Tag::ZeroNotEqual,
    Tag::AndV,
    Tag::AndB,
    Tag::AndOr,
    Tag::OrB,
    Tag::OrC,
    Tag::OrD,
    Tag::OrI,
    Tag::Thresh,
];

/// All to_miniscript-supported tags the typed grammar emits.
const T_TARGET_TAGS: [Tag; 23] = [
    Tag::Wsh,
    Tag::Sh,
    Tag::Tr,
    Tag::TapTree,
    Tag::PkK,
    Tag::PkH,
    Tag::Multi,
    Tag::MultiA,
    Tag::After,
    Tag::Older,
    Tag::Sha256,
    Tag::Hash256,
    Tag::Ripemd160,
    Tag::Hash160,
    Tag::Verify,
    Tag::Swap,
    Tag::Alt,
    Tag::AndV,
    Tag::AndB,
    Tag::AndOr,
    Tag::OrD,
    Tag::OrI,
    Tag::Thresh,
];

const T_BOUNDARY_AFTER: [u32; 7] = [
    1,
    144,
    0xFFFF,
    0x0001_0000,
    499_999_999,
    500_000_000,
    0x7FFF_FFFF,
];
const T_BOUNDARY_OLDER: [u32; 7] = [
    1,
    144,
    0xFFFF,
    0x0001_0000,
    0x0040_0001,
    0x0040_FFFF,
    0x0041_0000,
];

#[test]
fn w_generator_covers_all_fragments() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let strat = wire_descriptor_strategy();
    let mut tags: HashSet<Tag> = HashSet::new();
    let mut locks: HashSet<u32> = HashSet::new();
    let (mut pubkeys, mut fps, mut origins, mut divergent, mut shared) =
        (false, false, false, false, false);
    for _ in 0..1024 {
        let d = strat.new_tree(&mut runner).expect("generates").current();
        collect_tags_and_locks(&d.tree, &mut tags, &mut locks);
        pubkeys |= d.tlv.pubkeys.is_some();
        fps |= d.tlv.fingerprints.is_some();
        origins |= d.tlv.origin_path_overrides.is_some();
        divergent |= matches!(d.path_decl.paths, md_codec::PathDeclPaths::Divergent(_));
        shared |= matches!(d.path_decl.paths, md_codec::PathDeclPaths::Shared(_));
    }
    for t in W_TARGET_TAGS {
        assert!(tags.contains(&t), "W strategy never generated {t:?}");
    }
    for v in W_BOUNDARY_TIMELOCKS {
        assert!(
            locks.contains(&v),
            "W strategy never generated boundary timelock {v:#x}"
        );
    }
    assert!(pubkeys, "W strategy never attached a Pubkeys TLV");
    assert!(fps, "W strategy never attached a Fingerprints TLV");
    assert!(origins, "W strategy never attached OriginPathOverrides");
    assert!(divergent && shared, "W must mix Shared and Divergent decls");
}

#[test]
fn t_generator_covers_all_fragments() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let strat = typed_descriptor_strategy();
    let mut tags: HashSet<Tag> = HashSet::new();
    let mut locks: HashSet<u32> = HashSet::new();
    for _ in 0..2048 {
        let d = strat.new_tree(&mut runner).expect("generates").current();
        collect_tags_and_locks(&d.tree, &mut tags, &mut locks);
    }
    for t in T_TARGET_TAGS {
        assert!(tags.contains(&t), "T strategy never generated {t:?}");
    }
    for v in T_BOUNDARY_AFTER {
        assert!(
            locks.contains(&v),
            "T strategy never generated boundary after/older value {v:#x}"
        );
    }
    for v in T_BOUNDARY_OLDER {
        assert!(
            locks.contains(&v),
            "T strategy never generated boundary older value {v:#x}"
        );
    }
}
