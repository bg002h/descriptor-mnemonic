# R0 round 6 — SPEC_wallet_form_converter.md, fold review (the principle made executable)

**Artifact:** `design/SPEC_wallet_form_converter.md` @ `827480c5`
(`git diff 3a679d4b..827480c5 -- design/`).
**Question:** does the executable principle resolve r5's C1/I1/M1/M2 without a new defect?
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED (0C / 3I).**
**Counts: 0 Critical / 3 Important / 3 Minor.**

r1–r5 measurements and r4's *Blessed* items stay settled. Nothing in any repo was modified.

**Lead with the result, because it is the one that matters.** The rewrite is right, and it
is right in the direction that counts. **Compose-canonicalise-compare is SOUND for
seating:** if the comparison forms are byte-equal then the descriptors carry the same key
expressions at the same positions modulo sorted-group internal order, hence the same script
at every derivation index, hence the same wallet. I attacked it on the axis that killed
every previous formulation and it holds — r5's use-site-path swap produces two different
key-expression multisets, so the forms differ and the procedure refuses. **The axis-chasing
is over**: no structural axis can be missed, because no structural axis is consulted. Four
rounds of Criticals end here.

What is left is three defects, none of which can emit a wrong wallet. All three are in the
safe direction — an unstable disposition, an unbounded enumeration, and a manufactured
refusal — and all three are in the machinery *around* the procedure rather than in it.

---

## 0. Fold disposition of the 4 r5 findings

| r5 | resolved? | check |
| --- | --- | --- |
| C1 (test ≠ principle) | **YES — at the root** | the clause test is gone; the check IS the principle. Verified against every construction this cycle produced: r5's `wsh(sortedmulti(2,@0/…/<0;1>/*,@1/…/<2;3>/*))` now refuses, because the two assignments compose to different key-expression multisets ({X/<0;1>, Y/<2;3>} vs {Y/<0;1>, X/<2;3>}) and sorting within the group cannot merge them. r4's repeated-slot and internal-key cases likewise: both compose differently, both measured |
| I1 (equivalence-class frame) | **YES in THE PRINCIPLE and A3** | "origin-equivalence classes are the wrong frame and are gone from this spec"; A2 is a bipartite satisfaction relation; the mixed-declaration case now has a unique perfect matching and seats. **But the word returns in the cap** — see I2 |
| M1 (conservative, not `iff`) | **partially — see M1** | the clause test's over-strictness is gone and r5-M1's case now seats. The replacement carries a *new* equivalence over-claim ("exactly wallet-equality") that is false in the same harmless direction |
| M2 (fp-free residue) | **yes** | named in A2 as the policy author's accepted risk, tied to `md encode`'s mint-time warning, with the converter's inheritance of that choice stated |

Structural sweep, end to end: matrices **byte-identical**; nine unique `##` headings in
order; no `two-tier`, no `rule N`, no `Rules, in order`, no `per-position`; the only
surviving `origin-equivalence` mention is the deliberate deletion note at line 105; every
`A1`–`A5`/`B1`/`B2` reference outside NORMATIVE resolves.

---

## IMPORTANT

### I1 — "Seat any" makes B1's disposition nondeterministic: the same card set yields wallet-confirmed or an unconfirmed WARNING depending on which equal matching the enumerator returns.

A3 proves several matchings wallet-equal and says *"all equal ⇒ seat any (a free seat,
PROVEN)"*. B1 then computes *"the composed wallet's WalletPolicyId under the supplied
origin declarations"* from whichever assignment was seated. Those are not the same
invariance. Measured:

```
wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*))
  both slots fingerprint-free at one path -> both matchings satisfy A2

  addr(@0=X,@1=Y) = bc1qf4jpv99wj36eqez9fzxrzww6sdy97uw5gmgp38t6trqpk5lre8qsv3ttqz
  addr(@0=Y,@1=X) = bc1qf4jpv99wj36eqez9fzxrzww6sdy97uw5gmgp38t6trqpk5lre8qsv3ttqz   <- same wallet
                                                                                        A3: seat any

  WalletPolicyId(@0=X,@1=Y) = 568989eaf0344d86c41b9d02e2708f03
  WalletPolicyId(@0=Y,@1=X) = 13415d4759ac9cfb53de2d851282b727   <- B1's input, DIFFERENT
```

A card carrying stub `568989ea` is **wallet-confirmed** if the enumerator happened to
return the first matching and draws the **unconfirmed WARNING** if it returned the second —
same cards, same wallet, same declarations, decided by a tie-break A3 declared immaterial.
`wallet-confirmed` is the strongest claim this engine makes (*"true binding — a foreign card
cannot reach this tier"*), and r3 C3 was spent getting it right; leaving it on a coin flip
undoes part of that.

The direction is safe — no *false* confirmation becomes reachable, since a foreign stub
still has to hit one specific 4-byte value — so this is Important, not Critical. But the
failure an operator sees is a legitimate card downgraded to *"or a different wallet; verify
address 0 before trusting"*, which is precisely the warning that is supposed to mean
something.

Two clean closures, either sufficient: make B1 compute the WalletPolicyId from a
**deterministic canonical representative** of the equal set (A3 already has a canonical
comparison form; a canonical *choice* costs nothing more), or have B1's wallet-confirmed
test accept a stub matching the WalletPolicyId of **any** of the proven-equal matchings.

Same root, worth naming in the same fix: the emitted descriptor is also assignment-dependent
(`…#ljvy57cm` vs `…#shc29dps` above), so `md descriptor` on one card set can print two
different strings depending on the order the operator listed the cards, for a set A3
declares identical. stdout being reproducible for a fixed card *set* rather than a fixed
card *sequence* is worth one sentence.

### I2 — The cap is stated in the frame this same fold deleted, and it does not bound the work.

> "Matching counts are capped (a class of k mutually-ambiguous cards contributes k!
> compositions; above a stated bound — 720, i.e. k>6 in one class — the engine refuses and
> says the bound…)"

`class` is the word THE PRINCIPLE deletes eleven lines earlier as *"the wrong frame"*, and
it is undefined here for the same reason: A2's satisfaction graph need not decompose into
classes, and a card's neighbourhood need not equal any other card's. Worse, a per-class
factorial bound is not a bound on the enumeration:

* **Under-strict.** Two independent 6-card components give 720 × 720 = **518,400** perfect
  matchings, with no "class" exceeding 6 — no refusal fires, and the engine must compose
  and canonicalise half a million descriptors. Three such components give 373 million.
* **Over-strict.** A single connected component of 8 cards where card *i* satisfies only
  slots *i* and *i+1* has **2** perfect matchings, yet reads as one class of k = 8 > 6 and
  refuses.

The quantity that needs bounding is the **total number of perfect matchings enumerated**,
which is well-defined on the graph and is what actually costs. Enumerate with early
termination — walk to N+1 and refuse above N — so the bound is enforced without first
counting (counting bipartite perfect matchings is #P-complete in general; enumerating to a
cap is not). Then k! and "class" can both go, and the cap survives the frame change that
removed everything else built on classes.

### I3 — A3's policy-side collapse cannot run on the split-card path, so the dedupe can still manufacture the unfilled-slot refusal it was written to prevent.

> "Identical (origin, xpub) pairs collapse to one key, on the policy side too (r2 M3),
> pinned by a row."

r2 M3's purpose was that *"the dedupe never manufactures an unfilled-slot refusal"*. The
policy-side half of that guarantee needs the policy to carry xpubs — and on P2's path the
policy card is **keyless** by construction: its slots declare origins only. So the
policy-side collapse is unavailable exactly where the composition happens.

The consequence is reachable. `md` composes a policy with the same key at two distinct
slots:

```
$ md descriptor --template "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*))" \
      --key @0=<X> --key @1=<X>
wsh(sortedmulti(2,xpub661MyMwAqRbcGQnC8zMG…/<0;1>/*,xpub661MyMwAqRbcGQnC8zMG…/<0;1>/*))   exit 0
```

Now the split set for that wallet: two card scans of the one key (or two engraved copies)
carry identical `(origin, xpub)` and collapse to **one** key; the two slots declare the same
fingerprint-free origin and cannot collapse, because a keyless card has no xpub to compare.
One card, two slots ⇒ **zero perfect matchings** ⇒ A4 refuses `@1` as unfilled. The dedupe
produced the refusal the clause promises it never will.

Safe direction, and the wallet is degenerate (a 2-of-2 holding one key twice is a 1-of-1),
so this is Important on the "unsound claim / missing case" reading rather than on impact.
The fix is one clause and is clearly right whatever the shape's merits: **dedupe cards only
down to the multiplicity the policy requires at that declaration** — never below the slot
count the matching needs. State the guarantee conditionally too, since as written it claims
something the keyless path cannot deliver.

---

## MINOR

* **M1 — "exactly wallet-equality" is an equivalence claim, and only one direction holds.**
  Byte-equality ⇒ wallet-equality is sound and is the direction seating depends on. The
  converse fails, because taptree branches commute and the comparison form sorts *within*
  group instances, not *across* them:

  ```
  tr(@0,{sortedmulti_a(2,@1,@2),sortedmulti_a(2,@3,@4)})   all five slots at one origin
    order 0 1 2 3 4   addr=bc1pfv38mkt0q0twjhgkvvzpu5u7yfaarguqsjntgs5hsq2f46kar66skzrayn
    order 0 3 4 1 2   addr=bc1pfv38mkt0q0twjhgkvvzpu5u7yfaarguqsjntgs5hsq2f46kar66skzrayn
    composed descriptors DIFFER: …#k028wqu9  vs  …#4fantml7
  ```

  Same wallet, different comparison forms ⇒ the procedure refuses a card set that is in fact
  invariant. That is the right direction and `--seat` is the escape, but the claim should
  say so: **sound, and deliberately conservative — refusing more than the principle requires
  is intended; refusing less is a defect.** This is r5 M1's point surviving into the new
  procedure; an unqualified "exactly" is what invites a later reader to close the gap by
  loosening, which is how C1 was reopened three times.

* **M2 — acceptance 2 gates refusals and dispositions, but not the SEAT-proving rows the
  procedure now requires.** THE PRINCIPLE promises that r5-M1's case "ships as a row proving
  the procedure SEATS what the clauses wrongly refused". Acceptance 2 reads "Every seating
  refusal AND every B1 disposition demonstrated by a vector row that FAILS if the behaviour
  is removed" — a seat is neither. The one row whose whole purpose is to catch a future
  re-tightening therefore has no acceptance clause behind it. Add "and every proven free
  seat" to acceptance 2.

* **M3 — say what the over-cap refusal can print.** The cap's remedy line says "`--seat`
  being the remedy there too", but above the cap the matchings were never enumerated, so the
  refusal cannot list candidate assignments. It *can* still list the ambiguous cards by
  chunk-set id and each card's candidate slots — both are properties of the graph, not of
  the enumeration — which is exactly what `--seat` needs. One sentence keeps the over-cap
  refusal actionable instead of leaving an implementer to discover the shortfall.

---

## Gate

**RED — 0 Critical, 3 Important.** Not GREEN under the 0C/0I rule; one more round.

But the character of this round is different from the five before it, and that is worth
recording plainly for whoever reads the series. **The funds-shaped core is settled.** The
seating decision no longer reasons about structure at all, so the failure mode that
generated a Critical in r2, r3, r4 and r5 — an invariance axis nobody had thought of — is
not merely patched but *unreachable*, and I could not construct against it. The soundness
argument is short enough to check by eye: equal comparison forms ⇒ equal key expressions per
position modulo sorted-group order ⇒ equal script at every index. That is the property the
whole cycle was trying to buy.

Everything still open is in the safe direction and outside the procedure:

1. **I1** — give B1 a deterministic (or matching-independent) WalletPolicyId, so the
   disposition stops depending on a tie-break A3 called immaterial.
2. **I2** — bound the enumeration, not a "class"; drop the last user of a frame this fold
   deleted.
3. **I3** — dedupe never below the multiplicity the policy needs; state the collapse
   guarantee conditionally.
4. **M1–M3** — three sentences, riding the same fold.

None of the four requires new design, and none touches compose-canonicalise-compare. Scope
r7 to these six items and to whatever the fold newly writes; if they land as described, the
next round should close.
