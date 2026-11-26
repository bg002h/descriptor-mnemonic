#!/usr/bin/env bash
# Gate for FIX_bh1119_md_out_names_input ([md-out-names-input]), FIX section 7.
# Usage: bash scripts/fix-bh1119-md-out-names-input-gate.sh   (no arguments; run from the repo root;
#   $HOME/.cargo/bin first in PATH; --locked everywhere; every cargo call with CARGO_BUILD_JOBS=6).
#   MD=<path> runs step 1 against that md binary instead of building one (used to show it RED on the
#   base). Every step runs; exit 0 only if all passed, else exit 1 after printing the failed step names.
# Steps: 1 build md-cli, then run design/FIX_bh1119_md_out_names_input/red.sh (rows n1-n29, c1-c7)
#   against the built md and compare its whole output (each row's rc, stdout size, stderr line and the
#   protected file's state) with design/FIX_bh1119_md_out_names_input/expected.txt, which is the
#   FIX section 4 "fixed" column; 2 the repo CI set (ci.yml); 3 the vendor-freshness gate (network for
#   its check 4).
# Exit codes (2026-11-26): expected.txt's n rows now expect 8 (refused) and c5 expects 1 (NOT
#   spend-equal), after the constellation exit-code cutover (mnemonic-engrave
#   design/SPEC_constellation_exit_codes.md); they were 2 and 5 when this fix landed.
# Does NOT cover: the toolkit manual edit (M3; separate toolkit change), the man-pages and release
#   workflows, the macOS/Windows/FreeBSD legs (io-lib's same_file limits off Unix), the
#   WORDING_REVIEW rows.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=6
cd "$(git rev-parse --show-toplevel)" || exit 2
failed=()
step() { local name=$1; shift; echo "=== $name"; "$@" || { echo "FAILED: $name"; failed+=("$name"); }; }

if [ -n "${MD:-}" ]; then
  build() { echo "using MD=$MD"; }
else
  MD="${CARGO_TARGET_DIR:-target}/debug/md"
  build() { cargo build --locked -p md-cli --bin md; }
fi

rows() {
  local D rc
  D=$(mktemp -d -p /scratch/cache)
  bash design/FIX_bh1119_md_out_names_input/red.sh "$MD" "$D/rows" > "$D/actual.txt" 2>&1
  diff -u design/FIX_bh1119_md_out_names_input/expected.txt "$D/actual.txt"
  rc=$?
  if [ "$rc" = 0 ]; then echo "PASS rows n1-n29 c1-c7 match the fixed column"; fi
  rm -r "$D"
  return $rc
}

# Step 1
step "1a build" build
step "1b rows" rows

# Step 2
step "2a nextest" cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast --build-jobs 6 -j 6
step "2b doctests" cargo test --locked --workspace --doc --all-features
step "2c clippy" cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
step "2d fmt" cargo fmt --all --check
step "2e no-default-features" cargo build --locked -p md-cli --no-default-features
step "2f doc" env RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps --document-private-items --all-features
step "2g grouping sha" bash -c 'cd design && sha256sum -c display-grouping-vectors.tsv.sha256'
step "2h shared-ci sha" sha256sum --check --strict ci/shared-ci.sha256
step "2i docs-only test" bash ci/docs-only.test.sh
step "2j code-docs test" bash ci/code-docs-check.test.sh
step "2k code-docs check" bash ci/code-docs-check.sh

# Step 3
step "3 vendor-freshness" bash ci/repro/vendor-freshness.sh

if [ ${#failed[@]} -ne 0 ]; then
  echo "GATE FAILED steps: ${failed[*]}"
  exit 1
fi
echo "GATE OK"
