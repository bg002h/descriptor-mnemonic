# R0 round 2 — SPEC_wallet_form_converter.md, fold review

**Artifact:** `design/SPEC_wallet_form_converter.md` @ `ef35d4c6`
(fold `6a5cf3ad`, matrix sync `6a51d667`, follow-up filing `ef35d4c6`;
mk companion `mnemonic-key@bcd8505`).
**Question:** does the fold resolve each r1 finding without introducing a new defect?
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED.**
**Counts: 2 Critical / 7 Important / 6 Minor.**

r1's *"What was NOT found"* stays settled and was not re-derived. r1's measurements
stand. Nothing in any repo was modified. Scope: the fold's own text, plus the two
pressure points named in the brief.

---

## 0. Fold disposition of the 22 r1 findings

Checked one by one against the folded text and, where the fold makes a factual claim,
against the tools.

| r1 | ruling folded as stated? | note |
| --- | --- | --- |
| C1 | **partial** | honest SHAPE framing landed, CE-1 row landed, wallet-binding claim deleted from rule 1 — but the form-aware stub split is unhandled (**new I1**) and the deleted claim survives in Motivation (**new I5**) |
| C2 | **partial** | acceptance rewritten to wallet equality; the relation chosen is too coarse (**new C2**) |
| C3 | **yes** | keys AS PARSED; verified: `mk encode --keys` accepts the depth-4 fixture key file, `md encode --key`+`--fingerprint` mints from it, addresses derive. Acceptance 1(d) runs. |
| I1 | **yes** | rule 1 now reads "the field is `Vec<[u8;4]>` … any-of" |
| I2 | **partial** | correct for the motivating case (verified below); over-reaches into multi-group and no-group policies (**new C1**), and omits `multi_a`/`sortedmulti_a` (**new I3**) |
| I3 | **yes** | canonical = `'`, measurements cited, cross-repo comparability deleted |
| I4 | **partial** | P3 says UNGATED; Gates and the brainstorm still say gated (**new I4**) |
| I5 | **yes** | origin-less keys excluded from the mintable set; `--emit commands` refuses naming them |
| I6 | **yes** | D = bare strings; JSON and receive/change pairs refuse with guidance; F-420 class named |
| I7 | **yes** | decoded-value comparison; the four spelling axes cited |
| I8 | **partial** | P1 correctly rescoped to the flag gap + precedence — but the stated inline syntax does not parse and the precedence rule's second clause is dead (**new I2**); Motivation's refuted premise survives (**new I5**) |
| I9 / N1 | **yes** | verified mechanically: the spec's and brainstorm's matrix rows diff **IDENTICAL** byte-for-byte |
| M1 | **partial** | acceptance 4 pins 1,648 chars / 1,649 bytes; Motivation still says "1,649-char" (**new I5**) |
| M2 | **yes**, with a gap | stream fixed (stderr), oracles added — but no piece owns them (**new I6**) |
| M3 | **yes** | mk pinned `93cebfb`; the no-`scripts/` note added |
| M4 | **yes** | repeated keys collapse on read AND write, with a pinning row |
| M5 | **yes** | new-walker budget written into P3 for the plan to inherit |
| M6 | **yes** | acceptance 3 now requires both halves |
| M7 | **yes** | leftover refusal names the stub |
| M8 | **yes** | message fix rides P1 |
| N2 | **yes** | acceptance 4 pins the 21×86 + 59 profile |

**Fully closed: 12 of 22.** The remaining ten are the two Criticals and six Importants
below (several r1 items share a fold defect).

---

## 0b. The `ef35d4c6` delta — the C1 upgrade filing

Asked: do the rule-1 pointer, the two FOLLOWUPS entries, and CE-1's row description
form a consistent story? **Mostly yes; two defects.**

**What is sound.** The operator ruling is recorded verbatim and identically in both
entries. The cross-citation convention is honoured in both directions. The compat
argument is correct and is the right reason to schedule this pre-v1.0. The spec's rule 1
no longer claims a compat burden it does not have, and the "flip in lockstep" instruction
is the right shape. My r1/r2 C1 measurement (`a235ee75` shared by two wallets) is cited
accurately in all three places.

**What is not.** The mk primary entry's mechanism claim is measurably wrong (**I7**), and
the lockstep enumerates 2 of at least 5 sites that must flip together (**M6**). Neither
touches rule 1's correctness as written, and neither changes the verdict — both Criticals
above are independent of this delta.

---

## CRITICAL

### C1 — Rule 3/4 replaced r1's refusal with a silent guess, in exactly the case where rule 6 has no oracle. Wrong wallet, no warning.

The fold's ruling for I2 was "sortedmulti groups seat freely; multi groups seat in
supplied order with a stated stderr warning + the address check as the confirm."

**The half that is right, and should be kept.** Verified — for a single `sortedmulti`
group the assignment genuinely is immaterial:

```
wsh(sortedmulti(2,@0,@1,@2))     order 0 1 2 -> bc1q2sz6vvu6k7y9gtc6kfgfe0p6xkhmvmdlu97eecjkykpdktvps08scdjgr5
                                 order 2 1 0 -> bc1q2sz6vvu…  (identical)
                                 order 1 2 0 -> bc1q2sz6vvu…  (identical)
control wsh(multi(2,…))          order 0 1 2 -> bc1q8z8kvwnpeqy79hfkggrtfm26hkgq2tu708a86tcwtm5gy5wrc85s99fe24
```

`sortedmulti` also cannot nest — measured: `wsh(or_i(sortedmulti(…),sortedmulti(…)))`
refuses with *"sortedmulti() is valid only as the sole child of sh() or wsh()"* — so a
`sortedmulti` descriptor has exactly ONE group spanning every slot. For the case r1 I2
raised (the shared-path privacy-preserving 2-of-3) the fold is correct and closes it.

**The half that opens a wrong-wallet path.** Rule 4 says refuse **"only when the path
matches slots in more than one non-interchangeable group"**, and rule 3's fallback is
"seat in SUPPLIED order". Two constructions, both encoded by `md` today:

**(a) Multiple `multi` groups.** Four privacy-preserving cards, all at
`48'/0'/0'/2'`, template
`wsh(or_i(multi(2,@0,@1),multi(2,@2,@3)))` (all slots carry that origin inline):

```
supplied order 0 1 2 3               bc1qural2jexg8yrjlrc56m84sn423vxe6cv920ehx9duckku9z7j26qswngek
supplied order 2 3 0 1 (groups swapped) bc1qsyld9jctwuncam7n4mm85xsmwx6kwg40ed2uygknxfzsplthudhspzla5z
supplied order 1 0 2 3 (within group A) bc1qyatz86rn384ce42dx47zlketzejqgsqkpqpw0ullx7x0x4puczys6x354j
```

Three orders, three different wallets, 4! = 24 candidates. The engine picks the one the
operator happened to type. Rule 3's mitigation is one stderr line and *"verify address 0
before trusting"* — but **rule 6 says that for a card set with no keyed card "nothing
external exists"**. The confirm the rule leans on is unavailable in precisely the input
form P2 exists to serve, and an operator restoring from plates is by definition without
the wallet software the instruction points at.

**(b) A slot in NO group at all — worse, because not even the warning fires.** Taproot,
five privacy-preserving cards at one shared path:

```
tr(@0/48'/0'/0'/2'/<0;1>/*,{sortedmulti_a(2,@1…,@2…),sortedmulti_a(2,@3…,@4…)})
   ^^ the internal key: the unilateral keyspend path, and a member of no group

supplied order 0 1 2 3 4              bc1pfv38mkt0q0twjhgkvvzpu5u7yfaarguqsjntgs5hsq2f46kar66skzrayn
supplied order 0 3 4 1 2 (groups swap) bc1pfv38mkt0q0…   (identical — taptree branches commute)
supplied order 0 2 1 3 4 (within a grp) bc1pfv38mkt0q0…   (identical — sortedmulti_a sorts)
supplied order 1 0 2 3 4 (INTERNAL KEY) bc1pg6w98ga5wzm0pfejhumy7e68ymehhm77tmvf65afpk4dzaagglwq4lpvcq
```

Rules 3 and 4 speak only of **groups**. `@0` is in none, so rule 4's "more than one
non-interchangeable group" cannot fire, rule 3's `multi()` warning is scoped to a `multi`
group and does not fire either, and rule 5 is satisfied by any bijection. The engine
seats **silently**, and the card that lands on `@0` holds the keyspend path — the single
most powerful position in the descriptor.

**This is a regression the fold introduced.** r1's rule 3 refused here ("never a guess");
the folded rule guesses, and rule 5's totality now always succeeds because any permutation
is a total assignment. The spec's own principle — *"Ambiguity refuses … never a guess"* —
was deleted for a class wider than the one I2 complained about.

**What r2 is not saying.** Reverting to a blanket refusal re-opens I2. The gap between
"refuse everything" and "guess silently" has room in it: refuse where the assignment is
observable in the addresses and seat freely where it is not — which is decidable, not a
judgement call, since the engine can derive the candidate assignments and compare. For a
group of n interchangeable slots there are n! candidates and for realistic n they can be
enumerated and shown. Whatever the ruling, rules 3 and 4 must (i) classify a slot that
belongs to no multi-group, (ii) say what happens when one path matches slots in two
different groups, and (iii) not lean on an oracle rule 6 has already said is absent.

### C2 — Acceptance 1's equality relation admits two different wallets. Constructed, twice, from the fixture's own keys.

The relation is *"address 0 and 1 on both chains agree … and the decoded key sets agree
as values"* — four derivations plus a set comparison. It is blind on two axes, and the
spec's subject matter sits on both.

**(i) Origin-blind.** Same keys, same script, two different declared origins:

```
wsh(multi(2,@0,@1)) keys K0,K1 fp 73c5da0a
  origin 48'/0'/0'/2'  -> …#ukf6lfzq
  origin 48'/0'/3'/2'  -> …#wxhqkvj6
  chain 0 index 0: IDENTICAL   bc1qpa7l8h70m9vjty6580zfhvkmu8xgu5g5gt7c86agydecu0yppd2s29csg0
  chain 0 index 1: IDENTICAL   bc1qxhhs2ptvx7v5plzuta7m6krsgauut4ll7plqvfhr3geep0wqn8kqrs2hs0
  chain 1 index 0: IDENTICAL   bc1qlkypls8k6ujrnqcfehc6e7q46extvtj3mkupwhh9dukzuzsuqtes9a73h9
  chain 1 index 1: IDENTICAL   bc1qy6055jwye2pvrxp4fv4wfksgg58qthr3rrm85a84u3eduve86nfseplxqe
```

All four checkpoints identical; the decoded key sets are identical; only the checksum
differs, and the acceptance deliberately does not compare checksums. A descriptor with a
consistent-but-wrong origin is **watch-only correct and unsignable** — the signer cannot
locate the key. `mk encode --keys` will not catch it either: it checks depth against path
length, and both paths are four components. **Per-slot origins are this cycle's entire
subject, and the gate cannot see them.**

**(ii) Template-blind.** Searched the 11 fixture keys for a pair where the relation
cannot separate `multi` from `sortedmulti`; found one:

```
X = wsh(sortedmulti(2,K1,K3))          Y = wsh(multi(2,K3,K1))
    identical decoded key set {K1,K3}; DIFFERENT templates
  chain 0 index 0: same      chain 0 index 1: same
  chain 1 index 0: same      chain 1 index 1: same      <-- 4 of 4, acceptance PASSES
  chain 0 index 2: DIFFER    <-- first divergence
  chain 0 index 3..7: same
```

`Wsh(SortedMulti)` and `Wsh(Ms(Multi))` are different `Descriptor` variants in
rust-miniscript, so a decomposer that reconstructs the wrong one is an ordinary slip, not
a contrived one. Under this acceptance it ships green, and every address the operator had
already used at index ≥ 2 is unrecoverable.

**Class.** The repo's severity rule names this explicitly as blocking: *"defects in what
a tool claims to have done (a gate that cannot fail, a refusal that does not refuse, a
test that reports a false PASS)."* Acceptance 1 is the gate for a funds-shaped feature and
it passes on both a wrong-origin and a wrong-template result.

**Why it happened, and the cheap direction out.** r1 C2 correctly killed *descriptor
string* equality; the fold over-corrected into a relation that discards everything except
four address samples. The middle ground already exists in the tree and costs nothing:
`md inspect` computes `wallet-descriptor-template-id` and `wallet-policy-id`, and prints
per-`@N` origins. Comparing the decoded template (or one of those ids) and the per-slot
origin declarations, alongside the addresses, separates both counterexamples. Sampling
more indices does not — (i) is invariant at every index by construction.

---

## IMPORTANT

### I1 — Rule 1 ignores the form-aware stub split, so a legitimate card set is refused with a message that is false.

r1 quoted `mk-cli/src/cmd/mod.rs:102-116`: the stub is the **WalletPolicyId** for a keyed
wallet-policy md1 and the **WalletDescriptorTemplateId** for a keyless template md1. The
folded rule 1 uses only the second: *"at least one of the card's stubs … matches the
policy card's template id."* Measured, same key, same wallet, same shape:

```
$ mk encode --xpub <K0> --origin-fingerprint 73c5da0a --origin-path "m/48'/0'/0'/2'" \
      --from-md1 <the 6 KEYLESS chunks>   ->  policy_id_stubs: 5b48af35
$ mk encode … --from-md1 <the 22 KEYED chunks>
  note: policy 232214e4 has 11 cosigner(s); 1 of them carded here, 10 not carded
                                            ->  policy_id_stubs: 232214e4
```

An operator who minted key cards while holding the keyed card carries `232214e4`. Rule 1
compares it to `5b48af35` and refuses with *"key card `<prefix>` was minted for a
different policy SHAPE (stub `232214e4`; this template is `5b48af35`)"* — which is
**false**: identical shape, identical wallet, identical key. Engraved plates rejected at
restore, with a message that misdirects the diagnosis.

Both ids are computable from the policy card in hand (`md inspect` prints both), so the
predicate should be membership against **either**, and the refusal text should not assert
"different shape" for a value it did not test against.

### I2 — P1's stated inline syntax does not parse, and its precedence rule's second clause describes a conflict that cannot occur.

P1 writes the authoritative inline form as `[fp/path]@i/...`. Measured:

```
$ md descriptor --template "wsh(multi(2,[73c5da0a/48'/0'/0'/2']@0/<0;1>/*,…))" --key …
md: template parse error: @0 carries a descriptor-style origin prefix
  `[73c5da0a/48'/0'/0'/2']`; md templates take the origin AFTER the placeholder —
  write `@0/48'/0'/0'/2'/…` … The fingerprint `73c5da0a` is NOT part of the template:
  pass it as `--fingerprint @0=73c5da0a`.
```

The working form is `@i/48'/0'/0'/2'/<0;1>/*` plus a separate `--fingerprint` flag — which
is what r1 I8 measured and what the spec's own matrix row now cites.

The consequence is not cosmetic. P1's precedence rule reads *"inline template origins are
authoritative; `--fingerprint @i=` must AGREE with an inline origin when both name slot i
(mismatch refuses — never silent override)"*. An inline origin **cannot carry a
fingerprint** — md says so verbatim — so the two sources are disjoint by construction:
the template supplies the path, the flag supplies the fingerprint. There is nothing to
agree about and the refusal can never fire. A NORMATIVE rule in the funds-shaped section
whose guard is unreachable is the same class as C2's gate.

The precedence that *does* need defining is the one the fold's third clause gestures at:
inline path vs `--path` vs (under P1) the path inside `--key '@i=[fp/path]xpub'`. Three
sources, and the flag form is the new one.

### I3 — Rule 3's group vocabulary omits `multi_a` and `sortedmulti_a`, which is where multiple groups actually coexist.

Rule 3 classifies `sortedmulti` (immaterial) and `multi` (order-sensitive). Measured, both
of the taproot forms encode today:

```
tr(@0/<0;1>/*,multi_a(2,@1/<0;1>/*,@2/<0;1>/*))        -> md1yzpqqxq3yqskqenp7q6zxr6kq
tr(@0/<0;1>/*,sortedmulti_a(2,@1/<0;1>/*,@2/<0;1>/*))  -> md1yzpqqxq3ysskujjnyl7xkfa7r
```

`multi_a` is order-sensitive like `multi`; `sortedmulti_a` is not. And unlike
`sortedmulti` — which cannot nest, so a `wsh(sortedmulti)` descriptor has exactly one
group — the `_a` forms live in taptree leaves and several can coexist in one descriptor,
which is the multi-group situation rule 4 was written for. Rule 3 as written classifies
neither, so an implementer either guesses or falls through to whichever branch the code
happens to reach. (This is separable from C1: it remains a gap whatever C1's ruling.)

### I4 — The `cli-compiler` gating is stated three ways in two documents; the plan will inherit the wrong one.

```
SPEC   P3, line 176   "UNGATED (r1 I4): parsing needs no feature; only `compile` needs
                       `miniscript/compiler`; verified with a compiler-free probe"
SPEC   Gates, line 238 "`decompose` sits behind `cli-compiler`; P1/P2 ship ungated."
BRAINSTORM decision 3  "`md decompose`, `cli-compiler`-gated."
```

The *Gates and process* section is the one a plan reads for its feature matrix and CI
invocation. The fold updated the argument and left the ruling.

### I5 — Ruled-away claims survive verbatim in the sections a reader hits first.

The fold corrected each finding at the site the finding cited and left the duplicates —
the propagation failure mode, four sites:

* **Motivation bullet 2 (line 29)** still reads *"the only path flag is the SHARED
  `--path` … (`md encode` has per-slot Divergent mode; its **read-side siblings do not** —
  an asymmetry, not a design.)"* — the exact premise I8 refuted and which P1 now opens by
  calling wrong. Both halves are false: neither side has a per-slot *flag*, and both
  support Divergent through inline template origins.
* **Motivation ¶5 (line 41)** still frames the danger as *"it did NOT verify the policy-id
  stubs. A composition that seats an unbound key card can reconstruct a DIFFERENT
  wallet"* — the wallet-binding implicature C1 deleted from rule 1. An implementer reading
  the motivation believes the stub check closes what rule 1 now says it cannot.
* **Motivation line 34** still says *"the full 1,649-char concrete descriptor"* against
  acceptance 4's *"1,648 characters (1,649 bytes with trailing newline)"*.
* **Brainstorm decision 4** still says *"hence stub binding first, exact-origin seating,
  ambiguity refuses"* — three claims, all three superseded (C1: shape not binding; I7:
  decoded-value not exact-string; I2/C1: ambiguity no longer refuses).

The brainstorm's **matrix** was synced and verified identical; its **decisions** were not
touched. The travel directive is about the matrix, but a brainstorm whose recorded reasons
contradict the spec they feed is not a usable record.

### I6 — Rule 6's automated oracles have no owner, and one of them is undefined against the spec's own fixture.

Rule 6 gained the two checks r1 M2 asked for, and neither is attached to a piece:

* *"For input D, the composed result's address 0 MUST equal the input descriptor's own
  derivation (automated; a mismatch is an internal error, refuse loudly)."* For input D the
  pipeline is **decompose**, and P3's described outputs are a template, key lines and
  `--fingerprint` flags — no composed result. Nothing in P1/P2/P3 says decompose
  re-composes and compares, so as written no code path performs this check.
* *"For the split set, when a keyed card of the same wallet is also supplied the two
  compositions MUST agree."* Agree on what? r1 C2 measured that the fixture's two cards
  produce 1,901 chars `#s5a2k003` and 1,648 chars `#xn3k4jmt` — a descriptor reading makes
  this check fail on the spec's own walk; an address reading passes. Rule 6 is NORMATIVE
  and does not say.

An unowned normative gate is the class r1 flagged in the repo's own closure rule: a gate
that has never been run is a hypothesis. Name the owning piece and the comparison
relation — and note the relation is C2's, so the two findings fold together.

### I7 — The mk primary entry misdescribes the mechanism it proposes to change, and its lockstep omits a third repo whose live test forbids the naive fix.

`mnemonic-key/design/FOLLOWUPS.md` → `stub-keyed-wallet-binding-at-mint` says:

> "(Distinct from `stub-formula-divergence`, RESOLVED v0.8.0: the FORMULA is
> WalletPolicyId-rooted; the keyless mint path simply has no keys to feed it, so the
> binding collapses to policy SHAPE.)"

Both halves are false, measured:

* **The formula is not WalletPolicyId-rooted for a keyless md1.** v0.8.0 made it
  unconditionally so; a *later* change (toolkit #28) made it FORM-AWARE and routes
  keyless cards to `compute_wallet_descriptor_template_id`
  (`mk-cli/src/cmd/mod.rs`, `decode_md1_card`). Live:

  ```
  $ mk encode --xpub <K0> --origin-fingerprint 73c5da0a --origin-path "m/48'/0'/0'/2'" \
        --from-md1 md1yqpqqxzq2qwfv8urt848e          # the pkh_basic KEYLESS template
  policy_id_stubs:     559e64b2       <-- WalletDescriptorTemplateId
       v0.8.0 froze:   3d190af3       <-- unconditional WalletPolicyId
  ```
  `mk-cli/tests/template_id_stub.rs` names `3d190af3` "the pre-#28 (**BUGGY for a
  template**) value" and asserts the emitted stub does **not** equal it.

* **It is not for want of keys.** `compute_wallet_policy_id` runs fine on a keyless
  descriptor and discriminates without any: r1 measured two same-template keyless cards
  giving `073e6088` vs `04829c78` on fingerprints and origins alone, and the pathological
  keyless card's own policy id is `024a9921`. The keyless arm does not lack an input; the
  dispatch deliberately chooses the other identity.

**Two consequences the entry hides.**

1. **A cheaper intermediate tier exists and is not offered.** Simply routing the keyless
   arm to `compute_wallet_policy_id` needs no new mint-time inputs and would already
   refuse the *fingerprint-bearing* foreign card — the ordinary drawer mistake. It would
   NOT close CE-1, whose two wallets have byte-identical keyless cards, so only the full
   keyed stub kills CE-1. The entry collapses two tiers with different costs into one.
2. **It is a THREE-repo lockstep, not two.** `mnemonic-toolkit` mints the same binding
   stubs (`crates/mnemonic-toolkit/src/cmd/bundle.rs:1239 bundle_binding_stub`), and
   toolkit #28 agreement is precisely why the keyless arm is the template id today. The
   test comment states the hazard directly: *"swapping the keyless arm to WalletPolicyId
   … survived this test while killing its siblings."* The naive change is an
   already-guarded regression, and the toolkit is an unnamed party to the flip.

Rule 1 in the spec remains correct as written — it describes today's behaviour, which I
re-measured. This is a defect in the **filed plan**: executed as described it would be
attempted in the wrong repo, against a test written to stop it. (The descriptor-mnemonic
companion entry does not repeat the claim; it says only "keyless-mint stubs are
shape-only", which is accurate.)

---

## MINOR

* **M1** — Line 222 reads `## Non-goals## Non-goals`. A duplicated heading introduced by
  the fold.
* **M2** — Rule 1 says the stub *"matches the policy card's template id"*. The stub is
  4 bytes and the id is 16; say "the top 4 bytes of". (Both are printed by `md inspect`.)
* **M3** — Rule 3's dedupe (*"Identical (origin, xpub) pairs deduplicate to one key"*)
  against rule 5's totality: a policy that genuinely declares two distinct slots holding
  the same (origin, xpub) — constructible by an encoder other than md's, which collapses
  them — dedupes to one key and then fails rule 5 as an unfilled slot. Say whether the
  collapse is assumed on the policy side too.
* **M4** — `h`-spelling in an inline template origin is rejected, with a message that
  blames the wrong thing: `wsh(multi(2,@0/48h/0h/0h/2h/<0;1>/*,…))` → *"derivation steps
  after the multipath group are not representable in md1; the multipath `<…>` must be the
  final derivation step before the wildcard"*. Only `'` parses. P1 makes inline origins
  authoritative, so this parse path is now load-bearing and its refusal misdirects — the
  F-420 class the spec cites approvingly elsewhere. (Distinct from I3's canonical
  *emission* ruling, which is about output and is correctly folded.)
* **M5** — Acceptance 1(d) requires a *"depth-consistent coordinator-grade concrete
  descriptor"* and names none. It is satisfiable — verified end to end with the first three
  lines of the fixture's `keys.txt` (depth-4 keys, matching 4-component origins):
  `mk encode --keys` accepts them, `md encode --key`+`--fingerprint` mints a card, and
  addresses derive on both chains. Pin that fixture by name so the leg cannot be reported
  as blocked for want of an input.

* **M6** — The lockstep set is under-enumerated, and "PERMANENT" is attached to the
  wrong noun. Rule 1 says CE-1 "ships as a PERMANENT vector row **asserting exactly this
  accepted behaviour**" and then, three lines later, that the row "flips to a refusal".
  The intended reading — permanent as a *row*, with a *changing assertion* — is coherent
  but is nowhere stated; as written the emphatic PERMANENT binds to the assertion that
  flips. And the flip list names two sites ("this rule tightens and CE-1's row flips")
  where at least five must move together: rule 1's predicate, rule 1's refusal TEXT
  (which says "different policy SHAPE" and would become a wallet-binding message), CE-1's
  vector row, **acceptance 2**'s parenthetical (*"a same-stub foreign card seats and the
  derived address differs; the row asserts BOTH halves"*), and **acceptance 3**'s second
  half (*"the same-shape case proven to seat"*, which inverts to "refused"). Both
  acceptances would contradict the new rule 1 the day the mk release lands. Given this
  repo's demonstrated failure mode is incomplete propagation — I5 above is the realised
  version — enumerating the full set now is the cheap moment.

---

## Gate

**RED — 2 Critical, 7 Important.** The loop does not close.

Both Criticals are **fold-introduced**, and both come from the same over-correction:
r1 said "rule 3 refuses too much" and "descriptor-string equality is false", and the fold
answered by removing the refusal and by weakening the equality until neither can fail.
The corrections were right; their range was too wide.

Shortest path to GREEN:

1. **C1** — classify a slot in no group; define the two-groups-one-path case; and do not
   cite an oracle rule 6 has already declared absent. Whatever seats silently must be
   provably address-invariant, which is decidable.
2. **C2** — add the decoded template (or the template/policy id md already computes) and
   the per-slot origin declarations to the equality relation. Both counterexamples above
   should become vector rows: they are constructed, from fixture keys, and each fails only
   under the strengthened relation.
3. **I1, I2, I3, I6** are localised, and I6 merges into C2's relation once it is fixed.
4. **I4, I5** are propagation sweeps — five sites named, one pass.
5. **I7** is a correction to the mk FOLLOWUPS entry, not to this spec; it can land
   independently and should, before anyone schedules the upgrade.
6. **M1–M6** ride the same fold.

Re-dispatch after the fold. Nothing in the r1 disposition table marked **yes** needs
revisiting; scope r3 to the ten items above and to whatever the fold newly writes.
