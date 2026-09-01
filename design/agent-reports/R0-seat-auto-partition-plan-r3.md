# R0 review — IMPLEMENTATION_PLAN_seat_auto_partition.md, round 3 (micro fold-check)

**Artifact:** `design/IMPLEMENTATION_PLAN_seat_auto_partition.md` @ `d263abeb`
**Fold diff:** `git diff 4bb3696b..d263abeb -- design/IMPLEMENTATION_PLAN_seat_auto_partition.md`
**Against:** `design/agent-reports/R0-seat-auto-partition-plan-r2.md` (0C/1I/2M)
**Lens:** fold-check ONLY — does `d263abeb` discharge each of plan-r2's three
findings (I3-residue, and the two Minors), and did the fold introduce a new
defect or internal contradiction? NOT a fresh audit.

## Verdict

**0 Critical / 0 Important — GREEN.** All three plan-r2 findings are closed
against the report's own stated closure bar, and the fold introduces no new
inconsistency in the row-7 taxonomy, the fixture list, or the step numbering.

## Disposition

### I3-residue (Important) — DISCHARGED

Report's remedy: fixture with one class short a piece; row 7 split/citation;
"What GREEN requires" narrowed the bar to "fixture + citation".

> "**incomplete-class set: one complete 2-chunk card + a 3-chunk card
> missing one piece, one id (plan-r2 I3 residue — the r1-C3 fail-closed
> composition rule's separating input: the whole group must refuse via
> arm 1, nothing seats, no pieces dropped)**" (P0 item 3)

> "7 (BOTH sub-rows: 7a both-classes-complete seats; 7b
> one-class-incomplete refuses via arm 1 on the P0 incomplete-class set
> — plan-r2), 9, 10a" (P1 step 3 RED list)

Fixture shape matches the remedy exactly (2-chunk complete / 3-chunk
missing-one-piece, one id). 7a/7b map cleanly onto spec Acceptance row 7's
own two stated arms (`design/SPEC_seat_auto_partition.md:167-169`: "both
classes complete → both seat" / "one class incomplete → whole group
refuses via arm 1, nothing seats") — no invented taxonomy. `grep -n "row
7\|7a\|7b"` over the plan finds exactly one citation site (P1 step 3) plus
the untouched, still-correct `spec rows 7 and 10b` reference at line 29
(pre-existing plan-r1 N1 text tying `v-collide.txt` to row 7's *other* arm,
7a — unaffected by this fold).

**Non-contradiction checks (per the dispatch brief):**
- **Group-cap set** (3+3 two-class, one id, Σk=6) vs. the new
  **incomplete-class set** (2-chunk+3-chunk, one incomplete, one id): different
  declared-totals shapes, different ids, different purposes (cap-refusal vs.
  admissibility-refusal). No overlap, no aliasing.
- **Row 7b** (one-class-incomplete → refuses, nothing seats) vs. **row 10b**
  (surplus variant (b): same-id legitimate extra cards, `v-collide.txt`,
  BOTH classes complete → both seat, then downstream leftover-label refusal):
  confirmed via `design/SPEC_seat_auto_partition.md:177-186` — row 10b is
  built on the *complete*-both-classes fixture (row 7a's fixture, shared by
  design since plan-r1 N1), never the new incomplete-class fixture. 7b and
  10b sit on disjoint fixtures and disjoint outcomes (refuse-nothing-seats
  vs. seat-then-leftover-refuse); the plan keeps them distinct.

**One observation, non-blocking.** The report's fuller remedy also said "add
its shape test"; P0 item 5 (shape tests) is untouched by this fold and does
not cite the incomplete-class set, unlike the group-cap set (which gets an
explicit "Σk = 6" shape-test citation there). The report's own "What GREEN
requires" line narrowed the bar to "fixture + citation" and marked the rest
optional, so this doesn't reopen I3-residue — noting it only because the
fuller remedy text mentioned it. Not filed as a new finding; no owning phase
assigned.

### Minor — P1 step-number citation: DISCHARGED

> "unused by production until P1 step 1 wires the stage" (P0 item 2, was
> "step 2")

Matches P1 step 1's own unchanged text: "§1 canonicalisation stage wired
after `dedupe_strings` using P0's key fn." Confirmed correct against current
step numbering (step 0 = signature change, step 1 = §1 wiring, step 2 = §2
engine) — no off-by-one remains.

### Minor — determinism guard in P0's Gate line: DISCHARGED

> "Gate: shape tests green; suite green; fmt/clippy clean; **fixture
> regeneration re-run yields a clean `git diff` (the determinism guard, in
> the gate itself — plan-r2)**." (P0 Gate line, was missing the clause)

Placed in P0's own `Gate:` line specifically (not P1's), matching the
finding's ask. Restates — doesn't contradict — the header's and P0 item 3's
existing "regen = command + clean diff" prose; appropriate redundancy for a
machine-checkable gate criterion, not a duplication defect.

## No new inconsistency found elsewhere

Walked the full current file (135 lines) for any other reference to row 7,
the group-cap set, or step numbering that the fold might have left stale:
none found. The three edits are surgical and confined to their own findings.

## What GREEN requires

Nothing further for this plan's R0 — 0C/0I closes the loop. The report's own
"stop when you run out of questions, not when a round comes back clean" bar
is met: this round's one question (do the three findings close cleanly) has
no more answers.
