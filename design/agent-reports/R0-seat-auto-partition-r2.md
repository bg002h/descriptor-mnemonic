# R0 — `design/SPEC_seat_auto_partition.md` @ `55d28f99`, round 2

**Questions asked:** (1) does the fold discharge each r1 finding? (2) did the
structural revision of §§1–5 introduce new defects?

**VERDICT: 0 Critical / 7 Important / 6 Minor / 2 Nit — NOT GREEN.**

Of the 7 Important, **3 are r1 items still open** (I2, I5, I6 — PARTIAL, tier
retained per brief) and **4 are new**, all in the machinery the fold added:
canonicalisation (§1), the decode budget (§2.3), and the outcome order (§3).

Reviewed against descriptor-mnemonic `55d28f99` (spec) over origin/main
`54ab1cd6` (code) and vendored `mk-codec 0.5.0`. Not re-litigated, per brief:
the oracle choice, total-class sub-grouping, AP2-as-refusal, the frankencard
surface, and rulings AP1–AP3.

## What was MEASURED for this round (not inferred)

| # | Measurement | Command / source | Result |
| --- | --- | --- | --- |
| N-a | Two GENUINELY DIFFERENT key cards, pinned to one chunk-set id, can share a **byte-identical piece** | `mk` 0.13.0: two distinct xpubs, **different** `--origin-fingerprint`, 13 shared `--policy-id-stub`, both `--chunk-set-id 12345` | chunk 0 **string-identical**; the two 3-chunk cards yield **5** distinct strings, not 6; each card decodes alone (`mk decode`) |
| N-b | The threshold is exactly 13 shared stubs | same, N = 11, 12, 13, 14 | 6, 6, **5**, **5** distinct strings |
| N-c | Why: bytecode is `[header][stub_count][stubs…][fp][path][xpub73]` and chunks are 53-byte slices of `bytecode‖hash` | `vendor/mk-codec/src/bytecode/encode.rs:86-97`; `string_layer/chunk.rs:155-181` | `2 + 4N ≥ 53` ⇒ chunk 0 is a pure function of the header byte and the shared stub list |
| N-d | Budget-vs-cap arithmetic | `k^n` vs the spec's `≤ 4096` ceiling | `3^7=2187 ≤ 4096 < 3^8=6561`; `5^5=3125 ≤ 4096 < 5^6=15625`; `2^12=4096` exactly; `5^32` overflows `u64` |
| N-e | Card size from the shipped encoder | model `1+1+4N+4+1+73`, cross-checked against r1 M-d | N=128 → **12 chunks** (matches M-d); N=139 → 13; N=255 (encoder cap, `encode.rs:28`) → **21** |
| N-f | `v_collide_reaches_the_command`'s input already carries a FULL card set | `crates/md-cli/tests/seating_vectors.rs:834-853`; `tests/fixtures/seating/v-usp.txt` (2 slots, 2 cards), `v-collide.txt` (5 mk1 lines: a 2-chunk + a 3-chunk card) | post-change the run has **4 cards for 2 slots** → a completeness/leftover refusal, **not** "seat+note" |
| N-g | Both v-collide cards label identically | `v-collide.txt` provenance (both `--policy-id-stub 5b48af35`, both id `12345`) + `input.rs:82-90` | `12345 (stub 5b48af35)` twice — r1 M-f, reconfirmed as the leftover-list case |
| N-h | §1 is implementable without touching mk-codec | `decode_string` → `from_5bit_symbols` (returns `consumed`, discarded today at `input.rs:171`) → `five_bit_to_bytes`; all `pub` (`string_layer/mod.rs:32-39`) | confirmed — the out-of-scope line holds |

---

# PART 1 — Per-finding disposition

## C1 — canonical pieces precondition + BCH-twin row → **FIXED**

Discharged by §1 in full:

> "two strings are the SAME piece iff their `(chunk_set_id, total_chunks,
> chunk_index, decoded fragment bytes)` are equal; duplicates collapse"

and the precondition is bound to the partition contract:

> ""Uses every piece exactly once" below means canonical pieces."

with acceptance row 2:

> "BCH-twin row (r1 M-b construction): double transcription with ≤4 flips
> per piece → collapses, seats as ONE card, NO note, NO refusal."

and mutation gate row 9 (`skip canonicalisation → row 2 fails`). The r1 M-b
construction lands where C1 demanded. Checked: the key includes `chunk_index`,
so a card's own two chunks never collapse into each other.

**But the converse was not considered — see NEW-I1.** Collapsing on decoded
bytes is right for *transcriptions of one piece* and wrong for *a piece two
different cards genuinely share*, which is constructible with the shipped
encoder (N-a/N-b).

## C2 — arm-1 re-entry + totality vs csid contract 7 → **FIXED**

> "**no complete partition** (incl. unequal counts, any class incomplete) →
> **arm 1 is re-entered with its FULL shipped predicate set and message**
> (r1 C2); only a group whose arm-1 predicates are all false proceeds to
> arms 2/3. The shipped four-arm classifier stays total exactly as the
> csid spec's contract 7 froze it; auto-partition is a pre-pass, not a
> replacement."

Verified against `classify` (`input.rs:235-259`) on all three r1 C2 shapes:

- the `44444` classification-order fixture (2× chunk 0 of 3-chunk cards):
  counts 2/0/0 → unequal → arm 1 → `duplicate` true → `Merged` →
  "2 strings declare piece 1 of 3", and `!msg.contains("scan the missing
  piece")` still holds. Pinned by row 5.
- 3 pieces at indices 0,0,1 of total 2: `duplicate` **and** `excess` true →
  `Merged`, so the W15 piece evidence survives (r1's second instance).
- `chunk_index 5 of total 2`: `out_of_range` true → `Merged` (r1's third).

Also checked the honest short scan (2 of 3 chunks, one card): counts 1/1/0 →
unequal → arm 1 predicates all false → arm 2 fires with "scan the missing
piece(s)", unchanged. Contract 7's totality argument is intact.

## C3 — fail-closed cross-class composition + no-partial-seating row → **FIXED**

> "A COMPLETE PARTITION of the id group is a set of verified candidates
> consuming every canonical piece in EVERY total-class exactly once
> (composition is fail-closed across classes — r1 C3: a class that cannot
> complete fails the whole group; no partial seating, no dropped pieces,
> ever)."

with row 6:

> "Mixed totals both complete → both seat; one class incomplete → whole
> group refuses via arm 1, nothing seats, no pieces dropped (r1 C3)."

The r1 C3 scenario (a complete 2-chunk card + 2 pieces of a 3-chunk card)
now refuses: class 3 has counts 1/1/0 → unequal → whole group fails. The
data-drop path is closed.

## C4 — `#<k>` ordinal + `--seat` ambiguity refusal + doc-invariant churn → **FIXED** (residues in I5 and NEW-M2)

> "**`--seat` (r1 C4):** `@i=<id>#<k>` is accepted and resolves exactly one
> collided card. A bare `@i=<id>` naming an id carried by MORE than one
> seated card → a named ambiguity refusal pointing at the `#<k>` form
> (silent first-match is removed for collided ids)."

This is r1's remedy (a) plus (b), which is strictly better than either: the
silent `position()` first-match at `directive.rs:114-116` is removed *and*
`REMEDIES` stays true because the printed handle is now resolvable. The three
UNREACHABLE doc invariants are named for rewrite.

Two residues, filed elsewhere rather than against C4: the `#<k>` grammar's own
refusals are undefined (NEW-M2), and the churn list omits the two operator
texts that must change with it (I5).

## C5 / I7 — exact-count invariant, cap on `k`, `PARTITION_DECODE_BOUND ≤ 4096`, zero-decode short-circuit, the 160-string row → **FIXED as asked** (the new mechanism has its own defects: NEW-I2, NEW-I3, NEW-I4)

I7's invariant is now normative and derives the cheapest refusal:

> "a complete partition of a total-class with total n requires EVERY index
> 0..n-1 to hold the same piece count k, and the class to hold exactly k·n
> pieces. Unequal per-index counts ⇒ no complete partition exists — decided
> in O(pieces) with ZERO decodes."

C5's budget exists, names the precedent, and states the ceiling:

> "the total number of candidate decode attempts across the group is bounded
> by `PARTITION_DECODE_BOUND` (value fixed at implementation from
> measurement; the `matching::MATCHING_BOUND = 720` precedent applies; the
> spec requires it ≤ 4096) — exceeding it → over-budget refusal naming the
> bound."

and the false AP3 arithmetic is retracted in the same paragraph ("k^n is NOT
bounded by the cap alone"). Row 4 carries r1's 160-string input.

Everything r1 asked for is present. What r1 could not ask for is whether the
new bound is *well-defined* (NEW-I2), whether it keeps AP3's own floor
(NEW-I3), and whether it composes with the invariant it sits beside
(NEW-I4). All three are new.

## I1 — content order + tie-break extension + permutation row → **FIXED** (residue NEW-M5)

> "cards produced by one partition are ordered by ascending canonical
> bytecode; this order is normative. The A3 tie-break key is extended with
> it so distinct matchings remain discriminated"

Checked the discrimination claim actually holds: distinct cards in one group
have distinct bytecodes (identical cards would have collapsed at §1), so
`(GroupId, ordinal)` re-totalises the key `matching.rs:143-146` lost. V-ORD's
"ascending set-id order" (`input.rs:327-330`) also stays total, because the
within-group tie is broken by the same content order. Row 8 pins permutation
invariance.

## I2 — distinguishable labels + strengthened surplus row → **PARTIAL (Important, tier retained)**

The **label** half is fixed:

> "each collided card is labelled `<id>#<k>` … used by `label()` and every
> downstream message that names cards, so leftover lists and pair-refusals
> stay distinguishable"

The **gate** half is not. §4 promises a row —

> "(r1 I2; the surplus row asserts the two leftover lines DIFFER)"

— and the security section relies on it —

> "Surplus-card injection: partitions cleanly, then completeness refuses
> downstream with DISTINGUISHABLE labels (§4)."

— but **the acceptance list contains no surplus row.** `grep -c surplus`
over the spec returns **1**, and it is the §4 reference above. The draft at
`24d3b613` had one ("Surplus-card row: partition succeeds, seating
completeness refuses downstream"); the fold deleted it while adding two
citations to it. r1's finding was precisely that the surplus row *passes
while degraded*; the response removed the row instead of strengthening it, so
the label regression now has no gate at all. This is the "a plan may not close
while one of its own gates has never been run" shape, one step earlier: a gate
cited twice and never written.

**Remedy:** restore the row and give it the assertion I2 asked for — two
leftover lines, both present, **and textually different from each other**.
The fixture already exists: N-f/N-g show `v_collide_reaches_the_command` is
exactly this input and its two leftover cards label identically today.

## I3 — note-as-value, success-only emission, ordering → **FIXED** (residue NEW-M6)

> "The AP1 note is a VALUE returned with the cards, carried into
> `Seating.notes` and emitted ONLY on a successful seating, ordered ahead of
> that group's R2 warnings (r1 I3 — never printed on a downstream refusal)."

Checked against the plumbing: `Seating { descriptor, notes }` is only built
after every downstream check (`seat/mod.rs:124-181`), so "only on success" is
structural rather than promised, exactly as I3 asked. `notes` is seeded with
`csid_warnings` at `seat/mod.rs:177`, so "ahead of the R2 warnings" is a
prepend. Row 1 pins the composition and the order.

## I4 — unpinned-twin control incl. WalletPolicyId → **FIXED**

> "descriptor, address AND WalletPolicyId byte-identical to the CONTROL: the
> same key material minted UNPINNED (natural distinct ids), seated in one run"

This is r1's exact remedy, and it is constructible (`set_id` never enters the
composed descriptor, so the unpinned twin is the right control).

## I5 — enumerated churn with per-row guarantee inheritance → **PARTIAL (Important, tier retained)**

Row 10 replaces the false "only the arm-1 entry row" claim with a real
enumeration and assigns each retired guarantee a new home. Three of the four
rows are correctly classified. But the row ends with a completeness claim —

> "Every retired assertion's guarantee is named above; none silently
> disappears."

— that the row does not earn, on two counts.

**(a) One entry states the wrong outcome. MEASURED (N-f).** Row 10 says:

> "`v_collide_reaches_the_command` REWRITTEN (asserts the new seat+note
> outcome end-to-end)"

That test does not offer the collided cards alone. It offers **V_USP's full
card set plus the collided pair** (`seating_vectors.rs:840-844`:
`let mut cards = mk1(V_USP); cards.extend(collide);`), against the 2-slot
V_USP policy. Post-change that is 4 cards for 2 slots, so `enumerate` returns
no matchings (`matching.rs:96-99`) and the outcome is `complete::refusal` —
a **leftover refusal**, and per I3 **no note at all**. An implementer
following row 10 literally would strip V_USP's cards from the fixture to make
"seat+note" true, and in doing so would delete the only end-to-end coverage of
the surplus/leftover path — the very path §4's ordinal fix exists for (N-g:
both leftover cards label `12345 (stub 5b48af35)` today). The two open halves
of I2 and I5 are therefore the same row: this test *is* the surplus row.

**(b) Two operator-facing texts falsified by §4 are missing from the
enumeration.** Both are `--seat` texts that C4's change makes wrong:

- `matching.rs:216-221` `REMEDIES`: "assert the seating yourself with
  `--seat '@i=<chunk-set-id>'` (repeatable), **using the ids printed
  above**" — the ids printed above are now `12345#1`/`12345#2`, which are not
  chunk-set ids.
- `directive::parse` (`directive.rs:41-43, 60-73`) refuses anything that is
  not exactly five hex digits, with the measured prefix-collision rationale.
  `12345#1` fails `is_ascii_hexdigit` and is refused as "not a chunk-set id".

Also unlisted: the `decode_cards` signature change I3 requires, and its
callers (`seat/mod.rs:143`, plus 8 test call sites in
`complete.rs`/`disposition.rs`/`input.rs`).

## I6 — committed grind fixture + script + "a BCH twin is not a fixture" → **PARTIAL (Important, tier retained)**

Row 7 delivers three of r1's four asks — a committed fixture, a committed
regeneration script, and the exclusion —

> "a BCH-equivalent duplicate is NOT a valid fixture, it must seat per row 2"

— but not the **construction**, and its cost figure is short by half:

> "~2^32 SHA-256 + KeyCard-validity — feasibility demonstrated by the script"

"Feasibility demonstrated by the script" is circular: the script is the thing
that has to be built, and r1's finding was that an implementer who cannot
build it will weaken or waive the gate. The single-grind figure makes it
worse, because the obvious single-grind attempt is **circular and fails**:

For `k = 2, n = 2`, ambiguity requires all four pairings to verify. Writing
`f0a, f0b` for the index-0 fragments and `S_a, S_b` for the index-1 fragments
minus their trailing 4 bytes, the two *own* cards are free (you compute their
hashes), and the two crossed cards impose

- (1) `H(f0a‖S_b)[0..4] == H(f0b‖S_b)[0..4]`
- (2) `H(f0b‖S_a)[0..4] == H(f0a‖S_a)[0..4]`

Grinding `f0a` for (1) changes `H(f0a‖S_a)`, the target of (2); grinding `f0b`
for (2) changes `H(f0b‖S_b)`, the target of (1). The naive order never
terminates. A working order exists: fix `f0a, S_a`; grind `f0b` for (2)
(~2^32); then, with `f0a, f0b, S_a` frozen, grind `S_b` for (1) (~2^32) —
which cannot disturb (2) because (2) does not mention `S_b`. **Two sequential
2^32 grinds, ~2^33 SHA-256 total**, minutes on this box, negligible memory.

The KeyCard-validity constraint then decides *where* the grind freedom lives:
`S_b` is xpub-tail bytes for a 2-chunk card (N ≤ 5 stubs at n = 2, by N-e's
model), so grinding it means grinding a compact-73 xpub that must stay a valid
point. Choosing `n = 3` with 13–26 stubs puts stub bytes in chunks 0 **and**
1, giving 4-byte-aligned grind freedom in both halves with validity free.

**Remedy:** put that construction (or an equivalent) and the corrected
~2^33 cost in row 7, so the script has a specification rather than an
existence claim.

## Minor / Nit carry-over

| r1 | Disposition | Discharging text |
| --- | --- | --- |
| M1 (plates) | **FIXED** | AP2 draft now ends "re-scan one card's pieces alone, from **a source** you trust"; ""source", not "plates" — r1 M1" |
| M2 (oracle overstated) | **FIXED** | "each card's own **4-byte** integrity check **accepted** its pieces"; ""accepted its pieces", not "confirms" — r1 M2" |
| M3 (glosses) | **FIXED** | note: "each key card's mk1 strings are its pieces (chunks)"; AP2: "these pieces (chunks)" |
| M4 (trigger defined twice) | **FIXED** for the entry trigger — §2's header and the one-sentence summary now agree ("fails single-card reassembly" / "do not form one key card") and §3 settles it with "auto-partition is a pre-pass, not a replacement". **But the same shape reappears one level down, at the outcome order — NEW-I4.** |
| M5 (degenerate shapes) | **FIXED** | Out of scope: "`GroupId::Single` groups and `total = 1` chunked headers (unreachable from real mints — 73-byte compact-xpub floor forces total ≥ 2; stated, not silently assumed — r1 M5)" |
| N1 (index base) | **FIXED** | "Indices are 0-based, matching the wire (r1 N1)." |
| N2 (id framing) | **FIXED** | note renders "(chunk-set 12345)" |

---

# PART 2 — New findings

## NEW-I1 (Important) — §1's collapse merges a piece that two DIFFERENT cards legitimately share, and the exact-count invariant then declares "no complete partition" for the very input the feature exists to untangle. MEASURED.

**The claim under attack.** §1: "two strings are the SAME piece iff their
`(chunk_set_id, total_chunks, chunk_index, decoded fragment bytes)` are
equal", and §2.4's "consuming every canonical piece … exactly once".

Both are sound for *transcriptions of one piece*. Neither is sound as a rule
about **cards**, because a piece is a per-card resource and two different
cards can carry the same bytes at the same index.

**Measured, end to end (N-a, N-b, N-c).** The bytecode is
`[header][stub_count][stubs…][fp][path][xpub73]`
(`bytecode/encode.rs:86-97`) and the chunks are 53-byte slices of
`bytecode‖SHA-256(bytecode)[0..4]` (`chunk.rs:155-181`). So whenever
`2 + 4N ≥ 53` — i.e. **N ≥ 13 policy-id stubs** — chunk 0 is a pure function
of the header byte and the stub list, and the xpub and fingerprint never
reach it. Cards of one wallet share their stub list by construction.

```
mk encode --xpub <KEY A> --origin-path m --origin-fingerprint 73c5da0a \
  --policy-id-stub aabbcc00 … (13 stubs) --chunk-set-id 12345
mk encode --xpub <KEY B> --origin-path m --origin-fingerprint b8688df1 \
  --policy-id-stub aabbcc00 … (same 13)   --chunk-set-id 12345

chunk 0: IDENTICAL   chunk 1: differs   chunk 2: differs
distinct strings across BOTH cards: 5   (N=11,12 → 6; N=13,14 → 5)
each card decodes alone under `mk decode`
```

**The failure.** The operator scans both plates — two real cards, one stamped
id, the AP1 case exactly. Canonical pieces: index 0 → **1**, index 1 → 2,
index 2 → 2. §2.2 fires: unequal per-index counts ⇒ "no complete partition
exists", zero decodes. §3 sends it to arm 1, whose `duplicate`/`excess`
predicates are both true (`input.rs:241-253`), and the operator gets today's
merged-cards refusal. The correct partition — `A = (P0, A1, A2)`,
`B = (P0, B1, B2)` — needs `P0` **twice**, which §2.4 forbids.

Note the shipped byte-identity dedupe (`input.rs:123-138`) already collapses
these two strings today, so this is not a regression. What is new is that the
spec now *builds a normative contract on the invariant*, and states the
invariant as exact ("requires EVERY index to hold the same piece count k")
when it is exact only under an assumption the format does not guarantee.

The outcome is fail-closed (a refusal with a workable remedy — "Re-scan one
card's pieces alone"), which is why this is Important and not Critical. But
AP3's "at least 3 colliding cards supported" is silently false for this class
of card, and nothing in the spec or the acceptance list mentions it.

**It also poisons the natural fixtures.** Any many-chunk fixture built the
obvious way — several cards sharing a stub list, pinned to one id — collapses
instead of partitioning. With 255 stubs (the encoder cap) the stub region
spans chunks 0–18 of a 21-chunk card, so ~19 of every card's pieces are shared.
See NEW-M1.

**Remedy, either way:**

- **(a) keep the feature working:** let a canonical piece be consumed by more
  than one candidate when its multiplicity is *forced* — restate §2.2 as "each
  index holds at most k **distinct** canonical pieces, and an index holding
  fewer than k is admissible only when the shortfall is explained by identical
  fragments", with the search allowed to reuse such a piece; or
- **(b) state the limitation:** say normatively that a group in which two
  cards share a canonical piece cannot be auto-partitioned and refuses via arm
  1, and add a row built from the N-a construction pinning that refusal.

(a) keeps AP3's promise; (b) is cheap and honest. What is not acceptable is
the current text, which asserts exactness it does not have.

## NEW-I2 (Important) — the over-budget outcome is not a function of the input.

§2.3 defines the budget over a **dynamic** quantity:

> "the total number of candidate decode attempts across the group is bounded
> by `PARTITION_DECODE_BOUND`"

A behaviour contract's outcomes must be a function of the input. This one is a
function of the enumeration strategy, and two conforming implementations
disagree:

- **Naive.** Enumerate all `k^n` candidates, decode each. 3 colliding
  8-chunk cards ⇒ `3^8 = 6561 > 4096` ⇒ **over-budget refusal**.
- **Length-filtered.** A candidate's reassembled stream must be
  `53·(n−1) + len(last fragment)` bytes, and every non-terminal fragment must
  be exactly 53 (`chunk.rs:170-173`); for small stub counts the total bytecode
  length is derivable from chunk 0's own `stub_count`/fingerprint-flag byte.
  Rejecting length-mismatched candidates costs zero decodes, so the same input
  decodes ~3 candidates and **seats**.

Same strings, two lawful outcomes, one of them a refusal. Row 4 ("headers
driving > PARTITION_DECODE_BOUND candidates") is only well-defined under the
naive strategy, and the mutation-gate discipline cannot bite on a trigger the
implementer chooses.

**Remedy:** make the trigger structural and pre-computed, which is also
cheaper and matches §2.2's zero-decode style:

> refuse when `Σ_classes k^n > PARTITION_DECODE_BOUND`, evaluated **before any
> decode**, with saturating arithmetic

The saturation matters: `5^32 = 2.3×10^22` overflows `u64` (N-d), and under
this repo's `opt-level = 2` + `debug_assertions` profile an unchecked `pow`
**panics** rather than refusing.

## NEW-I3 (Important) — `PARTITION_DECODE_BOUND ≤ 4096` silently narrows AP3's "at least 3 colliding cards supported" to n ≤ 7 chunks, and the cap of 5 to n ≤ 5.

The spec states the ceiling as a requirement ("the spec requires it ≤ 4096")
and never checks it against the ruling it sits under. Arithmetic (N-d):

| cards `k` | largest `n` within 4096 | first `n` that refuses |
| --- | --- | --- |
| 2 | 12 (`2^12 = 4096` exactly) | 13 (`8192`) |
| 3 (**AP3's floor**) | 7 (`2187`) | 8 (`6561`) |
| 5 (**AP3's cap**) | 5 (`3125`) | 6 (`15625`) |

These are not hypothetical sizes. r1 M-d measured a **legitimately minted
12-chunk card** (128 stubs), reproduced by N-e's model; the encoder's own stub
cap (`policy_id_stubs.len() > u8::MAX` refused, `bytecode/encode.rs:28`)
reaches **21 chunks**. So two colliding cards at the measured legitimate size
sit *exactly* on the ceiling — one more stub (N = 139 → 13 chunks) and they
refuse — and three colliding 8-chunk cards refuse outright, while §"Rulings"
quotes AP3 as "at least 3 colliding cards supported".

**The gate cannot catch it.** Row 3 — "3-card collision → seats (AP3 floor)"
— is naturally built from 2-chunk cards (`3^2 = 9`), passes, and says nothing
about the input that falsifies the ruling.

**Remedy:** put the chunk bound on AP3's face in §"Rulings" and §2.3 ("3 cards
for n ≤ 7; beyond that the budget refuses, by design"), **or** raise the
ceiling with the measurement r1 asked for; and add a row at the `n` where the
budget first bites, so the boundary is pinned rather than discovered.

## NEW-I4 (Important) — §2's step order and §3's outcome order disagree, and `k` is undefined for the input that separates them.

§2 numbers the exact-count invariant **2** and cap/budget **3**. §3 is titled
"Outcomes (in order)" and puts them the other way:

> "- **cap / budget exceeded** → refusals per §2.3.
>  - **no complete partition** (incl. unequal counts, any class incomplete) → …"

`k` is defined only as the *common* per-index count ("requires EVERY index …
to hold the same piece count k"). §2.3 then says "**k > 5 in any class** → cap
refusal", a phrase that presupposes `k` exists.

**Failing input.** One id group, one total-class with `n = 2`: **7 pieces at
index 0, 1 piece at index 1**.

- Under §2's order: unequal counts ⇒ no complete partition ⇒ arm 1, zero
  decodes, the shipped merged-cards message.
- Under §3's order: the cap arm is evaluated first, `k` does not exist, and
  the implementer falls back to the max-per-index **proxy** r1 I7 explicitly
  rejected (7 > 5) — emitting "more than 5 cards share this stamped chunk-set
  id; auto-separation caps at 5" over an input in which nothing measured a
  card count at all. Seven strings at one index is equally well one card
  transcribed seven times.

That is the W15 / AP1 "never assert more than measured" failure r1 filed as
C1, re-entering through the cap clause instead of the AP2 message.

**Remedy:** one order, stated once. Make §3 read: exact-count first (unequal ⇒
arm 1, zero decodes) → cap on the now-well-defined `k` → budget → partition
outcomes; and drop "in any class" from §2.3, which is what invites the
per-index-maximum reading.

## NEW-M1 (Minor) — row 4's construction is under-specified, and the obvious build lands in row 5's outcome.

The over-budget row needs a class with **exactly** `k` pieces at *every*
index; anything else short-circuits at §2.2 into the zero-decode arm-1
refusal, which is **row 5**, not row 4. r1's "160 strings declaring
total_chunks = 32" is over-budget only if it is 5 × 32 with equal counts. Two
further constraints the row does not mention: a 32-chunk card cannot be minted
by `mk encode` (255 stubs = 21 chunks, N-e), so the fixture needs a
`split_into_chunks` + `encode_5bit_to_string` helper; and per NEW-I1 the five
cards must carry **distinct** stub lists, or their leading chunks collapse and
the row silently becomes a different row.

## NEW-M2 (Minor) — the `#<k>` grammar's own refusals are undefined.

§4 says only that `@i=<id>#<k>` "is accepted and resolves exactly one collided
card". `directive::parse` today refuses everything else with two distinct,
carefully argued messages including the measured prefix-collision rationale
(`directive.rs:41-43, 60-73`). Undefined after §4: `#<k>` naming an id that
resolves to a single non-collided card; `k` out of range; `k = 0`; `#` with no
digits; and whether the five-digit / never-a-prefix rule still binds the id
half. This module's whole discipline is that every refusal is spelled out.

## NEW-M3 (Minor) — "extending the shipped case-fold dedupe" points at a function shared with the md1 path.

§1: "extending the shipped case-fold dedupe to BCH-corrected identity —
enumerated churn to pipeline step 1", and row 10 extends the dedupe doc at
`input.rs:106-121`. The shipped case-fold dedupe is `input::dedupe_strings`,
and `seat::run` calls it on the **md1 phrases** as well
(`seat/mod.rs:127`: `input::dedupe_strings(&crate::cmd::strip_md1_inputs(req.phrases))`).
Canonicalisation decodes with `mk_codec::decode_string`, which cannot run on
md1 input. Say that canonicalisation is a separate, mk1-only stage *after*
step 1 rather than an extension of that function.

## NEW-M4 (Minor) — "decoded fragment bytes" is not a value mk-codec hands md-cli, and producing it adds an unrouted failure.

The accessible route (N-h) is `decode_string(s)?.data()` →
`StringLayerHeader::from_5bit_symbols` (whose `consumed` is discarded today at
`input.rs:171`) → `five_bit_to_bytes(&data[consumed..])`. That last call
returns `None` on bad payload padding — `pipeline.rs:119` maps it to
`Error::MalformedPayloadPadding` — and is reachable by a crafted string with a
valid BCH checksum, today landing in `terminal_refusal`. §1 defines no outcome
for a piece that fails canonicalisation. Keying on the **5-bit symbol tail**
is exactly as discriminating and introduces no new failure mode; if the byte
form is kept, route its failure explicitly. (Also unstated, and free to fix:
which of two collapsing strings survives — the shipped rule is first
appearance wins, `input.rs:119-122`.)

## NEW-M5 (Minor) — "ascending canonical bytecode" is ambiguous, and §2.5 makes the ambiguity reachable.

§4's order key could be the **reassembled stream** or
`mk_codec::bytecode::encode_bytecode(&card)` — the canonical re-encode this
file already uses as an operand (`input.rs:377-380`). They differ whenever a
bytecode is non-canonical: the path field has both a standard-table indicator
and an explicit `0xFE` escape for the same path (`bytecode/encode.rs`,
`path.rs`), so two distinct bytecodes can decode to one `KeyCard`. §2.5's
"two partitions are THE SAME iff they yield the same multiset of decoded
cards" makes that pair a single partition — at which point the ordinal, and
with it `#<k>`, `label()` and the tie-break, has no defined value. Name the
function.

## NEW-M6 (Minor) — the AP1 note's string count is ambiguous after a collapse.

The note opens "these 4 strings carried one stamped chunk-set id". After §1 it
is unstated whether the count is canonical pieces or lines the operator
supplied: an operator who pasted 5 lines is told "these 4 strings" and has to
guess which four. Same rule as M2 — say what was measured.

## NEW-N1 (Nit) — ordinal stability across input variations is unstated.

`#<k>` is stable across runs on identical input, and unaffected by cards in
other id groups. It is **not** stable under removing a collided card: with one
card left the group is not a collision, the label reverts to bare `12345`, and
a `--seat '@0=12345#2'` line copied from an earlier refusal stops resolving.
One sentence in §4 covers it.

## NEW-N2 (Nit) — two small inaccuracies.

- §3 says arm 1 "is **re-entered**", but the pre-pass runs before
  classification, so arm 1 was never entered. "Entered, with its full shipped
  predicate set" says the same thing without implying a second visit.
- The V-COLLIDE doc invariant is cited as `input.rs:489-492` (inherited from
  r1); at `54ab1cd6` that comment is at **490-493** (`input.rs:489` is the
  `fn` line).

---

# Gate-can-fail summary

| Acceptance row | Constructible? | Can it fail? |
| --- | --- | --- |
| 1 canonical 2×2 + unpinned control | yes (control fixed per I4) | yes |
| 2 BCH-twin collapses | yes (r1 M-b) | yes — mutation row 9 drives it |
| 3 3-card seats / 6-card cap | yes at small `n` | yes — but **passes while AP3 is only conditionally honoured** (NEW-I3), and the cap arm's order is undefined (NEW-I4) |
| 4 over-budget | **under-specified** (NEW-M1) | **outcome is implementation-dependent** (NEW-I2) |
| 5 unequal counts + classification order | yes — the shipped `44444` fixture | yes; verified it still lands in arm 1 |
| 6 mixed totals / one class incomplete | yes — `v-collide.txt` is exactly it | yes |
| 7 AP2 ambiguity | **construction still unspecified**; cost understated ~2× | at risk of never running (I6) |
| 8 permutation + `--seat` | yes | yes; `#<k>` refusal cases undefined (NEW-M2) |
| 9 mutation gates | well-formed | the force-accept gate inherits row 7 |
| 10 enumerated churn | — | **contains a false statement** (I5, N-f) |
| 11 suite green | yes | yes |
| **surplus / leftover** | **MISSING** — cited by §4 and by the security section, absent from the list (I2) | — |

# Recommended order of fold

1. **NEW-I1** first — it decides whether §2.2's invariant is exact or
   approximate, and everything about the budget and the cap sits on top of it.
2. **NEW-I2 + NEW-I3 + NEW-I4** as one edit: they are all §2.3/§3, and the
   structural pre-computed trigger from NEW-I2 is also what makes NEW-I4's
   ordering statable and NEW-I3's boundary row constructible.
3. **I2 + I5** as one edit: the missing surplus row and the mis-stated
   `v_collide_reaches_the_command` rewrite are the same test.
4. **I6** — drop the corrected construction and cost into row 7.
5. Minors/Nits last; they are wording and citation, and none of them moves an
   outcome.

No Critical remains. The structural revision landed the five r1 Criticals
correctly; what is open is one unsound assumption inside the new
canonicalisation (NEW-I1), three defects in the new budget/order machinery,
and three r1 Importants whose *gates* — not whose text — are still missing.
