# R0 round 2 — IMPLEMENTATION_PLAN_wallet_form_converter.md, scoped fold review

**Artifact:** `design/IMPLEMENTATION_PLAN_wallet_form_converter.md` @ `502b25f6`
(fold of r1 `2d929d96`).
**Question:** did the fold fix each r1 finding, and did it introduce a new defect?
Scoped review, not a fresh audit.
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: GREEN (0C / 0I).**
**Counts: 0 Critical / 0 Important / 1 Minor.**

The SPEC stays settled and was not re-opened. Nothing in any repo was modified.

All five r1 findings close. The two substantive checks — do the new rows demand what my
r2/r4 counterexamples and A2's restrictive sentence actually demand, and does the
redefined gate still admit a false PASS — were run against the tools rather than read,
and the fold's own I3 measurement reproduces digit for digit.

---

## (a) Each r1 finding closed by the text

| r1 | closed? | check performed |
| --- | --- | --- |
| **I1** — three spec-promised rows missing | **yes** | `V-R2-ORD`, `V-R4-IK`, `V-FPFREE-CARD` are in the roster *and* in C2's step lists (steps 3, 3, 2). `V-ORD` is rescoped to "three supply orders of a **SEATABLE** set → identical descriptor bytes (determinism)", so it can no longer be mistaken for r2's refusal case. Expect columns verified in (b) |
| **I2** — C0/C3 exit gates could pass with rows unwritten | **yes** | "the gate" is redefined once at C0 to include, for every phase, "the phase's named roster rows present and passing, proven by a row-scoped run … an empty filter is a FAIL, not a pass"; C3's exit now reads "the gate (its row-scoped run covers all eight V-D-* rows)". Residual precision issue → M1 |
| **I3** — wrong keyed-card fixture provenance | **yes, and machine-checked before folding** | provenance corrected to `journey_pathological.html`; §8's *verified* list carries the measurement. Re-run today, every number reproduces: 44 `md1fatzr2…` tokens (42×86 + 2×59), 22 unique (21×86 + one 59-char tail) — the W-PIN shape. C4's copy step extracts, dedupes and asserts that shape before any walk runs, which is the right place for it |
| **M1** — ordering rationale contradicted D2 | **yes** | rewritten to "C1 is the small standalone surface … keeps the big phase's diff pure engine", and the shared-parser claim now reads "C1's flags and C3's emission checks, per D2" — matching D2's "shared by P1 flags and P3 emission checks" exactly |
| **M2** — A5 note deferred a settled question | **yes** | "A5's 'ambiguous id' refusal is UNREACHABLE, settled by SPEC A3(a) step 3 … no implementer determination is open"; `V-COLLIDE`'s test carries the subsuming comment |

## (b) Do the new rows' Expect columns demand what the counterexamples demand? — **yes, all three; re-run**

**`V-R2-ORD`** — "the r2 three-orders counterexample fixture | REFUSES identically in all
three orders (the verdict, not just the bytes, is order-invariant)". Re-run:

```
wsh(or_i(multi(2,@0,@1),multi(2,@2,@3))), four fingerprint-free cards at one path
  order 0 1 2 3   bc1qural2jexg8yrjlrc56m84sn423vxe6cv920ehx9duckku9z7j26qswngek
  order 2 3 0 1   bc1qsyld9jctwuncam7n4mm85xsmwx6kwg40ed2uygknxfzsplthudhspzla5z
  order 1 0 2 3   bc1qyatz86rn384ce42dx47zlketzejqgsqkpqpw0ullx7x0x4puczys6x354j
```

Three wallets ⇒ not all matchings equal ⇒ the procedure refuses. And because the matching
set is a property of A2's graph rather than of arrival order, the *verdict* is
order-invariant. The Expect is correct and **stronger than what r1 asked for** — r1 asked
only that the case refuse; the fold pins that the refusal itself does not depend on supply
order, which is the sharper regression target.

**`V-R4-IK`** — "r4's internal-key case, reuse-free five-distinct-key form | refuse
(internal-key/leaf repartition composes unequal wallets)". Re-run:

```
tr(@0/O,{sortedmulti_a(2,@1/O,@2/O),sortedmulti_a(2,@3/O,@4/O)}), five distinct keys, all at O
  baseline        0 1 2 3 4   bc1pfv38mkt0q0twjhgkvvzpu5u7yfaarguqsjntgs5hsq2f46kar66skzrayn
  group swap      0 3 4 1 2   bc1pfv38mkt0q0twjhgkvvzpu5u7yfaarguqsjntgs5hsq2f46kar66skzrayn   equal
  IK <-> leaf     1 0 2 3 4   bc1pg6w98ga5wzm0pfejhumy7e68ymehhm77tmvf65afpk4dzaagglwq4lpvcq   DIFFERS
```

This is the **r4** fixture (internal key inside the candidate class), not r9's
group-repartition fixture — the right one, and the mechanism the Expect names is the one
that fires: group swap alone is invariant, and it is the internal-key/leaf move that
composes an unequal wallet.

**`V-FPFREE-CARD`** — "fp-free CARD against a fp-BEARING declaration | cannot satisfy
(A2's restrictive half); slot otherwise unfillable → unfilled-slot refusal naming the
declared origin". SPEC A2's restrictive sentence is *"A fingerprint-free CARD satisfies
only a fingerprint-free declaration by path — a declared fingerprint is a requirement the
card cannot meet blind"*, and SPEC A4's observable is *"Unfilled slot: refuse naming the
slot and its declared origin"* — the Expect matches both, and routes the unobservable
predicate through the observable refusal correctly. It discriminates: invert A2's
restrictive half and the slot fills, so no refusal fires and the row fails. The fixture
condition that makes it work ("slot otherwise unfillable") is stated in the row itself.

## (c) Does the redefined gate still admit a false PASS? — one residual, filed as M1

## (d) Any new contradiction between §3's ordering text and D2? — **no**

Grepped: the only surviving "P2 consumes" occurrence is inside the r1 M1 correction note,
quoting the wrong reason in order to retract it. No stale `CONTINUITY_2026-08-29-s2`
citation, no "implementer proves which", no "may be UNREACHABLE".

---

## MINOR

* **M1 — the gate's *requirement* is complete but its *proof artifact* is weaker than the
  requirement, and C3 is the phase where that gap is unbacked.** The clause reads: "the
  phase's named roster rows present and passing, proven by a row-scoped run … whose
  **NONZERO** matched-test count is quoted in the phase-close commit message". The
  requirement ("named roster rows present and passing") is exactly right and closes r1 I2
  — a phase can no longer close by passively writing no tests. Two residuals remain in the
  proof:

  1. **Nonzero ≠ complete.** C3 names eight `V-D-*` rows; a run matching one of them
     reports a nonzero count and satisfies the quoted artifact. Quoting the count against
     an **expected** count (C3: 8) rather than against zero closes this mechanically, and
     costs one word.
  2. **Present ≠ asserting.** A named-but-empty test matches the filter and passes. No
     count-based gate can close this in principle — it is what a reviewer is for. C2 has a
     scoped review whose brief already asks "do the rows assert what the spec's rows
     demand"; **C3 has eight rows and no scoped review**, with C4's mandatory whole-diff
     adversarial review as the only backstop. Extending C2's scoped-review pattern to C3
     would close it at the phase rather than at the merge.

  Filed Minor rather than Important: the fold closed the defect as raised (the passive
  omission is gone), both residuals require a deliberate act to hit, and C4's non-deferrable
  review stands between them and merge. Both fixes are one line each.

---

## Blessed

* **The matrix travels byte-identically across all three artifacts** — spec, plan and
  brainstorm matrix blocks diff clean, six rows each. The operator directive's fourth home
  (`seat/mod.rs`'s doc comment) remains a D2 requirement.
* **Roster ↔ phase-body correspondence is exact.** 43 rows (C2=28, C3=8, C1=5, C0/C1=1,
  C4=1). Set-differencing the roster's C2 rows against the rows named in C2's seven step
  lists returns **empty in both directions** — no rostered row is unscheduled, and no
  scheduled row is unrostered. That is the property r1 I1 was about, now mechanically true.
* **The I3 measurement is real and reproduces**, and the fold did the thing r1's pattern
  note asked for: measured before writing, then wrote the number. Re-run today: 44 tokens,
  42×86 + 2×59, 22 unique, 21×86 + one 59-char tail.
* **§8's audit boundary is now honest** — the keyed-card fixture moved from unstated into
  the verified list, and the two deliberate non-verifications (mk-codec 0.5.0's registry
  presence; its decode API surface) remain correctly named as C0's entry gate.

---

## Gate

**GREEN — 0 Critical, 0 Important.** The plan's R0 loop closes.

Every r1 finding is closed by text rather than by reference, the three restored rows demand
what the counterexamples that motivated them actually demand — verified by re-running both
fixtures, not by reading the table — and the fold machine-checked its fixture claim before
writing it, which is the practice this cycle spent several rounds learning.

The single Minor is a precision improvement to a gate that already does its job, and does
not hold the gate: quote the expected row count rather than merely nonzero, and consider
giving C3 the scoped review C2 has. Both can ride the first implementation commit or be
filed with C3 as the owning phase — neither warrants a review round.

**Recommendation: close the plan's R0 and begin C0.** Carry forward, per the plan's own
§7: re-validate plan freshness immediately before each phase's implementer is dispatched
(a GREEN expires), keep UC off for implementation, and hold C4 item 5's mandatory whole-diff
adversarial review as non-deferrable before merge.
