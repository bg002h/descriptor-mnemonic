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

## R1 fold (2026-11-13)

Folded into `design/FIX_md_output_sink.md` and `scripts/md-output-sink-fix-gate.sh`:
- The architect's read-only stdout ruling: section 3 is `fix-with-docfix.diff` (sha256
  `9e6ffbe6...359df2`, which includes the one-line `cargo doc` fix in `out.rs`), section 4 is
  `rows.diff` (sha256 `4164126f...798e69`), and all 18 `old:`/`new:` pairs of the architect's
  `md/texts.md`. Checked: head plus the blocks plus the 18 pairs equals the architect's folded
  document byte for byte, before the docfix block and its sha256 were swapped in.
- Counts moved: rows 10 to 11, RED commands 8 to 9, RED whole set `11 tests run: 2 passed, 9 failed`,
  suites 943 and 905, `out.rs` 125 lines, plants P13 and P14 recorded (not re-run).
- Review I1: section 2 line on `--out` replaced verbatim. Review I2: the S2 bullet, build step 3 and
  the "One STOP is open" line replaced verbatim (now "No STOP is open (S1, S2 ruled)").
- Gate: expectations as above, the byte compare runs against the architect's files
  (`fix-with-docfix.diff`, `rows.diff`), and it runs md's CI `cargo doc` command
  (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`)
  to exit 0; `cargo-doc` is out of its "does NOT cover" line.
- Filed, not fixed: 5 Minors (M1 to M5) and 4 Nits (N1 to N4) as `deferred-minor` entries
  `md-output-sink-fix-r1-*` in `design/FOLLOWUPS.md`, inserted before `vendor-freshness-shared` so the
  document's block 3.5 still applies (`git apply --check` of all section 3 blocks passes on this head).
- Gate: `GATE PASS`.

Point for the coordinator (not a STOP, nothing was chosen): the review's ruling on the suite scope
(M2) says to add the CI `test` job's workspace command as a third GREEN line expecting 1619 passed.
The brief files Minors unfixed, and 1619 was measured before the read-only row (the folded tree has
one more test), so M2 is filed as an entry and not folded. If the coordinator wants it folded, the
figure needs measuring on the folded tree.

Counts: Critical 0, Important 0, Minor 0, Nit 0 (fold; 9 review findings filed).
