//! Composition, the COMPARISON FORM, and the spend-equality checker.
//!
//! ## Composition
//!
//! Seating a candidate assignment is TWO edits to the keyless policy: fill its
//! `Pubkeys` TLV, and fill an ABSENT per-slot fingerprint from the card that
//! seated that slot. Every other field — the tree, the origin-path
//! declaration, the use-site paths — is the POLICY CARD's and stays untouched,
//! which is what "the declaration is the constraint" means once an assignment
//! exists.
//!
//! ## The fingerprint rule, exactly (composer fable review r0, lens 3, I-2)
//!
//! - **A fingerprint the policy card DECLARES is never overwritten.** That
//!   rule stands unchanged. A card whose fingerprint disagrees cannot reach
//!   here anyway — A2 (`satisfy::satisfies`) refuses it — so the guarantee is
//!   belt and braces, and its test drives the assignment A2 would reject.
//! - **A slot with NO declared fingerprint takes the seated card's, when the
//!   card states one.** This was previously inherited as "the policy author's
//!   choice not to state one". It is not a choice. A composer template minted
//!   with an UNSEATED slot (SPEC §8p) has no fingerprint there because the key
//!   was not known yet, and the host is the moment it becomes known.
//! - **Nothing is invented.** A privacy-preserving card states no master, so
//!   the slot stays fingerprint-free.
//!
//! WHY IT MATTERS, and it is not cosmetic. `to_miniscript` drops the WHOLE
//! origin when the fingerprint is absent (`e.fingerprint.map(…)`), so a
//! host-completed partial template rendered its completed key as a bare xpub
//! with no `[fp/path]` — a BIP-380 key no coordinator and no HWI can attribute
//! to a signer — and the completed wallet's id differed from the id the device
//! mints for the same keys seated on-device. SPEC §5 l.218: "EVERY slot
//! declares an origin (§4f) and, when seated, the master fingerprint of the
//! seated key."
//!
//! A card's fingerprint is carried VERBATIM, all-zero included: `00000000` is
//! md1's absent-master sentinel (md-codec 0.44.1) and transcribing it is what
//! makes the host's id agree with the device's, which writes the same bytes.
//!
//! ## The comparison form (SPEC A3 / THE PRINCIPLE)
//!
//! > compose each candidate assignment, canonicalise FOR COMPARISON (an
//! > internal form that additionally sorts keys within each
//! > `sortedmulti`/`sortedmulti_a` group instance … this form is never
//! > emitted), and byte-compare
//!
//! Byte-equality of this form is SOUND for wallet-equality and deliberately
//! CONSERVATIVE — the converse fails on e.g. taptree branch commutation,
//! measured, which the engine treats as inequality and refuses. Refusing
//! more than the principle requires is intended; refusing less is a defect
//! (r5 M1).
//!
//! Two properties make it work:
//!
//! 1. **The sort.** `sortedmulti`/`sortedmulti_a` sort their keys at script
//!    construction, so two assignments that differ only by a permutation
//!    within one group instance are the same wallet. Sorting the group's
//!    placeholder indices BY THEIR SEATED KEY reproduces that here.
//! 2. **`canonical_payload_bytes`.** md-codec renumbers placeholders by
//!    first appearance before emitting, so the sorted tree's indices land in
//!    a canonical numbering and the per-`@N` TLV maps are permuted with it
//!    atomically. Without that renumbering the sort would move the indices
//!    and leave the keys behind.
//!
//! It is NEVER emitted. `md descriptor` renders the SEATED descriptor
//! through `md_codec::to_miniscript_descriptor_multipath`, which preserves
//! the card's own key order.

use crate::error::CliError;
use crate::seat::input::DecodedCard;
use bitcoin::bip32::Xpub;
use md_codec::canonicalize::expand_per_at_n;
use md_codec::encode::Descriptor;
use md_codec::tag::Tag;
use md_codec::tree::{Body, Node};
use std::collections::BTreeMap;

/// The 65-byte `Pubkeys` TLV payload for an xpub: 32-byte chain code
/// followed by the 33-byte compressed point, the layout
/// `md_codec::derive::xpub_from_tlv_bytes` reads back.
///
/// The xpub's depth, PARENT FINGERPRINT and child number are not part of this
/// payload. Depth and child number are nonetheless recovered on the way back
/// out — `to_miniscript::assemble_origin_and_xkey` reads them off the key
/// origin the card carries — so only the parent fingerprint is genuinely lost,
/// and a rendered key no longer contradicts the origin printed beside it. That
/// residue is the known limitation r1 C3 records for the DECOMPOSE side; on the
/// compose side nothing reads any of them.
pub fn payload_of(xpub: &Xpub) -> [u8; 65] {
    let mut out = [0u8; 65];
    out[..32].copy_from_slice(xpub.chain_code.as_ref());
    out[32..].copy_from_slice(&xpub.public_key.serialize());
    out
}

/// Seat `assignment[slot] = card index` into a copy of the keyless policy.
///
/// See the module doc for the fingerprint rule: an ABSENT one is filled from
/// the seated card, a DECLARED one is never touched.
pub fn compose(
    policy: &Descriptor,
    cards: &[DecodedCard],
    assignment: &[usize],
) -> Result<Descriptor, CliError> {
    let mut seated = policy.clone();
    let mut pubkeys: Vec<(u8, [u8; 65])> = assignment
        .iter()
        .enumerate()
        .map(|(slot, card_idx)| (slot as u8, payload_of(&cards[*card_idx].card.xpub)))
        .collect();
    // md_codec::tlv requires strictly ascending @i.
    pubkeys.sort_by_key(|(i, _)| *i);
    seated.tlv.pubkeys = Some(pubkeys);

    // Fill ABSENT fingerprints only. `assignment` is indexed by @N — the same
    // index the `Pubkeys` TLV above is keyed by — so a slot's declaration is
    // found by that index and nothing is re-derived.
    let mut fingerprints: Vec<(u8, [u8; 4])> = seated.tlv.fingerprints.clone().unwrap_or_default();
    let mut filled = false;
    for (slot, card_idx) in assignment.iter().enumerate() {
        let i = slot as u8;
        if fingerprints.iter().any(|(j, _)| *j == i) {
            // DECLARED. Never overwritten, whatever the card says.
            continue;
        }
        if let Some(fp) = cards[*card_idx].card.origin_fingerprint {
            fingerprints.push((i, fp.to_bytes()));
            filled = true;
        }
    }
    if filled {
        // Strictly ascending @i, as md_codec::tlv requires.
        fingerprints.sort_by_key(|(i, _)| *i);
        seated.tlv.fingerprints = Some(fingerprints);
    }
    Ok(seated)
}

/// Recursively sort the placeholder indices of every `sortedmulti` /
/// `sortedmulti_a` INSTANCE by the key seated in each.
fn sort_sorted_group_instances(node: &mut Node, keys: &BTreeMap<u8, [u8; 65]>) {
    let is_sorted_family = matches!(node.tag, Tag::SortedMulti | Tag::SortedMultiA);
    match &mut node.body {
        Body::MultiKeys { indices, .. } => {
            if is_sorted_family {
                indices.sort_by_key(|i| keys.get(i).copied().unwrap_or([0u8; 65]));
            }
        }
        Body::Tr { tree: Some(t), .. } => sort_sorted_group_instances(t, keys),
        Body::Children(children) | Body::Variable { children, .. } => {
            for c in children {
                sort_sorted_group_instances(c, keys);
            }
        }
        _ => {}
    }
}

/// The internal, never-emitted byte form two candidate compositions are
/// compared in. See the module doc comment for why it is sound.
pub fn comparison_form(seated: &Descriptor) -> Result<Vec<u8>, CliError> {
    let mut d = seated.clone();
    let keys: BTreeMap<u8, [u8; 65]> = d
        .tlv
        .pubkeys
        .as_ref()
        .map(|v| v.iter().copied().collect())
        .unwrap_or_default();
    sort_sorted_group_instances(&mut d.tree, &keys);
    let (bytes, total_bits) = d.canonical_payload_bytes()?;
    // `total_bits` is load-bearing: the final byte may carry up to 7 pad
    // bits, so two payloads of different bit length can share a byte
    // vector. Append it rather than comparing `bytes` alone.
    let mut out = bytes;
    out.extend_from_slice(&(total_bits as u64).to_be_bytes());
    Ok(out)
}

/// The failing half of a **NOT spend-equal** verdict, named per SPEC R3:
/// which of the three properties SPEND-EQUALITY requires (canonicalised
/// template STRUCTURE, per-slot xpub VALUES, per-slot USE-SITE paths —
/// origin metadata excluded throughout) is where the two candidates
/// diverge. `Equal` means all three hold.
///
/// **Checked in this order — VALUES, then USE-SITES, then STRUCTURE —**
/// deliberately the reverse of how it reads above: the per-slot checks are
/// FINE-GRAINED (one xpub, one use-site suffix) while the STRUCTURE check
/// (`comparison_form`'s stripped byte form) is coarse and catches
/// everything the per-slot checks do too, since it serialises the whole
/// descriptor — tree, `Pubkeys` TLV and use-site declarations together —
/// with only origin metadata blanked (see `comparison_form`'s own doc
/// comment). Running it FIRST would report every values-only mismatch as
/// "structure", which SPEC R3's own row ("one-xpub-off … names the values
/// half") rules out. Running the per-slot checks first, and falling back to
/// the byte form only once both pass, gives the byte form its actual job:
/// catching what the per-slot loop cannot see — e.g. a script family swap
/// (`multi` vs `sortedmulti`) at unchanged key values and use-sites
/// (`v_spendeq_multi_and_sortedmulti_are_not_spend_equal`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpendEqualVerdict {
    Equal,
    Structure,
    Values,
    UseSites,
}

impl SpendEqualVerdict {
    pub fn is_equal(self) -> bool {
        matches!(self, Self::Equal)
    }

    /// The name SPEC R3's CLI message states — "structure", "values" or
    /// "use-sites". Never called on `Equal`.
    pub fn failing_half(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Structure => "structure",
            Self::Values => "values",
            Self::UseSites => "use-sites",
        }
    }
}

/// **SPEND-EQUALITY** (SPEC Acceptance 1, the cross-form relation; SPEC B2's
/// split-vs-keyed "agree"), with the failing half named (SPEC R3).
///
/// > canonicalised template structures equal AND per-slot xpub values and
/// > use-site paths equal — origin metadata EXCLUDED (it is seating/signing
/// > guidance, not script content).
///
/// The exclusion is not a convenience. The pathological fixture's split set
/// and keyed card declare DIFFERENT legitimate origin metadata, so an
/// origin-INCLUDING relation fails nine of eleven slots on the spec's own
/// walk (r3 C2). Round-trip-equality — spend-equality AND origin metadata
/// preserved exactly — is the stricter relation, and it belongs to C3's
/// decompose leg.
///
/// R3 wires this — `md descriptor --verify-against` — for the naming; the
/// plain [`spend_equal`] wrapper below stays the boolean entrance the P2
/// tests already pin.
pub fn spend_equal_verdict(a: &Descriptor, b: &Descriptor) -> Result<SpendEqualVerdict, CliError> {
    // Per-slot xpub values and use-site paths, read through the same
    // expansion the identity computation uses. Checked BEFORE the byte
    // form — see the enum's doc comment for why the order is load-bearing.
    let ea = expand_per_at_n(a)?;
    let eb = expand_per_at_n(b)?;
    if ea.len() == eb.len() {
        for (x, y) in ea.iter().zip(eb.iter()) {
            if x.xpub != y.xpub {
                return Ok(SpendEqualVerdict::Values);
            }
            if x.use_site_path != y.use_site_path {
                return Ok(SpendEqualVerdict::UseSites);
            }
        }
    }
    // Structure: the comparison form of each side with the origin metadata
    // blanked, so `sortedmulti` permutation is absorbed exactly as it is in
    // A3 while a `multi` vs `sortedmulti` difference still separates them
    // (r2 C2 (ii) found a key pair the address relation could not). Reached
    // only once the per-slot checks above already agree (or a length
    // mismatch, itself a structural fact) — see the enum's doc comment.
    let stripped = |d: &Descriptor| -> Result<Vec<u8>, CliError> {
        let mut c = d.clone();
        c.tlv.fingerprints = None;
        c.tlv.origin_path_overrides = None;
        c.path_decl = md_codec::origin_path::PathDecl {
            n: c.path_decl.n,
            paths: md_codec::origin_path::PathDeclPaths::Shared(
                md_codec::origin_path::OriginPath { components: vec![] },
            ),
        };
        comparison_form(&c)
    };
    if stripped(a)? != stripped(b)? {
        return Ok(SpendEqualVerdict::Structure);
    }
    Ok(SpendEqualVerdict::Equal)
}

/// The boolean entrance the P2 tests pin — `spend_equal_verdict(a,
/// b)?.is_equal()`.
///
/// **Corrected (review r1 M5) — the claim below was stronger than proven.**
/// Reordering the checks inside `spend_equal_verdict` does not change WHICH
/// PAIRS ARE EQUAL, only which named half a NOT-equal pair reports —
/// PROVIDED both sides actually expand (`expand_per_at_n` succeeds on both).
/// It is NOT unchanged bit for bit for a pair where expansion FAILS:
/// `expand_per_at_n` now runs BEFORE the structural byte compare, so such a
/// pair returns `Err` where the structure-first order would have reached
/// `Ok(SpendEqualVerdict::Structure)` (a length mismatch always changes
/// `comparison_form`'s bytes, so the `ea.len() == eb.len()` skip still gets
/// there) without ever calling `expand_per_at_n`. Unreachable through the
/// CLI today — a `MissingExplicitOrigin` descriptor cannot be minted, so
/// `--verify-against` can never hand this function one — but the original
/// wording covered the error case and did not hold for it.
pub fn spend_equal(a: &Descriptor, b: &Descriptor) -> Result<bool, CliError> {
    Ok(spend_equal_verdict(a, b)?.is_equal())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seat::satisfy::fixture::*;
    use crate::seat::satisfy::{satisfies, slot_declarations};

    /// Seat a fixture by an EXPLICIT assignment.
    fn seat_with(text: &str, assignment: &[usize]) -> Descriptor {
        compose(&policy(text), &cards(text), assignment).unwrap()
    }

    /// Seat a fixture by the unique satisfying card per slot.
    fn seat_unique(text: &str) -> Descriptor {
        let policy = policy(text);
        let cards = cards(text);
        let decls = slot_declarations(&policy).unwrap();
        let assignment: Vec<usize> = decls
            .iter()
            .map(|d| {
                let hits: Vec<usize> = cards
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| satisfies(d, c))
                    .map(|(i, _)| i)
                    .collect();
                assert_eq!(hits.len(), 1, "fixture must seat uniquely for this helper");
                hits[0]
            })
            .collect();
        compose(&policy, &cards, &assignment).unwrap()
    }

    #[test]
    fn composition_touches_only_the_pubkeys_and_absent_fingerprints() {
        // The tree, the origin-path declaration and the use-site paths are
        // the POLICY CARD's and are never edited.
        //
        // The fingerprint assertion is narrower than it was before the I-2
        // fill and says so: it holds here because PATHOLOGICAL declares a
        // fingerprint for EVERY slot, which the first assertion measures
        // rather than assumes -- so there is nothing absent to fill and the
        // TLV really must come through untouched. The case where one IS
        // absent has its own tests below.
        let policy = policy(PATHOLOGICAL);
        let declared = policy.tlv.fingerprints.clone().unwrap_or_default();
        assert_eq!(
            declared.len(),
            usize::from(policy.n),
            "this row's premise: PATHOLOGICAL declares every slot's fingerprint"
        );
        let seated = seat_unique(PATHOLOGICAL);
        assert!(policy.tlv.pubkeys.is_none());
        assert!(seated.is_wallet_policy());
        assert_eq!(seated.n, policy.n);
        assert_eq!(seated.tree, policy.tree);
        assert_eq!(seated.path_decl, policy.path_decl);
        assert_eq!(seated.use_site_path, policy.use_site_path);
        assert_eq!(seated.tlv.fingerprints, policy.tlv.fingerprints);
        assert_eq!(
            seated.tlv.use_site_path_overrides,
            policy.tlv.use_site_path_overrides
        );
    }

    #[test]
    fn comparison_form_absorbs_a_swap_inside_one_sorted_group() {
        // V-BOUND-SEAT's shape: one `sortedmulti`, two interchangeable
        // slots. Both assignments must reach the SAME bytes -- this is the
        // property that lets the boundary's seat side seat.
        let policy = policy(V_BOUND_SEAT);
        let cards = cards(V_BOUND_SEAT);
        let a = compose(&policy, &cards, &[0, 1]).unwrap();
        let b = compose(&policy, &cards, &[1, 0]).unwrap();
        assert_eq!(comparison_form(&a).unwrap(), comparison_form(&b).unwrap());
    }

    #[test]
    fn comparison_form_separates_a_swap_across_different_use_site_paths() {
        // r5's counterexample: the same `sortedmulti`, but the two slots
        // derive at DIFFERENT use-site paths, so sorting cannot recover
        // what derivation already changed. Two wallets, two forms.
        let policy = policy(V_USP);
        let cards = cards(V_USP);
        let a = compose(&policy, &cards, &[0, 1]).unwrap();
        let b = compose(&policy, &cards, &[1, 0]).unwrap();
        assert_ne!(comparison_form(&a).unwrap(), comparison_form(&b).unwrap());
        // ...and the addresses really do differ, so the form is not merely
        // pedantic about bytes nobody spends through.
        assert_ne!(
            a.derive_address(0, 0, bitcoin::Network::Bitcoin).unwrap(),
            b.derive_address(0, 0, bitcoin::Network::Bitcoin).unwrap()
        );
    }

    #[test]
    fn comparison_form_is_never_what_gets_emitted() {
        // The emitted text preserves the CARD's key order; the comparison
        // form sorts. On a fixture where the two disagree, the rendered
        // descriptor must follow the card, not the form.
        let policy = policy(V_BOUND_SEAT);
        let cards = cards(V_BOUND_SEAT);
        let a = compose(&policy, &cards, &[0, 1]).unwrap();
        let b = compose(&policy, &cards, &[1, 0]).unwrap();
        let ta = md_codec::to_miniscript_descriptor_multipath(&a)
            .unwrap()
            .to_string();
        let tb = md_codec::to_miniscript_descriptor_multipath(&b)
            .unwrap()
            .to_string();
        assert_ne!(
            ta, tb,
            "the two seatings render differently even though they are one wallet -- \
             which is exactly why A3 needs a deterministic tie-break"
        );
        assert_eq!(comparison_form(&a).unwrap(), comparison_form(&b).unwrap());
    }

    // ─── V-SPENDEQ ──────────────────────────────────────────────────────

    #[test]
    fn v_spendeq_same_keys_different_origin_metadata_are_spend_equal() {
        // r3 C2 (i)'s construction, as the POSITIVE half: identical keys and
        // script, two different declared origins. Spend-equality must hold;
        // an origin-including relation would fail here, which is the whole
        // reason there are two relations.
        let seated = seat_with(V_BOUND_SEAT, &[0, 1]);
        let mut relabelled = seated.clone();
        relabelled.tlv.fingerprints = Some(vec![(0, [0xAA; 4]), (1, [0xBB; 4])]);
        assert!(spend_equal(&seated, &relabelled).unwrap());
    }

    #[test]
    fn v_spendeq_one_xpub_off_is_not_spend_equal() {
        // The NEGATIVE the roster names. Without it the checker could
        // return `true` unconditionally and still pass the row above.
        let policy = policy(PATHOLOGICAL);
        let cards = cards(PATHOLOGICAL);
        let seated = seat_unique(PATHOLOGICAL);
        let mut wrong = seated.clone();
        let victim = payload_of(&cards[0].card.xpub);
        let intruder = payload_of(&cards[1].card.xpub);
        assert_ne!(victim, intruder);
        let pubkeys = wrong.tlv.pubkeys.as_mut().unwrap();
        let slot = pubkeys
            .iter()
            .position(|(_, p)| *p == victim)
            .expect("the victim key is seated");
        pubkeys[slot].1 = intruder;
        assert!(!spend_equal(&seated, &wrong).unwrap());
        let _ = policy;
    }

    #[test]
    fn v_spendeq_multi_and_sortedmulti_are_not_spend_equal() {
        // r2 C2 (ii): the address relation could not separate
        // `wsh(sortedmulti(2,K1,K3))` from `wsh(multi(2,K3,K1))` at four
        // checkpoints. The structural half of spend-equality does.
        let seated = seat_with(V_BOUND_SEAT, &[0, 1]);
        assert_eq!(seated.tree.tag, Tag::Wsh);
        let mut as_multi = seated.clone();
        if let Body::Children(children) = &mut as_multi.tree.body {
            assert_eq!(children[0].tag, Tag::SortedMulti);
            children[0].tag = Tag::Multi;
        } else {
            panic!("fixture shape: wsh(sortedmulti(...))");
        }
        assert!(!spend_equal(&seated, &as_multi).unwrap());
    }

    // ── the ABSENT-fingerprint fill (composer fable review r0, lens 3, I-2) ──

    /// The declared fingerprint for each slot of a fixture's policy card,
    /// read off the card rather than assumed.
    fn declared_fps(policy: &Descriptor) -> Vec<Option<[u8; 4]>> {
        let decls = policy.tlv.fingerprints.clone().unwrap_or_default();
        (0..policy.n)
            .map(|i| decls.iter().find(|(j, _)| *j == i).map(|(_, f)| *f))
            .collect()
    }

    #[test]
    fn the_c13_fixture_really_is_a_partial_template() {
        // The premise. If the fixture were fully declared, every assertion
        // below would pass for the wrong reason.
        let policy = policy(V_PARTIAL_C13);
        assert_eq!(
            declared_fps(&policy),
            vec![Some(*b"\x73\xc5\xda\x0a"), Some(*b"\xb8\x68\x8d\xf1"), None],
            "@2 must be the UNSEATED slot"
        );
        // ...and the completion card for @2 does carry one.
        let cards = cards(V_PARTIAL_C13);
        assert!(
            cards.iter().any(|c| c.card.origin_fingerprint.is_some()),
            "the host card must state a fingerprint, or there is nothing to inherit"
        );
    }

    #[test]
    fn seating_fills_an_absent_fingerprint_from_the_seated_card() {
        let policy = policy(V_PARTIAL_C13);
        let seated = seat_unique(V_PARTIAL_C13);
        let cards = cards(V_PARTIAL_C13);
        // The card that seats @2 -- found the same way `seat_unique` finds it.
        let decls = slot_declarations(&policy).unwrap();
        let card = cards
            .iter()
            .find(|c| satisfies(&decls[2], c))
            .expect("a card seats @2");
        let want = card.card.origin_fingerprint.expect("the card states one");
        assert_eq!(
            declared_fps(&seated)[2],
            Some(want.to_bytes()),
            "@2's fingerprint was not inherited from the card that seated it"
        );
    }

    #[test]
    fn seating_never_overwrites_a_fingerprint_the_policy_already_declares() {
        // The rule that STANDS. Driven past `satisfies` on purpose: the CLI
        // could never assemble this assignment (A2 refuses a card whose
        // fingerprint disagrees with the declaration), so the only way to
        // prove `compose` itself does not overwrite is to hand it the
        // assignment `satisfies` would have rejected.
        let policy = policy(V_PARTIAL_C13);
        let cards = cards(V_PARTIAL_C13);
        let decls = slot_declarations(&policy).unwrap();
        let wrong = cards
            .iter()
            .position(|c| c.card.origin_fingerprint.map(|f| f.to_bytes()) != decls[0].fingerprint)
            .expect("a card whose fingerprint is not @0's");
        assert!(
            !satisfies(&decls[0], &cards[wrong]),
            "the control must be a card A2 really would refuse at @0"
        );
        let seated = compose(&policy, &cards, &[wrong, 1, 2]).unwrap();
        assert_eq!(
            declared_fps(&seated)[0],
            declared_fps(&policy)[0],
            "a declared fingerprint was overwritten by the seated card"
        );
    }

    #[test]
    fn a_fingerprint_free_card_leaves_an_absent_declaration_absent() {
        // Nothing is invented. A privacy-preserving card states no master, so
        // there is nothing to inherit and the slot stays fingerprint-free --
        // which is also what keeps `md descriptor`'s output honest.
        let policy = policy(V_FPFREE_CARD);
        let cards = cards(V_FPFREE_CARD);
        let fp_free = cards
            .iter()
            .position(|c| c.card.origin_fingerprint.is_none())
            .expect("V-FPFREE-CARD has a privacy-preserving card");
        let mut stripped = policy.clone();
        stripped.tlv.fingerprints = None;
        let seated = compose(&stripped, &cards, &[fp_free, 1]).unwrap();
        assert_eq!(
            declared_fps(&seated)[0],
            None,
            "a fingerprint was invented for a card that states none"
        );
    }
}
