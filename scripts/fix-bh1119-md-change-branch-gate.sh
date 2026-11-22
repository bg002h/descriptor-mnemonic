#!/usr/bin/env bash
# Gate for FIX_bh1119_md_change_branch (bughunt-1119 row #30), FIX section 7.
# Usage: bash scripts/fix-bh1119-md-change-branch-gate.sh   (no arguments; run from the repo root;
#   $HOME/.cargo/bin first in PATH; --locked everywhere). Every step runs; exit 0 only if all
#   passed, else exit 1 after printing the failed step names. There is no quick mode.
# Steps: 1 every TRACE row by its exact nextest name (match count 1 each), then run;
#   2 the repo CI set (ci.yml); 3 no golden moved against `git merge-base HEAD origin/main`.
# Does NOT cover: the bitcoind-differential, fuzz-smoke, freebsd and musl/aarch64 jobs, the
#   toolkit-manual PR, the release.
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

# Step 1
for r in r4_to_miniscript_chain1_without_multipath_refused c6_to_miniscript_override_change_chain_still_renders; do
  step "1 md-codec $r" named_row md-codec "derive::tests::$r"
done
for r in r1_descriptor_change_on_single_chain_wallet_refused \
  r2_descriptor_chain2_on_single_chain_wallet_refused \
  r3_descriptor_change_json_on_single_chain_wallet_refused \
  r5_descriptor_chain1_hardened_wildcard_single_chain_refused \
  c1_descriptor_chain0_on_single_chain_wallet_unchanged \
  c2_descriptor_change_on_mixed_wallet_follows_address_rule \
  c3_descriptor_chain2_counts_per_slot_override \
  c4_address_change_on_single_chain_wallet_unchanged \
  c5_descriptor_chain2_on_two_branch_wallet_unchanged \
  q1_r1_descriptor_single_path_no_chain_names_no_form \
  q1_r2_descriptor_single_path_chain0_same_verdict_as_no_chain \
  q1_r3_descriptor_single_alt_multipath_names_no_form \
  q1_r4_every_printed_chain_remedy_runs \
  q1_c1_descriptor_multipath_no_chain_unchanged \
  q1_c2_descriptor_mixed_and_override_multipath_keep_multipath_form; do
  step "1 md-cli $r" named_row md-cli "$r"
done

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
