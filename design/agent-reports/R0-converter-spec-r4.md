# R0 round 4 — SPEC_wallet_form_converter.md, fold review (NORMATIVE rewritten wholesale)

**Artifact:** `design/SPEC_wallet_form_converter.md` @ `ee46c7fe`
(`git diff fc9fd142..ee46c7fe -- design/`, plus the mnemonic-key FOLLOWUPS successor to
`bcd8505`).
**Question:** does the rewrite resolve the 12 r3 findings without a new defect?
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED.**
**Counts: 1 Critical / 2 Important / 4 Minor.**

r1–r3 measurements and *"What was NOT found"* stay settled. Nothing in any repo was
modified.

**The short version.** The rewrite is the right move and most of it lands. THE PRINCIPLE
is correct, the phase split is genuine and coherent, `--seat` is now real, the
three-disposition stub model resolves r3 C3 properly, and the two equality relations
resolve r3 C2. **One defect blocks: the practical test offered as the implementation of
THE PRINCIPLE is not equivalent to it, and I constructed two card sets where the test
seats freely and the assignments yield different wallets.** The principle is sound; the
sentence an implementer will code is not.

---

## 0. Fold disposition of the 12 r3 findings

| r3 | resolved? | check performed |
| --- | --- | --- |
| C1a (cross-group) | **partially — see C1** | the two-sorted-groups case is now correctly caught by THE PRINCIPLE, and rule 4's contradicting clause is gone. But the practical test that implements the principle admits two *other* invariance failures (below) |
| C1b (rule 4 unswept) | **yes** | grepped: no "refuse only when", no rule-4 remnant; the clause is subsumed by THE PRINCIPLE |
| C2 (equality) | **yes** | SPEND-EQUALITY excludes origins (the fixture's nine-of-eleven mismatch no longer fails the walk); ROUND-TRIP-EQUALITY includes them. Verified satisfiable: on the pinned fixture, `md encode --key/--fingerprint` → card → `md descriptor` preserves all three origins **and** fingerprints byte-exactly (`[73c5da0a/48'/0'/0'/2']`, `…/1'/…`, `…/2'/…` in and out) |
| C3 (tier-2 false refusal) | **yes** | B1's three dispositions, mismatch never a hard refusal. Verified **wallet-confirmed is reachable, not vacuous**: a card minted `--from-md1 <keyed card built under the split set's own declarations>` carries stub `ced22709`, and the composed WalletPolicyId is `ced2270948ecb5af…` — they match |
| C4 (`--seat` undefined) | **yes**, with one gap | selector, scoping, ownership (P2) and two vector rows all present. Verified the selector is total for mk1: the smallest card I could construct (depth-0 xpub, empty path, privacy-preserving, one stub) still emitted **3 strings**, so every mk1 card is chunked and has a chunk-set id. Residual: M1, M2 |
| I1 (P1 spelling) | **yes** | P1 now opens with the path-only inline form and names the bracketed spelling as non-parsing |
| I2 (rule 1 incoherent) | **yes** | two explicit phases, "no check is cited before its phase can compute it"; A1 records, B1 disposes |
| I3 (acceptances unswept) | **yes** | acceptance 2 scopes CE-1 to "not wallet-confirmed"; acceptance 3 rewritten around the three dispositions with the `232214e4`/`ced22709` counterexample as a named permanent row |
| I4 (mk entry) | **yes** | both entries stop restating formulas, cite the falsifications, and grow the lockstep to three repos including `mnemonic-toolkit` |
| M1 ("caught") | **yes** | B2 says "SURFACES … nothing in this engine can catch it alone" |
| M2 (`--seat` rows) | **yes** | two rows named |
| M3 (P2 scope) | **yes** | P2's own paragraph budgets the SPEND-EQUALITY checker and `--seat` |
| M4 (canonical origins) | **yes** | recorded in the mk entry as a design obligation on the upgrade |

Also re-verified: the two matrices remain **byte-identical**.

---

## CRITICAL

### C1 — THE PRINCIPLE is right; the practical test given as its implementation is not equivalent to it. Two constructions where the test seats freely and the wallet changes.

THE PRINCIPLE:

> "a card set seats without operator input iff EVERY complete candidate assignment yields
> the SAME wallet."

Correct, and it subsumes rule 4 cleanly. The sentence an implementer codes is the next one:

> "within one origin-equivalence class, free seating requires **every candidate slot to lie
> in ONE sorted group**"

That tests **slot membership**. Invariance depends on more than membership — it depends on
each candidate slot's *arity* and on *all* of its occurrences. Two counterexamples, both
encoded by `md` today, both with every candidate inside a single sorted group:

**(a) A slot with multiplicity inside the group.**

```
wsh(sortedmulti(2, @0/48'/0'/0'/2'/<0;1>/*,
                   @0/48'/0'/0'/2'/<0;1>/*,     <-- @0 occupies TWO of the three positions
                   @1/48'/0'/0'/2'/<0;1>/*))

both candidates (@0, @1) are in the one sorted group; both declare 48'/0'/0'/2'
  @0=cardX @1=cardY   bc1qc6ssvugf29fq9z78d9559tuyplxlf06cwt57zc8uquamnte00w7qeqfts6
  @0=cardY @1=cardX   bc1qtjd2cjlyz9em9c0k2gzscn8fvav3xmsdmf76039nkjpy8v6x5n4sayalyq
```

The key multiset is `{X,X,Y}` one way and `{Y,Y,X}` the other. Sorting cannot reconcile
them — sorting fixes *order*, not *multiplicity*.

**This shape is not hypothetical for this spec: P3 is specified to produce it.** "Repeated
keys … identical (origin, xpub) appearing in multiple positions collapses to ONE slot
referenced multiply (matching `md encode`'s accepted `@0…@0` form) — **on read AND
write**." So `decompose` emits exactly these templates, and A3's test mis-seats the card
set that `compose` is then handed. The two clauses are in the same document.

**(b) A slot that is in the sorted group AND somewhere else.**

```
tr(@0/48'/0'/0'/2'/<0;1>/*,                       <-- @0 is ALSO the internal key
   {sortedmulti_a(2, @0/48'/0'/0'/2'/<0;1>/*,
                     @1/48'/0'/0'/2'/<0;1>/*),
    pk(@2/48'/0'/9'/2'/<0;1>/*)})                 <-- @2 at another origin, so the class is {@0,@1}

  @0=cardX @1=cardY   bc1pexf96kdhevqryw6ry4w3dug5gf24a0syxckcj3mny3gxp4ma5s9s69x9u7
  @0=cardY @1=cardX   bc1py6ynfg9kupzdkk3mty44t8z7zfra094je5jwlqc23z9l847q9qqs5kchsy
```

Both candidates lie in one `sortedmulti_a` group, so the test permits free seating — but
@0 also holds the taproot keyspend path, and the swap moves it. This is the r3
internal-key hazard returning through a slot that *is* a sorted-group member, which is
precisely the case the membership test was written to allow.

**Why the test fails, stated once.** Sorted-group membership makes two *positions*
interchangeable. Free seating needs two *slots* to be interchangeable, and a slot is a set
of positions with a multiplicity. The test collapses the two.

**What the test has to say instead** (the principle already implies it; only the practical
sentence is short): candidates are freely seatable iff, for every pair of candidate slots,
(i) they have equal multiplicity, and (ii) *every* occurrence of each lies in the same
sorted group. Under (i)+(ii) both constructions refuse: (a) fails (i) — arity 2 vs 1;
(b) fails (ii) — @0 has an occurrence outside the group.

Everything else about A3 stands: the refusal text, the two remedies, and the pointer to
A5 are all right once the predicate is.

---

## IMPORTANT

### I1 — A2's symmetric value match makes a legitimate card set unrestorable: fingerprint-bearing key cards against a fingerprint-free policy declaration.

A2:

> "A card seats at a slot iff the values match; fingerprint-less cards match
> fingerprint-free declarations by path."

The stated asymmetry is the right one and closes the unsafe direction. The unstated one
closes a safe direction too. Measured, both artifacts built from this cycle's own tools:

```
POLICY CARD minted without --fingerprint      KEY CARD minted with --origin-fingerprint
  @0: m/48'/0'/0'/2'                            origin_fingerprint:  73c5da0a
  @1: m/48'/0'/1'/2'                            origin_path:         48'/0'/0'/2'
  (no Fingerprints TLV)

  card = (Some(73c5da0a), 48'/0'/0'/2')
  slot = (None,           48'/0'/0'/2')     -> values differ -> no seat
```

Nothing forces a policy card and its key cards to agree about carrying fingerprints:
`md encode` omits them unless `--fingerprint` is passed, `mk encode` includes one unless
`--privacy-preserving` is passed, and the two are minted at different times by different
people. Under A2 as written, **every** card fails **every** slot and A4 refuses the whole
set as unfilled — a legitimate, self-consistent backup that cannot be restored.

The direction is safe (a refusal, not a mis-seat), which is why this is Important and not
Critical. The fix is to make the declaration the constraint rather than requiring
equality: a card matches a slot iff every component the *declaration* states is matched by
the card. A fingerprint-free declaration constrains only the path; the card's extra
fingerprint is additional information, not a conflict. Note this hands more cards to A3 —
so it needs C1's predicate fixed first, and a vector row either way.

### I2 — Motivation and the brainstorm still point at "the seating engine's two-tier rule 1", which the rewrite deleted.

```
SPEC line 46:        "…was settled by r1 C1 and r2 I1: see the seating engine's
                      two-tier rule 1.)"
BRAINSTORM line 45:  "the seating engine's rules (two-tier stub check, decoded-value
                      origin seating, refuse-where-order-matters …)"
```

There is no "rule 1" and no two-tier model any more: the stub check is A1 + B1 with
**three** dispositions, and "refuse-where-order-matters" has been replaced by THE
PRINCIPLE. Both pointers are dangling, and they dangle on the one question the spec has
spent four rounds settling — what the stub check can and cannot promise. The Motivation is
where a reader arrives before the NORMATIVE section, so this is the first answer they get
and it names a model that was falsified by their own fixture.

The brainstorm carries a "see the SPEC, which supersedes this line's earlier wording"
hedge, which helps; "two-tier stub check" is nonetheless a positive statement of a
superseded model, not a vague one. Both are one-line edits. (This is the fourth round in
which the Motivation section lagged the NORMATIVE rewrite — worth a habit rather than
another finding: sweep Motivation and the brainstorm whenever NORMATIVE changes shape.)

---

## MINOR

* **M1 — A5's "part of the refused ambiguity" conjunct leaves one case undefined, and the
  conjunct is not load-bearing.** A5 requires the card to "already origin-match slot i
  under A2 **and** be part of the refused ambiguity". When A3 did *not* refuse, is a
  consistent `--seat` (naming the slot A2 would pick anyway) vacuously satisfied or
  violated? The spec's refusal list — "unknown id, ambiguous id, or a `--seat`
  contradicting A2" — does not cover it. **Judgement: drop the conjunct.** `--seat` can
  only ever place a card where A2 would place it, so the origin-match requirement already
  carries the whole safety argument; the conjunct adds no protection and breaks the
  reasonable habit of scripting `--seat` for every slot (in a mixed run where one origin
  class is ambiguous and another is not, the strict reading refuses the second class's
  seats). Keep "contradicting A2 refuses"; that is the clause doing the work.
* **M2 — the chunk-set id is a 20-bit value that no `mk` surface prints.** A5's selector
  is "the exact label the A3 refusal printed", so md must compute and print it — fine, and
  `md_codec::md1_chunk_set_id` exists. But `mk decode` and `mk inspect --json` emit
  `chunks` and `chunk_variants` and no set id, so an operator holding plates has no way to
  read the label off a card independently of md. Worth one sentence saying the label is
  md-side only, or a follow-up to surface it in `mk`. Related: `mk encode --chunk-set-id`
  lets two cards be *pinned* to the same 20-bit id, so A5's "ambiguous id refuses" is
  reachable deliberately, not only by birthday luck — a good thing to name in the row.
* **M3 — the Status line is three rounds stale:** "R0 r1 (RED 3C/9I/8M/2N …) folded
  2026-08-30", with r2 and r3 folded since and this rewrite on top. It is the first line a
  resumer reads.
* **M4 — Gates' mk clause reads as if R0 found nothing on the mk side.** "mk is untouched;
  the cross-repo mirror rule applies only if R0 finds an mk-side action." R0 has found one:
  `stub-keyed-wallet-binding-at-mint`, now a three-repo lockstep with a canonical-origin
  design obligation. No mk *code* changes in this cycle, which is what the clause probably
  means — say that, and point at the entry.

---

## Blessed (asked for explicitly; recorded so r5 does not re-derive)

* **Phase coherence, walked end to end on the fixture.** A1: 11 cards, stubs all
  `5b48af35`, policy template-id top-4 `5b48af35` ⇒ all shape-matched (needs only decode).
  A2: eleven distinct (fp, path) declarations, one card each, exact matches. A3: no
  ambiguity — each origin class is a single card and a single slot. A4: total. A5: unused.
  B1: composed WalletPolicyId `ced22709` ≠ `5b48af35` ⇒ not wallet-confirmed, but
  shape-matched from A1 ⇒ **shape-confirmed** — the correct disposition. B2: no keyed card
  supplied ⇒ address 0 to stderr. **No step needs data a later step computes**, and the
  A1-records/B1-disposes split is what makes that true.
* **B1's top tier is reachable** — measured, not assumed (a vacuous tier would have been a
  gate that cannot fire): stub `ced22709` on a card minted `--from-md1` the keyed card
  built under the split set's declarations, against composed WalletPolicyId
  `ced2270948ecb5af…`. Acceptance 3's third clause is satisfiable.
* **ROUND-TRIP-EQUALITY is satisfiable on the pinned fixture** — origins *and*
  fingerprints survive `md encode --key/--fingerprint` → card → `md descriptor` exactly.
* **A5's selector is total for mk1 cards.** Every card I could construct, down to the
  minimal payload, emitted ≥ 2 strings, so a chunk-set id always exists. (The
  "single-string card has no set id" concern raised by `mk-cli/src/error.rs:63`'s
  `"single-string chunk 2"` label applies to md1 policy cards, not mk1 key cards.)
* **r3 C1b is genuinely gone** — no "refuse only when" clause survives anywhere.
* **The classification's catch-all is exhaustive over positions** ("any position the
  classifier cannot place … refuses"). The defect in C1 is not the position taxonomy; it
  is that positions are the wrong unit.

---

## Gate

**RED — 1 Critical, 2 Important.** One more round.

This is the round where the design became right. THE PRINCIPLE is the correct abstraction
and it retired three rounds of patchwork; the phase split answered r3 I2 structurally
rather than by wording; B1's three dispositions are the honest model that the measurements
kept demanding; and the two equality relations finally separate two jobs that had been
fighting since r1. Nine of the twelve r3 findings close cleanly and four of the remaining
checks came back blessed on measurement rather than on reading.

What is left is small and local:

1. **C1** — replace the membership test with the interchangeability test: equal
   multiplicity, and every occurrence of every candidate inside the same sorted group.
   Both constructions above become vector rows; (a) especially, since P3 is specified to
   emit that template shape.
2. **I1** — make the declaration the constraint in A2, not an equality; add the row.
3. **I2** — two one-line pointer fixes (spec line 46, brainstorm line 45).
4. **M1–M4** ride the same fold.

Scope r5 to those seven items and to whatever the fold newly writes. Nothing in the
disposition table marked **yes**, and nothing in *Blessed*, needs revisiting.
