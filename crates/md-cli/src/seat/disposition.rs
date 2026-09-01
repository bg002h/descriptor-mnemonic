//! **PHASE B — B1 stub disposition and B2 oracles.**
//!
//! PHASE A ran before any assignment was chosen. Everything here needs a
//! TOTAL assignment, and nothing here is cited before it can be computed:
//! the composed wallet's `WalletPolicyId` does not exist until the keys are
//! seated, which is precisely why A1 only TRIAGES and B1 disposes (r3 C3).
//!
//! ## B1 — three dispositions, no refusal
//!
//! | condition | disposition |
//! | --- | --- |
//! | a stub matches the composed `WalletPolicyId`'s top 4 bytes | **wallet-confirmed** |
//! | else a stub matches the policy's `WalletDescriptorTemplateId` top 4 | **shape-confirmed** |
//! | else | **unconfirmed — a WARNING** |
//!
//! Wallet-confirmed is a TRUE binding WITHIN CE-1's threat model, which is
//! the ACCIDENTAL one -- a drawer scan, a mixed-up card: such a card matches
//! by 2^-32 accident, so CE-1 is impossible for it. It is NOT a binding
//! against an adversarial minter who knows the cosigner set: `policy_id_stubs`
//! is an any-of `Vec<[u8;4]>`, so someone who can compute the composed id of
//! the wallet that results from substituting their own key can mint a stub
//! for it. That substitution threat is outside CE-1 and is not claimed here. Shape-confirmed carries CE-1's
//! accepted limitation. Unconfirmed is a WARNING and never a hard refusal,
//! because legitimate mismatches are MEASURED: a card minted
//! `--from-md1 <keyed card>` carries a WalletPolicyId-rooted stub
//! (`232214e4` on the fixture) that matches neither the template id
//! (`5b48af35`) nor the WalletPolicyId the composer computes under the split
//! set's own origin declarations (`ced22709`) — same wallet, three values,
//! all legitimate, because WalletPolicyId is origin-sensitive.
//!
//! Both readings are named in the warning and the human check is directed,
//! per SPEC B1's message shape.
//!
//! ## B2 — oracles where they exist, the human where none does
//!
//! With no keyed card to cross-check against, there is no automated oracle
//! at all, so address 0 goes to stderr with the standing instruction. This
//! SURFACES the CE-1 residue for human comparison; nothing in this engine
//! can catch it alone (r3 M1 — "caught" was an overstatement and is
//! withdrawn). The split-vs-keyed branch is
//! [`super::compose::spend_equal`], which C4's acceptance walk drives; the
//! Input-D branch belongs to C3's decompose leg.

use crate::error::CliError;
use crate::seat::input::DecodedCard;
use md_codec::encode::Descriptor;

/// B1's three tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// A stub matches the COMPOSED wallet id — a true binding.
    WalletConfirmed,
    /// A stub matches the policy's shape id. CE-1's limitation applies.
    ShapeConfirmed,
    /// Neither. A warning, never a refusal.
    Unconfirmed,
}

fn top4(bytes: &[u8; 16]) -> [u8; 4] {
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

fn hex4(b: [u8; 4]) -> String {
    format!("{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3])
}

/// Per-card disposition, in card order.
pub fn dispositions(
    policy: &Descriptor,
    seated: &Descriptor,
    cards: &[DecodedCard],
) -> Result<Vec<Disposition>, CliError> {
    let wallet = top4(md_codec::compute_wallet_policy_id(seated)?.as_bytes());
    let shape = top4(md_codec::compute_wallet_descriptor_template_id(policy)?.as_bytes());
    Ok(cards
        .iter()
        .map(|c| {
            if c.card.policy_id_stubs.contains(&wallet) {
                Disposition::WalletConfirmed
            } else if c.card.policy_id_stubs.contains(&shape) {
                Disposition::ShapeConfirmed
            } else {
                Disposition::Unconfirmed
            }
        })
        .collect())
}

/// The stderr notes B1 emits for one seating.
///
/// The two identifiers are stated once at the top so a reader can check the
/// arithmetic themselves. The CONFIRMED tiers are then summarised one line
/// per tier, listing every card's set id — on the eleven-card fixture the
/// per-card form repeated a three-line sentence eleven times, which buries
/// the line that matters. WARNINGS stay one per card: each one is a
/// separate thing to go and check.
pub fn notes(
    policy: &Descriptor,
    seated: &Descriptor,
    cards: &[DecodedCard],
) -> Result<Vec<String>, CliError> {
    let wallet = top4(md_codec::compute_wallet_policy_id(seated)?.as_bytes());
    let shape = top4(md_codec::compute_wallet_descriptor_template_id(policy)?.as_bytes());
    let tiers = dispositions(policy, seated, cards)?;
    let ids = |want: Disposition| -> Vec<String> {
        cards
            .iter()
            .zip(tiers.iter())
            .filter(|(_, t)| **t == want)
            .map(|(c, _)| c.set_id.to_string())
            .collect()
    };
    let mut out = vec![format!(
        "note: composed wallet id {} · policy shape id {}",
        hex4(wallet),
        hex4(shape)
    )];
    let confirmed = ids(Disposition::WalletConfirmed);
    if !confirmed.is_empty() {
        out.push(format!(
            "note: {} card(s) WALLET-CONFIRMED — stub matches this exact composed wallet: {}",
            confirmed.len(),
            confirmed.join(", ")
        ));
    }
    let shaped = ids(Disposition::ShapeConfirmed);
    if !shaped.is_empty() {
        out.push(format!(
            "note: {} card(s) SHAPE-CONFIRMED — stub matches this policy's shape, not this \
             composed wallet; a card minted for a different wallet of the same shape would \
             look identical here: {}",
            shaped.len(),
            shaped.join(", ")
        ));
    }
    for (card, tier) in cards.iter().zip(tiers.iter()) {
        if *tier == Disposition::Unconfirmed {
            out.push(format!(
                "warning: card {}'s stub matches neither this policy's shape id nor the \
                 composed wallet id — minted under different origin metadata (legitimate), \
                 or a different wallet; verify address 0 before trusting.",
                card.set_id
            ));
        }
    }
    Ok(out)
}

/// B2's "otherwise" branch: no oracle exists, so surface address 0 for the
/// human.
pub fn address_zero_note(
    seated: &Descriptor,
    network: bitcoin::Network,
) -> Result<String, CliError> {
    let addr = seated.derive_address(0, 0, network)?.assume_checked();
    Ok(format!(
        "note: address 0 (chain 0, index 0) is {addr} — compare against your wallet \
         software before trusting."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seat::compose::{compose, spend_equal};
    use crate::seat::input::decode_cards;
    use crate::seat::matching::{Outcome, candidates, decide};
    use crate::seat::satisfy::fixture::*;
    use crate::seat::satisfy::slot_declarations;

    /// Seat a fixture's own cards and return everything B1 needs.
    fn seat(text: &str) -> (Descriptor, Descriptor, Vec<DecodedCard>) {
        seat_with(text, &mk1_lines(text))
    }

    fn seat_with(text: &str, mk1: &[String]) -> (Descriptor, Descriptor, Vec<DecodedCard>) {
        let policy = policy(text);
        let decls = slot_declarations(&policy).unwrap();
        let (cards, _) = decode_cards(mk1).unwrap();
        let per_slot = candidates(&decls, &cards);
        let Outcome::Seated(a) = decide(&policy, &decls, &cards, &per_slot).unwrap() else {
            panic!("this fixture must seat")
        };
        let seated = compose(&policy, &cards, &a).unwrap();
        (policy, seated, cards)
    }

    // ─── V-B1-WALLET ────────────────────────────────────────────────────

    #[test]
    fn v_b1_wallet_a_card_stubbed_with_the_composed_wallet_id_is_wallet_confirmed() {
        let (policy, seated, cards) = seat(V_B1_WALLET);
        let tiers = dispositions(&policy, &seated, &cards).unwrap();
        assert_eq!(tiers, vec![Disposition::WalletConfirmed; cards.len()]);
        let notes = notes(&policy, &seated, &cards).unwrap();
        assert!(
            notes.iter().any(|n| n.contains("WALLET-CONFIRMED")),
            "{notes:?}"
        );
        assert!(
            notes.iter().all(|n| !n.starts_with("warning:")),
            "no warning on a confirmed set: {notes:?}"
        );
        // The stub really is the composed wallet id -- read off the
        // composition, not off the fixture's comment.
        let wallet = top4(
            md_codec::compute_wallet_policy_id(&seated)
                .unwrap()
                .as_bytes(),
        );
        assert!(
            cards
                .iter()
                .all(|c| c.card.policy_id_stubs.contains(&wallet))
        );
    }

    // ─── V-B1-SHAPE ─────────────────────────────────────────────────────

    #[test]
    fn v_b1_shape_a_card_stubbed_with_the_template_id_is_shape_confirmed() {
        let (policy, seated, cards) = seat(V_B1_SHAPE);
        let tiers = dispositions(&policy, &seated, &cards).unwrap();
        assert_eq!(tiers, vec![Disposition::ShapeConfirmed; cards.len()]);
        // ...and NOT wallet-confirmed: the two ids differ on this fixture,
        // so the tiers are actually being told apart.
        let wallet = top4(
            md_codec::compute_wallet_policy_id(&seated)
                .unwrap()
                .as_bytes(),
        );
        let shape = top4(
            md_codec::compute_wallet_descriptor_template_id(&policy)
                .unwrap()
                .as_bytes(),
        );
        assert_ne!(wallet, shape);
        let notes = notes(&policy, &seated, &cards).unwrap();
        assert!(
            notes.iter().any(|n| n.contains("SHAPE-CONFIRMED")),
            "{notes:?}"
        );
        assert!(
            notes
                .iter()
                .any(|n| n.contains("different wallet of the same shape")),
            "CE-1's limitation is stated where it applies: {notes:?}"
        );
    }

    #[test]
    fn v_b1_shape_the_pathological_set_is_shape_confirmed_too() {
        // The real 11-card fixture: every card carries 5b48af35, the
        // policy's template id, while the composition's wallet id is
        // ced22709. Both values measured 2026-08-30.
        let (policy, seated, cards) = seat(PATHOLOGICAL);
        assert_eq!(
            dispositions(&policy, &seated, &cards).unwrap(),
            vec![Disposition::ShapeConfirmed; 11]
        );
        let notes = notes(&policy, &seated, &cards).unwrap();
        assert!(
            notes[0].contains("composed wallet id ced22709")
                && notes[0].contains("policy shape id 5b48af35"),
            "{}",
            notes[0]
        );
    }

    // ─── V-B1-WARN ──────────────────────────────────────────────────────

    #[test]
    fn v_b1_warn_the_232214e4_card_warns_with_both_readings_named() {
        let (policy, seated, cards) = seat(V_B1_WARN);
        assert_eq!(
            dispositions(&policy, &seated, &cards).unwrap(),
            vec![Disposition::Unconfirmed; cards.len()]
        );
        let notes = notes(&policy, &seated, &cards).unwrap();
        let warning = notes
            .iter()
            .find(|n| n.starts_with("warning:"))
            .expect("a warning fires");
        // SPEC B1's message shape: both readings named, the human check
        // directed.
        assert!(
            warning.contains("matches neither this policy's shape id nor the composed wallet id"),
            "{warning}"
        );
        assert!(
            warning.contains("minted under different origin metadata (legitimate)"),
            "{warning}"
        );
        assert!(warning.contains("or a different wallet"), "{warning}");
        assert!(
            warning.contains("verify address 0 before trusting"),
            "{warning}"
        );
    }

    #[test]
    fn v_b1_warn_is_a_warning_and_never_a_refusal() {
        // The seating itself succeeded above; assert the descriptor really
        // is emittable, so "warning, not refusal" is a property of the
        // output rather than of the message text.
        let (_, seated, _) = seat(V_B1_WARN);
        assert!(
            md_codec::to_miniscript_descriptor_multipath(&seated)
                .unwrap()
                .to_string()
                .starts_with("wsh(sortedmulti(2,")
        );
    }

    // ─── V-B1-CROSS ─────────────────────────────────────────────────────

    #[test]
    fn v_b1_cross_a_card_carrying_another_wallets_shape_id_warns_without_refusing() {
        let (policy, seated, cards) = seat(V_B1_CROSS);
        assert_eq!(
            dispositions(&policy, &seated, &cards).unwrap(),
            vec![Disposition::Unconfirmed; cards.len()]
        );
        // The stub is a REAL shape id -- the pathological policy's -- so
        // this row is about a card from another wallet, not about a
        // nonsense value.
        let pathological = crate::seat::satisfy::fixture::policy(PATHOLOGICAL);
        let other = top4(
            md_codec::compute_wallet_descriptor_template_id(&pathological)
                .unwrap()
                .as_bytes(),
        );
        assert_eq!(hex4(other), "5b48af35");
        assert!(
            cards
                .iter()
                .all(|c| c.card.policy_id_stubs.contains(&other))
        );
        assert!(
            notes(&policy, &seated, &cards)
                .unwrap()
                .iter()
                .any(|n| n.starts_with("warning:"))
        );
    }

    // ─── V-CE1 ──────────────────────────────────────────────────────────

    #[test]
    fn v_ce1_a_same_stub_foreign_card_seats_and_the_address_differs() {
        // BOTH halves are the row (SPEC acceptance 2). Scoped to cards that
        // are NOT wallet-confirmed: assert that scope too, since a
        // wallet-confirmed card could not reach this state.
        let genuine = mk1_lines(V_CE1);
        let foreign_only = mk1_lines(V_CE1_FOREIGN);

        let (policy, seated_genuine, cards_genuine) = seat_with(V_CE1, &genuine);

        // Swap the card at @0 for the foreign one: same path, same stub,
        // another master.
        let (genuine_cards, _) = decode_cards(&genuine).unwrap();
        let victim = genuine_cards
            .iter()
            .find(|c| c.card.origin_path.to_string().contains("0'/2'"))
            .unwrap()
            .set_id;
        let mut swapped: Vec<String> = genuine
            .iter()
            .filter(|s| crate::seat::input::group_id_of(s).ok() != Some(victim))
            .cloned()
            .collect();
        swapped.extend(foreign_only.clone());

        let (_, seated_foreign, cards_foreign) = seat_with(V_CE1, &swapped);

        // HALF ONE: it seats.
        assert_eq!(cards_foreign.len(), 2);
        // HALF TWO: the derived address differs.
        let a = seated_genuine
            .derive_address(0, 0, bitcoin::Network::Bitcoin)
            .unwrap()
            .assume_checked();
        let b = seated_foreign
            .derive_address(0, 0, bitcoin::Network::Bitcoin)
            .unwrap()
            .assume_checked();
        assert_ne!(
            a, b,
            "a different key was seated, so it is a different wallet"
        );

        // SCOPE: neither seating is wallet-confirmed, so CE-1's limitation
        // is the one that applies.
        for (p, s, c) in [
            (&policy, &seated_genuine, &cards_genuine),
            (&policy, &seated_foreign, &cards_foreign),
        ] {
            assert!(
                dispositions(p, s, c)
                    .unwrap()
                    .iter()
                    .all(|d| *d != Disposition::WalletConfirmed),
                "a wallet-confirmed card could not be foreign in the first place"
            );
        }

        // And B2's residue-surfacing note carries the standing instruction,
        // which is the only thing that can catch this.
        let note = address_zero_note(&seated_foreign, bitcoin::Network::Bitcoin).unwrap();
        assert!(note.contains(&b.to_string()), "{note}");
        assert!(
            note.contains("compare against your wallet software before trusting"),
            "{note}"
        );
    }

    // ─── V-SPENDEQ, the cross-form half ─────────────────────────────────

    #[test]
    fn v_spendeq_the_split_set_and_the_keyed_card_are_spend_equal() {
        let (_, split, _) = seat(V_B1_WALLET);
        let keyed_strings = md1_lines(V_SPENDEQ_KEYED);
        let refs: Vec<&str> = keyed_strings.iter().map(String::as_str).collect();
        let keyed = if refs.len() == 1 {
            md_codec::decode_md1_string(refs[0]).unwrap()
        } else {
            md_codec::reassemble(&refs).unwrap()
        };
        assert!(keyed.is_wallet_policy(), "the keyed card carries its keys");

        // The two forms declare DIFFERENT origin metadata...
        assert_ne!(split.tlv.fingerprints, keyed.tlv.fingerprints);
        assert_ne!(
            md_codec::to_miniscript_descriptor_multipath(&split)
                .unwrap()
                .to_string(),
            md_codec::to_miniscript_descriptor_multipath(&keyed)
                .unwrap()
                .to_string(),
            "so the rendered descriptors differ"
        );
        // ...and are still SPEND-EQUAL, which is why acceptance 1 needs two
        // relations rather than one (r3 C2).
        assert!(spend_equal(&split, &keyed).unwrap());
        // Same addresses, as confirmation rather than as the definition.
        assert_eq!(
            split
                .derive_address(0, 0, bitcoin::Network::Bitcoin)
                .unwrap(),
            keyed
                .derive_address(0, 0, bitcoin::Network::Bitcoin)
                .unwrap()
        );
    }
}
