# REVIEW — IMPLEMENTATION_PLAN_mdcli_mini.md, R0 round 3 (scoped fold re-review)

- **Artifact:** `design/IMPLEMENTATION_PLAN_mdcli_mini.md` (status DRAFT p2),
  plus the one consistency edit the same commit made to
  `design/SPEC_mdcli_mini.md:371-374`
- **Commit reviewed:** `3323d030` (main tip)
- **Fold under review:** `git diff 062237e4..3323d030` (81 changed lines in the
  plan, 6 in the spec)
- **Checklist:** `design/agent-reports/REVIEW-mdcli-mini-plan-r2.md` (0C/2I/4M/2N,
  plus r1's C1 held PARTIAL)
- **Date:** 2026-08-30
- **Reviewer:** independent, opus
- **Scope:** SCOPED fold re-review, two halves — (1) does the fold fix each of
  the 8 r2 findings and thereby COMPLETE r1's C1; (2) did the fold's NEW text
  introduce a defect (spec contradiction, internal contradiction, unexecutable
  step, or a claim the tree contradicts). NOT a fresh audit.
- **Taken as settled, not re-derived:** everything r1 and r2 verified (both ran
  full command logs); the spec's four GREEN rounds; the operator rulings.

## What this reviewer RAN (all at `3323d030`)

```
$ ./target/debug/md vectors --out $T ; ls $T | grep -cE '^(keyed_tr_sortedmulti_a|keyed_tr_multi_a|keyed_wsh_timelock_hashlock)\.'
exit 0                                                    15      # I-1's arithmetic, exactly
$ diff -r --exclude=manifest.rs --exclude=.gitkeep \
    --exclude=bip341-wallet-test-vectors.json $T crates/md-codec/tests/vectors
(empty)   # in-place regeneration is a no-op today ⇒ the cited command is the right one

$ ls -1 crates/md-codec/tests/vectors | grep -cE '^(keyed_tr_sortedmulti_a|keyed_tr_multi_a|keyed_wsh_timelock_hashlock)\.'
15                                                        # committed count matches

$ ./target/debug/md encode "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@0/48'/0'/0'/2'/<2;3>/*))" --group-size 0
md: template parse error: @0 appears with inconsistent path/multipath/hardening
exit 1     # an R-N1c-shaped POLICY CARD cannot be minted, so it cannot reach the door check

$ grep -rn 'keyed_tr_sortedmulti_a|keyed_tr_multi_a|keyed_wsh_timelock_hashlock' \
    (tree, minus target/vendor/.git/design)
crates/md-codec/src/test_vectors.rs:161,164,177  +  the 15 corpus files only
                                          # no unnamed consumer ⇒ C1's enumeration is exhaustive

$ grep -rn 'check_no_repeated_placeholder' crates/
src/seat/mod.rs:134 (sole production caller)  src/seat/satisfy.rs:188,532,552,553
```

Read and resolved: `crates/md-cli/src/main.rs:261-264`, `src/cmd/vectors.rs:16-102`,
`src/cmd/build.rs:66,69-77`, `src/cmd/descriptor.rs:44-77`, `src/seat/mod.rs:100-141`,
`src/seat/satisfy.rs:129-211`, `crates/md-codec/src/encode.rs:17-28`,
`crates/md-cli/tests/vector_corpus.rs:15`,
`crates/md-cli/tests/conformance_vectors_roundtrip.rs:23-38`,
`crates/md-cli/tests/corpus_origin_consistency.rs:1-72`,
`crates/md-cli/tests/fixtures/seating/v-r5m1.txt`, `.github/workflows/ci.yml:31-49`.

---

## Fix verification — 8 rows + C1

| r2 finding | verdict | evidence |
| --- | --- | --- |
| **I-1** blast-radius enumeration missed the two tests that actually bind, the regeneration command and the 15 corpus files | **FIXED** | Plan:200-218. Both binding tests named with resolving citations (`vector_corpus.rs:15`, `conformance_vectors_roundtrip.rs:36` — both verified). The regeneration command is the tool's real invocation form: `main.rs:262-263` declares `#[arg(long, value_name = "DIR")] out: Option<String>`, and `cmd/vectors.rs:25` `create_dir_all`s the target and writes into it — it never clears, which is REQUIRED here because `manifest.rs` and the BIP-341 fixture live in that directory. RAN: regenerating into a temp dir writes exactly 15 files for the three names and `diff -r` against the committed corpus is empty, so the command is correct and is a no-op until P2 changes MANIFEST. The false "turn green via the replacements" claim is replaced by the accurate `force_chunked`-skip explanation. The Rust-primary lockstep sentence is moved onto the corpus bullet, where r2 said it belonged. The `corpus_origin_consistency` constraint is stated and its two citations check out: lines 11-14 do carry *"vendored into the Go port and compared byte for byte"*, and the test does read `.conformance.json` (`corpus_origin_consistency.rs:68`). |
| **I-2** `check_no_repeated_placeholder` unnamed and P3.1 unruled | **FIXED** (ruling) | Plan:121-133 names it with call site, declares it NOT in the stays-as-shipped set, rules P3 UNIFIES it through the shared classifier with per-verb disposition, and writes down the disposition for both pinning sites (`seating_vectors.rs:679-687`, `satisfy.rs:530-548` update in the same commit). All three of r2's asks are discharged. **The ruling is right; its stated justification is false — new finding I-1 below.** |
| **M-a** blind spot under-declares by two CI contexts | **FIXED** | Plan:36-40 adds *"AND the windows/macos legs of the test matrix (ci.yml:31-49 runs three OS contexts; a local run reproduces one)"*. Verified: `ci.yml:31` `test:`, `:37` `os: [ubuntu-latest, windows-latest, macos-latest]`, job body ends at `:49`. |
| **M-b** "nine later fixtures" is seventeen | **FIXED** | Plan:242-243 makes the criterion *"`git diff` clean over EVERY fixture written after that block"*, count-free, with the correction stated inline. (Residual "nine" six lines above: **N-1**.) |
| **M-c** eleven-diagnostic list miscounts and drops two | **FIXED** | Plan:56-65 retires the number (*"The binding rule is the sentence above — EVERY introduced or rewritten diagnostic, no fixed count"*), demotes the list to *"for orientation only"*, and adds both items r2 named as dropped: the garbage-argument decode error, and the R9 refusals on BOTH verbs. |
| **M-d** "P1 touches only `compile.rs` (feature-gated) and `ci.yml`" falsified by P1.3 | **FIXED** | Plan:154-156 now reads *"P1 touches nothing on the `md encode` path … its edits are a feature-gated compile module, CI config, and test-file text"*. Verified against P1's three steps: `crates/md-cli/src/compile.rs` (behind `cli-compiler`, `Cargo.toml:26`), `.github/workflows/ci.yml`, and test text (`bitcoind_differential.rs:671` comment + one test rename). None is on `md encode`'s path (`cmd/build.rs` → `parse_template`), so the premise is now true as written and the conclusion still holds. |
| **N-a** "get DISTINCT leaf placeholders" is a no-op | **FIXED** | Plan:190-192 adopts r2's wording verbatim: *"give the INTERNAL KEY a placeholder that appears nowhere in the leaf"*. Verified against `test_vectors.rs:164,177`: both leaves are `…(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*)` — distinct today; what repeats is the `tr` internal key `@0`. |
| **N-b** spec calls the plan's gate a verbatim quote | **FIXED** | `SPEC_mdcli_mini.md:371-374` now reads *"quoted and EXTENDED by the plan … the plan adds the doctest and conformance-checksum lines"*. Verified: the spec lists 4 lines (nextest, clippy, fmt, doc); the plan's block is those 4 plus `cargo test --workspace --doc` and the `sha256sum -c` pin, and says *"all six lines"*. Consistent, and the added-line description is accurate. Spec stays GREEN — consistency edit only. |
| **r1 C1** R-N1a blast radius unenumerated (r2: PARTIAL, solely for I-1's gap) | **COMPLETE** | I-1 fixed closes the only reason r2 withheld it. Independently swept: grepping the whole tree (minus `target/`, `vendor/`, `.git/`, `design/`) for the three vector names returns only `crates/md-codec/src/test_vectors.rs:161,164,177` — the MANIFEST source, already ruled by P2 step 6 — and the 15 committed corpus files themselves. No test file or script references them by name. The incidental fourth site (`cli_keyed_excess_origin_note.rs:169`) is separately enumerated at plan:229-231. The enumeration is exhaustive. |

**8 FIXED / 0 PARTIAL / 0 NOT FIXED. r1 C1: COMPLETE.**

---

## NEW FINDINGS

### IMPORTANT

#### I-1 — the fold's justification for the I-2 unification ruling is false in every part: it asserts a case that cannot arise at the cited call site, and points P3 at the wrong spec message mandate

**Where.** Plan:124-127, the fold's newest paragraph:

> already refuses Family-1 shapes on `md descriptor`'s card input — but
> COARSER than spec N1: it counts occurrences only, so it refuses
> R-N1d-disjoint spellings with Family-1 wording, which the spec's distinct
> R-N1d message mandate forbids.

**Failure construction, three independent parts.**

*(a) R-N1d is Family 2, and a per-index count cannot see it.* `SPEC_mdcli_mini.md:88`
defines *"**Family 2 — one key at more than one placeholder (R-N1d …)**"*, and
`:100-101` fixes R-N1d proper as *"identical key material (public key + chain
code) at two **placeholders** whose use sites differ."* `count_occurrences`
(`seat/satisfy.rs:129+`) increments `counts[index]` per placeholder INDEX; two
different placeholders holding the same key each count 1, so
`check_no_repeated_placeholder` cannot fire on Family 2 under any input. Its
entire domain is Family 1.

*(b) R-N1d is undetectable at that call site in principle, not just in this
implementation.* `seat/mod.rs:122-130` refuses any `policy.is_wallet_policy()`
card BEFORE the door check, so the policy reaching `seat/mod.rs:134` is KEYLESS
and carries no key material at all. `SPEC:167-173` states Family 2 needs the
classifier's input (ii), the resolved per-`@i` key bindings — which do not exist
until `matching::decide` assigns cards, 17 lines later in the same function.

*(c) The check is not "coarser" on its reachable input either.* `Descriptor`
(`md-codec/src/encode.rs:17-28`) holds one `use_site_path` for the whole
descriptor plus a per-`@N` `path_decl`, so the wire cannot express one
placeholder at two different triples; the mint side refuses it too. RAN:

```
$ md encode "wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@0/48'/0'/0'/2'/<2;3>/*))" --group-size 0
md: template parse error: @0 appears with inconsistent path/multipath/hardening   exit 1
```

So a repeated placeholder that reaches the door check ALWAYS has identical
triples — spec row `:77`, **R-N1a** — for which the shipped Family-1 wording is
exactly correct. The plan's own V-R5M1 fixture confirms the shape: its
provenance header mints `tr(@2/48'/0'/0'/2'/<0;1>/*,{sortedmulti_a(2,@0/48'/0'/0'/2'/<0;1>/*,@1/…),sortedmulti_a(1,@0/48'/0'/0'/2'/<0;1>/*,@1/…)})`
— identical triples throughout. The R-N1b/R-N1c/origin/hardening rows, which
DO differ in triple, are unreachable on this path.

**Why it is Important rather than cosmetic.** The ruling directly above it says
*"its wording becomes the taxonomy's messages"*, and the paragraph tells the P3
implementer that R-N1d arises here and that `SPEC:115-129`'s mandate binds — a
mandate that requires attributing the pairwise-distinct violation to the
spelling's key vector and naming `me sysw pack --as descriptor --in <your export
file>`. Implementing that at this site yields at best a dead branch with an
Acceptance-4 row pinning a rendered line no input can produce. The worse route
is live: an implementer who notices the predicate needs key material it does not
have may move the door check past seating to obtain the bindings — and
`seat/mod.rs:100-106`'s own doc comment declares that ordering normative
(*"Each of those refusals is accurate only where it sits: deferred past A3,
V-IMPOSS would surface as a leftover-card message about the wrong thing"*). The
plan's newest sentence therefore supplies a concrete path to a refusal-ordering
regression the code explicitly warns against.

Second-order: this is the ONLY ground the plan gives for the unification. A
reviewer or implementer who checks it — one command — finds it false, and the
plan offers no fallback, so a correct and necessary ruling is left resting on
nothing.

**Direction (one line).** Replace the coarseness clause with the ground that
actually holds — `SPEC:176-177` (*"each predicate has ONE implementation (no
per-verb second copy)"*) plus `SPEC:191-192` and `:184-185` (`descriptor`'s card
input is in the REFUSE mint/compose surface, which is what the single-source
rule binds), so P3.1's new card-input Family-1 refusal would be a second
implementation of the predicate already at `satisfy.rs:188` — and drop the
R-N1d reference. If the coarseness point is wanted at all, it is
"identical-triple only, i.e. exactly R-N1a; the wire cannot carry the other
Family-1 rows to this call site."

*Recorded for the diff:* r2's own I-2 contained the same mislabel and the fold
transcribed it faithfully. The plan is the artifact under review, so the finding
is filed against the plan, not as a fold error of omission.

### MINOR

- **M-1 — the P3-binding ruling lives only in P2's preamble, and no P3 gate
  fails if it is skipped.** The ruling sits at plan:121-133, inside
  `### P2 — N1 mint/compose refusals (template path)`. P3 (plan:252-270) has
  four numbered steps and none of them mentions `check_no_repeated_placeholder`,
  the unification, or the two pinning sites. If the implementer omits it,
  nothing turns red: r2 measured all three pinning tests green against the
  shipped wording, and P3's gate is the same six-line script. The Acceptance-4
  obligation binds a *rewritten* diagnostic — it cannot force the rewrite to
  happen. r2's direction ("rule in P3") is satisfied in substance, so this is
  placement, not omission. Add a fifth P3 step, or a one-line pointer from P3
  to plan:121-133, so the deliverable sits where its implementer reads it.

### NIT

- **N-1 — the retired "nine" survives six lines above its own correction.**
  Plan:236-237 still states as fact *"the script would die AFTER truncating
  `v-r5m1.txt`, leaving nine later fixtures unregenerated"*, while plan:243
  says *"that is 17 files, not r1's nine"*. Measured: 17. Harmless — the
  correction is in the same step and the gate criterion is now count-free — but
  the two sentences disagree inside one paragraph. Change 237 to "every later
  fixture".

- **N-2 — two bare file citations in the fold's new text.**
  `corpus_origin_consistency.rs:11-14` (plan:211) and
  `corpus_origin_consistency.rs` (plan:216) omit the directory; the file is
  `crates/md-cli/tests/corpus_origin_consistency.rs`. Both citations are
  substantively CORRECT (verified above) and the name is unique in the tree, so
  this is inconsistency rather than ambiguity — the same bullet gives full paths
  for `crates/md-cli/tests/vector_corpus.rs:15` and
  `crates/md-cli/tests/conformance_vectors_roundtrip.rs:36`, and r1 M2
  established the full-path convention.

---

## Verdict

**COUNTS (new): 0C / 1I / 1M / 2N; r2 findings: 8/8 FIXED; r1 C1: COMPLETE.**

The loop does **not** close, on one Important.

The fold discharged every r2 finding, and r1's C1 is now genuinely complete —
an independent tree-wide sweep for the three replaced vectors turns up no
consumer the plan does not name, the regeneration command is the tool's real
invocation form, and the 15-file arithmetic is exact (measured by running it).
M-a, M-b, M-c, M-d, N-a and N-b are all fixed against citations that resolve,
and the spec's gate paragraph now describes the plan's gate accurately.

What blocks is inside the fold's newest paragraph. The I-2 ruling — unify the
seating door check through the shared classifier — is correct and needed, but
the justification the fold wrote for it is false in all three of its claims:
R-N1d is Family 2 and a per-placeholder count cannot detect it; the policy at
`seat/mod.rs:134` is keyless by construction so Family 2 is undetectable there
in principle; and the wire cannot carry a placeholder at two different triples,
so the check's reachable domain is exactly R-N1a, where its shipped wording is
right. The consequence is not cosmetic: the paragraph points P3 at
`SPEC:115-129`'s R-N1d mandate, and an implementer chasing the key bindings that
mandate needs would be pulled toward moving a refusal that `seat/mod.rs:100-106`
declares order-normative. The fix is a substitution, not new work — the
single-source ground at `SPEC:176-177` + `:184-185` + `:191-192` supports the
same ruling and is true.
