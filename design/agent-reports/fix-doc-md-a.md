# FIX-A document author report (md bug hunt)

Brief: `coordinator/briefs/fix-doc-md-bughunt.md`, FIX letter A. Worktree
`dm-worktrees/bughunt-fix-a`, branch `fix/bughunt-a`, base b2bfdd63.
Output: `design/FIX_bughunt_md_a.md`, and RED rows in
`crates/md-cli/tests/bughunt.rs`. No production code is committed.

## What was done

- Tightened the 7 triage rows (01, 03, 04, 10, 11, 12, 14) to assert the
  rulings' exact texts and exit codes. Their names are unchanged.
- Added 3 rows: 04b (address), 04c (descriptor) and 11b (inspect text).
- RED on unmodified code: `/scratch/cache/bughunt/md/fixa-red-unmodified.log`.
  All 10 FIX-A rows fail for their stated reasons (table in FIX §3).
- GREEN check: the hint patches were applied as a scratch change and then
  reverted.
  - The 10 rows ran 10 and passed 10: `/scratch/cache/bughunt/md/fixa-green-scratch.log`.
  - The workspace run had 11 failures: `/scratch/cache/bughunt/md/fixa-green-scratch-ws.log`. They are:
    - the 8 FIX-B codec rows;
    - the private rows 06 and 13;
    - `cmd_encode` `encode_with_policy_id_fingerprint_prints_two_lines`, the one test that changes under R-10.
- #13 (Q-B) probe: md fails closed. Fed the same mk1 card twice, it exits 1 with "not a decodable mk1 string: invalid data-part length: 135". So #13 stays Minor and its row is left out.
- Toolkit manual lines 40 and 43 were checked on the toolkit's `origin/master` f6355e22, not on 87758398 as the earlier note said. Their old text is the same on both.

## Findings

- **STOP S-1: R-4 contradicts itself on values in [2^29, 2^31).**
  - R-4 says address and descriptor keep accepting these values.
  - R-4 also makes `to_miniscript` return VarintOverflow for any value above 2^29-1.
  - address and descriptor go through `to_miniscript`, so once FIX-B lands they refuse `<536870912;1>`. Today they accept it.
  - This goes to the architect. FIX-B owns the codec rule.
- **STOP S-2:** the names of rows 04b and 04c are my proposal; the grade names neither.
- **STOP S-3:** `<4294967296;1>` is ≥ 2^31 but does not parse as a u32. Today it gets "not a bare unsigned integer". It is undecided whether it should get the hardened message instead.
- **STOP S-4:** the private RED rows 06 (deferred to the migration) and 13 (Minor, not fixed) are in neither FIX. While they stay, the workspace nextest stays red. How they are disposed of, and by which FIX, is undecided.
- **Minor:** R-10's manual text says "stdout carries only the md1 artifact". That does not hold under `--json`, where the fingerprint stays a JSON field on stdout. The manual row describes the text mode, so this is recorded and not changed.
- FIX-B's doc did not exist when this one was written. My statement that no FIX-A row depends on a FIX-B codec change rests on my reading of the grade's R-2/R-4/R-8 text, not on FIX-B's doc.

Counts: 0 Critical, 0 Important, 1 Minor, 0 Nit; 4 STOPs (S-1..S-4).
