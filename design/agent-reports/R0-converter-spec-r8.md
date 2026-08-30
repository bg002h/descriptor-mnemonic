# R0 round 8 — SPEC_wallet_form_converter.md, fold review (+ the key-reuse operator ruling)

**Artifact:** `design/SPEC_wallet_form_converter.md` @ `c2c132cf`
(reviewed across `c380d9cd` — the r7 fold — and the mid-round operator ruling that
superseded part of it).
**Question:** do the r7 folds plus the ruling resolve the three r7 findings without a new
defect — GREEN?
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED.**
**Counts: 1 Critical / 1 Important / 2 Minor.**

r6's blessing of compose-canonicalise-compare stands settled. Nothing in any repo was
modified.

**The ruling is a good trade and it lands cleanly in the normative section.** Deleting the
whole collapse/capacity/supply-twice arc rather than repairing it removes the case that
produced r7 C1, and it removes it at the root: with key reuse invalid there is no
repeated-key restore, so there is nothing for a capacity rule to get wrong. A3's three
consequences are each sound, and consequence (c)'s reasoning checks out — from one master,
(fingerprint, path) determines the xpub, so two identical fingerprint-bearing declarations
could only be filled by repetition. **What did not follow the ruling is the acceptance
section**, which still requires two rows the ruling has invalidated, and one promise about
double-scans that the shipped string layer refuses.

---

## 0. Disposition

| r7 | resolved? | check |
| --- | --- | --- |
| C1 (capacity fabricates) | **yes, by deletion** | no capacity, no card-side collapse; each supplied instance fills one slot. The `sortedmulti(2,X,X,Y)` fabrication is doubly refused (A4 unfilled-slot **and** reuse-invalidity), both pinned as must-REFUSE rows. My earlier concern that the "tap it twice" recipe was unreachable through the mk1 wire is **moot** — the recipe is gone with the case |
| I1 (form-based tie-break) | **yes** — see *Blessed* | ordering moved to the ASSIGNMENT VECTOR; total and discriminating on every input the engine can reach, including the pinned-id corner. Verified below |
| I2 ("perfect matchings" stale) | **yes** | capacity's removal restores the object; both sites (lines 106, 200) now agree |

**Deletion completeness** (asked explicitly): grep finds `capacity` / `supply-twice` /
`collapse` only in the four places that *narrate* the deletion (spec 186, 202, 319;
brainstorm 57) — plus one that does not. See C1. Matrices remain **byte-identical**; nine
unique `##` headings; brainstorm decision 7 records the ruling verbatim.

---

## CRITICAL

### C1 — Acceptance 2 still requires two must-SEAT rows that the ruling invalidated, so the gate contradicts the normative section.

Acceptance 2 is unchanged by the ruling fold:

> "AND every PROVEN-FREE-SEAT case (the rows that must SEAT — **r5-M1's two-instance
> case**, **r6-I3's capacity case**, the mixed-declaration unique matching — r6 M2)
> demonstrated by a vector row that FAILS if the behaviour is removed or inverted"

Both named rows are now impossible:

* **"r6-I3's capacity case"** — capacity is deleted. There is no capacity behaviour to
  demonstrate, so the row it demands cannot be written. This is the one surviving
  reference to deleted machinery that is *load-bearing* rather than narrative.

* **"r5-M1's two-instance case"** — that fixture is
  `tr(@2,{sortedmulti_a(2,@0,@1),sortedmulti_a(1,@0,@1)})`, all slots at one origin. `@0`
  occupies two positions, both with use-site path `<0;1>/*`, so it is *"the same
  (xpub, use-site path) at two positions"* — **exactly what the ruling declares invalid**,
  and what A3 now says "compose refuses to emit". Acceptance 2 requires a vector row
  asserting that the engine SEATS it. An implementer writing the suite as specified pins
  behaviour the normative section forbids, and the two gates disagree about the same input.

Only the third row, the mixed-declaration unique matching, survives the ruling (distinct
keys, distinct declarations, no reuse).

**The consequence beyond bookkeeping, and the reason this is Critical rather than a
tidy-up.** r5 M1 exists because the clause-based tests kept over-refusing, and its row was
the *only* anti-regression guard against the engine tightening back into refusing valid
card sets. The ruling removes that guard's fixture, and the natural replacement is not
available either: the other measured over-strictness case — taptree branch commutation,
`tr(@0,{sortedmulti_a(2,@1,@2),sortedmulti_a(2,@3,@4)})` with five distinct keys, wallet-equal
yet refused — is reuse-free but is deliberately a **refused** case (r6 M1's
sound-and-conservative ruling), so it cannot serve as a must-SEAT row. After this fold the
must-SEAT side of acceptance 2 rests on one row. Either find a reuse-free free-seat fixture
to replace the two, or state plainly that the proven-free-seat class has shrunk to the
unique-matching case and that over-refusal is no longer gated — but do not leave the
acceptance demanding proofs of behaviour the spec forbids.

---

## IMPORTANT

### I1 — "An accidentally double-scanned identical card dedupes harmlessly" is falsified by the shipped string layer, which refuses instead.

A3(a) closes with:

> "an accidentally double-scanned identical card dedupes harmlessly (no valid wallet could
> need the duplicate)"

That is the right *policy*, and the accidental double-tap is the most likely operator slip
this feature will meet. But duplicate strings never reach a place where the engine could
dedupe them: mk1 strings are grouped into cards by **chunk-set id**, so a card supplied
twice lands in one group and reassembly refuses on the chunk count. Measured, on the
fixture's card 1:

```
$ mk decode <S1> <S2>                 -> 1 card, decodes cleanly
$ mk decode <S1> <S2> <S1> <S2>       (the accidental double scan)
error: chunked-header malformed: received 4 chunks, header declares total_chunks = 2
```

The operator gets a malformed-header error naming neither the duplicate nor the remedy —
not a harmless dedupe. For the spec's sentence to be true, P2 must dedupe identical strings
**before** grouping and reassembly, which is a placement requirement, not an implementation
detail: the layer that would notice the duplicate is upstream of the layer that currently
refuses it.

This is the same shape as the F-420 class the spec cites approvingly elsewhere — a correct
policy stated at a layer that never gets to run. One sentence in P2 fixes it ("duplicate
`--from-mk1` strings are discarded before reassembly"), plus a row, since "harmlessly" is
a claim about observable behaviour and is currently false.

---

## MINOR

* **M1 — the Status line is stale again.** It records r1–r4 only; r5 (RED 1C/1I/2M), r6
  (RED 0C/3I/3M), r7 (RED 1C/2I) and the operator ruling have all folded since. This was
  r6 M3, fixed at r6, and has re-staled by four rounds — worth making the status line part
  of the fold checklist rather than a finding each time.
* **M2 — the mk1 grouping rule is never stated, and two settled results now depend on it.**
  Nothing in the spec says how `--from-mk1` strings are assembled into cards. The
  assignment-vector tie-break is total *because* grouping is by chunk-set id (see
  *Blessed*), and I1's dedupe promise is a statement about where that grouping happens.
  Both are conclusions about a rule the document does not contain. One sentence in P2
  naming chunk-set-id grouping carries both.

---

## Blessed (constructed against, or argued and checked)

* **The assignment-vector tie-break is total and discriminating, including the pinned-id
  corner.** Distinct matchings differ in some slot's seated card, so the slot-ordered id
  lists differ — provided distinct cards carry distinct ids. The spec itself names the
  exception (`mk encode --chunk-set-id` can pin two cards to one 20-bit id), so I built it:
  two different keys at different origins, both pinned to `0xabcde`, each decoding
  correctly alone (`48'/0'/0'/2'` and `48'/0'/1'/2'`). Supplied together they are **merged**
  by set-id grouping and refuse — `error: chunked-header malformed: received 5 chunks,
  header declares total_chunks = 2` — so two id-colliding cards can never both reach the
  engine as distinct cards. The corner is unreachable and the order needs no second key.
  (The blessing is conditional on chunk-set-id grouping; hence M2.)
* **The pathological fixture is untouched by the ruling.** Eleven distinct xpubs, and all
  eleven declarations are distinct `(fingerprint, path)` pairs — three fingerprints across
  four accounts each — so consequence (c) does not fire and no `(xpub, use-site path)`
  repeats. The 22-chunk keyed card declares eleven *fingerprint-free* slots at one shared
  path, and (c) is scoped to fingerprint-**bearing** duplicates, so it survives too.
* **The privacy-preserving 2-of-3 family survives, as the spec claims.** Different masters
  at one path are different xpubs, not reuse; A3 names this explicitly.
* **Decompose's pinned acceptance fixture survives** — the first three lines of `keys.txt`
  are three distinct keys at three distinct origins.
* **Consequence (c) is sound and adds no new hazard.** From one master (fingerprint, path)
  determines the xpub, so two identical fingerprint-bearing declarations are fillable only
  by repetition. The residual corner — two *different* masters colliding on a 4-byte
  fingerprint — was already refused by the pre-existing rule ("identical declared origin
  with DIFFERENT xpubs on fingerprint-bearing cards refuses"), so (c) overlaps it rather
  than opening anything.
* **P3's flip is complete and correctly directed** — the repeated-key input refuses by name
  with a row, superseding r1 M4's collapse; and `md encode`'s `@0,@0` acceptance is filed
  as an md-side question rather than silently changed, which is the right boundary for this
  cycle.

---

## Gate

**RED — 1 Critical, 1 Important.**

The ruling did the hard part well: it removed a class of case rather than adding a rule to
handle it, and everything downstream in the normative section followed. Both of my r7
findings and the whole capacity arc are genuinely gone, and the tie-break survived the one
corner the spec itself flagged.

What is left is the acceptance section not following the ruling, and one promise stated at
the wrong layer. Both are small, and neither touches the seating procedure:

1. **C1** — retire the two invalidated must-SEAT rows, and say what now gates over-refusal
   (the reuse-free replacement, or an explicit statement that the class has shrunk to the
   unique-matching row).
2. **I1** — place the double-scan dedupe ahead of reassembly in P2, with a row, since the
   shipped layer errors instead.
3. **M1, M2** — the status line, and one sentence naming the grouping rule.

Scope r9 to these four. Nothing else in the disposition table, nothing in *Blessed*, and
nothing in r6's blessing of the core needs revisiting.
