#!/usr/bin/env bash
# matrix-identity-check.sh -- the MATRIX TRAVELS directive, as a command.
#
# Operator directive (2026-08-30, verbatim): "Make sure that 'in \ out' table
# stays embedded in all design documents … From brainstorm to spec to plan (and
# maybe even in the code as a comment)." Four copies of one table drift the
# moment a cell is edited in three places out of four, and drift in THIS table
# is not cosmetic: the cells are the cycle's claim about what works.
#
# So the table is machine-compared rather than eyeballed. This script extracts
# the six table lines (header, separator, four rows) from each of the four
# homes, strips the Rust doc-comment prefix from the code copy, and requires
# all four extractions to be byte-identical.
#
#   scripts/matrix-identity-check.sh          # exit 0 = identical
#
# It checks IDENTITY, not truth: four copies of a wrong cell pass. Whether a
# cell is true is what `crates/md-cli/tests/acceptance_walks.rs` and the vector
# roster answer.
set -euo pipefail
cd "$(dirname "$0")/.."

FILES=(
  design/BRAINSTORM_wallet_form_converter.md
  design/SPEC_wallet_form_converter.md
  design/IMPLEMENTATION_PLAN_wallet_form_converter.md
  crates/md-cli/src/seat/mod.rs
)

TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
n=0
for f in "${FILES[@]}"; do
  [ -r "$f" ] || { echo "FATAL: missing matrix home: $f" >&2; exit 1; }
  # Take from the header row through the last table row, then drop the `//! `
  # prefix the code copy carries.
  sed -n '/^\(\/\/! \)\?| in \\ out |/,/^\(\/\/! \)\?| \*\*K\*\*/p' "$f" \
    | sed 's|^//! ||' > "$TMP/$n.tbl"
  lines=$(wc -l < "$TMP/$n.tbl" | tr -d ' ')
  [ "$lines" = 6 ] || {
    echo "FATAL: $f yielded $lines table lines, expected 6 (header + separator + 4 rows)" >&2
    exit 1; }
  n=$((n + 1))
done

status=0
for i in $(seq 1 $((n - 1))); do
  if ! diff -u "$TMP/0.tbl" "$TMP/$i.tbl" > "$TMP/$i.diff"; then
    echo "DIFFERS: ${FILES[0]} vs ${FILES[$i]}"
    cat "$TMP/$i.diff"
    status=1
  fi
done

if [ "$status" = 0 ]; then
  echo "matrix identical across ${#FILES[@]} homes, 6 lines each:"
  printf '  %s\n' "${FILES[@]}"
  echo "sha256: $(sha256sum "$TMP/0.tbl" | cut -d' ' -f1)"
fi
exit "$status"
