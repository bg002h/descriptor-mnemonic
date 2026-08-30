# R0 round 7 — SPEC_wallet_form_converter.md, fold review

**Artifact:** `design/SPEC_wallet_form_converter.md` @ `12353cb8`
(`git diff 827480c5..12353cb8 -- design/`).
**Question:** do the r6 folds resolve the three Importants and three Minors without a new
defect — GREEN?
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED.**
**Counts: 1 Critical / 2 Important / 0 Minor.**

r6's blessing of compose-canonicalise-compare stands settled and was not re-derived; r1–r5
measurements stand. Nothing in any repo was modified.

**The three Minors close. Of the three Importants, one closes, one is fixed by a mechanism
that cannot work by construction, and one was fixed in a direction that opens a
wrong-wallet path.** Both pressure points the brief raised are real, and both are measured
below.

---

## 0. Fold disposition

| r6 | resolved? | check |
| --- | --- | --- |
| I1 (nondeterministic B1) | **no — see I1** | "seat any" → "the matching whose canonical-comparison form is lexicographically least". Measured: on the branch where this rule is used, all candidate matchings share **one** comparison form by construction, so the order never discriminates and the tie is exactly as arbitrary as before |
| I2 (per-class cap) | **yes**, with a naming consequence — see I2 | the bound is now TOTAL matchings enumerated, 720, early-terminating at the 721st; the per-class framing and its `k!` are gone, and both r6 constructions are cited as the reason |
| I3 (dedupe manufactured a refusal) | **no — see C1** | collapse-as-deletion became a capacity node that "may fill **any number** of slots whose declarations it satisfies". That seats r6's case correctly and seats a wallet the operator does not own when a card is missing |
| M1 ("exactly" over-claim) | **yes** | "SOUND for wallet-equality and deliberately conservative — the converse fails on e.g. taptree branch commutation, measured, which the engine treats as inequality and refuses" |
| M2 (seat rows ungated) | **yes** | acceptance 2 now reads "every seating refusal, every B1 disposition, AND every PROVEN-FREE-SEAT case … removed **or inverted**", naming r5-M1's two-instance case, r6-I3's capacity case and the mixed-declaration matching |
| M3 (over-cap refusal) | **yes** | prints the cards and their candidate slots, "graph properties it has even when the matchings are uncounted" |

Structural sweep: matrices **byte-identical**; nine unique `##` headings; the only surviving
`class` mentions are the deliberate deletion notes (lines 105, 125–128) and two unrelated
uses ("danger class", "F-420 class").

---

## CRITICAL

### C1 — The capacity node seats a wallet the operator does not own whenever a card is missing from a fingerprint-free policy. Measured, on the most likely restore scenario this feature has.

A3 now reads:

> "Identical cards collapse to one NODE WITH CAPACITY — the node **may fill any number of
> slots whose declarations it satisfies**, the composed descriptor repeating the key at
> each"

Unbounded capacity means a card can fill slots the operator supplied no card for. The
shape that exposes it is the ordinary privacy-preserving multisig — three devices, all at
`48'/0'/0'/2'`, no fingerprints — which is exactly the shape r4 I1 and r5 M2 worked to make
restorable:

```
policy card (keyless, fingerprint-free, three slots at one path):
  md1yzfdsssj5qqcy8pzrqrxahye32v7pju
  = wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/…/*,@2/…/*))

the operator's REAL wallet  (cards X, Y, Z):
  bc1q2sz6vvu6k7y9gtc6kfgfe0p6xkhmvmdlu97eecjkykpdktvps08scdjgr5

the operator supplies only X and Y — the third plate is lost, or simply not to hand.
X satisfies all three declarations, so a COMPLETE cover exists: X->{@0,@1}, Y->{@2}.

what the engine composes:  sortedmulti(2, X, X, Y)
  bc1qc6ssvugf29fq9z78d9559tuyplxlf06cwt57zc8uquamnte00w7qeqfts6
```

Every gate passes. **A4** is satisfied — every slot filled, every supplied card seated.
**A3's compare step** is satisfied *with full confidence*, because all covers of the
two-card set are wallet-equal:

```
  X->{@0,@1}, Y->{@2}   bc1qc6ssvugf29fq9z78d9559tuyplxlf06cwt57zc8uquamnte00w7qeqfts6
  X->{@0,@2}, Y->{@1}   bc1qc6ssvugf29fq9z78d9559tuyplxlf06cwt57zc8uquamnte00w7qeqfts6
  X->{@1,@2}, Y->{@0}   bc1qc6ssvugf29fq9z78d9559tuyplxlf06cwt57zc8uquamnte00w7qeqfts6
```

so the procedure reports "all equal ⇒ seat", which is the *strongest* thing it can say. No
ambiguity refusal, no unfilled slot, no warning from A3 or A4. The operator asked to
restore a 2-of-3 while missing one plate and received a checksummed descriptor for a
degenerate 2-of-3 holding one key twice — a wallet nobody owns, and one whose funds are
controlled by X alone.

Before this fold the same input produced the correct answer: one card, three slots, zero
complete matchings, **"slot @2 unfilled"**. The capacity change turned a correct refusal
into a confident wrong seat.

**Why the fold went this way, and where the boundary actually is.** r6 I3 asked that the
dedupe never collapse *below the multiplicity the policy requires*; the fold removed the
bound entirely instead. Both directions fail, because a **keyless policy card cannot
distinguish the two readings**: given two same-origin slots and one card X, the wallet may
use X twice, or may use X and a key the operator has not supplied. The card declares
origins only, so nothing in the input decides it. That is the irreducible ambiguity r6 I3
named, and unbounded capacity resolves it silently in the direction that fabricates a
wallet.

This is precisely the case THE PRINCIPLE exists for and the one place it currently cannot
see, because it quantifies over *complete covers* and the missing-card reading produces no
complete cover at all — so the ambiguity never reaches the compare step. Whatever the
ruling, it has to be made where the ambiguity is visible: a slot that only a
capacity-expanded node can fill is not the same evidential situation as a slot a distinct
card fills, and the spec should say so rather than let the cover enumeration absorb the
difference. A refusal that *names* the choice — "slot @2 can only be filled by repeating
card `<set-id>`; if that is your wallet, say so with `--seat`" — keeps the operator's
`--seat` remedy without the engine inventing the answer. I state the direction, not the
remedy; the row this needs is a must-REFUSE row alongside r6-I3's must-SEAT row, and the
pair is what pins the boundary.

---

## IMPORTANT

### I1 — "Lexicographically least comparison form" cannot select a matching: on the branch where it is used, every candidate shares one form by construction. r6 I1 survives intact.

The new text:

> "all equal ⇒ seat the CANONICAL matching — the one whose canonical-comparison form is
> lexicographically least, a stated total order"

The order is total on **forms**. It is applied to choose among **matchings**. Those are
different sets, and on this branch the map collapses: the branch is entered precisely when
all candidates' comparison forms are byte-equal. So the "least form" is shared by every
candidate and selects none of them.

Measured on r6 I1's own fixture — the comparison form computed as specified (composed
descriptor, key expressions sorted within the `sortedmulti` group instance):

```
wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*)), cards X and Y

  matching 1 (@0=X,@1=Y)  comparison-form sha256: 3d1c9cb4c6437a57…
  matching 2 (@0=Y,@1=X)  comparison-form sha256: 3d1c9cb4c6437a57…   IDENTICAL

  emitted descriptor 1: …)#ljvy57cm        emitted descriptor 2: …)#shc29dps
  WalletPolicyId 1: 568989eaf0344d86c41b9d02e2708f03
  WalletPolicyId 2: 13415d4759ac9cfb53de2d851282b727
```

Identical forms, different downstream. So B1's disposition is still decided by an arbitrary
tie-break: a card stubbed `568989ea` is *wallet-confirmed* on one arm and draws the
*unconfirmed WARNING* on the other, and `md descriptor` still prints two different strings
for one card set depending on the order the cards were listed. The finding is unchanged
from r6; only its stated remedy moved.

This is not a near-miss to be patched with a tighter form — the failure is structural. The
tie-break must be a total order on the **matchings themselves**, which are distinct objects
even when their forms coincide: the assignment vector (slot index → card chunk-set id) is
already distinct per matching and already totally ordered, so "least assignment vector
among the proven-equal set" is well-defined and needs nothing new. The brief's own question
— *does anything downstream differ between two matchings sharing one comparison form* —
answers itself above: the WalletPolicyId and the emitted text both do, which is why the
tie-break has to exist at all.

### I2 — The cap's unit is fixed, but capacity changed what is being enumerated, and A3 still calls it a perfect matching.

r6 I2 is genuinely resolved: the bound is now total enumerations with early termination at
the 721st, the per-class framing is gone, and both constructions are cited. The residue is
that the *object* moved underneath the word in the same fold.

With capacity nodes the structure is no longer a matching. A3 (line 106) still says:

> "A3 enumerates the **PERFECT MATCHINGS** of that graph"

and A3's body (line 173) repeats "perfect matchings of A2's satisfaction graph". A perfect
matching pairs each slot with a *distinct* card; a capacity node fills several. What A3 now
enumerates is a degree-constrained cover — each slot covered once, each node used up to its
capacity — and the space is shaped differently: for *n* slots over *k* nodes with unbounded
capacity it is up to *kⁿ*, not *k!*. For eight slots and three nodes that is 6,561 covers
where "matchings" suggests at most 6.

The 720 bound still fires and still terminates, so nothing runs away; but the normative
sentence names an object the procedure no longer computes, the reader cannot check the
bound's adequacy against the wrong space, and an implementer reaching for a bipartite
perfect-matching enumerator writes something that cannot express capacity at all. Restate
the object (cover, with capacities) wherever "perfect matching" appears, and keep the bound
where it is — the unit is right, only its noun is stale.

(The C1 ruling may remove capacity again, in which case "perfect matching" becomes correct
and this finding closes with it. Sequencing it after C1 avoids two edits.)

---

## Gate

**RED — 1 Critical, 2 Important.**

The Minors all closed and r6 I2 closed, so the fold's craft is not in question. What this
round shows is narrower and worth stating exactly: **two of the three remedies were chosen
without a counterexample run against them.** The canonical-matching rule fails on the very
fixture whose measurement was quoted in its own justification — one command would have
shown the two forms hashing identically. The capacity rule was written to satisfy r6 I3's
must-SEAT case and never tested against the adjacent must-REFUSE case, which is the more
common one by a wide margin: an operator restoring a multisig with a plate missing is the
ordinary event this whole feature exists to serve.

That is the same lesson the cycle has been paying for since r2, arriving now on the fold
side rather than the spec side: **a remedy is a claim, and claims in this document have
been machine-checkable throughout.** Both of this round's findings took one script each.

1. **C1** — bound capacity, or rule the ambiguity explicitly where it is visible; pair the
   must-SEAT row with a must-REFUSE row for the missing-card cover.
2. **I1** — order the matchings, not their forms; the assignment vector is already a total
   order and needs no new machinery.
3. **I2** — rename the enumerated object to a capacity-constrained cover (or let it close
   with C1 if capacity goes).

Scope r8 to these three and to whatever the fold newly writes. Nothing else in the
disposition table, and nothing in r6's blessing of compose-canonicalise-compare, needs
revisiting — I attacked the core again this round only where capacity and the tie-break
touch it, and the compare step itself continues to hold.
