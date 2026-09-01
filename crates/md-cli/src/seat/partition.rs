//! SPEC `design/SPEC_seat_auto_partition.md` §2 — the partition engine.
//!
//! Runs as a PRE-PASS `decode_cards`'s per-group loop reaches only when the
//! shipped R5 classifier ([`super::input`]'s `classify`) would report
//! `Failure::Merged` for a chunked group — a genuine collision SIGNAL
//! (duplicate chunk index, and/or disagreeing declared totals across the
//! group). A plain `Failure::Incomplete` group (fewer strings than one
//! card's own declared total, no duplicates at all) never reaches here: by
//! construction it can only ever admit ONE total-class whose admissibility
//! (§2.2) fails at at least one index, so routing it through this engine
//! would be equivalent but pointless — and SPEC's own constraint that the
//! shipped arm-2 message/tests stay UNTOUCHED is cleanest kept by never
//! calling this engine for that case at all.
//!
//! One evaluation order, ONE id group of canonical pieces (SPEC §2's own
//! framing): sub-group by declared total (§2.1) → per-class admissibility +
//! `k_class` (§2.2) → group-wide cap (§2.3) → static saturating budget
//! (§2.4) → enumerate/verify/seat-condition per class (§2.5) →
//! ambiguity/failure split (§2.6). The first refusal reached wins; nothing
//! past it runs.

use crate::error::CliError;
use crate::seat::canonical::{CanonicalPieceKey, canonical_piece_key};
use crate::seat::input::GroupId;
use mk_codec::KeyCard;
use std::collections::{BTreeMap, HashSet};

/// SPEC §2.3's group-wide AP3 cap: at most 5 colliding cards, summed
/// ACROSS every total-class in the id group (r3-I5 — not per class).
pub const GROUP_CAP: usize = 5;

/// SPEC §2.4's static, saturating decode budget — `Σ_classes Π_indexes
/// count_i` over the CANONICAL PIECE COUNTS, decided before any decode.
///
/// **Measured on THIS repo's `[profile.test]` (opt-level = 2,
/// `debug_assertions` on — the profile `cargo nextest run` actually
/// builds, per the repo-wide directive to raise optimisation rather than
/// drop assertions): decoding one real 11-chunk card (the row-4 floor
/// fixture's card 0) 20,000 times via `mk_codec::decode` measured
/// 196.334 ms / 20,000 = 9,816 ns = 9.816 µs/candidate**
/// (`cargo test -p md-cli --bin md -- --ignored --nocapture
/// budget_measurement`, `#[ignore]`d so routine runs don't pay for it — see
/// `budget_measurement_per_candidate_decode_cost` below to re-measure).
///
/// At that rate: the floor (177,147 candidates) costs ≈ 1.74 s; the
/// boundary (531,441) costs ≈ 5.22 s. `200_000` sits inside the plan's
/// acceptance window `177_147 ≤ BOUND < 531_441`, sits comfortably above
/// the floor (which must never itself bound-refuse) and comfortably below
/// the boundary (which must), and keeps the worst case ANY accepted group
/// can cost at `200_000 * 9.816µs ≈ 1.96 s` — inside the ~2 s target SPEC
/// §2.4 states. SPEC §2.4's own prose cites a larger constant (≈255,000)
/// derived from a DIFFERENT (release-profile, 7.845 µs) measurement; this
/// constant is re-derived on the profile the gate actually runs, per the
/// plan's explicit instruction, rather than carried over from the release
/// number — the two profiles are ~25% apart in per-candidate cost, which
/// would have pushed 255,000 candidates to ≈2.50 s, over target.
pub const PARTITION_DECODE_BOUND: u64 = 200_000;

/// Compile-time guard on the plan's own acceptance window
/// (`177_147 ≤ BOUND < 531_441`, plan-r1 I4): outside it the row-4 floor
/// and boundary rows invert. A build fails here rather than a test failing
/// later if the constant is ever changed without re-checking this.
const _: () = assert!(PARTITION_DECODE_BOUND >= 177_147 && PARTITION_DECODE_BOUND < 531_441);

/// One string collapsed to its SPEC §1 canonical identity, retaining ONE
/// representative string (first appearance) — the string a candidate is
/// actually decoded through.
#[derive(Debug, Clone)]
struct Piece<'a> {
    key: CanonicalPieceKey,
    string: &'a str,
}

/// What SPEC §2 decided for one id group.
#[derive(Debug)]
pub enum Outcome {
    /// Every total-class seated. Cards are in SPEC §4's order key (ascending
    /// `encode_bytecode`), group-wide (never restarted per class).
    Seated(Vec<KeyCard>),
    /// SPEC §2.6: `|V_class| > k_class` in some class — AP2, a constructed
    /// ambiguity. Never a guess.
    Ambiguous,
    /// SPEC §2.3: `Σ_classes k_class > GROUP_CAP`.
    CapExceeded { sigma_k: usize },
    /// SPEC §2.4: the static product exceeds [`PARTITION_DECODE_BOUND`].
    /// ZERO decodes are issued when this fires — the product is computed
    /// entirely from canonical piece COUNTS.
    OverBudget { product: u64 },
    /// SPEC §2.2/§2.6: admissibility failed (an index with zero pieces in
    /// some class) OR `|V_class| < k_class` / an uncovered piece in some
    /// class. Falls through to the shipped arm-1 message, UNCHANGED.
    NoPartition,
}

/// Test-only instrumentation: counts calls into [`verify_candidate`], the
/// ENGINE's single `mk_codec::decode` call site. Rows 5 and 6's "zero
/// decodes" claims are proved by resetting this to 0, running [`partition`],
/// and asserting it is STILL 0 — not inferred from reading the code.
#[cfg(test)]
pub(crate) static DECODE_CALLS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

fn verify_candidate(refs: &[&str]) -> Option<KeyCard> {
    #[cfg(test)]
    DECODE_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    mk_codec::decode(refs).ok()
}

/// One total-class, sub-grouped by declared index.
struct Class<'a> {
    total_chunks: u8,
    by_index: BTreeMap<u8, Vec<&'a Piece<'a>>>,
    k_class: usize,
}

/// SPEC §2: run the whole partition contract over one id group's raw
/// strings (already proven decodable by `group_key_of` upstream — see
/// [`super::input::canonicalize_group`], whose collapse this function
/// idempotently repeats so it stays independently testable directly on raw
/// fixture text, exactly as [`super::p0_shapes`] already does).
pub fn partition(strings: &[&str]) -> Outcome {
    // Canonicalise + collapse duplicates (SPEC §1, via the shipped key fn —
    // never a second implementation, plan-r1 M3).
    let mut seen: Vec<CanonicalPieceKey> = Vec::new();
    let mut pieces: Vec<Piece> = Vec::new();
    for s in strings {
        let Ok(key) = canonical_piece_key(s) else {
            // M3 (whole-diff review): `partition` is `pub(crate)` and its
            // OWN unit tests (below) call it directly on raw fixture text
            // -- unlike `decode_cards`, which never reaches this arm
            // (every string it passes already round-tripped through
            // `group_key_of`). Refuse the WHOLE group rather than
            // silently shrinking the piece set and seating on what
            // remains: a fail-open shape in a test-facing entry point.
            return Outcome::NoPartition;
        };
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        pieces.push(Piece { key, string: s });
    }

    // §2.1 — sub-group by declared total.
    let mut by_total: BTreeMap<u8, Vec<&Piece>> = BTreeMap::new();
    for p in &pieces {
        by_total.entry(p.key.total_chunks).or_default().push(p);
    }

    // §2.2 — per-class admissibility (zero decodes) + k_class.
    let mut classes: Vec<Class> = Vec::new();
    for (&total_chunks, members) in &by_total {
        let mut by_index: BTreeMap<u8, Vec<&Piece>> = BTreeMap::new();
        for &p in members {
            by_index.entry(p.key.chunk_index).or_default().push(p);
        }
        for idx in 0..total_chunks {
            if !by_index.contains_key(&idx) {
                // Any index with ZERO pieces -> the class fails -> the
                // WHOLE GROUP is "no partition" (r1 C3: fail-closed
                // composition), decided without a single decode call.
                return Outcome::NoPartition;
            }
        }
        let k_class = by_index.values().map(Vec::len).max().unwrap_or(0);
        classes.push(Class {
            total_chunks,
            by_index,
            k_class,
        });
    }

    // §2.3 — group-wide cap, BEFORE budget/enumeration.
    let sigma_k: usize = classes.iter().map(|c| c.k_class).sum();
    if sigma_k > GROUP_CAP {
        return Outcome::CapExceeded { sigma_k };
    }

    // §2.4 — static, saturating budget. A function of canonical piece
    // COUNTS alone; decided before any decode. I1 (whole-diff review):
    // SUM across classes (each class is enumerated SEPARATELY below, in
    // its own `verify_class` call), product WITHIN one class's own
    // indices -- SPEC §2.4 and this constant's own doc comment above both
    // say `Sigma_classes Pi_indexes count_i`; the shipped fold multiplied
    // ACROSS classes too, a Pi over the whole group, which refuses a
    // family of inputs SPEC row 7 requires to seat (fails closed, so
    // never a wrong seat -- just an over-eager refusal) and, for a
    // multi-class group, prints a candidate count the engine would never
    // actually have to check.
    let product: u64 = classes.iter().fold(0u64, |acc, c| {
        let class_product = c
            .by_index
            .values()
            .fold(1u64, |a, v| a.saturating_mul(v.len() as u64));
        acc.saturating_add(class_product)
    });
    if product > PARTITION_DECODE_BOUND {
        return Outcome::OverBudget { product };
    }

    // §2.5/§2.6 — enumerate, verify, seat condition, per class; AP2 takes
    // priority over a mere no-partition verdict in another class (the
    // stricter, security-relevant outcome wins — SPEC's ambiguity rule is
    // "never a guess", which outranks "incomplete").
    let mut seated_all: Vec<KeyCard> = Vec::new();
    let mut saw_ambiguous = false;
    let mut saw_no_partition = false;
    for class in &classes {
        match verify_class(class) {
            ClassVerdict::Seats(cards) => seated_all.extend(cards),
            ClassVerdict::Ambiguous => saw_ambiguous = true,
            ClassVerdict::NoPartition => saw_no_partition = true,
        }
    }
    if saw_ambiguous {
        return Outcome::Ambiguous;
    }
    if saw_no_partition {
        return Outcome::NoPartition;
    }

    order_and_seat(seated_all)
}

/// SPEC §4 order key: ascending `mk_codec::bytecode::encode_bytecode`,
/// group-wide (never restarted per class -- R0-seat-auto-partition-r5's §4
/// cross-check: the ordinal keys off the shared id, not `(id, total)`).
///
/// M5 (whole-diff review): a card that fails to re-encode refuses the
/// GROUP rather than defaulting its sort key to empty -- two co-failing
/// cards would otherwise tie on that default and fall back to enumeration
/// (supply) order, making `#<k>` supply-order dependent, the one thing
/// SPEC §4 exists to prevent. Unreachable through the real pipeline: every
/// card here already round-tripped through `mk_codec::decode`, which
/// enforces every `encode_bytecode` invariant (non-empty stubs, `depth ==
/// path length`, path <= `MAX_PATH_COMPONENTS`) -- but this function is
/// also called directly by its own unit test on a hand-built card that
/// violates one, same rationale as M3.
fn order_and_seat(seated_all: Vec<KeyCard>) -> Outcome {
    let mut keyed: Vec<(Vec<u8>, KeyCard)> = Vec::with_capacity(seated_all.len());
    for card in seated_all {
        match mk_codec::bytecode::encode_bytecode(&card) {
            Ok(key) => keyed.push((key, card)),
            Err(_) => return Outcome::NoPartition,
        }
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    Outcome::Seated(keyed.into_iter().map(|(_, c)| c).collect())
}

enum ClassVerdict {
    Seats(Vec<KeyCard>),
    Ambiguous,
    NoPartition,
}

/// SPEC §2.5: enumerate every candidate (one canonical piece per index,
/// reuse permitted), verify each via `mk_codec::decode`, and decide the
/// class per §2.6. `V_class` is DISTINCT verified cards (identity = decoded
/// card); "cover" is the union of pieces used by every candidate that
/// verifies (SPEC: "no chosen cover, no subset search" — r3-M5).
fn verify_class(class: &Class) -> ClassVerdict {
    let indices: Vec<u8> = (0..class.total_chunks).collect();
    let mut distinct_cards: Vec<KeyCard> = Vec::new();
    let mut covered: HashSet<CanonicalPieceKey> = HashSet::new();
    let mut current: Vec<&Piece> = Vec::with_capacity(indices.len());
    enumerate_candidates(
        &indices,
        &class.by_index,
        0,
        &mut current,
        &mut |candidate: &[&Piece]| {
            let refs: Vec<&str> = candidate.iter().map(|p| p.string).collect();
            if let Some(card) = verify_candidate(&refs) {
                if !distinct_cards.iter().any(|c| c == &card) {
                    distinct_cards.push(card);
                }
                for p in candidate {
                    covered.insert(p.key.clone());
                }
            }
        },
    );

    let total_pieces: usize = class.by_index.values().map(Vec::len).sum();
    if distinct_cards.len() > class.k_class {
        ClassVerdict::Ambiguous
    } else if distinct_cards.len() == class.k_class && covered.len() == total_pieces {
        ClassVerdict::Seats(distinct_cards)
    } else {
        ClassVerdict::NoPartition
    }
}

/// Recursively enumerate the cartesian product of `by_index`'s entries, one
/// piece per declared index, calling `on_candidate` once per combination.
fn enumerate_candidates<'a>(
    indices: &[u8],
    by_index: &BTreeMap<u8, Vec<&'a Piece<'a>>>,
    pos: usize,
    current: &mut Vec<&'a Piece<'a>>,
    on_candidate: &mut impl FnMut(&[&'a Piece<'a>]),
) {
    if pos == indices.len() {
        on_candidate(current);
        return;
    }
    for piece in &by_index[&indices[pos]] {
        current.push(piece);
        enumerate_candidates(indices, by_index, pos + 1, current, on_candidate);
        current.pop();
    }
}

/// SPEC §2.3's cap refusal — AP3's exact wording.
pub fn cap_refusal(set_id: GroupId, sigma_k: usize) -> CliError {
    CliError::Seat(format!(
        "chunk-set {set_id}: these pieces (chunks) would need more than {GROUP_CAP} key cards \
         to explain ({sigma_k} across this id's classes) — auto-separation caps at {GROUP_CAP}. \
         Re-scan one card's pieces alone."
    ))
}

/// SPEC §2.4's budget refusal — names the boundary (the computed product
/// and the fixed bound) and AP3's rationale (checking every candidate would
/// take too long, so this refuses rather than guess or hang).
pub fn budget_refusal(set_id: GroupId, product: u64) -> CliError {
    CliError::Seat(format!(
        "chunk-set {set_id}: these pieces (chunks) admit {product} candidate key-card \
         combinations to check, more than auto-separation's budget of \
         {PARTITION_DECODE_BOUND} — checking them all would take too long, so auto-separation \
         refuses rather than guess or hang. Re-scan one card's pieces alone."
    ))
}

/// SPEC §2.6 / §3's AP2 hard refusal — verbatim per the spec draft.
pub fn ap2_refusal(set_id: GroupId) -> CliError {
    CliError::Seat(format!(
        "chunk-set {set_id}: these pieces (chunks) verify as more key cards than they can \
         belong to, and the tool will not guess which cards are your wallet. This is not \
         expected from accidental damage — treat the strings as untrusted and re-scan one \
         card's pieces alone, from a source you trust."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    fn mk1_lines(text: &str) -> Vec<&str> {
        text.lines()
            .map(str::trim)
            .filter(|l| l.starts_with("mk1"))
            .collect()
    }

    fn reset_decode_calls() {
        DECODE_CALLS.store(0, Ordering::SeqCst);
    }

    // ─── budget measurement (re-derives PARTITION_DECODE_BOUND's comment) ──

    /// Re-measures the per-candidate `mk_codec::decode` cost ON THIS TEST
    /// PROFILE, using the row-4 floor fixture's own real card. `#[ignore]`d
    /// so the routine suite never pays for 20,000 decodes; run explicitly:
    /// `cargo nextest run --locked -p md-cli budget_measurement -- --ignored --nocapture`.
    #[test]
    #[ignore = "timing measurement, run explicitly to re-derive PARTITION_DECODE_BOUND"]
    fn budget_measurement_per_candidate_decode_cost() {
        let floor = include_str!("../../tests/fixtures/seating/v-ap-floor.txt");
        let card0: Vec<&str> = mk1_lines(floor).into_iter().take(11).collect();
        assert_eq!(card0.len(), 11);
        assert!(mk_codec::decode(&card0).is_ok(), "sanity: card 0 decodes");

        const ITERS: u32 = 20_000;
        let start = std::time::Instant::now();
        for _ in 0..ITERS {
            std::hint::black_box(mk_codec::decode(std::hint::black_box(&card0)).unwrap());
        }
        let elapsed = start.elapsed();
        let per_iter_ns = elapsed.as_nanos() / u128::from(ITERS);
        eprintln!(
            "MEASURED: {ITERS} decodes in {elapsed:?} = {per_iter_ns} ns/candidate \
             ({:.3} us/candidate)",
            per_iter_ns as f64 / 1000.0
        );
    }

    // ─── §2.2 admissibility, §2.3 cap, §2.4 budget arithmetic (engine-unit,
    // ─── directly on P0's fixtures) ─────────────────────────────────────

    #[test]
    fn admissibility_missing_index_is_no_partition_with_zero_decodes() {
        // The shipped `44444` fixture (r5_classification_order...): two
        // 3-chunk cards, only chunk 0 of each supplied -- indices 1 and 2
        // have ZERO pieces. SPEC row 6.
        reset_decode_calls();
        let strings = [
            "mk1qpg3zyzqqsq4kj90x3eutks2lcztpqyqsqygpqyqsqygrqyqsqyg9qyqsqyqfz9jrcld706hn9svfgll7zvw5qnkxgea7nkj6jsf2avy9zwj",
            "mk1qpg3zyzqqsq4kj90x3eutks2lcztpqyqsqygpqyqsqyg9qyqsqyg9qyqsqyqfz9jrej0n5eghh0620cpg9jly68gp3qxjnv0ty9cpzm2edu5",
        ];
        match partition(&strings) {
            Outcome::NoPartition => {}
            other => panic!("expected NoPartition, got {other:?}"),
        }
        assert_eq!(
            DECODE_CALLS.load(Ordering::SeqCst),
            0,
            "admissibility failure must be decided with ZERO decode calls"
        );
    }

    #[test]
    fn group_cap_set_refuses_with_sigma_k_six() {
        let strings = mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-groupcap.txt"
        ));
        reset_decode_calls();
        match partition(&strings) {
            Outcome::CapExceeded { sigma_k } => assert_eq!(sigma_k, 6),
            other => panic!("expected CapExceeded{{6}}, got {other:?}"),
        }
        assert_eq!(
            DECODE_CALLS.load(Ordering::SeqCst),
            0,
            "the cap fires before any candidate is enumerated or decoded"
        );
    }

    #[test]
    fn incomplete_class_set_is_no_partition_the_whole_group_fails_closed() {
        // r1-C3's separating shape: a complete 2-chunk class + a 3-chunk
        // class missing index 2. Every class must seat or the whole group
        // fails -- even though the 2-chunk class alone is trivially fine.
        let strings = mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-incomplete.txt"
        ));
        match partition(&strings) {
            Outcome::NoPartition => {}
            other => panic!("expected NoPartition, got {other:?}"),
        }
    }

    #[test]
    fn floor_and_boundary_fixtures_match_p0s_measured_shapes() {
        // Cross-check against `p0_shapes.rs`'s own pinned shapes; the
        // ARITHMETIC bound itself is a COMPILE-TIME fact, checked below by
        // `BOUND_IS_WITHIN_THE_PLANS_ACCEPTANCE_WINDOW` rather than a
        // runtime assertion on two constants (clippy
        // `assertions_on_constants`).
        let floor = mk1_lines(include_str!("../../tests/fixtures/seating/v-ap-floor.txt"));
        assert_eq!(floor.len(), 33);
        let boundary = mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-boundary.txt"
        ));
        assert_eq!(boundary.len(), 36);
    }

    #[test]
    fn over_budget_synthetic_set_refuses_statically_with_zero_decodes() {
        // The chunker's own n=32, 5-card set (P0 item 4 / row 5): the
        // product saturates to u64::MAX, far past the bound.
        const CHUNK_SET_ID: u32 = 0x6_6666;
        const TOTAL_CHUNKS: u8 = 32;
        let owned: Vec<Vec<String>> = (0..5u8)
            .map(|card| super::super::synth::synth_card_strings(CHUNK_SET_ID, TOTAL_CHUNKS, card))
            .collect();
        let strings: Vec<&str> = owned.iter().flatten().map(String::as_str).collect();
        assert_eq!(strings.len(), 160);
        reset_decode_calls();
        match partition(&strings) {
            Outcome::OverBudget { product } => assert_eq!(product, u64::MAX),
            other => panic!("expected OverBudget, got {other:?}"),
        }
        assert_eq!(
            DECODE_CALLS.load(Ordering::SeqCst),
            0,
            "over-budget must refuse with ZERO decode calls -- no hang"
        );
    }

    // ─── §2.5 V = k on canonical / shared-piece / AP2 fixtures ─────────────

    #[test]
    fn canonical_pair_seats_two_cards_v_equals_k() {
        let strings = mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-canonical.txt"
        ));
        match partition(&strings) {
            Outcome::Seated(cards) => assert_eq!(cards.len(), 2),
            other => panic!("expected Seated(2), got {other:?}"),
        }
    }

    #[test]
    fn shared_piece_pair_seats_two_cards_via_reuse() {
        let strings = mk1_lines(include_str!("../../tests/fixtures/seating/v-ap-shared.txt"));
        match partition(&strings) {
            Outcome::Seated(cards) => assert_eq!(cards.len(), 2),
            other => panic!("expected Seated(2), got {other:?}"),
        }
    }

    #[test]
    fn ap2_fixture_is_ambiguous_v_greater_than_k() {
        let strings = mk1_lines(include_str!("../../tests/fixtures/seating/v-ap2.txt"));
        match partition(&strings) {
            Outcome::Ambiguous => {}
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn floor_set_seats_three_cards_within_budget() {
        let strings = mk1_lines(include_str!("../../tests/fixtures/seating/v-ap-floor.txt"));
        match partition(&strings) {
            Outcome::Seated(cards) => assert_eq!(cards.len(), 3),
            other => panic!("expected Seated(3), got {other:?}"),
        }
    }

    #[test]
    fn boundary_set_refuses_over_budget() {
        let strings = mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-boundary.txt"
        ));
        match partition(&strings) {
            Outcome::OverBudget { product } => assert_eq!(product, 531_441),
            other => panic!("expected OverBudget{{531441}}, got {other:?}"),
        }
    }

    // ─── whole-diff-seat-ap-review findings (I1/I2/I3/M1-M5) ───────────

    /// M3. `partition` is `pub(crate)` and ITS OWN unit tests (this module)
    /// call it directly on raw fixture text, unlike `decode_cards`, which
    /// never reaches an uncanonicalisable string here (every string it
    /// passes already round-tripped through `group_key_of`). A malformed
    /// line must refuse the WHOLE group, not silently shrink the piece set
    /// and seat on what remains -- the failure mode is fail-OPEN in a
    /// test-facing entry point.
    #[test]
    fn partition_refuses_rather_than_silently_drop_an_uncanonicalisable_string_m3() {
        let mut owned: Vec<String> = mk1_lines(include_str!(
            "../../tests/fixtures/seating/v-ap-canonical.txt"
        ))
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(
            owned.len(),
            4,
            "the normally-seating pair (row 1's own fixture)"
        );
        owned.push("mk1notavalidstring".to_string());
        let strings: Vec<&str> = owned.iter().map(String::as_str).collect();
        match partition(&strings) {
            Outcome::NoPartition => {}
            other => panic!(
                "an uncanonicalisable string must refuse the group, not silently drop it and \
                 seat on the remaining pieces (M3): got {other:?}"
            ),
        }
    }

    // M2 (report: "out-of-range chunk indices inflate k_class and the
    // budget product") is NOT applied here -- reproduction attempt found
    // it unreachable, not merely untested. See the fold report for the
    // full trace: `canonical_piece_key`/`group_key_of` both resolve a
    // chunked string through `mk_codec::string_layer::StringLayerHeader::
    // from_5bit_symbols`, which itself REFUSES `chunk_index >=
    // total_chunks` (`vendor/mk-codec/src/string_layer/header.rs:160-162`,
    // "chunked-header malformed: chunk_index = N >= total_chunks = M") --
    // confirmed empirically: `synth_string(id, 2, 9, ..)` (a hand-built
    // out-of-range header) makes `canonical_piece_key` return `Err`, not
    // an out-of-range `Piece`. So no by_index entry with chunk_index >=
    // total_chunks can ever be built, `partition`'s k_class/budget
    // computations can never see one, and `classify`'s own `out_of_range`
    // flag in `input.rs` is dead for the same reason -- no test in this
    // suite has ever driven it true either. No code change made.

    /// I1. SPEC §2.4 is `Sigma_classes Pi_indexes count_i` (the doc comment
    /// on [`PARTITION_DECODE_BOUND`] already said so); the shipped fold
    /// multiplied ACROSS classes too. One id, two total-classes: 2 cards x
    /// 4 chunks (class product 2^4=16) and 3 cards x 11 chunks (3^11=
    /// 177,147, the SAME shape as the row-4 floor fixture). Sigma_k=5 (AT
    /// the AP3 cap, not exceeding it). Sigma=177,163 <= BOUND(200,000) --
    /// SPEC row 7's "mixed-totals: both classes complete -> both seat".
    /// Pi=2,834,352 > BOUND -- the shipped code refuses. These numbers are
    /// the review's own measured PROBE (`spec_sum=177163`,
    /// `code_product=2834352`), reproduced here from real, decodable
    /// cards rather than argued.
    #[test]
    fn budget_sums_across_classes_not_products_i1_separating_row() {
        const ID: u32 = 0xB_2001;
        let (card_a1, str_a1) = real_card_at_exact_chunk_count(ID, 3, 1, 4);
        let (card_a2, str_a2) = real_card_at_exact_chunk_count(ID, 3, 2, 4);
        let (card_b1, str_b1) = real_card_at_exact_chunk_count(ID, 3, 3, 11);
        let (card_b2, str_b2) = real_card_at_exact_chunk_count(ID, 3, 4, 11);
        let (card_b3, str_b3) = real_card_at_exact_chunk_count(ID, 3, 5, 11);

        let mut owned: Vec<String> = Vec::new();
        for v in [str_a1, str_a2, str_b1, str_b2, str_b3] {
            owned.extend(v);
        }
        assert_eq!(owned.len(), 4 + 4 + 11 + 11 + 11, "2x4 + 3x11 = 41 strings");
        let strings: Vec<&str> = owned.iter().map(String::as_str).collect();

        match partition(&strings) {
            Outcome::Seated(cards) => {
                assert_eq!(cards.len(), 5, "both classes fully seat: 2 + 3 cards");
                for expected in [&card_a1, &card_a2, &card_b1, &card_b2, &card_b3] {
                    assert!(
                        cards.contains(expected),
                        "missing an expected card: {expected:?}"
                    );
                }
            }
            other => panic!(
                "SPEC row 7 (Sigma=177163 <= BOUND=200000): expected Seated(5), got {other:?} \
                 -- if this is OverBudget{{product}} with product near 2,834,352, the budget \
                 fold is still multiplying ACROSS classes instead of summing (I1)"
            ),
        }
    }

    /// I1, second row: the SUM must still correctly cap a genuinely
    /// over-budget multi-class group -- proving the fix does not turn the
    /// budget into a no-op for the multi-class case. Two classes at
    /// total_chunks=12 sharing one id: 2 cards (2^12=4,096) and 3 cards
    /// (3^12=531,441, the row-4 BOUNDARY fixture's own single-class
    /// product). Sigma_k=5 (at the cap, not exceeding it). Sigma=535,537
    /// > BOUND(200,000): OverBudget under the FIXED sum formula too.
    #[test]
    fn sum_still_refuses_a_genuinely_over_budget_two_class_group_i1() {
        const ID: u32 = 0xB_2002;
        const TOTAL_CHUNKS: u8 = 12;
        let class_a: Vec<Vec<String>> = (0..2u8)
            .map(|card| super::super::synth::synth_card_strings(ID, TOTAL_CHUNKS, card))
            .collect();
        // A second, ADDITIONAL total-class under the SAME id: reuse the
        // synthetic chunker at a DIFFERENT total_chunks so it forms its
        // own class (by_total groups by declared total, SPEC §2.1).
        let class_b: Vec<Vec<String>> = (0..3u8)
            .map(|card| super::super::synth::synth_card_strings(ID, TOTAL_CHUNKS + 1, card))
            .collect();
        let mut owned: Vec<String> = Vec::new();
        owned.extend(class_a.into_iter().flatten());
        owned.extend(class_b.into_iter().flatten());
        assert_eq!(owned.len(), 2 * 12 + 3 * 13);
        let strings: Vec<&str> = owned.iter().map(String::as_str).collect();
        reset_decode_calls();
        match partition(&strings) {
            Outcome::OverBudget { product } => {
                assert_eq!(product, 2u64.pow(12) + 3u64.pow(13));
            }
            other => panic!("expected OverBudget, got {other:?}"),
        }
        assert_eq!(
            DECODE_CALLS.load(Ordering::SeqCst),
            0,
            "over-budget must refuse with ZERO decode calls"
        );
    }

    /// Build ONE real, decodable card's mk1 strings at an EXACT
    /// `target_total_chunks` and a PINNED `chunk_set_id` (independent of
    /// the content-derived one) -- I1's separating fixture needs two
    /// DIFFERENT total-classes sharing one id, which no `mk encode`
    /// invocation can produce (its chunking is derived from bytecode
    /// length, never chosen). Mirrors `seat::synth`'s own real-card
    /// round-trip construction (mk-codec's public bytecode/string-layer
    /// API only, see `synth.rs`'s
    /// `real_card_round_trips_through_the_chunker`) with `frag_len`
    /// computed BACKWARDS from the desired chunk count instead of
    /// mk-codec's fixed 53-byte fragment size.
    fn real_card_at_exact_chunk_count(
        chunk_set_id: u32,
        n_stubs: u32,
        seed: u8,
        target_total_chunks: u8,
    ) -> (KeyCard, Vec<String>) {
        use bitcoin::NetworkKind;
        use bitcoin::bip32::{ChainCode, ChildNumber, DerivationPath, Fingerprint, Xpub};
        use bitcoin::hashes::{Hash, sha256};
        use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
        use std::str::FromStr;

        let path = DerivationPath::from_str("48'/0'/0'/2'").unwrap();
        let secp = Secp256k1::new();
        let mut sk_bytes = [0x11u8; 32];
        sk_bytes[31] = seed;
        let sk = SecretKey::from_slice(&sk_bytes).unwrap();
        let xpub = Xpub {
            network: NetworkKind::Main,
            depth: 4,
            parent_fingerprint: Fingerprint::from([seed; 4]),
            child_number: ChildNumber::Hardened { index: 2 },
            public_key: PublicKey::from_secret_key(&secp, &sk),
            chain_code: ChainCode::from([seed; 32]),
        };
        let card = KeyCard::new(
            (0..n_stubs).map(u32::to_be_bytes).collect(),
            Some(Fingerprint::from([seed; 4])),
            path,
            xpub,
        );
        let bytecode = mk_codec::bytecode::encode_bytecode(&card).expect("card must encode");
        let hash = sha256::Hash::hash(&bytecode);
        let mut stream = bytecode.clone();
        stream.extend_from_slice(&hash.to_byte_array()[..4]);

        let frag_len = stream.len().div_ceil(target_total_chunks as usize);
        let strings: Vec<String> = stream
            .chunks(frag_len)
            .enumerate()
            .map(|(i, frag)| {
                super::super::synth::synth_string(chunk_set_id, target_total_chunks, i as u8, frag)
            })
            .collect();
        assert_eq!(
            strings.len(),
            target_total_chunks as usize,
            "frag_len must hit the exact target chunk count"
        );
        (card, strings)
    }

    /// M5. `encode_bytecode`'s error is unreachable through the real
    /// pipeline -- every card here already round-tripped through
    /// `mk_codec::decode`, which enforces every `encode_bytecode`
    /// invariant (non-empty stubs, `depth == path length`, path <=
    /// `MAX_PATH_COMPONENTS`). This exercises the extracted
    /// `order_and_seat` directly on a hand-built invariant-violating card
    /// (empty `policy_id_stubs`, which `encode_bytecode` itself rejects)
    /// to prove the fold closed the "silently default the order key"
    /// shape, rather than asserting an unreachable production scenario.
    #[test]
    fn order_key_failure_refuses_rather_than_default_the_sort_key_m5() {
        use bitcoin::NetworkKind;
        use bitcoin::bip32::{ChainCode, ChildNumber, DerivationPath, Fingerprint, Xpub};
        use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
        use std::str::FromStr;

        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[0x11u8; 32]).unwrap();
        let xpub = Xpub {
            network: NetworkKind::Main,
            depth: 4,
            parent_fingerprint: Fingerprint::from([0xAA; 4]),
            child_number: ChildNumber::Hardened { index: 2 },
            public_key: PublicKey::from_secret_key(&secp, &sk),
            chain_code: ChainCode::from([0xAA; 32]),
        };
        let broken = KeyCard::new(
            Vec::new(), // empty stubs: `encode_bytecode`'s own rejected shape
            Some(Fingerprint::from([0xAA; 4])),
            DerivationPath::from_str("48'/0'/0'/2'").unwrap(),
            xpub,
        );
        assert!(
            mk_codec::bytecode::encode_bytecode(&broken).is_err(),
            "sanity: this hand-built card really does fail to re-encode"
        );
        match order_and_seat(vec![broken]) {
            Outcome::NoPartition => {}
            Outcome::Seated(_) => panic!(
                "an order-key encode failure must refuse the group, not silently default the \
                 sort key (M5) -- #<k> would become supply-order dependent"
            ),
            other => panic!("unexpected: {other:?}"),
        }
    }
}
