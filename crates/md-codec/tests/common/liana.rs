// Liana evidence fixture + kind-1 descriptor builder, spliced into several
// crate roots the same way `vendored.rs` is (see its own header comment for
// why: `examples/` and `tests/` are separate crate roots, and `use` across
// them does not compile). Pulled in with
// `include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/common/liana.rs"))`.
//
// This file assumes `tests/common/vendored.rs` has ALREADY been spliced in
// above it at the call site -- it uses `load_vendored_phrase`,
// `decode_vendored` and `all_vendored_vector_names` from there and does not
// redefine them (stage 1a shipped those; this file is stage 1b's own).
//
// The evidence itself lives in ANOTHER repository (mnemonic-engrave) and is
// vendored into `tests/fixtures/liana/cases.json` by
// `scripts/vendor-liana-evidence.sh`, not read live from this file.

// `Descriptor` is already in scope from `vendored.rs`, spliced in above this
// file at the call site -- importing it again here would collide (E0252,
// vendored.rs's own header comment explains it). `Body`/`InternalKey` are
// NOT re-exported from the `md_codec` crate root (only `Tag` is; see
// `crates/md-codec/tests/address_derivation.rs:19` for the same path), so
// they need their own `use` here and a consumer root must not re-import
// them either.
use md_codec::tree::{Body, InternalKey};

/// A real kind-1 Descriptor, built by decoding a vendored kind-0 vector and
/// swapping only the internal key. A Descriptor cannot be built from
/// `cases.json`'s TLV bytes alone -- it would have no taptree, no older(),
/// no fingerprints and no divergent origins, all of which a byte-identical
/// descriptor gate needs.
fn kind1_from_vector(vector_name: &str) -> Descriptor {
    let mut d = decode_vendored(&load_vendored_phrase(vector_name))
        .unwrap_or_else(|e| panic!("{vector_name}: decode: {e}"));
    match &mut d.tree.body {
        Body::Tr { internal_key, .. } => {
            assert_eq!(
                *internal_key,
                InternalKey::NumsPoint,
                "{vector_name}: fixture must start at kind 0"
            );
            *internal_key = InternalKey::LianaUnspendable;
        }
        _ => panic!("{vector_name} is not a tr descriptor"),
    }
    d
}

/// One vendored Liana evidence case -- see `scripts/vendor-liana-evidence.sh`
/// for exactly how each field was extracted.
#[derive(serde::Deserialize, Clone)]
struct Case {
    name: String,
    accepted: bool,
    leaf_tlv_hex: Vec<String>,     // 65-byte chain code || compressed pubkey
    leaf_pubkeys_hex: Vec<String>, // the 33-byte slices at [32..65]
    expected_xpub: String,
    descriptor_with_checksum: String,
    liana_receive: Vec<String>,
    liana_change: Vec<String>,
}

impl Case {
    /// The 33-byte compressed pubkeys SPEC section 2 hashes, in wire order.
    fn leaf_pubkeys(&self) -> Vec<[u8; 33]> {
        self.leaf_pubkeys_hex
            .iter()
            .map(|h| {
                <[u8; 33]>::try_from(hex::decode(h).expect("hex").as_slice()).expect("33 bytes")
            })
            .collect()
    }
}

/// Every vendored case -- all EIGHT, including the four Liana refused on
/// policy shape: section 2's recipe is correct for those too, and one of
/// them is the only nested taptree in the evidence.
fn all_cases() -> Vec<Case> {
    serde_json::from_str(include_str!("../fixtures/liana/cases.json")).expect("cases.json")
}

fn case(name: &str) -> Case {
    all_cases()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no vendored case {name}"))
}

/// The vendored vector names whose decoded tree is a root `tr` at kind 0.
/// Stage 1a measured 23 of the 65 vectors as carrying `Body::Tr`.
fn all_kind0_tr_vectors() -> Vec<String> {
    all_vendored_vector_names()
        .into_iter()
        .filter(|n| {
            matches!(
                decode_vendored(&load_vendored_phrase(n)).map(|d| d.tree.body),
                Ok(Body::Tr { .. })
            )
        })
        .collect()
}
