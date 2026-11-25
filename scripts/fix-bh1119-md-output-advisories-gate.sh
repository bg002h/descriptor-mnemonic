#!/usr/bin/env bash
# Gate for FIX_bh1119_md_output_advisories (bughunt-1119 bh-md-M5, bh-md-I8), FIX section 8.
# Usage: bash scripts/fix-bh1119-md-output-advisories-gate.sh   (no arguments; run from the repo root;
#   $HOME/.cargo/bin first in PATH; --locked everywhere; every cargo call with CARGO_BUILD_JOBS=6).
#   Every step runs; exit 0 only if all passed, else exit 1 after printing the failed step names.
# Steps: 1 every TRACE row by its exact nextest name (match count 1 each), the two changed tests,
#   then run; 2 the repo CI set (ci.yml); 3 no golden moved against `git merge-base HEAD origin/main`.
# Does NOT cover: the bitcoind-differential, fuzz-smoke, freebsd and musl/aarch64 jobs, the toolkit
#   `manual` job (run in the toolkit PR), the release.
set -u
export CARGO_BUILD_JOBS=6
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
  n=$(cargo nextest list --locked -p "$1" --build-jobs 6 --message-format oneline -E "test(=$2)" 2>/dev/null | wc -l)
  if [ "$n" != 1 ]; then echo "match count for $2 is $n, not 1"; return 1; fi
  cargo nextest run --locked -p "$1" --build-jobs 6 -j 6 -E "test(=$2)"
}

# Step 1
for r in m5_r1_decode_json_keyed_card_is_watch_only \
  m5_r2_bytecode_text_keyed_card_is_watch_only \
  m5_r3_bytecode_json_keyed_card_is_watch_only \
  m5_c1_decode_text_keyed_card_stays_template \
  m5_c2_keyless_card_stays_template \
  m5_r4_keyed_keyless_card_never_says_cannot_spend \
  i8_r1_encode_absolute_timelock_path_names_the_lock \
  i8_r2_encode_relative_timelock_path_names_the_lock \
  i8_r3_encode_hashlock_path_names_the_hash \
  i8_r4_encode_no_condition_path_says_no_secret_no_timelock \
  i8_r5_encode_hash_or_timelock_names_both \
  i8_r6_encode_json_prints_the_same_warning \
  i8_r7_decode_text_timelock_card_names_the_lock \
  i8_r8_decode_json_timelock_card_warns \
  i8_r9_decode_json_hashlock_card_warns \
  i8_r10_decode_no_condition_card_text_and_json \
  i8_r11_encode_help_names_timelock_paths \
  i8_r12_refusal_hint_names_no_preimage \
  i8_r13_relative_time_lock_in_seconds \
  i8_r14_hash_and_timelock_path_says_no_secret_is_not_free \
  i8_r15_hash_only_on_a_keyed_path_is_not_bearer \
  i8_c1_encode_not_needed_note_unchanged \
  i8_c2_decode_json_ordinary_card_has_no_warning; do
  step "1 md-cli $r" named_row md-cli "$r"
done
for r in keyless_advisory::tests::i8_u1_fallback_when_policy_cannot_be_lifted \
  keyless_advisory::tests::i8_u2_fallback_when_every_path_needs_a_key \
  decoding_a_bearer_card_warns_the_restorer \
  experimental_does_not_claim_bearer_access_for_a_wallet_that_needs_no_relaxing; do
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
