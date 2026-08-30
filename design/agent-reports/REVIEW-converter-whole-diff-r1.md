# REVIEW-converter-whole-diff-r1 — pre-merge adversarial review, whole cycle

**Artifact under review:** `git diff 7a3f7a68..9d0c30dc` on `impl/converter-c4`
(phases C0–C4 of the wallet-form-converter cycle).
**Worktree:** `/scratch/code/worktrees/converter-c4`
**Date:** 2026-08-30
**Reviewer:** independent adversarial pass, whole-diff scope. This is the last
gate before merge to main.

**Counts: 1 Critical / 5 Important / 11 Minor / 3 Nit.**
(Body below is 1C/4I/7M/2N; the ADDENDUM at the end adds I5, M8–M11 and N3 from
a records sweep that landed after the body was written. Every addendum claim was
re-verified against the tree before being recorded.)

Scope taken as given (not re-derived, per the dispatch brief): the per-phase
reviews `REVIEW-converter-c2-r1` and `REVIEW-converter-c3-r1` (both 0C/0I/2M,
folded), the four `IMPL-converter-c*.md` reports, and the controller's
reproduction of every phase exit gate. The R0-closed design decisions in
`design/SPEC_wallet_form_converter.md` and
`design/IMPLEMENTATION_PLAN_wallet_form_converter.md` are treated as settled;
this pass targets **what the code does** and **what the records claim**.

---

## Gate reproduction (lens 4) — all clean, one out-of-scope red

Re-run independently in the worktree at `9d0c30dc`:

```
$ cargo nextest run --locked
     Summary [   0.812s] 1047 tests run: 1047 passed, 2 skipped
$ cargo clippy --locked --all-targets -- -D warnings     # exit 0, no output
$ cargo fmt --check                                      # exit 0, no output
$ cargo nextest run --locked -E 'binary(acceptance_walks)'
     Summary [   0.007s] 9 tests run: 9 passed, 0 skipped
$ bash scripts/matrix-identity-check.sh
matrix identical across 4 homes, 6 lines each:
sha256: 0527deee4a60b60385b07d7bfda8d5987672d2d5195ced56db0780836a5baab1
```

Every one of these matches the value quoted in `IMPL-converter-c4.md`,
including the matrix sha to all 64 digits. **The IMPL reports' gate quotes are
honest.**

**Oracle independence, checked one level deeper than C3's reviewer did.**
`crates/md-cli/tests/common/facts.rs` imports exactly three things
(`miniscript::ForEachKey`, `miniscript::descriptor::{Descriptor,
DescriptorPublicKey}`, `std::str::FromStr`) and
`crates/md-cli/tests/acceptance_walks.rs` imports exactly `assert_cmd::Command`,
the same two miniscript items, `FromStr`, and `facts`/`spend_equal`/
`spend_equal_report` from `facts.rs`. Neither touches `src/seat` or
`src/decompose`. This is structurally guaranteed as well as textually true:
`crates/md-cli/Cargo.toml` declares only `[[bin]] name = "md"` and no `[lib]`,
so an integration test **cannot** link the implementation even if it wanted to.
Independence confirmed.

**No test in the new files can silently skip.** Swept
`acceptance_walks.rs`, `seating_vectors.rs`, `cmd_decompose.rs`,
`cmd_decompose_roundtrip.rs` and `cli_p1_origin_key.rs` for `#[ignore]`, env
gating, existence guards and bare early `return`. One `std::env::var("PATH")`
hit at `cmd_decompose.rs:536` is building a `PATH` for a subprocess, not a skip
gate; that test then `assert!`s on the subprocess status. The `2 skipped` in the
nextest summary are pre-existing and outside this diff.

**Out-of-scope red, reported for completeness (M5 below):** under
`--all-features` the suite is RED — `md-cli::bin/md
compile::tests::upstream_display_is_still_broken_delete_local_renderer_when_this_fails`
fails. This is **not** introduced here and does not gate this merge; see M5.

---

## CRITICAL

### C1 — the T row composes a BIP-388-forbidden shape-(1) wallet that `md encode` refuses and `md decompose` refuses: `sortedmulti(2,X,X,Y)` ships with exit 0 and no warning

**This is the seam the controller named, and it is real.**

The spec's guarantee, `design/SPEC_wallet_form_converter.md` (A3):

> The converter refuses BOTH forbidden shapes in BOTH directions — compose
> refuses to emit, decompose refuses as input. Rows: shape (1) both directions

and, naming shape (1) as the binding case:

> while shape (1) (the same xpub filling two slots) is the reachable case where
> the engine's refusal binds.

and, the consequence the spec calls dead:

> (b) r7-C1's missing-card fabrication (`sortedmulti(2,X,X,Y)` from a 2-of-3
> missing Z — measured, X alone controlling funds) is **doubly dead**: A4's
> unfilled-slot refusal AND the BIP-388-unsupported refusal — both ship as
> permanent must-REFUSE rows

**Measured, 2026-08-30, at `9d0c30dc`, `cargo build --bin md`:**

```
$ X=xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf
$ Y=xpub6FQya7zGhR92kacYsNnjreouvnHJMpXYsUXnW6NJJAJRCKsa26TzDy4LdnGhEurr3d6y1J8PJ7EEMKQp74XTqYvmGJNogYXSKDszYHtF8mX
$ md descriptor --template "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))" \
    --key "@0=$X" --key "@1=$X" --key "@2=$Y"
wsh(sortedmulti(2,xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/<0;1>/*,xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/<0;1>/*,xpub661MyMwAqRbcGCyvirZEuFJCJKttDAf4taPXZUbkAhkPU9DHxQBWa6xhX19Hm1o3CC9TcZxhaMpvUjuMGSVchJzieDUX7YUoavpPGd6SKB1/<0;1>/*))#wuqaypwh
note: stdout is watch-only — public keys only, cannot spend
$ echo $?
0
```

That is `sortedmulti(2,X,X,Y)` — verbatim the wallet the spec calls "doubly
dead", emitted clean. `md address` has the identical hole (measured: same flags,
prints `bc1ql5j095gqvdv6ugccf956pduc2e0vevtfnf9r72nhmln9lf8tlmmsd9ujlz`, exit 0).
The two-slot form composes too, with a checksum:

```
$ md descriptor --template "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))" \
    --key "@0=[73c5da0a/48'/0'/0'/2']$X" --key "@1=[73c5da0a/48'/0'/0'/2']$X"
wsh(sortedmulti(2,[73c5da0a]xpub661My…/<0;1>/*,[73c5da0a]xpub661My…/<0;1>/*))#xn0gcxt8
```

**The funds consequence, in the repo's own words.**
`crates/md-codec/src/validate.rs:339-344`:

> Such a policy reads as k-of-n and is satisfiable by fewer parties than it
> names: one key seated twice lets its holder produce two of the required
> signatures. The script is legal; the wallet is not what it looks like.

So the operator who is missing one cosigner's xpub and pastes the one they have
into two slots receives a clean, checksummed 2-of-3 that **one key alone can
spend**, with no diagnostic.

**Why it is reachable here and nowhere else — the mechanism.** The guard exists
and is one call away. `validate_no_duplicate_key_slots` is defined at
`crates/md-codec/src/validate.rs:361` and has **exactly one call site in the
whole workspace**:

```
$ grep -rn "validate_no_duplicate_key_slots" crates/ --include="*.rs" \
    | grep -v "^crates/md-codec/src/validate.rs"
crates/md-codec/src/encode.rs:120:    crate::validate::validate_no_duplicate_key_slots(d)?;
```

`md encode` encodes, so it hits the guard.
`md descriptor` / `md address` build a `Descriptor` through
`cmd::build::build_descriptor` → `parse::template::parse_template` and **render
it without ever encoding**, so the guard never runs. Three verbs in one binary
give three different answers about the same wallet:

| verb | same xpub on `@0` and `@1` | measured |
| --- | --- | --- |
| `md encode` | **REFUSES** | `md: codec error: @0 and @1 carry the same key at the same use-site: this policy names 2 cosigners but one of them holds two of the seats` |
| `md descriptor` / `md address` | **COMPOSES, exit 0** | descriptor above |
| `md decompose` | **REFUSES** | `md: decompose: the key expression … appears at 2 positions whose multipath sets are NOT DISJOINT … Forbidden by BIP 388 — UNSUPPORTED here, never invalid.` |

`md decompose` refuses **the exact string `md descriptor` just emitted** —
verified by piping one into the other. The converter contradicts itself across
its own two directions.

**Honest classification of the spec question the brief asked.** Does the spec's
key-reuse rule bind the T row, or only the engine? **It binds the T row.** The
spec does contain a scope carve-out, but it is for **shape (2)**, not shape (1):

> Measured scope note: md's template surface is NARROWER than BIP 388 here and
> currently INVERTS it — `md descriptor` refuses the BIP-LEGAL disjoint form
> (`wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))` → "@0 appears with inconsistent
> path/multipath/hardening") while composing the BIP-FORBIDDEN same-path form
> (`wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))` composed clean; both measured
> 2026-08-30) … `md descriptor`/`md encode`'s current acceptance of the
> same-path form predates the ruling and is FILED as an md-side question rather
> than changed by this spec (see FOLLOWUPS).

Every example in that carve-out is **`@0` twice** — one placeholder at two
positions, which is shape (2). My probe is **one xpub bound to two DISTINCT
placeholders `@0` and `@1`**, which is shape (1) — the case the same paragraph
calls "the reachable case where the engine's refusal binds", and which the
matrix's T row marks `✓ P1 (shipped this cycle)`
(`design/SPEC_wallet_form_converter.md:80`). The carve-out does not cover it.
Note also that the carve-out's own wording ("`md encode`'s current acceptance of
the same-path form") is itself falsified for shape (1) — `md encode` **refuses**
it, measured above.

**The CHANGELOG tells the operator the opposite (lens 5).** The entry added by
this diff, under the heading "### Refused, deliberately — BIP-388 key reuse":

> **Both converter directions refuse it**, at three points: … the same extended
> key offered for two slots, or found at two positions of a descriptor being
> decomposed.

The three enumerated points are the policy check, the door check and the
card-set/decompose check — all of them S-row or D-row. The T row is absent, and
the sentence that frames them says "both directions". An operator who reads the
CHANGELOG and then builds a descriptor with `--template --key` believes md is
protecting them from precisely the wallet md is about to hand them. That makes
this a false record as well as a code defect.

**Severity.** Critical on all three of the brief's triggers: a wrong result
(a wallet that is not what it looks like), a funds-unsafe path (X alone spends),
and an unmet spec guarantee reachable through an entrance the cycle's own matrix
marks shipped.

**Caveat stated plainly, because it bears on the remedy, not the severity.**
The `md descriptor --template --key` path is **pre-existing** — it is reachable
at `7a3f7a68` too. What this cycle did was (a) newly *promise* to close it in
both directions, (b) actually close it in the S row (`seat/satisfy.rs:285`,
"Card-set check 2 — the same xpub offered twice") and the D row
(`cmd_decompose.rs:422`), and (c) newly extend the T row with origin-notated
`--key`. So this is not a regression the diff introduced; it is a guarantee the
diff asserts and does not deliver, on the one row it left unguarded. The
minimal, non-prescriptive observation: the check that would close it already
exists, already carries the right message, and is already called from one place.
I am not prescribing where the call goes — reproduce the defect, not the remedy.

---

## IMPORTANT

### I1 — the origin-notated `--key`'s bracket path is accepted and then silently discarded, so the natural `md decode` → `md descriptor` journey emits a truncated or actively FALSE origin, exit 0, no warning

C1 of this cycle added `--key '@i=[fingerprint/path]xpub'`. The bracket
**fingerprint** is used. The bracket **path** is not — it is only cross-checked
against the *inline template* path, and only when one exists.
`crates/md-cli/src/cmd/build.rs` says so deliberately:

> An origin-notated `--key`'s bracket PATH is NEVER a source of descriptor path
> data — it exists only to be checked against the slot's inline template path
> when BOTH are present (V-PATHAGREE)

That matches the R0-closed spec, so the *precedence* is not what I am
questioning. What is unaddressed is the **outcome when the bracket path is the
only path the operator stated**, which is neither refused nor warned.

**Manifestation A — truncation. The journey, measured end to end.**

```
$ TPL=$(md decode md15zfdsssjjtvyyw2fdssj54qqxppcgsc97v883w6pfw0za5z5u79mg9qp3wxzvhhu3l6n)
$ echo "$TPL"
wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))
```

`md decode` prints the template on **stdout with no inline origins** — the
origins go to a stderr *note*. So the operator's template has no inline paths.
The operator's key material is in `[fp/path]xpub` form — which is exactly what
`md decompose --emit keys` emits and exactly what an `mk encode --keys` file
holds (see the `# @@ keys` section of
`crates/md-cli/tests/fixtures/decompose/v-d-rt.txt`). Combining the two:

```
$ md descriptor --template "$TPL" \
    --key "@0=[73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2…" \
    --key "@1=[73c5da0a/48'/0'/1'/2']xpub6Dzhyrn…" \
    --key "@2=[73c5da0a/48'/0'/2'/2']xpub6EGx8sP…"
wsh(sortedmulti(2,[73c5da0a]xpub661My…/<0;1>/*,[73c5da0a]xpub661My…/<0;1>/*,[73c5da0a]xpub661My…/<0;1>/*))#ra3r4ldv
note: stdout is watch-only — public keys only, cannot spend
$ echo $?
0
```

All three origin paths are gone. Ground truth for this wallet, from the pinned
fixture's `# @@ canonical-descriptor` line, is `#auwzhqew` with
`[73c5da0a/48'/0'/0'/2']` etc. The emitted `[73c5da0a]` with a depth-0 xpub is a
BIP-380 statement that the key **is** master 73c5da0a — it is not.

**There is no correct alternative through this journey.** `--path` is a *shared*
path; this wallet's three slots are at three different paths
(`48'/0'/0'/2'`, `48'/0'/1'/2'`, `48'/0'/2'/2'`). The only per-slot path input
the CLI offers is the inline template origin — which `md decode` did not give
the operator — and the `--key` bracket, which is discarded. So for a
divergent-origin policy card, the T row cannot express the wallet at all, and it
does not say so.

**Manifestation B — a silent override producing an actively false origin.**
When `--path` *is* supplied, `apply_path_override_per_slot`
(`crates/md-cli/src/parse/path.rs`) fills every slot not in `inline_declared`.
A bracket path never enters `inline_declared`, so `--path` overwrites it:

```
$ md descriptor --template "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*,@2/<0;1>/*))" \
    --key "@0=[73c5da0a/48'/0'/0'/2']X0" \
    --key "@1=[73c5da0a/48'/0'/1'/2']X1" \
    --key "@2=[73c5da0a/48'/0'/2'/2']X2" --path "48'/0'/0'/2'"
wsh(sortedmulti(2,[73c5da0a/48'/0'/0'/2']…,[73c5da0a/48'/0'/0'/2']…,[73c5da0a/48'/0'/0'/2']…))#963szlwy
```

Slot `@1` now declares `[73c5da0a/48'/0'/0'/2']` for a key the operator
explicitly told md is at `48'/0'/1'/2'`. This is worse than truncation: it looks
complete and it is wrong. And it is a **silent override on the path datum**,
which is precisely what the spec forbids on the sibling datum — "when BOTH name
slot i they must agree (mismatch refuses — never silent override)" — and what
V-PATHAGREE enforces for the inline-vs-bracket pair. The bracket-vs-`--path`
pair is the one combination left unguarded.

**Coverage gap confirmed.** There is no test row for it.
`crates/md-cli/tests/cli_p1_origin_key.rs` carries V-KEYORIG, V-FPAGREE,
V-PATHAGREE and V-PRECEDENCE; V-PRECEDENCE
(`v_precedence_shared_path_fills_only_the_slot_without_an_inline_origin`,
line 358) uses **bare** keys via `xpub_at(...)`, so no bracket path is in play
in any `--path` test.

**Impact.** Addresses are unaffected — I verified both forms derive the same
`bc1q2sz6vvu6k7y9gtc6kfgfe0p6xkhmvmdlu97eecjkykpdktvps08scdjgr5`, so the backup
is watch-only-recoverable. What breaks is **spending**: a signer handed
`[73c5da0a]` (or the false `[…/48'/0'/0'/2']`) derives the wrong child and
cannot match the key. This repo already names that exact failure mode as the one
it exists to prevent — `Cargo.toml`'s miniscript pin comment: "md derived
correct addresses and emitted a descriptor no other wallet could parse, which is
the 'not recoverable with shipped tooling' the DD6 advisory named."

**The CHANGELOG asserts the guard that is missing.** The entry added by this
diff says of the origin-notated `--key`:

> Where two sources name the same slot they must AGREE or the command refuses —
> a disagreeing fingerprint or **path** is never silently overridden.

For the fingerprint datum that is true (V-FPAGREE, measured). For the path
datum it is true only of the inline-template-vs-bracket pair; the
`--path`-vs-bracket pair is silently overridden, as measured above. The record
promises a refusal the code does not perform.

**Why Important and not Critical:** the code matches the R0-closed spec's stated
precedence, so no *stated* guarantee is unmet, and the wallet remains
watch-only-recoverable. It is an Important-class missing refusal/warning on a
reachable path that produces a materially wrong origin record.

### I2 — a case-variant double scan defeats step 1 of the normative input pipeline and draws a refusal that misdiagnoses the cause and prescribes re-minting a good card

`crates/md-cli/src/seat/input.rs` states the pipeline's whole purpose:

> The ORDER is the whole content. `mk decode` itself has no dedupe step … so a
> double scan of one card is only harmless because step 1 runs first.

and the spec (A3(a)):

> An accidental double-scan is made harmless BY ORDER OF OPERATIONS, not by
> assumption

`dedupe_strings` normalises **whitespace only** (`s.chars().filter(|c|
!c.is_whitespace())`). It does not normalise case. But mk1 strings **are**
case-insensitive at the decoder — measured: an all-uppercase card set seats
identically to the lowercase set, same descriptor, same checksum `#9uzthz8n`.

So the same card supplied once lowercase and once uppercase survives step 1 as
two strings, merges into one group at step 2, and blows up at step 3:

```
$ md descriptor <policy> --from-mk1 $A --from-mk1 $B --from-mk1 $C --from-mk1 $D \
    --from-mk1 $E --from-mk1 $(echo $A | tr a-z A-Z)
md: seating refused: chunk-set 373cd: the 3 string(s) declaring this id do not
reassemble into one key card: chunked-header malformed: received 3 chunks,
header declares total_chunks = 2. Two DIFFERENT cards pinned to one chunk-set id
merge into one group here and refuse exactly like this — re-mint one of them so
the set ids differ
```

Two defects in one:

1. **The guarantee is unmet.** The double scan was *not* made harmless. Controls
   confirm the surrounding cases work: byte-identical double supply of the whole
   set seats (`#9uzthz8n`); a copy with spaces inserted every 10 characters
   dedupes correctly (`#9uzthz8n`).
2. **The diagnostic is wrong and its remedy is harmful.** The cause is one card
   scanned twice; the message asserts "Two DIFFERENT cards pinned to one
   chunk-set id" and tells the operator to **re-mint one of them**. An operator
   who follows that instruction re-engraves a plate to fix a problem that does
   not exist. Under the journey rule this is worse than telling the user
   nothing.

Uppercase is not exotic: it is the canonical bech32 form for QR encoding and is
what a scanner or an all-caps transcription produces, and md's own decoder
accepts it everywhere else.

### I3 — `md encode` still sends a user holding a concrete descriptor to a different binary in another repo, for the conversion `md decompose` now performs

`crates/md-cli/src/parse/template.rs:340-347` (untouched by the diff — the
nearest hunk starts at `@@ -290`):

> `"this is a concrete wallet descriptor (it carries a real extended key), not an md1 template — `md encode` takes a template whose keys are `@i` placeholders. A concrete descriptor is packed by the engraver's own converter:\n    me sysw pack --as <descriptor|md1> --in <your export file>"`

and its doc comment at line 317, "refer to the tool that takes it".

Before this diff that was true. After it, **md is the tool that takes it** —
`md decompose <DESCRIPTOR>` is the whole D row and it shipped in this cycle. The
operator who does the most natural wrong thing (paste a concrete descriptor into
`md encode`) is told md cannot help and pointed at `me`, a separate binary in a
sibling repo they may not have installed.

This is compounded by an untouched test that **pins** the stale referral:
`crates/md-cli/tests/cli_f420_descriptor_referral.rs:12`
`const REFERRAL: &str = "me sysw pack --as <descriptor|md1>";`, whose header
still reads "no recognition, no referral, from the tool NAMED for descriptors".
Meanwhile `crates/md-cli/tests/cmd_decompose.rs:220` (added by this diff)
asserts decompose does **not** emit that referral. Two test files in one suite
now encode opposite assumptions about whether md can read a descriptor.

Classified Important rather than Minor because it is a false record that changes
an operator's decision — it routes them away from the feature this cycle exists
to ship, at the exact moment they need it.

*(The BlueWallet arm at lines 340-341 is a genuinely different case —
`decompose` takes a descriptor, not a `Key: value` export — and is correct
as-is.)*

### I4 — `--key`, `--fingerprint` and `--path` are accepted and silently ignored on the new seating route; the `requires = "template"` that should have refused them never fires

`cmd/descriptor.rs`'s seating branch constructs
`seat::SeatingRequest { phrases, from_mk1, seats, network, cmd }` — it does not
carry `keys`, `fingerprints` or `path` at all. Those flags are therefore
inert on this route, and nothing says so:

```
$ md descriptor <policy card> --from-mk1 A B C D E
wsh(multi(2,xpub661My…/<0;1>/*,xpub661My…/<0;1>/*))#9uzthz8n

$ md descriptor <policy card> --from-mk1 A B C D E \
    --path "48'/0'/0'/2'" --fingerprint "@0=73c5da0a" --fingerprint "@1=73c5da0a"
wsh(multi(2,xpub661My…/<0;1>/*,xpub661My…/<0;1>/*))#9uzthz8n     # identical, exit 0

$ md descriptor <policy card> --from-mk1 A B C D E --key "@0=xpub6DkFAXWQ2dHxq…"
wsh(multi(2,xpub661My…/<0;1>/*,xpub661My…/<0;1>/*))#9uzthz8n     # identical, exit 0
```

Same checksum all three times. `--key` is funds-relevant material: an operator
supplying an xpub to a wallet-composition command has it discarded without a
word. This bites hardest on exactly the wallets that need it — the v-ce1 policy
above declares fingerprint-free origins, so the composed descriptor carries **no
origin metadata at all**, and the obvious operator response ("add the origins I
know: `--fingerprint @0=… --path …`") is accepted and does nothing.

**The declared refusal that should have caught it does not fire.** `main.rs`
marks all three flags `requires = "template"`, and `--from-mk1` /
`--seat` `conflicts_with = "template"`, so the combination is declared
impossible. Measured, it is not:

```
$ md descriptor <policy card> --path "48'/0'/0'/2'"     # no --template anywhere
md: descriptor requires wallet-policy mode (Pubkeys TLV): …    # NOT the clap error
```

The `requires` clause only fires when the whole `<PHRASES|--template>` group is
absent (`md descriptor --key @0=X` alone does error). Whenever phrases are
supplied it is inert. This is the "a refusal that does not refuse" class, which
the severity rules keep blocking.

**Pre-existing root, newly extended surface — stated so the fix can be scoped.**
The `requires = "template"` clauses are byte-identical at `7a3f7a68`
(`git show 7a3f7a68:crates/md-cli/src/main.rs` lines 294/297/301 and
328/331/341), so the dead constraint predates this branch. What this diff adds
is a route where the silent ignore happens on a **successful composition** that
prints a descriptor, rather than on a path that refuses shortly afterwards.

**Not a wrong wallet, checked.** The adversarial case — 2 cards plus a loose
`--key` for a 3-slot policy — is caught by A4: the seating refuses with
"1 slot(s) unfilled" rather than composing something half-and-half. So no
incorrect descriptor was constructible through this seam; the defect is the
silent discard and the unfired refusal.

---

## MINOR

**M1 — the shipped-surface enumerations were not updated and now omit
`decompose`.** Three sites, all untouched by the diff:
- `crates/md-cli/README.md:39-49` — the `## Subcommands` table, which root
  `README.md:64` and `crates/md-codec/README.md:42` both designate as *the*
  subcommand reference. No `decompose` row (diff-caused) and no `descriptor`
  row (pre-existing).
- `bip/bip-mnemonic-descriptor.mediawiki:1247` — the BIP draft's "Reference
  Implementation" list of what the binary ships. No `md decompose`.
- `README.md:147` — "for ad-hoc encode/decode/verify/inspect/bytecode/vectors
  operations". Already stale pre-diff (omits address, descriptor, repair,
  compile, gen-man); the diff's good new "Moving between wallet forms" section
  at `README.md:116-134` now reads as a contradiction eleven lines above it.

**M2 — `crates/md-cli/README.md:46`'s `md address` row lists the input modes as
exactly two.** The diff added a third (keyless md1 phrases + `--from-mk1` /
`--from-mk1-file` / `--seat`, `main.rs:412/418/437`) and widened `--key` to the
origin-notated form (`main.rs:379-384`).

**M3 — "Mirrors `md encode --path`" is now attached to semantics that do not
mirror it.** The diff's new help for `md descriptor --path` / `md address
--path` (`main.rs:315`, `main.rs:402`) reads "applied PER SLOT … a slot's inline
template origin always wins. Mirrors `md encode --path`". But `md encode
--path`'s own untouched help (`main.rs:135-137`) says it "flattens Divergent
mode to Shared", and `cmd/encode.rs:543` says "`--path` replaces the declaration
wholesale". `md verify --path` (`main.rs:219`) carries the same "Mirrors"
phrasing with the wholesale semantics. Three subcommands now claim to mirror one
another while two of them differ. This one sits *inside* a touched file, so it
is diff-internal, not a sweep miss.

**M4 — the "TRUE binding" claim for the wallet-confirmed tier is scoped to the
accidental threat model, and does not say so.** `seat/disposition.rs:16-17`:
"Wallet-confirmed is a TRUE binding: a foreign card cannot reach that tier, so
CE-1 is impossible for it." In code the tier is
`c.card.policy_id_stubs.contains(&wallet)` where `wallet` is the top-4 of the
`WalletPolicyId` **of the descriptor that includes this card**. For a card
minted for a different wallet the match is a 2^-32 accident, so the claim holds
for CE-1's actual scope (a drawer scan, a mixed-up card). It does **not** hold
against an adversarial minter who knows the cosigner set: `policy_id_stubs` is a
`Vec<[u8;4]>` any-of, and such a minter can compute the composed id of the
wallet that results from substituting their own key and mint a stub for it. The
code's absolute phrasing invites the wrong reading. Recording, not blocking —
the substitution threat is outside CE-1 and the note text itself is accurate.

**M5 — `--all-features` is RED, pre-existing and ungated by CI.**
`cargo nextest run --locked --all-features` → `406 passed, 1 failed, 2 skipped`
(677 not run, fail-fast). The failure is
`compile::tests::upstream_display_is_still_broken_delete_local_renderer_when_this_fails`
at `crates/md-cli/src/compile.rs:352`:

```
left:  "tr(@4,{{pk(@3),pk(@2)},{pk(@1),pk(@0)}})"
right: "tr(@4,{{pk(@3),pk(@2),pk(@1),pk(@0)}})"
```

i.e. upstream Display no longer flattens, so the local `render_tr_template`
should be deleted. **Derivation that this is not introduced here:** the diff
touches neither `crates/md-cli/src/compile.rs` (absent from `git diff --stat`)
nor the miniscript pin — the only `Cargo.lock` change is the addition of
`mk-codec 0.5.0` and its three deps, with the `miniscript` entry byte-identical.
The test's inputs are therefore unchanged from `7a3f7a68`, so it failed there
too. It is invisible to CI because `.github/workflows/ci.yml:48` runs
`cargo test --workspace --all-targets` with **no `--all-features`**, and
`compile` is behind `#[cfg(feature = "cli-compiler")]`. Reported so it is on the
record and can be filed; it does not gate this merge.

**M7 — one brainstorm decision entry still states the retracted S→K premise.**
`design/BRAINSTORM_wallet_form_converter.md:47`, decision 2: "no new surface
needed, only bridges into that call." That is the claim C4 measured false.
Borderline-historical — it sits in a "Decisions made live, with their reasons"
log and the retraction is stated twelve lines above it in the same file
(`:32-36`, "One cell this cycle owned did NOT close … so the bridge refuses —
filed as `md-cannot-mint-a-keyed-card-from-a-split-set`") — so it reads as a
dated decision that was later revised. Recording it because a reader who greps
for "bridge" lands on the stale sentence as easily as on the retraction.

**M6 — `md decompose -` does not read stdin while `md decode -` does.**
Measured: `echo <md1> | md decode -` succeeds (exit 0, prints the template);
`echo <descriptor> | md decompose - --emit template` refuses with
`this is not a descriptor md can parse: unrecognized name '-'`. `decompose` has
`--in FILE` by design and the refusal is otherwise well-formed, but the `-`
convention was deliberately generalised across the reading verbs in this same
cycle's P3 §6b (`cmd/mod.rs:20-33`) and `decompose` is the one reading verb that
did not get it. The message also does not mention `--in`.

---

## NITS

**N1 — `crates/md-cli/tests/cli_f420_descriptor_referral.rs` header is stale
prose.** "from the tool NAMED for descriptors" no longer describes a tool
lacking a descriptor entrance. Bundled with I3 since the fix is the same edit.

**N2 — the sibling-repo CLI manual is a lockstep obligation this diff triggers.**
`CLAUDE.md:35` requires `docs/manual/src/40-cli-reference/42-md.md` in
`bg002h/mnemonic-toolkit` to be updated in lockstep with any flag/API change,
gated by `tests/lint.sh flag-coverage`. This diff adds one subcommand and six
flags. That file is outside this worktree so I could not verify its state;
flagging the trigger only.

---

## What I looked for and did NOT find (negatives, scoped)

Stated explicitly so a later reader knows how wide these negatives are.

- **Cross-phase flag seams are safe, by measurement.** `--seat` + `--key`,
  `--from-mk1` + `--key`, `--from-mk1-file` + `--template` are all refused by
  clap's `conflicts_with = "template"` combined with `--key`'s
  `requires = "template"` (measured: `error: the argument '--template
  <TEMPLATE>' cannot be used with '--seat …'`). `--seat` with phrases but no
  cards hits the explicit guard in both `cmd/descriptor.rs` and
  `cmd/address.rs`. `md address` mirrors `md descriptor` line for line on the
  seating branch.
- **`--seat` cannot place a card the engine would not.** Measured all three
  ways: a nonexistent id refuses naming the supplied ids; an id that violates
  the slot's declared origin refuses quoting both origins and restating the
  guarantee; the legitimate id succeeds.
- **A4 completeness holds.** A short card set refuses naming the count of
  unfilled slots and leftover cards.
- **`--from-mk1-file` refuses rather than ignores.** A junk line and an md1
  string in an mk1 file each refuse by line number and by what the line starts
  with.
- **`md decompose` entrance parity holds.** argv and `--in FILE` produce
  byte-identical refusals on the shape-(1) repeated-key input. Two descriptors
  draw the receive/change-pair guidance, not a parse error.
- **The dedupe/grouping order does not produce a different wallet.** Byte-
  identical double supply of the whole card set seats to the same descriptor and
  checksum; separator variants dedupe. The only order-related defect found is
  I2, which is a spurious *refusal*, not a wrong wallet.
- **No card set was found that seats a wrong wallet with exit 0 and NO warning.**
  The CE-1 foreign-card path emits both a `SHAPE-CONFIRMED` note naming the
  limitation and the address-0 note with the standing compare instruction. The
  wallet-confirmed tier's unreachability holds within CE-1's threat model
  (see M4 for its boundary).
- **Every follow-up filed this cycle carries a real, FUTURE owning phase
  (lens 5).** Checked all eight entries added by
  `git diff 7a3f7a68..9d0c30dc -- design/FOLLOWUPS.md`. None is owned by a phase
  that has already closed. `md-decompose-rejects-double-wildcard-input` is
  marked "**C4 CONSIDERED AND DECLINED 2026-08-30**" — a documented decline, not
  a silent carry. The one entry that quotes an owning phase of `C4`
  (`md-verify-against-flag-for-cross-form-comparison`, `FOLLOWUPS.md:2213`) is
  quoting the *prior* C2 item it closes, and C4 discharged it with an explicit
  decision plus measured evidence, re-owning the residue to the post-converter
  mini-cycle. That is compliant with the per-phase burndown rule.
  **Gap:** I3 (the stale `me sysw pack` referral) is filed nowhere —
  `grep -rn "sysw pack\|F-420\|referral" design/FOLLOWUPS.md` returns empty.
- **The S→K bridge retraction IS consistently propagated (lens 5).** The
  controller folded `SPEC:358-361`; I checked every other site that could still
  assert it. All four matrix homes carry `✗ P2+bridge — md encode --key needs a
  depth-3/4 xpub, a card composes depth-0 (measured C4, filed)`, and byte-
  identity across the four is machine-proved by `matrix-identity-check.sh`
  (sha `0527deee…`, re-run above). The retraction prose appears in all four
  homes too — `BRAINSTORM:35`, `SPEC:94`, `PLAN:61`, `seat/mod.rs:56`. The
  remaining "bridge" hits (`SPEC:75`, `PLAN:43`, `seat/mod.rs:38`) are D-row
  prose, and the D→keyed-card cell is legitimately still `✓`. One stale
  sentence remains, filed as M7.
- **Search scope for the above:** `crates/md-cli/src/{cmd,seat,decompose,parse}`,
  `crates/md-codec/src/validate.rs`, `crates/md-cli/tests/`, plus live
  invocation of the built binary. The negatives are exactly as wide as that.

## Secret-handling

None found in this diff. Per the standing rule these would not gate in any case.

---

## Verdict

**NOT GREEN. 1 Critical / 5 Important open** (I5 is in the ADDENDUM below).
C1 is the blocker: the cycle's own
spec promises a compose-side refusal for shape-(1) key reuse, the S and D rows
deliver it, the T row does not, and the result is a funds-unsafe descriptor
emitted with exit 0 that md's own `decompose` refuses to read back.

---

# ADDENDUM — records sweep, landed after the main body

A second records pass returned after the body above was written. Every claim
below was re-verified by the reviewer against the tree before being recorded
here; nothing is transcribed from the sweep unchecked. **Revised counts: 1
Critical / 5 Important / 11 Minor / 3 Nit.** The Critical is unchanged.

## IMPORTANT (new)

### I5 — `v_d_rt_mk_encode_keys_accepted_the_emitted_file` cannot fail on the property its name asserts, and the reproduction path behind it has decayed

`crates/md-cli/tests/cmd_decompose_roundtrip.rs:285-304`. The test's name claims
`mk encode --keys` accepted decompose's emitted key file — SPEC Acceptance 1(c).
Its body asserts three things:

1. the `mk1-cards` fixture section's lines start with `mk1`;
2. `header.contains("mk encode --keys keys.txt --from-md1-set policy.md1")`;
3. `header.contains("route 2 (md encode")`, with the failure message *"the
   fixture must record mk's measured exit code"*.

Assertions 2 and 3 grep the fixture file's **own comment header** for literal
strings. They pass whether or not `mk` accepts anything, so the test reports a
PASS for a property it never exercises — and assertion 3's message says "exit
code" while the substring it looks for is provenance prose, not an exit code.

**The reproduction path is dead, so the claim cannot be refreshed either.**

```
$ command -v mk ; mk --version
/home/bcg/.cargo/bin//mk
mk 0.13.0
$ mk encode --keys /dev/null
error: unexpected argument '--keys' found
```

`crates/md-cli/tests/fixtures/decompose/v-d-rt.txt:13-15` records that route 2
was "eval'd verbatim in a scratch directory with md and mk on PATH … route 2
(md encode --out, then mk encode --keys --from-md1-set) = 0". With the `mk` on
PATH today that is not reproducible; `generate.sh` would fail. This is the
reproduction-decay class: an artifact keeps vouching for a generator nobody can
re-run.

**Stated fairly, because it bounds the severity.** The mk1 cards in the fixture
are real artifacts and they *are* executed —
`v_d_rt_round_trip_equality_through_the_split_set` (line 341) seats them through
`md descriptor --from-mk1` and asserts round-trip equality, and it passes. The
module doc at `:31` is honest that "`generate.sh` runs the REAL `mk encode
--keys`", i.e. in the generator rather than the test. So the acceptance is not
vacuous end-to-end. What is unbacked is the one assertion named for mk's
acceptance, and the ability to re-derive it. Filed Important because "a test
that reports a false PASS" is named in the severity rules as still blocking, on
a clause of a funds-relevant acceptance.

## MINOR (new)

**M8 — `README.md:128` overstates `--emit commands` by one form.** The comment
reads `# D → the mint commands for T, S and K`. Measured: the output carries
exactly two route headers —

```
$ md decompose "<v-d-rt canonical descriptor>" --emit commands | grep -c '^# ── route'
2
# ── route 1: the KEYED card — one md1 artifact carrying template + keys ──
# ── route 2: the SPLIT set — a keyless policy card + one mk1 card per key ──
```

There is no T mint command; the template appears only as an argument inside the
two routes. `CHANGELOG.md:54-56` gets this right ("prints **both** mint
routes"), so README and CHANGELOG disagree and README is the wrong one.

**M9 — "the converter makes moving between them cheap" is false in both
directions.** `README.md:132` (added by this diff) and
`design/SPEC_wallet_form_converter.md:420-422`:

> Keyed (compact, monolithic) and split (distributable custody) are peers; the
> converter makes moving between them cheap and recommends neither.

S→K refuses from both ends — this cycle's own C4 measurement, and the matrix
cell is `✗`. K→S is a declared non-goal three lines below the SPEC sentence
(`:424-425`, "K→S splitting … mechanical but no motivating journey yet"), and
its matrix cell is `✗ non-goal`. Neither direction is cheap; neither exists.
The brainstorm's version of the sentence
(`design/BRAINSTORM_wallet_form_converter.md:48-49`) has no "cheap" in it, so
the word was added downstream. This diff newly propagates it into README.

**M10 — the seating engine's own module doc lists a keyed card among its output
forms.** `crates/md-cli/src/seat/mod.rs:37-38`:

> `//! Output forms: concrete descriptor · addresses · keyed card (via the`
> `//! existing `md encode --key` bridge) · template + origin-notated key lines.`

Sixteen lines later the same comment block retracts exactly that
(`:54-57`, "so the bridge refuses — filed as
`md-cannot-mint-a-keyed-card-from-a-split-set`"). The sentence is
self-contradicting *within one doc comment*. The identical text at
`design/SPEC_wallet_form_converter.md:74-75` and
`design/IMPLEMENTATION_PLAN_wallet_form_converter.md:42-43` is converter-wide,
where D→K keeps it partly true; in `seat/mod.rs` it is scoped to the S row and
is simply false.

**M11 — C4's own task list still says every cell flips.**
`design/IMPLEMENTATION_PLAN_wallet_form_converter.md:228`:

> `2. Matrix cells flip to ✓ in all four embedded copies, same commit.`

Contradicted twice in the same file: `:58` ("One cell this cycle owned did NOT
close") and `:338` ("the one matrix cell that did not flip").

## NIT (new)

**N3 — `README.md:134` names three matrix homes; there are four.** It lists
BRAINSTORM, SPEC and IMPLEMENTATION_PLAN. `crates/md-cli/src/seat/mod.rs` is the
fourth, and `scripts/matrix-identity-check.sh` checks all four — it printed all
four homes in the gate re-run at the top of this report. An omission rather than
a false statement.

## Also confirmed by the sweep, no finding

The FOLLOWUPS reproductions were independently re-run and each reproduces:
`md-decompose-rejects-double-wildcard-input`'s `/**` refusal and the `md encode`
asymmetry; `md-decompose-has-no-json-output` (measured: every reading verb has
`--json` except `decompose`); `md-cannot-mint-a-keyed-card-from-a-split-set`'s
two refusals verbatim from both ends; `phase-gate-omits-cargo-doc`'s cited gate
definition, CI `doc` job and fix commit `d75214f7`. One arithmetic looseness in
`md-verify-against-flag-for-cross-form-comparison` ("the strings differ in 254
characters" against a measured 253-char delta) — not load-bearing, recorded only.
Fifteen distinct CHANGELOG claims were verified TRUE against the binary; the two
CHANGELOG claims this report does contest are named in C1 and I1 above.
