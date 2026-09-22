//! `md-cli`'s OWN reader for md-codec's vendored Liana-evidence fixture
//! (`crates/md-codec/tests/fixtures/liana/cases.json`). `case`/`Case` in
//! md-codec's own test root are `include!`d via `env!("CARGO_MANIFEST_DIR")`,
//! which resolves to md-cli's own directory from a file living HERE and
//! finds nothing there — the third time this exact class has bitten (r5/I-1
//! on `kind1_from_vector`, r4/I-2 on the same, now `case`) — so md-cli gets
//! its OWN loader over the SAME committed JSON: one fixture, two readers.
//!
//! Pulled into `liana_input_side.rs` (and, per the plan's own task-order
//! ruling, Task 8's later additions to that SAME file) as a genuine nested
//! MODULE — `#[path = "liana_cases.rs"] mod liana_cases;` — not `include!`:
//! this file is also auto-discovered by cargo as its own (test-less)
//! integration-test binary, like every top-level `tests/*.rs` file, and an
//! `include!`-spliced copy cannot carry this very module doc comment (inner
//! attributes/doc comments are only legal at the true start of the file
//! containing them; `common/facts.rs`, this crate's existing precedent for
//! the identical problem, uses the same `#[path]` module form for the same
//! reason). `pub` throughout, matching `common/facts.rs`'s own convention:
//! a `#[path]` module's items are private to it by default.
#![allow(missing_docs)]

/// One vendored Liana-evidence case — md-cli's slice of the fields its tests
/// shell out to `md` and check: `crates/md-codec/tests/common/liana.rs`'s own
/// (private, differently-shaped) `Case` also carries `leaf_tlv_hex`/
/// `leaf_pubkeys_hex`/`source_commit`, none of which any md-cli test touches
/// (serde ignores unknown JSON fields by default, so this narrower struct
/// still deserializes the same committed file).
///
/// `#[allow(dead_code)]`: not every consumer of `case()` reads every field
/// (this task's own tests read only `descriptor_with_checksum` and
/// `expected_xpub`) — the same reasoning md-codec's `common/liana.rs` states
/// on its own `Case`.
#[allow(dead_code)]
#[derive(serde::Deserialize, Clone)]
pub struct Case {
    pub name: String,
    pub accepted: bool,
    pub descriptor_with_checksum: String,
    pub expected_xpub: String,
    pub liana_receive: Vec<String>,
    pub liana_change: Vec<String>,
}

pub fn case(name: &str) -> Case {
    let raw = include_str!("../../md-codec/tests/fixtures/liana/cases.json");
    serde_json::from_str::<Vec<Case>>(raw)
        .expect("cases.json")
        .into_iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no vendored case {name}"))
}

/// A real `tr` whose internal key is a SPENDABLE xpub with NO recorded
/// origin — G-8: not Liana's recipe (it does not byte-match SPEC §2's
/// recomputation), not NUMS, and `md decompose` must keep TODAY'S
/// annotated-slot behaviour for it. The phantom property G-8 names is NO
/// ORIGIN, not "not Liana's" — refusing it would also lock out libnunchuk's
/// PR-1746 form and real origin-less spendable keys.
///
/// Built from `cases.json`'s own `preset-kofn-recovery-tr` key material: its
/// recorded internal-key xpub (origin stripped — the shape under test) as
/// THIS tr's internal key, and one of its own leaf keys (kept WITH its
/// origin) as the sole script-path leaf. Not the brief's own worked example:
/// that constant is truncated and does not parse (its own text says so) —
/// this one is real, vendored key material, and parses.
pub const ORIGINLESS_SPENDABLE_TR: &str = concat!(
    "tr(xpub6DXuQW1Q2JpZyweiMewTZuMPvjG8hKhV2qoF6wL9VFxsMBExtbfqAAoR4oMG4GyxFzVdfas1v2eAdfLxyjc4Ceo5B6w6zTpf7F2BuXCJ52i/<0;1>/*,",
    "and_v(v:pk([73c5da0a/48'/0'/1'/3']xpub6DXuQW1Q2JpZzLV9igdwdnmCSoaPVd4ZNZnvfgUsGvQ8AbNAhEmfBEMCMHctwZBuxWK8HkjqUW5F72MCSJCFfisVwRY62Kb1FuDZ66nNQe1/<0;1>/*),older(26280)))"
);

/// I-1 fix round 1 (Important, M17 on Task 9's mutation list) — the
/// STRONGEST near miss: an internal key that IS a real, valid Liana recipe
/// output (genuine BIP-341 NUMS pubkey, depth 0, zero parent fingerprint,
/// zero child number — every structural property a NUMS-pubkey/depth-0
/// PATTERN-MATCH would accept), computed over `preset-hashlock-gated-tr`'s
/// own (2-leaf) key set — swapped into `preset-kofn-recovery-tr`'s own
/// (DIFFERENT, 4-leaf) script tree. Recomputing SPEC §2 over the leaves
/// ACTUALLY present here reproduces `preset-kofn-recovery-tr`'s own 4-leaf
/// recipe xpub, which is NOT this descriptor's internal key — so only a
/// comparison that recomputes and checks full byte equality can tell this
/// apart from the real thing. A pattern-match cannot: it would wrongly
/// accept this and relabel a DIFFERENT wallet's unspendable key as this
/// one's. No checksum suffix: the swap invalidates the original one, and
/// `md decompose` accepts checksum-less input.
///
/// WHY A POSITIVE-ONLY TEST SUITE CANNOT CATCH THIS (the review's own
/// finding, recorded here so it survives independent of the review report):
/// every existing positive test hands the recogniser an internal key that
/// legitimately matches its OWN tree's leaves. A weakened pattern-match
/// recognises those cases too — pattern-matching is a SUPERSET of the real
/// check on every input where the real check accepts. The only shape that
/// separates the two is one the real check REJECTS and the pattern-match
/// ACCEPTS: a key that looks right but was computed over the wrong leaves.
pub fn near_miss_liana_recipe_over_different_leaves() -> String {
    let kofn = case("preset-kofn-recovery-tr");
    let kofn_recipe = case("preset-kofn-recovery-tr").expected_xpub;
    let hashlock_recipe = case("preset-hashlock-gated-tr").expected_xpub;
    let bare = kofn
        .descriptor_with_checksum
        .rsplit_once('#')
        .map(|(body, _)| body)
        .unwrap_or(kofn.descriptor_with_checksum.as_str());
    let swapped = bare.replacen(&kofn_recipe, &hashlock_recipe, 1);
    assert_ne!(
        swapped, bare,
        "the kofn recipe xpub must appear exactly once, as the internal key, \
         for this swap to do anything"
    );
    swapped
}

/// I-2 fix round 1 (Important) — a tr() whose RECOGNISED Liana-unspendable
/// internal key ALSO appears, byte-identically, as a tapleaf key: the
/// exact shape that exposed `decompose/mod.rs`'s retain running BEFORE
/// `check_no_repeated_key` instead of after.
///
/// CONSTRUCTIBLE, not merely hypothetical: SPEC §2 step 2 fixes the
/// recipe's own public key to the BIP-341 NUMS point REGARDLESS of chain
/// code, so placing that SAME xpub as a leaf contributes a FIXED,
/// known-in-advance 33-byte value to the leaf-pubkey hash no matter what
/// chain code the xpub itself carries. That makes the self-referential
/// point directly computable (not searched for): hash the NUMS point's own
/// compressed bytes together with one other real leaf's pubkey bytes via
/// the SAME `liana_unspendable_xpub` the recogniser itself calls, and the
/// result — call it X — satisfies X == recompute(leaves including X's own
/// pubkey bytes) BY CONSTRUCTION. `X` is then placed at BOTH the internal
/// key position and a `pk()` leaf position, verbatim.
pub fn self_referential_liana_key_descriptor() -> String {
    self_referential_liana_key_descriptor_at_leaf_use_site("<0;1>")
}

/// The same construction with the LEAF's use-site under the caller's control,
/// so one derivation of `X` serves both the same-use-site and the
/// disjoint-use-site variants.
///
/// `X` does NOT depend on the use-site: SPEC §2 hashes the leaves' PUBLIC
/// KEYS, and a multipath suffix changes neither the xpub nor the pubkey it
/// carries. So the SAME `X` is recognised as this descriptor's own recipe
/// output whatever path the leaf is spelled with — which is exactly why the
/// disjoint variant is reachable at all.
pub fn self_referential_liana_key_descriptor_at_leaf_use_site(leaf_use_site: &str) -> String {
    use bitcoin::Network;
    use bitcoin::secp256k1::PublicKey;
    use std::str::FromStr;

    // BIP-341 NUMS H-point, byte-identical to `md_codec::nums::
    // NUMS_H_POINT_X_ONLY_HEX` and md-cli's own (both crate-private)
    // `parse::template::NUMS_H_POINT_X_ONLY_HEX` — a third copy of one
    // fixed, published constant, not a fourth source of truth for it.
    const NUMS_H_POINT_X_ONLY_HEX: &str =
        "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
    let nums_pubkey = PublicKey::from_str(&format!("02{NUMS_H_POINT_X_ONLY_HEX}"))
        .expect("BIP-341 NUMS H-point is a fixed, valid compressed secp256k1 point");

    // A real, distinct, vendored xpub (WITH an origin) as the other leaf —
    // any second leaf works; this is `preset-kofn-recovery-tr`'s own first
    // `multi_a` key, reused only for its real, valid xpub bytes.
    let other_leaf_display = "[73c5da0a/48'/0'/0'/3']xpub6DXuQW1Q2JpZyweiMewTZuMPvjG8hKhV2qoF6wL9VFxsMBExtbfqAAoR4oMG4GyxFzVdfas1v2eAdfLxyjc4Ceo5B6w6zTpf7F2BuXCJ52i";
    let other_leaf_xpub = bitcoin::bip32::Xpub::from_str(
        "xpub6DXuQW1Q2JpZyweiMewTZuMPvjG8hKhV2qoF6wL9VFxsMBExtbfqAAoR4oMG4GyxFzVdfas1v2eAdfLxyjc4Ceo5B6w6zTpf7F2BuXCJ52i",
    )
    .expect("valid xpub");

    // Leaf order matches the tree built below: `{pk(X),pk(OTHER)}` walks
    // left-to-right, X first.
    let leaves: Vec<[u8; 33]> = vec![
        nums_pubkey.serialize(),
        other_leaf_xpub.public_key.serialize(),
    ];
    let x = md_codec::nums::liana_unspendable_xpub(&leaves, Network::Bitcoin);
    let x_str = x.to_string();

    format!(
        "tr({x_str}/<0;1>/*,{{pk({x_str}/{leaf_use_site}/*),pk({other_leaf_display}/<0;1>/*)}})"
    )
}
