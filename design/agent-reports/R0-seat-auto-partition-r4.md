# R0 — `design/SPEC_seat_auto_partition.md` fold-check, round 4

**Questions asked:** (1) does the fold at `0d6ccf89` discharge each r3 finding
(5I/6M/1N)? (2) did the fold introduce new defects — particularly around the
ONE semantic change it made, the §2.5 seat condition (`|V| = k` AND V covers
every piece, no chosen cover, no subset search)?

**VERDICT: 0 Critical / 1 Important (NEW) / 0 Minor / 0 Nit — NOT GREEN.**

All 12 r3 findings (5I/6M/1N) are genuinely FIXED — not transcribed, checked
against the actual spec text at `0d6ccf89` and, where cited, against measured
facts in `R0-seat-auto-partition-r3.md`. One new Important surfaced from
row 10's restructuring: the churn note misattributes a shipped SAME-id fixture
to row 10's DIFFERENT-id sub-case, orphaning the specific assertion (r1
I2 → r2 I2 → r3's own FIXED disposition) that a same-id, mixed-totals surplus
produces a distinguishable leftover refusal.

Reviewed: `design/SPEC_seat_auto_partition.md` @ `0d6ccf89` (diff against
`9deebb47` via `git diff 9deebb47..0d6ccf89`), against
`design/agent-reports/R0-seat-auto-partition-r3.md`'s findings and measured
table (P-a..P-o). Not re-litigated per brief: everything r3 verified FIXED
from earlier rounds, and AP1–AP3 rulings' substance (only their text on the
page was checked for internal consistency, not re-derived).

---

# PART 1 — r3 finding dispositions

## r3-I1 (floor/ceiling unsatisfiable) → **FIXED**, option (b)

§2.4: "`PARTITION_DECODE_BOUND` is fixed at implementation from measured
timing to keep the worst case under ~2 s with the literal `mk_codec::decode`
oracle — ≈ 255,000 at the measured 7.845 µs (r3-I1 option (b): the floor of
531,441 is RETRACTED as unsatisfiable with this oracle; AP3's face above
states what the honest constant reaches)."

AP3's face: "3 cards guaranteed to n = 11 chunks (177,147 candidates ≈ 1.4 s),
5 cards to n = 7, 2 cards to n = 17". Arithmetic checked: 3^11 = 177,147 ✓,
5^7 = 78,125 ✓, 2^17 = 131,072 ✓ — all ≤ the ~255,000 bound. 3^12 = 531,441 >
255,000, consistent with n = 12 being pinned as the first refusing size
(boundary row, r3-I2). Out-of-scope now states: "mk-codec/md-codec (incl. the
two-stage oracle idea — rejected: it needs a mk-codec change or a local
reimplementation of the trailing-hash rule; the literal-oracle budget above is
the accepted cost)" — the two-stage oracle is explicitly rejected, not merely
omitted. Option (b) fully executed.

## r3-I2 (row 4 gate can't fail) → **FIXED**, jointly resolved with I1

Row 4: "**floor row:** 3 cards, n = 11, **DISTINCT stub lists** (r3-I2 — a
shared list collapses the product) → seats within budget (177,147 candidates
≈ 1.4 s measured). **boundary row:** the same shape at n = 12 (mintable,
N = 128 stubs) → budget refusal naming AP3's rationale — the first refusing
size, pinned as designed behaviour."

The remedy text in r3 assumed the floor stayed at 3^12; since r3-I1 retracted
it to ~255k instead, the floor row correctly moved to n = 11 and the boundary
to n = 12 — a coherent joint fix, not a literal transcription of the r3
remedy (remedies are advisory per brief). "DISTINCT stub lists" is now stated
on the row that needs the gate to be able to fail; the obvious build (r3's
measured product-27 shared-list construction) is explicitly excluded.

## r3-I3 (V=k seat condition) → **FIXED**, matches remedy verbatim

§2.5: "Let **V_class := the set of DISTINCT verified candidate cards**
(identity = decoded card). The class SEATS iff `|V_class| = k_class` AND
V_class's pieces cover every canonical piece of the class. ... There is no
chosen cover and no subset search (r3-M5 deleted): honest input yields
exactly the real cards (measured 5/5 constructions incl. both shared-piece
rows); a ground EXTRA verified card makes `|V| > k` ⇒ refusal (never a silent
drop, never a wrong seat); a dominated card cannot be omitted because V is
all of them."

This is exactly r3-I3's proposed remedy ("require the cover to *be* the
verified set — `|V| = k` and V covers every piece"), and it subsumes r3-M5
(no subset search) in the same stroke, as r3 anticipated ("or it disappears
entirely under r3-I3's `|V| = k` remedy").

## r3-I4 (one-grind security re-derivation) → **FIXED**

Security: "**Reaching the AP2 refusal costs ONE ~2^32 grind** (an extra
verified candidate constrained to valid KeyCard bytecode; r3's `[2,3,3]`
construction). Outcome: refusal — service degraded, never selection."

Row 9: "COMMITTED fixture from a committed **ONE-grind** script (~2^32 +
KeyCard-validity; regeneration documented; a BCH twin is NOT a valid fixture
— it must seat per the bch-twin row) → AP2 refusal, nothing seats." "two
sequential" / "two-stage-grind" no longer appear anywhere in the spec
(`grep -n -i "two sequential\|two-stage-grind"` → no hits). Row 9's fixture
spec now points at the cheaper one-grind construction as r3 required.

## r3-I5 (group-wide cap) → **FIXED**

§2.3: "**Cap (AP3), group-wide:** `Σ_classes k_class > 5` ⇒ cap refusal". AP3's
face: "hard cap 5 **on the whole id group** (r3-I5 — the cap is not per
total-class)." Row 7: "group cap applies across classes: a 3+3 two-class
group → cap refusal, r3-I5" — 3+3=6>5, arithmetically correct, and
constructible (two total-classes each independently mintable to 3 cards, per
the same mechanism `v-collide.txt` already demonstrates at count 1+1).

## r3-M1 (Security cited wrong row) → **FIXED**, matches the finding's own remedy

The Security section no longer cites a row number at all for the surplus
case. Instead, every acceptance row is now bold-named ("**canonical-collision
row**", "**shared-piece row**", "**floor row**"/"**boundary row**", "**ap2
row**", "**surplus row**", etc.), and the section header states: "## Acceptance
(vector rows; cited by name elsewhere — r3-M1)". This is precisely r3-M1's
own suggested fix ("cheap to prevent by citing rows by name"), applied
document-wide, not just at the one broken citation. Checked every remaining
row citation (§1 "(row 2)", §4 "(row 8)", §5 "(row 1)", row 11's four names,
row 12's names) — all resolve to the correct row. One residual issue found in
this sweep is filed below as **r4-I1** (a *content* mismatch, not a
row-number error).

## r3-M2 (decode_cards churn undercounted) → **FIXED**

§12: "`decode_cards` signature change (note-as-value): 1 production call
site (`seat/mod.rs:143`) + 22 test call sites (input.rs ×13, complete.rs ×3,
matching.rs ×3, disposition.rs ×2, satisfy.rs ×1 — measured, r3-M2)". Matches
r3's P-m measurement exactly (13+3+3+2+1 = 22).

## r3-M3 ("headers alone" wording) → **FIXED**

§2.4: "a function of the CANONICAL PIECE COUNTS (not raw strings, not headers
alone — r3-M3), evaluated before any decode". Matches the requested wording.

## r3-M4 (soundness of k as exact count) → **FIXED**

§2.2: "**Why k_class is EXACT for honest input (r3-M4, measured):** the
73-byte compact xpub + 4-byte trailing hash span 2–3 chunks, so at least one
index carries ≥ 25 bytes of key material where distinct keys cannot coincide;
violating "max count = card count" costs a ~2^32 grind, which §2.5 turns into
a refusal, never a wrong seat." Matches r3's P-k-grounded remedy.

## r3-M5 (unbounded cover search) → **FIXED**, and confirmed nothing reintroduces it

§2.5 explicitly: "There is no chosen cover and no subset search (r3-M5
deleted)". Swept the whole document for "subset", "C(V,k)", "cover search" —
no other reference to an enumeration over covers exists anywhere; §2.5's
"cover" is now simply `V_class` itself, no combinatorics.

## r3-M6 (row 11 mutation unobservable at minimal size) → **FIXED**

Row 5: "...This size is chosen so row 11's skip-the-budget mutation
observably hangs (5^32-scale), not pauses (r3-M6)." Explicitly pins the
fixture size to the extreme, as required.

## r3-N1 (AP3 face missing the mintable-but-refuses gap) → **FIXED**

AP3's face: "Mints up to n = 21 are possible (255-stub cap) and refuse by
design (r3-N1)." Matches verbatim.

---

# PART 2 — new-defect checks on the V=k semantics

## Benign extra verified candidate without a grind

Constructed exactly the dispatcher's shared-piece scenario (P0 shared,
A1/A2, B1/B2, candidate `(P0,A1,B2)`) against r3's own measurement, P-g: "n =
3, counts `[1,2,2]`, k = 2, **4 candidates, 2 verified**". The 4 candidates
are `(P0,A1,A2)`, `(P0,B1,B2)` (real) and `(P0,A1,B2)`, `(P0,B1,A2)` (cross);
only 2 of 4 verify, i.e. **the cross-combinations measurably fail** — the
4-byte cross-chunk hash covers the whole bytecode, so an unengineered cross
splice needs a ~2^-32 accidental match, exactly as expected. The current spec
doesn't spell out "2^-32" as a bare probability anywhere (`grep` for
`2^-32`/`2\^32` finds only the ~2^32 *grind-cost* framing, not a residual
*accident* framing), but it grounds the claim in the stronger, empirical form:
§2.5 "honest input yields exactly the real cards (**measured** 5/5
constructions incl. both shared-piece rows)" — a measured 5/5 clean result is
better evidence than a bare probability statement would be. Not filed as a
defect: the property is stated and it is grounded in fact, just not in the
literal "2^-32" phrasing.

## AP2 message accuracy and trigger scope

§2.6: "`|V_class| > k_class` in any class ⇒ **AP2 hard refusal**". Confirmed
this is the ONLY AP2 trigger in the document — no other clause routes to AP2.
The message ("verify as more key cards than they can belong to") is an
accurate paraphrase of `|V| > k`: pieces verify as more decoded-card
identities than the class's own admissible-card count. `grep -n "multiset"`
across the whole spec returns **zero hits** — the old "more than one distinct
card multiset" language (§2.6/§3/Security, all three places r2-era used it)
is completely gone, not just relocated.

## Evaluation order (§2 vs §3)

§2 is numbered 1→6 (sub-group, per-class k_class, cap, budget, enumerate/
verify/seat, ambiguity split) and §3's header states "the §2 order IS the
outcome order; first refusal wins" — one declaration, no competing order
statement anywhere else in the document. The cap (step 3) depends only on
`k_class` from step 2, which is itself computed with "zero decodes" per its
own text — so the cap genuinely runs before any candidate enumeration, with
no circular dependency on §2.5's verified set.

## Row 10's two variants — **NEW FINDING, r4-I1 (Important)**

Row 10 now reads: "**surplus row:** injected extra valid card ground to
verify → `|V| > k` ⇒ AP2 refusal (per §Security — updated from the r2
leftover-refusal expectation); an extra valid card with a DIFFERENT id still
seats-then-leftover-refuses downstream with distinguishable labels, as
today." ("ground" here is deliberate spec terminology — the same word appears
in §2.5, "a ground EXTRA verified card makes `|V| > k` ⇒ refusal" — meaning
grind-constructed, not a typo; both sub-clauses are internally coherent on
their own.)

The problem is §12's churn note: "`v_collide_reaches_the_command` REWRITTEN
with a new minimal 2-slot fixture (its shipped input carries a full extra
card set → leftover path, **row 10's different-id variant**)."

But `v_collide_reaches_the_command`'s shipped input is `v-collide.txt`, and
r3's own P-o measured it precisely: "`tests/fixtures/seating/v-collide.txt`:
a 2-chunk card and a 3-chunk card, **both pinned to id 12345**" — a SAME-id
fixture, not a different-id one. r3's own disposition of this exact fixture
(discharging r2 I2) described it correctly: "that input's two collided cards
sit in **two different total-classes** (P-o), both classes complete, both
seat, and 4 cards against 2 slots reaches the leftover refusal — which is
exactly what row 10 asserts." Same id, split across two totals — not two ids.

Before this fold, the churn note said "the old input becomes row 10's
**end-to-end variant**" — neutral wording that didn't assert an id
relationship. This fold's restructuring of row 10 into an explicit
same-id-in-class-surplus/different-id split, combined with re-labelling the
same citation as "different-id variant," now misdescribes the one fixture
that historically proved the leftover-with-distinguishable-labels claim (r1
I2 → r2 I2 → r3's FIXED disposition). Checked whether row 7 (mixed-totals
rows) picks up the slack instead: it does not — row 7 only asserts "both
classes complete → both seat", never the downstream "too many cards for the
requested slots → leftover, distinguishable labels" consequence that
`v_collide_reaches_the_command` specifically tests. So the SAME-id,
mixed-totals, exceeds-requested-slots scenario that r3 confirmed constructible
no longer has a row that correctly names its own fixture — it is not
literally row 10(a) (that requires `|V_class| > k_class` within one class,
which this input does not produce — each of its two total-classes
independently seats with `|V|=k=1`) and, per its own real shape, it is not
row 10(b) either (that clause is explicit about a DIFFERENT declared id).
Row 10(b)'s claim is itself still fine and low-risk on its own terms (an
honestly-different-id surplus card hitting a pre-existing, unchanged
completeness refusal — "as today" — needs no grind and is trivial to build),
but as currently worded it is not actually backed by the fixture the churn
note points to.

**Why Important, not Minor (unlike r3-M1):** r3-M1 was a bare row-*number*
citation error with no effect on what gets tested. This is a row *content*
mismatch that risks silently dropping a specific, previously-hard-won
assertion (the distinguishable-`#<k>`-labels guarantee under a same-id
mixed-totals leftover) rather than merely mis-numbering a correct one.

**Remedy (one of, not prescriptive):** (a) reword row 10(b) to "an extra
valid card in the SAME id but a DIFFERENT total-class..." to match the actual
fixture, or (b) add a third row-10 clause explicit about the same-id
mixed-totals leftover case and re-point the churn note at it, or (c) if a
genuinely different-id fixture is intended for 10(b), name a NEW committed
fixture for it and leave `v_collide_reaches_the_command`'s citation pointing
at a (renamed) same-id sub-case.

## Cap and mixed-totals arithmetic

3+3 = 6 > 5 (row 7's cap sub-row) is correct. No other arithmetic claims
found un-checked; all spot-checks (3^11, 5^7, 2^17, 3^12) pass as reported in
r3-I1's disposition above.

---

# Gate-can-fail note

Row 10 as now split is constructible and failable for its (a) grind-required
sub-case (shares the AP2 committed script with row 9) and its (b)
different-id sub-case (trivial, pre-existing behaviour) — but, per r4-I1,
neither sub-case, as currently worded, is the fixture the churn note actually
cites, so the SAME-id-mixed-totals-leftover assertion that fixture was built
to prove has no row whose wording matches it.

# Recommended fold order

1. **r4-I1** — reword row 10(b) or add a third sub-case, and fix the churn
   note's citation to match `v-collide.txt`'s actual (same-id) shape.

No Critical, no residual r3 finding. The V=k semantic change (r3-I3) is
sound and its consequences were correctly re-derived everywhere the brief
asked to check (Security, the cap, the evaluation order, the AP2 message,
the benign-candidate probability). The one new defect is a citation
mismatch introduced by restructuring row 10, not a flaw in the V=k rule
itself.
