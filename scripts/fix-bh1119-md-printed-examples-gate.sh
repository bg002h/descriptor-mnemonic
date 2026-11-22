#!/usr/bin/env bash
# Gate for FIX_bh1119_md_printed_examples (bughunt-1119 row #41), FIX section 7.
# Usage: bash scripts/fix-bh1119-md-printed-examples-gate.sh   (no arguments; run from the repo root;
#   $HOME/.cargo/bin first in PATH; --locked everywhere). Every step runs; exit 0 only if all
#   passed, else exit 1 after printing the failed step names. There is no quick mode.
# Steps: 1 every TRACE row by its exact nextest name (match count 1 each), then run; 1d the
#   README doctests (2 passed); 2 the repo CI set (ci.yml); 3 no golden moved against
#   `git merge-base HEAD origin/main`.
# Does NOT cover: the bitcoind-differential, fuzz-smoke, freebsd, windows/macos and
#   musl/aarch64 jobs, the toolkit-manual PR, the release.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(git rev-parse --show-toplevel)" || exit 2
failed=()
step() { # $1 name, rest: command
  local name=$1; shift
  echo "=== $name"
  "$@" || { echo "FAILED: $name"; failed+=("$name"); }
}

named_row() { # $1 crate, $2 test name
  local n
  n=$(cargo nextest list --locked -p "$1" --message-format oneline -E "test(=$2)" 2>/dev/null | wc -l)
  if [ "$n" != 1 ]; then echo "match count for $2 is $n, not 1"; return 1; fi
  cargo nextest run --locked -p "$1" -E "test(=$2)"
}

readme_doctests() {
  local out
  out=$(cargo test --locked -p md-codec --doc -- readme_doctests::ReadmeDoctests 2>&1) || { echo "$out"; return 1; }
  echo "$out"
  echo "$out" | grep -qF 'test result: ok. 2 passed'
}

# Step 1
step "1 md-codec i11_readme_dependency_pin_matches_crate_version" named_row md-codec \
  "readme_doctests::tests::i11_readme_dependency_pin_matches_crate_version"
for r in i10_encode_help_example_runs_as_printed \
  i10_decompose_help_example_runs_as_printed \
  c1_decode_help_example_runs_as_printed \
  f1_descriptor_help_example_runs_as_printed \
  f1_address_help_example_runs_as_printed \
  f1_repair_help_examples_run_as_printed; do
  step "1 md-cli $r" named_row md-cli "$r"
done
step "1d readme doctests" readme_doctests

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
step "3 goldens" bash -c 'git diff --exit-code "$(git merge-base HEAD origin/main)" -- crates/md-codec/tests/vectors design/display-grouping-vectors.tsv'

if [ ${#failed[@]} -ne 0 ]; then
  echo "GATE FAILED steps: ${failed[*]}"
  exit 1
fi
echo "GATE OK"
