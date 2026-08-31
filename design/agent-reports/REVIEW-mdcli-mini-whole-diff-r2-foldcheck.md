# REVIEW — mdcli-mini whole-diff fold, mechanical verification, r2

**Date:** 2026-08-31
**Scope:** `git diff 00f08a78..e3d293c7` (9 commits: 8 fold commits
`6d8c655a`..`4b9881e0` responding to `design/agent-reports/REVIEW-mdcli-mini-whole-diff-r1.md`,
plus the FOLD report's own persist commit `e3d293c7`). Worktree
`/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`, branch `mdcli-mini`, HEAD `e3d293c7`.
**Reviewer:** independent mechanical fold-verification — no fresh audit, no re-derivation of
design judgment calls. In scope: I1–I5, M1, M3, M4, M5 from r1 (9 findings; the FOLD report
folds I1 and M4 together in one commit since they share a match arm, but they are two
distinct findings). Out of scope per the dispatch brief: M2 (deliberately deferred, confirmed
"NOT taken" in the fold report) and the five nits (deliberately deferred, confirmed "none
fixed inline" in the fold report).

---

## 9-row table

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| I1 | R-N1a cited the wrong BIP 388 rule (pairwise-distinct) on 4 rendered surfaces | **FIXED** | `bip388.rs` gained `DISJOINTNESS_RULE` (verbatim BIP-388 line 195 quote); `reuse.rs`'s `Finding::SamePathExpression` arm now cites it instead of `PAIRWISE_DISTINCT_RULE`; all pinned test strings (`n1_admission_taxonomy.rs`, `seating_vectors.rs`, `satisfy.rs:506`) updated. Live-verified below. |
| I2 | Funds-shaped `apply_path_override_per_slot` → `refuse_key_reuse_across_slots` ordering had no pin | **FIXED** | `duplicate_key_slots.rs` gained 2 rows (`t_row_key_reuse_through_a_noncanonical_wrapper_sourced_solely_by_path_refuses_at_descriptor` / `_at_address`) using a non-canonical `tr(...)` wrapper whose slots have no inline origin, so only the correct order makes the check fire. Comment states the swap-red proof and points at the FOLD report; not independently re-run per the brief. |
| I3 | `--emit md1` dropped the legacy-P2SH (F-A4) and pathless (P1.2) advisories | **FIXED** | `emit_legacy_p2sh_advisory` / `emit_pathless_advisory` in `cmd/encode.rs` made `pub(crate)`; both now called from `emit_md1_card` in `cmd/descriptor.rs`, in `md encode`'s relative order. New fixture `v-legacy-p2sh.txt` + `n2_emit_md1.rs` rows. |
| I4 | `[Unreleased]` CHANGELOG section stated the opposite of what the cycle ships; cycle added no entry | **FIXED** | Both false paragraphs in the converter section corrected (disjoint-form composing claim, template-admission-inversion claim); a new stacked `## md-cli [Unreleased] — the post-converter mini-cycle` section added covering N1, read-side warnings, `--emit md1`, `--verify-against`, N3, R9's arity guards, decompose `/**`/`-`, and the Fixed item. |
| I5 | decompose's D-row disjoint-multipath refusal quoted a message `md encode` no longer prints | **FIXED** | `decompose/mod.rs:256` doc comment and `:339` rendered message now cite R-N1c and quote the same runnable escape (`parse::reuse::ESCAPE`, made `pub(crate)`) R-N1c/R-N1d use, replacing the dead `"@N appears with inconsistent path/multipath/hardening"` quotation. Third site the finding named, `cmd_decompose.rs`'s matching doc comment, updated identically. |
| M1 | N3's "bracket sources some slots, another slot has no source" variant had no row | **FIXED** | `cli_p1_origin_key.rs` gained `v_n3_a_slot_with_no_path_from_any_source_still_refuses`: `@0` sourced by an N3 bracket, `@1` with no origin from any source; asserts refusal naming `@1` and the non-canonical-wrapper message. |
| M3 | `--verify-against` on an empty FILE named the wrong flag (`--in`) in its error | **FIXED** | `read_md1_inputs` in `cmd/mod.rs` gained a `flag: &str` parameter threaded through all 6 call sites (`"--in"` for the 5 owning verbs, `"--verify-against"` for `resolve_verify_against`); new row `r3_an_empty_verify_against_file_names_verify_against_not_in` asserts the message starts with `md: --verify-against ` and never contains `--in `. |
| M4 | Read-side warning ended with a mint/compose remedy-imperative when nothing was refused | **FIXED** | `Finding::message` now takes a `Disposition` parameter; only `SamePathExpression`'s tail varies — REFUSE keeps "md declines to mint or compose…", WARN now reads "This shape can no longer be minted or composed; the card remains readable." New structural row `r_n1a_warn_tail_differs_from_refuse_tail_per_m4`; `n1_admission_taxonomy.rs` gained `MSG_N1A_WARN` used directly (no more prefix-strip derivation). Live-verified below. |
| M5 | `spend_equal`'s "unchanged bit for bit" comment was stronger than proven | **FIXED** | `seat/compose.rs` comment reworded to state the qualification (holds only when both sides' `expand_per_at_n` succeeds) and the reachability note (unreachable via CLI today). Comment-only, no behavior change. |

**9/9 FIXED.**

---

## Three live verifications

### (a) I1 — R-N1a refusal live run

```
$ ./target/debug/md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" \
    --key "@0=xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf" \
    --path "48'/0'/0'/2'"
md: unsupported: @0 appears at 2 use sites in this template with the same path expression, so
ONE key would fill every one of them. That is forbidden by BIP 388's disjointness rule
("if two KEY are KP/<M;N>/* and KP/<P;Q>/* for the same key placeholder KP, then the sets
{M, N} and {P, Q} must be disjoint"), whose forbidden-example list names
sh(multi(1,@0/**,@0/**)) — "Repeated keys with the same path expression". md declines to
mint or compose this shape: give each distinct key its own placeholder.
exit 1
```

Confirmed: cites the **disjointness rule**, not pairwise-distinct; never says "invalid".

R-N1d, same key material, two placeholders repeating one key at disjoint use sites:

```
$ ./target/debug/md encode "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=<K0> --key @1=<K0>
md: unsupported: @0 and @1 were given the SAME extended public key at DIFFERENT use sites —
<0;1>/* and <2;3>/*. Spelled with two placeholders, this policy lists that key TWICE in
BIP 388's key information vector, and rule (1) requires "the public keys obtained by
deserializing elements of the key information vector must be pairwise distinct" — …
exit 1
```

Confirmed: R-N1d **still cites pairwise-distinct**, correctly — untouched by the fold, as
directed.

### (b) I2 — the two new rows

Confirmed present in `crates/md-cli/tests/duplicate_key_slots.rs`:
`t_row_key_reuse_through_a_noncanonical_wrapper_sourced_solely_by_path_refuses_at_descriptor`
and `..._at_address`. The preceding block comment states the ordering guarantee explicitly
("that order is load-bearing…a slot whose ONLY origin source is `--path` must have it applied
before the duplicate-key check runs, or the check never fires") and cites the swap-red proof
recorded in `FOLD-mdcli-mini-whole-diff-r1.md`. Per the brief, the swap itself was **not**
re-run here.

### (c) M4 — read-side warning tail on the R-N1a fixture card

```
$ ./target/debug/md decode md1fakqnqspqztvyyy4qqxppcgg4gythgx8egtq4pcwl6u5p2us6r6zsnl2rd0q6gghvalgywfyx3z0nn28m7t \
    md1fakqnqs0cdlz64mrqgdrha0m7umapumfj075dhzfzvynh66n94j5lcxlmx9ayav9mj0jjqpx5yl5n7q5v9j
md: warning: @0 appears at 2 use sites … This shape can no longer be minted or composed;
the card remains readable.
wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))
note: key origins carried by this card (not shown in the template): @0: m/48'/0'/0'/2'
note: stdout is a keyless descriptor template (no keys)
exit 0
```

(Two chunks from `crates/md-cli/tests/fixtures/n1/r-n1a-keyed.txt`, passed positionally —
`--in` on the raw fixture file fails to decode because the file's comment/provenance lines
are not stripped by `--in`'s reader; the fixture is designed for the same positional-chunk
invocation the test suite uses.) Confirmed: exit 0, template printed, and the tail no longer
reads "md declines to mint or compose this shape" as a remedy-imperative.

---

## Survivor grep

- `PAIRWISE_DISTINCT_RULE` usages (4 total): `bip388.rs` (definition),
  `reuse.rs:348` (R-N1d, correct — untouched), `seat/build.rs:355` and
  `seat/satisfy.rs:256` (both the card-path/door-check equivalent of R-N1d, same
  substantively-correct citation). **No survivor** — none of these are in the R-N1a
  context the fold retired.
- `"inconsistent path/multipath/hardening"` (live code/test hits, outside
  `design/agent-reports/`): `crates/md-cli/src/parse/template.rs:782` (the original
  generic message in `resolve_placeholders`, now preempted for every Family-1 case
  `reuse::classify` covers — accompanied by an accurate same-file comment at `:2667`
  explaining exactly that preemption) and `crates/md-cli/tests/n1_admission_taxonomy.rs:253`
  (a comment explicitly labeled "REACHABILITY, MEASURED at the plan's baseline binary
  (b8a64938 surface)" — a historical measurement predating N1, not a live claim).
  Neither is a rendered message the fold's 9 findings named for correction — I5 named
  three specific sites (`decompose/mod.rs:256`, `:339`, `cmd_decompose.rs`'s matching
  comment) and all three are fixed (verified above). Design-doc hits
  (`design/BRAINSTORM_mdcli_mini.md`, `design/IMPLEMENTATION_PLAN_mdcli_mini.md`,
  `design/FOLLOWUPS.md`, `design/SPEC_wallet_form_converter.md`, and the
  `agent-reports/*` files) are historical planning/closure records — the
  `FOLLOWUPS.md` hit sits inside an entry explicitly marked
  "**✓ CLOSED by P2+P3 (2026-08-30)**", and `SPEC_wallet_form_converter.md`'s copy is
  explicitly out of scope per I5's own stated ruling ("no edit to the shipped
  converter spec"). **No problematic survivors found.**

---

## Gate

`./scripts/phase-gate.sh` — all six steps passed:

```
cargo nextest run --locked --all-features: 1186 tests run: 1186 passed, 2 skipped
cargo test --workspace --doc: ok (0 doctests)
cargo clippy --locked --all-targets --all-features -- -D warnings: clean
cargo fmt --check: clean
cargo doc --workspace --no-deps --document-private-items --all-features: clean
design/display-grouping-vectors.tsv.sha256: OK
phase-gate: all six steps passed
```

Test count (1186) matches the fold report's own count. Working tree clean
(`git status --porcelain` empty) after all verification.

---

## Verdict

This closes the whole-diff review loop at 0 Critical / 0 Important: all 8 in-scope
finding-groups (I1–I5, M1, M3, M4, M5) verified FIXED against the diff and, where specified,
live-reproduced; no survivors of the retired wordings found outside historical record; the
gate is green.

**findings: 9/9 FIXED; survivors: 0; gate: PASS**
