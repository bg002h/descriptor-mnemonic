# SPEC — seat auto-partition: untangle same-id card collisions instead of refusing

**Status: DRAFT for R0.** Grounded in `design/WALK_seat_auto_partition_2026-08-31.md`
(rulings AP1–AP3, operator verbatim) and chunk_set_id-cycle walk W15.
Baselines: descriptor-mnemonic `43f35170`, mnemonic-key `1602e41` (both
on origin). Risk-set: changes the seat pipeline's normative refusal
behaviour. No code before 0C/0I.

## The change, in one sentence

When the seat path's grouping step finds strings sharing one declared
chunk-set id that do not form one key card, it now ATTEMPTS to partition
them into multiple verified cards and seat them all — refusing only when
the partition is ambiguous, impossible, or over the size cap.

## Rulings this spec implements (operator 2026-08-31, verbatim in the walk)

- **AP1** — on success: seat, plus ONE stderr note whose cause clause
  names all three possible origins of a shared stamped id — "a mint
  defect, an attack, or a deliberate choice at encode time" — asserting
  none. Per-card R2 mismatch warnings still fire independently where a
  card's stamped id ≠ derived id (shipped behaviour, unchanged).
- **AP2** — if MORE than one complete partition verifies: HARD REFUSAL,
  never a guess.
- **AP3** — support at least 3 colliding cards per id group; hard cap
  **5**. Above the cap: today's merged-cards refusal, plus one clause
  naming the cap. Timing measured at implementation (expected trivial:
  worst case ≤ 5 pieces per index over ≤ 5-piece cards → ≤ a few
  hundred decode attempts, microseconds each — measure, don't assume).

## Normative behaviour (replaces classifier arm 1's direct refusal;
## arms 2–4 of the shipped four-arm classifier are unchanged)

Input: one id group whose strings do not reassemble as a single card.

1. **Sub-group by declared total.** Pieces declaring different
   `total_chunks` can never share a card; each total-class partitions
   independently. (This makes the old "mixed totals" evidence a
   separable case, not a refusal.)
2. **Within a total-class of total n:** a CANDIDATE CARD is one piece
   per index 1..n. A candidate VERIFIES iff `mk_codec::decode` accepts
   it (the 4-byte cross-chunk hash is the oracle; no new crypto). A
   COMPLETE PARTITION is a set of verified candidates using every piece
   exactly once.
3. **Outcomes, in order:**
   - **cap:** more than 5 cards implied in any total-class (piece count
     per index > 5) → refusal: the shipped merged-cards message plus
     "more than 5 cards share this stamped id; auto-separation caps at
     5 — re-scan one card's pieces alone." (AP3)
   - **exactly one complete partition** → SEAT all resulting cards
     (they join the normal seating flow: completeness, satisfies(),
     composition — none of that changes) and emit the AP1 note, once
     per id group, drafted:
     > `note: these 4 strings carried one stamped chunk-set id (12345) but are 2 different key cards; each card's own integrity check confirms its pieces, so they were separated and seated individually. A shared stamped id can be a mint defect, an attack, or a deliberate choice at encode time — if it is unexpected, check each card alone with mk inspect.`
     Counts/ids substituted per case; `{:05x}` rendering; cards never
     plates; glosses at first use (the shipped W-rules bind).
   - **more than one complete partition** → refusal (AP2), drafted:
     > `md: seating refused: chunk-set 12345: these pieces separate into different card sets in more than one self-consistent way, and the tool will not guess which pairing is your wallet. This does not happen by accident — treat the strings as untrusted. Re-scan one card's pieces alone, from plates you trust.`
   - **no complete partition** → fall through to the shipped arms
     (incomplete scan / terminal otherwise) exactly as today.
4. **Interplay with the R2 mismatch warning (shipped):** after a
   successful partition, each seated card recomputes stamped-vs-derived
   as it already does; colliding cards are usually pinned, so expect
   the note PLUS one warning per mismatched card. A vector row pins
   this composition (note + 2 warnings for the walk's canonical case).

## Security considerations (for R0 to attack, stated honestly)

- The cross-chunk hash is 4 bytes. An attacker able to inject strings
  can grind (~2^32 offline) a piece that verifies against a victim's
  piece, manufacturing a second complete partition → AP2 refuses; the
  attack degrades service, never selects a wallet.
- An attacker who REPLACES a victim piece (their ground piece is the
  only counterpart present) gets a verifying frankencard — but that is
  exactly as true TODAY for a single-card scan; auto-partition does not
  enlarge that surface. (The R2 warning may fire on it; the address
  check the tool already prints is the operator's end control.)
- A surplus valid card (attacker adds a whole card pinned to the
  victim's id): partitions cleanly into N+1 cards; seating's
  completeness rule ("every supplied card must be seated") then
  refuses downstream. Unchanged surface; a vector row pins it.

## Acceptance (vector rows; executable, per the vectors-first rule)

- The walk's canonical case: 2×2-piece pinned cards, one id → seats,
  composed descriptor and address BYTE-IDENTICAL to scanning the two
  cards separately; AP1 note fires once; R2 warning fires twice.
- 3-card collision (AP3 floor) → seats, same identity property.
- 6-card collision → cap refusal naming 5.
- Ambiguity row: a constructed second verifying partition → AP2
  refusal; message contains "will not guess"; exit nonzero; nothing
  seats.
- No-partition row (mixed halves, the shipped terminal case) → still
  the terminal message; classification-order row proves auto-partition
  runs BEFORE the old arm-1 refusal and falls through cleanly.
- Mixed totals row: a 2-piece and a 3-piece card sharing one id →
  both seat via total-class separation, note fires.
- Surplus-card row: partition succeeds, seating completeness refuses
  downstream (unchanged message).
- Mutation gates: disable the partition search → canonical row fails
  (refuses instead of seating); force-accept the first partition when
  two verify → ambiguity row fails. Mutated-line-RAN evidence both.
- Whole suite green (`cargo nextest run --locked -p md-cli`);
  `cargo fmt --check` + clippy clean; the shipped csid tests
  (warning wording pin, four-arm rows) unchanged except the arm-1
  entry row, which now asserts the fall-through order.

## Not in scope

- mk-cli, me-cli, the fork/device: this is the converter (md-cli seat)
  leg only. Their groups refuse exactly as today.
- Any change to mk-codec/md-codec, the R2 warning text, or admission.
- Collisions across DIFFERENT declared ids (not a collision).
