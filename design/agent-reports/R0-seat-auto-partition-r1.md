# R0 — `design/SPEC_seat_auto_partition.md` @ `24d3b613`, round 1

**Question asked:** is the spec sound, complete and safe as a behavior
contract, and can every acceptance gate actually fail?

**VERDICT: 5 Critical / 7 Important / 5 Minor / 2 Nit — NOT GREEN.**

Reviewed against descriptor-mnemonic `24d3b613` (spec) over origin/main
`54ab1cd6` (code), mnemonic-key `1602e41` (csid spec — pin verified to
exist), and vendored `mk-codec 0.5.0`.

Settled inputs honoured, not re-litigated: AP1/AP2/AP3 as ruled;
warnings-not-refusals for csid mismatch; shipped arms 2–4;
cards-never-plates / human-first / `{:05x}`; the walk's measured 2-candidate
pairing.

## What was MEASURED for this review (not inferred)

| # | Measurement | Command / source | Result |
| --- | --- | --- | --- |
| M-a | mk1's string layer is **t = 4 error-CORRECTING**, and corrections are silent | `vendor/mk-codec/src/string_layer/bch_decode.rs:22,70` ("Runs in `O(t²)` for `t = 4`", "satisfying the BCH bound and giving `t = 4` correction"), step 5 "Apply corrections" | confirmed |
| M-b | A card string with **4 flipped bech32 characters decodes to the identical card**, and all four cross-pairings of {A,A'}×{B,B'} succeed | `mk decode` (mk 0.13.0) on the shipped `csid_warning_…` clean-twin card + 4-char mutants | 4/4 pairings → identical `xpub6FQya7zGhR92…HtF8mX`; the 4 strings are pairwise distinct bytes |
| M-c | `MAX_CHUNKS = 32`; `total = ceil(len/53)`; the cross-chunk hash is `SHA-256(bytecode)[0..4]` stored as the **trailing 4 bytes of the last chunk** | `vendor/mk-codec/src/consts.rs:42`, `string_layer/chunk.rs:4,161-163,278-289` | confirmed |
| M-d | A **legitimately mintable** key card reaches **12 chunks** — `--policy-id-stub` is repeatable | `mk encode --xpub … --policy-id-stub <8 hex> ×N`: N=1→2, N=8→3, N=32→4, N=64→7, N=128→12 chunks | confirmed |
| M-e | Candidate-card enumeration is `k^n` | arithmetic | k=5,n=5 → **3,125**; k=5,n=12 → **244,140,625**; k=5,n=32 → 2.3×10²² |
| M-f | Both `v-collide.txt` cards carry stub `5b48af35` **and** chunk-set `12345` | `mk decode` on each card's chunks | `DecodedCard::label()` is byte-identical for both: `12345 (stub 5b48af35)` |
| M-g | `--seat` resolves a card by **first match on set id, silently** | `crates/md-cli/src/seat/directive.rs:114-116` `cards.iter().position(|c| c.set_id == GroupId::Chunked(d.set_id))` | confirmed |
| M-h | Four shipped rows depend on the arm-1 refusal, not one | see C2 / I5 | confirmed |

---

# CRITICAL

## C1 — A benign double transcription of ONE card triggers the AP2 "this does not happen by accident" refusal. MEASURED.

**The claim under attack.** AP2's drafted message: *"these pieces separate
into different card sets in more than one self-consistent way … **This does
not happen by accident — treat the strings as untrusted.** Re-scan one card's
pieces alone, from plates you trust."* The security section reinforces it:
ambiguity is framed as reachable only by a ~2^32 grind.

**The falsifying scenario, measured end to end (M-a, M-b).** mk1's string
layer is a BCH code with **t = 4 correction**, applied silently by
`decode_string`. `dedupe_strings` (`seat/input.rs:120-136`) compares the
**raw** string after whitespace-strip and case-fold — it never sees the
corrected payload. So two textually different strings that decode to the
*same chunk* both survive step 1.

Take the shipped clean-twin card (A = piece 1, B = piece 2). Flip 4 bech32
characters in each to make A′ and B′. Measured with `mk decode`:

```
A,B    -> xpub6FQya7zGhR92kacYsNnjreouvnHJMpXYsUXnW6NJJAJRCKsa26TzDy4Ldn…HtF8mX
Ap,B   -> xpub6FQya7zGhR92kacYsNnjreouvnHJMpXYsUXnW6NJJAJRCKsa26TzDy4Ldn…HtF8mX
A,Bp   -> xpub6FQya7zGhR92kacYsNnjreouvnHJMpXYsUXnW6NJJAJRCKsa26TzDy4Ldn…HtF8mX
Ap,Bp  -> xpub6FQya7zGhR92kacYsNnjreouvnHJMpXYsUXnW6NJJAJRCKsa26TzDy4Ldn…HtF8mX
```

Under §3.2 all four candidates VERIFY. Two complete partitions exist —
`{(A,B), (A′,B′)}` and `{(A,B′), (A′,B)}` — so §3.3 fires **AP2: hard
refusal**, telling an operator who merely transcribed one card twice with a
few slips that their strings are untrusted and their plates suspect.

This is the exact input class a t = 4 code exists to absorb. It costs the
attacker nothing and the honest operator everything: the named remedy
("re-scan… from plates you trust") points at plates that are fine, and the
message asserts an attack that did not happen — the same W15/M3 failure the
walk corrected in AP1 ("never assert more than measured"), reintroduced in
AP2.

**Note the pre-existing half.** Today the single-sided version (one piece
duplicated with corrected errors) already lands in arm 1 and reports *"A
duplicated piece number is proof this chunk-set id is pinned to two DIFFERENT
key cards"* — also false. The case-fold fix (REVIEW-converter-whole-diff-r1
I2, documented at `input.rs:106-121`) closed this class for **case** and left
it open for **BCH correction**. The spec inherits that hole and escalates its
consequence from a wrong refusal to a wrong accusation.

**Remedy direction.** Make piece identity the **decoded** chunk, not the
string. `group_key_of` already calls `decode_string`; retain
`decoded.data()`/the fragment and collapse pieces equal on
`(chunk_set_id, total_chunks, chunk_index, fragment)` before grouping or
partitioning. Then A′ collapses into A, the group reassembles as one card
normally, and neither the false arm-1 refusal nor the false AP2 refusal is
reachable. The spec must state this canonicalisation as a **precondition of
the partition contract** — "uses every piece exactly once" is only meaningful
over canonical pieces. Add a vector row built from M-b's construction
asserting it SEATS as one card.

## C2 — "No complete partition → fall through" replaces the correct merged-cards diagnosis with a wrong one, and the spec contradicts itself about which arms it falls to.

**Two incompatible statements in one document.**

- §3.3 bullet 4: *"**no complete partition** → fall through to the shipped
  arms (**incomplete scan / terminal otherwise**) exactly as today."* — arm 1
  is explicitly excluded from the list.
- Acceptance bullet 5: *"classification-order row proves auto-partition runs
  BEFORE **the old arm-1 refusal** and falls through cleanly."* — arm 1 still
  exists after the attempt.

These produce different messages for the same input, so the contract is
undefined at its most important branch.

**Under the §3.3 reading the tool misdiagnoses, and it breaks a shipped test
written to prevent exactly this.** `input.rs:671`
`r5_classification_order_prefers_merged_over_incomplete` supplies chunk 0 of
two different 3-chunk cards, both pinned `44444`:

- total-class 3: index 0 holds 2 pieces; indices 1 and 2 hold none → no
  candidate card exists → no complete partition → fall through.
- The remaining predicate in `classify` (`input.rs:245`) is
  `infos.len() (2) < declared_total (3)` → **true** → `incomplete_refusal`:
  *"the pieces carrying this id say there should be 3; you supplied 2 — **scan
  the missing piece(s)**."*

The operator is sent to hunt for a piece that does not exist, when what they
actually hold is chunk 0 of two different cards. The shipped test asserts
`!msg.contains("scan the missing piece")` precisely to forbid this outcome —
it will fail, and it will fail because the tool regressed, not because the
test is stale.

**It also contradicts the shipped csid spec.** mnemonic-key
`SPEC_chunk_set_id_verification.md` contract 7 fixes the fork as *"the first
matching arm of 1–3, where arm 3 has NO precondition, so the classification is
total by construction (r2 C3)"*, and freezes four normative elements arm 1's
message MUST carry. Deleting arm 1 from the reachable set for any merged group
that is also short a piece makes those elements unreachable for the inputs
they were written for. The auto-partition spec's "Not in scope" section claims
no change to the shipped contracts other than arm 1's entry; this is a change
to contract 7's totality argument.

Second instance of the same shape: a group of 3 pieces (indices 0,0,1 of
total 2) has no complete partition, is not `len < total`, and reaches
`terminal_refusal` — losing the W15 piece evidence entirely. Third: a piece
declaring `chunk_index 5 of total 2` (today an arm-1 `out_of_range`) can never
be used by any candidate, so it too falls through to the wrong arm.

**Remedy direction.** State one rule and pin it: after a failed partition
attempt, **arm 1 is re-entered with its full predicate set and its shipped
message**, and only a group whose arm-1 predicates are all false proceeds to
arms 2/3. Add the classification-order fixture as an explicit acceptance row
asserting the arm-1 wording still appears.

## C3 — Per-total-class outcomes are never composed: a class that fails to partition can be silently dropped while another class seats.

§3.1 makes each total-class partition *"independently"*, but §3.3's outcome
list is written for a whole id group. **What happens when one class yields
exactly one complete partition and another yields none is unspecified.**

**Failing scenario.** One id group, 4 pieces: card A complete (2 pieces,
`total = 2`) plus 2 pieces of a 3-chunk card B (`total = 3`, one piece
missing). Disagreeing totals is an arm-1 predicate today, so the whole group
refuses. Under the spec:

- total-class 2 → exactly one complete partition → **seat card A**.
- total-class 3 → no complete partition → ? 

The naive implementation — union the successful classes' cards — **seats A and
silently discards B's two pieces.** Nothing downstream catches it:
completeness (`seat/complete.rs`) counts **cards**, never pieces, and the two
orphaned pieces never become a `DecodedCard`. The AP1 note would then also
misreport ("these 4 strings … are N different key cards" over strings that
were not all used). The operator gets a successful seating from an input they
know is incomplete, with no signal that two of their strings were thrown away.

This is a data-drop path in a restore tool, and it is the one place the
spec's own mixed-totals acceptance row (bullet 6) touches — that row covers
only the both-succeed case, so the gate cannot catch it.

**Remedy direction.** Make the composition rule normative and fail-closed:
**every piece in the id group must be consumed by some class's complete
partition; otherwise the whole group refuses** (arm 1's message, per C2).
Add an acceptance row for the partial case asserting a refusal that names the
unconsumed pieces.

## C4 — `--seat '@i=<id>'` becomes ambiguous, resolves first-match silently, and the refusal that names it as the remedy becomes a dead end.

Auto-partition produces N `DecodedCard`s sharing one `GroupId::Chunked(id)`.
`GroupId` is the ONLY handle `--seat` has (`directive.rs:38-77` refuses any
token that is not exactly the five-hex-digit set id).

- **Silent first-match (M-g).** `directive.rs:114-116` is
  `cards.iter().position(|c| c.set_id == GroupId::Chunked(d.set_id))` — the
  first card wins, with **no ambiguity check**. `--seat @0=12345` is the
  operator's explicit "put THIS key in THIS slot" assertion; after this change
  it can silently bind a different card than the one they meant, gated only by
  `satisfies()`, which two same-origin colliding cards commonly both pass.
- **The remedy becomes unexpressible.** `matching.rs:216-221` `REMEDIES` —
  printed by both the A3 ambiguity refusal and the over-bound refusal — tells
  the operator: *"(2) assert the seating yourself with `--seat
  '@i=<chunk-set-id>'` (repeatable), **using the ids printed above**."* When
  two cards print the same id, that instruction cannot distinguish them. The
  operator is handed a remedy that provably cannot work.
- **Three shipped invariants are falsified, all citing the case as settled.**
  `directive.rs:23-26` ("A5's *ambiguous id* case is **UNREACHABLE**, settled
  by SPEC A3(a) step 3 and pinned by V-COLLIDE"); `input.rs:12-20` ("step 3
  running LAST is what keeps an id collision fatal … That is also why A5's
  *ambiguous `--seat` id* case is unreachable"); `input.rs:489-492` (the same
  claim inside V-COLLIDE). The spec re-opens the case and adds no guard.

**Remedy direction.** Either (a) give auto-partitioned cards a distinguishing
identity for operator-facing purposes — e.g. a per-group ordinal appended to
the label and accepted by `--seat` (`12345#1`, `12345#2`), with the ordinal
derived from a **content**-deterministic order, not supply order (see I1); or
(b) make `--seat` refuse with a named ambiguity error when more than one card
carries the requested id. (b) is the smaller change but leaves the operator
without a resolution path, so (a) is the one that keeps `REMEDIES` true. Pin
whichever is chosen with a row that drives `--seat` at a collided id.

## C5 — The cap of 5 does NOT bound the search. Candidates are `k^n` with `n` up to 32; a legitimately minted card already reaches n = 12. MEASURED.

AP3's justification: *"worst case ≤ 5 pieces per index over ≤ 5-piece cards →
**≤ a few hundred decode attempts**, microseconds each"*.

Both halves are false.

- **The arithmetic is wrong on its own assumption.** A candidate is one piece
  per index, so with k = 5 pieces per index over an n = 5-chunk card the
  candidate count is `5^5 = 3,125` (M-e) — an order of magnitude past "a few
  hundred", before any exact-cover search over the verified candidates.
- **`n` is not capped at 5, and is not capped by AP3.** AP3 caps **cards**;
  the search is exponential in **chunks per card**. `MAX_CHUNKS = 32`
  (`consts.rs:42`) and `total = ceil((len+4)/53)` (M-c), and
  `--policy-id-stub` is repeatable, so a card minted by the shipped `mk encode`
  reaches **12 chunks at 128 stubs** (M-d, measured). k = 5 over n = 12 is
  **244,140,625** candidate decodes; at the codec ceiling n = 32 it is
  2.3×10²². Each decode re-runs BCH per chunk.
- **`n` is operator/attacker input, not a property of a card anyone owns.**
  The headers drive the search before anything verifies, so a pasted
  `--from-mk1-file` of ~160 strings declaring `total_chunks = 32` hangs the
  tool with no card ever decoding.

"Timing measured at implementation" does not rescue this: the measurement
would be taken on the walk's 2×2 case and report microseconds, while the
reachable input space is `k^n`. That is a gate that cannot fail on the input
it was written for.

**Remedy direction.** Bound the **search**, not the card count, and reuse this
codebase's own precedent — `matching::MATCHING_BOUND = 720` with an explicit
over-bound refusal (`matching.rs:47,237-248`). Concretely: (1) state the exact
necessary condition from I7 and refuse *before* enumerating when it fails;
(2) add a hard budget on candidate decodes with a named refusal on exceeding
it; (3) restate AP3 as "≥ 3 cards supported, cap 5 cards **and** a decode
budget", with the budget's value measured, not asserted.

---

# IMPORTANT

## I1 — The A3 tie-break stops being discriminating; the emitted wallet becomes order-dependent, and the spec defines no card order.

`matching.rs:143-146` keys the tie-break on
`assignment_vector: Vec<GroupId>`, and the module doc (`matching.rs:33-37`)
carries the safety argument verbatim: *"Assignment vectors differ between
distinct matchings by construction, so this order is total AND
discriminating"* — with the explicit note (r7 I1) that this branch is entered
when comparison forms are byte-equal *"while the emitted descriptors and their
WalletPolicyIds still differ."*

Two auto-partitioned cards share a `GroupId`, so **two distinct matchings can
produce identical assignment vectors**. `min_by_key` then falls back to
first-encountered order, i.e. to the order the partition search happened to
emit cards in — which the spec never defines. Same input, different internal
ordering, different emitted descriptor and WalletPolicyId. `V-ORD` exists to
guarantee supply order cannot do this.

**Remedy:** define a deterministic, **content**-derived order for cards
produced by one partition (e.g. ascending canonical bytecode, or ascending
derived chunk-set id), state it as normative, and either extend the tie-break
key with that discriminator or pin a row proving the emitted descriptor is
invariant under input permutation of a collided group.

## I2 — `DecodedCard::label()` collides, so every downstream refusal names two different cards identically. MEASURED. The surplus-card gate would still PASS.

`label()` (`input.rs:81-90`) is `"{set_id} (stub …)"`. Measured on the shipped
fixture (M-f): v-collide's two cards carry stub `5b48af35` **and** id
`12345` → both label as `12345 (stub 5b48af35)`. Cards of one wallet normally
share a policy-id stub, so this is the common case, not a corner.

Affected surfaces: `complete::refusal`'s leftover list — whose stated purpose
is *"which wallet do these extras belong to"* and which promises *"Each is
named by its full chunk-set id and the policy-id stub it was minted against"*
(`complete.rs:102-105`); `check_no_impossible_card_pair` and
`check_no_repeated_xpub` (`satisfy.rs:207-263`), both of which name a *pair*
of cards; and `--seat`'s "no supplied card has chunk-set id …" list
(`directive.rs:118-125`), which would print the same id twice.

**This makes the spec's surplus-card acceptance row a false PASS.** The row
asserts the downstream refusal message is "unchanged" — it *is* unchanged in
wording, and it has stopped identifying the cards. The gate passes while the
behaviour it exists to protect has regressed.

**Remedy:** carry a distinguishing token in the label for auto-partitioned
cards (see C4's remedy (a)) and strengthen the surplus row to assert the two
leftover lines are **distinguishable from each other**, not merely present.

## I3 — The AP1 note has no emission channel, no ordering rule, and asserts a seating that may not happen.

`decode_cards` returns `Result<Vec<DecodedCard>, CliError>`
(`input.rs:325`) — there is no channel for a note. The shipped note pathway is
`Seating.notes` → `emit_seating_notes` (`cmd/descriptor.rs:295-299`), and
`Seating` does not exist until the whole engine succeeds (`seat/mod.rs:176-180`).

So an implementer either changes the signature (affecting every caller
including the test fixture helper at `satisfy.rs:266-282`) or prints eagerly
from inside `decode_cards`. **The eager option makes the note lie.** Its
drafted text ends *"so they were separated and seated individually"* — but
seven checks run after `decode_cards` and any of them can refuse
(`check_no_repeated_placeholder`, `slot_declarations`,
`check_no_identical_fp_bearing_declarations`, `check_no_impossible_card_pair`,
`check_no_repeated_xpub`, `directive::apply`, `matching::decide`/completeness).
The spec's **own surplus-card acceptance row is one of those runs**: the note
would claim the cards were seated, immediately followed by a refusal saying
they were not.

The related sub-question — how `note:` composes with the R2 `warning:` on
stderr — is likewise unanswered: §3.4 says "expect the note PLUS one warning
per mismatched card" but fixes no order, and the two mismatch warnings for a
collided pair carry the **same** `declared` id with different `derived` ids,
which reads confusingly without the note adjacent.

**Remedy:** make the note a value, not a side effect — return it alongside the
cards, carry it into `Seating.notes` **ahead of** that group's R2 warnings, and
state that it is emitted **only on a successful seating**. Pin the ordering and
the refusal-path suppression as rows.

## I4 — The byte-identity acceptance row's control is not constructible as worded.

Bullet 1: *"composed descriptor and address BYTE-IDENTICAL to **scanning the
two cards separately**."*

Scanning them separately cannot produce a descriptor. Completeness is total
and `matching::enumerate` returns no matchings when `n_slots != n_cards`
(`matching.rs:96-99`), so a 2-slot policy offered one card yields a refusal,
not a descriptor to compare against. And the two cards cannot be scanned
together *without* the collision — sharing the stamped id is the premise.

The intended control is almost certainly "the same key material minted with
distinct (natural) chunk-set ids, seated in one run". That IS constructible
and is the right control, since `set_id` never enters the composed descriptor.
As written the row is unbuildable, which is how a gate silently becomes a
weaker assertion at implementation time.

**Remedy:** state the control explicitly — un-pinned twin mint, same keys and
origins, one run — and require the comparison to cover descriptor **and**
address **and** WalletPolicyId.

## I5 — "the shipped csid tests unchanged except the arm-1 entry row" is false: four rows flip, plus three doc invariants.

Acceptance bullet 9 claims a single row changes. Enumerated against
`54ab1cd6`:

1. `crates/md-cli/src/seat/input.rs:569`
   `r5_merged_two_cards_pinned_to_one_id_classify_as_merged` — two 2-chunk
   cards pinned `11111`; this is the spec's own canonical case, so it now
   seats and `decode_cards(&strings).unwrap_err()` **panics**.
2. `crates/md-cli/src/seat/input.rs:671`
   `r5_classification_order_prefers_merged_over_incomplete` — fails, and fails
   because the tool regressed (C2).
3. `crates/md-cli/src/seat/input.rs:489`
   `v_collide_two_cards_pinned_to_one_chunk_set_id_refuse_at_reassembly` — the
   fixture is a 2-chunk and a 3-chunk card under one id (verified: both decode,
   stubs `5b48af35`), i.e. the spec's *mixed totals* row, so it now seats and
   `unwrap_err()` **panics**.
4. `crates/md-cli/tests/seating_vectors.rs:834`
   `v_collide_reaches_the_command` — asserts exit 1 plus `"piece order does not
   matter"` and `` "`mk inspect`" ``; both wordings disappear.

Also falsified, and not mentioned anywhere in the spec: the module-doc
invariants at `input.rs:12-20`, `input.rs:489-492` and `directive.rs:23-26`
(C4), and `complete.rs`/`matching.rs`'s identity assumptions (I1, I2).

**Remedy:** replace bullet 9 with the enumerated list above, stating for each
row whether it is retired, rewritten, or moved — and say which new row
inherits each retired row's guarantee. A row that merely disappears takes its
guarantee with it.

## I6 — The AP2 gate has no stated construction; the only cheap one pins the C1 defect. Never-run-gate risk.

Acceptance bullet 4 requires *"a constructed second verifying partition"* and
says nothing about how.

- The **cheap** construction is C1's: four flipped characters per piece,
  zero compute (M-b). If the implementer finds this — the obvious route — the
  gate will pin the false-accusation bug as correct behaviour.
- The **adversarial** construction is genuine but non-trivial. Because the
  4-byte hash is `SHA-256(bytecode)[0..4]` in the trailing bytes of the last
  chunk (M-c), a two-card ambiguity needs both cross-pairings to verify: one
  ~2^32 grind to match the victim's fixed trailing hash, plus a second ~2^32
  grind for a 4-byte collision between two prefixes — and every grind
  candidate must also reassemble into a **valid `KeyCard` bytecode**, which
  the spec never mentions and which is what makes the grind non-obvious.
- The mutation gate *"force-accept the first partition when two verify →
  ambiguity row fails"* inherits this risk entirely: with no valid fixture,
  the mutation cannot be detected.

**Remedy:** specify the construction, its cost, and that the fixture is
committed with a regeneration script; and state explicitly that a
BCH-equivalent duplicate is **not** a valid ambiguity fixture (it must SEAT,
per C1).

## I7 — The cap rule omits the necessary condition that makes it exact, and misses the cheapest refusal.

§3.3 caps on *"piece count per index > 5"*. The underlying invariant is never
stated: since a candidate card consumes **exactly one piece at every index**,
`k` cards consume exactly `k` pieces at every index. Therefore a complete
partition **requires every index in a total-class to hold the same count `k`**,
and the class to hold exactly `k·n` pieces.

Consequences of leaving it out:

- The cap check is a proxy (max-per-index) rather than the exact quantity, so
  "5 cards" is not what is being bounded.
- The cheapest possible refusal is missed: unequal per-index counts ⇒ **no
  complete partition exists**, decidable in O(pieces) with zero decodes. This
  is the condition that would short-circuit most of C5's blow-up, and it is
  exactly the shape of the C3 partial-class case.
- "Complete partition uses every piece exactly once" is stated but its
  arithmetic consequence is not, so an implementer can write a search that
  enumerates first and discovers impossibility last.

**Remedy:** state the invariant, derive the cap from it (`k` = the common
per-index count; `k > 5` → cap refusal; unequal counts → immediate
no-partition), and pin a row for the unequal-count input.

---

# MINOR

- **M1 — AP2's draft violates the spec's own cards-never-plates rule.** The
  message ends *"from **plates** you trust"*, while §3.3 asserts *"cards never
  plates; … (the shipped W-rules bind)"* and the shipped arm-1 row asserts
  `!msg.contains("plate")` (`input.rs:596-599`). Either the draft or the claim
  must move; the shipped R2 warning shows "plate" is permissible in a
  *diagnostics-name* clause, so the rule needs stating precisely rather than
  as a blanket ban.
- **M2 — AP1's note overstates the oracle.** *"each card's own integrity check
  confirms its pieces"* — a 4-byte hash is a 2^-32 check, and the union over
  `k^n` candidates weakens it further. The walk's AP1 correction was precisely
  "never assert more than measured". Prefer "each card's own 4-byte integrity
  check accepted its pieces".
- **M3 — glosses.** Both drafts use "pieces" without the shipped first-use
  gloss (arm 1: *"each key card's mk1 strings are its chunks (pieces)"*; arms
  2/3: *"the pieces (chunks)"*). §3.3 claims "glosses at first use" bind.
- **M4 — the trigger is defined twice, differently.** §3 header says the
  contract *"replaces classifier arm 1"*; §3's first line says the input is
  *"one id group whose strings do not reassemble as a single card"*. These are
  different predicates — the terminal case (cross-chunk hash mismatch, arm-1
  predicates all false) satisfies the second and not the first. Outcome
  happens to coincide today; the definition should still be single-valued.
- **M5 — degenerate shapes unaddressed.** `Chunked { total_chunks: 1 }` is
  representable (header range is 1..=32) and would make every piece its own
  card; it is unreachable from a real mint (the 73-byte compact xpub floor
  noted at `input.rs:41-45` forces `total ≥ 2`), but the spec should say so
  rather than leave it to the implementer. Likewise `GroupId::Single` groups
  (never classified, `input.rs:340-343`) are silently out of scope.

# NIT

- **N1 —** §3.2 says "one piece per index **1..n**"; `chunk_index` is 0-based
  on the wire and in the code (`piece_evidence` converts for humans at
  `input.rs:274`). State which convention the contract uses.
- **N2 —** The AP1 note renders the id as `"(12345)"` while every shipped
  message renders it as `"chunk-set 12345"`. Consistent framing costs nothing
  and matters for grep-ability in operator transcripts.

---

# What is SOUND (so the next round does not re-derive it)

- The oracle choice is right: `SHA-256(bytecode)[0..4]` recomputed by
  `reassemble_from_chunks` (`chunk.rs:278-289`) genuinely rejects cross-card
  pairings, needs no new crypto, and the walk's 2-candidate measurement holds.
- Sub-grouping by declared total (§3.1) is correct and is a real improvement:
  pieces with different `total_chunks` provably cannot share a card
  (`chunk.rs` rejects disagreeing totals outright).
- AP2-as-refusal is the right call for genuine ambiguity; the objection in C1
  is to the *reachability* and the *wording*, not the ruling.
- The security section's frankencard claim survives attack: an attacker who
  replaces a single victim piece already faces the same 2^32 grind today on a
  single-card scan, and pinning a whole substitute card to the victim's id was
  *penalised* before (arm 1) and is now merely *noted* — which is a real
  reduction in fail-closed behaviour, but not an enlargement of the frankencard
  surface, and the AP1 note is a proportionate mitigation for it.
- The mutation gates are well-formed as stated ("disable the search" /
  "force-accept the first partition"), subject to I6 supplying a fixture the
  second one can act on.

# Gate-can-fail summary

| Acceptance row | Constructible? | Can it fail? |
| --- | --- | --- |
| canonical 2×2 seats, byte-identical | partially — control unbuildable as worded (I4) | yes, once the control is restated |
| 3-card collision | yes | yes |
| 6-card cap refusal | yes | yes — but bounds the wrong quantity (C5) |
| **ambiguity → AP2** | **not as stated** (I6) | **at risk of never running, or of pinning C1** |
| no-partition / classification-order | yes | **expected output undefined** (C2) |
| mixed totals both seat | yes (it is `v-collide.txt`) | yes — but silent on the partial case (C3) |
| surplus card → downstream refusal | yes | **passes while degraded** (I2) |
| mutation: disable search | yes | yes |
| mutation: force-accept first partition | inherits I6 | inherits I6 |

# Recommended order of fold

C1 and C5 change the algorithm (piece canonicalisation; a search budget plus
the I7 invariant). C2 and C3 change the outcome table (arm-1 re-entry;
all-pieces-consumed). C4/I1/I2 change card identity and should be folded as
one decision, since a single distinguishing token answers `--seat`, the
tie-break and the labels together. I3–I6 are contract/acceptance text that
depends on those four decisions, so fold them last.

**0C/0I is not close; this needs a structural revision of §3, not wording
passes.**
