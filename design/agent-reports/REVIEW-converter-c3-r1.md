# REVIEW — converter C3 `md decompose`, round 1 (mechanical)

Scope: `git diff c940529c..cd69794d`, against `design/SPEC_wallet_form_converter.md`
"P3 — the concrete descriptor becomes an entrance" + "Canonicalisation" + A3's
key-reuse rules, and `design/IMPLEMENTATION_PLAN_wallet_form_converter.md` §3 C3
/ §4 V-D-* rows. SETTLED per the dispatch brief and NOT re-run: the four exit
gates (1038/1038 nextest, clippy -D warnings, fmt, 30 v_d_* rows across 8/8
families), the fixture generator's determinism sha, the suite-growth
arithmetic. This round's budget went only to the six questions below.

**Result: 0 Critical, 0 Important, 2 Minor.**

One terminology note before Q1: the brief's "eight bullets in SPEC P3" is the
plan's **row** count (§4 lists 8 V-D-* families, confirmed at
`design/IMPLEMENTATION_PLAN_wallet_form_converter.md:202-203,273-280`); SPEC
P3's own "Details forced by measurement" list is 5 bullets (round-trip-grade
emission, origin-less keys, repeated-keys wording, new-walker, input
boundary). Not a code finding — mapped the 5 SPEC bullets to rows below.

## Q1 — ROWS vs SPEC P3

**(a) V-D-RT "keys AS PARSED — never re-serialised depth-0."** PASS.
`cmd_decompose_roundtrip.rs:321-353`,
`v_d_rt_key_lines_are_as_parsed_never_a_depth_zero_reserialisation`, is the
assertion that fails on a depth-0 emission: it parses each emitted key line,
asserts `xkey.depth == 4`, and separately asserts `usize::from(xkey.depth) ==
comps.len()` and `xkey.child_number == *comps.last().unwrap()` against the
origin path — a depth-0 re-serialisation would fail both. It also asserts the
emitted lines equal `pinned_key_records()` (read straight from
`tests/fixtures/pathological/keys.txt`, not derived from decompose), so a
byte-changed re-serialisation fails the string comparison too, independent of
the depth check. Redundant coverage: `cmd_decompose.rs:91-94`
(`decompose_emits_template_keys_and_fingerprints`) and
`cmd_decompose_roundtrip.rs:302-319`
(`v_d_rt_emissions_still_match_what_mk_consumed`) both also assert the exact
K0/K1/K2 strings appear verbatim.

**(b) Never-say-"invalid," applied to both v_d_reuse and v_d_shape2's
overlapping case.** PASS, with a Minor. `assert_bip388_wording`
(`cmd_decompose.rs:54-68`) is called at lines 424 (`v_d_reuse_same_xpub…`),
454 (`v_d_shape2_same_placeholder_with_non_disjoint…`), and 473
(`v_d_shape2_partially_overlapping…`) — the reuse row and *both* non-disjoint
SHAPE2 rows. `v_d_shape2_disjoint_sets_refuse_naming_mds_narrower_template_surface`
(the BIP-legal case) correctly does **not** call it — that refusal is
deliberately not "forbidden by BIP 388" wording, since there is no BIP
violation; it asserts its own three substrings instead
(`cmd_decompose.rs:488-500`).

Grepped `invalid` case-insensitively across `crates/md-cli/src/decompose/` and
`crates/md-cli/src/cmd/decompose.rs`: two hits in emitted diagnostic text,
both in `mod.rs`'s `check_no_repeated_key` — line 244 ("…UNSUPPORTED here,
never invalid.") and line 264-265 ("…among its invalid examples. Forbidden by
BIP 388 — UNSUPPORTED here, never invalid."). Read both in full: the first is
a negation ("never invalid"); the second quotes BIP 388's own phrase for its
listed example (`sh(multi(1,@0/**,@0/**))`) and immediately disambiguates in
the next clause. Neither contains `"is invalid"` or `"invalid descriptor"` as
a substring (checked by hand against `assert_bip388_wording`'s exact
lowercase-substring test) — confirmed no false pass on that clause.

**Minor:** `assert_bip388_wording`'s first check —
`stderr.contains("forbidden by BIP 388") || stderr.contains("BIP 388")` — is
case-sensitive. Rule (1)'s message (line 243-244) has lower-case
"…forbidden by BIP 388…" mid-sentence and matches the specific first
disjunct. Rule (2)'s message (line 264-265, used by both SHAPE2 non-disjoint
rows) has sentence-initial "**F**orbidden by BIP 388", which does **not**
match the lower-case literal — the assertion only passes via the generic
`"BIP 388"` fallback, which would also pass on any message merely mentioning
the phrase without the word "forbidden" at all. Not a false pass in practice
(both required substrings — BIP-388 citation and `UNSUPPORTED`/`unsupported`
— are genuinely present, and `UNSUPPORTED` is checked as an independent,
case-covering assertion right after), but the helper's specific branch is
effectively dead code for two of its three call sites.

**(c) V-D-JSON/V-D-PAIR name the guidance and are not reachable as the bare
checksum error.** PASS, structurally, not just by assertion. `resolve_input`
(`mod.rs:112-123`) runs the JSON test over every raw entry **first** and
returns before the length-dispatch that would otherwise route to
`pair_refusal`; `decompose()` (`mod.rs:382-384`) calls `resolve_input` and
then `parse_descriptor` — the function that does the `#checksum` comparison
— strictly *after*. For `raw.len() > 1` (PAIR) or any JSON-shaped entry, the
function returns from `resolve_input` and `parse_descriptor` is never
called, so the checksum path is unreachable by construction, not merely
untested. The four tests' `!stderr.contains("checksum")` assertions
(`cmd_decompose.rs:236-239,267-269`) are then belt-and-suspenders, and the
guidance content itself is checked specifically: `"listdescriptors"`,
`"\"desc\""` (JSON); `"<0;1>"`, `"ONE descriptor"`/`"one descriptor"` (PAIR).

## Q2 — CIRCULARITY IN V-D-RT

**Not circular.** Dependency chain, read directly:

- `facts()` (`cmd_decompose_roundtrip.rs:190-238`) parses a raw descriptor
  string with `Descriptor::<DescriptorPublicKey>::from_str` and
  `miniscript::ForEachKey::for_each_key` directly. The file's imports
  (`cmd_decompose_roundtrip.rs:41-43`) are `miniscript::ForEachKey`,
  `miniscript::descriptor::{Descriptor, DescriptorPublicKey}`,
  `std::str::FromStr` — no `crate::decompose`/`md_cli::decompose` import
  anywhere in the file. `facts()` never calls `walk::collect_occurrences`,
  `walk::order_by_appearance`, or anything else in `src/decompose`; it
  re-implements its own ordering (`keys.sort_by_key(|k|
  rendered.find(&k.to_string())…)`), value extraction (chain code ‖
  compressed point) and origin extraction from scratch.
- `assert_round_trip_equal(&one("input-descriptor"), &recomposed, route)`
  (`:255-281`) computes `facts(input)` and `facts(recomposed)`. The
  **expected** side, `input`, is the literal committed fixture string
  (`one("input-descriptor")`) — it never touches decompose at all.
- The **actual** side, `recomposed`, is `md descriptor <minted cards>`'s
  output. The minted cards come from `mint_keyed_card`/`mint_policy_card`
  (`:126-170`), which run `md encode` over `live_emissions()`
  (`:111-124`) — decompose's own `--emit template`/`--emit keys` output,
  taken live, not from the fixture. So `recomposed` legitimately depends on
  decompose (the code under test), `md encode` (mint) and `md descriptor`
  (reconstruction) — that dependency is the point of a round-trip test, not
  a defect.
- The **relation** (`spend_equal`, and the origin-equality half) and its
  inputs (`facts`) share no code with `src/decompose`. That is what makes
  the report's claim — "computed from rust-miniscript directly —
  independent of `src/decompose`" — accurate: the oracle's *implementation*
  is independent, even though its *input* (the recomposed side) is
  necessarily downstream of the code under test.

Residual, not a defect: `facts()`'s ordering step is conceptually the same
algorithm as `walk::order_by_appearance` (sort by first appearance in the
canonical rendering), independently re-implemented rather than shared. A
shared conceptual bug in that ordering (not a shared-code bug) could in
principle affect both sides identically. That specific class is covered
elsewhere by a test with a hand-written expected string, not derived from
either implementation: `walk::tests::taproot_internal_key_is_slot_zero`
(`walk.rs:315-320`) asserts `t ==
"tr(@0/48'/0'/0'/2'/<0;1>/*,pk(@1/48'/0'/1'/2'/<0;1>/*))"` literally.

## Q3 — THE CI-FACING mk EVIDENCE (deviation 1)

PASS. `v_d_rt_emissions_still_match_what_mk_consumed`
(`cmd_decompose_roundtrip.rs:302-319`) asserts byte-for-byte with `assert_eq!`
against `one("template")`, `section("keys")`, `one("canonical-descriptor")` —
all read from the committed `tests/fixtures/decompose/v-d-rt.txt`, not
regenerated. Read the fixture directly: it contains the exact mk command
`mk encode --keys keys.txt --from-md1-set policy.md1` (line 36) and
`Measured exit codes: route 1 (md encode --key) = 0; route 2 (md encode
--out, then mk encode --keys --from-md1-set) = 0` (lines 14-15) —
`v_d_rt_mk_encode_keys_accepted_the_emitted_file`
(`cmd_decompose_roundtrip.rs:355-375`) asserts both substrings are present.
`generate.sh` (`:78-83`) captures `$?` into `R1`/`R2` from **running** route1.sh
and route2.sh for real (with `md`/`mk` symlinked onto `PATH`) and exits 1,
dumping stderr, before ever writing the header if either is nonzero — so the
header's "= 0" claims are measurements, not decoration.

No `#[ignore]` anywhere in the touched files (grepped
`cmd_decompose.rs`/`cmd_decompose_roundtrip.rs`/`decompose/`/`cmd/decompose.rs`
— zero hits). No conditional early-return or `is_ok()`/`.ok()`-swallow
pattern that could silently skip a check: the only `continue` in
`cmd_decompose_roundtrip.rs` (line 61, inside `section()`) skips
non-matching `# @@` header lines while scanning the fixture, not a test
assertion. Confirmed live: `cargo nextest run --locked -E 'test(v_d_)'` → `30
tests run: 30 passed, 1010 skipped` — the 1010 are the rest of the 1038-test
suite not matching the filter (i.e. everything outside `v_d_*`), not silently
dropped `v_d_*` rows; all 30 named rows show `PASS` in the per-test log.

## Q4 — FALSE-PASS SHAPES ACROSS THE 30 ROWS

PASS, modulo the Minor already noted under Q1(b). Read every one of the 30
`v_d_*` tests in both files. Findings:

- **No bare exit-code-only refusal assertion.** Every test that asserts a
  refusal pairs `assert_ne!(code, 0)` with specific stderr content (checksum
  values, `"listdescriptors"`, `"\"desc\""`, `"<0;1>"`, `"ONE
  descriptor"`/`"one descriptor"`, both depths, `"child"`, `"@0"` + a
  16-char key prefix + `"origin"`, `"pairwise distinct"` + both origin
  strings, `"disjoint"` + the overlapping path set, `"BIP 388
  permits"`/`"permitted by BIP 388"` + `"md encode"`/`"md's template"`).
  Rows that check `code == 0` alone (`v_d_depth_a_consistent_key_is_not_refused`,
  `v_d_noorig_emit_commands_succeeds_when_every_key_has_an_origin`,
  `v_d_reuse_distinct_keys_are_not_refused`) are legitimate negative halves —
  proving *absence* of a refusal, where a bare exit check is the correct
  assertion.
- **Negative halves exercise genuinely different inputs, not the same
  fixture re-asserted.** `v_d_pair_a_single_fixed_path_descriptor_is_still_accepted`
  passes a single fixed-path descriptor (not the pair) so an "always refuse
  N≠1" implementation can't hide behind it; `v_d_reuse_distinct_keys_are_not_refused`
  uses the 3-key fixture that shares one master fingerprint across all keys,
  specifically guarding against an implementation that grouped by
  fingerprint instead of full xpub.
- **Stream discipline is clean.** `run()` (`cmd_decompose.rs:37-48`) and `md()`
  (`cmd_decompose_roundtrip.rs:90-101`) both split `stdout`/`stderr` from
  `assert_cmd`'s `Command::output()`; spot-checked every `stdout.contains`/
  `stderr.contains` call site across both files — refusal-content checks are
  always against `stderr`, emitted-artifact checks always against `stdout`.
  No site asserts refusal text on stdout or artifact content on stderr.
- **`emit_commands_route1_line_actually_runs`** (`cmd_decompose.rs:505-547`)
  executes the emitted line through `sh -c` with the real `md` on `PATH` and
  asserts both `status.success()` and `stdout` starts with `md1` — this is
  the test that caught the real `'…'`-quoting defect the report names; it is
  not a substring/exit-only check.

## Q5 — THE WALKER'S TWO FAIL-CLOSED GUARDS

PASS — both in the production path. `walk::build_template`
(`walk.rs:229-271`) contains both guards inline (the `xpub`/`tpub`/…
substring check at `:255-262`, the `@` count check at `:263-269`) with **no**
`#[cfg(test)]` gate around the function or the checks. `decompose()`
(`mod.rs:382-398`) calls `walk::build_template(&desc, &occurrences)?`
unconditionally as the production entry point's own logic — every `md
decompose` invocation runs through it. (They are currently unreachable
*given* the code that runs before them — `check_no_repeated_key` already
refused any collision before `build_template` runs, so the translator's
`self.map.get(&rendered)` lookup can't miss — which is exactly what makes
them fail-closed defense-in-depth rather than load-bearing on the happy
path; that is the stated intent, not a defect.)

## Q6 — DEVIATIONS 2-5 vs SPEC TEXT

**2 (BIP-legal disjoint repetition also refuses, naming md not the BIP).**
No contradiction — and stronger than the report states. SPEC A3
(`design/SPEC_wallet_form_converter.md:204-206`) reads: "Rows: shape (1) both
directions; shape (2) DECOMPOSE side only (rust-miniscript parses all three
forms, so that refusal is reachable and testable), while the compose side has
no row." The disjoint sub-case (`v_d_shape2_disjoint_sets_refuse_naming_mds_narrower_template_surface`)
is exactly "shape (2)... reachable and testable" on the decompose side — this
is a SPEC-mandated row, not merely "an extra refusal... needed for
soundness" as the report's deviation list frames it (§Deviations item 2).
The plan's 8-row table just doesn't split V-D-SHAPE2 into its 3 sub-cases by
name; the code and the test are correct either way. Recorded as the second
Minor below (report-accuracy, not a code defect).

**3 (`--network` added).** No contradiction — SPEC text has zero mentions of
"network" anywhere (grepped case-insensitively across the whole spec file;
the only hit came from the unrelated phrase "**M**easured scope note").
Purely additive, matching the report's own "not named in the plan" framing.

**4 (`--emit descriptor` exists).** No contradiction — SPEC-anticipated.
SPEC P3's "Origin-less keys" bullet states verbatim: "The template and
descriptor outputs still work"
(`design/SPEC_wallet_form_converter.md:368`) — a descriptor-emission mode is
presupposed by the SPEC text itself, not invented by C3.

**5 (route 2's `cat > keys.txt <<'MDKEYS'` heredoc).** No contradiction — SPEC
does not address the presentation mechanics of `--emit commands`' output at
all; the section is labelled as shell rather than `md`/`mk`, matching the
report's own caveat.

## Summary

| # | Severity | Finding |
| --- | --- | --- |
| 1 | Minor | `assert_bip388_wording`'s first check (`"forbidden by BIP 388"`, lower-case) is case-sensitive and doesn't match rule (2)'s sentence-initial "Forbidden by BIP 388" — both SHAPE2 non-disjoint rows pass that clause only via the generic `"BIP 388"` fallback. Not a false pass (UNSUPPORTED and the BIP-388 citation are independently, genuinely present) but the specific branch is dead for those two call sites (Q1b) |
| 2 | Minor | The report's deviation-2 entry frames `v_d_shape2_disjoint_sets_refuse_naming_mds_narrower_template_surface` as "an extra refusal the roster does not list," needed only "for soundness" — SPEC A3 (lines 204-206) explicitly requires this exact row on the decompose side ("shape (2) DECOMPOSE side only... reachable and testable"). Report-accuracy only; the row itself is correct and present (Q6) |

No Critical, no Important. No secret-handling findings encountered in this
scope.
