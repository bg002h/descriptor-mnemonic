# REVIEW — IMPLEMENTATION_PLAN_mdcli_mini.md, R0 round 2 (scoped fold re-review)

- **Artifact:** `design/IMPLEMENTATION_PLAN_mdcli_mini.md` (status DRAFT p1)
- **Commit reviewed:** `bb14b47c` (main tip)
- **Fold under review:** `git diff 6449a80b..bb14b47c -- design/IMPLEMENTATION_PLAN_mdcli_mini.md`
  (167 insertions, 36 deletions)
- **Checklist:** `design/agent-reports/REVIEW-mdcli-mini-plan-r1.md` (2C/7I/7M/3N)
- **Date:** 2026-08-31
- **Reviewer:** independent, opus
- **Scope:** SCOPED fold re-review, two halves — (1) does the fold fix each of
  the 19 r1 findings; (2) did the fold's NEW text introduce a defect (spec
  contradiction, unexecutable step, a disposition that breaks something r1 did
  not cover, or a claim the tree contradicts). NOT a fresh audit.
- **Taken as settled, not re-derived:** `design/SPEC_mdcli_mini.md` (GREEN,
  four rounds); the operator rulings; r1's own command log and measurements;
  the controller's spot-checks (corpus shapes, the V-R5M1 block, `main.rs:560`,
  `ci.yml` jobs 49/73/95).

## What this reviewer RAN (all at `bb14b47c`)

```
$ ( cd design && sha256sum -c display-grouping-vectors.tsv.sha256 )
display-grouping-vectors.tsv: OK                          exit 0   # the fold's new gate line

$ cargo nextest run --locked --all-features -p md-cli \
    --test vector_corpus --test conformance_vectors_roundtrip \
    --test template_roundtrip --test json_snapshots
12 tests run: 12 passed, 0 skipped                                 # all four green TODAY

$ cargo nextest run --locked --all-features -p md-cli -E 'test(v_r5m1)'
3 tests run: 3 passed
  seat::satisfy::tests::v_r5m1_repeated_placeholder_refuses_as_bip388_forbidden
  seating_vectors::v_r5m1_reaches_the_command
  seat::satisfy::tests::v_r5m1_control_a_reuse_free_policy_passes_the_same_check

$ ./target/debug/md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" \
    --key @0=<KEY 1> --path "48'/0'/0'/2'" --group-size 0
chunk-set-id: 0xed813    2 chunks on stdout   exit 0    # the fold's spelling, VERBATIM
$ ...same with --path "m/48'/0'/0'/2'"      -> 0xed813, identical                # both work
$ ./target/debug/md encode "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" \
    --key @0=<KEY 1> --key @1=<KEY 1> --path "48'/0'/0'/2'" --group-size 0
chunk-set-id: 0x00ee4    4 chunks on stdout   exit 0                             # confirmed

$ cargo fmt --check -v | sort > A ; cargo fmt --all --check -v | sort > B ; diff A B
(empty — 78 files each)   # the gate's `cargo fmt --check` == CI's `cargo fmt --all --check`

$ grep 'force_chunked' for the three replaced vectors
keyed_wsh_timelock_hashlock: true   keyed_tr_sortedmulti_a: true   keyed_tr_multi_a: true

$ grep -c 'f="$HERE/|b1_fixture "$HERE/' crates/md-cli/tests/fixtures/seating/generate.sh
22 write-sites, 17 of them AFTER the V-R5M1 block (line 160)
```

---

## Fix verification — 19 rows

| r1 finding | verdict | evidence |
| --- | --- | --- |
| **C1** R-N1a blast radius unenumerated | **PARTIAL** | P2 step 6 enumerates per-site dispositions, replaces (not deletes) the 3 MANIFEST vectors and RULES `md vectors`' invocation point — all three asks discharged. But the enumeration is incomplete and two of its entries are no-ops: see **I-1**. |
| **C2** generator truncates `v-r5m1.txt`, kills later fixtures | **FIXED** | P2 step 7: regenerate once from baseline, convert the block to an existence-assert with a frozen-by-design note, re-run the generator in P2's gate. Verified executable: V-R5M1 is the ONLY `"$MD" encode` in the script with a repeated placeholder (all 11 templates extracted and checked); nothing else regenerates or diffs the fixture (`include_str!` only, at `seating_vectors.rs:650` and `seat/satisfy.rs:364`); no test runs `generate.sh`; the sibling `mk` binary the script needs exists. Freezing it does NOT break the other fixtures' git-diff self-check — `header()` is what truncates, and an existence-assert does not call it. (Count nit: **M-b**.) |
| **I1** `md compile` obligation never discharged | **FIXED** | P2 step 8 records the probe verbatim and adds a pinning row so an upstream bump cannot silently open the path. |
| **I2** R9 written for one verb | **FIXED** | P4 step 2 names both `main.rs:400` (`Descriptor`) and `:560` (`Address`), `num_args = 1..` on both, both guards on both, and duplicates at minimum the two guard rows on `address`. |
| **I3** gate narrower than CI | **FIXED** (by the entry's own criterion) | Gate adds `cargo test --workspace --doc` and the `ci.yml:73` checksum pin, commits `scripts/phase-gate.sh` in P1, and states a blind spot. All 6 ci.yml jobs are now named by the script or the blind spot. The checksum-pin line RUNS as written (exit 0). Residual: the blind-spot statement under-declares by two matrix legs — **M-a**. |
| **I4** P1's gate omits P1's own deliverable | **FIXED** | "P1's own final commit runs the WIDENED lines … the narrow form exists only for re-validating work that predates P1." |
| **I5** R3 admissible on every input mode | **FIXED** | P6.1 states the flag must NOT inherit `requires = "template"` + `conflicts_with_all`, declares it admissible on all three composing modes, and adds the keyed-card-POSITIONAL mode row plus a `--from-mk1`-spelled equal row. |
| **I6** Acceptance-4 mandated for 2 of 11 | **FIXED** | Stated once in the gate section as binding on EVERY phase, with the universal quantifier intact ("every diagnostic any phase introduces or rewrites"). The illustrative list itself miscounts — **M-c**. |
| **I7** mint recipe does not produce the cited artifact | **FIXED** | P2 step 1 now carries both commands in full. RAN VERBATIM: `0xed813`/2 chunks and `0x00ee4`/4 chunks both reproduce. The fold's `--path "48'/0'/0'/2'"` (no `m/`) works identically to r1's `m/`-prefixed spelling. |
| **M1** `--from-mk1-file` dropped | **FIXED** | P5.1 names both spellings, cites `collect_mk1` at `main.rs:891` (verified), and requires one row in the `--from-mk1-file` spelling. |
| **M2** wrong path | **FIXED** | `crates/md-cli/tests/fixtures/seating/generate.sh` — full path, resolves. |
| **M3** disambiguation not run | **FIXED** | P1.1 records the measured result (tripwire fails ALONE, sibling passes ⇒ genuine #953) and says "do not re-derive". Citation `compile.rs:329-333` verified. |
| **M4** `#checksum` strip unnamed | **FIXED** | P1.1 names it and names `render_tr_template_pins_every_topology_class` as the gate. Verified: `compile.rs:445-455`'s own comment says the tree path is covered elsewhere. |
| **M5** orphan sweep scoped to one file | **FIXED** | P1.3 adds `crates/md-codec/tests/bitcoind_differential.rs:671`; verified the comment names `md-cli/src/compile.rs: render_tr_template` at that line. (Side effect: **M-d**.) |
| **M6** `--emit md1` literal vs the R9 prefix guard | **FIXED** | P4.2 guard-scope note: the md1-prefix guard applies to `--from-mk1` values and the positional ONLY. This also covers P6's `--verify-against <md1>` value by construction. |
| **M7** two baselines, no build procedure | **FIXED** | P2.1: `git worktree add <dir> b8a64938 && cargo build -p md-cli`, plus the benign-difference argument. |
| **N1** nine vs eleven | **FIXED** | P7.3 sweeps ALL ELEVEN burndown rows, naming the two extras and what each closure cites. |
| **N2** trigger already recorded | **FIXED** | P7.1 → verify-and-add-one-line, do not duplicate. |
| **N3** test named after a deleted function | **FIXED** | P1.1 instructs the rename. |

**18 FIXED / 1 PARTIAL / 0 NOT FIXED.**

---

## NEW FINDINGS

### IMPORTANT

#### I-1 — P2 step 6 says "the implementer improvises none of them", and its enumeration misses the only two tests that actually exercise the three replaced vectors, plus the 15 committed corpus files they diff against

**Where.** Plan P2 step 6, second bullet: *"`template_roundtrip.rs:37-45` and
`json_snapshots.rs:38-51` turn green via the replacements (they drive
MANIFEST)."* And the step's opening claim: *"enumerated; the implementer
improvises none of them."*

**Failure construction, in two parts.**

*(a) The two named sites are no-ops.* All three replaced vectors carry
`force_chunked: true` (measured above). Both named loops begin with

```rust
for v in manifest::MANIFEST {
    if v.force_chunked { continue; }        // template_roundtrip.rs:38-40
                                            // json_snapshots.rs:48-50
```

so neither test has ever encoded `keyed_tr_sortedmulti_a`,
`keyed_tr_multi_a` or `keyed_wsh_timelock_hashlock`. They were not red before
the replacement and do not "turn green via" it. A third loop,
`template_roundtrip.rs:207-211` (`reencode_round_trip_each_manifest_entry`),
skips them for the same reason. Confirming this is what led to (b).

*(b) The sites that ARE affected are absent from the plan entirely.*

| site | mechanism | state after P2 as written |
| --- | --- | --- |
| `crates/md-cli/tests/vector_corpus.rs:15` `vectors_output_matches_committed_corpus` | runs `md vectors --out <tmp>`, then `diff -r <tmp> crates/md-codec/tests/vectors` | RED — "vectors corpus drift detected" |
| `crates/md-cli/tests/conformance_vectors_roundtrip.rs:36` | runs `md vectors --out <tmp>`, asserts success, cross-derives every record through `md address` | RED at `generate()` while any MANIFEST vector is still a refused shape |
| 15 committed files: `{keyed_tr_sortedmulti_a, keyed_tr_multi_a, keyed_wsh_timelock_hashlock}.{template, bytes.hex, phrase.txt, descriptor.json, conformance.json}` | written by `cmd/vectors.rs:63-95`, pinned by the diff above | still hold the OLD templates/bytes/cards |

Both tests are green today (measured). The plan gives no regeneration step
(`md vectors --out crates/md-codec/tests/vectors`), does not name either test,
and does not name the 15 files. This is C1's exact shape one iteration in: the
implementer reaches P2's gate with a red suite and no instruction — under a
step that has just told them not to improvise.

It also matters beyond the gate. `crates/md-codec/tests/vectors/` is the
cross-language artifact: `corpus_origin_consistency.rs:11-14` says these are
*"files, vendored into the Go port and compared byte for byte"*. Regenerating
15 of them is a normative-artifact change, which is the thing the plan's own
Rust-primary lockstep sentence is supposed to be flagging — but that sentence
sits under a bullet about MANIFEST source, not about the committed corpus, and
names neither the command nor the file count.

Checked and clear, so the disposition is bounded: `design/display-grouping-vectors.tsv`
is hand-written render/strip rows with zero MANIFEST content (`grep -c multi_a`
= 0), so the checksum pin is NOT disturbed; the insta snapshots under
`crates/md-cli/tests/snapshots/` contain no `keyed_*` entries (force_chunked is
skipped), so none needs regenerating; `wire_golden.rs` pins
`wsh_sortedmulti_2chunk`, not these three; the BIP mediawiki's test-vector
table names none of them (`grep -n 'keyed_' bip/bip-mnemonic-descriptor.mediawiki`
→ no matches); `corpus_origin_consistency.rs` reads the committed conformance
JSONs and stays green provided the replacement does not bind one origin to two
xpubs.

**Direction (one line).** Add to P2 step 6: the two tests by name, the
regeneration command, the 15-file count, and the constraint that the
replacement must not put two different xpubs at one `[fingerprint/path]`
(`corpus_origin_consistency.rs`) — then the "improvises none of them" claim is
true.

*Role-preservation, checked as briefed and CLEAR:* `keyed_tr_multi_a`'s stated
role (`test_vectors.rs:167-174` — the only order-sensitive tap leaf,
mutation-proven, "reversing a leaf's key indices in the Go port passed the
entire suite") survives either available replacement. The leaf is
`multi_a(2,@0/P,@1/Q)`; the R-N1a violation is internal-key↔leaf, not
within-leaf. Whether the implementer changes the internal key to a fresh `@2`
or the leaf's `@0` to a fresh `@2`, the leaf retains two distinct keys in
written order, so a reversal still changes the emitted script and still fails
to round-trip to the same address. The mutation the vector was built to catch
is still caught. (Wording nit: **N-a**.)

#### I-2 — the Family-1 predicate is ALREADY implemented and shipped on `md descriptor`'s card path, with its message pinned by two tests; the plan names every other shipped check and is silent on this one

**Where.** Plan P2 preamble: *"NOTHING new inside `encode_payload`'s validator
set; the codec floor (`validate_no_duplicate_key_slots` at `encode.rs:120`) and
the S-row `check_no_repeated_xpub` stay as shipped."* And P3 step 1, which adds
a NEW card-input refusal on `md descriptor`/`md address`.

**Failure construction.** `crates/md-cli/src/seat/satisfy.rs:188`:

```rust
pub fn check_no_repeated_placeholder(policy: &Descriptor) -> Result<(), CliError> {
    let mut counts = vec![0u32; policy.n as usize];
    count_occurrences(&policy.tree, &mut counts);
    ...
    "this policy uses the same placeholder at more than one position — {} … \
     which is forbidden by BIP 388 … That shape is UNSUPPORTED here …"
```

It is called at `crates/md-cli/src/seat/mod.rs:134`, as the FIRST door check on
`md descriptor`'s keyless-card + `--from-mk1` seating path. That is a shipped
implementation of spec N1's Family-1 predicate, reached from a card input,
already refusing, already carrying "forbidden by BIP 388" + "UNSUPPORTED" +
no "invalid" + per-placeholder arity. Its wording is pinned by
`crates/md-cli/tests/seating_vectors.rs:679-687` (asserts the rendered stderr
contains "same placeholder at more than one position") and by
`crates/md-cli/src/seat/satisfy.rs:530-548`. All three pass today (measured).

The plan enumerates the shipped checks that "stay as shipped" and this one is
not among them — `check_no_repeated_xpub` (`satisfy.rs:294`, same-xpub-two-cards)
and `validate_no_duplicate_key_slots` (the codec floor, wrapped by
`build.rs:300 refuse_key_reuse_across_slots` on the TEMPLATE path) are different
predicates. So P3.1's new card-input refusal lands with no ruling, and both
available outcomes are defects the plan has not chosen between:

- **Add a second implementation** → satisfies every word of P2/P3 and violates
  spec N1's *Single-source* constraint ("each predicate has ONE implementation;
  per-verb disposition is a parameter"). One predicate, two messages, two
  code paths, silently.
- **Route the card path through the new classifier** → if the check lands
  before `seat/mod.rs:134` (which is where a "card-input refusal" naturally
  goes, at the top of `cmd::descriptor::run`), `seating_vectors.rs:679`'s
  message assertion goes red, and so does `satisfy.rs:530` if the function is
  removed. Neither is named anywhere in the plan.

Note also that `check_no_repeated_placeholder` is *coarser* than spec N1: it
counts occurrences per placeholder index without testing the (inline origin
path, multipath set, wildcard hardening) triple, so on the seating path it
already refuses **R-N1d** (disjoint use sites) with the **Family-1** wording —
while the spec mandates a distinct R-N1d message that attributes the violation
to the spelling's key vector. Unifying is therefore not optional if the spec's
message mandate is to hold on the card path.

This is adjacent to P2 step 7 but not covered by it: step 7 rules the
generator's fate and stops there, leaving the frozen fixture's two consumers —
which assert on the shape the classifier is about to reclassify — unexamined.

**Direction (one line).** Name `check_no_repeated_placeholder`
(`seat/satisfy.rs:188`, called at `seat/mod.rs:134`) in the P2 preamble
alongside the other shipped checks, and rule in P3 whether the card-input
refusal replaces it or sits before it — with the disposition for
`seating_vectors.rs:679-687` and `satisfy.rs:530-548` written down.

### MINOR

- **M-a — the gate's stated blind spot under-declares by two CI contexts.**
  The plan says the script's blind spot is *"the freebsd and musl compile/test
  jobs (ci.yml:95+)"*. Read end to end, `ci.yml` has six jobs, and the `test`
  job (`:31-49`) is a matrix over `[ubuntu-latest, windows-latest,
  macos-latest]` — three check contexts, of which a local gate run reproduces
  one. By the FOLLOWUP's literal criterion ("name every CI job") all six jobs
  are named, so I3 closes; but a reader is told the only locally-unreachable
  jobs are freebsd and musl, and that is not so. Mitigating: `ci.yml:17` names
  only `cargo test (ubuntu-latest)` and `cargo clippy` as required contexts, so
  a red windows leg does not block the push ritual — it does still turn CI red,
  which is the incident the entry was filed for. Add the two legs to the
  blind-spot sentence. (Checked and clear: the gate's `cargo fmt --check` and
  CI's `cargo fmt --all --check` cover an identical 78-file set here — the
  virtual manifest makes `--all` redundant — and `cargo nextest run` at the
  workspace root covers both members, there being no `default-members`.)

- **M-b — "nine later fixtures" is seventeen.** P2 step 7 makes the count a
  gate criterion: *"`git diff` clean over the nine later fixtures is the
  check."* Measured: 17 fixture files are written after the V-R5M1 block
  (`v-bound-seat`, `v-bound-ref`, `v-usp`, `v-mix`, `v-r2-ord`, `v-r4-ik`,
  `v-grp`, `v-cap`, `v-leftover`, `v-unfilled`, the four `v-b1-*`,
  `v-spendeq-keyed`, `v-ce1`, `v-ce1-foreign`). "Nine" is r1's grouping of
  families restated as a file count. Say "every fixture after it" and the
  criterion is right regardless.

- **M-c — the eleven-diagnostic list does not reconcile with the plan's own
  phases, and drops two.** The plan's enumeration reads: R-N1a/b/c/origin
  (/hardening) and R-N1d (P2) = 5–6; two card-input refusals + the read-side
  warning (P3) = 3; three R9 refusals (P4) = 3; two `--emit md1` refusals (P5)
  = 2; SPEND-EQUAL/NOT verdict lines + the rewritten decompose refusal (P6) =
  2 → **15–16**, not eleven. Two diagnostics are missing from it outright:
  (i) P6.1's *"garbage argument (1, decode error, no verdict)"*, which r1
  explicitly named as new; (ii) the `md address` R9 refusals that the I2 fold
  itself added in P4.2 — if `address`'s rendered lines differ from
  `descriptor`'s, they are diagnostics with no place in the list. Harmless to
  the gate, because the binding sentence above the list is universally
  quantified and catches both; but a later reviewer ticking off "all eleven"
  will come up short. Drop the number or fix it.

- **M-d — "P1 touches only `compile.rs` (feature-gated) and `ci.yml`" is
  falsified by the fold's own P1.3.** P2.1's baseline argument rests on that
  claim; P1.3, added by this same fold, also edits
  `crates/md-codec/tests/bitcoind_differential.rs:671`. The conclusion (a
  post-P1 tree mints byte-identical cards) survives — a test-file comment and a
  test rename touch no encode path — but the premise as written is now false in
  its own document. Say "nothing on the `md encode` path" instead of listing
  files.

### NIT

- **N-a — "get DISTINCT leaf placeholders" is already true of both tr
  vectors.** `keyed_tr_multi_a`'s leaf is `multi_a(2,@0/P,@1/Q)` and
  `keyed_tr_sortedmulti_a`'s is `sortedmulti_a(2,@0/P,@1/Q)` — the leaf
  placeholders are distinct today. What repeats is the INTERNAL KEY against a
  leaf key. Read literally the instruction is a no-op. (Role preservation is
  unaffected either way — see I-1's closing paragraph.) Say "give the internal
  key a placeholder that appears nowhere in the leaf".

- **N-b — the gate is now a strict superset of the spec's "quoted verbatim"
  gate.** `SPEC_mdcli_mini.md:370-376` describes a four-line gate "quoted
  verbatim by the plan"; the plan now runs six lines plus a script. This is an
  extension demanded by the spec's own cited FOLLOWUP, not a contradiction, and
  the spec is GREEN and out of scope here — but the spec's next touch should
  stop calling the plan's gate a verbatim quote.

---

## Verdict

**COUNTS (new): 0C / 2I / 4M / 2N; r1 findings: 18/19 FIXED** (C1 PARTIAL, 0
NOT FIXED).

The loop does **not** close. Both new Importants are the same shape as C1 and
sit in the same place: P2's blast-radius enumeration is the plan's strongest
new section and it is still not complete. I-1 is its residual — the two tests
it names never touch the vectors in question, and the two that do
(`vector_corpus.rs`, `conformance_vectors_roundtrip.rs`), together with the 15
committed cross-language corpus files, are absent. I-2 is the same omission on
the other side of the predicate: `check_no_repeated_placeholder` already ships
Family-1 on `md descriptor`'s card path with its wording pinned by two passing
tests, and the plan's list of shipped checks that "stay as shipped" does not
include it, so P3.1 lands unruled against either a spec single-source violation
or two red message assertions.

Everything else the fold did holds up under execution: the mint commands
reproduce both cited chunk-set-ids verbatim, the checksum-pin gate line runs
clean, the generator disposition is sound and V-R5M1 is genuinely the only
abort site in the script, the M3/M4/M5 citations resolve, and
`keyed_tr_multi_a`'s mutation-proven order-sensitivity role survives the
replacement under either reading.
