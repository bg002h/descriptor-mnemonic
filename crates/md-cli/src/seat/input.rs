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
//! double scan of one card is only harmless because step 1 runs first. **Since
//! P3** (SPEC contract 7 / R5), a group that does not form one key card is
//! caught by [`decode_cards`]'s own classifier — from the retained per-chunk
//! headers, before `mk_codec::decode` is even called for the shapes it can
//! name (merged cards, incomplete scan); only the residual "terminal" shapes
//! still reach `mk_codec::decode` and surface its error verbatim on a labeled
//! line.
//!
//! **Since P1 (SPEC §2, seat auto-partition): step 3 running LAST no longer
//! makes an id collision automatically fatal.** A group the classifier would
//! flag `Failure::Merged` is now handed to [`crate::seat::partition`] FIRST:
//! a clean collision (every canonical piece resolves to exactly the right
//! number of verified, fully-covering cards) SEATS as several
//! [`DecodedCard`]s sharing one `set_id`, distinguished by
//! [`DecodedCard::ordinal`] — the seating engine genuinely DOES see colliding
//! cards as several candidates now, deliberately. The classifier's own
//! message is unchanged and still fires whenever the engine reports "no
//! partition" (an inadmissible or under-verified group). Consequence for A5:
//! `--seat`'s "ambiguous id" case, once unreachable, is now the documented
//! `@i=<id>#<k>` grammar (SPEC §4) — see [`crate::seat::directive`].
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
use crate::seat::canonical::{CanonicalPieceKey, canonical_piece_key};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// SPEC §4 — `Some(k)` (1-based) when this card is one of several
    /// AUTO-PARTITIONED off ONE colliding `set_id` (`decode_cards`'s P1
    /// partition branch is the only constructor of `Some`); `None` for
    /// every card reassembled the ordinary, non-collided way. Mechanical
    /// P1 step 0 (plan-r1 M4): the field exists from here, but every
    /// construction site still sets it to `None` until step 4 wires the
    /// real ordinal.
    pub ordinal: Option<u32>,
}

impl DecodedCard {
    /// `set-id (stub …)` / `set-id#k (stub …)` — the identifier pair every
    /// A4/B1 message names a card by. Stubs are joined with `,` when a card
    /// declares several. SPEC §4: a collided card's ordinal is part of its
    /// name everywhere a card is named, so this one function is the single
    /// source every A3/A4/A5/B1 message renders through.
    pub fn label(&self) -> String {
        let stubs: Vec<String> = self
            .card
            .policy_id_stubs
            .iter()
            .map(|s| format!("{:02x}{:02x}{:02x}{:02x}", s[0], s[1], s[2], s[3]))
            .collect();
        match self.ordinal {
            Some(k) => format!("{}#{k} (stub {})", self.set_id, stubs.join(",")),
            None => format!("{} (stub {})", self.set_id, stubs.join(",")),
        }
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

/// SPEC §3's AP1 note text, on a successful auto-partition — draft wording,
/// with both counts SPEC §3 requires: `n_supplied` (this group's raw string
/// count BEFORE SPEC §1's canonicalisation collapse) and `n_distinct` (the
/// canonical piece count AFTER it) — two DIFFERENT numbers whenever a
/// benign duplicate (BCH twin, literal repeat) was also present, one number
/// when it was not. Neutral: names three possible origins, asserts none.
fn ap1_note(n_supplied: usize, n_distinct: usize, n_cards: usize, set_id: GroupId) -> String {
    let string_noun = if n_supplied == 1 { "string" } else { "strings" };
    let piece_noun = if n_distinct == 1 { "piece" } else { "pieces" };
    let card_noun = if n_cards == 1 { "card" } else { "cards" };
    format!(
        "note: these {n_supplied} supplied {string_noun} are {n_distinct} distinct {piece_noun} \
         (chunks) carrying one stamped chunk-set id (chunk-set {set_id}), and they are \
         {n_cards} different key {card_noun} — each card's own 4-byte integrity check accepted \
         its pieces, so they were separated. A shared stamped id can be a mint defect, an \
         attack, or a deliberate choice at encode time — if it is unexpected, check each card \
         alone with `mk inspect`."
    )
}

/// SPEC §3's AP1 note, tagged with the id GROUP it explains.
///
/// `decode_cards` can seat more than one colliding group in one call, and
/// SPEC §5 requires each group's note to precede only ITS OWN group's R2
/// warnings — never a global prepend (plan-r1 N3). Carrying `set_id` here,
/// rather than returning a bare `String`, is what lets `seat::run` place
/// each note correctly without re-deriving which group it belongs to.
#[derive(Debug, Clone)]
pub struct PartitionNote {
    /// The colliding id this note explains.
    pub set_id: GroupId,
    /// The rendered SPEC §3 note text (no `note: ` prefix — callers push it
    /// into `Seating.notes` the same way every other note arrives there).
    pub text: String,
}

/// SPEC §1 -- collapse a GROUP's strings that canonicalise to the SAME
/// piece, first appearance wins.
///
/// Every entry here has ALREADY been proven decodable by [`group_key_of`]
/// (step 2, immediately above this function's only call site, propagates
/// any decode failure with `?` before this ever runs), so
/// [`canonical_piece_key`] is infallible for every `Chunked` entry;
/// `Single`-headered entries (`info.is_none()`) carry no key to
/// canonicalise against (SPEC §1: "out of scope") and pass through
/// unchanged, one-for-one, never collapsed against each other or against a
/// `Chunked` entry.
fn canonicalize_group(group: &[(String, Option<ChunkInfo>)]) -> Vec<(String, Option<ChunkInfo>)> {
    let mut seen: Vec<CanonicalPieceKey> = Vec::new();
    let mut out = Vec::with_capacity(group.len());
    for (s, info) in group {
        let Some(_) = info else {
            out.push((s.clone(), *info));
            continue;
        };
        match canonical_piece_key(s) {
            Ok(key) => {
                if !seen.contains(&key) {
                    seen.push(key);
                    out.push((s.clone(), *info));
                }
                // else: a duplicate canonical piece (BCH-correctable twin or
                // a literal duplicate step 1's coarser check missed) --
                // first appearance already kept, this one is dropped.
            }
            // Structurally unreachable per this function's own contract
            // (see the doc comment): pass through rather than panic, so an
            // unforeseen future header shape is refused downstream exactly
            // as today rather than silently dropped here.
            Err(_) => out.push((s.clone(), *info)),
        }
    }
    out
}

/// The whole pipeline: `&[String]` of mk1 strings in, reassembled cards out,
/// in the normative order, plus any SPEC §2 auto-partition notes (P1 step 0
/// — plan-r1 M5: the mechanical signature change lands first so every later
/// test targets the final shape; `partition_notes` is always empty until P1
/// step 3 wires the engine in).
///
/// Groups are returned in ascending set-id order, NOT in supply order —
/// determinism of everything downstream (the assignment vector, every
/// refusal listing) must not depend on how the operator happened to type
/// the strings, and V-ORD pins that.
pub fn decode_cards(
    strings: &[String],
) -> Result<(Vec<DecodedCard>, Vec<PartitionNote>), CliError> {
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
    let mut partition_notes: Vec<PartitionNote> = Vec::new();
    for (set_id, group) in groups {
        // SPEC §1 (mk1-only; documented adjacent to `dedupe_strings` above,
        // not inside it — that function also serves md1). Collapses pieces
        // that canonicalise to the SAME `(chunk_set_id, total_chunks,
        // chunk_index, symbol_tail)` -- a benign double transcription (BCH
        // twins, literal duplicates step 1's coarser check missed) -- BEFORE
        // `classify` ever sees the group, so it cannot misread a collapsed
        // duplicate as a genuine second card. Run PER GROUP rather than once
        // globally across `deduped`: `CanonicalPieceKey` embeds
        // `chunk_set_id`, so two pieces can never canonicalise together
        // across different groups, and canonicalising after step 2's
        // grouping is what lets `n_supplied` (this group's count BEFORE the
        // collapse) survive for P1 step 3's AP1 note, without a second pass.
        let n_supplied = group.len();
        let group = canonicalize_group(&group);
        let n_distinct = group.len();
        let infos: Vec<ChunkInfo> = group.iter().filter_map(|(_, i)| *i).collect();
        // R5 arms 1/2: classify from the retained headers BEFORE ever
        // calling `mk_codec::decode`, so a merged/incomplete group gets the
        // classifier's own message rather than the codec's reassembly
        // error. Only non-empty (i.e. genuinely chunked) groups classify —
        // a `Single` group carries no chunk header to classify from.
        if !infos.is_empty() {
            match classify(&infos) {
                Some(Failure::Merged) => {
                    // SPEC §2 — a genuine collision SIGNAL (duplicate chunk
                    // index and/or disagreeing declared totals). Attempt
                    // auto-partition as a PRE-PASS before falling through to
                    // arm 1's shipped message; `Failure::Incomplete` below
                    // never reaches the engine (module doc comment on
                    // `partition` explains why that is safe, not merely
                    // unreached).
                    let group_strings: Vec<&str> = group.iter().map(|(s, _)| s.as_str()).collect();
                    match crate::seat::partition::partition(&group_strings) {
                        crate::seat::partition::Outcome::Seated(seated) => {
                            let k = seated.len();
                            for (i, card) in seated.into_iter().enumerate() {
                                cards.push(DecodedCard {
                                    set_id,
                                    card,
                                    ordinal: Some(i as u32 + 1),
                                });
                            }
                            partition_notes.push(PartitionNote {
                                set_id,
                                text: ap1_note(n_supplied, n_distinct, k, set_id),
                            });
                            continue;
                        }
                        crate::seat::partition::Outcome::Ambiguous => {
                            return Err(crate::seat::partition::ap2_refusal(set_id));
                        }
                        crate::seat::partition::Outcome::CapExceeded { sigma_k } => {
                            return Err(crate::seat::partition::cap_refusal(set_id, sigma_k));
                        }
                        crate::seat::partition::Outcome::OverBudget { product } => {
                            return Err(crate::seat::partition::budget_refusal(set_id, product));
                        }
                        crate::seat::partition::Outcome::NoPartition => {
                            return Err(merged_refusal(set_id, &infos));
                        }
                    }
                }
                Some(Failure::Incomplete { declared_total }) => {
                    return Err(incomplete_refusal(set_id, group.len(), declared_total));
                }
                None => {}
            }
        }
        let refs: Vec<&str> = group.iter().map(|(s, _)| s.as_str()).collect();
        let card = mk_codec::decode(&refs).map_err(|e| terminal_refusal(set_id, e))?;
        cards.push(DecodedCard {
            set_id,
            card,
            ordinal: None,
        });
    }
    Ok((cards, partition_notes))
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
///
/// A thin wrapper over [`seat_notes`] with no AP1 notes to interleave —
/// UNCHANGED signature and behaviour, so this function's own pinned tests
/// keep testing exactly what they always tested. `seat::run` now calls
/// [`seat_notes`] directly (it always has AP1 notes, even if empty), so this
/// wrapper is exercised only by its own test module — kept as a real,
/// independently-testable unit rather than folded away, mirroring how
/// `canonical_piece_key` shipped one cycle ahead of its production caller.
#[allow(dead_code)]
pub fn seat_chunk_set_id_warnings(cards: &[DecodedCard]) -> Vec<String> {
    seat_notes(cards, &[])
}

/// SPEC §5 / plan-r1 N3 — interleave P1's SPEC §2 auto-partition notes with
/// the R2/R6 mismatch warnings above, PER GROUP: a group's own AP1 note is
/// emitted the moment its FIRST card is reached (which — `cards` being
/// ascending set-id order and every card of one collided group being
/// contiguous, per `decode_cards`'s own contract — is always ahead of every
/// R2 warning that group can produce), never as a global prepend ahead of
/// every group's warnings.
pub fn seat_notes(cards: &[DecodedCard], ap_notes: &[PartitionNote]) -> Vec<String> {
    let mut out = Vec::new();
    let mut emitted: std::collections::HashSet<GroupId> = std::collections::HashSet::new();
    for c in cards {
        if emitted.insert(c.set_id) {
            if let Some(n) = ap_notes.iter().find(|n| n.set_id == c.set_id) {
                out.push(n.text.clone());
            }
        }
        let GroupId::Chunked(declared) = c.set_id else {
            continue;
        };
        let Some(derived) = derived_chunk_set_id(&c.card) else {
            continue;
        };
        if declared != derived {
            out.push(chunk_set_id_mismatch_warning(declared, derived));
        }
    }
    out
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
        let (cards, notes) = decode_cards(&twice).expect("a doubled full set must reassemble");
        assert_eq!(cards.len(), 11, "the doubled set is still 11 cards");
        assert!(notes.is_empty(), "no collisions here, no partition notes");
        let (once, _) = decode_cards(&one).expect("the single set must reassemble");
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
        let (cards, _) = decode_cards(&mixed).expect("grouped + unbroken is one set, not two");
        assert_eq!(cards.len(), 11);
    }

    #[test]
    fn v_dup_supply_order_does_not_change_the_decoded_card_list() {
        let one = pathological_mk1();
        let mut reversed = one.clone();
        reversed.reverse();
        let (a, _) = decode_cards(&one).unwrap();
        let (b, _) = decode_cards(&reversed).unwrap();
        let ids_a: Vec<GroupId> = a.iter().map(|c| c.set_id).collect();
        let ids_b: Vec<GroupId> = b.iter().map(|c| c.set_id).collect();
        assert_eq!(ids_a, ids_b, "groups are returned in set-id order");
    }

    // ─── V-COLLIDE ──────────────────────────────────────────────────────

    /// SPEC row 7a (mixed-totals, both classes complete): `v-collide.txt`'s
    /// two DISAGREEING-total cards (card A 2-chunk, card B 3-chunk, one
    /// pinned id) now AUTO-PARTITION apart instead of refusing at
    /// reassembly (plan-r1 N1 / SPEC row 7 / row 10b — REWRITTEN per the
    /// spec's own §12 churn note: this row moves from "refuses" to
    /// "seats", and the OLD refusal assertions below move to
    /// `r5_merged_two_cards_pinned_to_one_id_classify_as_merged`, which now
    /// exercises the genuine-duplicate-index shape directly rather than via
    /// this fixture).
    #[test]
    fn v_collide_two_cards_pinned_to_one_chunk_set_id_both_seat() {
        let strings: Vec<String> = include_str!("../../tests/fixtures/seating/v-collide.txt")
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .map(str::to_string)
            .collect();
        assert_eq!(strings.len(), 5, "card A (2 chunks) + card B (3 chunks)");
        let (cards, notes) = decode_cards(&strings)
            .expect("both total-classes trivially seat: k_class=1 each, Sigma k=2, tiny budget");
        assert_eq!(cards.len(), 2, "both classes' one card each seats");
        assert_eq!(notes.len(), 1, "one AP1 note for the one colliding group");
        assert!(notes[0].set_id == GroupId::Chunked(0x12345));
        assert!(notes[0].text.starts_with("note:"), "{}", notes[0].text);
        // Ordinal identity (SPEC §4), both cards share the id, distinguished
        // by `#<k>` -- ascending `encode_bytecode`, so which physical card
        // (A or B) gets #1 vs #2 is decided by that order, not supply order.
        let mut labels: Vec<String> = cards.iter().map(DecodedCard::label).collect();
        labels.sort();
        assert_eq!(
            labels,
            vec!["12345#1 (stub 5b48af35)", "12345#2 (stub 5b48af35)"]
        );
        // Origins are preserved from each physical card, whichever ordinal
        // it landed on -- one is 0'/2', the other 1'/2'.
        let paths: std::collections::BTreeSet<String> = cards
            .iter()
            .map(|c| c.card.origin_path.to_string())
            .collect();
        assert!(paths.contains("48'/0'/0'/2'") && paths.contains("48'/0'/1'/2'"));
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

    // ─── SPEC §1 canonicalisation (P1 step 1) — row 2, BCH-twin ────────

    /// SPEC row 2: a card's two chunks, PLUS a 1-char-flipped (BCH
    /// t=4-correctable) twin of each chunk — four raw strings, one
    /// declared id, two of them duplicating chunk_index 0 and two
    /// duplicating chunk_index 1. Without step 1's canonicalisation this
    /// would classify as `Failure::Merged` (duplicate chunk index) and
    /// refuse; WITH it, `decode_string` corrects each twin back to the
    /// identical 5-bit data, `canonical_piece_key` sees ONE piece per
    /// index, and the group reassembles as ONE card, silently (no error,
    /// no partition note — this is not a collision at all).
    #[test]
    fn row2_bch_correctable_twin_collapses_to_one_card_silently() {
        let canonical_card0: Vec<String> =
            include_str!("../../tests/fixtures/seating/v-ap-canonical.txt")
                .lines()
                .map(str::trim)
                .filter(|l| l.starts_with("mk1"))
                .take(2) // card 0's own two chunks
                .map(str::to_string)
                .collect();
        let twins: Vec<String> = include_str!("../../tests/fixtures/seating/v-ap-bchtwin.txt")
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .map(str::to_string)
            .collect();
        assert_eq!(twins.len(), 2, "one flipped twin per chunk");

        let mut strings = canonical_card0.clone();
        strings.extend(twins);
        assert_eq!(strings.len(), 4, "2 genuine chunks + 2 BCH-flipped twins");

        let (cards, notes) = decode_cards(&strings)
            .expect("a BCH-correctable double transcription must collapse and seat, not refuse");
        assert_eq!(cards.len(), 1, "the twin collapses to ONE card, not two");
        assert!(
            notes.is_empty(),
            "not a genuine collision, so no AP1 partition note: {notes:?}"
        );

        // The reconstructed card is byte-identical to decoding the genuine
        // chunks alone (the twin contributes nothing new — same card).
        let (alone, _) = decode_cards(&canonical_card0).unwrap();
        assert_eq!(cards[0].card, alone[0].card);
        assert_eq!(cards[0].set_id, alone[0].set_id);
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

    /// SPEC row 1 (canonical-collision), REWRITTEN from the pre-P1 arm-1
    /// pin (plan §12 churn note: "→ canonical row"). Two DIFFERENT 2-chunk
    /// cards, both pinned to chunk-set `11111`, no shared pieces at all (4
    /// strings, 4 distinct canonical pieces) — the SAME fixture that used
    /// to prove arm 1's message now proves the auto-partition seats it: a
    /// clean, no-sharing 2-card collision is exactly what SPEC §2 exists to
    /// untangle. Arm 1's own message is still pinned, unchanged, by
    /// [`r5_classification_order_prefers_merged_over_incomplete`] (the
    /// `44444` admissibility-failure shape, SPEC row 6) — this fixture no
    /// longer exercises that path at all.
    #[test]
    fn row1_canonical_collision_two_cards_seat_with_ap1_note() {
        let strings: Vec<String> = [
            "mk1qpzyg3pqqsq4kj90x9eutks2q5zg3vs7rnefw94m5rru59s2su80aw2q4wgdpapgfl4pkhsdyytkwl5z8lphut2hvvpp5x94fvrdwgu6g0lq",
            "mk1qpzyg3pp806lhaeh6reknylagmwyjycf8044xtt9flsdlkvt6f6cthyl99lqmcdjwej7x7ylmk0lq",
            "mk1qpzyg3pqqsq4kj90xxux3r03q5zg3vs7llvu2xd8x2rk7av9gmew82jq5zap9302ynhp37ggd6z5u4emag0zr8gh9upnj76samqgd9kc0zqu",
            "mk1qpzyg3ppwyp4dfykwfkgg6fxyxetdcmythf4hsqzd3v879jprztejzs7ruvy26n2gd25y20mx4xyg",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let (cards, notes) = decode_cards(&strings).expect("a clean 2-card collision must seat");
        assert_eq!(cards.len(), 2);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].set_id, GroupId::Chunked(0x11111));
        // Both counts pinned: 4 supplied strings ARE 4 distinct pieces here
        // (no shared/BCH-twin collapse in this fixture, unlike V-AP-SHARED)
        // -- and 2 different key cards.
        assert_eq!(
            notes[0].text,
            "note: these 4 supplied strings are 4 distinct pieces (chunks) carrying one \
             stamped chunk-set id (chunk-set 11111), and they are 2 different key cards — each \
             card's own 4-byte integrity check accepted its pieces, so they were separated. A \
             shared stamped id can be a mint defect, an attack, or a deliberate choice at \
             encode time — if it is unexpected, check each card alone with `mk inspect`."
        );
        let mut labels: Vec<String> = cards.iter().map(DecodedCard::label).collect();
        labels.sort();
        assert_eq!(
            labels,
            vec!["11111#1 (stub 5b48af31)", "11111#2 (stub 5b48af31)"]
        );
        // Physically distinct cards, not one card counted twice.
        assert_ne!(cards[0].card.xpub, cards[1].card.xpub);
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
        let (cards, _) = decode_cards(&all).expect("both cards reassemble cleanly");
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
        let (clean_only, _) = decode_cards(&clean).unwrap();
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

    // ─── SPEC §2/§3 outcomes, via `decode_cards` directly (P1 step 3) ──
    //
    // Engine-unit coverage (admissibility/cap/budget arithmetic, V=k) lives
    // in `seat::partition`'s own tests, directly on these SAME P0 fixtures.
    // What lives HERE is the OUTCOME as `decode_cards` actually returns or
    // refuses it -- the pre-pass ordering and message content, re-asserted
    // at this layer per plan-r1 I1 ("both layers are required").

    fn seating_mk1_lines(text: &str) -> Vec<String> {
        text.lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .map(str::to_string)
            .collect()
    }

    /// Row 3 -- shared-piece pair (2 cards, 13 shared stubs at chunk 0):
    /// seats via reuse: `|V_class| = k_class = 2`, chunk 0's ONE canonical
    /// piece counted in BOTH cards' cover.
    #[test]
    fn row3_shared_piece_pair_seats_two_cards_via_reuse() {
        let strings =
            seating_mk1_lines(include_str!("../../tests/fixtures/seating/v-ap-shared.txt"));
        let (cards, notes) = decode_cards(&strings).expect("shared-piece pair must seat via reuse");
        assert_eq!(cards.len(), 2);
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].text.contains("2 different key cards"),
            "{}",
            notes[0].text
        );
    }

    /// Row 4 floor -- 3 cards, n=11, distinct stubs: seats within budget
    /// (177,147 candidates, well under `PARTITION_DECODE_BOUND`).
    #[test]
    fn row4_floor_set_seats_three_cards_within_budget() {
        let strings =
            seating_mk1_lines(include_str!("../../tests/fixtures/seating/v-ap-floor.txt"));
        let (cards, notes) = decode_cards(&strings).expect("the floor set must seat");
        assert_eq!(cards.len(), 3);
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].text.contains("3 different key cards"),
            "{}",
            notes[0].text
        );
    }

    /// Row 4 boundary -- the SAME shape at n=12 (531,441 candidates): the
    /// first refusing size, budget refusal naming AP3's rationale.
    #[test]
    fn row4_boundary_set_refuses_over_budget_naming_ap3_rationale() {
        let strings = seating_mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-boundary.txt"
        ));
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("531441"), "names the computed product: {msg}");
        assert!(
            msg.contains(&crate::seat::partition::PARTITION_DECODE_BOUND.to_string()),
            "names the fixed bound: {msg}"
        );
        assert!(
            msg.contains("too long") && msg.contains("hang"),
            "AP3's rationale (checking every candidate would take too long / could hang): {msg}"
        );
        assert!(msg.contains("Re-scan one card's pieces alone"), "{msg}");
    }

    /// Row 5 -- the over-budget synthetic set (n=32, 5 cards, 5^32-scale):
    /// static refusal, ZERO decodes (checked at the engine-unit layer;
    /// re-asserted HERE that the refusal reaches `decode_cards` with the
    /// same shape as the boundary row's).
    #[test]
    fn row5_over_budget_synthetic_set_refuses_statically() {
        const CHUNK_SET_ID: u32 = 0x7_7777;
        const TOTAL_CHUNKS: u8 = 32;
        let owned: Vec<Vec<String>> = (0..5u8)
            .map(|card| crate::seat::synth::synth_card_strings(CHUNK_SET_ID, TOTAL_CHUNKS, card))
            .collect();
        let strings: Vec<String> = owned.into_iter().flatten().collect();
        assert_eq!(strings.len(), 160);
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("chunk-set 77777"), "{msg}");
        assert!(
            msg.contains(&u64::MAX.to_string()),
            "the saturated product names itself: {msg}"
        );
    }

    /// Row 7b -- incomplete-class set (a complete 2-chunk class + a
    /// 3-chunk class genuinely missing index 2, one id): the WHOLE GROUP
    /// refuses via arm 1 (r1-C3 fail-closed composition) -- even though the
    /// 2-chunk class alone would trivially seat. This is the SAME
    /// `merged_refusal` call/message the pre-P1 build made; P1 changes
    /// only whether the engine is consulted first, never arm 1's wording.
    #[test]
    fn row7b_incomplete_class_set_refuses_the_whole_group_via_arm_1() {
        let strings = seating_mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-incomplete.txt"
        ));
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("chunk-set a1006"), "{msg}");
        // Arm 1's own message shape (never arm 2's "scan the missing
        // piece(s)", and never a codec `error:` line -- this is arm 1, not
        // arm 3).
        assert!(
            msg.contains("piece order does not matter"),
            "arm 1's message: {msg}"
        );
        assert!(
            msg.contains("Re-scan one card's pieces alone"),
            "arm 1's remedy: {msg}"
        );
        assert!(
            !msg.contains("scan the missing piece"),
            "must NOT be arm 2's incomplete message: {msg}"
        );
        assert!(!msg.contains("error:"), "no codec line -- not arm 3: {msg}");
        assert_no_retired_wording(&msg);
    }

    /// Row 9 -- the AP2 fixture (committed, one-grind script): a GENUINE
    /// ambiguity (`|V| = 4 > k = 2`) -- AP2 hard refusal, nothing seats.
    #[test]
    fn row9_ap2_fixture_hard_refuses_nothing_seats() {
        let strings = seating_mk1_lines(include_str!("../../tests/fixtures/seating/v-ap2.txt"));
        let err = decode_cards(&strings).unwrap_err();
        let msg = err.to_string();
        assert_eq!(
            msg,
            "seating refused: chunk-set a1999: these pieces (chunks) verify as more key cards \
             than they can belong to, and the tool will not guess which cards are your wallet. \
             This is not expected from accidental damage — treat the strings as untrusted and \
             re-scan one card's pieces alone, from a source you trust."
        );
    }

    /// Row 10(a) -- the AP2 fixture's OWN framing under §Security: a
    /// GROUND same-id extra verified candidate raises `|V| > k` in one
    /// class -- the SAME construction and refusal as row 9 (r3-I4's
    /// `[2,3,3]` grind IS the ground-extra-candidate case), asserted here
    /// under its §Security name so the row is not missed by number.
    #[test]
    fn row10a_ground_extra_verified_candidate_is_the_row9_ap2_construction() {
        let strings = seating_mk1_lines(include_str!("../../tests/fixtures/seating/v-ap2.txt"));
        assert!(
            decode_cards(&strings)
                .unwrap_err()
                .to_string()
                .contains("verify as more key cards than they can belong to")
        );
    }

    /// Row 8 -- permutation invariance: the ORDER KEY (SPEC §4, ascending
    /// `encode_bytecode`) is a property of each card's OWN CONTENT, not of
    /// how the operator happened to type the strings. Feeding the shared
    /// group's chunks in supply order vs. fully reversed must assign the
    /// SAME physical card to `#1` and the SAME one to `#2`.
    #[test]
    fn row8_ordinal_assignment_is_invariant_under_supply_order() {
        let strings: Vec<String> = seating_mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-canonical.txt"
        ));
        assert_eq!(strings.len(), 4);
        let mut reversed = strings.clone();
        reversed.reverse();

        let (forward, _) = decode_cards(&strings).unwrap();
        let (backward, _) = decode_cards(&reversed).unwrap();
        assert_eq!(forward.len(), 2);
        assert_eq!(backward.len(), 2);

        let by_ordinal = |cards: &[DecodedCard]| -> Vec<(Option<u32>, bitcoin::bip32::Xpub)> {
            let mut v: Vec<_> = cards.iter().map(|c| (c.ordinal, c.card.xpub)).collect();
            v.sort_by_key(|(k, _)| *k);
            v
        };
        assert_eq!(
            by_ordinal(&forward),
            by_ordinal(&backward),
            "the SAME card must land on the SAME ordinal regardless of supply order"
        );
    }
}
