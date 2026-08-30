//! P2's **normative input pipeline** (SPEC `A3(a)`, restated in P2):
//!
//! > (1) dedupe byte-identical input strings; (2) group the survivors by
//! > declared chunk-set id; (3) reassemble each group under `mk decode`
//! > semantics.
//!
//! The ORDER is the whole content. `mk decode` itself has no dedupe step —
//! measured 2026-08-30, `mk decode S1 S2 S1 S2` refuses with *"chunked-header
//! malformed: received 4 chunks, header declares total_chunks = 2"* — so a
//! double scan of one card is only harmless because step 1 runs first. And
//! step 3 running LAST is what keeps an id collision fatal: two different
//! cards pinned to one chunk-set id merge into one group at step 2 and then
//! refuse at reassembly ("received 5 chunks, header declares total_chunks =
//! 2"), so the seating engine never sees colliding cards. That is also why
//! A5's "ambiguous `--seat` id" case is unreachable (plan §4 roster note).
//!
//! Step 1 normalises display separators before comparing, mirroring
//! [`crate::cmd::strip_md1_inputs`] on the md1 side: a card transcribed off
//! an engraving card in grouped form is byte-identical to the same card
//! pasted unbroken, and treating them as two cards would defeat the dedupe
//! this pipeline exists for.

use crate::error::CliError;
use mk_codec::KeyCard;
use mk_codec::string_layer::{StringLayerHeader, decode_string};
use std::collections::BTreeMap;
use std::fmt;

/// The identity a group of mk1 strings is keyed by at step 2 — the label
/// A3's ambiguity refusal prints and the exact token `--seat` accepts.
///
/// [`GroupId::Chunked`] carries the 20-bit `chunk_set_id` from the chunked
/// string-layer header. [`GroupId::Single`] exists because the header enum
/// has a `SingleString` variant that carries no such field; no `mk encode`
/// invocation can produce one (a 73-byte compact xpub alone exceeds the
/// 56-byte single-string ceiling, so every real card chunks — mk-codec's own
/// `pipeline.rs` says so), which is why it is keyed by input position rather
/// than by a wire value that does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupId {
    /// A chunked card, keyed by its 20-bit `chunk_set_id`.
    Chunked(u32),
    /// A single-string card, keyed by the position of its string in the
    /// deduped input.
    Single(usize),
}

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Five lowercase hex digits — the full 20-bit field, never a
            // prefix (SPEC A5: "the card's full decoded set id … never a
            // string prefix", after r3 measured prefix collisions at 6
            // characters on the 11-card fixture).
            GroupId::Chunked(id) => write!(f, "{id:05x}"),
            GroupId::Single(n) => write!(f, "single#{n}"),
        }
    }
}

/// One reassembled mk1 card, carrying the set id it was grouped under so
/// every refusal can name it.
#[derive(Debug, Clone)]
pub struct DecodedCard {
    /// The chunk-set id this card's strings declared.
    pub set_id: GroupId,
    /// The decoded card itself.
    pub card: KeyCard,
}

impl DecodedCard {
    /// `set-id (stub …)` — the identifier pair every A4/B1 message names a
    /// card by. Stubs are joined with `,` when a card declares several.
    pub fn label(&self) -> String {
        let stubs: Vec<String> = self
            .card
            .policy_id_stubs
            .iter()
            .map(|s| format!("{:02x}{:02x}{:02x}{:02x}", s[0], s[1], s[2], s[3]))
            .collect();
        format!("{} (stub {})", self.set_id, stubs.join(","))
    }
}

/// Step 1 — dedupe byte-identical strings, after stripping display
/// separators, preserving first-appearance order.
///
/// Order preservation is not cosmetic: step 2's groups inherit it, and the
/// `GroupId::Single` fallback keys on it.
pub fn dedupe_strings(strings: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::with_capacity(strings.len());
    for s in strings {
        let normalised: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        if normalised.is_empty() {
            continue;
        }
        if !seen.iter().any(|k| *k == normalised) {
            seen.push(normalised);
        }
    }
    seen
}

/// Read the string-layer header of ONE mk1 string and return the group key
/// it declares.
///
/// The call chain is mk-codec's own, resolved against `vendor/mk-codec`:
/// [`decode_string`] runs the BCH layer and yields a [`DecodedString`]
/// whose `.data()` is the 5-bit symbol stream, and
/// [`StringLayerHeader::from_5bit_symbols`] reads the header off the front
/// of that stream. Nothing here re-implements either.
///
/// [`DecodedString`]: mk_codec::string_layer::DecodedString
pub fn group_key_of(s: &str, position: usize) -> Result<GroupId, CliError> {
    let decoded = decode_string(s).map_err(|e| {
        CliError::Seat(format!(
            "--from-mk1 string {} is not a decodable mk1 string: {e}",
            position + 1
        ))
    })?;
    let (header, _consumed) =
        StringLayerHeader::from_5bit_symbols(decoded.data()).map_err(|e| {
            CliError::Seat(format!(
                "--from-mk1 string {}: malformed mk1 string-layer header: {e}",
                position + 1
            ))
        })?;
    Ok(match header {
        StringLayerHeader::Chunked { chunk_set_id, .. } => GroupId::Chunked(chunk_set_id),
        _ => GroupId::Single(position),
    })
}

/// The group key of ONE mk1 string, read in isolation.
///
/// Exact for the chunked form (the id is on the wire). A single-string card
/// has no such field, so this reports `Single(0)` for one; that fallback is
/// only meaningful inside a whole [`decode_cards`] run, where the position
/// is real.
// Used by the vector rows that build a card subset by GROUP rather than by
// guessing which lines belong to a card (V-UNFILLED, V-CE1). The pipeline
// itself calls `group_key_of` with a real position.
#[allow(dead_code)]
pub fn group_id_of(s: &str) -> Result<GroupId, CliError> {
    group_key_of(s, 0)
}

/// The whole pipeline: `&[String]` of mk1 strings in, reassembled cards out,
/// in the normative order.
///
/// Groups are returned in ascending set-id order, NOT in supply order —
/// determinism of everything downstream (the assignment vector, every
/// refusal listing) must not depend on how the operator happened to type
/// the strings, and V-ORD pins that.
pub fn decode_cards(strings: &[String]) -> Result<Vec<DecodedCard>, CliError> {
    // Step 1.
    let deduped = dedupe_strings(strings);
    if deduped.is_empty() {
        return Err(CliError::Seat(
            "no mk1 key-card strings supplied (--from-mk1 / --from-mk1-file)".into(),
        ));
    }
    // Step 2.
    let mut groups: BTreeMap<GroupId, Vec<String>> = BTreeMap::new();
    for (position, s) in deduped.iter().enumerate() {
        let key = group_key_of(s, position)?;
        groups.entry(key).or_default().push(s.clone());
    }
    // Step 3.
    let mut cards = Vec::with_capacity(groups.len());
    for (set_id, group) in groups {
        let refs: Vec<&str> = group.iter().map(String::as_str).collect();
        let card = mk_codec::decode(&refs).map_err(|e| {
            CliError::Seat(format!(
                "chunk-set {set_id}: the {} string(s) declaring this id do not reassemble \
                 into one key card: {e}. Two DIFFERENT cards pinned to one chunk-set id \
                 merge into one group here and refuse exactly like this — re-mint one of \
                 them so the set ids differ",
                refs.len()
            ))
        })?;
        cards.push(DecodedCard { set_id, card });
    }
    Ok(cards)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── V-DUP (pipeline half) ──────────────────────────────────────────
    //
    // The end-to-end must-SEAT row lands with the CLI surface (step 7); this
    // is the pipeline property it rests on. Without step 1 running BEFORE
    // step 3, `mk decode` refuses the doubled set outright (measured
    // 2026-08-30: "received 4 chunks, header declares total_chunks = 2").

    fn pathological_mk1() -> Vec<String> {
        include_str!("../../tests/fixtures/pathological/backup-strings.txt")
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn v_dup_byte_identical_strings_collapse_before_grouping() {
        let one = pathological_mk1();
        assert_eq!(one.len(), 30, "fixture shape: 11 cards over 30 mk1 chunks");
        let mut twice = one.clone();
        twice.extend(one.clone());
        assert_eq!(twice.len(), 60);
        let cards = decode_cards(&twice).expect("a doubled full set must reassemble");
        assert_eq!(cards.len(), 11, "the doubled set is still 11 cards");
        let once = decode_cards(&one).expect("the single set must reassemble");
        assert_eq!(cards.len(), once.len());
        for (a, b) in cards.iter().zip(once.iter()) {
            assert_eq!(a.set_id, b.set_id);
            assert_eq!(a.card, b.card);
        }
    }

    #[test]
    fn v_dup_grouped_display_form_is_the_same_string_as_the_unbroken_form() {
        let one = pathological_mk1();
        let grouped: Vec<String> = one
            .iter()
            .map(|s| {
                s.chars()
                    .collect::<Vec<_>>()
                    .chunks(5)
                    .map(|c| c.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let mut mixed = one.clone();
        mixed.extend(grouped);
        let cards = decode_cards(&mixed).expect("grouped + unbroken is one set, not two");
        assert_eq!(cards.len(), 11);
    }

    #[test]
    fn v_dup_supply_order_does_not_change_the_decoded_card_list() {
        let one = pathological_mk1();
        let mut reversed = one.clone();
        reversed.reverse();
        let a = decode_cards(&one).unwrap();
        let b = decode_cards(&reversed).unwrap();
        let ids_a: Vec<GroupId> = a.iter().map(|c| c.set_id).collect();
        let ids_b: Vec<GroupId> = b.iter().map(|c| c.set_id).collect();
        assert_eq!(ids_a, ids_b, "groups are returned in set-id order");
    }

    // ─── V-COLLIDE ──────────────────────────────────────────────────────

    #[test]
    fn v_collide_two_cards_pinned_to_one_chunk_set_id_refuse_at_reassembly() {
        // NOTE: this row also subsumes A5's "ambiguous `--seat` id" case,
        // which SPEC A3(a) step 3 makes UNREACHABLE (plan §4 roster note):
        // colliding cards merge into one group and refuse HERE, so no
        // ambiguous id can ever reach `--seat` parsing.
        let strings: Vec<String> = include_str!("../../tests/fixtures/seating/v-collide.txt")
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .map(str::to_string)
            .collect();
        assert!(
            strings.len() > 2,
            "fixture must carry both pinned cards' chunks"
        );
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("chunk-set "),
            "the refusal names the colliding set id: {msg}"
        );
        assert!(
            msg.contains("do not reassemble"),
            "the refusal is the reassembly one, not a seating one: {msg}"
        );
    }

    #[test]
    fn v_collide_each_pinned_card_alone_still_decodes() {
        // Proves the refusal above comes from the MERGED group and not from
        // either card being malformed — without this, the row would pass in
        // a world where the fixture is simply broken.
        let text = include_str!("../../tests/fixtures/seating/v-collide.txt");
        let a: Vec<String> = text
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .take(2)
            .map(str::to_string)
            .collect();
        assert_eq!(a.len(), 2);
        assert!(decode_cards(&a).is_ok(), "card A alone decodes");
    }

    #[test]
    fn dedupe_strips_whitespace_and_drops_blanks() {
        let v = vec![
            "mk1abc".to_string(),
            "mk1 abc".to_string(),
            "   ".to_string(),
            "mk1def".to_string(),
        ];
        assert_eq!(dedupe_strings(&v), vec!["mk1abc", "mk1def"]);
    }

    #[test]
    fn group_id_renders_the_full_twenty_bit_field() {
        assert_eq!(GroupId::Chunked(0x1_C77F).to_string(), "1c77f");
        assert_eq!(GroupId::Chunked(0).to_string(), "00000");
        assert_eq!(GroupId::Chunked(0xF_FFFF).to_string(), "fffff");
    }
}
