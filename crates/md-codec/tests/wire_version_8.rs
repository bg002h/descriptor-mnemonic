//! Stage 1b task 2: wire version 8 (the header, the accepted-version set, and
//! `Descriptor::wire_version()` derived from the tree — not assumed).
//!
//! Version-4 bytes are unchanged in this task (stage 1a's byte-equality gate
//! proves it over all 65 vendored vectors). This gate proves the plumbing:
//! the dispatch-safety arithmetic, the accepted version set, the mismatch
//! message naming both accepted versions, and that `wire_version()` reads
//! the tree (kind 1 ⇒ 8, kind 0 ⇒ 4) rather than assuming `Slot(0)`/kind 0
//! everywhere.
//!
//! Task 2 itself deliberately did NOT round-trip a `LianaUnspendable`
//! descriptor through `encode_payload`/`decode_payload` — `wire_version()`
//! already returned 8 for one, but `Body::Tr`'s wire encoding didn't carry
//! a kind bit yet, so such a round trip mis-decoded as `NumsPoint` (and a
//! chunked one failed to reassemble). Stage 1b task 3 closed that: the two
//! tests below, appended by task 3, DO round-trip a `LianaUnspendable`
//! descriptor now that `write_node`/`read_node` carry the kind bit at
//! version 8. See `Descriptor::wire_version`'s doc comment for the
//! now-historical detail of the state task 3 closed.

use md_codec::error::Error;
use md_codec::header::Header;
use md_codec::{decode_payload, encode_payload};

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/common/vendored.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/common/liana.rs"
));

#[test]
fn version_8_is_even_so_the_dispatch_routes_it_as_single_payload() {
    // decode.rs:191-193 reads bit 0 of the FIRST SYMBOL as the chunked flag.
    // For a single payload that bit IS v0, so every usable version must be
    // EVEN. Version 5 would route a single-string plate into the chunk
    // reassembler, which then reports WireVersionMismatch{got:2}.
    for divergent in [false, true] {
        let sym = (u16::from(divergent) << 4) | u16::from(Header::WF_UNSPENDABLE_VERSION);
        assert_eq!(
            (((sym << 3) as u8) >> 3) & 1,
            0,
            "version {} dispatches as CHUNKED",
            Header::WF_UNSPENDABLE_VERSION
        );
    }
}

#[test]
fn the_decoder_accepts_4_and_8_and_refuses_everything_else() {
    for v in 0u8..16 {
        assert_eq!(
            Header::is_supported_version(v),
            matches!(v, 4 | 8),
            "version {v}"
        );
    }
}

#[test]
fn wire_version_is_derived_from_the_tree_not_assumed() {
    // G-2: do NOT assume Slot(0). A non-zero slot is constructible.
    assert_eq!(
        kind1_from_vector("keyed_compose_tr_nums_three_leaves").wire_version(),
        8
    );
    for name in all_kind0_tr_vectors() {
        assert_eq!(
            decode_vendored(&load_vendored_phrase(&name))
                .unwrap()
                .wire_version(),
            4,
            "{name}"
        );
    }
}

#[test]
fn the_mismatch_message_names_the_accepted_set_not_a_single_version() {
    let s = Error::WireVersionMismatch { got: 9 }.to_string();
    assert!(s.contains('9'), "must name what it got: {s}");
    assert!(!s.contains("expected 4"), "stale single-version claim: {s}");
    assert!(
        s.contains('4') && s.contains('8'),
        "must name {{4, 8}}: {s}"
    );
}

// Stage 1b task 3: the kind bit itself.

#[test]
fn kind_0_and_kind_1_over_the_same_tree_encode_to_different_bytes() {
    let k0 = decode_vendored(&load_vendored_phrase("keyed_compose_tr_nums_three_leaves")).unwrap();
    let k1 = kind1_from_vector("keyed_compose_tr_nums_three_leaves");
    assert_ne!(
        encode_payload(&k0).unwrap().0,
        encode_payload(&k1).unwrap().0
    );
}

#[test]
fn a_kind_1_tree_round_trips_at_v8() {
    let d = kind1_from_vector("keyed_compose_tr_nums_three_leaves");
    let (bytes, bits) = encode_payload(&d).unwrap();
    assert_eq!(decode_payload(&bytes, bits).unwrap(), d);
}
