# SPEC — seat auto-partition: untangle same-id card collisions instead of refusing

**Status: DRAFT for R0 r2.** Grounded in `design/WALK_seat_auto_partition_2026-08-31.md`
(rulings AP1–AP3, operator verbatim); folded once against
`design/agent-reports/R0-seat-auto-partition-r1.md` (5C/7I/5M/2N — §3 was
structurally revised in response). Baselines: descriptor-mnemonic `24d3b613`
(+ this fold), mnemonic-key `1602e41`. Risk-set. No code before 0C/0I.

## The change, in one sentence

When the seat path's grouping step finds strings sharing one declared
chunk-set id that do not form one key card, it ATTEMPTS to partition their
CANONICAL pieces into multiple verified cards and seat them all — refusing
(with the shipped arm-1 diagnosis) when the partition is ambiguous,
incomplete, over-cap, or over-budget.

## Rulings (operator 2026-08-31, verbatim in the walk)

- **AP1** — on success: seat, plus ONE note whose cause clause names all
  three possible origins neutrally ("a mint defect, an attack, or a
  deliberate choice at encode time"), asserting none.
- **AP2** — genuinely ambiguous partitions: HARD REFUSAL, never a guess.
- **AP3** — at least 3 colliding cards supported; hard cap 5; the cap is
  performance-motivated, so the search itself is also budget-bounded (r1 C5).

## §1 Piece canonicalisation (precondition — r1 C1, measured)

mk1's string layer silently corrects up to t=4 symbols, and today's dedupe
compares raw strings, so two transcriptions of ONE piece with small slips
survive as two "different" pieces. Before grouping and partitioning, pieces
are canonicalised on DECODED content: two strings are the SAME piece iff
their `(chunk_set_id, total_chunks, chunk_index, decoded fragment bytes)`
are equal; duplicates collapse (extending the shipped case-fold dedupe to
BCH-corrected identity — enumerated churn to pipeline step 1). Consequence:
a benign double transcription of one card collapses to one card and SEATS
normally — it is NOT ambiguity, NOT a merged-cards case (vector row from the
r1 M-b construction: 4-flipped-char twins of both pieces → seats as ONE
card). "Uses every piece exactly once" below means canonical pieces.

## §2 The partition contract (over one id group of canonical pieces that
## fails single-card reassembly)

Indices are 0-based, matching the wire (r1 N1).

1. **Sub-group by declared total** (pieces declaring different totals can
   never share a card).
2. **Exact-count invariant (r1 I7, normative):** a complete partition of a
   total-class with total n requires EVERY index 0..n-1 to hold the same
   piece count k, and the class to hold exactly k·n pieces. Unequal
   per-index counts ⇒ no complete partition exists — decided in O(pieces)
   with ZERO decodes.
3. **Cap and budget (AP3 + r1 C5):** k > 5 in any class → cap refusal (the
   shipped arm-1 message plus one clause: "more than 5 cards share this
   stamped chunk-set id; auto-separation caps at 5"). Independently, the
   total number of candidate decode attempts across the group is bounded by
   `PARTITION_DECODE_BOUND` (value fixed at implementation from measurement;
   the `matching::MATCHING_BOUND = 720` precedent applies; the spec requires
   it ≤ 4096) — exceeding it → over-budget refusal naming the bound. Note
   n ≤ 32 (codec) and legitimate mints reach n = 12, so k^n is NOT bounded
   by the cap alone; the budget is the binding constraint (r1 M-d/M-e).
4. **Candidates and verification:** a candidate card is one canonical piece
   per index; it VERIFIES iff `mk_codec::decode` accepts it (the 4-byte
   cross-chunk hash is the oracle). A COMPLETE PARTITION of the id group is
   a set of verified candidates consuming every canonical piece in EVERY
   total-class exactly once (composition is fail-closed across classes —
   r1 C3: a class that cannot complete fails the whole group; no partial
   seating, no dropped pieces, ever).
5. **Partition identity (r1 C1/AP2 refinement):** two partitions are THE
   SAME iff they yield the same multiset of decoded cards. Ambiguity means
   more than one DISTINCT decoded-card-set — which, post-canonicalisation,
   is reachable only by constructed ~2^32-grind collisions, never by
   accident.

## §3 Outcomes (in order)

- **cap / budget exceeded** → refusals per §2.3.
- **no complete partition** (incl. unequal counts, any class incomplete) →
  **arm 1 is re-entered with its FULL shipped predicate set and message**
  (r1 C2); only a group whose arm-1 predicates are all false proceeds to
  arms 2/3. The shipped four-arm classifier stays total exactly as the
  csid spec's contract 7 froze it; auto-partition is a pre-pass, not a
  replacement.
- **exactly one distinct decoded-card-set** → SEAT all cards. They join the
  normal seating flow unchanged (slot checks, directives, matching,
  completeness). The AP1 note is a VALUE returned with the cards, carried
  into `Seating.notes` and emitted ONLY on a successful seating, ordered
  ahead of that group's R2 warnings (r1 I3 — never printed on a downstream
  refusal). Draft (glosses + shipped framing, r1 M3/N2):
  > `note: these 4 strings carried one stamped chunk-set id (chunk-set 12345) but are 2 different key cards — each key card's mk1 strings are its pieces (chunks), and each card's own 4-byte integrity check accepted its pieces, so they were separated. A shared stamped id can be a mint defect, an attack, or a deliberate choice at encode time — if it is unexpected, check each card alone with mk inspect.`
  ("accepted its pieces", not "confirms" — r1 M2.)
- **more than one distinct decoded-card-set** → HARD REFUSAL (AP2), draft:
  > `md: seating refused: chunk-set 12345: these pieces (chunks) separate into different key-card sets in more than one self-consistent way, and the tool will not guess which set is your wallet. This is not expected from accidental damage — treat the strings as untrusted and re-scan one card's pieces alone, from a source you trust.`
  ("source", not "plates" — r1 M1; the claim is true post-§1, where
  accidental BCH twins can no longer reach this arm.)

## §4 Identity of collided cards (r1 C4, I1, I2)

- **Deterministic content order:** cards produced by one partition are
  ordered by ascending canonical bytecode; this order is normative. The A3
  tie-break key is extended with it so distinct matchings remain
  discriminated and the emitted descriptor/WalletPolicyId is invariant
  under input permutation (r1 I1; a vector row pins permutation-invariance).
- **Ordinal identity:** each collided card is labelled `<id>#<k>` (k = 1-based
  position in the content order), e.g. `12345#1`, `12345#2` — used by
  `label()` and every downstream message that names cards, so leftover
  lists and pair-refusals stay distinguishable (r1 I2; the surplus row
  asserts the two leftover lines DIFFER).
- **`--seat` (r1 C4):** `@i=<id>#<k>` is accepted and resolves exactly one
  collided card. A bare `@i=<id>` naming an id carried by MORE than one
  seated card → a named ambiguity refusal pointing at the `#<k>` form
  (silent first-match is removed for collided ids). The three shipped
  "UNREACHABLE ambiguous-id" doc invariants (`directive.rs:23-26`,
  `input.rs:12-20`, `input.rs:489-492`) are rewritten — enumerated churn.

## §5 Interplay with the shipped R2 warning

Unchanged mechanics; after a successful partition each seated card
recomputes stamped-vs-derived as today. Expected composition for the
canonical case: one AP1 note, then two R2 warnings (same declared id,
different derived ids). The ordering (note first) is pinned by a row.

## Security considerations (updated per r1)

- Post-§1, ambiguity requires constructed collisions: ~2^32 grind against
  the victim's trailing 4-byte hash PLUS validity of the ground bytecode as
  a KeyCard — the AP2 fixture below demonstrates feasibility. Outcome of
  any successful grind: refusal (service degraded), never wallet selection.
- Piece-replacement (frankencard) surface unchanged from today (r1 sound
  list). Surplus-card injection: partitions cleanly, then completeness
  refuses downstream with DISTINGUISHABLE labels (§4).

## Acceptance (vector rows)

1. Canonical 2×2 pinned collision → seats; descriptor, address AND
   WalletPolicyId byte-identical to the CONTROL: the same key material
   minted UNPINNED (natural distinct ids), seated in one run (r1 I4). AP1
   note once + two R2 warnings, in that order.
2. BCH-twin row (r1 M-b construction): double transcription with ≤4 flips
   per piece → collapses, seats as ONE card, NO note, NO refusal.
3. 3-card collision → seats (AP3 floor). 6-card → cap refusal naming 5.
4. Over-budget synthetic (headers driving > PARTITION_DECODE_BOUND
   candidates) → budget refusal; no hang (r1 C5's 160-string input).
5. Unequal per-index counts (r1 I7) → zero-decode arm-1 refusal; the r1
   classification-order fixture (2× chunk-0 of 3-chunk cards) still gets
   the ARM-1 message, not "scan the missing piece(s)" (r1 C2).
6. Mixed totals both complete → both seat; one class incomplete → whole
   group refuses via arm 1, nothing seats, no pieces dropped (r1 C3).
7. AP2 ambiguity row: a COMMITTED fixture built by a committed grind
   script (regeneration documented; ~2^32 SHA-256 + KeyCard-validity —
   feasibility demonstrated by the script; a BCH-equivalent duplicate is
   NOT a valid fixture, it must seat per row 2) → AP2 refusal, exit
   nonzero, nothing seats (r1 I6).
8. Permutation-invariance row (§4); `--seat` rows: `#<k>` resolves, bare
   collided id refuses with the named ambiguity error.
9. Mutation gates: disable the partition search → row 1 refuses (fails);
   force-accept the first card-set when two distinct sets verify → row 7
   fails; skip canonicalisation → row 2 fails. Mutated-line-RAN evidence.
10. **Enumerated churn (r1 I5 — replaces "only the arm-1 entry row"):**
    `r5_merged_two_cards_pinned_to_one_id_classify_as_merged` REWRITTEN
    (its input now seats; its guarantee moves to row 1);
    `r5_classification_order_prefers_merged_over_incomplete` KEPT, asserts
    arm-1 wording via row 5; `v_collide_two_cards_pinned_…_refuse_at_
    reassembly` REWRITTEN (mixed totals now seat; guarantee moves to
    row 6); `v_collide_reaches_the_command` REWRITTEN (asserts the new
    seat+note outcome end-to-end). Doc invariants: `input.rs:12-20`,
    `input.rs:489-492`, `directive.rs:23-26` rewritten per §4; dedupe doc
    (`input.rs:106-121`) extended per §1. Every retired assertion's
    guarantee is named above; none silently disappears.
11. Suite green (`cargo nextest run --locked -p md-cli`), fmt + clippy
    clean; the shipped wording-pin test and arms 2–3 rows unchanged.

## Out of scope

mk-cli, me-cli, fork/device (their groups refuse as today); mk-codec /
md-codec; the R2 warning text; `GroupId::Single` groups and `total = 1`
chunked headers (unreachable from real mints — 73-byte compact-xpub floor
forces total ≥ 2; stated, not silently assumed — r1 M5); collisions across
different declared ids.
