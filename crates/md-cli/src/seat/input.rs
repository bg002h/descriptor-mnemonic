//! P2's **normative input pipeline** (SPEC `A3(a)`, restated in P2):
//!
//! > (1) dedupe input strings that name the same card, after normalising
//! > display separators AND case; (2) group the survivors by
//! > declared chunk-set id; (3) reassemble each group under `mk decode`
//! > semantics.
//!
//! The ORDER is the whole content. `mk decode` itself has no dedupe step —
//! measured 2026-08-30, `mk decode S1 S2 S1 S2` refuses with *"chunked-header
//! malformed: received 4 chunks, header declares total_chunks = 2"* — so a
//! double scan of one card is only harmless because step 1 runs first. And
//! step 3 running LAST is what keeps an id collision fatal: two different
//! cards pinned to one chunk-set id merge into one group at step 2, so the
//! seating engine never sees colliding cards as two candidates. **Since P3**
//! (SPEC contract 7 / R5), the merge is caught by [`decode_cards`]'s own
//! classifier — from the retained per-chunk headers, before `mk_codec::decode`
//! is even called for the shapes it can name (merged cards, incomplete scan);
//! only the residual "terminal" shapes still reach `mk_codec::decode` and
//! surface its error verbatim on a labeled line. That is also why A5's
//! "ambiguous `--seat` id" case is unreachable (plan §4 roster note).
//!
//! Step 1 normalises display separators AND case before comparing. The
//! separator half mirrors [`crate::cmd::strip_md1_inputs`] on the md1 side: a
//! card transcribed off an engraving card in grouped form is the same card as
//! one pasted unbroken. The case half is
//! REVIEW-converter-whole-diff-r1 I2 — mk1 is bech32, uppercase is the
//! canonical QR form, and md's decoder accepts it, so byte identity missed one
//! card scanned twice in two cases. Treating either as two cards defeats the
//! dedupe this pipeline exists for. See [`dedupe_strings`].

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

/// Step 1 — dedupe strings that name the SAME card, after stripping display
/// separators and normalising case, preserving first-appearance order.
///
/// Order preservation is not cosmetic: step 2's groups inherit it, and the
/// `GroupId::Single` fallback keys on it.
///
/// TWO NORMALISATIONS, EACH FOR A MEASURED REASON.
///
/// - WHITESPACE, mirroring [`crate::cmd::strip_md1_inputs`]: a card
///   transcribed off an engraving card in grouped form is the same card as
///   one pasted unbroken.
/// - CASE (REVIEW-converter-whole-diff-r1 I2). mk1 strings are bech32, so
///   UPPERCASE is the canonical QR form and `mk decode` accepts it: measured
///   2026-08-30, an all-uppercase card set seats to the identical descriptor
///   (`v_dup_an_all_uppercase_card_set_seats_identically`). A byte-identity
///   key therefore did NOT recognise one card scanned twice, once in each
///   case. The two survivors merged into one group at step 2 and refused at
///   step 3 with the pre-P3 wrapper's message: it reported the two survivors
///   as though they were two DIFFERENT key cards pinned to one chunk-set id,
///   with a fix that meant re-engraving the survivor whose stamped id it
///   named — a diagnosis of the wrong problem whose remedy is re-engraving
///   a plate that is fine. SPEC A3(a)'s
///   guarantee is that the double scan is harmless BY ORDER OF OPERATIONS,
///   and it can only be that if step 1 recognises the equivalence the
///   decoder already honours.
///
/// The comparison key is case-folded; the string KEPT is the one as supplied
/// (first appearance wins). Lower-casing the survivor would take a decision
/// that belongs to the decoder — bech32 forbids MIXED case in one string, and
/// that refusal is `decode_string`'s to make, not this step's.
pub fn dedupe_strings(strings: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::with_capacity(strings.len());
    let mut keys: Vec<String> = Vec::with_capacity(strings.len());
    for s in strings {
        let normalised: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        if normalised.is_empty() {
            continue;
        }
        let key = normalised.to_lowercase();
        if !keys.iter().any(|k| *k == key) {
            keys.push(key);
            seen.push(normalised);
        }
    }
    seen
}

/// One string's declared chunk header, retained past grouping to drive the
/// R5 classifier (SPEC contract 7, plan P3 IMPL). Before P3, `group_key_of`
/// discarded `chunk_index`/`total_chunks` once the group key was read; the
/// classifier below cannot answer "duplicate piece number" or "how many were
/// declared" without them.
#[derive(Debug, Clone, Copy)]
pub struct ChunkInfo {
    /// Zero-based index of this chunk within its declared set.
    pub chunk_index: u8,
    /// The set's declared chunk count, as THIS string states it (siblings
    /// may disagree — that disagreement is itself arm 1 evidence).
    pub total_chunks: u8,
}

/// Read the string-layer header of ONE mk1 string and return the group key
/// it declares, plus its chunk header info when it has one.
///
/// The call chain is mk-codec's own, resolved against `vendor/mk-codec`:
/// [`decode_string`] runs the BCH layer and yields a [`DecodedString`]
/// whose `.data()` is the 5-bit symbol stream, and
/// [`StringLayerHeader::from_5bit_symbols`] reads the header off the front
/// of that stream. Nothing here re-implements either.
///
/// [`DecodedString`]: mk_codec::string_layer::DecodedString
pub fn group_key_of(s: &str, position: usize) -> Result<(GroupId, Option<ChunkInfo>), CliError> {
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
        StringLayerHeader::Chunked {
            chunk_set_id,
            total_chunks,
            chunk_index,
            ..
        } => (
            GroupId::Chunked(chunk_set_id),
            Some(ChunkInfo {
                chunk_index,
                total_chunks,
            }),
        ),
        _ => (GroupId::Single(position), None),
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
    group_key_of(s, 0).map(|(g, _)| g)
}

/// R5's classification of a FAILED chunked group (SPEC contract 7 / plan P3).
///
/// Arms are evaluated in this fixed order; arm 3 has **no precondition**, so
/// together they are TOTAL over any chunked group that isn't a clean
/// reassembly (situation 4, handled outside this enum entirely).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    /// Situation 1 — duplicate chunk index, sibling strings disagreeing on
    /// `total_chunks`, or a chunk whose index/count exceeds its own
    /// declared total: two DIFFERENT key cards pinned to one chunk-set id.
    Merged,
    /// Situation 2 — fewer strings than the (agreed) declared total, no
    /// duplicates: one card, short some pieces.
    Incomplete { declared_total: u8 },
    // Situation 3 ("terminal", SPEC r2 C3) has NO precondition of its own —
    // it is simply "neither of the above", so it carries no variant here.
    // `decode_cards` reaches it by falling through to `mk_codec::decode`
    // whenever `classify` returns `None`, and wraps whatever that call
    // reports via `terminal_refusal`.
}

/// Arm 1/2 predicates read directly off the retained per-chunk headers —
/// SPEC contract 7 deliberately answers these WITHOUT ever calling
/// `mk_codec::decode`, so a merged or incomplete group gets the classifier's
/// message rather than whatever the codec's own reassembly error happens to
/// say about the mixed bytes.
fn classify(infos: &[ChunkInfo]) -> Option<Failure> {
    let declared_total = infos[0].total_chunks;
    let disagreeing_total = infos.iter().any(|i| i.total_chunks != declared_total);
    let mut seen = [false; 256];
    let mut duplicate = false;
    let mut out_of_range = false;
    for i in infos {
        if (i.chunk_index as usize) >= i.total_chunks as usize {
            out_of_range = true;
        }
        let idx = i.chunk_index as usize;
        if seen[idx] {
            duplicate = true;
        }
        seen[idx] = true;
    }
    let excess = infos.len() > declared_total as usize;
    if duplicate || disagreeing_total || out_of_range || excess {
        return Some(Failure::Merged);
    }
    if infos.len() < declared_total as usize {
        return Some(Failure::Incomplete { declared_total });
    }
    None
}

/// The per-string evidence arm 1's message must carry (W15(a)): literally,
/// how many supplied strings declare each (piece number, declared total)
/// pair, e.g. "2 strings declare piece 1 of 2 and 2 strings declare piece 2
/// of 2" — a duplicated piece number is what proves two cards, so the count
/// is the whole argument.
fn piece_evidence(infos: &[ChunkInfo]) -> String {
    let mut counts: BTreeMap<(u8, u8), usize> = BTreeMap::new();
    for i in infos {
        *counts.entry((i.chunk_index, i.total_chunks)).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|((idx, total), n)| {
            let piece = idx as u32 + 1;
            let (noun, verb) = if n == 1 {
                ("string", "declares")
            } else {
                ("strings", "declare")
            };
            format!("{n} {noun} {verb} piece {piece} of {total}")
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Situation 1 (W15/W16). Glosses "key card"/"chunk"/"pinned"/"re-mint" at
/// first use; counts CARDS, never plates (W16(a)); states piece order does
/// not matter; the remedy is physical and names no plate count; the id-check
/// is a named command, gated on "only if" (W15(d) — an unnamed check is
/// decoration).
fn merged_refusal(set_id: GroupId, infos: &[ChunkInfo]) -> CliError {
    let evidence = piece_evidence(infos);
    CliError::Seat(format!(
        "chunk-set {set_id}: {evidence}. A duplicated piece number is proof this chunk-set id \
         is pinned (stamped as a fixed value rather than derived from content) to two \
         DIFFERENT key cards, not one — each key card's mk1 strings are its chunks (pieces), \
         and piece order does not matter. Re-scan one card's pieces alone, not mixed with any \
         other card's pieces. Only if two cards truly show the same stamped id is a re-mint \
         (re-encoding without --chunk-set-id) needed — check each card alone first with \
         `mk inspect`."
    ))
}

/// Situation 2 (W5 floor). Does not assert a single card (r1 M3) — only
/// that this id's pieces say there should be N and K arrived.
fn incomplete_refusal(set_id: GroupId, received: usize, declared_total: u8) -> CliError {
    CliError::Seat(format!(
        "chunk-set {set_id}: the pieces (chunks) carrying this id say there should be \
         {declared_total}; you supplied {received} — scan the missing piece(s)."
    ))
}

/// Situation 3 (W16(b)) — human sentence first, codec diagnostic on its own
/// labeled line. No precondition: reached for every remaining failure,
/// including `mk_codec`'s own reassembly refusal (measured exemplar:
/// `Error::CrossChunkHashMismatch`, "cross-chunk integrity hash mismatch").
fn terminal_refusal(set_id: GroupId, e: mk_codec::Error) -> CliError {
    CliError::Seat(format!(
        "chunk-set {set_id}: these pieces (chunks) carry one id but do not form one key card; \
         re-scan one card's pieces alone.\nerror: {e}"
    ))
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
    // Step 2. Per-string chunk headers are retained (R5 — pre-P3 this
    // discarded chunk_index/total_chunks the moment the group key was read).
    let mut groups: BTreeMap<GroupId, Vec<(String, Option<ChunkInfo>)>> = BTreeMap::new();
    for (position, s) in deduped.iter().enumerate() {
        let (key, info) = group_key_of(s, position)?;
        groups.entry(key).or_default().push((s.clone(), info));
    }
    // Step 3.
    let mut cards = Vec::with_capacity(groups.len());
    for (set_id, group) in groups {
        let infos: Vec<ChunkInfo> = group.iter().filter_map(|(_, i)| *i).collect();
        // R5 arms 1/2: classify from the retained headers BEFORE ever
        // calling `mk_codec::decode`, so a merged/incomplete group gets the
        // classifier's own message rather than the codec's reassembly
        // error. Only non-empty (i.e. genuinely chunked) groups classify —
        // a `Single` group carries no chunk header to classify from.
        if !infos.is_empty() {
            match classify(&infos) {
                Some(Failure::Merged) => return Err(merged_refusal(set_id, &infos)),
                Some(Failure::Incomplete { declared_total }) => {
                    return Err(incomplete_refusal(set_id, group.len(), declared_total));
                }
                None => {}
            }
        }
        let refs: Vec<&str> = group.iter().map(|(s, _)| s.as_str()).collect();
        let card = mk_codec::decode(&refs).map_err(|e| terminal_refusal(set_id, e))?;
        cards.push(DecodedCard { set_id, card });
    }
    Ok(cards)
}

/// SPEC "The comparison" operand: `derive_chunk_set_id(encode_bytecode(card))`
/// — the canonical re-encode of the successfully decoded card. Fully
/// qualified as `mk_codec::` throughout (plan P3 IMPL / r1 N2): this crate
/// also imports a DIFFERENT, same-named `md_codec::chunk::derive_chunk_set_id`
/// in `cmd/encode.rs`, and an unqualified call here would silently compute
/// the md-side id instead of the mk-side one this contract is about.
fn derived_chunk_set_id(card: &KeyCard) -> Option<u32> {
    let bytecode = mk_codec::bytecode::encode_bytecode(card).ok()?;
    Some(mk_codec::derive_chunk_set_id(&bytecode))
}

/// R2/R6 frozen warning wording (SPEC "Behavior contracts" §2, byte-identical
/// to mk-cli's `chunk_set_id_mismatch_warning`). R6 ("one warning everywhere")
/// requires the SAME content on every reassembly surface; each surface
/// computes its own operand and prints its own copy of this string, since
/// md-cli and mk-cli are independent binaries sharing no runtime code.
pub fn chunk_set_id_mismatch_warning(declared: u32, derived: u32) -> String {
    format!(
        "warning: this key card's stamped chunk-set id ({declared:05x}) was not derived from \
         its content, which computes {derived:05x}. The card decodes fine, but diagnostics that \
         name plates by id will call it {declared:05x}. To fix it, re-mint: run mk encode again \
         without --chunk-set-id and the id is derived from the key data automatically."
    )
}

/// Contract 6 — after a chunked group reassembles CLEANLY, recompute the
/// operand and warn on stamped != derived. One note per mismatching group,
/// in EXISTING group order (`cards` is already ascending set-id order, per
/// `decode_cards`'s own contract) — `Single` groups have no declared id to
/// compare and are silently skipped.
pub fn seat_chunk_set_id_warnings(cards: &[DecodedCard]) -> Vec<String> {
    let mut notes = Vec::new();
    for c in cards {
        let GroupId::Chunked(declared) = c.set_id else {
            continue;
        };
        let Some(derived) = derived_chunk_set_id(&c.card) else {
            continue;
        };
        if declared != derived {
            notes.push(chunk_set_id_mismatch_warning(declared, derived));
        }
    }
    notes
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
        // R5 rewrite (SPEC contract 7): card A is 2 chunks, card B is 3 --
        // DISAGREEING total_chunks is arm 1's second predicate, so this row
        // is now "merged cards", not the old one-message-fits-all wrapper.
        assert!(
            msg.contains("piece 1 of 2") && msg.contains("piece 1 of 3"),
            "the per-string evidence names both cards' declared shapes: {msg}"
        );
        assert!(
            msg.contains("piece order does not matter"),
            "arm 1 states order doesn't matter: {msg}"
        );
        assert!(
            msg.contains("mk inspect"),
            "the id-check is named as a runnable command: {msg}"
        );
        assert_no_retired_wording(&msg);
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

    // ─── R5 classifier (SPEC contract 7, plan P3) ──────────────────────
    //
    // Fixtures below were minted live with `mk encode --chunk-set-id`
    // (mnemonic-key `mk` 0.13 on PATH) and their chunk headers verified
    // with a throwaway `mk_codec = "0.5"` reader before being pinned here
    // as literals -- see `impl-csid-p3.md` for the exact mint commands.

    /// The retired pre-P3 wording must appear in NO message (SPEC contract
    /// 7: "the retired message ... appears nowhere").
    fn assert_no_retired_wording(msg: &str) {
        assert!(
            !msg.contains("re-mint one of them"),
            "retired wording resurfaced: {msg}"
        );
        assert!(
            !msg.contains("Two DIFFERENT cards pinned"),
            "retired wording resurfaced: {msg}"
        );
    }

    /// Situation 1 -- MERGED CARDS (W15 exemplar). Two DIFFERENT 2-chunk
    /// cards, both pinned to chunk-set `11111`: 4 strings, two declaring
    /// piece 1 of 2 and two declaring piece 2 of 2.
    #[test]
    fn r5_merged_two_cards_pinned_to_one_id_classify_as_merged() {
        let strings: Vec<String> = [
            "mk1qpzyg3pqqsq4kj90x9eutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5x94fvrdwgu6g0lq",
            "mk1qpzyg3pp806lhaeh6reknylagmwyjycf8044xtt9flsdlkvt6f6cthyl99lqmcdjwej7x7ylmk0lq",
            "mk1qpzyg3pqqsq4kj90xxux3r03q5zg3vs7llvu2xd8x2rk7av9gmew82jq5zap9302ynhp37ggd6z5u4emag0zr8gh9upnj76samqgd9kc0zqu",
            "mk1qpzyg3ppwyp4dfykwfkgg6fxyxetdcmythf4hsqzd3v879jprztejzs7ruvy26n2gd25y20mx4xyg",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("chunk-set 11111"), "{msg}");
        assert!(
            msg.contains("2 strings declare piece 1 of 2")
                && msg.contains("2 strings declare piece 2 of 2"),
            "the per-string evidence the tool already holds (W15(a)): {msg}"
        );
        assert!(msg.contains("piece order does not matter"), "{msg}");
        assert!(
            msg.contains("Re-scan one card's pieces alone"),
            "the physical remedy (W15(c)): {msg}"
        );
        assert!(
            !msg.contains("plate"),
            "counts CARDS, never plates (W16(a)): {msg}"
        );
        assert!(
            msg.contains("`mk inspect`"),
            "the id-check is a named command, not decoration (W15(d)): {msg}"
        );
        assert!(
            msg.contains("Only if"),
            "the id-check is gated on \"only if\", not asserted: {msg}"
        );
        assert_no_retired_wording(&msg);
        // Not the other two situations' messages.
        assert!(!msg.contains("scan the missing piece"), "{msg}");
        assert!(!msg.contains("error:"), "no codec line on arm 1: {msg}");
    }

    /// Situation 2 -- INCOMPLETE SCAN. One 2-chunk card, only its first
    /// chunk supplied: received 1 < declared 2, no duplicates.
    #[test]
    fn r5_incomplete_one_of_two_chunks_classifies_as_incomplete() {
        let strings = vec![
            "mk1qpxvenpqqsq4kj90xdeutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5drdl5w8ame3clux"
                .to_string(),
        ];
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("chunk-set 33333"), "{msg}");
        assert!(msg.contains("should be 2"), "{msg}");
        assert!(msg.contains("you supplied 1"), "{msg}");
        assert!(msg.contains("scan the missing piece"), "{msg}");
        assert!(
            !msg.contains("single card") && !msg.contains("one card"),
            "does not assert a single card (r1 M3): {msg}"
        );
        assert!(!msg.contains("piece order does not matter"), "{msg}");
        assert!(!msg.contains("error:"), "no codec line on arm 2: {msg}");
        assert_no_retired_wording(&msg);
    }

    /// Situation 3 -- TERMINAL (SPEC's measured exemplar). Chunk 0 of card
    /// T1 + chunk 1 of card T2, both pinned to chunk-set `22222`, both
    /// declaring `total_chunks = 2` (arms 1/2's predicates are all false):
    /// the codec's own cross-chunk integrity hash refuses it.
    #[test]
    fn r5_terminal_cross_chunk_hash_mismatch_classifies_as_terminal() {
        let strings = vec![
            "mk1qpyg3zpqqsq4kj90xfeutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5fkjjqxnyhx4glde"
                .to_string(),
            "mk1qpyg3zppwyp4dfykwfkgg6fxyxetdcmythf4hsqzd3v879jprztejzs7rlhgvt7a4x7n4h7uagdls".to_string(),
        ];
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("chunk-set 22222"), "{msg}");
        assert!(
            msg.contains("do not form one key card"),
            "the neutral remedy leads (W16(b)): {msg}"
        );
        assert!(msg.contains("re-scan one card's pieces alone"), "{msg}");
        let error_line = msg.lines().find(|l| l.starts_with("error: "));
        assert!(
            error_line.is_some_and(|l| l.contains("cross-chunk integrity hash mismatch")),
            "the codec diagnostic is on its own labeled line (W16(b)): {msg}"
        );
        assert!(
            msg.find("do not form one key card").unwrap() < msg.find("error:").unwrap(),
            "human sentence leads, codec line follows (W16(b)): {msg}"
        );
        assert_no_retired_wording(&msg);
    }

    /// Classification-ORDER (plan r1 I2 / spec Acceptance): a supply
    /// matching BOTH arm 1's (duplicate piece index) and arm 2's (received
    /// < declared) raw predicates must land in the EARLIER arm (merged).
    /// Two DIFFERENT 3-chunk cards, both pinned to chunk-set `44444`, only
    /// their chunk 0 supplied: received=2 < declared=3 (arm 2's predicate),
    /// AND both declare piece 1 of 3 (arm 1's predicate).
    #[test]
    fn r5_classification_order_prefers_merged_over_incomplete() {
        let strings = vec![
            "mk1qpg3zyzqqsq4kj90x3eutks2lcztpqyqsqygpqyqsqygrqyqsqyg9qyqsqyqfz9jrcld706hn9svfgll7zvw5qnkxgea7nkj6jsf2avy9zwj"
                .to_string(),
            "mk1qpg3zyzqqsq4kj90x3eutks2lcztpqyqsqygpqyqsqyg9qyqsqyg9qyqsqyqfz9jrej0n5eghh0620cpg9jly68gp3qxjnv0ty9cpzm2edu5"
                .to_string(),
        ];
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("2 strings declare piece 1 of 3"),
            "landed in the MERGED arm (arm 1), not incomplete: {msg}"
        );
        assert!(
            !msg.contains("scan the missing piece"),
            "must NOT be classified incomplete despite received(2) < declared(3): {msg}"
        );
        assert_no_retired_wording(&msg);
    }

    // ─── Contract 6 -- R2/R6 seat warning ──────────────────────────────

    #[test]
    fn csid_warning_fires_on_a_pinned_mismatch_and_is_silent_on_the_clean_twin() {
        // KEY1's V-USP card, re-minted with --chunk-set-id 0x99999 pinned
        // (declared 99999 != content-derived 69f0e); KEY5's V-USP card,
        // UNCHANGED (natural mint, declared == derived == decb1).
        let pinned = vec![
            "mk1qpnxvepqqsq4kj90x4eutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5fxwekt2hrvmq69e"
                .to_string(),
            "mk1qpnxvepp806lhaeh6reknylagmwyjycf8044xtt9flsdlkvt6f6cthyl995lpm5zlc7d38ev0su0k".to_string(),
        ];
        let clean = vec![
            "mk1qpmm93pqqsq4kj90xkux3r03q5zg3vs7llvu2xd8x2rk7av9gmew82jq5zap9302ynhp37ggd6z5u4emag0zr8gh9upnj5stuxqtzdn4uxkd"
                .to_string(),
            "mk1qpmm93ppwyp4dfykwfkgg6fxyxetdcmythf4hsqzd3v879jprztejzs7rl0vk8mlv7lzva7w6nage".to_string(),
        ];
        let mut all = pinned.clone();
        all.extend(clean.clone());
        let cards = decode_cards(&all).expect("both cards reassemble cleanly");
        assert_eq!(cards.len(), 2);
        let warnings = seat_chunk_set_id_warnings(&cards);
        assert_eq!(
            warnings.len(),
            1,
            "exactly one mismatching group: {warnings:?}"
        );
        assert_eq!(
            warnings[0],
            chunk_set_id_mismatch_warning(0x99999, 0x69f0e),
            "warning content is the frozen R2/R6 wording with the (declared, derived) pair"
        );

        // The clean card alone -> silent (contract 6's clean-twin control).
        let clean_only = decode_cards(&clean).unwrap();
        assert!(seat_chunk_set_id_warnings(&clean_only).is_empty());
    }

    /// Pins `chunk_set_id_mismatch_warning`'s WORDING against INDEPENDENT
    /// literals — not by re-calling the function (which is tautological).
    /// This is the interim R6-drift guard: it must stay byte-identical to
    /// mk-cli's `chunk_set_id_mismatch_warning` and the mk corpus
    /// `warning_text`. The mechanical cross-repo binding is the follow-up
    /// `go-mk-vector-corpus-ingestion`; until then an md-cli-only edit that
    /// drops the (declared, derived) pair or the remedy sentence fails here.
    #[test]
    fn csid_warning_wording_is_pinned_against_literals_not_the_function_itself() {
        let w = chunk_set_id_mismatch_warning(0x99999, 0x69f0e);
        assert!(w.starts_with("warning:"), "{w}");
        assert!(
            w.contains(
                "this key card's stamped chunk-set id (99999) was not derived from its content"
            ),
            "{w}"
        );
        assert!(w.contains("which computes 69f0e"), "{w}");
        assert!(
            w.contains("diagnostics that name plates by id will call it 99999"),
            "{w}"
        );
        assert!(
            w.contains(
                "To fix it, re-mint: run mk encode again without --chunk-set-id and the id is \
                 derived from the key data automatically."
            ),
            "{w}"
        );
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

    /// REVIEW-converter-whole-diff-r1 I2. The key is case-folded; the string
    /// KEPT is the one as supplied, so the decoder still gets to rule on a
    /// mixed-case string rather than having it silently lower-cased here.
    #[test]
    fn dedupe_folds_case_and_keeps_the_first_spelling() {
        let v = vec![
            "mk1abc".to_string(),
            "MK1ABC".to_string(),
            "mk1 ABC".to_string(),
            "mk1def".to_string(),
        ];
        assert_eq!(dedupe_strings(&v), vec!["mk1abc", "mk1def"]);

        // First-appearance order, and the first SPELLING, both survive.
        let upper_first = vec!["MK1ABC".to_string(), "mk1abc".to_string()];
        assert_eq!(dedupe_strings(&upper_first), vec!["MK1ABC"]);
    }

    #[test]
    fn group_id_renders_the_full_twenty_bit_field() {
        assert_eq!(GroupId::Chunked(0x1_C77F).to_string(), "1c77f");
        assert_eq!(GroupId::Chunked(0).to_string(), "00000");
        assert_eq!(GroupId::Chunked(0xF_FFFF).to_string(), "fffff");
    }
}
