# REVIEW — mechanical fold-check, IMPLEMENTATION_PLAN_mdcli_mini.md r3 → r4

- **Artifact:** `design/IMPLEMENTATION_PLAN_mdcli_mini.md`, plus one addition to
  `design/SPEC_mdcli_mini.md`'s Principle section
- **Commit checked:** `2433fd19` (fold under review: `git diff 2a21ece0..2433fd19`)
- **Date:** 2026-08-31
- **Scope:** mechanical fold-verification only, against
  `design/agent-reports/REVIEW-mdcli-mini-plan-r3.md` findings I-1, M-1, N-1,
  N-2. No fresh audit; no re-review of settled rounds.

## Four-row table

| r3 finding | verdict | evidence |
| --- | --- | --- |
| **I-1** — fold's justification for the door-check unification ruling was false (asserted R-N1d/coarseness, undetectable in principle) | **FIXED** | Plan:121-133 rewritten. Grounds the unification on the spec's SINGLE-SOURCE rule: "descriptor's card input is on the REFUSE mint/compose surface, so P3.1's new card-input Family-1 refusal would otherwise be a SECOND implementation of the predicate already shipped at `satisfy.rs:188`" — matches spec's Verb-dispositions ("REFUSE (mint/compose): … descriptor (both --template and card input)") and its Single-source paragraph ("each predicate has ONE implementation — no per-verb second copy"). States the reachable domain as "EXACTLY R-N1a — where its shipped wording is correct." Drops the retired "COARSER"/R-N1d claims (confirmed absent, see consistency check (a)). Forbids moving the refusal past A3: "its position is order-normative (`seat/mod.rs:100-106`…) and MUST NOT move past A3 to obtain key bindings it does not need." All four sub-asks present. |
| **M-1** — unification deliverable lived only in P2's preamble; no P3 step named it | **FIXED** | Plan:277-282 adds step **3b** inside P3: "**The door-check unification (r3 M-1 — the deliverable ruled in P2's preamble, restated here where its implementer reads):** `check_no_repeated_placeholder` becomes an in-place invocation of the shared classifier with the R-N1a rendered line; `seating_vectors.rs:679-687` and `satisfy.rs:530-548` update in the same commit; the refusal does not move past A3." P3 now names the check, the unification, and both pinning sites as its own step. |
| **N-1** — stale "nine later fixtures" sentence six lines above its own correction | **FIXED** | Plan:249: "truncating `v-r5m1.txt`, leaving **every later fixture** unregenerated)" replaces "nine later fixtures." Grep for the literal string across the file returns zero hits (below). |
| **N-2** — two bare `corpus_origin_consistency.rs` citations missing directory | **FIXED** | Plan:223 and :228 both now read `crates/md-cli/tests/corpus_origin_consistency.rs` in full. Grep for a bare (non-full-path) occurrence returns zero hits (below). |

**4/4 FIXED.**

## Consistency check (a) — grep for retired claims

Ran against `design/IMPLEMENTATION_PLAN_mdcli_mini.md` at `2433fd19`:

```
$ git grep -in "coarser" 2433fd19 -- design/IMPLEMENTATION_PLAN_mdcli_mini.md
(no output)
$ git grep -in "R-N1d-disjoint spellings with Family-1 wording" 2433fd19 -- design/IMPLEMENTATION_PLAN_mdcli_mini.md
(no output)
$ git show 2433fd19:design/IMPLEMENTATION_PLAN_mdcli_mini.md | grep -noE '[^/`]corpus_origin_consistency\.rs'
(no output — both occurrences are preceded by `/`, i.e. carry a full path)
$ git grep -in "nine later fixtures" 2433fd19 -- design/IMPLEMENTATION_PLAN_mdcli_mini.md
(no output)
```

No sentence in the plan still asserts any of the four retired claims.

## Consistency check (b) — spec Principle section vs. adjacent N1 text

Read `design/SPEC_mdcli_mini.md` at `2433fd19`, Principle section (added
sentence) against the N1 "Untouched" paragraph and the R-N1c row.

Added sentence: *"No carve out for reused keys unless different origin
paths" — the same-path reuse refusals stand with no exception, and one
master at different origin paths (different derived xpubs) remains the
legitimate, control-row-pinned family.*

- **Against the "Untouched" families text** (N1 section): family (ii) is
  defined as *"ONE master (same fingerprint) at DIFFERENT account paths —
  different derived xpubs, no reuse."* The added sentence's "one master at
  different origin paths (different derived xpubs) remains the legitimate…
  family" restates this definition exactly — same fingerprint, different
  account/origin path, different derived xpub. **Ratifies, no disposition
  change; no contradiction.**
- **Against the R-N1c row:** R-N1c governs a different axis — one
  placeholder whose **multipath sets** differ and are disjoint (e.g. `<0;1>`
  vs `<2;3>` on the *same* origin path), refused for F-417 wire-narrowness
  reasons ("one path per key slot"), independent of key-reuse status. The
  Principle sentence's "different origin paths" refers to the origin-path
  axis (the spec's separate R-N1-origin row), not the multipath axis R-N1c
  sits on. The two do not overlap in scope, so there is no contradiction —
  R-N1c's "refusal STANDS" is untouched by the ruling.

No contradiction found.

## Verdict

**r3 findings: 4/4 FIXED; contradictions: 0**

This closes the plan's R0 loop at 0C/0I (r3's sole Important, its one
Minor, and both Nits are all fixed; the one spec addition is a ratifying
record consistent with the surrounding text).
