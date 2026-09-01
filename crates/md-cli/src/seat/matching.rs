//! **A3 — THE PRINCIPLE, executable.**
//!
//! > a card set seats without operator input iff every complete candidate
//! > assignment composes to the SAME WALLET — and that is DECIDED BY
//! > COMPOSING, never by structural shortcuts.
//!
//! Three review rounds discovered invariance axes one counterexample at a
//! time — order (r2), group choice (r3), multiplicity and occurrence (r4),
//! use-site paths (r5) — each time by extending a STRUCTURAL predicate along
//! the axis just measured. The cure is that the check IS the principle, so
//! no axis can be missed. There is deliberately no interchangeability
//! predicate anywhere in this module.
//!
//! The procedure:
//!
//! 1. Enumerate the PERFECT MATCHINGS of A2's satisfaction graph.
//! 2. Zero ⇒ A4's refusals identify the unsatisfied side ([`super::complete`]
//!    once step 4 lands; here it is reported as
//!    [`Outcome::NoPerfectMatching`]).
//! 3. Exactly one ⇒ seat it.
//! 4. Several ⇒ compose each, take [`super::compose::comparison_form`], and
//!    byte-compare. All equal ⇒ seat the CANONICAL matching. Any pair
//!    unequal ⇒ the ambiguity refusal.
//!
//! **The bound is on TOTAL matchings enumerated — 720, early-terminating at
//! the 721st.** A per-class `k!` bound neither bounds the work nor tracks it
//! (r6 I2): two independent 6-card components are 518,400 matchings with no
//! component over 6, while an 8-card path component has 2. V-CAP is the
//! first of those, built so the distinction is measured rather than argued.
//!
//! **The tie-break is the ASSIGNMENT VECTOR** — the slot-ordered list of
//! seated chunk-set ids — lexicographically least. Ordering by the
//! comparison FORM cannot discriminate (r7 I1): that branch is entered
//! precisely when all forms are byte-equal, while the emitted descriptors
//! and their WalletPolicyIds still differ. Assignment vectors differ between
//! distinct matchings by construction, so this order is total AND
//! discriminating.

use crate::error::CliError;
use crate::seat::compose::{comparison_form, compose};
use crate::seat::input::{DecodedCard, GroupId};
use crate::seat::satisfy::{SlotDecl, satisfies};
use md_codec::encode::Descriptor;

/// SPEC A3's enumeration bound: 720 TOTAL perfect matchings, refusing on the
/// 721st.
pub const MATCHING_BOUND: usize = 720;

/// What A3 decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// A total assignment `slot -> card index`, ready for PHASE B.
    Seated(Vec<usize>),
    /// No perfect matching exists. A4 diagnoses which side is unsatisfied.
    NoPerfectMatching,
}

/// Result of the bounded enumeration.
enum Enumerated {
    /// Every perfect matching, at most [`MATCHING_BOUND`] of them.
    All(Vec<Vec<usize>>),
    /// A 721st matching was reached; enumeration stopped there.
    OverBound,
}

/// Per-slot candidate card indices under A2.
pub fn candidates(decls: &[SlotDecl], cards: &[DecodedCard]) -> Vec<Vec<usize>> {
    decls
        .iter()
        .map(|d| {
            cards
                .iter()
                .enumerate()
                .filter(|(_, c)| satisfies(d, c))
                .map(|(i, _)| i)
                .collect()
        })
        .collect()
}

/// Enumerate perfect matchings by depth-first assignment of slots in index
/// order, bounded at [`MATCHING_BOUND`].
///
/// A perfect matching covers BOTH sides — every slot filled and every card
/// seated (A4: "completeness is total") — so a card count differing from the
/// slot count has none, and the search says so immediately rather than
/// enumerating maximal-but-not-perfect assignments nobody asked for.
///
/// Forward checking (every unassigned slot must still have an unused
/// candidate) is what keeps the search from paying for dead ends before it
/// reaches the bound; without it a graph with many candidates and few
/// matchings can burn exponential time under a bound expressed in COMPLETE
/// matchings.
fn enumerate(per_slot: &[Vec<usize>], n_cards: usize) -> Enumerated {
    let n_slots = per_slot.len();
    if n_slots != n_cards {
        return Enumerated::All(Vec::new());
    }
    let mut used = vec![false; n_cards];
    let mut current = vec![usize::MAX; n_slots];
    let mut found: Vec<Vec<usize>> = Vec::new();
    let mut over = false;
    dfs(per_slot, 0, &mut used, &mut current, &mut found, &mut over);
    if over {
        Enumerated::OverBound
    } else {
        Enumerated::All(found)
    }
}

fn dfs(
    per_slot: &[Vec<usize>],
    slot: usize,
    used: &mut Vec<bool>,
    current: &mut Vec<usize>,
    found: &mut Vec<Vec<usize>>,
    over: &mut bool,
) {
    if *over {
        return;
    }
    if slot == per_slot.len() {
        if found.len() == MATCHING_BOUND {
            // This is the 721st. Stop here, having enumerated the bound.
            *over = true;
            return;
        }
        found.push(current.clone());
        return;
    }
    for &card in &per_slot[slot] {
        if used[card] {
            continue;
        }
        used[card] = true;
        current[slot] = card;
        if forward_check(per_slot, slot + 1, used) {
            dfs(per_slot, slot + 1, used, current, found, over);
        }
        used[card] = false;
        current[slot] = usize::MAX;
        if *over {
            return;
        }
    }
}

fn forward_check(per_slot: &[Vec<usize>], from: usize, used: &[bool]) -> bool {
    per_slot[from..]
        .iter()
        .all(|cands| cands.iter().any(|c| !used[*c]))
}

/// The slot-ordered list of seated chunk-set ids — A3's tie-break key.
///
/// SPEC §4 extends this key with the ordinal (`(GroupId, Option<u32>)`
/// rather than bare `GroupId`): two candidate matchings can now assign
/// DIFFERENT physical cards that happen to share one collided `set_id`
/// (e.g. `12345#1` to one matching, `12345#2` to another) to the SAME
/// slot. Without the ordinal, both would render the same key and the
/// tie-break would stop discriminating between them at exactly the
/// moment two DIFFERENT cards are in play — the ordinal is what keeps it
/// total over every axis SPEC A3(a)'s principle names.
fn assignment_vector(cards: &[DecodedCard], assignment: &[usize]) -> Vec<(GroupId, Option<u32>)> {
    assignment
        .iter()
        .map(|c| (cards[*c].set_id, cards[*c].ordinal))
        .collect()
}

/// Run A3.
pub fn decide(
    policy: &Descriptor,
    decls: &[SlotDecl],
    cards: &[DecodedCard],
    per_slot: &[Vec<usize>],
) -> Result<Outcome, CliError> {
    match enumerate(per_slot, cards.len()) {
        Enumerated::OverBound => Err(over_bound_refusal(decls, cards, per_slot)),
        Enumerated::All(matchings) if matchings.is_empty() => Ok(Outcome::NoPerfectMatching),
        Enumerated::All(matchings) if matchings.len() == 1 => Ok(Outcome::Seated(
            matchings.into_iter().next().expect("len 1"),
        )),
        Enumerated::All(matchings) => {
            // COMPOSE, canonicalise for comparison, byte-compare. Never a
            // structural shortcut.
            let first = comparison_form(&compose(policy, cards, &matchings[0])?)?;
            for m in &matchings[1..] {
                if comparison_form(&compose(policy, cards, m)?)? != first {
                    return Err(ambiguity_refusal(decls, cards, per_slot, matchings.len()));
                }
            }
            // Every candidate is the same wallet. Seat the canonical one:
            // lexicographically least assignment vector.
            let best = matchings
                .into_iter()
                .min_by_key(|m| assignment_vector(cards, m))
                .expect("at least two matchings");
            Ok(Outcome::Seated(best))
        }
    }
}

/// Cards that could sit in more than one slot, each with its candidate
/// slots — the graph property both multi-candidate refusals name. Available
/// even when the matchings themselves are uncounted (r6 M3), which is why
/// the cap refusal can print it.
fn multi_slot_cards(
    decls: &[SlotDecl],
    cards: &[DecodedCard],
    per_slot: &[Vec<usize>],
) -> Vec<String> {
    let mut lines = Vec::new();
    for (ci, card) in cards.iter().enumerate() {
        let slots: Vec<String> = per_slot
            .iter()
            .enumerate()
            .filter(|(_, cands)| cands.contains(&ci))
            .map(|(si, _)| decls[si].label())
            .collect();
        if slots.len() > 1 {
            lines.push(format!("  card {} -> {}", card.label(), slots.join(", ")));
        }
    }
    lines
}

const REMEDIES: &str = "Two remedies, both of which make the seating an assertion rather than a \
     guess:\n  (1) re-mint the POLICY card with one --fingerprint per slot. It costs about \
     one extra md1 chunk and changes no path, no key and no policy — it only makes the \
     slots tell apart.\n  (2) assert the seating yourself with --seat '@i=<chunk-set-id>' \
     (repeatable; add '#<k>' for a chunk-set id that auto-partitioned into several collided \
     cards, e.g. '@0=12345#1'), using the ids (and, where shown, the '#<k>' labels) printed \
     above. A --seat must still satisfy the slot's declared origin, so it can never place a \
     card the engine would not.";

fn ambiguity_refusal(
    decls: &[SlotDecl],
    cards: &[DecodedCard],
    per_slot: &[Vec<usize>],
    count: usize,
) -> CliError {
    CliError::Seat(format!(
        "this card set has {count} complete candidate assignments and they do NOT all \
         compose to the same wallet, so seating it would be a guess about which wallet \
         you meant.\nCards that fit more than one slot:\n{}\n{REMEDIES}",
        multi_slot_cards(decls, cards, per_slot).join("\n")
    ))
}

fn over_bound_refusal(
    decls: &[SlotDecl],
    cards: &[DecodedCard],
    per_slot: &[Vec<usize>],
) -> CliError {
    CliError::Seat(format!(
        "this card set admits more than {MATCHING_BOUND} complete candidate assignments — \
         the enumeration bound — so they cannot all be composed and compared, and seating \
         one would be a guess.\nCards that fit more than one slot:\n{}\n{REMEDIES}",
        multi_slot_cards(decls, cards, per_slot).join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seat::satisfy::fixture::*;
    use crate::seat::satisfy::slot_declarations;

    struct Case {
        policy: Descriptor,
        decls: Vec<SlotDecl>,
        cards: Vec<DecodedCard>,
        per_slot: Vec<Vec<usize>>,
    }

    fn case(text: &str) -> Case {
        let policy = policy(text);
        let decls = slot_declarations(&policy).unwrap();
        let cards = cards(text);
        let per_slot = candidates(&decls, &cards);
        Case {
            policy,
            decls,
            cards,
            per_slot,
        }
    }

    fn run(c: &Case) -> Result<Outcome, CliError> {
        decide(&c.policy, &c.decls, &c.cards, &c.per_slot)
    }

    fn refusal(text: &str) -> String {
        run(&case(text))
            .expect_err("this fixture must refuse")
            .to_string()
    }

    fn rendered(c: &Case, out: &Outcome) -> String {
        let Outcome::Seated(a) = out else {
            panic!("expected a seating")
        };
        md_codec::to_miniscript_descriptor_multipath(&compose(&c.policy, &c.cards, a).unwrap())
            .unwrap()
            .to_string()
    }

    // ─── V-ORD ──────────────────────────────────────────────────────────

    #[test]
    fn v_ord_three_supply_orders_give_identical_descriptor_bytes() {
        let text = PATHOLOGICAL;
        let mk1: Vec<String> = mk1_lines(text);
        let orders: Vec<Vec<String>> = vec![mk1.clone(), mk1.iter().rev().cloned().collect(), {
            // interleave halves — a third order that is neither the
            // supplied one nor its reverse
            let (a, b) = mk1.split_at(mk1.len() / 2);
            a.iter()
                .zip(b.iter())
                .flat_map(|(x, y)| [y.clone(), x.clone()])
                .collect()
        }];
        let policy = policy(text);
        let decls = slot_declarations(&policy).unwrap();
        let mut renders = Vec::new();
        for order in &orders {
            let (cards, _) = crate::seat::input::decode_cards(order).unwrap();
            let per_slot = candidates(&decls, &cards);
            let out = decide(&policy, &decls, &cards, &per_slot).unwrap();
            let Outcome::Seated(a) = out else {
                panic!("the pathological set must seat")
            };
            renders.push(
                md_codec::to_miniscript_descriptor_multipath(
                    &compose(&policy, &cards, &a).unwrap(),
                )
                .unwrap()
                .to_string(),
            );
        }
        assert_eq!(renders[0], renders[1]);
        assert_eq!(renders[0], renders[2]);
        assert!(renders[0].starts_with("wsh(or_i("));
    }

    // ─── V-R2-ORD ───────────────────────────────────────────────────────

    #[test]
    fn v_r2_ord_refuses_identically_in_three_supply_orders() {
        let text = V_R2_ORD;
        let mk1 = mk1_lines(text);
        let policy = policy(text);
        let decls = slot_declarations(&policy).unwrap();
        let orders: Vec<Vec<String>> = vec![mk1.clone(), mk1.iter().rev().cloned().collect(), {
            let mut v = mk1.clone();
            v.rotate_left(3);
            v
        }];
        let mut messages = Vec::new();
        for order in &orders {
            let (cards, _) = crate::seat::input::decode_cards(order).unwrap();
            let per_slot = candidates(&decls, &cards);
            let err = decide(&policy, &decls, &cards, &per_slot)
                .expect_err("r2's four-card two-group set must refuse");
            messages.push(err.to_string());
        }
        // r1 I1: the VERDICT is order-invariant, not merely the bytes.
        assert_eq!(messages[0], messages[1]);
        assert_eq!(messages[0], messages[2]);
        assert!(
            messages[0].contains("24 complete candidate assignments"),
            "{}",
            messages[0]
        );
        assert!(messages[0].contains("--seat"), "{}", messages[0]);
    }

    // ─── V-R4-IK / V-GRP / V-USP ────────────────────────────────────────

    #[test]
    fn v_r4_ik_internal_key_repartition_refuses() {
        let msg = refusal(V_R4_IK);
        assert!(msg.contains("120 complete candidate assignments"), "{msg}");
        assert!(
            msg.contains("do NOT all compose to the same wallet"),
            "{msg}"
        );
    }

    #[test]
    fn v_grp_two_groups_of_different_arity_refuse() {
        let msg = refusal(V_GRP);
        assert!(
            msg.contains("do NOT all compose to the same wallet"),
            "{msg}"
        );
        assert!(msg.contains("re-mint the POLICY card"), "{msg}");
        assert!(msg.contains("--seat '@i=<chunk-set-id>'"), "{msg}");
    }

    #[test]
    fn v_usp_use_site_path_swap_refuses() {
        let msg = refusal(V_USP);
        assert!(msg.contains("2 complete candidate assignments"), "{msg}");
        // Cards named by their FULL chunk-set id, and slots by index +
        // declared origin.
        let c = case(V_USP);
        for card in &c.cards {
            assert!(
                msg.contains(&card.set_id.to_string()),
                "names {}: {msg}",
                card.set_id
            );
        }
        assert!(msg.contains("@0 ["), "{msg}");
        assert!(msg.contains("@1 ["), "{msg}");
    }

    // ─── V-BOUND-SEAT / V-MIX (the must-SEAT side) ──────────────────────

    #[test]
    fn v_bound_seat_fp_free_same_path_different_masters_seats() {
        let c = case(V_BOUND_SEAT);
        assert_eq!(
            c.per_slot,
            vec![vec![0, 1], vec![0, 1]],
            "genuinely ambiguous graph"
        );
        let out = run(&c).expect("both assignments are one wallet, so it must SEAT");
        let Outcome::Seated(a) = &out else { panic!() };
        // The tie-break: lexicographically least assignment vector.
        let chosen = assignment_vector(&c.cards, a);
        let other: Vec<usize> = a.iter().rev().copied().collect();
        assert!(chosen < assignment_vector(&c.cards, &other));
        assert!(rendered(&c, &out).starts_with("wsh(sortedmulti(2,"));
    }

    #[test]
    fn v_bound_seat_the_choice_is_deterministic_under_reordered_input() {
        // The tie-break must not depend on how the cards arrived.
        let text = V_BOUND_SEAT;
        let mk1 = mk1_lines(text);
        let policy = policy(text);
        let decls = slot_declarations(&policy).unwrap();
        let mut renders = Vec::new();
        for order in [mk1.clone(), mk1.iter().rev().cloned().collect::<Vec<_>>()] {
            let (cards, _) = crate::seat::input::decode_cards(&order).unwrap();
            let per_slot = candidates(&decls, &cards);
            let Outcome::Seated(a) = decide(&policy, &decls, &cards, &per_slot).unwrap() else {
                panic!()
            };
            renders.push(
                md_codec::to_miniscript_descriptor_multipath(
                    &compose(&policy, &cards, &a).unwrap(),
                )
                .unwrap()
                .to_string(),
            );
        }
        assert_eq!(renders[0], renders[1]);
    }

    #[test]
    fn v_mix_mixed_declarations_with_a_unique_matching_seat() {
        let c = case(V_MIX);
        // @0 declares a fingerprint (one candidate); @1 declares none (two).
        assert_eq!(c.per_slot[0].len(), 1);
        assert_eq!(c.per_slot[1].len(), 2);
        let out = run(&c).expect("a unique perfect matching must seat");
        let Outcome::Seated(a) = &out else { panic!() };
        assert_eq!(a[0], c.per_slot[0][0]);
        assert_ne!(a[0], a[1]);
        assert!(rendered(&c, &out).starts_with("wsh(multi(2,"));
    }

    // ─── V-CAP ──────────────────────────────────────────────────────────

    #[test]
    fn v_cap_two_independent_six_card_components_refuse_at_the_bound() {
        let c = case(V_CAP);
        assert_eq!(c.cards.len(), 12);
        assert_eq!(c.decls.len(), 12);
        // 6! x 6! = 518,400 matchings, with no component over 6 -- the
        // shape a per-class k! bound neither bounds nor tracks (r6 I2).
        assert!(c.per_slot.iter().all(|s| s.len() == 6));
        let msg = run(&c).expect_err("over the bound").to_string();
        assert!(
            msg.contains("more than 720 complete candidate assignments"),
            "{msg}"
        );
        assert!(msg.contains("Cards that fit more than one slot"), "{msg}");
        for card in &c.cards {
            assert!(
                msg.contains(&card.set_id.to_string()),
                "names every card: {msg}"
            );
        }
        assert!(msg.contains("--seat"), "{msg}");
    }

    #[test]
    fn v_cap_the_bound_is_on_total_matchings_not_per_component() {
        // A per-class bound would see two components of six and pass. This
        // asserts the enumerator itself stops at the 721st.
        let c = case(V_CAP);
        match enumerate(&c.per_slot, c.cards.len()) {
            Enumerated::OverBound => {}
            Enumerated::All(v) => panic!("enumerated {} matchings without stopping", v.len()),
        }
    }

    #[test]
    fn v_bound_seat_a_set_at_exactly_the_bound_still_enumerates() {
        // 720 = 6!, the largest count that is NOT over the bound. Built
        // directly on the graph so the boundary is pinned from below as
        // well as above -- an off-by-one that refused at 720 would pass
        // every other row in this file.
        let per_slot: Vec<Vec<usize>> = (0..6).map(|_| (0..6).collect()).collect();
        match enumerate(&per_slot, 6) {
            Enumerated::All(v) => assert_eq!(v.len(), 720),
            Enumerated::OverBound => panic!("720 is within the bound, not over it"),
        }
        let per_slot7: Vec<Vec<usize>> = (0..7).map(|_| (0..7).collect()).collect();
        match enumerate(&per_slot7, 7) {
            Enumerated::OverBound => {}
            Enumerated::All(v) => panic!("7! = 5040 must trip the bound, got {}", v.len()),
        }
    }

    #[test]
    fn a_card_count_mismatch_has_no_perfect_matching() {
        let c = case(PATHOLOGICAL);
        let short: Vec<DecodedCard> = c.cards[..10].to_vec();
        let per_slot = candidates(&c.decls, &short);
        assert_eq!(
            decide(&c.policy, &c.decls, &short, &per_slot).unwrap(),
            Outcome::NoPerfectMatching
        );
    }
}
