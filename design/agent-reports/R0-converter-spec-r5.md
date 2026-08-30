# R0 round 5 — SPEC_wallet_form_converter.md, fold review

**Artifact:** `design/SPEC_wallet_form_converter.md` @ `3a679d4b`
(`git diff ee46c7fe..3a679d4b -- design/`).
**Question:** does the fold resolve the 7 r4 findings without a new defect?
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED.**
**Counts: 1 Critical / 1 Important / 2 Minor.**

r1–r4 measurements stay settled; the four r4 *Blessed* items (phase walk, top-tier
reachability, round-trip satisfiability, selector totality) were not re-derived. Nothing
in any repo was modified.

**Six of the seven r4 findings close cleanly.** The full-document sweep is clean for the
first time in the cycle. What blocks is one thing, and it is the same thing as r4: THE
PRINCIPLE is right and the practical test that implements it is still not equivalent to
it — I have a third construction, on a new axis, and it survives the strengthened
predicate. Separately, the A2 fix landed correctly but its interaction with A3's framing
was not followed through.

---

## 0. Fold disposition of the 7 r4 findings

| r4 | resolved? | check |
| --- | --- | --- |
| C1 (test ≠ principle) | **no — see C1** | equal-multiplicity + same-one-group is now **sound** for the two r4 constructions and both are cited and required as rows. A third axis defeats it: per-slot use-site paths |
| I1 (A2 symmetric equality) | **yes, with a follow-through gap — see I1** | A2 is now declaration-as-constraint, with the fingerprint asymmetry stated in both directions. The r4 counterexample seats. But A3/THE PRINCIPLE still frame the problem as origin-equivalence *classes*, which A2 no longer produces |
| I2 (dangling two-tier refs) | **yes** | grepped both documents: no `two-tier`, no `rule 1`/`rule 4`/`rule 6`, no `Rules, in order`, no `refuse only when`, no `values match`. Motivation now points at "the A1/B1 three-disposition stub model"; brainstorm decision 4 names the PRINCIPLE, phases A/B and the three-disposition model |
| M1 (`--seat` conjunct) | **yes** | conjunct dropped; "a consistent `--seat` on a NON-ambiguous slot is simply satisfied"; contradicting-A2 kept as the working clause, with the scripting rationale recorded |
| M2 (chunk-set-id label) | **yes** | label named md-side, `mk inspect` follow-up option noted, and `mk encode --chunk-set-id` deliberate pinning called out so the ambiguous-id row does not rest on birthday luck |
| M3 (stale status) | **yes** | all four rounds with their counts, plus the report glob |
| M4 (Gates mk clause) | **yes** | "No mk CODE changes in this cycle … but R0 DID find the mk-side action", pointing at the filed three-repo lockstep |

Re-verified structurally: the two matrices remain **byte-identical**; headings are unique
and in order; every `A1`–`A5`/`B1`/`B2` reference outside the NORMATIVE section (Motivation
line 48, P2 lines 242–245, acceptance 2 and 3) resolves to a label that exists.

---

## CRITICAL

### C1 — Still open. The strengthened test is sound on both r4 constructions and is defeated by a third: per-slot USE-SITE paths inside one sorted group.

The predicate now reads:

> "every candidate slot to have EQUAL MULTIPLICITY … AND every OCCURRENCE of every
> candidate slot to lie in the SAME ONE sorted group"

Both r4 constructions are correctly refused by it, and both are required as rows. The
predicate is nonetheless not equivalent to THE PRINCIPLE, because it reasons about a
slot's **structural position** and a slot also carries a **derivation path**.

```
wsh(sortedmulti(2, @0/48'/0'/0'/2'/<0;1>/*,
                   @1/48'/0'/0'/2'/<2;3>/*))     <-- same origin, DIFFERENT use-site path

  same origin class (both declare 48'/0'/0'/2')          ✓ one class
  equal multiplicity (1 each)                            ✓ passes (i)
  every occurrence of each in the SAME ONE sorted group  ✓ passes (ii)
  => the test permits FREE SEATING

  @0=cardX @1=cardY   bc1q28cl6v9qpxhzn524txgn7twl8ac4fvhxukr5zq6zuu6ddwdxlrjshaxjk3
  @0=cardY @1=cardX   bc1q6n2xk83pjztu8apensgn6gzzgvldkfcc0054kmftexan6hfj692sr9yngh
```

Two wallets. Control, identical use-site paths — the case the test is actually for:

```
wsh(sortedmulti(2,@0/…/<0;1>/*,@1/…/<0;1>/*))
  @0=cardX @1=cardY   bc1qf4jpv99wj36eqez9fzxrzww6sdy97uw5gmgp38t6trqpk5lre8qsv3ttqz
  @0=cardY @1=cardX   bc1qf4jpv99wj36eqez9fzxrzww6sdy97uw5gmgp38t6trqpk5lre8qsv3ttqz   (invariant)
```

**Why it slips the predicate.** `sortedmulti` sorts the *derived* keys. Swapping two cards
between slots with different use-site paths changes *which xpub is derived at which chain*,
so the derived key set itself differs — sorting cannot recover what derivation already
changed. The predicate's two clauses both hold, because both are about where the slot sits,
and nothing about where the slot derives.

**This is a real engraved-card shape, not a template-text artifact.** The per-slot use-site
path survives the md1 wire format — `md encode` mints
`md1ypfdsssj5qqcy8ppgz6vpgtqalk0p3c4wjw32`, `md decode` returns
`wsh(sortedmulti(2,@0/<0;1>/*,@1/<2;3>/*))`, and `md inspect --json` carries a populated
`use_site_path_overrides`. `md-cli/src/parse/template.rs` builds that override whenever a
slot's `UseSitePath` differs from `@0`'s. The seating engine will be handed exactly this.

**The pattern, now that there are three data points, is the finding.** r4(a) was
multiplicity, r4(b) was occurrence set, r5 is use-site path. Each round the predicate was
extended along the axis just measured, and the next axis was still open. The axes are not
being enumerated — they are being discovered one counterexample at a time. What makes two
candidate slots interchangeable is that **swapping them is a symmetry of the descriptor**,
and the checkable form of that is: identical `UseSitePath` (both fields — the multipath
alternatives *and* `wildcard_hardened`, which `make_use_site_path` carries per
occurrence), identical multiplicity, and every occurrence of each inside the same one
sorted group. Stating it as "swapping is a symmetry, and here are the three checks that
establish it" is what stops a fourth round: a reader can then test a new shape against the
*reason* rather than against a list.

I have no fourth construction. Under (i)+(ii)+(identical use-site path) the argument closes:
all candidate occurrences are positions of one sorted group, each candidate contributes the
same multiplicity, each derives identically, so every bijection yields the same derived-key
multiset at every index and therefore the same script at every index. Nothing outside the
group moves, by (ii).

---

## IMPORTANT

### I1 — A2 is now correct, but A3 and THE PRINCIPLE still partition the problem into "origin-equivalence classes", which declaration-as-constraint no longer produces.

A2's rewrite is right and the r4 counterexample now seats. But satisfaction is no longer
an equivalence relation, and two other places still assume it is:

```
THE PRINCIPLE: "cards and slots are grouped by declared origin (decoded values, rule A2);
                within one origin-equivalence class, free seating requires …"
A3:            "Where all of an origin-class's candidates lie in one sorted group …"
```

Under A2 a card can satisfy declarations that are *not equal to each other*. Measured — a
policy card may declare a fingerprint for some slots and not others, and `md` mints it:

```
$ md encode "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*))" \
      --fingerprint '@0=73c5da0a'
md1ypfdsssj5qqcy8ppgtgfeutks20pgxut2g35g3z
  origins:
    @0: [73c5da0a/48'/0'/0'/2']      <-- fingerprint declared
    @1: m/48'/0'/0'/2'               <-- fingerprint-free
```

Card A (fp `73c5da0a`, path P) satisfies **both** declarations. Card B (fp `b8688df1`,
path P) satisfies **only** `@1`. There is exactly one complete matching — B→`@1`, A→`@0` —
and it should seat with no ambiguity.

An implementer following the class language partitions by declared-origin value: `@0` into
class `(73c5da0a, P)`, `@1` into class `(None, P)`, card B into class `(b8688df1, P)` —
which contains no slot. A4 then refuses `@1` as unfilled and B as a leftover. **That is r4
I1's failure surviving in the mixed-declaration case**, through the framing rather than
through A2.

The fix is framing, not policy: A2 defines a bipartite *satisfaction* relation between
cards and slots, so A3's ambiguity analysis is over that graph — a card set seats freely
when every complete matching yields the same wallet, which is THE PRINCIPLE stated exactly
as it already is. Dropping "origin-equivalence class" for "the candidate slots a card
satisfies under A2" costs two phrases and removes the contradiction. A mixed-declaration
vector row belongs with it, since that is the shape that exposes the difference.

---

## MINOR

* **M1 — say that the practical test is a CONSERVATIVE approximation, not the principle's
  `iff`.** THE PRINCIPLE is an equivalence; the practical test is (correctly) stronger.
  Measured over-strictness: two group *instances* sharing placeholders is invariant yet
  refused —

  ```
  tr(@2,{sortedmulti_a(2,@0,@1),sortedmulti_a(1,@0,@1)})   all slots at one origin
    @0=cardX @1=cardY   bc1pkf63f99emlz7hcg328xp9fewt39nz0frhqykklp85en5j73v5t5s4v36dk
    @0=cardY @1=cardX   bc1pkf63f99emlz7hcg328xp9fewt39nz0frhqykklp85en5j73v5t5s4v36dk
  ```

  (ii) fails — @0's occurrences span two group instances — so the engine refuses a case
  the principle permits. That is the right direction and `--seat` is the escape, but the
  spec should say the test is deliberately conservative. Otherwise a later reader sees the
  gap against the `iff` and "corrects" it by loosening, which is how C1 gets reopened. One
  sentence: *refusing more than the principle requires is intended; refusing less is a
  defect.* (Confirms, incidentally, that "the SAME ONE sorted group" means the same group
  *instance* — the pressure point's question. That reading is the safe one and is what the
  measurement above assumes; making it explicit costs three words.)

* **M2 — name the CE-1 residue that declaration-as-constraint widens.** A fingerprint-free
  declaration now "accepts any card at that path". Under the old symmetric rule a *foreign*
  fingerprint-bearing card bounced off such a declaration; now it seats. The trade is
  correct and unavoidable — the policy card carries no fingerprint to discriminate with, so
  the old behaviour blocked the legitimate case (r4 I1) and the foreign one by the same
  accident, and a fingerprint-free *foreign* card always seated regardless. But the rule's
  rationale currently reads as pure gain ("information, not a mismatch"). It should say
  that a fingerprint-free declaration is the policy author's accepted risk: it buys privacy
  and mixed-mint restorability at the cost of the only per-slot filter the engine has, and
  the residue is CE-1's, surfaced by B2's address check. The B1 disposition already handles
  such a card as shape-confirmed at best, so nothing else changes.

---

## Gate

**RED — 1 Critical, 1 Important.** One more round, and it should be the last.

Everything outside C1 and I1 is now clean, and the sweep says so mechanically rather than
impressionistically: no stale model references anywhere in either document, matrices
byte-identical, headings unique, every cross-reference resolving. Five of the seven r4
items closed with nothing left over, and the two that did not are one substantive gap and
one follow-through.

1. **C1** — add identical-use-site-path as the third clause, and state the predicate's
   *reason* (swapping is a symmetry) above the three checks, so the next unusual shape is
   tested against the argument instead of against a list. The construction above becomes a
   third refusal row alongside the two r4 ones.
2. **I1** — replace "origin-equivalence class" with the A2 satisfaction relation in THE
   PRINCIPLE and A3; add the mixed-declaration row.
3. **M1, M2** — two sentences, riding the same fold.

Scope r6 to those four items and to whatever the fold newly writes. Nothing in the
disposition table marked **yes**, and none of r4's *Blessed* items, needs revisiting.
