# SPEC — seat auto-partition: untangle same-id card collisions instead of refusing

**Status: DRAFT for R0 r3.** Grounded in `design/WALK_seat_auto_partition_2026-08-31.md`
(AP1–AP3, operator verbatim); folded against
`design/agent-reports/R0-seat-auto-partition-r1.md` (5C/7I/5M/2N, §3
structurally revised) and `…-r2.md` (0C/7I/6M/2N — partition semantics
gained shared pieces; the budget became static). Baselines:
descriptor-mnemonic `55d28f99` (+ this fold), mnemonic-key `1602e41`.
Risk-set. No code before 0C/0I.

## The change, in one sentence

When the seat path's grouping step finds strings sharing one declared
chunk-set id that do not form one key card, it ATTEMPTS to partition their
CANONICAL pieces into multiple verified cards (pieces may be SHARED by
cards when the format forces it) and seat them all — refusing, with the
shipped arm-1 diagnosis or a named cap/budget/ambiguity refusal, otherwise.

## Rulings (operator 2026-08-31, verbatim in the walk)

- **AP1** — on success: seat, plus ONE note naming all three possible
  origins neutrally ("a mint defect, an attack, or a deliberate choice at
  encode time"), asserting none.
- **AP2** — genuinely ambiguous partitions: HARD REFUSAL, never a guess.
- **AP3** — at least 3 colliding cards supported; hard cap 5. **Honoured
  within a computation budget, per the ruling's own "if it takes too
  long":** guaranteed for every measured legitimately-mintable size
  (3 cards to n = 12 chunks — the 128-stub mint measured in r1); beyond
  the budget the tool refuses BY DESIGN, naming the boundary (r2 NEW-I3,
  stated here on the ruling's face, not discovered in a constant).

## §1 Piece canonicalisation (mk1-only stage, AFTER the shipped step-1
## dedupe — r1 C1, r2 NEW-M3/M4)

mk1's string layer silently corrects up to t=4 symbols; raw-string dedupe
cannot see that. After the shipped whitespace/case dedupe (which also
serves md1 and is unchanged), mk1 strings are canonicalised: two strings
are the SAME piece iff their `(chunk_set_id, total_chunks, chunk_index,
5-bit payload symbol tail)` are equal — the symbol tail, NOT re-derived
bytes, so canonicalisation introduces no new failure route (r2 NEW-M4); a
string that fails `decode_string` outright is refused exactly as today.
First appearance survives a collapse (the shipped rule). Consequence: a
benign double transcription collapses and the group seats as ONE card —
never ambiguity, never merged-cards (row 2, from the r1 M-b measured
construction).

## §2 The partition contract — ONE evaluation order (r2 NEW-I4), over one
## id group of canonical pieces that fails single-card reassembly

Indices 0-based (wire convention).

1. **Sub-group by declared total.** Pieces declaring different totals
   never share a card.
2. **Per-class admissibility, zero decodes.** In a class with total n:
   any index 0..n-1 holding ZERO pieces ⇒ no candidate card can exist ⇒
   the class fails (→ §3 "no partition"). Otherwise define
   **k := the MAXIMUM per-index count of distinct canonical pieces**.
   Indexes holding fewer than k pieces are ADMISSIBLE: two different
   cards can legitimately share a byte-identical piece (r2 NEW-I1,
   measured — ≥13 shared policy stubs make chunk 0 a pure function of the
   stub list), so a candidate may REUSE a piece; k is the number of cards
   a cover must contain, and any index's distinct-piece count is a
   measured LOWER bound on cards.
3. **Cap (AP3), on the now-well-defined k:** k > 5 in a class ⇒ cap
   refusal, wording claiming only what was measured (r2 NEW-I4): "these
   pieces (chunks) would need more than 5 key cards to explain;
   auto-separation caps at 5 — re-scan one card's pieces alone."
4. **Budget, STATIC and pre-computed (r2 NEW-I2/I3):** refuse when
   `Σ_classes Π_indexes count_i > PARTITION_DECODE_BOUND`, evaluated
   BEFORE any decode with **saturating** arithmetic (5^32 overflows u64;
   an unchecked pow panics under this repo's test profile). The refusal
   is a function of the headers alone — implementations may decode fewer
   candidates via filters, but the over-budget outcome is decided by this
   product. `PARTITION_DECODE_BOUND` is fixed at implementation from
   measured decode timing, with spec floors/ceilings: **≥ 531,441**
   (3 cards at n = 12, AP3's floor at the largest measured legitimate
   mint) and small enough to keep the worst case under ~2 s. The refusal
   names the boundary and its AP3 rationale.
5. **Enumerate and verify.** A candidate card is one canonical piece per
   index (reuse permitted); it VERIFIES iff `mk_codec::decode` accepts it
   (the 4-byte cross-chunk hash is the oracle). A COMPLETE PARTITION of
   the id group is, per class, a set of exactly k distinct verified cards
   whose pieces COVER every canonical piece of the class (each piece used
   by ≥ 1 card), composed fail-closed across classes (r1 C3): every class
   must complete or the whole group fails — no partial seating, no
   dropped pieces, ever.
6. **Partition identity:** two partitions are THE SAME iff they yield the
   same multiset of decoded cards. Ambiguity = more than one DISTINCT
   decoded-card multiset — post-§1 reachable only by constructed
   collisions (r2 confirmed: every accidental route lands elsewhere).

## §3 Outcomes (the §2 order IS the outcome order — evaluated 1→6, first
## refusal wins; then:)

- **exactly one distinct card multiset** → SEAT all cards; they join the
  unchanged seating flow. The AP1 note is a VALUE carried into
  `Seating.notes`, emitted ONLY on a successful seating, ordered ahead of
  the group's R2 warnings (r1 I3). Draft (counts say what was measured —
  r2 NEW-M6):
  > `note: these 5 supplied strings are 4 distinct pieces (chunks) carrying one stamped chunk-set id (chunk-set 12345), and they are 2 different key cards — each card's own 4-byte integrity check accepted its pieces, so they were separated. A shared stamped id can be a mint defect, an attack, or a deliberate choice at encode time — if it is unexpected, check each card alone with mk inspect.`
- **more than one distinct card multiset** → HARD REFUSAL (AP2):
  > `md: seating refused: chunk-set 12345: these pieces (chunks) separate into different key-card sets in more than one self-consistent way, and the tool will not guess which set is your wallet. This is not expected from accidental damage — treat the strings as untrusted and re-scan one card's pieces alone, from a source you trust.`
- **no complete partition** (missing index, failed cover, failed class)
  → **arm 1 is entered with its FULL shipped predicate set and message**
  (r1 C2; "entered", not re-entered — the pre-pass runs first, r2
  NEW-N2); only a group whose arm-1 predicates are all false proceeds to
  arms 2/3. The shipped classifier stays total per csid contract 7.

## §4 Identity of collided cards (r1 C4/I1/I2; r2 NEW-M2/M5/N1)

- **Order key, named function:** ascending
  `mk_codec::bytecode::encode_bytecode(&card)` — the canonical re-encode
  already used as the R2 operand; well-defined on decoded cards even when
  on-wire bytecodes were non-canonical (r2 NEW-M5). Normative; the A3
  tie-break key is extended with it; emitted descriptor/WalletPolicyId is
  invariant under input permutation (row 8).
- **Ordinal identity:** collided cards are labelled `<id>#<k>` (1-based in
  the order above) in `label()` and every message that names cards —
  leftover lists and pair-refusals stay distinguishable (r1 I2).
  **Stability (r2 NEW-N1):** stable for identical input and unaffected by
  other groups; when a collision dissolves (one card left), the label
  reverts to the bare id and a stale `#<k>` directive stops resolving —
  it gets the no-such-card refusal, which lists current labels.
- **`--seat` grammar (r2 NEW-M2), every refusal spelled out:** the
  five-digit/never-a-prefix rule binds the id half unchanged. `@i=<id>`
  with >1 seated card carrying the id → named ambiguity refusal pointing
  at the `#<k>` form and listing the labels (silent first-match removed
  for collided ids). `@i=<id>#<k>`: on a NON-collided id → refusal
  ("card `<id>` is not part of a collision; use `<id>`"); k = 0, k out of
  range, or `#` without digits → refusal naming the valid range;
  otherwise resolves exactly one card.

## §5 Interplay with the shipped R2 warning

Unchanged mechanics; canonical composition (row 1): one AP1 note, then
the group's R2 warnings (same declared id, different derived ids), in
that order.

## Security considerations

- Post-§1, ambiguity requires construction: **two sequential ~2^32
  grinds** (r2's corrected derivation — one against the victim's trailing
  hash, one for the cross-prefix collision), each constrained to valid
  KeyCard bytecode. Feasible offline; outcome is refusal (service
  degraded), never wallet selection.
- Piece-replacement surface unchanged from today (r1). Surplus valid
  card: partitions cleanly, completeness refuses downstream with
  DISTINGUISHABLE `#<k>` labels (row 12).

## Acceptance (vector rows)

1. Canonical 2×2 pinned collision → seats; descriptor, address AND
   WalletPolicyId byte-identical to the CONTROL (same key material minted
   unpinned, natural ids, one run). AP1 note once, then two R2 warnings.
2. BCH-twin double transcription (≤4 flips/piece) → collapses, seats as
   ONE card, no note, no refusal.
3. **Shared-piece collision (r2 NEW-I1's measured construction: two
   cards, ≥13 shared stubs, identical chunk 0) → SEATS as two cards via
   piece reuse**; the note's counts distinguish supplied strings from
   distinct pieces.
4. 3-card collision at n = 12 (AP3 floor at the measured legitimate
   maximum) → seats within budget. 6-card → cap refusal (the "would
   need more than 5 key cards" wording). **Boundary row at the first n
   where the budget refuses** (pinning NEW-I3's edge as designed
   behaviour, message naming AP3's rationale).
5. Over-budget synthetic: equal per-index counts k=5 across an n with
   `5^n > PARTITION_DECODE_BOUND`, fixture built by a committed
   synthetic-chunker helper (mk encode cannot mint it; distinct stub
   lists so leading chunks do NOT collapse — r2 NEW-M1) → static budget
   refusal, no decode, no hang.
6. Missing-index class (the shipped `44444` classification-order fixture)
   → zero-decode arm-1 message, never "scan the missing piece(s)".
7. Mixed totals both complete → both seat; one class incomplete → whole
   group refuses via arm 1, nothing seats, no pieces dropped.
8. Permutation-invariance; `--seat` rows: `#<k>` resolves; bare collided
   id → ambiguity refusal listing labels; `#<k>` on non-collided id,
   k=0, k out-of-range → their named refusals (r2 NEW-M2).
9. AP2 ambiguity: COMMITTED fixture from a committed two-stage-grind
   script (r2 I6's corrected construction + regeneration doc; a BCH twin
   is NOT a valid fixture — it must seat per row 2) → AP2 refusal,
   nothing seats.
10. **Surplus/leftover row (r1 I2, restored):** injected extra valid card
    → partition succeeds, completeness refuses downstream, and the two
    leftover lines are DISTINGUISHABLE (`#<k>` labels differ).
11. Mutation gates: disable the partition search → row 1 fails;
    force-accept the first card multiset when two verify → row 9 fails;
    skip canonicalisation → row 2 fails; skip the static budget → row 5
    hangs/fails. Mutated-line-RAN evidence for each.
12. **Enumerated churn (r1 I5, corrected per r2):**
    `r5_merged_two_cards_pinned_to_one_id_classify_as_merged` REWRITTEN
    (guarantee → row 1); `r5_classification_order_prefers_merged_over_
    incomplete` KEPT (row 6); `v_collide_two_cards_pinned_…` REWRITTEN
    (mixed totals seat; guarantee → row 7);
    `v_collide_reaches_the_command` REWRITTEN **with a new minimal
    2-slot fixture** (its shipped input carries a full extra card set and
    would hit the leftover refusal, not seat — r2 I5; the old input
    becomes row 10's end-to-end variant). Plus: `REMEDIES`
    (`matching.rs:216-221`) gains the `#<k>` form; `directive::parse`
    churn per §4; doc invariants `input.rs:12-20`, `input.rs:490-493`
    (cite corrected — r2 NEW-N2), `directive.rs:23-26` rewritten; the §1
    stage documented adjacent to (not inside) `dedupe_strings`
    (r2 NEW-M3). Every retired assertion's inheriting row is named.
13. Suite green (`cargo nextest run --locked -p md-cli`), fmt + clippy
    clean; the shipped wording-pin test and arms 2–3 rows unchanged.

## Out of scope

mk-cli, me-cli, fork/device; mk-codec/md-codec; the R2 warning text;
`GroupId::Single` and `total = 1` chunked headers (unreachable from real
mints, 73-byte floor); collisions across different declared ids.
