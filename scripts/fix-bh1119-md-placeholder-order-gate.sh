#!/usr/bin/env bash
# Gate for FIX_bh1119_md_placeholder_order (bughunt-1119 row #5), FIX section 8.
# Usage: bash scripts/fix-bh1119-md-placeholder-order-gate.sh   (no arguments; run from the repo root;
#   $HOME/.cargo/bin first in PATH; --locked everywhere). Every step runs; exit 0 only if all
#   passed, else exit 1 after printing the failed step names. There is no quick mode.
# Steps: 1 E-row script on a fresh md-cli build; 2 the named library/unit rows (match count 1 each);
#   3 the repo CI set (ci.yml); 4 no golden moved against `git merge-base HEAD origin/main`
#   (the default branch is main).
# Does NOT cover: the bitcoind-differential and fuzz-smoke workflows, the freebsd and musl/aarch64
#   jobs, the consumer pin bumps, the release commit.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(git rev-parse --show-toplevel)" || exit 2
failed=()
step() { # $1 name, rest: command
  local name=$1; shift
  echo "=== $name"
  "$@" || { echo "FAILED: $name"; failed+=("$name"); }
}
sh_step() { local name=$1; shift; step "$name" bash -c "$*"; }

# Step 1
step "1a build md-cli" cargo build --locked -p md-cli
step "1b E rows" bash scripts/fix-bh1119-md-placeholder-order-gate-erows.sh "${CARGO_TARGET_DIR:-target}/debug/md"

# Step 2: crate|filter pairs (nextest test names)
named_row() { # $1 crate, $2 test name
  local n
  n=$(cargo nextest list --locked -p "$1" --message-format oneline -E "test(=$2)" 2>/dev/null | wc -l)
  if [ "$n" != 1 ]; then echo "match count for $2 is $n, not 1"; return 1; fi
  cargo nextest run --locked -p "$1" -E "test(=$2)"
}
step "2 e12" named_row md-codec e12_c2_identities_follow_the_canonical_spelling
step "2 e14" named_row md-codec e14_i12_error_leaves_descriptor_unchanged
step "2 e15" named_row md-codec e15_i12_repair_and_retry_matches_one_shot
step "2 e18" named_row md-codec e18_c2_duplicate_override_refused
step "2 permute" named_row md-codec canonicalize::tests::permute_with_use_site_path_overrides
step "2 short-divergent" named_row md-codec canonicalize::tests::canonicalize_short_divergent_returns_typed_error
step "2 e13" named_row md-cli seat::compose::tests::e13_i2_verdict_either_order

# Step 3
step "3a nextest" cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast --build-jobs 6 -j 6
step "3b doctests" cargo test --locked --workspace --doc --all-features
step "3c clippy" cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
step "3d fmt" cargo fmt --all --check
step "3e no-default-features" cargo build --locked -p md-cli --no-default-features
step "3f doc" env RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps --document-private-items --all-features
step "3g grouping sha" bash -c 'cd design && sha256sum -c display-grouping-vectors.tsv.sha256'
step "3h shared-ci sha" sha256sum --check --strict ci/shared-ci.sha256
step "3i docs-only test" bash ci/docs-only.test.sh
step "3j code-docs test" bash ci/code-docs-check.test.sh
step "3k code-docs check" bash ci/code-docs-check.sh

# Step 4
step "4 goldens" bash -c 'git diff --exit-code "$(git merge-base HEAD origin/main)" -- crates/md-codec/tests/vectors design/display-grouping-vectors.tsv'

if [ ${#failed[@]} -ne 0 ]; then
  echo "GATE FAILED steps: ${failed[*]}"
  exit 1
fi
echo "GATE OK"
