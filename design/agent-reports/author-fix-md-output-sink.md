# Report: author of design/FIX_md_output_sink.md (md output sink)

Branch `fix/md-output-sink` from main `6b70d5d964d9dafcae0c1cfef926a0a7887ec6a9`.
Worktree `/scratch/cache/wt-md-fix-output-sink`. Document, gate script and this
report only; no production code on the branch.

## Gate (scripts/md-output-sink-fix-gate.sh, last lines)

```
cmp: blocks identical to the grade's files
RED whole set: 10 tests run: 2 passed, 8 failed; both guards PASS
self-test ok: the checker rejects a broken row
GREEN rows: 10 tests run: 10 passed; unit: 2 tests run: 2 passed
GREEN suite cli-compiler: 942 tests run: 942 passed, 3 skipped
GREEN suite default: 904 tests run: 904 passed, 3 skipped
GREEN doc, fmt, clippy, code-docs check: ok
GATE PASS
```

The 8 RED commands each failed for the stated reason (rc 100 with the expect
substring). Counts match the grade's.

## Case table

In the document, section 2 (rows 1 to 9): closed reader (0; repair 5 kept), other
stdout error (0, 4, 5 become 1; 1 and 2 kept), `--help`/`--version` both ways,
stderr write error ignored, clap parse error unchanged, `descriptor
--verify-against` 5 becomes 1 with the verdict line kept, `gen_ap2_grind`
routed through `out.rs` (source-scan row only).

## STOPs and proposals

- **S2 / P-1 (proposed)**: the grade's FOLLOWUPS diff has `resolved <COMMIT>`
  and does not say how it is filled; a commit cannot hold its own SHA. Proposed:
  apply block 3.5 as is, then a second commit replaces `<COMMIT>` with the
  short SHA of the code commit. Needs a ruling before the build.
- S1 (manual timing) is ruled and carried as such (section 8, 10).

## Findings during the work (decided or noted)

- The grade's suite counts (942, 904) are `-p md-cli`; the workspace has 1619
  tests. The document and gate say `-p md-cli`, and list the workspace-wide
  run as a CI leg the gate does not replace.
- The unit test `out::tests::...` runs twice (once in `md`, once in
  `gen_ap2_grind`, which includes `out.rs`): the gate expects `2 tests run`.
- `ci/code-docs-check.sh` (clippy job) caught my first gate script naming the
  FOLLOWUPS file; reworded. The gate now runs that check.
- The manual paragraph (grade's text, verbatim) says "exits 1" and does not
  mention that md's 4 and 5 also become 1; true, so left as ruled.
- Grade's fmt ran on stable; the gate uses the pinned 1.85.0.

## Not run

The 14 plants, the macOS, Windows, FreeBSD, musl, cargo doc, man-pages, fuzz,
vendor-freshness and bitcoind-differential CI legs, the workspace-wide nextest,
the toolkit manual paragraph and lint, `mutation-gate.sh`, any review. All are
listed in section 9 of the document.

Counts: Critical 0, Important 0, Minor 0, Nit 0 (author report; 1 open STOP).
