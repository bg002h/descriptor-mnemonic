# SPEC — seat auto-partition: untangle same-id card collisions instead of refusing

**Status: DRAFT for R0 r4.** Grounded in `design/WALK_seat_auto_partition_2026-08-31.md`
(AP1–AP3, operator verbatim); folded against R0 rounds r1 (5C/7I/5M/2N),
r2 (0C/7I/6M/2N), r3 (0C/5I/6M/1N) — reports in `design/agent-reports/`.
Baselines: descriptor-mnemonic `9deebb47` (+ this fold), mnemonic-key
`1602e41`. Risk-set. No code before 0C/0I.

## The change, in one sentence

When the seat path's grouping step finds strings sharing one declared
chunk-set id that do not form one key card, it ATTEMPTS to explain their
CANONICAL pieces as a SET OF VERIFIED CARDS (pieces may be shared when the
format forces it) and seats that set — refusing, with the shipped arm-1
diagnosis or a named cap/budget/ambiguity refusal, in every other case.

## Rulings (operator 2026-08-31, verbatim in the walk)

- **AP1** — on success: seat, plus ONE note naming all three possible
  origins neutrally, asserting none.
- **AP2** — genuinely ambiguous partitions: HARD REFUSAL, never a guess.
- **AP3** — at least 3 colliding cards supported; hard cap 5 **on the
  whole id group** (r3-I5 — the cap is not per total-class). **Honoured
  within a computation budget, per the ruling's own "if it takes too
  long", with the honest measured boundary on its face (r3-I1, measured
  7.845 µs per candidate decode):** 3 cards guaranteed to n = 11 chunks
  (177,147 candidates ≈ 1.4 s), 5 cards to n = 7, 2 cards to n = 17;
  the r1-measured 128-stub mint (n = 12) with three distinct-stub cards
  is the first refusing size and is pinned as the boundary row. Mints up
  to n = 21 are possible (255-stub cap) and refuse by design (r3-N1).

## §1 Piece canonicalisation (mk1-only stage, AFTER the shipped step-1
## dedupe)

After the shipped whitespace/case dedupe (unchanged; it also serves md1),
mk1 strings are canonicalised: two strings are the SAME piece iff their
`(chunk_set_id, total_chunks, chunk_index, 5-bit payload symbol tail)`
are equal — the symbol tail, not re-derived bytes, so no new failure
route; a string failing `decode_string` is refused exactly as today.
First appearance survives. Consequence: a benign double transcription
(BCH twins, ≤ t=4 slips) collapses and seats as ONE card (row 2).

## §2 The partition contract — ONE evaluation order, over one id group of
## canonical pieces that fails single-card reassembly. Indices 0-based.

1. **Sub-group by declared total.**
2. **Per-class admissibility, zero decodes.** Any index with ZERO pieces
   ⇒ the class fails (→ §3 "no partition"). Else
   **k_class := the MAXIMUM per-index count of distinct canonical
   pieces**. Indexes below k_class are admissible — different cards can
   share a byte-identical piece (r2 NEW-I1, ≥13 shared stubs). **Why
   k_class is EXACT for honest input (r3-M4, measured):** the 73-byte
   compact xpub + 4-byte trailing hash span 2–3 chunks, so at least one
   index carries ≥ 25 bytes of key material where distinct keys cannot
   coincide; violating "max count = card count" costs a ~2^32 grind,
   which §2.5 turns into a refusal, never a wrong seat.
3. **Cap (AP3), group-wide:** `Σ_classes k_class > 5` ⇒ cap refusal,
   claiming only what was measured: "these pieces (chunks) would need
   more than 5 key cards to explain; auto-separation caps at 5 —
   re-scan one card's pieces alone."
4. **Budget, STATIC and pre-computed:** refuse when
   `Σ_classes Π_indexes count_i > PARTITION_DECODE_BOUND` — a function
   of the CANONICAL PIECE COUNTS (not raw strings, not headers alone —
   r3-M3), evaluated before any decode, **saturating** arithmetic (5^32
   overflows u64; unchecked pow panics under the test profile).
   Implementations may decode fewer candidates via filters (e.g. length
   prefilters), but the over-budget OUTCOME is decided by this product
   alone. `PARTITION_DECODE_BOUND` is fixed at implementation from
   measured timing to keep the worst case under ~2 s with the literal
   `mk_codec::decode` oracle — ≈ 255,000 at the measured 7.845 µs
   (r3-I1 option (b): the floor of 531,441 is RETRACTED as unsatisfiable
   with this oracle; AP3's face above states what the honest constant
   reaches). The refusal names the boundary and AP3's rationale.
5. **Enumerate, verify, and the SEAT CONDITION (r3-I3 — the cover IS the
   verified set):** a candidate card is one canonical piece per index
   (reuse permitted); it VERIFIES iff `mk_codec::decode` accepts it. Let
   **V_class := the set of DISTINCT verified candidate cards** (identity
   = decoded card). The class SEATS iff `|V_class| = k_class` AND
   V_class's pieces cover every canonical piece of the class. Composed
   fail-closed across classes (r1 C3): every class must seat or the
   whole group fails. There is no chosen cover and no subset search
   (r3-M5 deleted): honest input yields exactly the real cards (measured
   5/5 constructions incl. both shared-piece rows); a ground EXTRA
   verified card makes `|V| > k` ⇒ refusal (never a silent drop, never a
   wrong seat); a dominated card cannot be omitted because V is all of
   them.
6. **Ambiguity / failure split:** `|V_class| > k_class` in any class ⇒
   **AP2 hard refusal** (a constructed state — §Security). Any class
   with `|V_class| < k_class` or an uncovered piece ⇒ **no partition**
   (→ §3 arm-1 entry).

## §3 Outcomes (the §2 order IS the outcome order; first refusal wins)

- **Every class seats** → SEAT all cards; unchanged seating flow. The
  AP1 note is a VALUE in `Seating.notes`, emitted ONLY on a successful
  seating, ahead of the group's R2 warnings. Draft (counts state both
  measures — supplied strings and distinct pieces):
  > `note: these 5 supplied strings are 4 distinct pieces (chunks) carrying one stamped chunk-set id (chunk-set 12345), and they are 2 different key cards — each card's own 4-byte integrity check accepted its pieces, so they were separated. A shared stamped id can be a mint defect, an attack, or a deliberate choice at encode time — if it is unexpected, check each card alone with mk inspect.`
- **AP2 (per §2.6)** →
  > `md: seating refused: chunk-set 12345: these pieces (chunks) verify as more key cards than they can belong to, and the tool will not guess which cards are your wallet. This is not expected from accidental damage — treat the strings as untrusted and re-scan one card's pieces alone, from a source you trust.`
- **No partition** → **arm 1 is entered with its FULL shipped predicate
  set and message**; only a group whose arm-1 predicates are all false
  proceeds to arms 2/3. The shipped classifier stays total per csid
  contract 7.

## §4 Identity of collided cards

- **Order key:** ascending `mk_codec::bytecode::encode_bytecode(&card)`
  (named function; well-defined on decoded cards). Normative; extends
  the A3 tie-break key; emitted descriptor/WalletPolicyId invariant
  under input permutation (row 8).
- **Ordinal identity:** collided cards are labelled `<id>#<k>` (1-based
  in that order) in `label()` and every card-naming message. Stability:
  stable for identical input, unaffected by other groups; when a
  collision dissolves the label reverts to the bare id and a stale
  `#<k>` gets the no-such-card refusal listing current labels.
- **`--seat` grammar, every refusal spelled out:** five-digit/
  never-a-prefix binds the id half unchanged. Bare `@i=<id>` with >1
  seated carrier → named ambiguity refusal pointing at `#<k>` and
  listing labels. `@i=<id>#<k>` on a non-collided id → refusal ("not
  part of a collision; use `<id>`"); k = 0 / out of range / `#` without
  digits → refusal naming the valid range.

## §5 Interplay with the shipped R2 warning

Unchanged mechanics; canonical composition (row 1): AP1 note, then the
group's R2 warnings, in that order.

## Security considerations (re-derived against §2.5 — r3-I4)

- **Reaching the AP2 refusal costs ONE ~2^32 grind** (an extra verified
  candidate constrained to valid KeyCard bytecode; r3's [2,3,3]
  construction). Outcome: refusal — service degraded, never selection.
- **Seating a WRONG set is structurally impossible under §2.5:** real
  cards always verify, so no verified real card can be displaced; an
  extra verified card can only raise `|V|` above k, which refuses.
- Piece-replacement (frankencard) surface unchanged from today. Surplus
  cards, from pieces the operator ALREADY supplied (no new piece
  injected, so k is unchanged): a GROUND same-id extra verified candidate
  hits `|V| > k` ⇒ AP2 refusal (stricter and earlier than a completeness
  refusal). An attacker who instead INJECTS a wholly new, self-consistent
  piece under the victim's id raises k TOGETHER WITH `|V|`, so the class
  can legitimately SEAT (`|V| == k`) — the injected card then reaches the
  ordinary satisfy/complete machinery exactly like the pre-existing
  different-id surplus path (row 10c); this is not a new attack class,
  only the AP1 ruling's already-accepted trade (a clean same-id collision
  is, by design, indistinguishable from an injected one). LEGITIMATE
  extra cards — same-id across total-classes, or different-id — seat and
  then hit the downstream completeness/leftover refusal with
  distinguishable labels, exactly the surplus row's variants (b)/(c).

## Acceptance (vector rows; cited by name elsewhere — r3-M1)

1. **canonical-collision row:** 2×2 pinned → seats; descriptor, address
   AND WalletPolicyId byte-identical to the unpinned-twin CONTROL. AP1
   note once, then two R2 warnings, in order.
2. **bch-twin row:** double transcription (≤4 flips/piece) → collapses,
   seats as ONE card, silent.
3. **shared-piece row:** two cards, ≥13 SHARED stubs (identical chunk 0)
   → seats as two cards via reuse; note distinguishes supplied strings
   from distinct pieces.
4. **floor row:** 3 cards, n = 11, **DISTINCT stub lists** (r3-I2 — a
   shared list collapses the product) → seats within budget (177,147
   candidates ≈ 1.4 s measured). **boundary row:** the same shape at
   n = 12 (mintable, N = 128 stubs) → budget refusal naming AP3's
   rationale — the first refusing size, pinned as designed behaviour.
5. **over-budget row:** the r1 extreme (~160 strings declaring
   total_chunks = 32, equal per-index counts k = 5, distinct stub
   lists, committed synthetic-chunker helper — mk encode cannot mint
   n = 32) → static refusal, ZERO decodes, no hang. This size is chosen
   so row 11's skip-the-budget mutation observably hangs (5^32-scale),
   not pauses (r3-M6).
6. **missing-index row:** the shipped `44444` fixture → zero-decode
   arm-1 message, never "scan the missing piece(s)".
7. **mixed-totals rows:** both classes complete → both seat (group cap
   applies across classes: a 3+3 two-class group → cap refusal, r3-I5);
   one class incomplete → whole group refuses via arm 1, nothing seats.
8. **permutation + `--seat` rows:** ordinal invariance; `#<k>` resolves;
   bare collided id / non-collided `#<k>` / k=0 / out-of-range → their
   named refusals.
9. **ap2 row:** COMMITTED fixture from a committed ONE-grind script
   (~2^32 + KeyCard-validity; regeneration documented; a BCH twin is
   NOT a valid fixture — it must seat per the bch-twin row) → AP2
   refusal, nothing seats.
10. **surplus rows, three variants (r4-I1):**
    (a) same-id GROUND extra verified candidate in one class →
    `|V| > k` ⇒ AP2 refusal (per §Security);
    (b) same-id LEGITIMATE extra cards that seat — the shipped
    `v-collide.txt` fixture exactly: both cards pinned 12345, one
    2-chunk and one 3-chunk, so BOTH total-classes seat via §2 and the
    template's completeness then refuses downstream with
    DISTINGUISHABLE `12345#1`/`12345#2` leftover labels — this variant
    inherits the r1-I2 guarantee;
    (c) different-id extra card → today's leftover path, unchanged.
11. **mutation gates:** disable the partition attempt → canonical row
    fails; force-seat when `|V| > k` → ap2 row fails; skip
    canonicalisation → bch-twin row fails; skip the static budget →
    over-budget row hangs/fails (observable per row 5's sizing).
    Mutated-line-RAN evidence for each.
12. **enumerated churn:** `r5_merged_two_cards_pinned_to_one_id_
    classify_as_merged` REWRITTEN (→ canonical row);
    `r5_classification_order_prefers_merged_over_incomplete` KEPT
    (missing-index row); `v_collide_two_cards_pinned_…` REWRITTEN
    (mixed-totals row); `v_collide_reaches_the_command` REWRITTEN with a
    new minimal 2-slot fixture for the seat+note outcome (its shipped
    input carries a full extra card set and both collided cards seat →
    it becomes the surplus row's variant (b), the same-id
    seats-then-leftover case with distinguishable labels). Plus:
    `REMEDIES` (`matching.rs:216-221`) gains `#<k>`; `directive::parse`
    per §4; doc invariants `input.rs:12-20`, `input.rs:490-493`,
    `directive.rs:23-26` rewritten; the §1 stage documented adjacent to
    `dedupe_strings`; **`decode_cards` signature change (note-as-value):
    1 production call site (`seat/mod.rs:143`) + 22 test call sites
    (input.rs ×13, complete.rs ×3, matching.rs ×3, disposition.rs ×2,
    satisfy.rs ×1 — measured, r3-M2)**. Every retired assertion's
    inheriting row is named.
13. Suite green (`cargo nextest run --locked -p md-cli`), fmt + clippy
    clean; shipped wording-pin test and arms 2–3 rows unchanged.

## Out of scope

mk-cli, me-cli, fork/device; mk-codec/md-codec (incl. the two-stage
oracle idea — rejected: it needs a mk-codec change or a local
reimplementation of the trailing-hash rule; the literal-oracle budget
above is the accepted cost); the R2 warning text; `GroupId::Single` and
`total = 1` headers (unreachable, 73-byte floor); collisions across
different declared ids.
