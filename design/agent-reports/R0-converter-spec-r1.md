# R0 round 1 — SPEC_wallet_form_converter.md

**Artifact:** `design/SPEC_wallet_form_converter.md` @ descriptor-mnemonic `5912ad78`
(dispatched against `af15f7dd`; the delta is the "matrix travels" clause plus
`design/BRAINSTORM_wallet_form_converter.md`, both reviewed here).
**Reviewer:** independent R0 agent, 2026-08-30. **Verdict: RED.**
**Counts: 3 Critical / 9 Important / 8 Minor / 2 Nit.**

Companion trees: mnemonic-key `93cebfb`, mnemonic-engrave (pathological fixture at
`design/journeys/out/pathological/`). Binaries: `md 0.13.0`, `mk 0.13.0`.
Nothing in any repo was modified. All scratch work in
`/tmp/claude-1000/-scratch-code-shibboleth-mnemonic-engrave/1448953f-.../scratchpad`.

---

## 0. The measured claims — re-run, not re-derived

Every claim the spec presents as measured was reproduced. Results:

| spec claim | verdict | evidence |
| --- | --- | --- |
| Refusal 1: keyless phrases → *"this card is a keyless TEMPLATE…"* | **holds** | `md descriptor $(cat md1.txt)` → `md: descriptor requires wallet-policy mode (Pubkeys TLV): this card is a keyless TEMPLATE, which has no concrete form. Supply --key @i=XPUB, or use 'md decode' to see the template.` exit 2 |
| Refusal 1b: adding `--key` does not change the answer | **holds** | same invocation + `--key '@0=xpub6DkFA…'` → byte-identical message, exit 2 |
| Refusal 2: *"non-canonical wrapper requires explicit origin for @0"* | **holds** | `md descriptor --template <decoded tpl> --key @0..@10 --fingerprint @0..@10` → `md: codec error: non-canonical wrapper requires explicit origin for @0, but none provided`, exit 1 |
| Refusal 3: `--key '@i=[fp/path]xpub'` → base58check | **holds** | → `md: --key @0: base58check decode: decode`, exit 1 (unchanged by `--path`) |
| Keyed card round-trips to `…#xn3k4jmt` | **holds** | 22 `md1fatzr2*` strings extracted from `journey_pathological.html`; `md descriptor` → checksum `#xn3k4jmt`, exit 0 |
| "1,649-char" descriptor | **off by one** | 1,648 characters; 1,649 **bytes** with the trailing newline (Nit N1) |
| Address 0 `bc1qkuknuy6…` | **holds** | `md address <keyed>` → `bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64` |
| `me` stops at the seven plain forms | **holds** | `crates/me-cli/src/descriptor/admit.rs:93` — "the seven forms, plus the three `multi` twins" |
| 36 strings = 1 keyless 6-chunk card + 11 mk1 cards | **holds** | 6 md1 + 30 mk1 lines; grouped by chunk-set prefix = 11 cards (2+3+3+3+2+3+3+3+2+3+3) |

**Also measured, and NOT in the spec — these drive the findings below:**

```
$ md inspect <the 6 keyless chunks>          $ md inspect <the 22 keyed chunks>
wallet-policy-mode: false                    wallet-policy-mode: true
wallet-descriptor-template-id: 5b48af35…     wallet-descriptor-template-id: 5b48af35…   <-- IDENTICAL
wallet-policy-id:              024a9921…     wallet-policy-id:              232214e4…   <-- DIFFERENT
origins: @0 [73c5da0a/48'/0'/0'/2']          origins: @0..@10 ALL m/48'/0'/0'/2'
         @1 [73c5da0a/48'/0'/1'/2']                   (no fingerprints at all)
         … four accounts, three fingerprints
```

```
$ mk decode <card 1 of the 11>
policy_id_stubs:     5b48af35        <-- top 4 bytes of the WalletDescriptorTemplateId
```

---

## CRITICAL

### C1 — Rule 1 (stub binding) is inert. All six rules can pass and still emit a wallet the operator does not own.

The spec makes rule 1 the funds-safety anchor, twice:

> "It also exposed the danger: it did NOT verify the policy-id stubs. A composition
> that seats an unbound key card can reconstruct a DIFFERENT wallet"

> "1. **Stub binding first.** … A key that would seat perfectly but fails the stub
> check is the attack/mistake this rule exists for."

**The stub cannot do this job, and no strengthening of rule 1 can make it.**

`mk-codec/src/key_card.rs:25-34` and `mk-cli/src/cmd/mod.rs:102-126` state that the
stub is **FORM-AWARE**: for a **keyless template md1** — which is exactly the S-form
input P2 addresses — it is the top 4 bytes of `compute_wallet_descriptor_template_id`,
not the WalletPolicyId. And `md-codec/src/identity.rs:47-52` says what that identity
covers, verbatim:

> "Hashes ONLY the BIP 388 template content: use-site-path-decl bits, tree bits, and
> the `UseSitePathOverrides` TLV entry bits when present. **Excludes the header,
> origin-path-decl, `Fingerprints` TLV, HRP, and BCH checksum, so it is invariant to
> origin-path changes (e.g. account index) and to fingerprint additions.**"

Measured, two entirely different wallets:

```
$ md encode 'wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))' \
      --fingerprint @0=73c5da0a --fingerprint @1=b8688df1 --path "m/48'/0'/0'/2'"
  wallet-descriptor-template-id: a235ee7574702e45f80089c07e73ed22
  wallet-policy-id:              073e6088213bd1062b62e844ed16e950

$ md encode 'wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))' \
      --fingerprint @0=deadbeef --fingerprint @1=cafebabe --path "m/48'/0'/7'/2'"
  wallet-descriptor-template-id: a235ee7574702e45f80089c07e73ed22   <-- SAME STUB a235ee75
  wallet-policy-id:              04829c7858da7e200459b8a615ea40bc
```

The same effect in the fixture: the keyless card (three fingerprints, four accounts)
and the keyed card (no fingerprints, one shared account) — two *different* origin
declarations for the same key set — share WDT-id `5b48af35…`.

**The structural statement, which is the one that matters:** a keyless md1 policy card
carries **no key material**. Two operators whose wallets share a template and an origin
declaration hold *the same card bytes*. There is nothing in it for a stub to bind to.
Rule 1 therefore adds **zero discrimination beyond what rule 2 already performs** —
in the Divergent-with-fingerprints case the fingerprint does all the binding, and in
the privacy-preserving case nothing binds at all.

**Counterexample CE-1 — every rule passes, wrong wallet out.** Ingredients all measured:

* Wallet X, keyless privacy-preserving Divergent card, minted and inspected:
  ```
  $ md encode "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))"
  md15pfdsssjjtvyyw2sqrqsuy99velf0v0tlapq
  wallet-descriptor-template-id: aad0e0e0718cbe91da67cc2bd72c68c9
  origins: @0: m/48'/0'/0'/2'   @1: m/48'/0'/1'/2'    (no fingerprints)
  ```
* Wallet Y: a different seed, same template, same two standard accounts. Its keyless
  card is **byte-identical** to X's; its stub is necessarily `aad0e0e0`.
* Y's two privacy-preserving mk1 cards declare `48'/0'/0'/2'` and `48'/0'/1'/2'`.
* **Rule 1** passes (stubs equal). **Rule 2** seats each card at its exact declared
  path. **Rule 3** finds no ambiguity (the two paths are distinct). **Rule 4** is
  satisfied (exactly one unfilled slot per path). **Rule 5** is total. **Rule 6**
  prints address 0 — *of wallet Y*.
* Output: a checksummed, valid-looking descriptor for a wallet the operator does not
  own, with no refusal anywhere.

This is not an exotic setup: the pathological fixture is precisely the
one-seed-multiple-standard-accounts shape, ×3 seeds. Two such vaults whose plates got
mixed in a drawer is the ordinary version of CE-1.

**Consequences for the text as drafted:**
1. The prescribed refusal string is **factually wrong**: *"key card `<prefix>` is bound
   to a different wallet policy"* asserts a binding the check did not test. A matching
   stub means "same template shape", nothing more. This is the *"defects in what a tool
   claims to have done"* class, which still gates.
2. The spec states no residual risk anywhere. An implementer builds rule 1, sees it
   pass, and believes the composition is bound.
3. Acceptance 3 will pass while CE-1 remains open (see M6).

**What R0 should decide (this is the mk-side action the spec's own gate clause
anticipates):** either (a) require, for compose, that the mk1 cards carry the keyless
card's **WalletPolicyId** stub — which *is* origin- and fingerprint-sensitive
(`identity.rs:110-118`) and would have refused CE-1 — which is an `mk` change and a
Rust-primary cross-repo cycle; or (b) keep the WDT-id stub, **delete the binding claim**,
rename rule 1 to what it is ("template-shape screen"), and make the composed output
state plainly that the ONLY binding between these keys and this policy is the origin
declaration the operator engraved. Option (b) is cheap and honest; option (a) is the
one that closes CE-1. Either way the current text cannot ship.

---

### C2 — Acceptance 1, first half, is false by measurement: the split set and the keyed card do NOT yield the same descriptor "modulo canonicalisation".

Acceptance 1 requires:

> "the 36-string split set composes to the same wallet the 22-chunk keyed card yields
> today (**same descriptor modulo canonicalisation**, same address 0 `bc1qkuknuy6…`,
> checksum recomputed)"

Both sides were computed. The compose side was simulated exactly as the seating engine
would produce it (per-slot origins from the keyless card, the 11 keys from `keys.txt`):

```
keyed card   ->  1,648 chars,  checksum #xn3k4jmt,  ZERO '[' origin brackets
split set    ->  1,901 chars,  checksum #s5a2k003,  11 '[fp/path]' origin brackets
address 0    ->  bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64  (BOTH)
```

The **wallet** is the same — address 0 matches, which is the property that matters —
but the two descriptors differ by 253 characters and by the presence of every key
origin. That is not a canonicalisation difference; it is a structural one, and it is
forced by the fixture: the keyed card carries origins `m/48'/0'/0'/2'` for all eleven
slots **with no Fingerprints TLV**, and BIP-380 origin notation cannot be written
without a fingerprint, so `md descriptor` correctly emits bare keys. The keyless card
carries three fingerprints and four accounts, so compose correctly emits brackets.

As written the acceptance cannot pass. Restate it as the property that is actually
true and actually checkable: **same address 0, and identical key set in identical slot
order** — and, if descriptor equality is wanted, say against *which* origin declaration
and note that the two cards disagree about it.

Second-order: the spec's Motivation calls these two forms two encodings of one wallet.
They are two encodings of one *key set* under **two different origin declarations**.
The converter's own matrix should not imply K↔S is loss-free; it is not.

---

### C3 — P3's deliverable and Acceptance 1's second half are refused by `mk`: md descriptors carry depth-0 re-serialised xpubs.

P3 promises:

> "the origin-notated key lines (`[fp/path]xpub`, one per line — **a valid `mk encode
> --keys` file**)"

and Acceptance 1 requires that decompose of the pathological descriptor "re-emits a
template + key set that **re-encode to cards**".

`md descriptor` does not emit the operator's xpubs. md1 stores a 65-byte key identity
(chain code ‖ compressed pubkey — `mk-cli/src/cmd/mod.rs:130-140`), so every emitted
key is re-serialised at **depth 0, child 0**:

```
supplied:  xpub6DkFAXWQ2dHxq…  xpub6DzhyrnFFYQ1H…  xpub6EGx8sPr9FxPP…   (depth 4)
emitted:   xpub661MyMwAqRbcG…  xpub661MyMwAqRbcG…  xpub661MyMwAqRbcF…   (depth 0, all 11)
```

`mk` enforces `depth == component_count(origin_path)` (`key_card.rs:48-56`,
`Error::XpubOriginPathMismatch`). Feeding it exactly the file P3 promises to produce:

```
$ grep -o '\[[0-9a-f]\{8\}/[^]]*\]xpub[0-9A-Za-z]\{50,\}' composed.txt > decomposed-keys.txt   # 11 lines
$ mk encode --keys decomposed-keys.txt --policy-id-stub 5b48af35
error: --keys record 1 ([73c5da0a/48'/0'/0'/2']): xpub origin-path mismatch:
       xpub depth 0 / child 0 vs origin_path depth 4 / last Some(Hardened { index: 2 })
```

And the other reading of "that descriptor" — the 1,648-char keyed one — fails
differently and worse: it has **no origins at all**, so decompose has nothing to put
in `[fp/path]` and cannot emit the key file in any form (see I5).

The acceptance therefore cannot run under either reading. The remedy is available and
belongs in the spec: decompose must **reconstruct depth and child-number from the origin
path before emitting key lines** — which is exactly `mk`'s own documented rule
(`depth := component_count(origin_path)`, `child := last_component`, `Normal{0}` when
empty). A naive implementation that copies the descriptor's xpub string verbatim
produces a file `mk` refuses; the spec currently describes the naive implementation.

---

## IMPORTANT

### I1 — Rule 1 says "stub", singular. The field is `policy_id_stubs: Vec<[u8; 4]>`, and multi-stub cards are a supported, vectored shape.

`key_card.rs:34` — `pub policy_id_stubs: Vec<[u8; 4]>`. `mk encode --policy-id-stub`
and `--from-md1` are both documented **"Repeatable."** The corpus carries
`V6_3_stubs_mainnet_with_fp`. Minted and decoded here:

```
$ mk encode --xpub xpub6DkFA… --origin-fingerprint 73c5da0a --origin-path "m/48'/0'/0'/2'" \
     --policy-id-stub 5b48af35 --policy-id-stub aad0e0e0 --policy-id-stub a235ee75
$ mk decode <the two strings>
policy_id_stubs:     5b48af35, aad0e0e0, a235ee75
```

Rule 1's *"every card's `policy_id_stub` MUST match the policy card's stub"* has no
correct literal implementation against this type. Both obvious readings
(`stubs == vec![p]`, `stubs[0] == p`) **refuse a legitimate card** — a key that serves
this vault *and* a single-sig policy, which is the whole reason the field is a vector.
The predicate must be **set membership**: `card.policy_id_stubs.contains(&policy_stub)`.
State it that way, and say what the refusal prints when a card carries several stubs and
none matches.

### I2 — Rule 3 refuses the ordinary shared-path privacy-preserving multisig, which makes rule 4 unreachable for the common case.

Rule 3 is unconditional and stated before rule 4 ("Rules, in order"):

> "identical origin with different xpubs is the unseatable-backup defect and the
> refusal says so"

For a **fingerprint-bearing** card that verdict is right and worth keeping: fingerprint
+ path deterministically fixes the xpub, so identical origin with different xpubs means
a fingerprint collision or a corrupted card. Refuse.

For a **privacy-preserving** card the origin *is* the path alone — and the ordinary
2-of-3 built from three separate devices has all three at `48'/0'/0'/2'`. Three
different xpubs, identical declared origin. Rule 3 calls that "the unseatable-backup
defect"; it is the normal shape of the mode. Rule 4 then cannot rescue it either — it
requires "**exactly one** unfilled slot declares that path fingerprint-free", and three
slots declare it. **The privacy-preserving mode is unusable for shared-path multisig
under these rules, and the spec nowhere says so.**

Related, and the brief asks it directly: for a **`sortedmulti`** policy the slot
permutation is semantically irrelevant — any assignment yields the same addresses — so
the refusal is *conservative*, not *necessary*. For `multi` it is necessary (slot order
is consensus-relevant). The spec treats all policies alike and gives no rationale for
either. Rule 3 needs: (a) a fingerprint-present / fingerprint-absent split, (b) the
`multi` vs `sortedmulti` rationale written down, and (c) an explicit statement of what
an operator holding a legitimate shared-path privacy-preserving backup is supposed to
do — because today the answer is "engrave new plates".

### I3 — The Canonicalisation section's claims are false against both repos as they stand, and adopting them silently changes every shipped `md descriptor` output and its checksum.

> "Emitted descriptors use `h` hardened spelling … These follow `me`'s S1 cascade
> canonical form so the two repos' canonical descriptors are byte-comparable."

Measured:

* **md emits `'`.** The composed 1,901-char descriptor: 44 apostrophes, **0** `h`-forms;
  origins render `[73c5da0a/48'/0'/0'/2']`.
* **`me` emits `h`.** `cascade.rs:289-300` `path_encode` pushes `'h'`;
  `cascade.rs:280-282` — "`h` for hardened, **never** `'`".
* **rust-miniscript's `Display` emits `'`** regardless of input spelling — feeding it
  `48h/0h/0h/2h` re-encodes as `48'/0'/0'/2'` (probe, pinned rev `ff4732e`). So P3, if
  it emits through `Descriptor`'s Display, gets `'` and violates the rule.
* **The spelling changes the checksum.** Same body, two spellings:
  `…[73c5da0a/48h/0h/0h/2h]…` → `#vxep95en`; `…[73c5da0a/48'/0'/0'/2']…` → `#wst5xzus`.

So "byte-comparable" is false today, and making it true is a **breaking output change**
to a shipped command — every descriptor `md` prints, and every checksum with a hardened
origin, moves. The section presents this as a description of existing behaviour and
does not mention the goldens, the insta snapshots, or a migration note. Either commit
to the change and say it is breaking, or drop the byte-comparability claim and state
that md keeps `'`. The one thing the spec must not do is assert it as already true.

(Key order and checksum recomputation, the section's other two claims, **do** hold:
`me`'s `encode_no_checksum` iterates `self.keys` in order and always recomputes.)

### I4 — `decompose` does not need `cli-compiler`. The gate is mis-chosen and costs default builds the whole feature.

> "parses a concrete descriptor (rust-miniscript; **feature-gated `cli-compiler` like
> `compile`**)"

`crates/md-cli/Cargo.toml`:

```
[features]
cli-compiler = ["miniscript/compiler"]     # enables the POLICY COMPILER only
[dependencies]
miniscript = { workspace = true }          # NOT optional
```

The feature turns on miniscript's *policy compiler*, which `md encode --from-policy`
needs and a descriptor *parser* does not. `miniscript` is an unconditional dependency.
Verified end-to-end: the probe crate depends on the same pinned rev with
`default-features = false, features = ["std"]` — no `compiler` — and
`Descriptor::<DescriptorPublicKey>::from_str` parses every form the spec needs.

So decompose can and should ship **ungated**, alongside P1/P2. As written the spec
withholds the D entrance from the default build for no technical reason, and the
"P1/P2 ship ungated" boundary in *Gates and process* is drawn in the wrong place.

### I5 — P3 has no answer for origin-less keys, and the descriptor the acceptance names is exactly that case.

rust-miniscript parses bare xpubs happily (`origin=None`, probe row "BARE xpubs"). The
1,648-char pathological descriptor — the one Acceptance 1 hands to `decompose` — has
**zero** `[` characters. There is no fingerprint and no origin path to recover.

The spec says decompose "extracts each key **with its origin**" and emits
"`[fp/path]xpub`" lines and "the per-slot `--fingerprint` flags". All three are
impossible for this input, and the spec is silent on it. Decide and write it down:
refuse (naming what is missing and that `me`/the coordinator's export can supply it),
or emit key lines without origins and state plainly that the resulting card set is
**not** re-composable (`mk encode` requires `--origin-path`; a depth-0 xpub with an
empty path is the only self-consistent pair).

Same class, adjacent: `[73c5da0a]xpub…` — a fingerprint with an **empty** origin path —
parses to `Some((73c5da0a, ))`. Rule 2's "declares `[fingerprint/path]`" does not cover
a zero-length path.

### I6 — The matrix's D row omits real input forms, and the spec states no boundary against the container shapes operators actually paste.

The brief asks for the boundary; the spec gives none. Two concrete gaps:

1. **Split receive/change descriptor pairs.** Core and Sparrow export two lines,
   `wsh(...)/0/*` and `wsh(...)/1/*`, rather than the multipath `<0;1>` form. Both parse
   (probe row "single-chain /0/\*"), and decomposing only the receive line silently drops
   the change branch — a wallet with a hole, which the spec's own principle forbids.
   This is the most common real-world D shape and the spec does not mention it.
2. **`listdescriptors` JSON.** An operator pastes `{"desc":"wsh(...)#cksum",...}`. The
   error rust-miniscript produces is
   `invalid checksum (length 24, expected 8)` — which names nothing the operator can
   act on. This repo just closed the identical class in F-420 (`6c4a56fd`, "the
   no-placeholder refusal names descriptors and refers to me"). Whatever the ruling —
   my recommendation is **out of scope for `md decompose`, JSON unwrapping is `me`'s
   branch** — the spec must state it and the refusal must say so.

Worth recording as settled: `Descriptor::from_str` **does** verify a present checksum
(`invalid checksum aaaaaaaa; expected vxep95en`) and accepts its absence. So a corrupted
descriptor is caught for free; the spec should say it relies on that rather than leaving
it to an implementer to notice.

### I7 — Rule 2 mandates string comparison over values both codecs already normalise, and its normalisation list is incomplete.

> "String-exact after `h`/`'` normalisation; no fuzzy matching, no path suffixing."

Four real spellings measured, not two:

| axis | observed | where |
| --- | --- | --- |
| `'` vs `h` | both | the spec's own list |
| `m/` prefix | **present** in `md inspect` (`@0: m/48'/0'/0'/2'`) and accepted by `--path`; **absent** in `md decode`'s origins note (`[73c5da0a/48'/0'/0'/2']`) and in `mk decode` (`48'/0'/0'/2'`) | three of md/mk's own outputs disagree |
| fingerprint case | `mk encode --origin-fingerprint 73C5DA0A` and `md encode --fingerprint @0=73C5DA0A` both accepted and normalised | verified: uppercase and lowercase mint byte-identical cards |
| path spelling at the flag | `mk encode --origin-path` accepts `m/48'/…`, `m/48h/…` and `48'/…` — all three mint the identical card | verified |

The wire format normalises all of it, so comparing the **decoded** values
(`Fingerprint` bytes and `DerivationPath`) is exact by construction and cannot get this
wrong. Prescribing *string* comparison, with a normalisation list that names one of the
four axes, is how a legitimate key gets refused because the operator wrote `m/`. The
failures are refusal-shaped rather than mis-seating-shaped — safe direction, wrong
outcome — but this is a normative rule in a funds-shaped engine and should say
"compare decoded `(Option<Fingerprint>, DerivationPath)` values", not "string-exact".

### I8 — P1's stated premise is measurably wrong: per-slot origins already work on the read side, and the spec does not define precedence when the two spellings disagree.

> "the only path flag is the SHARED `--path`, and this vault's slots declare four
> different accounts. (`md encode` has per-slot Divergent mode; its read-side siblings
> do not — an asymmetry, not a design.)"

Divergent mode is not driven by a flag on *either* side. `make_path_decl`
(`md-cli/src/parse/template.rs:757-772`) builds `PathDeclPaths::Divergent` when the
per-`@i` origin paths written **into the template text** differ. The read side uses the
same parser, and it works today:

```
$ md descriptor --template "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))" \
      --key @0=xpub6DkFA… --key @1=xpub6Dzhy… --fingerprint @0=73c5da0a --fingerprint @1=73c5da0a
wsh(multi(2,[73c5da0a/48'/0'/0'/2']xpub661MyMwAqRbcGQnC8zMG…/<0;1>/*,
            [73c5da0a/48'/0'/1'/2']xpub661MyMwAqRbcG5161axq…/<0;1>/*))#gypewfdj
```

The full 11-slot pathological compose runs this way (that is how the C2 numbers were
produced). So the read side is **not** shared-path-only; it lacks a *flag*, not the
capability. That does not kill P1 — a flag matching `mk`'s key-file format is the right
interchange — but the motivation must be restated, and P1 now introduces a **second
spelling for the same fact**, which the spec must resolve:

* inline origin in the template says `48'/0'/0'/2'`
* `--key '@0=[73c5da0a/48'/0'/1'/2']xpub…'` says something else
* `--fingerprint @0=b8688df1` says a third thing

Precedence is undefined. In a seating engine the answer must be **refuse on
disagreement**, and it must be a vector row. Silence here is a wrong-wallet path
reachable from a copy-paste.

### I9 — The two embedded matrices already disagree, on the day the "matrix travels" directive landed.

The directive is normative: *"this table is … embedded, cells kept current, in EVERY
artifact … A document or module missing it is incomplete."*

* `BRAINSTORM_wallet_form_converter.md` — header `in \ out`, **four** output columns:
  concrete descriptor · addresses · keyed card · **keyless + mk1 cards**.
* `SPEC_wallet_form_converter.md` — header `in \ works today`, **three** output columns.
  The fourth is gone.
* The spec's own prose one line above the table names four:
  *"concrete descriptor · addresses · keyed card **· template + origin-notated key
  lines**"*.

The dropped column is where **P3's deliverable lives** — the template plus the
`mk encode --keys` file. A plan derived from the spec's matrix under-scopes P3 and has
no cell in which to mark it closed. It also loses two brainstorm cells outright:
`T → keyless + mk1 cards` (✓ today) and `K → keyless + mk1 cards` (✗ non-goal, the
K→S split), so the spec's *Non-goals* fence for K→S has no counterpart in its goal
statement.

Additionally, one cell is measurably wrong in **both** copies: `T | ⚠ shared-path only`
— see I8; per-slot origins work today via the template text.

---

## MINOR

* **M1** — "the full 1,649-char concrete descriptor": 1,648 characters, 1,649 bytes with
  the trailing newline. Say which.
* **M2** — Rule 6 is titled "Post-seat **verification**" and performs no check; it prints
  an address and delegates to the human. Honest for the card-set input, but for **D** an
  oracle exists (the input descriptor's own address 0) and for the pathological walk the
  keyed card is an oracle for the split set — both automatable, neither claimed. Also
  unspecified: which **stream** the always-printed address 0 goes to. `md descriptor`'s
  stdout is a machine contract today (its notes already go to stderr); "beside the
  descriptor" is not precise enough to implement without breaking a consumer.
* **M3** — Baselines: `md 6c4a56fd` is a revision; **`mk "mk1 v0.1 HEAD"` is not**, so
  the staleness check has nothing to compare against on the mk side (mk is at `93cebfb`,
  version 0.13.0 — "v0.1" is the *format* version, which reads as a rev and is not one).
  Pin the SHA. Separately: descriptor-mnemonic has **no `scripts/` directory**, so the
  `plan-build-gate.sh` / `plan-staleness-check.sh` the repo CLAUDE.md cites do not exist
  here; this spec carries no ```rust blocks, so the gate is CLI-command-shaped — say so
  in the plan rather than citing a script that is not in the tree.
* **M4** — P3's "substitutes `@0..@N` in **first-appearance order**" is ambiguous when a
  key repeats. `md encode` accepts both `wsh(multi(2,@0/<0;1>/*,@0/<0;1>/*))` (n=1) and a
  two-slot form, and rust-miniscript parses a descriptor with the identical key in both
  slots. The choice changes the number of engraved key cards and the WalletPolicyId. Pick
  one, and note the interaction with rule 3's silent dedupe: dedupe-on-read plus
  distinct-slots-on-write makes `compose(decompose(D))` fail rule 5 for such a D.
* **M5** — P3 is described as reusing `compile`'s plumbing, but md's existing walker
  inverts the invariant it needs. `substitute_synthetic` "strips EVERY placeholder suffix
  … and replaces the whole span with a BARE synthetic xpub", and the drift guard at
  `template.rs:2560` is *"NO substituted key may be a `MultiXPub`"*. A concrete-descriptor
  walker sees `MultiXPub` as the **normal** case (probe: every multipath key parses as
  `MultiXPub` with `DerivPaths([0, 1])`). New machinery, not a variant of the old path;
  the plan should budget for it.
* **M6** — Acceptance 3 ("a real foreign card … from another wallet in the corpus") is
  executable — the mk corpus carries synthetic stubs `11223344`, `c0ffee00`, … so a
  mismatch demonstrates. But it passes in both worlds: it proves the check fires on a
  *trivially* different stub while CE-1 (a foreign card that **shares** the stub) goes
  untested. Require the sharing case, and assert what the engine does with it.
* **M7** — Rule 5's leftover-key refusal "naming the card" should name the leftover's
  **stub** as well. A drawer scan is the motivating scenario and the operator's actual
  question is "which wallet do these extra plates belong to?" — the stub is already
  decoded at that point.
* **M8** — The refusal the spec quotes as evidence #1 prescribes a remedy that does not
  work: *"Supply --key @i=XPUB"* while `--key` requires `--template` (spec's own next
  clause). P2 makes this path real; the message should then point at `--from-mk1`. Not
  filed anywhere in the spec.

## NIT

* **N1** — Spec matrix header reads `in \ works today`; the brainstorm's reads `in \ out`.
  If the table is to travel byte-stable across four artifacts and a module doc comment,
  fix the header now.
* **N2** — "the KEYED card (22 chunks, Pubkeys TLV)": 22 strings confirmed, 21 of 86
  characters and one tail of 59. "22 chunks" is right; worth writing the length profile
  once somewhere, since the extraction step (`grep -o 'md1fatzr2[a-z0-9]*'`) is the kind
  of reproduction path that rots.

---

## What was NOT found

Recorded so a later round does not re-derive them:

* **rust-miniscript is sufficient for P3's parsing needs.** Verified at the pinned rev
  `ff4732e`: origins preserved (`Some((73c5da0a, 48'/0'/0'/2'))`), multipath preserved
  (`MultiXPub … DerivPaths([0, 1])`, including 3-way `<0;1;2>`), `sortedmulti` preserved
  and not rewritten, fixed paths `/0/0` accepted, duplicate keys accepted, uppercase
  fingerprints normalised to lowercase, present checksums verified, absent checksums
  tolerated. No parser gap.
* **Rule 3's refusal is CORRECT for fingerprint-bearing cards.** Identical
  `[fp/path]` with different xpubs is impossible from one master, so it means a
  fingerprint collision or a corrupted card. Keep it. (The defect is scope — see I2.)
* **Rule 5's "no leftover" default is right.** Accepting a partial seat is the
  wrong-wallet direction; refusing is correct. Only the refusal text needs work (M7).
* **`me` genuinely has no concrete-miniscript entrance** — `admit.rs` is the seven
  plain forms plus the three `multi` twins. The spec's D-row premise holds.
* **The non-goals fence is sound.** `me sysw pack --as md1` accepting miniscript does
  touch the S2-settled admission predicate; deferring it with its own cycle is right.
* **Key order and checksum recomputation** in the Canonicalisation section are true of
  `me` as measured; only the `h`-spelling and byte-comparability claims fail (I3).
* **Acceptance 2** ("every refusal demonstrated by a vector row that FAILS if the
  refusal is removed") is well-formed and mutation-shaped. It is the strongest item in
  the section; keep it exactly as written and extend it to cover CE-1.

---

## Gate

**RED — 3 Critical, 9 Important.** No code may begin.

The shortest path to GREEN, in dependency order:

1. **C1** is the ruling that shapes the rest: decide whether compose demands a
   WalletPolicyId-rooted stub (an `mk` change, Rust-primary, own cycle) or keeps the
   WDT-id and deletes the binding claim. Everything in rules 1–4 depends on the answer.
2. **C2 / C3** rewrite Acceptance 1 into properties that can pass: address-0 equality
   plus key-set-and-order equality, and a decompose that reconstructs depth/child from
   the origin path before emitting key lines.
3. **I1, I2, I7** are localised edits to rules 1–3 once C1 is settled.
4. **I3, I4, I8, I9** are independent of C1 and can be folded immediately.
5. **I5, I6** need one boundary ruling each, then a refusal string.

Re-dispatch after the fold. CE-1 should become a vector row whatever C1's ruling is —
if the design accepts it, the row records that it is accepted and why.
