//! **A4 — completeness is total.**
//!
//! > Every slot filled, every supplied key seated. Unfilled slot: refuse
//! > naming the slot and its declared origin. Leftover key: refuse naming
//! > the card AND its stub (the drawer-scan operator's question is "which
//! > wallet do these extras belong to").
//!
//! A3 reports [`super::matching::Outcome::NoPerfectMatching`] without saying
//! WHY; this module answers that, and it is the half a restoring operator
//! actually reads. Both sides are reported in ONE refusal when both are
//! non-empty — a foreign card standing in for a missing genuine one produces
//! exactly that pair, and naming only one of them describes half of what
//! happened.
//!
//! The diagnosis runs off a MAXIMUM matching rather than off "slots with no
//! candidate at all". The cheap version is wrong on a Hall deficiency: three
//! slots and three cards where two cards fit only slot 0 has every slot
//! holding a candidate and every card holding a slot, and still no perfect
//! matching. Kuhn's algorithm over slots in index order, candidates in card
//! order, makes the answer both correct and deterministic.
//!
//! `--seat` NEVER fills a gap here (SPEC A5). It restricts a slot's
//! candidates; it cannot invent a card.

use crate::error::CliError;
use crate::seat::input::DecodedCard;
use crate::seat::satisfy::SlotDecl;

/// Kuhn's augmenting-path maximum bipartite matching. Returns
/// `slot -> Some(card index)` for every matched slot.
fn maximum_matching(per_slot: &[Vec<usize>], n_cards: usize) -> Vec<Option<usize>> {
    let mut card_to_slot: Vec<Option<usize>> = vec![None; n_cards];
    for slot in 0..per_slot.len() {
        let mut seen = vec![false; n_cards];
        try_augment(per_slot, slot, &mut seen, &mut card_to_slot);
    }
    let mut slot_to_card = vec![None; per_slot.len()];
    for (card, slot) in card_to_slot.iter().enumerate() {
        if let Some(s) = slot {
            slot_to_card[*s] = Some(card);
        }
    }
    slot_to_card
}

fn try_augment(
    per_slot: &[Vec<usize>],
    slot: usize,
    seen: &mut Vec<bool>,
    card_to_slot: &mut Vec<Option<usize>>,
) -> bool {
    for &card in &per_slot[slot] {
        if seen[card] {
            continue;
        }
        seen[card] = true;
        let free_or_reroutable = match card_to_slot[card] {
            None => true,
            Some(other) => try_augment(per_slot, other, seen, card_to_slot),
        };
        if free_or_reroutable {
            card_to_slot[card] = Some(slot);
            return true;
        }
    }
    false
}

/// Build A4's refusal for a card set with no perfect matching.
pub fn refusal(decls: &[SlotDecl], cards: &[DecodedCard], per_slot: &[Vec<usize>]) -> CliError {
    let matched = maximum_matching(per_slot, cards.len());
    let unfilled: Vec<&SlotDecl> = decls
        .iter()
        .enumerate()
        .filter(|(i, _)| matched[*i].is_none())
        .map(|(_, d)| d)
        .collect();
    let seated: Vec<usize> = matched.iter().flatten().copied().collect();
    let leftover: Vec<&DecodedCard> = cards
        .iter()
        .enumerate()
        .filter(|(i, _)| !seated.contains(i))
        .map(|(_, c)| c)
        .collect();

    let mut out = format!(
        "this card set does not seat. Completeness is total: every slot must be filled and \
         every supplied card must be seated. {} slot(s) unfilled, {} card(s) left over \
         ({} slots, {} cards supplied).",
        unfilled.len(),
        leftover.len(),
        decls.len(),
        cards.len()
    );
    if !unfilled.is_empty() {
        out.push_str("\nUnfilled slots — no supplied card satisfies the declared origin:");
        for d in &unfilled {
            out.push_str(&format!("\n  {}", d.label()));
        }
    }
    if !leftover.is_empty() {
        out.push_str(
            "\nCards left over — which wallet do these belong to? Each is named by its full \
             chunk-set id and the policy-id stub it was minted against:",
        );
        for c in &leftover {
            let origin = match c.card.origin_fingerprint {
                Some(fp) => format!(
                    "[{fp}/{}]",
                    c.card.origin_path.to_string().trim_start_matches("m/")
                ),
                None => format!(
                    "[{}] (privacy-preserving, no fingerprint)",
                    c.card.origin_path.to_string().trim_start_matches("m/")
                ),
            };
            out.push_str(&format!("\n  {} declaring origin {origin}", c.label()));
        }
    }
    CliError::Seat(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seat::input::decode_cards;
    use crate::seat::matching::{Outcome, candidates, decide};
    use crate::seat::satisfy::fixture::*;
    use crate::seat::satisfy::slot_declarations;

    /// Run A3 then A4, the way the engine will.
    fn diagnose(policy_text: &str, mk1: &[String]) -> String {
        let policy = policy(policy_text);
        let decls = slot_declarations(&policy).unwrap();
        let (cards, _) = decode_cards(mk1).unwrap();
        let per_slot = candidates(&decls, &cards);
        match decide(&policy, &decls, &cards, &per_slot).unwrap() {
            Outcome::NoPerfectMatching => refusal(&decls, &cards, &per_slot).to_string(),
            Outcome::Seated(_) => panic!("this fixture must NOT seat"),
        }
    }

    // ─── V-UNFILLED ─────────────────────────────────────────────────────

    #[test]
    fn v_unfilled_a_missing_card_names_the_slot_and_its_declared_origin() {
        // The fixture is the pathological set with the card for
        // [73c5da0a/48'/0'/3'/2'] removed — dropped by DECODING each card,
        // not by slicing lines (generate.sh; chunk membership is a
        // property of the string-layer header).
        let policy = policy(V_UNFILLED);
        let decls = slot_declarations(&policy).unwrap();
        let (cards, _) = decode_cards(&mk1_lines(V_UNFILLED)).unwrap();
        assert_eq!(cards.len(), 10, "ten of the eleven cards");
        let victim = decls
            .iter()
            .find(|d| {
                !cards.iter().any(|c| {
                    c.card.origin_path == d.path
                        && c.card.origin_fingerprint.map(|f| *f.as_bytes()) == d.fingerprint
                })
            })
            .expect("exactly one declaration has no card");

        let msg = diagnose(V_UNFILLED, &mk1_lines(V_UNFILLED));
        assert!(msg.contains("1 slot(s) unfilled"), "{msg}");
        assert!(msg.contains("0 card(s) left over"), "{msg}");
        assert!(msg.contains("11 slots, 10 cards supplied"), "{msg}");
        assert!(
            msg.contains(&victim.label()),
            "names the slot + origin: {msg}"
        );
        assert!(msg.contains("48'/0'/3'/2'"), "{msg}");
        assert!(
            msg.contains("Unfilled slots — no supplied card satisfies the declared origin"),
            "{msg}"
        );
    }

    // ─── V-LEFTOVER ─────────────────────────────────────────────────────

    #[test]
    fn v_leftover_an_extra_foreign_card_names_the_card_and_its_stub() {
        let mut strings = mk1_lines(PATHOLOGICAL);
        strings.extend(mk1_lines(V_LEFTOVER));
        let msg = diagnose(PATHOLOGICAL, &strings);
        assert!(msg.contains("0 slot(s) unfilled"), "{msg}");
        assert!(msg.contains("1 card(s) left over"), "{msg}");
        assert!(msg.contains("11 slots, 12 cards supplied"), "{msg}");
        assert!(msg.contains("which wallet do these belong to"), "{msg}");
        // Named by full chunk-set id AND stub.
        let (extra, _) = decode_cards(&mk1_lines(V_LEFTOVER)).unwrap();
        assert_eq!(extra.len(), 1);
        assert!(msg.contains(&extra[0].label()), "{msg}");
        assert!(
            msg.contains("48'/0'/9'/2'"),
            "names its declared origin: {msg}"
        );
    }

    // ─── V-FPFREE-CARD (the refusal half) ───────────────────────────────

    #[test]
    fn v_fpfree_card_leaves_its_slot_unfilled_and_itself_over() {
        let msg = diagnose(V_FPFREE_CARD, &mk1_lines(V_FPFREE_CARD));
        assert!(msg.contains("1 slot(s) unfilled"), "{msg}");
        assert!(msg.contains("1 card(s) left over"), "{msg}");
        assert!(msg.contains("@0 [73c5da0a/48'/0'/0'/2']"), "{msg}");
        assert!(
            msg.contains("privacy-preserving, no fingerprint"),
            "the leftover card's own shape is named, which is the whole reason \
             the slot could not be filled: {msg}"
        );
    }

    // ─── the Hall-deficiency case the cheap diagnosis gets wrong ────────

    #[test]
    fn a4_diagnoses_a_hall_deficiency_where_every_slot_has_a_candidate() {
        // 3 slots, 3 cards; cards 0 and 1 fit ONLY slot 0. Every slot has a
        // candidate and every card has a slot, so a "no candidate at all"
        // diagnosis would report nothing at all.
        let per_slot = vec![vec![0, 1, 2], vec![2], vec![2]];
        let matched = maximum_matching(&per_slot, 3);
        assert_eq!(matched.iter().flatten().count(), 2);
        assert_eq!(
            matched.iter().filter(|m| m.is_none()).count(),
            1,
            "one slot must be reported unfilled"
        );
    }

    #[test]
    fn a4_maximum_matching_is_deterministic() {
        let per_slot = vec![vec![0, 1], vec![0, 1], vec![0, 1, 2]];
        let first = maximum_matching(&per_slot, 3);
        for _ in 0..5 {
            assert_eq!(maximum_matching(&per_slot, 3), first);
        }
    }
}
