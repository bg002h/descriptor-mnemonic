# R0 round 3 — SPEC_wallet_form_converter.md, fold review

**Artifact:** `design/SPEC_wallet_form_converter.md` @ `fc9fd142`
(`git diff ef35d4c6..fc9fd142 -- design/` is the r2 fold; mk companion unchanged at
`mnemonic-key@bcd8505`).
**Question:** does the fold resolve each r2 finding without a new defect?
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED.**
**Counts: 4 Critical / 4 Important / 4 Minor.**

r1 and r2 measurements and *"What was NOT found"* stay settled and were not re-derived.
Nothing in any repo was modified.

**Scope note.** The fold rules 13 of r2's 15 findings. r2's late additions — **I7** (the mk
FOLLOWUPS entry's mechanism claim) and **M6** (the lockstep enumeration) — are un-ruled;
`mnemonic-key/design/FOLLOWUPS.md` is unchanged since `bcd8505`. They are carried forward
below as **I4** and, since M6's prediction has now come true, as part of **I3**.

---

## 0. Fold disposition

| r2 | resolved? | note |
| --- | --- | --- |
| C1 | **no** | the refusal is restored and the classification is much better — but two paths still seat silently across position boundaries that change the wallet (**C1** below), and the prescribed remedy does not exist (**C4**) |
| C2 | **no** | equality swung from too coarse to too strict; the new definition cannot pass its own walk (**C2** below) |
| I1 | **no** | two-tier moves the false refusal rather than removing it (**C3**); the rule is also internally incoherent (**I2**) |
| I2 | **partial** | per-datum precedence is now right; P1 still opens by asserting the spelling it later says does not parse (**I1**) |
| I3 | **partial** | the four script forms + internal key are classified; the cross-group case falls through the partition (folded into **C1**) |
| I4 | **yes** | UNGATED in P3, Gates, and brainstorm decision 3 — all three checked |
| I5 | **yes** | all four stale sites swept; verified by grep (`1,649-char`, `read-side siblings`, `did NOT verify`, brainstorm decision 4) — zero hits |
| I6 | **yes** | equality checker → P2, D oracle → P3, stated in Acceptance 1 |
| I7 | **not ruled** | carried → **I4** |
| M1 | **yes** | one `## Non-goals` heading |
| M2 | **yes** | "TOP 4 BYTES of the policy card's 16-byte template id" |
| M3 | **yes** | policy-side collapse stated, pinning row required |
| M4 | **yes** | inline-`h` misdirection added to P1's scope |
| M5 | **yes** | fixture pinned to `keys.txt`'s first three lines — the wallet r2 verified end to end |
| M6 | **not ruled** | carried; its prediction is now realised — see **I3** |

Also verified still holding from earlier rounds: the two matrices remain byte-identical
(diffed), and Acceptance 1(d)'s pinned fixture is the one r2 measured through
`mk encode --keys` + `md encode --key`.

---

## CRITICAL

### C1 — The restored refusal does not cover two paths that still seat silently, and each changes the wallet.

Rule 3's rewrite is a real improvement: the guess is gone, `--seat` replaces it as an
explicit assertion, and the taproot internal key is now named as order-sensitive. Two
holes remain, one in the new text and one in text the fold did not touch.

**(a) Between two sorted groups — the classification partitions POSITIONS, but the hazard
is an ASSIGNMENT ACROSS groups.** Rule 3 defines order-INSENSITIVE as *"slots within one
`sortedmulti` or `sortedmulti_a` group"* and order-SENSITIVE as *"`multi`/`multi_a`
groups, taproot internal keys, and any position not in a sorted group"*. A slot in
sorted group B is in a sorted group, so it is not in the catch-all — every such slot is
classified order-insensitive, and rule 3 says seat freely. But the groups need not be
interchangeable with each other. Measured, all five slots at one shared origin, no
fingerprints:

```
tr(@0/48'/0'/0'/2'/<0;1>/*,{sortedmulti_a(2,@1…,@2…),sortedmulti_a(1,@3…,@4…)})
                            ^^ 2-of-2 leaf              ^^ 1-of-2 leaf: UNILATERAL spend

cards 1,2 -> grpA ; 3,4 -> grpB    bc1pgc0465sdn490zpde7kd6ec6h79j2ea2ac28lg8m4fa3canvxxnes4s2th4
cards 3,4 -> grpA ; 1,2 -> grpB    bc1p9z9kln4jc923rjc0rtz8fjdzl4lzw4e9vu40r8qx3s9kx25x56fq5r4sr2
cards 1,3 -> grpA ; 2,4 -> grpB    bc1pnqngqz2x5e0c9dqy4lj7xa5yp2zxtp09gmsx4gk2mmzff30rg38qeaguhv
```

Three wallets. And the misplacement is not merely different — a card that lands in the
1-of-2 leaf holds unilateral spending authority the operator meant to be shared. The
r2 taproot construction is now correctly refused; this sibling is not, because both
endpoints sit inside sorted groups.

The insensitivity that actually holds is *within a group*. The rule needs the assignment
predicate, not the position predicate: free seating only where every candidate position
for a card lies in the **same** group.

**(b) Rule 4 was not swept, and it contradicts the rewritten rule 3.** Rule 4 is verbatim
from the r1 fold:

> "**Privacy-preserving cards** (no fingerprint) seat by path under rule 3's group rules;
> **refuse only when** the path matches slots in more than one non-interchangeable group,
> naming `--privacy-preserving` as the reason the tiebreak is unavailable."

Rule 3 now says fingerprint-less cards matching order-sensitive positions REFUSE. Rule 4
says refuse **only** in the two-non-interchangeable-groups case — narrower, exclusive,
and later in an explicitly ordered list, so it reads as the governing word for exactly
the card class C1 is about. r2 quoted this clause as the enabling text for the taproot
internal-key seating; the fold rewrote rule 3 and left the clause that permitted it. An
implementer following rule 4's "only when" reintroduces the r2 counterexample verbatim.

### C2 — Acceptance 1's new structural equality cannot pass its own walk. Measured.

The definition is now:

> "(a) decoded templates structurally equal after canonicalisation … AND (b) the slot→key
> assignments equal as DECODED VALUES (**per slot: same xpub bytes, same origin
> fingerprint+path**, same use-site paths)."

and the walk requires *"the 36-string split set and the 22-chunk keyed card compose to
EQUAL wallets by this definition"*.

The two cards declare different origins by construction — r1 C2 established it, and it is
unchanged:

```
KEYLESS policy card (the split set)      KEYED card (22 chunks)
  @0: [73c5da0a/48'/0'/0'/2']              @0: m/48'/0'/0'/2'   (no fingerprint)
  @1: [73c5da0a/48'/0'/1'/2']              @1: m/48'/0'/0'/2'
  @2: [73c5da0a/48'/0'/2'/2']              @2: m/48'/0'/0'/2'
  @3: [73c5da0a/48'/0'/3'/2']              @3: m/48'/0'/0'/2'
  @4: [b8688df1/48'/0'/0'/2']              @4: m/48'/0'/0'/2'
```

At `@1` alone: fingerprint `73c5da0a` vs **absent**, path `48'/0'/1'/2'` vs
`48'/0'/0'/2'`. Clause (b) fails at nine of eleven slots. The acceptance is unsatisfiable.

**This is the third round in which Acceptance 1 cannot pass, and the swing is the
diagnosis.** r1 required descriptor-string equality (too strict — origin brackets differ).
The r1 fold required address sampling (too coarse — r2 constructed two different-wallet
pairs that passed). This fold requires origins to match (too strict again — the same
disagreement r1 C2 measured).

The reason the target keeps moving is that **one relation is being asked to do two jobs.**
They are different relations and both are needed:

* **Cross-FORM equality** — "do these two card sets name the same wallet?" The origin
  declaration is metadata the two forms legitimately disagree about, so it is *not* part
  of this relation: templates structurally equal, and per slot the same xpub bytes and
  use-site path. That relation the fixture satisfies.
* **Round-TRIP equality** — "did decompose→re-encode preserve everything?" Here the origin
  MUST be included, because carrying it is the whole point of P3, and r2 C2 showed an
  origin-blind relation passes a wrong-origin, unsignable result.

Acceptance 1 currently applies the second to a walk that needs the first.

### C3 — Rule 1's true-binding tier refuses the very card r2 I1 was raised to protect. Measured.

Rule 1 now accepts a card whose stub matches EITHER the policy card's template-id top-4
OR, *"verified POST-SEAT, the composed wallet's WalletPolicyId"*, and *"matching neither
refuses"*.

`WalletPolicyId` is origin-sensitive — `md-codec/src/identity.rs` describes it as "template
tree plus per-`@N` origin / use-site / fp / xpub records", "presence-significant on
fingerprint and xpub axes". The fixture's two card forms declare different origins (C2
above), so they have different policy ids:

```
KEYED card as engraved            wallet-policy-id: 232214e4d60c0fa83a6715ba2f7e8ec7
COMPOSED from the split set       wallet-policy-id: ced2270948ecb5af0779249ac7181f4a
  (same 11 keys, same script; only the declared origins differ)
policy card's template-id top-4:  5b48af35
```

Now walk r2 I1's case. An operator mints key cards with `mk encode --from-md1 <the keyed
card>` — measured to stamp stub `232214e4`. They compose the split set:

* shape tier: `232214e4` ≠ `5b48af35` → fail
* true-binding tier: `232214e4` ≠ `ced22709` → fail
* "matching neither refuses" → **the legitimate card is refused.**

Same wallet, same keys, same shape. r2 I1's defect is relocated, not closed: the flat
template-id check refused it with a false "different SHAPE" message; the two-tier check
refuses it with a false "bound to a different wallet" message.

The root cause is that WalletPolicyId identifies *a wallet together with one origin
declaration*, and this constellation has two legitimate declarations for one wallet. Any
binding rooted there inherits the split. (Worth carrying to
`stub-keyed-wallet-binding-at-mint`: the filed upgrade proposes keyed-WalletPolicyId
stubs and inherits the same property — see M4.)

### C4 — `--seat`, the sole remedy the C1 refusal directs every operator to, is undefined and unowned.

It occurs **once** in the entire cycle — inside rule 3's refusal string, plus one sentence
of gloss:

```
line 149:  explicitly with --seat '@i=<card prefix>'"*. `--seat` is the
line 150:  explicit escape hatch: the operator asserts intent; the tool never
line 151:  guesses it.
```

Not in P1, P2 or P3 — so no piece owns it. Not in the brainstorm or FOLLOWUPS. It is
nonetheless load-bearing: every order-sensitive ambiguity now terminates in a refusal
whose remedy is this flag.

**The funds-shaped question the spec does not answer: what does `--seat` bypass?** The
gloss — *"the operator asserts intent; the tool never guesses"* — invites the reading that
an explicit assignment overrides the engine. If `--seat` bypasses rule 2's decoded-origin
match, it is an operator-driven wrong-wallet path with no guard at all; if it bypasses
rule 1's stub check, it defeats the shape gate. Neither is stated. The correct answer is
almost certainly "`--seat` resolves ambiguity ONLY — every other rule still applies, and a
`--seat` that contradicts rule 1 or rule 2 refuses" — but that sentence is the one that
must be written, and it is not.

**`<card prefix>` is undefined on a measurable axis.** Across the fixture's 11 cards / 30
strings:

```
first  5 chars:  1 distinct value  (mk1qp — every card and every string)
first  6 chars: 10 distinct values across 11 cards   <-- one prefix names TWO cards
first  7 chars: 11 distinct values
```

A card's chunks share the first 10 characters and diverge at the 11th
(`mk1qpd8cwpq…` / `mk1qpd8cwpp…`), so the *card* identifier is the 10-char chunk-set
prefix — but only for chunked cards; a single-string card carries no chunk-set-id at all.
So the spec must say: minimum length or exact-match rule, what an ambiguous prefix does
(refuse, naming both cards), and whether the prefix names a string or a card. It says none
of these, and 6 characters is already ambiguous in the spec's own fixture.

Finally, the spec's own clause *"Every rule above ships as executable vector rows in the
SAME commit as its implementation"* has nothing to attach for `--seat`, since it is not a
rule but a flag introduced inside a message.

---

## IMPORTANT

### I1 — P1 still opens by asserting the spelling it later proves does not parse.

Within one paragraph:

> line 189: "Inline template origins (`[fp/path]@i/...`) already work on
> `md descriptor`/`md address` — measured on all 11 pathological slots."
> line 194: "(and the r1-fold's `[fp/path]@i/...` spelling does not parse — md refuses it
> by name)"

The correction landed and the claim it corrects did not. The false statement comes first
and is the one an implementer copies. The working spelling — measured in r2 — is
`@i/48'/0'/0'/2'/<0;1>/*` with the fingerprint supplied separately, which is exactly what
md's own refusal message tells the operator to write.

### I2 — Rule 1 is internally incoherent: a flat refusal it then overrides, and a "first" rule with a post-seat component.

Two problems inside one numbered rule:

* **The flat check is never withdrawn.** Line 93 states *"The check: at least one of the
  card's stubs … matches the TOP 4 BYTES of the policy card's … template id. A mismatch
  refuses"* — then 15 lines later the two-tier paragraph accepts a card that does not
  match it. Read top-down the rule contradicts itself, and the first statement is the
  implementable one.
* **The ordering is unstated.** The list is introduced as *"Rules, in order"* and rule 1
  is first, yet tier 2 is *"verified POST-SEAT"* — it cannot run until rules 2–5 have
  produced an assignment. The spec never says what the pre-seat phase does with a card
  that matches the template-id tier but not… or matches neither *yet*. As written, a
  strict reading of "rules, in order" refuses at rule 1 every card that will later match
  tier 2, which is C3's failure by a second route. Rule 6 is titled "Post-seat
  verification", so there are now two post-seat checks, one of them numbered 1.

A two-phase statement (pre-seat shape gate; post-seat binding verification, listed with
rule 6) would be coherent. The current numbering is not.

### I3 — Acceptances 2 and 3 were not swept for the two-tier rule 1. (r2 M6, realised.)

r2 M6 warned that the flip list named 2 of at least 5 sites. The two-tier change has now
touched rule 1 and nothing else:

* **Acceptance 2** still describes CE-1 flatly — *"a same-stub foreign card seats and the
  derived address differs; the row asserts BOTH halves"* — while rule 1 now scopes CE-1
  *"to shape-tier cards only"*. The row's precondition is missing from the acceptance that
  gates it.
* **Acceptance 3** still requires *"the same-shape case proven to seat"*. Under two-tier
  that is true only for a shape-tier card; the same-shape **keyed-mint** card is refused
  (C3). The acceptance would be written to assert a behaviour rule 1 forbids.

Both are one-line edits, and both are exactly the propagation class r2 I5 documented and
this fold otherwise swept well.

### I4 — Carried from r2 I7, un-ruled: the mk entry misdescribes the mechanism it proposes to change, and omits a third repo.

`mnemonic-key/design/FOLLOWUPS.md` → `stub-keyed-wallet-binding-at-mint` is unchanged at
`bcd8505`. Restating r2's measurements, unaltered:

* The formula is **not** WalletPolicyId-rooted for a keyless md1 — toolkit #28 made the
  dispatch FORM-AWARE. Live: `mk encode --from-md1 md1yqpqqxzq2qwfv8urt848e` stamps
  `559e64b2` (WalletDescriptorTemplateId); v0.8.0's `3d190af3` is named in
  `mk-cli/tests/template_id_stub.rs` as "the pre-#28 (**BUGGY for a template**) value",
  with a live assertion that the stub does not equal it.
* It is **not** for want of keys: `compute_wallet_policy_id` runs on a keyless descriptor
  and discriminates on origins and fingerprints alone (`073e6088` vs `04829c78` for two
  same-template keyless cards).
* `mnemonic-toolkit` mints the same stubs (`crates/mnemonic-toolkit/src/cmd/bundle.rs:1239
  bundle_binding_stub`), so the flip is a three-repo lockstep and the entry names two.

Rule 1 in the spec is unaffected; this is a defect in the filed plan.

---

## MINOR

* **M1** — Rule 1 still says CE-1's residual *"is caught the same way: the address-0
  verification in rule 6"*, while rule 6 says that for a card set with no keyed card
  *"nothing external exists"*. The fold removed this exact overstatement from rule 3 and
  left it in rule 1. "Surfaced for human comparison" is what rule 6 offers; "caught" is
  not.
* **M2** — `--seat` has no vector row, though the clause below the rules requires one for
  every rule shipped. Once C4 defines it, the ambiguity-resolved and
  `--seat`-contradicts-rule-2 cases both want rows.
* **M3** — Acceptance 1 assigns the equality checker to P2 ("it IS rule 6's split-vs-keyed
  'agree'"), but P2's own paragraph lists only `--from-mk1` / `--from-mk1-file`. A reader
  scoping P2 from its own text will not budget the checker.
* **M4** — The filed `stub-keyed-wallet-binding-at-mint` upgrade inherits C3's root cause:
  a keyed-WalletPolicyId stub binds a wallet *plus one origin declaration*
  (`232214e4` vs `ced22709` for one wallet, measured), so the same "legitimate card,
  wrong declaration" refusal will reappear at mint time unless the entry addresses it.
  Worth adding to the entry before it is scheduled, while it is cheap.

---

## Gate

**RED — 4 Critical, 4 Important.** The loop does not close.

**The fold is directionally right and most of it landed.** 11 of 15 r2 findings are
genuinely closed, several of them well: the propagation sweep (I5) is complete, the gating
is unified across all three statements (I4), the classification names the four script
forms and the internal key (I3, partially), and the ownership assignments (I6) are the
right ones. C1's core ruling — refuse rather than guess, with an explicit operator
override — is the correct design.

**What is left is one pattern, appearing four times: a rule was corrected at the site the
finding cited, and the correction's consequences were not followed out.** Rule 3 was
rewritten and rule 4 was not (C1b). The position classification was made exhaustive over
positions but the hazard is over assignments (C1a). Equality was moved off addresses but
onto a field the two card forms disagree about (C2). Rule 1 gained a tier but not a phase
model, and its acceptances did not move with it (C3, I2, I3). And the remedy invented to
carry C1's refusal was never given a definition (C4).

Shortest path to GREEN:

1. **C1** — make the free-seating predicate *"all candidate positions for this card lie in
   one sorted group"*, and delete or rewrite rule 4's "refuse only when" clause.
2. **C2** — split the relation: cross-FORM equality excludes the origin declaration,
   round-TRIP equality includes it. The fixture satisfies the first; P3's leg needs the
   second.
3. **C3** — tier 2 cannot be rooted in an origin-sensitive id if the two card forms may
   declare different origins. Either compare against the policy id of the declaration in
   hand for each form, or drop tier 2 and accept the shape tier plus rule 6.
4. **C4** — define `--seat`: identifier syntax and ambiguity behaviour, owning piece, and
   above all the sentence saying it resolves ambiguity only and never bypasses rules 1, 2
   or 5.
5. **I1, I2, I3** are localised; **I4** is a correction to the mk entry and can land
   independently.
6. **M1–M4** ride the same fold.

Re-dispatch after the fold. Nothing marked **yes** in the disposition table needs
revisiting; scope r4 to the twelve items above and to whatever the fold newly writes.
