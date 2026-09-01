# R0 — `design/SPEC_seat_auto_partition.md` fold-check, round 5 (micro)

**Question asked:** does the fold at `d51d68f8` close r4-I1 (row 10's
sub-cases didn't cover the shipped `v-collide.txt` fixture) without
introducing a new inconsistency?

**VERDICT: 0 Critical / 0 Important — GREEN. r4-I1 CLOSED.**

Reviewed: `design/SPEC_seat_auto_partition.md` @ `d51d68f8` (diff against
`0d6ccf89` via `git diff 0d6ccf89..d51d68f8 -- design/SPEC_seat_auto_partition.md`),
against `design/agent-reports/R0-seat-auto-partition-r4.md`'s finding text and
`R0-seat-auto-partition-r3.md`'s measured table (P-o) and prior disposition
(r1 I2 → r2 I2/N-f/I5 → r3 I5(a)/FIXED). Fixture measurements not re-derived
per brief — cross-checked against the report quotes, and independently
confirmed by reading `crates/md-cli/tests/fixtures/seating/v-collide.txt` and
`crates/md-cli/tests/seating_vectors.rs:834-853` directly (both exist as
described, HEAD is clean at `d51d68f8`).

---

## r4-I1 disposition → **FIXED**

Row 10 is now:

> "10. **surplus rows, three variants (r4-I1):**
> (a) same-id GROUND extra verified candidate in one class →
> `|V| > k` ⇒ AP2 refusal (per §Security);
> (b) same-id LEGITIMATE extra cards that seat — the shipped
> `v-collide.txt` fixture exactly: both cards pinned 12345, one
> 2-chunk and one 3-chunk, so BOTH total-classes seat via §2 and the
> template's completeness then refuses downstream with
> DISTINGUISHABLE `12345#1`/`12345#2` leftover labels — this variant
> inherits the r1-I2 guarantee;
> (c) different-id extra card → today's leftover path, unchanged."

And §12's churn note now reads:

> "`v_collide_reaches_the_command` REWRITTEN with a new minimal 2-slot
> fixture for the seat+note outcome (its shipped input carries a full
> extra card set and both collided cards seat → it becomes the surplus
> row's variant (b), the same-id seats-then-leftover case with
> distinguishable labels)."

**(1) Variant (b) matches the fixture, unchanged from the reports' own
measurement.** r3's P-o: `"tests/fixtures/seating/v-collide.txt: a 2-chunk
card and a 3-chunk card, both pinned to id 12345"`. My own read of the file
confirms: card A is 2 `mk1` lines (2-chunk), card B is 3 `mk1` lines
(3-chunk), header comment states `--chunk-set-id 0x12345` for both. Variant
(b)'s wording ("both cards pinned 12345, one 2-chunk and one 3-chunk")
matches exactly.

**(2) The churn entry now maps to variant (b) consistently**, both by name
("it becomes the surplus row's variant (b)") and by content ("the same-id
seats-then-leftover case with distinguishable labels" — identical in
substance to variant (b)'s own text). The specific defect r4-I1 named — the
prior text mislabelling this same-id fixture as "row 10's different-id
variant" — is gone; grepped the whole document for `different-id` (2 hits,
both correctly scoped to variant (c) only) and for `row 10` (2 hits, no
stray "different-id" attribution anywhere else).

**(3) Cross-section consistency:**
- **§2** — "BOTH total-classes seat via §2" is mechanically correct: within
  the 12345 id-group, sub-grouping by declared total (§2 step 1) yields two
  total-classes (n=2, n=3), each with `k_class = 1` and exactly one verified
  card, so each trivially satisfies §2.5's `|V_class| = k_class` + full
  cover — no cap, budget, AP2, or arm-1 refusal fires at the §2 layer.
- **§4** — confirmed the ordinal is per id-GROUP, not per total-class:
  "collided cards are labelled `<id>#<k>`" keys off the shared `<id>`
  (12345), not `<id,total>`, and "Order key: ascending
  `encode_bytecode(&card)`" is stated once, group-wide, with no per-class
  restriction. So `12345#1`/`12345#2` spanning two different total-classes
  is well-defined, as variant (b) requires.
- **Group-wide cap** — `Σ k_class = 1 + 1 = 2 ≤ 5`; no cap refusal, matching
  variant (b)'s silence on the cap.
- **§Security** — no textual change in this fold (confirmed: the diff
  touches only row 10 and row 12, not the Security section). One
  pre-existing, non-new tension worth naming: Security's line "Surplus valid
  card injection: `|V| > k` ⇒ AP2 refusal (this replaces the r2 claim that
  it seats then fails completeness — stricter and earlier)" reads, in
  isolation, as if seats-then-fails-completeness no longer happens at all —
  yet variants (b) and (c) both still assert exactly that pattern. This
  reads consistently only under the scoping that "surplus valid card
  injection" in that bullet means specifically a `|V_class| > k_class`
  violation *within one class* (row 10(a)'s case) — a reading r1-r4 already
  relied on to keep the old different-id variant (now (c)) coherent
  alongside the same Security text, unmodified since before this fold. Not
  filed as r4-I1-adjacent because it is unchanged by this diff and was
  already implicitly accepted through three prior rounds; flagged here only
  because the brief asked the §Security cross-check explicitly.

**(4) No stale two-variant wording remains elsewhere.** Swept the document
for `row 10`, `surplus`, `different-id`, `leftover`, `r1-I2`, `two variants`
— every hit is scoped to the current three-variant row 10 or its churn
citation; nothing else in §1–§13 or Out of scope references the old
two-clause shape.

---

## Gate-can-fail note

Variant (b)'s fixture is not hypothetical or newly invented — it is the
already-committed `v-collide.txt`, reachable today via
`v_collide_reaches_the_command` (currently asserting the pre-spec arm-1
refusal path; the churn note describes its post-implementation rewrite).
Constructible and observable exactly as r3/r4 already established for this
fixture (P-o, r3 I5(a)).

## Conclusion

r4-I1 is genuinely fixed, not transcribed: the fixture-to-variant mapping is
correct, the churn note's citation now agrees with row 10(b), and every
cross-section check (§2, §4, cap, §Security) is consistent — modulo one
pre-existing Security-wording tension that predates this fold and was not
disturbed by it. **0C/0I — this closes the spec's R0.**
