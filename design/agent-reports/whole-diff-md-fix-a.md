# Whole-diff review: md bug hunt FIX-A (single slice, Sonnet, medium)

Target: fix/bughunt-a, production diff f61e3c8e..42bd9c91 (md-cli); toolkit db7a4f68.
Not a re-review of a fold; no finding comes from a fix commit.

## Checks run
- Scratch worktree of 42bd9c91 (since removed). Filter of the 12 rows: 12 run, 12 passed.
- Planted each defect back (reverted each hunk): every RED row fails for the stated reason
  (assertion, not compile error):
  - R-1 (drop validate call): 01 fails at "--json exited 0".
  - R-3 (old if/else): 03 fails with "--json failed where the text path chunks ... 246 data symbols".
  - R-4 (drop u32 disjunct): 04, 04b, 04c fail comparing stderr; got the varint text, wanted the hardened text.
  - R-10 (eprintln -> println): 10 fails on the non-artifact stdout line.
  - R-11 (WatchOnly -> Template): 11 and 11b fail on keyless / missing WATCH_ONLY_NOTE.
  - R-12 (drop hexdigit check): 12 fails ("+f+f+f+f accepted"; from_str_radix takes "+f", so the plant is real).
  - R-14 (disable fingerprint conjunct): 14 fails on the note being printed.
- Ignores: grep of crates/md-cli finds no "owed to FIX" left; the two §2.9 attributes match the FIX text
  exactly; the seven deleted ignores belong to rows that all pass (12/12). Rows 06 and 13 stay ignored with FOLLOWUPS owners.
- Printed text, run as printed with the FIX-A binary:
  - R-1 `--json`: stderr identical to the §2.1 text, exit 1, stdout empty.
  - R-4: `<1;2147483648>` and `<2147483648;1>` give the hardened text; `<4294967296;1>` gives "not a bare unsigned integer";
    `<536870912;1>` gives the varint text. All exit 1.
  - R-10: stdout is the card alone; stderr has the line; `--json` keeps the field on stdout.
  - R-12: `md: --fingerprint @0: invalid hex byte: +f`, exit 1; uppercase hex still accepted.
  - R-14: stdout `wpkh(@0/<0;1>/*)`, stderr only the keyless note.
  - R-11 keyless input keeps the keyless note (keyed path covered by rows 11/11b).
- All printed strings match the FIX doc byte for byte. Manual lines 40 and 43 match §2.8 exactly.
  No regression found; CLI text elsewhere unchanged.

## Findings
No Critical, no Important.

Minor
- M1. toolkit `docs/manual-gui/src/50-md/53-encode.md:34` and `:209-210` still say the fingerprint prints
  "after the phrase ... on stdout" (R-10 makes this false), and `:218-220` `--json` describes nothing about chunks.
  Outside FIX §2.8's two lines, fails closed (documentation only). Fix wording line 209-210:
  "Prints the freshly-computed `policy_id_fingerprint` on stderr after the card (stdout carries only the md1 artifact). Default off."
  Owner: the toolkit manual-gui pass (or the md-cli pin bump that follows the md release).

Nits
- N1. `crates/md-cli/tests/bughunt.rs` doc comments on rows 10 and 14 still say "OPEN PRODUCT QUESTION" though rulings R-10 and R-14 closed them.

0 / 0 / 1 / 1
