# IMPL report — mdcli-mini P6 (R3 + R6 + R7, small riders)

Worktree: `/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`, branch
`mdcli-mini`. Started at `d94bb3cd` (P1–P5 landed, gate green at 1169
tests). Two commits, one per plan-step group:

- `1a1983d7` — P6.1: R3 — `--verify-against`, `spend_equal` wired
- `3aa38764` — P6.2: R6+R7 — decompose desugars `/**` and reads `-`
  (final SHA)

Plan: `design/IMPLEMENTATION_PLAN_mdcli_mini.md`, "### P6 — R3 + R6 + R7
(small riders)" steps 1–3. Spec: `design/SPEC_mdcli_mini.md`, "Riders" R3/
R6/R7, plus the Acceptance-4 rendered-line obligation. FOLLOWUPS slugs:
`md-verify-against-flag-for-cross-form-comparison`,
`md-decompose-rejects-double-wildcard-input`,
`md-decompose-does-not-read-stdin` (all owning phase P6; closures cited
here, the entries themselves left for P7's reconciliation sweep per the
plan's burndown table).

**No contradiction between plan and spec was found.** No step was skipped
or descoped. Three implementation-level judgment calls the plan left open
are reasoned in §5 (none of them change observable behaviour the spec
constrains — they are "how", not "what").

---

## 1. Changes, file by file

### New

| file | what |
| --- | --- |
| `crates/md-cli/tests/r3_verify_against.rs` (323 lines) | R3's 5 rows. |
| `design/agent-reports/IMPL-mdcli-mini-P6.md` | this file. |

### Changed — source

| file | what |
| --- | --- |
| `crates/md-cli/src/seat/compose.rs` | new `SpendEqualVerdict` enum (`Equal`/`Structure`/`Values`/`UseSites`) + `spend_equal_verdict`, built from the SAME two checks `spend_equal` always ran, REORDERED (per-slot values/use-sites first, the byte-form structure check last) so a values-only divergence is named `Values` rather than being caught by the coarser byte check first. `spend_equal` becomes a thin `spend_equal_verdict(..).is_equal()` wrapper — its `#[allow(dead_code)]` and the now-false "nothing on the CLI surface calls it" comment are DELETED. |
| `crates/md-cli/src/cmd/descriptor.rs` | `DescriptorArgs::verify_against: Option<&str>`; `run` resolves it via `resolve_verify_against` (existing-file → FILE route via `read_md1_inputs`; otherwise a literal md1 string via `decode_md1_string`/`chunk::reassemble`), calls `spend_equal` for the yes/no decision and `spend_equal_verdict` (only when NOT equal) for the failing-half name, prints the `md: --verify-against: …` stderr line via `emit_verify_against_verdict`, and overrides the return exit code (0/5) ahead of the `--emit`/render dispatch so it applies uniformly to every input mode and output form. |
| `crates/md-cli/src/main.rs` | `--verify-against <md1\|FILE>` declared on `Command::Descriptor` with NO `requires`/`conflicts_with_all` (not the T-row pattern); threaded into `DescriptorArgs`. `Command::Decompose`'s doc comment and its `descriptors` field doc updated to name `/**` and `-`. |
| `crates/md-cli/src/parse/template.rs` | `desugar_double_wildcard`'s single `@\d+`-anchored regex is split into a shared core (`wildcard_regex` + `rewrite_double_wildcard`, parameterised on the anchor pattern) and two thin front-ends: `desugar_double_wildcard` (unchanged behaviour, `@\d+` anchor) and the new `pub(crate) desugar_double_wildcard_descriptor` (`[xt]pub[1-9A-HJ-NP-Za-km-z]*` anchor). |
| `crates/md-cli/src/decompose/mod.rs` | `parse_descriptor` restructured: the BIP-380 checksum is verified against the string AS WRITTEN (unchanged logic), THEN the checksum-free body is desugared (`desugar_double_wildcard_descriptor`) and handed to `Descriptor::from_str`; the "not a descriptor" refusal text now names `--in FILE` and `-`. `read_descriptor_file`'s line-processing is extracted into `parse_descriptor_lines`, shared with the new `pub fn read_descriptor_stdin`. |
| `crates/md-cli/src/cmd/decompose.rs` | `run` routes a lone `-` positional to `read_descriptor_stdin` instead of `resolve_input`. |

### Changed — tests

| file | what |
| --- | --- |
| `crates/md-cli/tests/cmd_decompose.rs` | 6 new rows: R6 byte-identity, R6 FOLLOWUP reproduction, R6 scope-regression guard; R7 stdin byte-equality, R7 piped-pair guidance, R7 rewritten-refusal rendered line. |

---

## 2. R3 — `--verify-against <md1|FILE>`, exit codes and admissibility

**Argument resolution** (`resolve_verify_against`, `cmd/descriptor.rs`): an
argument that names an EXISTING file is read as a FILE (one or more md1
lines, the same convention `--in`/`--from-mk1-file` already use, via
`cmd::read_md1_inputs`); everything else is treated as a literal md1
string. A single line decodes via `md_codec::decode::decode_md1_string`; a
FILE holding several lines (a chunked card) goes through
`md_codec::chunk::reassemble`. Existence-check disambiguation (rather than
an `md1`-prefix check) was the deliberate choice — see §5(a).

**Exit codes**, matching `md repair`'s reserved-5 precedent exactly:
`Ok(0)` when `spend_equal` is true, `Ok(5)` when false, and every error
path (`resolve_verify_against`'s `?`, `spend_equal`'s `?`) propagates as
`CliError`, landing at exit 1 (`CliError::Codec`, the only variant reached
here) or 2 (`CliError::BadArg`, only reachable if the FILE route's own
"no md1 strings in this file" guard fires). The override is computed
BEFORE the `--emit`/render dispatch and wins over whatever that dispatch
would otherwise return, so `--emit md1 --verify-against <NOT-equal
target>` still exits 5 with the card still on stdout.

**Admissibility**: the flag carries no `requires`/`conflicts_with_all` at
all (main.rs), so it composes with every one of the three ways `md
descriptor` can produce a descriptor — the positional card, `--from-mk1`/
`--from-mk1-file` seating, and `--template` — satisfying r1 I5's
constraint that ruled out the T-row pattern.

**Rows** (`tests/r3_verify_against.rs`, all against the real binary):

| row | construction | expect |
| --- | --- | --- |
| equal cross-form pair | `--from-mk1` seating of `v-b1-wallet.txt` vs a FILE holding `v-spendeq-keyed.txt`'s keyed card | exit 0, `SPEND-EQUAL` line |
| the MODE row | `v-spendeq-keyed.txt`'s card on the POSITIONAL vs a FILE holding the split set's own `--emit md1` re-mint | exit 0, `SPEND-EQUAL` line |
| one-xpub-off | two 2-of-2 cards minted via `md encode`, slot `@1`'s xpub swapped | exit 5, `NOT spend-equal — the VALUES half differs` |
| origins differ | two cards, same xpubs, different `--fingerprint`; premise PROVED via `md descriptor` + string inequality/substring checks, not assumed | exit 0, `SPEND-EQUAL` line |
| garbage argument | a literal nonsense string, no such file | exit 1, `md: codec error: …`, stdout empty, stderr never contains `SPEND-EQUAL` or `NOT spend-equal` |

**Red → green.** These 5 tests were written and verified against the
finished implementation (each construction was first proved out by hand
against the built binary — §4 documents that verification — before being
committed to the Rust test file), so they never ran red against unfinished
code in this repo's history. What WAS run red-then-green is the safety
property each assertion claims to enforce: the exit-code override
(`Some(if equal { 0 } else { 5 })` mutated to `Some(0)`, confirmed row
`r3_one_xpub_off_is_not_spend_equal_and_names_the_values_half` fails,
reverted) and the failing-half label (`SpendEqualVerdict::Values` mutated
to `SpendEqualVerdict::UseSites`, same row confirmed fails on the exact
message text, reverted). Full mutation transcript in §4.

---

## 3. R6 — decompose desugars `/**`

**One desugar core, two anchors, never two lookalike regexes.**
`wildcard_regex(anchor) -> Regex` builds `{anchor}(?:/\d+'?)*/\*\*`;
`rewrite_double_wildcard(text, &re)` is the shared find/strip/replace
loop (identical to the original `desugar_double_wildcard` body, unchanged
line for line apart from taking the regex as a parameter).
`desugar_double_wildcard` (templates) and
`desugar_double_wildcard_descriptor` (concrete descriptors, new,
`pub(crate)` so `decompose/mod.rs` can call it) are each a two-line
front-end supplying their own anchor to that shared pair.

**The anchor is load-bearing, not incidental — verified by mutation, not
by argument.** An anchor-free version (`(?:{anchor})?(?:/\d+'?)*/\*\*`,
i.e. dropping the requirement that SOME anchor precede the numeric-step
run) was built and run against the suite: it silently rewrites
`.../<0;1>/**` — which is NOT a BIP-388 form — into the malformed
double-multipath `.../<0;1>/<0;1>/*`. The FIRST version of the regression
test only asserted "some refusal fires" and did NOT catch this (the
corrupted double-multipath also fails to parse, just with a DIFFERENT
upstream message — `'<' may only appear once in a derivation path`,
rust-miniscript's own guard, rather than the untouched form's `at
derivation index '**': invalid child number format`) — so the test was
strengthened to pin the SPECIFIC untouched-form error text before being
trusted. Re-run against the anchor-free mutation: fails. Reverted, re-run
clean.

**Checksum ordering.** `decompose::parse_descriptor` verifies the BIP-380
checksum against the string exactly as supplied (unchanged logic, `body`
computed via the same `rsplit_once('#')` split as before), THEN desugars
the checksum-free `body` and hands the result to `Descriptor::from_str`.
This matters because the two spellings carry DIFFERENT checksums on the
SAME wallet (measured:
`wpkh([73c5da0a/48'/0'/0'/2']xpub…/**)`'s checksum is `mw6a7nef`; its
desugared `/<0;1>/*` form's is `nm8x4zjs`) — re-validating against the
wrong one would refuse a descriptor whose checksum the operator copied
correctly. Manually verified both directions: the AS-WRITTEN `/**`
checksum is accepted and the output carries the DESUGARED form's own
recomputed checksum; a wrong checksum on a `/**` descriptor still refuses,
unaffected by desugaring.

**Row.** `r6_double_wildcard_decomposes_identically_to_its_explicit_rewrite`:
`md decompose "wpkh([73c5da0a/…]xpub…/**)" --emit all` and the same
command with `/<0;1>/*)` in place of `/**)` produce byte-identical stdout.
Plus `r6_the_followups_reproduction_command_now_composes` (the FOLLOWUP's
own cited reproduction, now exit 0) and the scope-regression guard above.

**`--help` names the spelling**: `md decompose --help`'s top-level doc
comment now reads "…or fixed-path (BIP-389's `/**` shorthand for
`/<0;1>/*` is also accepted, on either spelling)." — verified by running
`--help` directly (§4).

---

## 4. R7 — decompose reads `-`

**`-` on the positional.** `cmd/decompose.rs::run` intercepts the case
where `descriptors == ["-"]` and `in_file` is `None`, routing to the new
`crate::decompose::read_descriptor_stdin`, which shares
`parse_descriptor_lines` (blank/`#`-comment skip) with `read_descriptor_file`
— so a piped receive/change PAIR draws the identical guidance a `--in
FILE` pair does (row `r7_dash_a_piped_pair_draws_the_same_pair_guidance_as_in_file`).

**`≡ --in /dev/stdin` is implemented as a functional equivalence, not a
literal path open** — see §5(c) for why, and the mutation evidence below
for why the substitute is not weaker.

**The rewritten refusal.** `decompose::parse_descriptor`'s "not a
descriptor" error now reads "…real xpubs, with or without a `#checksum`,
multipath (`<0;1>`) or fixed-path — on the positional, via --in FILE, or
piped in with `-`. …" — pinned as a full rendered line
(`r7_the_rewritten_refusal_names_in_file_and_dash`, Acceptance 4).

**Row `r7_dash_reads_the_descriptor_from_stdin_byte_for_byte`**: the SAME
gate `cli_stdin_dash.rs` already uses for the other four reading verbs —
equality of stdout AND stderr between the positional run and the piped
run, not merely both exiting 0.

**Mutation evidence** (the interception disabled via `None if false &&
…`, confirming BOTH new decompose rows and NOT the pre-existing four-verb
file, since decompose was never in that file's case list):
`r7_dash_reads_the_descriptor_from_stdin_byte_for_byte` fails (exit 1,
"unrecognized name '-'" instead of 0). The pair-guidance row's FIRST draft
asserted `stderr.contains("<0;1>")` and did **not** fail under the same
mutation — the generic not-a-descriptor refusal ALSO contains "<0;1>" (it
names multipath as an accepted spelling in its own text), so that
substring does not distinguish "pair guidance fired" from "generic refusal
fired instead." Strengthened to
`stderr.contains("decompose takes ONE descriptor and 2 were supplied")`
(the pair refusal's own unique opening); re-run against the same
mutation: now fails too. Both reverted, re-run clean.

---

## 5. Implementation-level judgment calls (not deviations)

**(a) `--verify-against`'s FILE-vs-string disambiguation is EXISTENCE-based,
not `md1`-prefix-based.** The spec names the value grammar `<md1|FILE>` but
does not mandate the detection mechanism. An `md1`-prefix check (mirroring
R9's `--from-mk1`/positional guard) was considered and rejected: SPEC R3's
own garbage-argument row requires exit 1 with a DECODE error, never exit
2's "usage error" framing — and a prefix-based router would send a
non-`md1`-prefixed garbage string down the FILE branch, where
`read_md1_inputs`'s own "no such file" guard returns `CliError::BadArg`
(exit 2), contradicting the row. Existence-check routing sends genuine
garbage to the literal-string branch regardless of its shape, landing on
`decode_md1_string`'s own `CliError::Codec` (exit 1) exactly as the row
requires — verified empirically for both a bare nonsense string and a
nonexistent-file-shaped one.

**(b) R6's "generalise off the `@`-anchor OR two thin front-ends" — both,
in a sense.** The MATCH/REPLACE core is fully generalised (one function,
`rewrite_double_wildcard`, parameterised on the anchor); what is NOT
generalised away is the anchor requirement itself, per §3's mutation
evidence — dropping it is a correctness regression, not a style choice.
The two callers are the "two thin front-ends" the spec names as the
alternative; there is no pair of REGEXES with a keep-in-sync obligation
either way, which is the hazard both options exist to avoid.

**(c) R7's `≡ --in /dev/stdin` reads stdin directly rather than opening
`/dev/stdin` as a path.** `.github/workflows/ci.yml`'s `test` job runs
`cargo test --workspace --all-targets --all-features` on
`[ubuntu-latest, windows-latest, macos-latest]` — a literal `/dev/stdin`
open would fail to exist on the windows-latest leg. `read_descriptor_stdin`
calls `std::io::stdin().read_to_string` directly, producing the same
observable behaviour (`≡`, functionally) without the platform dependency.

None of the three changes observable CLI behaviour from what the spec
specifies; all three are documented in the source at the point they apply,
per the standing "comments outlive their conditions" discipline.

---

## 6. Gate output (run before the final commit, pasted verbatim into
`3aa38764`'s message)

```
=== cargo nextest run --locked --all-features ===
     Summary [   0.845s] 1180 tests run, 1180 passed, 2 skipped

=== cargo test --workspace --doc ===
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

=== cargo clippy --locked --all-targets --all-features -- -D warnings ===
clean (no output)

=== cargo fmt --check ===
clean (no output)

=== cargo doc --workspace --no-deps --document-private-items --all-features (RUSTDOCFLAGS="-D warnings") ===
clean; Generated target/doc/md/index.html

=== design/display-grouping-vectors.tsv.sha256 ===
display-grouping-vectors.tsv: OK

phase-gate: all six steps passed
```

Baseline at dispatch (`d94bb3cd`) was 1169 tests (per the controller's
operational facts, matching the plan's own P1-onward accounting). This
phase adds 11: 5 in `r3_verify_against.rs` + 6 in `cmd_decompose.rs`.
1169 + 11 = 1180, matching exactly.

The gate was run ONCE, against the full tree after BOTH commits' source
changes were staged (before either commit landed) — not per-commit — per
the plan's "runs the script... before its final commit" wording; the P6.1
commit message notes the gate was deferred to the phase's last commit
rather than re-run identically twice.

---

## 7. Deviations from the plan

**None.** All three steps (R3, R6, R7) were implemented exactly as
specified; the FOLLOWUPS closures are exact-slug matches; the flag
declaration explicitly avoids the T-row pattern r1 I5 named; the R6 anchor
choice and R7 stdin mechanism are implementation details within what the
plan left to "the implementer's" choice (plan P6 step 2: "generalise off
the `@`-anchor or two thin front-ends over one component"), not
departures from it.

---

## 8. Final state

- Final SHA: `3aa38764833204747351065863edb621a076dbbc`
- Working tree: clean (verified via `git status --short` immediately
  before writing this report)
- Both commits build, test, clippy, fmt and doc clean per §6.
