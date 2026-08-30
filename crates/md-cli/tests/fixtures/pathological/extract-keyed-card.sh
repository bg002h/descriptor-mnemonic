#!/usr/bin/env bash
# Re-extract `keyed-card.txt` — the 22-string KEYED md1 card of the
# pathological vault (plan D2; SPEC Acceptance 1(b) and 4).
#
# WHY AN EXTRACTOR AND NOT A COPY. The card has no standalone file in the
# journey output: `backup-strings.txt` next to it carries the SPLIT set (6 md1
# policy chunks + 30 mk1 key chunks), and the keyed card exists only inside the
# rendered journey page, where it appears TWICE (once per render of the same
# section). So the fixture is derived, and a derivation nobody re-runs rots
# silently — this script is the re-run.
#
# THE SHAPE IS ASSERTED BEFORE ANYTHING IS WRITTEN. A failed shape assertion is
# a FAILED EXTRACTION, not a skipped step: the script exits non-zero and leaves
# the committed fixture untouched.
#
#   raw tokens matching `md1fatzr2…`   44   (42 x 86 chars + 2 x 59)
#   after order-preserving dedupe      22   (21 x 86 chars + 1 x 59 tail)
#
# The `md1fatzr2` prefix is not decoration: the same page also carries the
# KEYLESS policy card (`md1fkl3cz…`, 6 chunks x 5 renders) and a third,
# unrelated 4-chunk card (`md1fveszps…`), so an unfiltered `md1[0-9a-z]+` sweep
# would splice three different cards together. Measured 2026-08-30: 78 md1
# tokens on the page, 44 of them this card's.
#
# Usage, from anywhere:
#   SRC=/path/to/journey_pathological.html \
#     crates/md-cli/tests/fixtures/pathological/extract-keyed-card.sh
#
# DETERMINISM: `grep -o` + first-appearance dedupe is a pure function of the
# source page, and the header's date line is the only field a re-run rewrites.
# `git diff` after running this is the check that the path has not rotted.
set -euo pipefail

SRC="${SRC:-/scratch/code/shibboleth/mnemonic-engrave/design/journeys/out/pathological/journey_pathological.html}"
OUT="$(cd "$(dirname "$0")" && pwd)/keyed-card.txt"
PREFIX='md1fatzr2'

[ -r "$SRC" ] || { echo "FATAL: source page not readable: $SRC" >&2; exit 1; }

raw=$(grep -o "${PREFIX}[0-9a-z]*" "$SRC" || true)
[ -n "$raw" ] || { echo "FATAL: no ${PREFIX}… tokens in $SRC" >&2; exit 1; }

n_raw=$(printf '%s\n' "$raw" | wc -l | tr -d ' ')
[ "$n_raw" = 44 ] || { echo "FATAL: expected 44 raw tokens, found $n_raw" >&2; exit 1; }

raw_shape=$(printf '%s\n' "$raw" | awk '{print length($0)}' | sort -n | uniq -c | awk '{printf "%sx%s ", $1, $2}')
[ "$raw_shape" = "2x59 42x86 " ] || {
  echo "FATAL: raw token shape is '$raw_shape', expected '2x59 42x86 '" >&2; exit 1; }

uniq_lines=$(printf '%s\n' "$raw" | awk '!seen[$0]++')
n_uniq=$(printf '%s\n' "$uniq_lines" | wc -l | tr -d ' ')
[ "$n_uniq" = 22 ] || { echo "FATAL: expected 22 unique strings, found $n_uniq" >&2; exit 1; }

# W-PIN: 21 strings of 86 chars, then a 59-char tail — and the tail must be
# LAST, because the dedupe preserves first-appearance order and the card's
# final chunk is the short one.
head_lens=$(printf '%s\n' "$uniq_lines" | head -21 | awk '{print length($0)}' | sort -u)
tail_len=$(printf '%s\n' "$uniq_lines" | tail -1 | awk '{print length($0)}')
[ "$head_lens" = "86" ] || { echo "FATAL: the first 21 strings are not all 86 chars ($head_lens)" >&2; exit 1; }
[ "$tail_len" = "59" ] || { echo "FATAL: the 22nd string is $tail_len chars, expected 59" >&2; exit 1; }

{
  echo "# PROVENANCE"
  echo "# EXTRACTED $(date +%F) from"
  echo "#   $SRC"
  echo "# by crates/md-cli/tests/fixtures/pathological/extract-keyed-card.sh"
  echo "# Extraction command (what the script runs):"
  echo "#   grep -o '${PREFIX}[0-9a-z]*' <page> | awk '!seen[\$0]++'"
  echo "#"
  echo "# The KEYED md1 card of the pathological vault — the same 11-key wallet"
  echo "# backup-strings.txt holds in SPLIT form, minted instead as one monolithic"
  echo "# keyed card (md Pubkeys TLV). The page carries it TWICE (44 tokens); the"
  echo "# dedupe above is order-preserving, so chunk order is the card's own."
  echo "#"
  echo "# W-PIN (SPEC Acceptance 4, plan section 3 C4 item 1): 22 strings ="
  echo "# 21 x 86 chars + one 59-char tail. ASSERTED by this script before the file"
  echo "# is written, and again by tests/acceptance_walks.rs before any walk runs."
  echo "# Composed, this card yields a 1,648-char descriptor (1,649 bytes with the"
  echo "# trailing newline) whose address 0 is"
  echo "#   bc1qkuknuy6dsm0fq44cyyhzqy9wl3ex2n6ed39zxhx867l9wlh4yhlsejms64"
  echo "#"
  echo "# Lines beginning with '#' and blank lines are provenance only; test"
  echo "# helpers skip them."
  printf '%s\n' "$uniq_lines"
} > "$OUT"

echo "== wrote $OUT: 22 strings (21x86 + one 59-char tail), from $n_raw raw tokens"
