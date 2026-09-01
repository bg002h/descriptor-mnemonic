# R0 — `design/SPEC_seat_auto_partition.md` @ `9deebb47`, round 3

**Questions asked:** (1) does the fold at `9deebb47` discharge each r2 finding
(7I/6M/2N)? (2) did the two semantics changes — SHARED canonical pieces
(reuse, `k :=` max distinct per-index count, cover) and the STATIC saturating
budget with AP3 floors — introduce new defects?

**VERDICT: 0 Critical / 5 Important / 6 Minor / 1 Nit — NOT GREEN.**

Every r2 finding is discharged: 3 partial Importants (I2, I5, I6) and all 4
new Importants (NEW-I1..I4), all 6 Minors and both Nits. Two carry residues
filed below as Minors. The 5 Importants are **new**: four in the machinery the
fold added (§2.2 cover semantics, §2.4's constant, row 4's construction, the
Security section), one pre-existing (§2.3's per-class cap) that r1 and r2 both
missed.

Reviewed against descriptor-mnemonic spec `9deebb47` over code `54ab1cd6`
(`origin/main`) and vendored `mk-codec 0.5.0`. Not re-litigated per brief:
AP1–AP3 rulings, and the r1 folds r2 verified FIXED (canonicalisation as
precondition, arm-1 entry, fail-closed composition, `#k` identity, the oracle,
AP2 wording truth post-§1).

## What was MEASURED for this round (not inferred)

Harness: a scratch crate with a path dependency on `vendor/mk-codec` (0.5.0).
Cards are `KeyCard::new(stubs, Some(fp), m/48'/0'/0'/2', xpub)` with xpubs
derived from distinct `Xpriv::new_master` seeds and
`encode_with_chunk_set_id(&card, 12345)`. Fingerprints differ in the FIRST
byte, so any chunk-0 sharing is forced by the stub-list layout alone.

| # | Measurement | Result |
| --- | --- | --- |
| P-a | Cost of the spec's stated oracle: all `3^12 = 531,441` candidates verified with `mk_codec::decode(&[&str])`, 3 cards × 12 chunks | **4.17 s** (release, 7.845 µs/cand); **5.21 / 5.33 / 5.68 s** on the repo's `opt-level = 2` test profile (9.8–10.7 µs/cand). 3 verified. |
| P-b | Cost of a two-stage oracle (each distinct piece parsed once at §1; per candidate = concat + SHA-256 + 4-byte compare) over the same 531,441 | **0.165 s** (release, 0.310 µs/cand); 0.18–0.20 s on the test profile. Same 3 verified. **25× cheaper.** |
| P-c | `decode_string` (BCH) alone | 0.48 µs (release) / 0.63 µs (test profile) — ×12 strings = 5.8 µs of P-a's 7.845 µs, i.e. the literal oracle's cost is ~75 % re-parsing pieces it already parsed |
| P-d | Is the cheap oracle reachable from md-cli? | **No.** `chunk::ChunkFragment` is `#[non_exhaustive]` with **no constructor** (`grep -n "impl ChunkFragment"` → nothing); building one outside mk-codec is `error[E0639]: cannot create non-exhaustive struct using struct expression` — measured, it is the first compile error the harness hit. `reassemble_from_chunks` is therefore dead API for md-cli. |
| P-e | 3 cards at n = 12 (128 stubs), **shared** stub list | per-index counts `[1,1,1,1,1,1,1,1,1,3,3,3]`, **product = 27**, 18 distinct pieces |
| P-f | 3 cards at n = 12 (128 stubs), **distinct** stub lists | per-index counts `[3]*12`, **product = 531,441**, 36 distinct pieces |
| P-g | Row 3's construction (13 shared stubs, 2 cards) | n = 3, counts `[1,2,2]`, k = 2, 4 candidates, **2 verified**, **exactly 1** exact-k cover → seats via reuse. Same shape at 3 cards: counts `[1,3,3]`, k = 3, 3 verified, 1 cover. |
| P-h | Verified-candidate count vs k, over all 5 constructions above | `verified_candidates == k` and `covers == 1` in 5 of 5 — honest input never produces a verified candidate outside the cover |
| P-i | Card size model (`mk-codec` encoder, fp present + std path), N = 1..45 | `bytecode_len = 80 + 4N` (N=1 → 84, N=45 → 260); n = ceil((len+4)/53). N=128 → 12 chunks; N=255 → 21 chunks (arithmetic from the measured model, matches r2 N-e) |
| P-j | Composition of the LAST chunk by stub count | N=19 → n=4, last fragment is **1 byte** and it is pure cross-chunk hash; N=6 → 2 bytes pure hash; N=33 → 4 bytes pure hash; otherwise the last chunk carries 1–49 bytecode bytes + 4 hash bytes |
| P-k | Where the 73-byte compact xpub lands | it plus the 4-byte hash is 77 bytes, so it spans 2–3 chunks and **some index always carries ≥25 xpub bytes** (at N=19/n=4, chunk 2 carries 50) |
| P-l | Cited code lines at `54ab1cd6` | `matching.rs:216-221` = the `REMEDIES` const ✓; `directive.rs:23-26` = the "A5's ambiguous id case is UNREACHABLE" invariant ✓; `input.rs:12-20` = the id-collision-is-fatal invariant ✓; `input.rs:490-493` = V-COLLIDE's NOTE (r2 NEW-N2's correction) ✓; `dedupe_strings` at `input.rs:123`, doc 104–122, "first appearance wins" at 119–122 ✓ |
| P-m | `decode_cards` call sites at `54ab1cd6` | **1 production** (`seat/mod.rs:143`) + **22 test** (`input.rs` ×13, `complete.rs` ×3, `matching.rs` ×3, `disposition.rs` ×2, `satisfy.rs` ×1) |
| P-n | `mk_codec::bytecode::encode_bytecode(&card)` exists and is callable externally | yes — called in the harness, returns `Result<Vec<u8>>` |
| P-o | An id group with TWO total-classes is a shipped fixture | `tests/fixtures/seating/v-collide.txt`: a 2-chunk card and a 3-chunk card, both pinned to id 12345 |

---

# PART 1 — Per-finding disposition

## r2 I2 (surplus row restored + distinguishable assertion) → **FIXED** (residue r3-M1)

Row 10 exists and carries the assertion r2 demanded:

> "**Surplus/leftover row (r1 I2, restored):** injected extra valid card →
> partition succeeds, completeness refuses downstream, and the two leftover
> lines are DISTINGUISHABLE (`#<k>` labels differ)."

It is constructible from the very input r2 measured (N-f/N-g): row 12 hands
`v_collide_reaches_the_command`'s shipped input to row 10 as "the end-to-end
variant". Under the new semantics that input's two collided cards sit in two
different total-classes (P-o), both classes complete, both seat, and 4 cards
against 2 slots reaches the leftover refusal — which is exactly what row 10
asserts. Failable: today both cards label `12345 (stub 5b48af35)`.

Residue: the Security section still points at the wrong row (r3-M1).

## r2 I5 (enumerated churn) → **FIXED**, tier dropped; residue r3-M2 (Minor)

(a) The false entry is corrected, with r2's measurement folded into the text:

> "`v_collide_reaches_the_command` REWRITTEN **with a new minimal 2-slot
> fixture** (its shipped input carries a full extra card set and would hit the
> leftover refusal, not seat — r2 I5; the old input becomes row 10's
> end-to-end variant)."

(b) Two of the three unlisted items are now listed and both citations verify
(P-l): `REMEDIES` at `matching.rs:216-221` and `directive::parse`. The third —
the `decode_cards` signature change that §3's note-as-a-VALUE requires, and its
call sites — is still absent (r3-M2).

**Tier call.** The brief says unfixed r2 items keep tier; this one is filed
down to Minor deliberately, because the Important half of r2 I5 was a *false
statement* in the artifact ("asserts the new seat+note outcome end-to-end"),
and that is fixed. What remains is a compile-forced signature change: it cannot
silently retire a guarantee, which was r2's stated reason for the tier. The
four retired *assertions* all have named inheriting rows.

## r2 I6 (grind construction + corrected cost) → **FIXED by reference**; see r3-I4

Row 9 replaces the circular "feasibility demonstrated by the script" with a
pointer to a construction that exists on disk:

> "AP2 ambiguity: COMMITTED fixture from a committed two-stage-grind script
> (r2 I6's corrected construction + regeneration doc; a BCH twin is NOT a valid
> fixture — it must seat per row 2)"

A pointer into a committed review report is a resolvable specification, and
r2's derivation (fix `f0a, S_a`; grind `f0b`; then grind `S_b`) is byte-exact
on disk. What the fold could not know is that its own §2.5 change makes that
construction non-minimal — **r3-I4**.

## r2 NEW-I1 (shared pieces collapse the feature) → **FIXED via remedy (a)**; cost filed as r3-I3

§2.2 admits reuse and says so with the measurement attached:

> "Indexes holding fewer than k pieces are ADMISSIBLE: two different cards can
> legitimately share a byte-identical piece (r2 NEW-I1, measured — ≥13 shared
> policy stubs make chunk 0 a pure function of the stub list), so a candidate
> may REUSE a piece"

The measured claim is **true as stated**: chunk 0 is a pure function of the
header + stub list exactly when `2 + 4N ≥ 53`, i.e. N ≥ 13. (Sharing is also
reachable at N = 12 when two fingerprints agree in their first three bytes —
measured — but that is a coincidence of key material, not the layout-forced
threshold the spec claims, so the ≥13 wording is right.)

Row 3 lands and **works**: measured P-g, 13 shared stubs × 2 cards gives counts
`[1,2,2]`, k = 2, 2 of 4 candidates verify, and there is exactly **one**
exact-k cover — it seats. The row is also failable: under r2-era exactly-once
semantics index 0 holds one piece for two cards, so the same input refuses.

What the fold did not carry is the *soundness argument* for using k as an exact
card count (r3-M4) and the case where that use is wrong (r3-I3).

## r2 NEW-I2 (over-budget outcome not a function of the input) → **FIXED**

§2.4 is now static, pre-decode, saturating, and — the part that actually closes
the finding — explicitly decouples the outcome from the strategy:

> "The refusal is a function of the headers alone — implementations may decode
> fewer candidates via filters, but the over-budget outcome is decided by this
> product."

Two conforming implementations can no longer disagree. `5^32` overflowing u64
is named with the right reason (this repo's profile keeps `debug_assertions`,
so an unchecked `pow` panics). Residues: r3-M3 ("headers alone" is not true of
the counts), r3-M5 (the product bounds stage 1 only), and the constant's own
bounds — r3-I1.

## r2 NEW-I3 (AP3 silently narrowed) → **FIXED on the ruling's face**; see r3-I2, r3-N1

AP3 now carries its own boundary:

> "**Honoured within a computation budget, per the ruling's own 'if it takes
> too long':** guaranteed for every measured legitimately-mintable size
> (3 cards to n = 12 chunks — the 128-stub mint measured in r1); beyond the
> budget the tool refuses BY DESIGN, naming the boundary"

and the floor is arithmetically right: `3^12 = 531,441` ✓, and the boundary row
was added. r2 asked for the bound on the face and for a boundary row; it got
both. The floor is unsatisfiable against the same paragraph's ceiling (r3-I1),
and the row meant to gate it does not (r3-I2).

## r2 NEW-I4 (order disagreement + `k` undefined) → **FIXED**

One order, stated once: §2 is numbered 1→6 and §3's header is "the §2 order IS
the outcome order — evaluated 1→6, first refusal wins". `k` is now total
(max per-index distinct count), so the cap clause can never be evaluated on an
undefined quantity, and the wording claims only what was measured:

> "these pieces (chunks) would need more than 5 key cards to explain;
> auto-separation caps at 5"

Checked the honesty of that claim independently: a card holds exactly one piece
per index, so m distinct pieces at one index imply ≥ m cards — the lower bound
is real, and r2's failing input (7 pieces at index 0, 1 at index 1) now yields
k = 7 and a true statement. "in any class" became "in a class", which is what
makes the pre-existing per-class scoping defect visible — r3-I5.

## Minor / Nit carry-over

| r2 | Disposition | Evidence |
| --- | --- | --- |
| NEW-M1 (row 4 under-specified) | **FIXED for the over-budget row** — now row 5, with "equal per-index counts k=5", a committed synthetic chunker, and "distinct stub lists so leading chunks do NOT collapse". **The identical defect now sits in row 4** → r3-I2 |
| NEW-M2 (`#<k>` refusals) | **FIXED** — all five of r2's cases enumerated in §4 (non-collided id, k = 0, k out of range, `#` without digits, and the five-digit/never-a-prefix rule binding the id half). Verified the last one is load-bearing: `directive::parse` rejects `12345#1` at `is_ascii_hexdigit`, so the id half must be split before the hex check |
| NEW-M3 (dedupe is shared with md1) | **FIXED** — §1 is titled "mk1-only stage, AFTER the shipped step-1 dedupe"; row 12 documents it "adjacent to (not inside) `dedupe_strings`" |
| NEW-M4 (decoded bytes add a failure route) | **FIXED** — the key is the "5-bit payload symbol tail", `five_bit_to_bytes` is never called, and "First appearance survives a collapse (the shipped rule)" matches `input.rs:119-122`. Noted, not filed: the symbol tail is strictly finer than the byte key for non-canonically-padded payloads, so such a string stays a distinct piece and raises that index's count — it then fails candidate verification, and if it pushes k past 5 the cap message is still true |
| NEW-M5 (order key ambiguous) | **FIXED** — `mk_codec::bytecode::encode_bytecode(&card)` named, and it exists and is externally callable (P-n). It returns `Result`; on a card that came out of `decode` the error arm is unreachable, so no outcome is missing |
| NEW-M6 (note's string count) | **FIXED** — "these 5 supplied strings are 4 distinct pieces (chunks)" says what was measured |
| NEW-N1 (ordinal stability) | **FIXED** — §4 covers identical input, other groups, and dissolution (stale `#<k>` gets the no-such-card refusal listing current labels) |
| NEW-N2 (entered / cite) | **FIXED** — "entered", not re-entered; `input.rs:490-493` verified correct at `54ab1cd6` (P-l) |

---

# PART 2 — New findings

## r3-I1 (Important) — §2.4's floor and its ceiling cannot both be satisfied under the spec's own oracle, and the oracle that would satisfy both is unreachable from md-cli. MEASURED.

**The claim under attack**, §2.4:

> "`PARTITION_DECODE_BOUND` is fixed at implementation from measured decode
> timing, with spec floors/ceilings: **≥ 531,441** (3 cards at n = 12, AP3's
> floor at the largest measured legitimate mint) and small enough to keep the
> worst case under ~2 s."

and §2.5's verifier:

> "it VERIFIES iff `mk_codec::decode` accepts it (the 4-byte cross-chunk hash
> is the oracle)"

**Measured (P-a).** 531,441 candidates through `mk_codec::decode` take
**4.17 s** in release and **5.2–5.7 s** at the repo's `opt-level = 2` test
profile, single-threaded. The largest bound that keeps the worst case under
~2 s is therefore ≈ **255,000** — *below* the mandated floor of 531,441. **No
value of `PARTITION_DECODE_BOUND` satisfies both stated constraints**, so the
constant the spec defers to implementation cannot be chosen.

**Why (P-c).** `mk_codec::decode` takes `&[&str]`, so it re-runs `decode_string`
(BCH, with t = 4 correction) on all 12 strings of every candidate: 12 × 0.48 µs
= 5.8 µs of the 7.845 µs. The pieces were already parsed once by §1.

**The fix that works is currently out of reach (P-b, P-d).** A two-stage
oracle — parse each distinct piece once at §1, then per candidate concatenate
fragments, SHA-256 the head and compare the 4-byte tail, calling
`mk_codec::decode` only on survivors — costs **0.310 µs/candidate**, i.e.
**0.165 s** for the same 531,441 and 25× headroom. But mk-codec's entry point
for it, `reassemble_from_chunks`, takes `Vec<ChunkFragment>`, and
`ChunkFragment` is `#[non_exhaustive]` with **no constructor**: building one
outside the crate is `E0639` (measured). Since "Out of scope" names
`mk-codec/md-codec`, the only in-scope route is md-cli **reimplementing the
cross-chunk-hash rule locally** — a normative codec rule, in a repo whose
codec-ownership discipline exists to prevent exactly that. The spec neither
names the two-stage oracle nor authorises the reimplementation.

**Remedy (one of):** (a) name the two-stage oracle in §2.5, state the measured
µs/candidate, and derive the bound from it — and say explicitly that the
prefilter re-expresses `chunk.rs`'s trailing-hash rule and is exempt from the
out-of-scope line; (b) keep the literal oracle and retract the floor, moving
AP3's face to the n it can actually reach (~255 k ⇒ 3 cards to n = 11, since
`3^11 = 177,147` and `3^12 = 531,441`); (c) raise the ceiling to the honest
number and say so on AP3's face. What is not acceptable is the present text,
which mandates a constant that does not exist.

## r3-I2 (Important) — row 4's AP3-floor construction is under-specified, and the obvious build never reaches the budget at all. MEASURED.

Row 4:

> "3-card collision at n = 12 (AP3 floor at the measured legitimate maximum) →
> seats within budget."

Measured both ways (P-e/P-f): three 128-stub cards built the obvious way — one
wallet's cards, so one **shared** stub list — give per-index counts
`[1,1,1,1,1,1,1,1,1,3,3,3]` and a product of **27**. Only with **distinct**
stub lists does the same shape reach `[3]*12` and 531,441.

So row 4 as written passes for any `PARTITION_DECODE_BOUND ≥ 27`, including
values that violate the floor it exists to gate — the AP3 floor has no gate
that can fail. This is r2 NEW-M1 one row over: the fold added exactly this
qualifier to row 5 ("distinct stub lists so leading chunks do NOT collapse")
and not to row 4, where the floor lives.

Filed Important rather than Minor because the defect is a gate that cannot
fail for the reason it exists, which is the shape r2 NEW-I3 was filed at
Important for and the fold accepted.

**Remedy:** say "distinct stub lists" in row 4 (measured product 531,441), and
state the boundary sub-row's `n` as a function of the chosen constant — at a
floor of exactly `3^12` the first refusing size is n = 13, which is mintable
(N = 139, P-i).

## r3-I3 (Important) — "exactly k … whose pieces COVER" lets a unique cover OMIT a supplied, verified card: a scanned card is silently dropped, and the AP1 note then asserts a card count nobody measured.

**The claims under attack**, §2.2 and §2.5:

> "k is the number of cards a cover must contain, and any index's
> distinct-piece count is a measured LOWER bound on cards"

> "a set of exactly k distinct verified cards whose pieces COVER every
> canonical piece of the class (each piece used by ≥ 1 card) … no partial
> seating, no dropped pieces, ever"

The spec states k as a **lower** bound and then uses it as the **exact** card
count. Where the truth has k+1 cards and the extra one is *piece-wise
dominated* — its piece at every index is also carried by one of the k — the
k-card cover exists, covers every piece, is unique, and seats. The extra card
vanishes with no diagnostic.

**Concrete construction** (mint-time, not remote — see the cost note below).
Cards A, B, C pinned to one id, sharing xpub, fingerprint and path, differing
only in their policy-id stub lists — a legitimate shape, one key serving
several policies. Take N = 19 stubs, so n = 4 and the measured layout (P-j) is:
chunk 0 = header + stubs, chunk 1 = the rest of the stubs + fp + path + the
first xpub bytes, chunk 2 = the remaining xpub + hash[0..3], chunk 3 = hash[3].

- `A = (a0, a1, a2, a3)`, `B = (b0, b1, b2, b3)`
- `C`'s stub list is **A's chunk-0 stubs followed by B's chunk-1 stubs** — a
  well-formed, mintable list — so `c0 = a0` and `c1 = b1` by construction, and
  `c2/c3` (shared xpub tail + C's own 4-byte cross-chunk hash) equal `b2/b3`
  iff `H(C)[0..4] == H(B)[0..4]`.
- That last condition is a **cross-prefix collision on a shared tail**: choose
  the chunk-1 stub bytes `T` such that `H(A_head‖T‖rest)` and `H(B_head‖T‖rest)`
  agree in four bytes. **One ~2^32 search**, two SHA-256 per trial, performed by
  whoever fixes the stub lists at mint time.

Pieces are then exactly A's ∪ B's: counts `[2,2,2,2]`, **k = 2**. The exact-k
covers are `{A,B}` ✓ (all pieces used), `{A,C}` ✗ (misses `b0`), `{B,C}` ✗
(misses `a1`). Unique ⇒ **seats A and B**, drops C — which is itself a verified
candidate, the tuple `(a0,b1,b2,b3)` — and emits

> "these N supplied strings are M distinct pieces (chunks) … and they are **2
> different key cards**"

over three plates the operator scanned. Both guards read as satisfied: "no
dropped pieces, ever" is true at the *piece* level, and the note's count is
false in exactly the way r2 NEW-I4 and r1 C1 policed — asserting more than was
measured.

The Security section's "outcome is refusal (service degraded), **never wallet
selection**" is also false here: the tool selected `{A,B}` out of `{A,B,C}`.

**Reachability, stated plainly.** This is not accidentally reachable — a
4-byte cross-chunk hash agreement is 2^-32 — and it is not a remote attack
either: the collision has to be arranged when the stub lists are chosen. The
finding is that §2.5 **fails open** where the count assumption breaks, and that
nothing in the spec says the assumption exists. Every other failure in this
document refuses; this one seats and prints a count that was never measured.
The tier is Important, not Critical, only because of the key-material bound
below — which the spec does not state either.

**What bounds the damage — and is absent from the spec.** The dropped card can
never carry a distinct xpub. Measured (P-k): the 73-byte compact xpub plus the
4-byte hash is 77 bytes, so it spans 2–3 chunks and *some* index always carries
≥25 xpub bytes (50 at N = 19). Two distinct keys cannot coincide there at any
feasible cost, so a card with a key nobody else has always contributes a piece
no other card can cover, and the cover fails ⇒ the group refuses. **Every
distinct key present is seated or the whole group refuses** — that is the
property that keeps this Important rather than Critical, and the spec says it
nowhere (see r3-M4).

**Remedy (one option, stated as a defect-closing shape, not a prescription):**
require the cover to *be* the verified set — `|V| = k` **and** V covers every
piece. Honest input always satisfies it (P-h: `verified_candidates == k` and
exactly 1 cover in 5 of 5 constructions, including both shared-piece rows), it
refuses the dominated card instead of dropping it, and it deletes the
unbounded k-subset search of r3-M5 outright.

## r3-I4 (Important) — the Security section was not re-derived against the cover semantics: ambiguity now costs ONE ~2^32 grind, not "two sequential" ones.

> "Post-§1, ambiguity requires construction: **two sequential ~2^32 grinds**
> (r2's corrected derivation — one against the victim's trailing hash, one for
> the cross-prefix collision), each constrained to valid KeyCard bytecode."

r2's derivation was correct **under exactly-once matching**, where a second
cover had to be a second perfect matching and therefore needed the complementary
pairing to verify too. §2.5's cover-with-reuse removes that: a second cover
need only *cover*, and a third card can absorb the leftovers.

**Construction (one grind).** One n = 3 class: cards A and B share a 13-stub
chunk 0 (P-g), card C carries a different stub list. Counts `[2,3,3]`, k = 3.
`{A,B,C}` covers. Grind a single frankencard `F = (C0, A1, A2)` — 2^32 over
stub bytes the attacker controls, to match A's trailing hash — and `{F,B,C}`
also covers all eight pieces (index 0: `C0`,`P0`,`C0`; indexes 1 and 2: all
three pieces each). Two distinct decoded-card multisets ⇒ AP2. Under the
pre-fold rule the same input was rejected at step 2 with **zero** decodes,
because the per-index counts were unequal.

The error is in the unsafe direction (it understates attacker reach), and it
propagates: row 9's fixture script is specified by pointing at r2's two-stage
construction, which is no longer the minimal one. Separately, the parenthetical
mis-describes what it cites — r2's two grinds are *both* cross-card 4-byte
collisions; neither is "against the victim's trailing hash", which is r1-era
wording.

**Remedy:** re-derive the section against §2.5 as folded, state the one-grind
cost, and let row 9's script build the cheaper fixture (which is also easier to
regenerate).

## r3-I5 (Important, PRE-EXISTING — survived r1 and r2) — AP3's "hard cap 5" is enforced per total-class, so one stamped id can seat 5 cards per class.

§2.3: "**Cap (AP3), on the now-well-defined k:** k > 5 in a class ⇒ cap
refusal", and k is defined per class in §2.2 ("In a class with total n …
define k").

AP3 caps *colliding cards* — cards sharing one stamped chunk-set id. The
mechanism caps them per total-class. An id group with two classes is not
hypothetical: `v-collide.txt` is exactly that shape and is shipped (P-o), and
row 7 requires it. Five 2-chunk cards plus five 3-chunk cards under one pinned
id give k = 5 in each class, no cap refusal, and **10 cards seated** under a
"hard cap 5". Totals run 2..32, so the group-level card count is bounded only
by the budget.

Row 4's "6-card → cap refusal" is naturally built inside one class and passes
while the mixed-totals shape violates the ruling — the same "gate passes while
the ruling is only conditionally honoured" pattern r2 filed as NEW-I3.

**Remedy:** cap on `Σ_classes k` (one word in §2.3), or put "the cap is per
total-class" on AP3's face with the reason, so the operator's hard cap means
what it says or is visibly narrowed.

## r3-M1 (Minor) — the Security section cites the wrong acceptance row.

> "Surplus valid card: partitions cleanly, completeness refuses downstream with
> DISTINGUISHABLE `#<k>` labels (row 12)."

The surplus/leftover row is **row 10**; row 12 is the enumerated churn. The
fold inserted row 10 and renumbered everything after it. All other row
citations check out (§1→row 2, §4→row 8, §5→row 1, row 11's four mutation
targets). r2's I2 was a row cited twice and never written; this is the same
citation class one fold later, and cheap to prevent by citing rows by name.

## r3-M2 (Minor) — the churn enumeration still omits the `decode_cards` signature change and its call sites.

§3 makes the AP1 note "a VALUE carried into `Seating.notes`", which
`decode_cards -> Result<Vec<DecodedCard>, CliError>` cannot return. Row 12
lists `REMEDIES`, `directive::parse`, three doc invariants and the §1 stage —
not this. Measured (P-m): **1 production call site** (`seat/mod.rs:143`) and
**22 test call sites** (`input.rs` ×13, `complete.rs` ×3, `matching.rs` ×3,
`disposition.rs` ×2, `satisfy.rs` ×1). r2's "8 test call sites in
`complete.rs`/`disposition.rs`/`input.rs`" undercounts by 14, so folding r2's
number verbatim would have imported a wrong count — measure it, then quote it.

## r3-M3 (Minor) — "a function of the headers alone" is not true of the quantity §2.4 sums.

The product is over `count_i` = **distinct canonical pieces** at index i, and
§1 decides identity on the 5-bit payload tail — not on the header. Two strings
with the same header and different payloads count as two; a BCH twin counts as
one. An implementer taking "headers alone" literally counts *strings* per
index, and a benign double transcription then multiplies the product by 2^n.
That misreading is ungated: row 2's product stays far below any candidate bound
either way. Say "a function of the canonical piece counts, computed before any
candidate decode".

## r3-M4 (Minor) — the soundness argument for using k as an exact count is missing, and it is the sentence that bounds r3-I3.

§2.2 justifies k only as a lower bound. What makes "exactly k" sound for honest
input is a mk-codec layout fact (P-k): the 73-byte compact xpub plus the 4-byte
cross-chunk hash spans 2–3 chunks, so at least one index carries ≥25 bytes of
key material and distinct keys cannot coincide there — therefore the maximum
per-index count equals the card count unless someone spends 2^32. State it,
with the measurement, as the anchor for "exactly k". Unstated, it is an
assumption that will decay silently if mk-codec's field order ever changes —
and mk-codec is out of scope here, so nothing else will notice.

## r3-M5 (Minor) — the static budget bounds candidate verification only; the cover search is unbounded.

§2.4 bounds `Σ_classes Π_indexes count_i` — stage 1. Deciding "exactly one
distinct card multiset" requires enumerating k-subsets of the V verified
candidates, `C(V,k)`, which nothing bounds. Honest input has V = k (P-h:
5 of 5), and each extra verified candidate costs an attacker ~2^32, so this is
not accidentally reachable; but the ruling the budget serves is "if it takes
too long", and row 5 asserts "no hang" over half the search. One clause fixes
it — or it disappears entirely under r3-I3's `|V| = k` remedy.

## r3-M6 (Minor) — row 11's budget mutation is not observable at row 5's minimal size.

Row 11: "skip the static budget → row 5 hangs/fails". Row 5 only requires "an n
with `5^n > PARTITION_DECODE_BOUND`". At the minimum such n the un-budgeted
enumeration is a **pause, not a hang** — at a bound near 5×10^5, n = 9 gives
`5^9 = 1.95M` candidates ≈ 15 s at the measured 7.845 µs (P-a), after which the
row still passes and the mutation gate reports nothing. r1's original input
(160 strings declaring `total_chunks = 32`, `5^32`) is unbounded in practice.
Pin the fixture's n at that end, or the mutation gate cannot fail.

## r3-N1 (Nit) — AP3's face gives the guaranteed size but not the mintable one.

> "guaranteed for every measured legitimately-mintable size (3 cards to n = 12
> chunks — the 128-stub mint measured in r1)"

"Every measured legitimately-mintable size" reads as "every mintable size", and
it is not: `mk encode`'s own stub cap (255) reaches **n = 21** (P-i), where 3
cards need `3^21 ≈ 1.05×10^10` and refuse by design. One clause — "mints up to
n = 21 are possible and refuse" — puts the gap on the face instead of leaving
it to be re-derived.

---

# Gate-can-fail summary

| Acceptance row | Constructible? | Can it fail? |
| --- | --- | --- |
| 1 canonical 2×2 + unpinned control | yes | yes |
| 2 BCH-twin collapses | yes | yes — mutation row 11 drives it |
| 3 shared-piece seats | **yes, MEASURED** (P-g: counts `[1,2,2]`, k=2, 2 verified, 1 cover) | yes — the same input refuses under exactly-once |
| 4 AP3 floor / cap / boundary | yes | **no, as written** — the obvious build has product 27 and gates no bound (r3-I2); the boundary sub-row's `n` moves with a constant that currently has no legal value (r3-I1) |
| 5 over-budget synthetic | yes (committed chunker; distinct stub lists stated) | yes for the refusal; **the row-11 mutation is unobservable at minimal n** (r3-M6) |
| 6 missing-index class | yes — shipped `44444` fixture | yes |
| 7 mixed totals | yes — `v-collide.txt` (P-o) | yes; it is also the shape that defeats the cap (r3-I5) |
| 8 permutation + `--seat` | yes | yes; all five grammar refusals now enumerated |
| 9 AP2 ambiguity | yes | yes — but the construction it points at is no longer minimal (r3-I4) |
| 10 surplus / leftover | yes — the old `v_collide_reaches_the_command` input | yes — labels are identical today (r2 N-g) |
| 11 mutation gates | well-formed | 3 of 4; the budget gate inherits r3-M6 |
| 12 enumerated churn | — | still omits `decode_cards` + 23 call sites (r3-M2) |
| 13 suite green | yes | yes |

# Recommended order of fold

1. **r3-I1** first — until the oracle is named, `PARTITION_DECODE_BOUND` has no
   legal value, and rows 4, 5 and 11 are all written against it.
2. **r3-I3 + r3-M4 + r3-M5** as one edit to §2.2/§2.5: they are the same
   sentence pair, and the `|V| = k` shape closes all three at once.
3. **r3-I4** — re-derive Security against §2.5 as folded; row 9's script spec
   follows from it.
4. **r3-I5** — one word in §2.3 (`Σ_classes k`) or one clause on AP3's face.
5. **r3-I2 + r3-M6** — both are acceptance-row constructions, both are
   "say which fixture", and both are downstream of r3-I1's constant.
6. Minors/Nit last: r3-M1 (row cite), r3-M2 (churn), r3-M3 (wording), r3-N1.

No Critical. The fold is a good one — all seven r2 Importants are genuinely
discharged, and the shared-piece semantics is measurably correct on the input
it was written for (P-g). What it did not do is re-derive the *arguments* that
the semantics change falsified: the budget constant (r3-I1), the exactness of
k (r3-I3), and the attacker cost (r3-I4) were all carried forward from a
document whose ground had moved underneath them.
