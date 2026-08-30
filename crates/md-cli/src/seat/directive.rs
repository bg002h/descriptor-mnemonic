//! **A5 — `--seat '@i=<chunk-set-id>'`, defined.**
//!
//! > the referenced card must satisfy slot i's declaration under A2 — that
//! > clause carries the whole safety argument, so a consistent `--seat` on a
//! > NON-ambiguous slot is simply satisfied … `--seat` can never place a
//! > card A2 would not, never suppresses A1/B1 stub dispositions, and never
//! > fills A4 gaps.
//!
//! Three consequences the code has to keep:
//!
//! - **The A2 clause is the safety argument.** `--seat` narrows a slot's
//!   candidate set to one card; it never adds an edge. So the operator can
//!   choose among the seatings the engine already considered legal, and
//!   cannot assert one it would have refused.
//! - **A consistent `--seat` on a non-ambiguous slot is simply satisfied**
//!   (r4 M1 dropped the "was it part of the refusal" conjunct: it protected
//!   nothing and broke scripting one `--seat` per slot across a mixed run).
//! - **Never a prefix.** The id is the card's FULL decoded set id, the exact
//!   label the A3 refusal printed — r3 measured prefix collisions at six
//!   characters on the 11-card fixture, so a shorter value is refused as a
//!   prefix rather than resolved.
//!
//! A5's "ambiguous id" case is UNREACHABLE, settled by SPEC A3(a) step 3 and
//! pinned by V-COLLIDE: two cards pinned to one chunk-set id merge into one
//! group in the input pipeline and refuse at reassembly, so no ambiguous id
//! survives to reach this parser.

use crate::error::CliError;
use crate::seat::input::{DecodedCard, GroupId};
use crate::seat::satisfy::{SlotDecl, satisfies};

/// One parsed `--seat '@i=<chunk-set-id>'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeatDirective {
    /// Placeholder index `@i`.
    pub i: u8,
    /// The full 20-bit chunk-set id named on the flag.
    pub set_id: u32,
}

/// Parse `@i=<chunk-set-id>`. The id is five hex digits (`0x` prefix
/// optional), matching what `GroupId`'s `Display` prints and what
/// `mk encode --chunk-set-id` accepts.
pub fn parse(arg: &str) -> Result<SeatDirective, CliError> {
    let (lhs, rhs) = arg.split_once('=').ok_or_else(|| {
        CliError::Seat(format!(
            "--seat expects '@i=<chunk-set-id>', got: {arg}. The chunk-set id is the \
             five-hex-digit label printed beside each card in a seating refusal."
        ))
    })?;
    let i: u8 = lhs
        .strip_prefix('@')
        .unwrap_or(lhs)
        .parse()
        .map_err(|_| CliError::Seat(format!("--seat slot index must be 0..255, got: {lhs}")))?;
    let digits = rhs
        .strip_prefix("0x")
        .or_else(|| rhs.strip_prefix("0X"))
        .unwrap_or(rhs);
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) || digits.is_empty() {
        return Err(CliError::Seat(format!(
            "--seat @{i}: `{rhs}` is not a chunk-set id. Expected five hex digits, the \
             label printed beside each card in a seating refusal."
        )));
    }
    if digits.len() != 5 {
        return Err(CliError::Seat(format!(
            "--seat @{i}: `{rhs}` is {} hex digit(s); a chunk-set id is exactly five. \
             Supply the card's FULL id, never a prefix — prefixes collide (measured at \
             six characters on an eleven-card set), so a prefix could seat the wrong key.",
            digits.len()
        )));
    }
    let set_id = u32::from_str_radix(digits, 16)
        .map_err(|e| CliError::Seat(format!("--seat @{i}: `{rhs}` is not hex: {e}")))?;
    Ok(SeatDirective { i, set_id })
}

/// Apply every directive to the A2 candidate sets.
///
/// Returns the narrowed candidates. Everything downstream — A3's
/// enumeration, A4's completeness, B1's dispositions — runs unchanged on
/// them, which is how `--seat` gets its "never suppresses, never fills"
/// properties structurally rather than by promise.
pub fn apply(
    directives: &[SeatDirective],
    decls: &[SlotDecl],
    cards: &[DecodedCard],
    per_slot: &[Vec<usize>],
) -> Result<Vec<Vec<usize>>, CliError> {
    let mut out = per_slot.to_vec();
    let mut pinned: Vec<(u8, u32)> = Vec::new();
    for d in directives {
        if let Some((_, previous)) = pinned.iter().find(|(i, _)| *i == d.i) {
            if *previous != d.set_id {
                return Err(CliError::Seat(format!(
                    "--seat @{}: named twice with different chunk-set ids ({previous:05x} \
                     and {:05x}). One card per slot.",
                    d.i, d.set_id
                )));
            }
            continue;
        }
        pinned.push((d.i, d.set_id));

        let Some(decl) = decls.iter().find(|s| s.i == d.i) else {
            return Err(CliError::Seat(format!(
                "--seat @{}: this policy has {} slot(s), numbered @0..@{}.",
                d.i,
                decls.len(),
                decls.len().saturating_sub(1)
            )));
        };
        let Some(card_idx) = cards
            .iter()
            .position(|c| c.set_id == GroupId::Chunked(d.set_id))
        else {
            let known: Vec<String> = cards.iter().map(|c| c.set_id.to_string()).collect();
            return Err(CliError::Seat(format!(
                "--seat @{}: no supplied card has chunk-set id {:05x}. The cards supplied \
                 are: {}.",
                d.i,
                d.set_id,
                known.join(", ")
            )));
        };
        let card = &cards[card_idx];
        // THE SAFETY ARGUMENT. `--seat` chooses among seatings A2 already
        // permits; it never adds one.
        if !satisfies(decl, card) {
            return Err(CliError::Seat(format!(
                "--seat @{}: card {} cannot sit in slot {}. The slot declares {}, and the \
                 card declares {}. --seat chooses among the seatings the declared origins \
                 already permit — it can never place a card the engine would not.",
                d.i,
                card.label(),
                decl.label(),
                origin_text(decl),
                card_origin_text(card)
            )));
        }
        out[d.i as usize] = vec![card_idx];
    }
    Ok(out)
}

fn origin_text(decl: &SlotDecl) -> String {
    match decl.fingerprint {
        Some(fp) => format!(
            "[{}/{}]",
            bitcoin::bip32::Fingerprint::from(fp),
            decl.path.to_string().trim_start_matches("m/")
        ),
        None => format!(
            "[{}] with no fingerprint",
            decl.path.to_string().trim_start_matches("m/")
        ),
    }
}

fn card_origin_text(card: &DecodedCard) -> String {
    let path = card.card.origin_path.to_string();
    let path = path.trim_start_matches("m/");
    match card.card.origin_fingerprint {
        Some(fp) => format!("[{fp}/{path}]"),
        None => format!("[{path}] with no fingerprint (privacy-preserving)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seat::matching::{Outcome, candidates, decide};
    use crate::seat::satisfy::fixture::*;
    use crate::seat::satisfy::slot_declarations;

    struct Case {
        policy: md_codec::encode::Descriptor,
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

    // ─── V-SEAT-OK ──────────────────────────────────────────────────────

    #[test]
    fn v_seat_ok_resolves_the_use_site_path_ambiguity() {
        let c = case(V_USP);
        // Without --seat this is the V-USP refusal.
        assert!(decide(&c.policy, &c.decls, &c.cards, &c.per_slot).is_err());

        let chosen = c.cards[0].set_id;
        let directive = parse(&format!("@0={chosen}")).unwrap();
        let narrowed = apply(&[directive], &c.decls, &c.cards, &c.per_slot).unwrap();
        assert_eq!(narrowed[0], vec![0], "@0 pinned to the named card");
        assert_eq!(narrowed[1], c.per_slot[1], "@1 untouched");

        let out = decide(&c.policy, &c.decls, &c.cards, &narrowed).unwrap();
        assert_eq!(out, Outcome::Seated(vec![0, 1]));

        // ...and the OTHER pin gives the OTHER wallet, which is exactly the
        // choice the refusal said only the operator could make.
        let other = parse(&format!("@0={}", c.cards[1].set_id)).unwrap();
        let narrowed2 = apply(&[other], &c.decls, &c.cards, &c.per_slot).unwrap();
        assert_eq!(
            decide(&c.policy, &c.decls, &c.cards, &narrowed2).unwrap(),
            Outcome::Seated(vec![1, 0])
        );
    }

    #[test]
    fn v_seat_ok_on_a_non_ambiguous_slot_is_simply_satisfied() {
        // r4 M1: no "was it part of the refusal" conjunct. Scripting one
        // --seat per slot across a mixed run has to work.
        let c = case(PATHOLOGICAL);
        let directives: Vec<SeatDirective> = c
            .decls
            .iter()
            .map(|d| {
                let card = c.per_slot[d.i as usize][0];
                parse(&format!("@{}={}", d.i, c.cards[card].set_id)).unwrap()
            })
            .collect();
        let narrowed = apply(&directives, &c.decls, &c.cards, &c.per_slot).unwrap();
        assert_eq!(narrowed, c.per_slot, "already unique — nothing to narrow");
        assert!(matches!(
            decide(&c.policy, &c.decls, &c.cards, &narrowed).unwrap(),
            Outcome::Seated(_)
        ));
    }

    #[test]
    fn v_seat_ok_never_fills_an_a4_gap() {
        // SPEC A5: `--seat` never fills A4 gaps. Pinning @1 on a set that is
        // one card short leaves @0 unfilled, and A3 still reports no
        // perfect matching.
        let c = case(V_FPFREE_CARD);
        let fp_bearing = c
            .cards
            .iter()
            .position(|x| x.card.origin_fingerprint.is_some())
            .unwrap();
        let d = parse(&format!("@1={}", c.cards[fp_bearing].set_id)).unwrap();
        let narrowed = apply(&[d], &c.decls, &c.cards, &c.per_slot).unwrap();
        assert_eq!(
            decide(&c.policy, &c.decls, &c.cards, &narrowed).unwrap(),
            Outcome::NoPerfectMatching
        );
    }

    // ─── V-SEAT-BAD ─────────────────────────────────────────────────────

    #[test]
    fn v_seat_bad_a_directive_contradicting_a2_refuses_by_name() {
        let c = case(V_MIX);
        // @0 declares fingerprint 73c5da0a; the b8688df1 card cannot meet it.
        let wrong = c
            .cards
            .iter()
            .find(|x| {
                x.card.origin_fingerprint
                    != Some(bitcoin::bip32::Fingerprint::from([0x73, 0xc5, 0xda, 0x0a]))
            })
            .unwrap();
        let d = parse(&format!("@0={}", wrong.set_id)).unwrap();
        let err = apply(&[d], &c.decls, &c.cards, &c.per_slot).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains(&wrong.set_id.to_string()),
            "names the card: {msg}"
        );
        assert!(
            msg.contains("@0 [73c5da0a/48'/0'/0'/2']"),
            "names the slot: {msg}"
        );
        assert!(
            msg.contains("b8688df1"),
            "names the card's own origin: {msg}"
        );
        assert!(
            msg.contains("can never place a card the engine would not"),
            "states the safety argument: {msg}"
        );
    }

    // ─── V-SEAT-UNK ─────────────────────────────────────────────────────

    #[test]
    fn v_seat_unk_an_unknown_id_refuses_naming_what_was_supplied() {
        let c = case(V_USP);
        let d = parse("@0=abcde").unwrap();
        let err = apply(&[d], &c.decls, &c.cards, &c.per_slot).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no supplied card has chunk-set id abcde"),
            "{msg}"
        );
        for card in &c.cards {
            assert!(
                msg.contains(&card.set_id.to_string()),
                "lists what IS there: {msg}"
            );
        }
    }

    #[test]
    fn v_seat_unk_a_prefix_is_refused_as_a_prefix() {
        let err = parse("@0=abc").unwrap_err().to_string();
        assert!(err.contains("3 hex digit(s)"), "{err}");
        assert!(err.contains("never a prefix"), "{err}");
        assert!(err.contains("collide"), "{err}");
    }

    #[test]
    fn v_seat_unk_malformed_values_refuse_by_shape() {
        assert!(
            parse("0abcd")
                .unwrap_err()
                .to_string()
                .contains("@i=<chunk-set-id>")
        );
        assert!(
            parse("@0=zzzzz")
                .unwrap_err()
                .to_string()
                .contains("not a chunk-set id")
        );
        assert!(
            parse("@notanum=0abcd")
                .unwrap_err()
                .to_string()
                .contains("0..255")
        );
        // The `0x` form `mk encode --chunk-set-id` accepts works too.
        assert_eq!(
            parse("@2=0x1c77f").unwrap(),
            SeatDirective {
                i: 2,
                set_id: 0x1_c77f
            }
        );
        assert_eq!(
            parse("@2=1C77F").unwrap(),
            SeatDirective {
                i: 2,
                set_id: 0x1_c77f
            }
        );
    }

    #[test]
    fn v_seat_bad_an_out_of_range_slot_refuses() {
        let c = case(V_USP);
        let d = parse(&format!("@9={}", c.cards[0].set_id)).unwrap();
        let msg = apply(&[d], &c.decls, &c.cards, &c.per_slot)
            .unwrap_err()
            .to_string();
        assert!(
            msg.contains("this policy has 2 slot(s), numbered @0..@1"),
            "{msg}"
        );
    }

    #[test]
    fn v_seat_bad_two_directives_for_one_slot_refuse() {
        let c = case(V_USP);
        let a = parse(&format!("@0={}", c.cards[0].set_id)).unwrap();
        let b = parse(&format!("@0={}", c.cards[1].set_id)).unwrap();
        let msg = apply(&[a, b], &c.decls, &c.cards, &c.per_slot)
            .unwrap_err()
            .to_string();
        assert!(
            msg.contains("named twice with different chunk-set ids"),
            "{msg}"
        );
        // The same directive twice is not a contradiction.
        assert!(apply(&[a, a], &c.decls, &c.cards, &c.per_slot).is_ok());
    }
}
