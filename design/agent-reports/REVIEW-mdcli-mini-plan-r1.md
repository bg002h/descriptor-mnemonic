# REVIEW — IMPLEMENTATION_PLAN_mdcli_mini.md, R0 round 1

- **Artifact:** `design/IMPLEMENTATION_PLAN_mdcli_mini.md` (status DRAFT p0)
- **Commit reviewed:** `d635458a` (main tip)
- **Spec:** `design/SPEC_mdcli_mini.md`, GREEN at `b8a64938`
- **Date:** 2026-08-31
- **Reviewer:** independent adversarial architect (R0), opus
- **Lenses, in priority order:** (1) COVERAGE — every normative spec
  requirement and every named vector row mapped to a phase; (2) SEQUENCING —
  P2's baseline mint after P1, the P2/P3 split, P4↔P5 on one verb;
  (3) EXECUTABILITY — does each step have a definite done-condition;
  (4) GATE — is the quoted gate runnable at each boundary, and can it fail;
  (5) BLAST RADIUS — what the plan makes false elsewhere, and whether each
  claimed FOLLOWUPS closure matches what the entry asks.
- **Taken as settled, not re-derived:** the spec's code citations and
  measurements (four rounds + controller), the operator rulings, the BIP-388
  line numbers. The plan carries no code blocks, so there was nothing to
  compile; its checkable surface is coverage, sequencing and executability,
  and everything below was run rather than read.

## THE ONE QUESTION

*If the seven phases were executed exactly as written, would the result
satisfy the spec?*

**No.** P2's R-N1a refusal reaches a body of shipped test vectors, tests, a
user-facing corpus generator and a committed fixture generator that the plan
never enumerates — it names two tests to flip, and the true count is at least
five test functions, three MANIFEST vectors, `md vectors` and one
self-truncating shell script. Executing P2 as written leaves the suite red at
P2's own gate with no plan guidance, and every improvisation available to the
implementer either strips mutation-proven cross-language coverage or weakens
the spec. Six further Importants sit around it.

---

## What the reviewer ran (all commands executed at `d635458a`)

```
$ ./target/debug/md encode "tr(@0/48'/0'/0'/2'/<0;1>/*,sortedmulti_a(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))" --group-size 0
exit 0                                    # test_vectors.rs:164 keyed_tr_sortedmulti_a
$ ./target/debug/md encode "wsh(or_i(and_v(v:after(1000000),…multi(2,@0/…,@1/…,@2/…))),and_v(v:older(65535),multi(1,@1/48'/0'/1'/2'/<0;1>/*,@2/48'/0'/2'/2'/<0;1>/*))))" --group-size 0
exit 0                                    # test_vectors.rs:161 keyed_wsh_timelock_hashlock
$ ./target/debug/md encode "wsh(or_d(pk(@0/0/<0;1>/*),pk(@0/0/<0;1>/*)))" --group-size 0
exit 0                                    # cli_unhardened_origin_note.rs:134
$ ./target/debug/md encode "tr(@2/48'/0'/0'/2'/<0;1>/*,{sortedmulti_a(2,@0/…,@1/…),sortedmulti_a(1,@0/…,@1/…)})" --group-size 0
exit 0                                    # generate.sh's V-R5M1 block

$ ./target/debug/md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" --group-size 0
exit 0 -> md1yqpqqxppcggqj6vw67aska9m      # ONE chunk, NO chunk-set-id  (the plan's recipe)
$ ./target/debug/md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" --key @0=<KEY 1> --path "m/48'/0'/0'/2'" --group-size 0
exit 0 -> 2 chunks, chunk-set-id: 0xed813  # the plan's cited id, only with --key/--path
$ ./target/debug/md encode "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=<K> --key @1=<K> --path "m/48'/0'/0'/2'" --group-size 0
exit 0 -> 4 chunks, chunk-set-id: 0x00ee4  # the plan's delta card: CONFIRMED
$ ./target/debug/md descriptor <the 0xed813 pair>
exit 0 -> wsh(sortedmulti(2,xpub…,xpub…))#kuunuuvp     # r3 I-1 confirmed
$ ./target/debug/md descriptor md1yqpqqxppcggqj6vw67aska9m
md: descriptor requires wallet-policy mode (Pubkeys TLV): this card is a keyless TEMPLATE…

$ ./target/debug/md compile 'thresh(2,pk(@0),pk(@0),pk(@1))' --context segwitv0
md: compile error: compile: Policy contains duplicate keys        # and(…), or(…), --context tap: same

$ cargo nextest run --locked --all-features -p md-cli --bin md
FAIL  compile::tests::upstream_display_is_still_broken_delete_local_renderer_when_this_fails
PASS  compile::tests::render_tr_template_pins_every_topology_class
      left: "tr(@4,{{pk(@3),pk(@2)},{pk(@1),pk(@0)}})"   # upstream is CORRECT now
$ cargo test --locked --workspace --doc
0 doctests in the workspace
$ git diff --stat ed2fe9c2 b8a64938 -- crates scripts .github
(empty — the two named baselines are code-identical)
```

---

## CRITICAL

### C1 — P2's R-N1a refusal breaks ≥5 existing tests, 3 shipped MANIFEST vectors and `md vectors`; the plan disposes of two unrelated tests and is silent on all of it

**Where.** Plan P2 step 2 (R-N1a refusal at `encode`, `descriptor --template`,
`address --template`) and P2 step 3 (the only test-disposition instruction in
the plan: flip `duplicate_key_slots.rs::one_key_at_two_different_use_sites_is_not_a_duplicate`
and `::t_row_one_key_at_two_disjoint_use_sites_still_composes` — both R-N1d,
neither R-N1a). Spec N1 Classification, row 1; Acceptance 2.

**Failure construction.** Spec N1 defines Family 1 as *one placeholder at more
than one use site*, keyed on the triple (inline origin path, multipath set,
wildcard hardening); triples identical ⇒ **R-N1a, mint/compose REFUSE**. That
predicate is true of the taproot internal-key-also-in-a-leaf shape and of every
template that names one placeholder twice at one path. Measured at `d635458a`,
all of the following are exit 0 today and must be exit 1 after P2:

| site | what it is | what P2 does to it |
| --- | --- | --- |
| `crates/md-codec/src/test_vectors.rs:164` `keyed_tr_sortedmulti_a` | `tr(@0/P,sortedmulti_a(2,@0/P,@1/Q))` | R-N1a |
| `crates/md-codec/src/test_vectors.rs:177` `keyed_tr_multi_a` | same shape, `multi_a` | R-N1a |
| `crates/md-codec/src/test_vectors.rs:161` `keyed_wsh_timelock_hashlock` | `@1` and `@2` each at two use sites, identical triples | R-N1a |
| `crates/md-cli/tests/template_roundtrip.rs:37-45` | loops `md_codec::test_vectors::MANIFEST` through `md encode` and unwraps | red on the three above |
| `crates/md-cli/tests/json_snapshots.rs:38-51` | same loop, same binary | red on the three above |
| `crates/md-cli/src/cmd/vectors.rs:14` | `md vectors` — the SHIPPED cross-language corpus generator, same MANIFEST | emits or refuses three vectors, depending on where the classifier is invoked |
| `crates/md-cli/tests/sortedmulti_a_taproot_leaf.rs:77` | 4× `md address --template` on the shape, `.expect("must derive")` | red |
| `crates/md-cli/tests/sortedmulti_a_taproot_leaf.rs:106` | asserts the refusal message contains `"valid only as a taproot leaf"` | a different refusal now fires; message assertion red |
| `crates/md-cli/tests/sortedmulti_a_taproot_leaf.rs:139` | `encode_and_derive_admit_the_same_shape`, `md encode` on the shape | red |
| `crates/md-cli/tests/cli_unhardened_origin_note.rs:134` | `assert_eq!(code, 0)` on `wsh(or_d(pk(@0/0/<0;1>/*),pk(@0/0/<0;1>/*)))` | red |
| `crates/md-cli/tests/cli_keyed_excess_origin_note.rs:169` | `assert_eq!(code, 0)`, same shape keyed | red |

The implementer arrives at P2's gate with a red suite and no instruction. The
two improvisations available are both bad, and the plan has ruled out neither:

- **Delete the offending vectors.** `keyed_tr_multi_a`'s own comment
  (`test_vectors.rs:167-174`) says it is *"the ONLY order-SENSITIVE tap leaf in
  the corpus… until this vector existed, 'preserve the WRITTEN key order in a
  tap leaf' was asserted by nothing. Found by mutation: reversing a leaf's key
  indices in the Go port passed the entire suite."* Deleting it silently
  removes mutation-proven coverage from a Rust-primary corpus the Go port is
  bound to (CLAUDE.md, Rust-primary rule; the plan's own Review protocol says
  "no Go port leads").
- **Narrow R-N1a to dodge them.** That is a spec violation of N1 row 1.

There is a third question the plan also owes, because the placement constraint
forces it: the classifier may not sit in `encode_payload`, so `md-codec` keeps
encoding these three shapes while `md encode` refuses them. Whether
`md vectors` (which calls `parse_template` directly, not `md encode`) refuses
or emits depends entirely on the invocation point the plan says it "chooses"
and then does not choose. Either answer is a normative fact about a shipped,
cross-language artifact and needs to be written down.

**Direction (one line).** Enumerate the R-N1a blast radius in P2 with a
per-site disposition, and rule explicitly on the three MANIFEST vectors and on
`md vectors`'s invocation point before P2 dispatches.

### C2 — P2 makes the committed seating-fixture generator abort mid-run and truncate a checked-in fixture; the plan points the implementer at that same script

**Where.** Plan P2 step 1: *"commit as fixtures with provenance headers in the
V-BOUND-REF pattern (`tests/fixtures/seating/generate.sh` documents it)"*.

**Failure construction.** `crates/md-cli/tests/fixtures/seating/generate.sh`
runs under `set -euo pipefail` (line 25) and regenerates every synthetic
seating fixture in order. Its V-R5M1 block calls `header "$f" …` — which
**truncates `v-r5m1.txt` with `>`** — and then pipes an R-N1a template through
`"$MD" encode`:

```
tr(@2/48'/0'/0'/2'/<0;1>/*,{sortedmulti_a(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*),
                            sortedmulti_a(1,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/0'/2'/<0;1>/*)})
```

`@0` and `@1` each appear at two use sites with identical triples. Measured
exit 0 today; exit 1 after P2. Under `set -e` the script then dies with
`v-r5m1.txt` holding five comment lines and no fixture, and the **nine
later fixtures never regenerate** (V-BOUND-SEAT, V-BOUND-REF, V-USP, V-MIX, the
depth-5 families, the B1 family, V-SPENDEQ, V-CE1). Nothing in CI runs the
generator, so this is silent — the script's own stated self-check is
*"`git diff` after a re-run is the check that the generator has not rotted"*,
and after P2 that check destroys a fixture instead of validating one. The
repo's V-BOUND-REF fixture is the very artifact P3 step 3 depends on.

**Direction (one line).** P2 must state what happens to the V-R5M1 block —
regenerate it from a pinned baseline binary, or move it to a committed-only
fixture with its own provenance header — and re-run the generator as part of
P2's gate.

---

## IMPORTANT

### I1 — the spec's `md compile` obligation is assigned to the plan and the plan never discharges it

**Where.** Spec N1 "Verb dispositions": *"**Obligation:** the plan determines
whether `md compile` (feature-gated) can EMIT a Family-1/2 shape; if it can, it
routes through the classifier with the refuse disposition."* The plan mentions
`md compile` only in P1, to delete a renderer from it.

**Failure construction.** The determination is never made or recorded, so
nothing in the cycle establishes whether `md compile` is a mint surface for a
refused shape. I ran it: `md compile 'thresh(2,pk(@0),pk(@0),pk(@1))' --context
segwitv0` → `md: compile error: compile: Policy contains duplicate keys`, and
the same for `and(pk(@0),pk(@0))`, `or(pk(@0),pk(@0))` and `--context tap`. So
the answer is **it cannot** — but that refusal is **rust-miniscript's**, not
md's, it is pinned by no local test, and P1 is precisely the phase that moves md
off a local workaround onto upstream behaviour. An upstream bump that relaxes
the compiler's duplicate-key check silently opens a mint path for R-N1a.

**Direction (one line).** Record the determination in the plan with the probe
above, and pin it with a row so an upstream bump cannot open the path silently.

### I2 — R9 is written for one verb; `md address` carries the identical defect and the four rows cannot see it

**Where.** Plan P4 step 2: *"`num_args = 1..` on `--from-mk1`"*, *"the
positional"*, four rows, no verb named. Spec R9 names no verb either.

**Failure construction.** `--from-mk1` is declared **twice** —
`crates/md-cli/src/main.rs:400` on `Descriptor` and `:560` on `Address` — both
`from_mk1: Vec<String>` with no `num_args`, both under a
`required(true).args(["phrases","template"])` input group (`:288`
`descriptor_input`, `:441` `address_input`). The FOLLOWUPS entry cites only
`main.rs:400`, so an implementer fixing `descriptor` alone closes the slug's
letter, passes all four of P4's rows, and leaves

```
md address <keyless md1 …> --from-mk1 <30 mk1 strings>
→ md: codec error: wire-format version mismatch: got 10, expected 4
```

which is the exact defect and the exact bare-codec-error class (F-420) the
entry was filed for, on the sibling verb of the same journey.

**Direction (one line).** Say "both `md descriptor` and `md address`" in P4
step 2 and duplicate at least the two guard rows on `address`.

### I3 — the plan claims to close `phase-gate-omits-cargo-doc` with a gate that is still narrower than CI, in the shape the entry warns about

**Where.** Plan gate section header, *"(every phase; closes
`phase-gate-omits-cargo-doc`)"*, and the burndown table row *"closed by this
plan's gate"*.

**Failure construction.** The entry's ask is explicit
(`design/FOLLOWUPS.md:2323-2325`): *"**A phase gate that is narrower than CI is
a gate that reports green for a tree CI will reject** — the next plan's gate
definition should name **every CI job**, or name the command that runs them
all."* The plan's gate names four commands. `.github/workflows/ci.yml` also
runs:

- `:49` `cargo test --workspace --doc` — and `cargo nextest run` **does not run
  doctests at all**, so substituting nextest for CI's `:48` drops `:49` by
  construction, not by oversight. (Verified mitigating fact: the workspace has
  **0** doctests today, so this hole is latent rather than live — but P6 adds
  `--help` text and a shared desugar core, prime doctest sites.)
- `:72` `conformance-vector checksum pin` — `cd design && sha256sum -c
  display-grouping-vectors.tsv.sha256`. **Live**, in-repo, and reachable by
  this cycle's admission changes.
- `:116` `freebsd-compile-gate`, `:156-196` the musl compile/test jobs.

So the closure claim is false against the entry's own criterion, and the
failure mode reproduces the incident: a phase closes green while a
required-adjacent CI job is red.

**Direction (one line).** Either name every CI job in the gate, or commit
`scripts/phase-gate.sh` that runs them and cite the script.

### I4 — P1's own gate omits the one check P1 exists to make pass

**Where.** Plan gate section: *"From P1 onward (P1 itself widens CI to
match)"*, then *"Until P1 lands, the same four lines without
`--all-features`"*, then *"Every phase's implementer runs the gate before its
final commit"*.

**Failure construction.** Read literally, P1 has not "landed" while P1's
implementer is working, so P1's implementer gates P1's final commit **without**
`--all-features`. P1's entire deliverable is an all-features-green suite; its
gate is therefore structurally incapable of observing whether the deliverable
was achieved. A P1 that deleted the tripwire but left, say, a clippy
`--all-features` warning in the orphan sweep (step 3) passes its own gate and
turns the newly widened CI red on the same commit — the never-skip-jobs
sequencing argument in step 2 protects the ordering but not the gate.

**Direction (one line).** State that P1 runs the **widened** four lines for its
own final commit, and that only pre-P1 re-validation uses the narrow form.

### I5 — R3's "admissible on every input mode" is normative, has no row, and no plan step; the obvious implementation violates it and still passes all four rows

**Where.** Spec Riders R3: *"`--verify-against` is admissible on every `md
descriptor` input mode that composes a descriptor."* Plan P6 step 1 says
*"output per spec"* and lists four rows, none of which varies the input mode.

**Failure construction.** `md descriptor` composes from three modes: a keyed-card
positional (measured exit 0 today), a keyless card + `--from-mk1`/
`--from-mk1-file`, and `--template` + `--key`. Every other value-carrying flag
on this verb is declared `requires = "template"` plus
`conflicts_with_all = ["phrases","from_mk1","from_mk1_file","seats"]`
(`main.rs:302-368`) — so the house pattern an implementer will copy makes
`--verify-against` **unusable on the two card modes**, which are exactly the
split-vs-keyed comparison the FOLLOWUP exists for
(`design/FOLLOWUPS.md:2209`, *"SPEC B2's split-vs-keyed comparison has no
operator-reachable command"*). All four rows still pass.

**Direction (one line).** Add a row exercising `--verify-against` on a
card-input composition, and say in P6 that the flag must NOT inherit the
T-row-flag conflict set.

### I6 — Acceptance 4's rendered-line obligation is mandated for two diagnostics and dropped for the other nine

**Where.** Spec Acceptance 4: *"Every diagnostic this cycle introduces or
rewrites HAS such a row [the RENDERED stderr line from the `md:` prefix onward]
— asserted by the vector rows, not by convention."* The plan mandates "full
rendered line" exactly twice: P2 step 2 (R-N1c) and P2 step 3 (R-N1d).

**Failure construction.** Every other new or rewritten diagnostic is described
in the plan as a *"named refusal"* or *"names `--from-mk1`"* — wording a
body-substring assertion satisfies:

- P3.1 card-input refusals (`descriptor`, `address`, two cards) — new
- P3.2 the read-side warning naming the BIP-388 violation — new
- P4.2 three R9 refusals (mk1-in-positional, flag-first trailing md1, no policy
  card) — new
- P5.2 two `--emit md1` input-mode refusals — new
- P6.1 the SPEND-EQUAL / NOT verdict lines and the garbage-argument decode
  error — new
- P6.3 the not-a-descriptor refusal, **rewritten** to name `--in` and `-`

That is nine diagnostics, each of which can ship with only a substring
assertion and still satisfy every word of the plan — the "no `invalid`" clause
of Acceptance 4 included, since a substring assertion cannot see the rest of
the line.

**Direction (one line).** State the Acceptance-4 obligation once in the gate
section as binding on every phase, rather than per-diagnostic in P2 only.

### I7 — P2's R-N1a fixture recipe does not produce the artifact it cites, and P3's rows depend on it

**Where.** Plan P2 step 1: *"mint the R-N1a card
(`wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))`, chunk-set-id `0xed813`
measured)"* — `--key` is named only for the delta card, and `--path` for
neither. P3 step 1 then refuses `descriptor` and `address` **on that card**.

**Failure construction.** Run exactly as written:

```
$ md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" --group-size 0
md1yqpqqxppcggqj6vw67aska9m          # ONE chunk, and NO chunk-set-id is printed at all
$ md descriptor md1yqpqqxppcggqj6vw67aska9m
md: descriptor requires wallet-policy mode (Pubkeys TLV): this card is a keyless TEMPLATE…
```

A keyless card composes nothing, so P3.1's `descriptor`/`address` card-input
refusal rows cannot be built from it, and the cited `0xed813` is unreachable.
The id is reproduced only by
`--key @0=<xpub> --path "m/48'/0'/0'/2'" --group-size 0` (measured today, 2
chunks). This matters permanently, not just locally: P2 states *"after this
phase no shipped binary can mint them — the fixtures are the read-side and
card-input rows' input forever."*

The delta card's numbers are **confirmed correct**: `--key @0=<K> --key @1=<K>
--path "m/48'/0'/0'/2'"` gives 4 chunks, chunk-set-id `0x00ee4`.

**Direction (one line).** Write both mint commands out in full in P2 step 1,
flags included, as the fixtures' provenance headers will have to anyway.

---

## MINOR

- **M1 — `--from-mk1-file` dropped from N2.** Spec N2 Input modes: *"admissible
  ONLY with `--from-mk1`/`--from-mk1-file` input"*. Plan P5.1 says *"on `md
  descriptor --from-mk1`"* and P5.2 enumerates two refusals; `--from-mk1-file`
  is never named and has no row. Mitigating: `collect_mk1` merges both into one
  list at `main.rs:891` before `cmd::descriptor::run`, so the natural
  implementation is correct by accident. `--from-mk1-file` is the spelling the
  FOLLOWUPS journey recommends for a 30-card set.
- **M2 — wrong path.** Plan P2.1 cites `tests/fixtures/seating/generate.sh`;
  the file is `crates/md-cli/tests/fixtures/seating/generate.sh`. The
  repo-root-relative form does not resolve.
- **M3 — P1 asserts the deletion is safe without running the repo's own
  documented disambiguation.** `compile.rs:329-333` requires checking whether
  `render_tr_template_pins_every_topology_class` **also** fired: both firing
  means an upstream *ordering* change, not PR #953, and deleting the renderer
  would then be wrong. The plan just says *"The tripwire IS the failing test"*.
  **I ran it: the tripwire FAILS, the sibling PASSES, and upstream now emits
  `tr(@4,{{pk(@3),pk(@2)},{pk(@1),pk(@0)}})` — genuine #953, so the plan's
  conclusion is correct.** Record that evidence in P1 so the next
  staleness re-validation does not have to re-derive it.
- **M4 — the `#checksum` strip is unnamed.** P1 step 1 says only *"render via
  upstream `Display`"*. `Descriptor::to_string()` appends a BIP-380 checksum;
  the deleted function's doc (`compile.rs:133-135`) says the `format!` build is
  the reason *"there is nothing to strip"*, and `compile_strips_descriptor_checksum`
  (`compile.rs:449`) covers only the keypath-only branch by its own measured
  scope note. `render_tr_template_pins_every_topology_class` does compare exact
  checksum-free strings, so a gate exists — but the instruction should say it.
- **M5 — the orphan sweep is scoped to one file.** P1 step 3 sweeps
  `compile.rs`; `crates/md-codec/tests/bitcoind_differential.rs:671` also names
  `md-cli/src/compile.rs: render_tr_template` in a comment that P1 falsifies.
- **M6 — P5 introduces a literal `md1` CLI token onto the verb whose P4 guard
  refuses `md1…`-prefixed strings by prefix.** `--emit md1`'s value is the
  string `md1`; an R9 guard implemented over too wide an argument set trips on
  it. One phase apart, same verb.
- **M7 — two baselines, and no procedure for "the BASELINE binary".** The plan
  header names `b8a64938`, the spec names `ed2fe9c2`. Verified benign:
  `git diff ed2fe9c2 b8a64938 -- crates scripts .github` is empty. Also
  verified: P1 touches only `compile.rs` (behind `cli-compiler`) and `ci.yml`,
  neither on the `md encode` path, so the post-P1 tree mints byte-identical
  cards — say that, or say how to build the baseline binary (worktree +
  `cargo build -p md-cli`).

## NIT

- **N1 — nine vs eleven.** P7 step 3 sweeps *"all nine owned entries (the eight
  originals + the walk-discovered R9 entry)"*, but the burndown table has 11
  rows; `phase-gate-omits-cargo-doc` and `sibling-toolkit-…` fall outside the
  nine while being claimed closed elsewhere in the plan, so neither gets a
  closure citing a commit.
- **N2 — the parked trigger is already recorded.** P7 step 1 appends a trigger
  to `md-decompose-has-no-json-output` that `design/FOLLOWUPS.md:2203-2206`
  already states in nearly those words.
- **N3 — a test named after a deleted function.**
  `render_tr_template_pins_every_topology_class` survives P1 (it calls
  `compile_policy_to_template`, and is the gate for M4) but its name refers to
  a function that no longer exists.

---

## Coverage table — spec item → phase → covered?

| spec item | phase | covered |
| --- | --- | --- |
| **N1 Classification** | | |
| R-N1a refuse (triples identical) | P2.2 | yes — but see **C1/C2** for its unenumerated blast radius |
| R-N1b refuse (overlapping multipath) | P2.2 | yes |
| R-N1c refusal stands, message rewritten, new variant | P2.2 | yes |
| R-N1-origin refuse, names origin axis, cites no BIP | P2.2 | yes |
| R-N1-hardening refuse if reachable / record unreachability | P2.2 | yes |
| R-N1d disjoint-use-site delta refuses | P2.3 | yes |
| the two pinning tests flip to refusal rows | P2.3 | yes |
| stale comments `build.rs:280-283`, `validate.rs:353-355` | P2.5 | yes (spec says "same commit"; plan does not restate) |
| codec floor `encode.rs:120` stays; S-row `check_no_repeated_xpub` stays | P2 preamble | yes |
| R-N1d message mandate (spelling's key vector, escape, no "invalid", no reuse of shipped wording) | P2.3 | yes |
| **N1 Placement constraint (no new check in `encode_payload`)** | P2 preamble | yes |
| **N1 Single-source** (one impl per predicate; disposition a parameter; identity + verify re-encode row-pinned) | P2 preamble / P3.2 | yes |
| **N1 Verb dispositions** | | |
| REFUSE: `encode`, `descriptor --template`, `address --template` | P2.2 | yes |
| REFUSE: `descriptor`/`address` on card input | P3.1 | yes (fixture blocked by **I7**) |
| WARN: `decode`, `inspect`, `bytecode`, `verify` incl. `verify --template` | P3.2 | yes |
| **Obligation: does `md compile` emit a Family-1/2 shape** | — | **NO — I1** |
| **N1 Vectors** | | |
| R-N1a at `encode` / `descriptor --template` / `address --template` | P2.2 | yes |
| card-input composing refusals on the R-N1a card | P3.1 | yes (**I7**) |
| R-N1b row | P2.2 | yes |
| R-N1c full rendered-line row | P2.2 | yes |
| R-N1-origin row | P2.2 | yes |
| R-N1-hardening row (conditional) | P2.2 | yes |
| R-N1d T-row refusal + full rendered-line row | P2.3 | yes |
| R-N1d card-input refusals (`descriptor`, `address`) | P3.1 | yes (**I7**) |
| R-N1d must-COMPOSE control (same fp, different accounts) | P2.4 | yes |
| V-BOUND-REF different-paths sibling row | P3.3 | yes |
| read-side rows on the R-N1a card (decode warns 0 / inspect 0 / verify 0 / bytecode) | P3.2 | yes |
| existing tests/vectors falsified by R-N1a | — | **NO — C1** |
| the seating fixture generator under R-N1a | — | **NO — C2** |
| **N2** | | |
| mint keyed card from seating result | P5.1 | yes |
| depth rule on `md encode --key` untouched | P5.1 | yes |
| minted card carries seating's origin metadata | P5.1 | yes |
| primary oracle (byte-identity vs `md encode` + inline origins + `--fingerprint`) | P5.3 | yes |
| secondary oracle (`spend_equal`, address-0 vs keyed fixture) | P5.3 | yes |
| `--template` + `--emit md1` refuses naming `md encode` | P5.2 | yes |
| keyed-card positional + `--emit md1` refuses as re-emit | P5.2 | yes |
| admissible with `--from-mk1-file` | — | thin — **M1** |
| composes with `--seat`; a seating refusal survives | P5.3 | yes |
| matrix S→K cell, 4 homes, one commit, identity script | P5.4 | yes |
| **N3** | | |
| precedence inline > `--path` > bracket; bracket last-resort only | P4.1 | yes |
| different-accounts wallet composes | P4.1 | yes |
| equals the inline-origins composition | P4.1 | yes |
| disagreeing bracket with `--path` still refuses | P4.1 | yes |
| slot with no path from any source still refuses | P4.1 | yes |
| **Riders** | | |
| R3 flag, `spend_equal` wiring, `#[allow(dead_code)]` + comment deleted | P6.1 | yes |
| R3 exit codes 0 / 5 / 1-2 | P6.1 | yes |
| R3 four rows | P6.1 | yes |
| R3 admissible on every composing input mode | — | **NO — I5** |
| R3 output names failing half + states origin exclusion and why | P6.1 ("output per spec") | thin |
| R5 (a) delete renderer + tripwire, (b) widen CI, in order | P1.1-1.2 | yes (**M3/M4**) |
| R6 one desugar core, `--help` names `/**`, equivalence row | P6.2 | yes |
| R7 `-` on decompose; refusal names `--in` and `-` | P6.3 | yes |
| R8 parked, trigger recorded | P7.1 | yes (**N2**) |
| R9 `num_args = 1..` + two symmetric guards + four rows | P4.2 | partial — **I2** (`md address`) |
| Docs — toolkit `42-md.md` + `tests/lint.sh flag-coverage` | P7.2 | yes |
| **Gates and process** | | |
| R0 loop, plan R0, one implementer/phase, whole-diff review | gate section + P7.4 | yes |
| phase gate quoted verbatim, `--all-features` from R5(b) | gate section | partial — **I3**, **I4** |
| Rust-primary, vectors before any Go port | Review protocol | yes on paper; strained by **C1** |
| push via `scripts/push-via-staging.sh`, `main` frozen | P7.5 | yes (script exists) |
| **Acceptance** | | |
| 1. every named row executable, in the same commit as its impl | implied by TDD ordering | thin |
| 2. suite green under the full gate incl. `--all-features` | gate section | **unachievable as written — C1** |
| 3. S→K matrix flipped in 4 homes, identity check green | P5.4 | yes |
| 4. rendered stderr line from `md:` onward for EVERY new/rewritten diagnostic; no "invalid" | P2.2/P2.3 only | **partial — I6** |
| 5. reading verbs exit 0 on newly-refused engraved cards | P3.2 | yes |
| **FOLLOWUPS closures** | | |
| `all-features-suite-is-red-and-ungated-by-ci` (a)+(b) | P1 | yes — plan exceeds the entry (clippy + doc too) |
| `md-repeated-placeholder-inverts-bip388` (both directions, with vectors) | P2+P3 | yes |
| `descriptor-key-bracket-path-as-a-last-resort-source` | P4 | yes |
| `from-mk1-arity-spills-card-strings-into-the-md1-positional` | P4 | partial — **I2** |
| `md-cannot-mint-a-keyed-card-from-a-split-set` | P5 | yes |
| `md-verify-against-flag-for-cross-form-comparison` | P6 | partial — **I5** |
| `md-decompose-rejects-double-wildcard-input` | P6 | yes |
| `md-decompose-does-not-read-stdin` | P6 | yes |
| `md-decompose-has-no-json-output` (parked + trigger) | P7 | yes (**N2**) |
| `sibling-toolkit-md-manual-lockstep-for-the-converter` | P7 | yes — surface list matches the entry's |
| `phase-gate-omits-cargo-doc` ("name every CI job") | gate section | **NO — I3** |

---

## Verdict

**COUNTS: 2C / 7I / 7M / 3N**

Not GREEN. The two Criticals share one root cause the plan never sized: R-N1a
is a *much* wider predicate than the plan's two-test disposition assumes, and
it reaches shipped cross-language vectors, a user-facing corpus generator and a
committed fixture generator that destroys its own output when it fails. Both
need a per-site ruling written into P2 before an implementer is dispatched, not
discovered at P2's gate.
