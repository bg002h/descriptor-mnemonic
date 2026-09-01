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
//! **A5's "ambiguous id" case IS reachable since SPEC §2 (seat auto-partition,
//! P1)** — this is a correction of the pre-P1 claim below, kept for
//! history: two cards pinned to one chunk-set id used to merge into one
//! group and refuse at reassembly (V-COLLIDE), so no ambiguous id ever
//! survived to reach this parser. Auto-partition changes that: a clean
//! collision now SEATS as several `DecodedCard`s sharing one `set_id`,
//! distinguished only by [`DecodedCard::ordinal`] — so a bare
//! `@i=<chunk-set-id>` naming a COLLIDED id is genuinely ambiguous among
//! its `#<k>` carriers, and SPEC §4 defines the grammar that resolves it:
//! `@i=<id>#<k>` binds the k-th (1-based) collided carrier; every other
//! shape refuses by name (see [`parse`]/[`apply`]).

use crate::error::CliError;
use crate::seat::input::{DecodedCard, GroupId};
use crate::seat::satisfy::{SlotDecl, satisfies};

/// One parsed `--seat '@i=<chunk-set-id>'` / `--seat '@i=<chunk-set-id>#<k>'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeatDirective {
    /// Placeholder index `@i`.
    pub i: u8,
    /// The full 20-bit chunk-set id named on the flag.
    pub set_id: u32,
    /// SPEC §4 — `Some(k)` when the flag names a collided carrier by its
    /// 1-based ordinal (`@i=<id>#<k>`); `None` for the bare `@i=<id>` form.
    pub ordinal: Option<u32>,
}

/// Parse `@i=<chunk-set-id>` or `@i=<chunk-set-id>#<k>` (SPEC §4). The id is
/// five hex digits (`0x` prefix optional), matching what `GroupId`'s
/// `Display` prints and what `mk encode --chunk-set-id` accepts; `#<k>`,
/// when present, is the 1-based ordinal a collided card's own label carries
/// (`DecodedCard::label`).
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

    // SPEC §4: an optional `#<k>` ordinal suffix on the id. Split on the
    // id/ordinal boundary FIRST so the id half's own hex-digit validation
    // below stays exactly as strict as it always was.
    let (id_part, ordinal_part) = match rhs.split_once('#') {
        Some((id, k)) => (id, Some(k)),
        None => (rhs, None),
    };

    let digits = id_part
        .strip_prefix("0x")
        .or_else(|| id_part.strip_prefix("0X"))
        .unwrap_or(id_part);
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) || digits.is_empty() {
        return Err(CliError::Seat(format!(
            "--seat @{i}: `{id_part}` is not a chunk-set id. Expected five hex digits, the \
             label printed beside each card in a seating refusal."
        )));
    }
    if digits.len() != 5 {
        return Err(CliError::Seat(format!(
            "--seat @{i}: `{id_part}` is {} hex digit(s); a chunk-set id is exactly five. \
             Supply the card's FULL id, never a prefix — prefixes collide (measured at \
             six characters on an eleven-card set), so a prefix could seat the wrong key.",
            digits.len()
        )));
    }
    let set_id = u32::from_str_radix(digits, 16)
        .map_err(|e| CliError::Seat(format!("--seat @{i}: `{id_part}` is not hex: {e}")))?;

    let ordinal = match ordinal_part {
        None => None,
        Some(k_str) => {
            if k_str.is_empty() || !k_str.chars().all(|c| c.is_ascii_digit()) {
                return Err(CliError::Seat(format!(
                    "--seat @{i}: `#{k_str}` after the chunk-set id must be `#<k>` with k a \
                     positive integer — the collided card's 1-based ordinal, exactly as its \
                     own label prints it (e.g. `{set_id:05x}#1`)."
                )));
            }
            let k: u32 = k_str.parse().map_err(|_| {
                CliError::Seat(format!(
                    "--seat @{i}: `#{k_str}` is not a valid ordinal (must fit a 32-bit count)."
                ))
            })?;
            if k == 0 {
                return Err(CliError::Seat(format!(
                    "--seat @{i}: `#0` is not a valid ordinal — collided cards are numbered \
                     from #1, matching their own label."
                )));
            }
            Some(k)
        }
    };

    Ok(SeatDirective { i, set_id, ordinal })
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
    let mut pinned: Vec<(u8, u32, Option<u32>)> = Vec::new();
    for d in directives {
        if let Some((_, prev_id, prev_ord)) = pinned.iter().find(|(i, _, _)| *i == d.i) {
            if *prev_id != d.set_id || *prev_ord != d.ordinal {
                let prev_suffix = prev_ord.map(|k| format!("#{k}")).unwrap_or_default();
                let this_suffix = d.ordinal.map(|k| format!("#{k}")).unwrap_or_default();
                return Err(CliError::Seat(format!(
                    "--seat @{}: named twice with different chunk-set values ({prev_id:05x}\
                     {prev_suffix} and {:05x}{this_suffix}). One card per slot.",
                    d.i, d.set_id,
                )));
            }
            continue;
        }
        pinned.push((d.i, d.set_id, d.ordinal));

        let Some(decl) = decls.iter().find(|s| s.i == d.i) else {
            return Err(CliError::Seat(format!(
                "--seat @{}: this policy has {} slot(s), numbered @0..@{}.",
                d.i,
                decls.len(),
                decls.len().saturating_sub(1)
            )));
        };

        // SPEC §4's `--seat` grammar: resolve which card(s) share this
        // bare id, THEN decide by the `#<k>` half — never a prefix, and
        // never a guess among collided carriers.
        let matches: Vec<usize> = cards
            .iter()
            .enumerate()
            .filter(|(_, c)| c.set_id == GroupId::Chunked(d.set_id))
            .map(|(i, _)| i)
            .collect();
        if matches.is_empty() {
            let known: Vec<String> = cards.iter().map(|c| c.set_id.to_string()).collect();
            return Err(CliError::Seat(format!(
                "--seat @{}: no supplied card has chunk-set id {:05x}. The cards supplied \
                 are: {}.",
                d.i,
                d.set_id,
                known.join(", ")
            )));
        }
        let card_idx = match d.ordinal {
            Some(k) => {
                if matches.len() == 1 {
                    return Err(CliError::Seat(format!(
                        "--seat @{}: chunk-set {:05x}#{k} — this id is not part of a \
                         collision (only one card supplied it). Use `{:05x}` alone.",
                        d.i, d.set_id, d.set_id
                    )));
                }
                match matches.iter().find(|&&idx| cards[idx].ordinal == Some(k)) {
                    Some(&idx) => idx,
                    None => {
                        let max = matches.len();
                        return Err(CliError::Seat(format!(
                            "--seat @{}: chunk-set {:05x}#{k} — this id's collided carriers \
                             are numbered #1..#{max}; #{k} is out of range.",
                            d.i, d.set_id
                        )));
                    }
                }
            }
            None => {
                if matches.len() > 1 {
                    let labels: Vec<String> =
                        matches.iter().map(|&idx| cards[idx].label()).collect();
                    return Err(CliError::Seat(format!(
                        "--seat @{}: chunk-set {:05x} names {} collided cards, not one — bind \
                         a specific carrier with `{:05x}#<k>`. Cards supplied: {}.",
                        d.i,
                        d.set_id,
                        matches.len(),
                        d.set_id,
                        labels.join(", ")
                    )));
                }
                matches[0]
            }
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
                set_id: 0x1_c77f,
                ordinal: None,
            }
        );
        assert_eq!(
            parse("@2=1C77F").unwrap(),
            SeatDirective {
                i: 2,
                set_id: 0x1_c77f,
                ordinal: None,
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
            msg.contains("named twice with different chunk-set values"),
            "{msg}"
        );
        // The same directive twice is not a contradiction.
        assert!(apply(&[a, a], &c.decls, &c.cards, &c.per_slot).is_ok());
    }

    // ─── SPEC §4 -- row 8's `--seat` grammar over a collided id ─────────
    //
    // `v-ap-canonical.txt`: a clean 2-card collision at id `a1001`,
    // 2-slot sortedmulti policy at ONE shared path (both slots satisfy
    // BOTH cards under A2, so id-resolution alone decides which card
    // lands where -- exactly what these rows are about). Built directly
    // via `decode_cards`/`policy` rather than `satisfy::fixture::cards`
    // (below `seat::run`'s card-set checks entirely -- this module tests
    // ONLY directive parsing/application, never the full pipeline).
    struct CollidedCase {
        decls: Vec<SlotDecl>,
        cards: Vec<DecodedCard>,
        per_slot: Vec<Vec<usize>>,
    }
    fn collided_case() -> CollidedCase {
        let text = include_str!("../../tests/fixtures/seating/v-ap-canonical.txt");
        let md1: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("md1"))
            .collect();
        let policy = md_codec::decode_md1_string(md1[0]).unwrap();
        let mk1: Vec<String> = text
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .map(str::to_string)
            .collect();
        let (cards, notes) = crate::seat::input::decode_cards(&mk1).unwrap();
        assert_eq!(cards.len(), 2, "the pair auto-partitions to two cards");
        assert_eq!(notes.len(), 1);
        let decls = crate::seat::satisfy::slot_declarations(&policy).unwrap();
        let per_slot = crate::seat::matching::candidates(&decls, &cards);
        CollidedCase {
            decls,
            cards,
            per_slot,
        }
    }

    #[test]
    fn v_seat_ordinal_resolves_each_collided_carrier_distinctly() {
        let c = collided_case();
        let d1 = parse("@0=a1001#1").unwrap();
        let narrowed1 = apply(&[d1], &c.decls, &c.cards, &c.per_slot).unwrap();
        assert_eq!(narrowed1[0].len(), 1);
        let d2 = parse("@0=a1001#2").unwrap();
        let narrowed2 = apply(&[d2], &c.decls, &c.cards, &c.per_slot).unwrap();
        assert_eq!(narrowed2[0].len(), 1);
        assert_ne!(
            narrowed1[0][0], narrowed2[0][0],
            "#1 and #2 must resolve to DIFFERENT physical cards"
        );
        // @1 (untouched by either directive) still carries both candidates.
        assert_eq!(narrowed1[1].len(), 2);
    }

    #[test]
    fn v_seat_bare_id_on_a_collided_group_is_ambiguous_names_both_labels() {
        let c = collided_case();
        let d = parse("@0=a1001").unwrap();
        let msg = apply(&[d], &c.decls, &c.cards, &c.per_slot)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("names 2 collided cards"), "{msg}");
        assert!(msg.contains("a1001#<k>"), "{msg}");
        assert!(msg.contains("a1001#1"), "{msg}");
        assert!(msg.contains("a1001#2"), "{msg}");
    }

    #[test]
    fn v_seat_ordinal_on_a_non_collided_id_refuses_naming_the_bare_id() {
        // PATHOLOGICAL's cards: no id is collided, so #1 on any of them
        // must refuse as "not part of a collision", never resolve.
        let text = crate::seat::satisfy::fixture::PATHOLOGICAL;
        let cards = crate::seat::satisfy::fixture::cards(text);
        let policy = crate::seat::satisfy::fixture::policy(text);
        let decls = crate::seat::satisfy::slot_declarations(&policy).unwrap();
        let per_slot = crate::seat::matching::candidates(&decls, &cards);
        let id = cards[0].set_id.to_string();
        let d = parse(&format!("@0={id}#1")).unwrap();
        let msg = apply(&[d], &decls, &cards, &per_slot)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("not part of a collision"), "{msg}");
        assert!(msg.contains(&format!("Use `{id}`")), "{msg}");
    }

    #[test]
    fn v_seat_ordinal_out_of_range_refuses_naming_the_valid_range() {
        let c = collided_case();
        let d = parse("@0=a1001#3").unwrap();
        let msg = apply(&[d], &c.decls, &c.cards, &c.per_slot)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("numbered #1..#2"), "{msg}");
        assert!(msg.contains("#3 is out of range"), "{msg}");
    }

    #[test]
    fn v_seat_ordinal_zero_refuses_at_parse_time() {
        let err = parse("@0=a1001#0").unwrap_err().to_string();
        assert!(err.contains("not a valid ordinal"), "{err}");
        assert!(err.contains("numbered from #1"), "{err}");
    }

    #[test]
    fn v_seat_ordinal_hash_without_digits_refuses_at_parse_time() {
        let err = parse("@0=a1001#").unwrap_err().to_string();
        assert!(err.contains("must be `#<k>`"), "{err}");
        assert!(err.contains("positive integer"), "{err}");
    }

    #[test]
    fn v_seat_ordinal_non_numeric_suffix_refuses_at_parse_time() {
        let err = parse("@0=a1001#abc").unwrap_err().to_string();
        assert!(err.contains("must be `#<k>`"), "{err}");
    }
}
