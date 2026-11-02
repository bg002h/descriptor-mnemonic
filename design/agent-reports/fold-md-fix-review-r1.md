**⚠** STOP: 2 items need a controller decision (see below).

# Fold report: md FIX-B review r1 (I-1, I-2) into FIX-B and FIX-A docs

Brief: `briefs/fold-md-fix-review-r1.md`. Source: `design/agent-reports/fix-review-r1-md-b.md` (ef7cc45b), §I-1 and §I-2. Docs only; no gate run.

## Applied (6 of 6 texts)

| # | doc | § | source | result |
|---|---|---|---|---|
| 1 | FIX-B `design/FIX_bughunt_md_b.md` | §5 "Expected workspace result for step 1" paragraph | report I-1 fix text | replaced verbatim |
| 2 | FIX-B | §4 bullet | report I-1 "Also add to §4" line | added as a bullet before "No other test changes." |
| 3 | FIX-B | §5 step 0 | report I-2 fix text | inserted before step 1, verbatim |
| 4 | FIX-A `design/FIX_bughunt_md_a.md` | §5 step 1 | brief text | replaced |
| 5 | FIX-A | §2.9 lead-in | brief text | replaced |
| 6 | FIX-A | §4 bullet (06 and 13) | brief text | replaced |

Commits (branches not pushed):
- FIX-B: `42a6b10b` on `fix/bughunt-b`, file `design/FIX_bughunt_md_b.md` only.
- FIX-A: `6b124904` on `fix/bughunt-a`, file `design/FIX_bughunt_md_a.md` only.

## Flags for the controller (not decided here)

1. **Residual duplicate sentence (FIX-B §5).** The brief's anchor ends at "…FIX-A `#[ignore]`s 06 and 13)." The original paragraph continues " Any other failure is / a FIX-B FAIL. Then the mutation gate per the coordinator's rules." That stays after the report's text, which already ends "Any failure is a FIX-B FAIL." The paragraph now says the same failure rule twice. I left it as the anchor dictates. Stop and decide whether to delete the residual sentence.
2. **Contradiction with §4 (FIX-B).** §4 opens "All committed with this doc; the builder does not edit tests." The new bullet has the builder add ignore attributes to three test files. Decide whether to amend the §4 lead-in.
3. **Bullet lead-in dropped (FIX-B §4).** The brief said to add the "Also add to §4" line as a bullet. I used its body only, starting "`crates/md-cli/tests/bughunt.rs`…", because the lead-in is redundant inside §4. Reinstate it if you want it verbatim.

## Machine checks run

- Replacement texts were extracted from the report by line number, and each anchor was asserted to occur exactly once before replacement.
- `git diff` on both files was read; the only changed hunks are the ones listed above.

Counts: 6 applied / 6 texts; 3 flags (2 STOP, 1 judgment).
