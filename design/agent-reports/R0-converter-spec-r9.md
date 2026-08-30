# R0 round 9 — SPEC_wallet_form_converter.md, scoped fold review

**Artifact:** `design/SPEC_wallet_form_converter.md` @ `8aba6ab3`
(fold of r8 `037e1e8a`, plus the operator refinement of 2026-08-30).
**Question:** did the fold fix each r8 finding, and did it introduce a new defect?
Scoped review, not a fresh audit.
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: GREEN (0C / 0I).**
**Counts: 0 Critical / 0 Important / 3 Minor.**

The settled facts named in the brief (BIP-388 clauses, the md inversion, the FOLLOWUPS
rename) were taken as given and not re-derived. r6's blessing of
compose-canonicalise-compare and r8's blessings stand. Nothing in any repo was modified.

---

## The four checks

### (a) Each r8 finding closed by the text, not merely referenced

| r8 | closed? | evidence in the text |
| --- | --- | --- |
| **C1** — acceptance 2 demanded two ruling-invalidated must-SEAT rows | **yes** | the roster no longer names r6-I3's capacity case or r5-M1's two-instance case. It now reads: the fingerprint-free same-path different-masters family (SEAT side), "its REFUSE side (the same xpub at two slots) sitting adjacent so the boundary is pinned from BOTH directions", plus the mixed-declaration unique matching. The duty transfer is stated rather than left implicit — and it holds; see (c) |
| **I1** — "dedupes harmlessly" falsified by the string layer | **yes** | A3(a) now says the harmlessness comes "BY ORDER OF OPERATIONS, not by assumption", quotes my falsifier verbatim (`received 4 chunks, header declares total_chunks = 2`), and makes the pipeline normative in three numbered steps with dedupe *first*. A full-duplicate-set must-SEAT row ships |
| **M1** — stale status line | **yes** | current through r8 with counts matching the persisted reports (r5 1C/1I/2M, r7 1C/2I, r8 1C/1I/2M) and the rulings noted. (r6 is rendered `0C/3I`, dropping its `/3M`; the other rows carry their Minor counts. Cosmetic, not staleness — noted, not filed.) |
| **M2** — grouping rule unstated | **yes** | step (2) of the pipeline: "group the survivors by declared chunk-set id (r8 M2 — the grouping rule the tie-break's totality depends on)". The dependency is named at the point of statement, which is what the finding asked for |

### (b) The reground is consistent spec-wide

**The spec is fully regrounded.** Every `invalid` hit in
`SPEC_wallet_form_converter.md` is deliberate: lines 190/204/242/363 are the reground
itself ("the ground is NOT invalidity", "never 'invalid'", "as unsupported, never as
invalid"), and line 199 quotes BIP 388's own term ("BIP 388's invalid-example list"),
which is correct usage of the BIP's language rather than the spec's own framing. A3's
consequence (c) was rewritten from "INVALID AT THE DOOR" to "REFUSED AT THE DOOR". No
survivor.

**The brainstorm is not** — see M1 below.

**The shape-(2) scoping does not contradict the A3 procedure**, but it does pull against
one sentence above it — see M2 below. The procedure itself is untouched by the reground:
the refusal ground moved from "invalid" to "BIP-388-forbidden", and the *behaviour* (refuse,
both directions, named rows) is unchanged, so nothing in A1–A5/B1–B2 or the matching
enumeration shifts.

### (c) Does the boundary pair actually pin what r5-M1 pinned? — **BLESSED, constructed**

r5-M1's duty was to catch the engine tightening back into refusing card sets that are
genuinely wallet-invariant. Its fixture is gone (it repeated `@0`, now unsupported), so the
question is whether the replacement covers the same *class*. I tried to construct the
member the replacement would miss: a **reuse-free** case where candidates span two sorted
groups and every matching is wallet-equal. There is none.

```
tr(@4/48'/0'/9'/2'/<0;1>/*,                      internal key at ANOTHER origin -> not a candidate
   {sortedmulti_a(2,@0/48'/0'/0'/2'/…,@1/…),      candidates @0..@3 at one fingerprint-free origin
    sortedmulti_a(2,@2/…,@3/…)})                  five DISTINCT keys -> no reuse
  encodes as an md1 policy: yes

  partition {c0,c1}|{c2,c3}   bc1pkpcp59lj8j506methz2dywk65wr5qhget2h34n0yqaga0a4934xslhwjer
  partition {c2,c3}|{c0,c1}   bc1pkpcp59lj8j506methz2dywk65wr5qhget2h34n0yqaga0a4934xslhwjer   group swap: equal
  partition {c0,c2}|{c1,c3}   bc1pe62lkev256895sf5pq23w9jj5eag34fn4a6jtzptg64zlzxemftshwgvwh   REPARTITION: differs
  partition {c0,c3}|{c1,c2}   bc1px0f3vzmp6eqxku7ulhz4f93jrs6cllvj673s8v8ykl6pceqduc4qlrnjtj   REPARTITION: differs
```

Group *swap* is invariant, but *repartition* is not, so not every matching agrees and A3
refuses — correctly. The multi-group class therefore has no must-SEAT instance once key
reuse is out; r5-M1's fixture qualified only *because* it repeated `@0`. The class the
replacement must cover is the single-sorted-group ambiguity, and the SEAT-side row covers
it and can pass:

```
wsh(sortedmulti(2,@0,@1,@2)), three DIFFERENT masters at one fingerprint-free path
  order 0 1 2 / 2 1 0 / 1 2 0 / 0 2 1  ->  bc1q2sz6vvu6k7y9gtc6kfgfe0p6xkhmvmdlu97eecjkykpdktvps08scdjgr5  (all four)
```

That row fails the moment the engine re-tightens into refusing shared-path
privacy-preserving ambiguity — the r2-I2 and clause-test regression class, which is what
r5-M1 was guarding. **The duty transfer is sound and complete for the class that still
exists**, and pinning the REFUSE side adjacent means a future loosening is caught too.

### (d) Does the pipeline ordering break the blessed pinned-id-collision corner? — **BLESSED**

It does not, and the ordering is load-bearing in the right direction:

* **Full duplicate set** (the must-SEAT row): `S1 S2 S1 S2` → step (1) dedupes
  byte-identical strings → `S1 S2` → step (2) groups → step (3) reassembles → one card.
  Dedupe **must** precede grouping or the duplicates inflate the group and refuse; it does.
* **Pinned-id collision** (r8's blessed corner): two *distinct* cards pinned to one 20-bit
  id are not byte-identical, so they survive step (1) unchanged, merge at step (2), and
  refuse at step (3) — exactly as measured (`received 5 chunks, header declares
  total_chunks = 2`). The spec quotes that measurement in place and draws the right
  conclusion: "the seating engine never sees colliding cards", so the assignment-vector
  tie-break stays total.

The two behaviours are separated precisely by byte-identity, which is the discriminator the
pipeline uses. Corner preserved.

---

## MINOR

* **M1 — the brainstorm's decision 7 still carries the pre-refinement framing its own
  refinement retracts.** Check (b) asked for "no surviving 'invalid wallet' framing
  anywhere". The spec is clean; `BRAINSTORM_wallet_form_converter.md` is not. Decision 7's
  first paragraph reads *"Repeated (xpub, use-site path) is an **invalid wallet** in both
  converter directions"* and *"fingerprint-bearing duplicate declarations are **invalid
  policies** at the door"*, and its Refinement paragraph three lines later says the ground
  is *"NOT invalidity — the diagnostics say 'unsupported', never 'invalid'"*. One entry,
  both framings. Graded Minor rather than Important because the correction is present and
  adjacent — unlike r4 I2, where the referenced model did not exist at all — and because
  nothing normative depends on the brainstorm's wording. But this is the sixth round in
  which the companion artifact lagged a normative change, so it is worth fixing as the
  habit rather than the instance: two sentences, rewritten to the refusal ground the
  spec now uses.

* **M2 — "each with a named row" over-promises for one of the four shape × direction
  cells.** A3 says the converter refuses both forbidden shapes in both directions "each
  with a named row", and then the scope note says shape (2) "is unreachable through md
  today and its refusal is recorded for completeness". Both are true; together they leave
  one cell without a writable row. On the **compose** side, a shape-(2) row cannot exercise
  the engine's BIP-388 refusal at all, because md's template parser refuses first for an
  unrelated reason ("@0 appears with inconsistent path/multipath/hardening") — the row
  would pass in both worlds, which is the false-PASS class this repo gates on. The other
  three cells are fine, and I confirmed the **decompose** side of shape (2) is genuinely
  reachable, so its row can fail honestly:

  ```
  rust-miniscript @ ff4732e:
    wsh(multi(2,K/<0;1>/*,K/<2;3>/*))   PARSES ok   <- BIP-legal disjoint; decompose sees it
    wsh(multi(2,K/<0;1>/*,K/<0;1>/*))   PARSES ok   <- forbidden same-path
    wsh(sortedmulti(2,K/<0;1>/*,K/<0;1>/*))  PARSES ok
  ```

  One clause fixes it: say three of the four cells carry rows, and that compose × shape (2)
  carries none until md's template surface changes — which is already the filed
  `md-repeated-placeholder-inverts-bip388` question.

* **M3 — the P2 input pipeline is normative but lives in A3(a), not in P2.** P2's paragraph
  exists so that "a reader scoping P2 from this paragraph budgets it here" — that is why it
  explicitly names the SPEND-EQUALITY checker and `--seat`. The dedupe → group → reassemble
  pipeline is now a P2 deliverable of comparable size and is described three sections
  earlier. One cross-reference in P2 closes it; the same fix r3 M3 and r6 M3 applied to the
  other two deliverables.

---

## Gate

**GREEN — 0 Critical, 0 Important.** The R0 loop closes.

All four r8 findings are closed by the text rather than by reference, the operator
refinement is applied consistently through the normative section, and both of the round's
substantive checks came back blessed on constructed evidence rather than on argument: the
key-reuse boundary pair genuinely inherits r5-M1's anti-over-refusal duty (and the class it
cannot cover turns out not to exist once reuse is forbidden), and the new input pipeline
preserves the pinned-id-collision corner by ordering dedupe ahead of grouping.

The three Minors are recorded and do not hold the gate: one is a companion-document
wording lag, one is a row-coverage clause on an already-flagged unreachable cell, and one
is a cross-reference. All three are single-sentence edits and can ride the next commit or
be filed with an owning phase — none needs a review round, and re-dispatching for them
would be the reassurance loop the closure rule exists to prevent.

**Recommendation: close R0 and proceed to the implementation plan.** Carry forward, as the
plan's inherited obligations: the vector-row roster named across A1–A5/B1/B2 and acceptance
2–3, the pinned decompose fixture (r2 M5), the three-repo `stub-keyed-wallet-binding-at-mint`
lockstep with its canonical-origin obligation, and the filed
`md-repeated-placeholder-inverts-bip388` md-side question.
