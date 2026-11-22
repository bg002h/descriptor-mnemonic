#!/usr/bin/env bash
# Gate for FIX_bh1119_md_slot_bindings (bughunt-1119 row #29), FIX section 7.
# Usage: bash scripts/fix-bh1119-md-slot-bindings-gate.sh   (no arguments; run from the repo root;
#   $HOME/.cargo/bin first in PATH; --locked everywhere). Every step runs; exit 0 only if all passed,
#   else exit 1 after printing the failed step names. There is no quick mode.
# Steps: 1 build md-cli, then every row s1-s17, e1-e6 as a printed command against the built binary
#   (exit code, empty stdout and the exact one-line stderr); 2 the repo CI set (ci.yml).
# On the base (before the fix) step 1 fails 22 of 23 rows (s17 passes).
# Does NOT cover: the toolkit manual edit (separate toolkit change), the mnemonic-gui schema mirror,
#   the macOS/Windows/FreeBSD/musl legs, the WORDING_REVIEW row for texts K, F, E-K, E-F.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(git rev-parse --show-toplevel)" || exit 2
failed=()
step() { local name=$1; shift; echo "=== $name"; "$@" || { echo "FAILED: $name"; failed+=("$name"); }; }

KA=tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M
KB=tpubDC8msFGeGuwnP2xwTZBBZSie1BLgRAkJhgzpFYTdpGgZNzguXQhNDVWp7mJbHJUjQQvV2myLU9dkx67a7VAUnzY7yT7nvhHj7FgS4oNivvq
T='wpkh(@0/<0;1>/*)'
T2='wsh(sortedmulti(1,@0/<0;1>/*,@1/<0;1>/*))'
MD="${CARGO_TARGET_DIR:-target}/debug/md"

kl() { echo "md: MISMATCH: @$1: --key arguments $2 and $3 both bind this slot, with different values; a slot takes exactly one key, and md never picks one for you. Keep the --key @$1=... you mean and remove the other."; }
fl() { echo "md: MISMATCH: @$1: --fingerprint $2 and --fingerprint $3 both bind this slot; a slot takes exactly one master fingerprint, and md never picks one for you. Keep the one you mean and remove the other."; }
ekl() { echo "md: @$1: --key arguments $2 and $3 bind this slot to the same value; give each slot's --key once."; }
efl() { echo "md: @$1: --fingerprint $2 is given more than once for this slot; give each slot's --fingerprint once."; }

rows() {
  local bad=0
  row() { # name rc expected-stderr args...
    local name=$1 rc=$2 exp=$3; shift 3
    local out err code ef
    ef=$(mktemp)
    out=$("$MD" "$@" --network signet 2>"$ef"); code=$?
    err=$(cat "$ef"); rm -f "$ef"
    if [ "$code" = "$rc" ] && [ -z "$out" ] && [ "$err" = "$exp" ]; then echo "PASS $name"
    else echo "FAIL $name (rc $code, stdout [$out], stderr [$err])"; bad=1; fi
  }
  mapfile -t CARD < <("$MD" encode "$T" --key "@0=$KA" --network signet)
  row s1 1 "$(kl 0 1 2)" address --template "$T" --key "@0=$KA" --key "@0=$KB"
  row s2 1 "$(kl 0 1 2)" address --template "$T" --key "@0=$KB" --key "@0=$KA"
  row s3 1 "$(kl 0 1 2)" descriptor --template "$T" --key "@0=$KA" --key "@0=$KB"
  row s4 1 "$(kl 0 1 2)" descriptor --template "$T" --key "@0=$KB" --key "@0=$KA"
  row s5 1 "$(fl 0 11223344 aabbccdd)" descriptor --template "$T" --key "@0=$KA" --fingerprint @0=11223344 --fingerprint @0=aabbccdd
  row s6 1 "$(fl 0 aabbccdd 11223344)" descriptor --template "$T" --key "@0=$KA" --fingerprint @0=aabbccdd --fingerprint @0=11223344
  row s7 1 "$(fl 0 11223344 aabbccdd)" address --template "$T" --key "@0=$KA" --fingerprint @0=11223344 --fingerprint @0=aabbccdd
  row s8 1 "$(kl 0 1 2)" descriptor --template "$T" --key "@0=[73c5da0a/84'/1'/0']$KA" --key "@0=[73c5da0a/84'/1'/1']$KA"
  row s9 1 "$(kl 0 1 2)" descriptor --template "$T" --key "@0=[73c5da0a/84'/1'/0']$KA" --key "@0=[aabbccdd/84'/1'/0']$KA"
  row s10 1 "$(kl 0 1 2)" descriptor --template "$T" --key "@0=$KA" --key "@0=[73c5da0a/84'/1'/0']$KA"
  row s11 1 "$(kl 0 1 2)" encode "$T" --key "@0=$KA" --key "@0=$KB"
  row s12 1 "$(fl 0 11223344 aabbccdd)" encode "$T" --key "@0=$KA" --fingerprint @0=11223344 --fingerprint @0=aabbccdd
  row s13 1 "$(kl 0 1 2)" verify "${CARD[@]}" --template "$T" --key "@0=$KA" --key "@0=$KB"
  row s14 1 "$(fl 0 11223344 aabbccdd)" verify "${CARD[@]}" --template "$T" --key "@0=$KA" --fingerprint @0=11223344 --fingerprint @0=aabbccdd
  row s15 1 "$(kl 1 2 3)" descriptor --template "$T2" --key "@0=$KA" --key "@1=$KB" --key "@1=$KA" --key "@0=$KB"
  row s16 1 "$(kl 0 1 2)" descriptor --template "$T" --key "@0=$KA" --key "@0=$KB" --fingerprint @0=11223344 --fingerprint @0=aabbccdd
  local o
  o=$("$MD" descriptor --template "$T2" --key "@0=$KA" --key "@1=$KB" --network signet 2>&1); rc=$?
  if [ "$rc" = 0 ] && [[ "$o" == *'#5zanlgag'* ]]; then echo "PASS s17"; else echo "FAIL s17 ($rc: $o)"; bad=1; fi
  row e1 2 "$(ekl 0 1 2)" descriptor --template "$T" --key "@0=$KA" --key "@0=$KA"
  row e2 2 "$(ekl 0 1 2)" encode "$T" --key "@0=$KA" --key "@0=$KA"
  row e3 2 "$(efl 0 11223344)" descriptor --template "$T" --key "@0=$KA" --fingerprint @0=11223344 --fingerprint @0=11223344
  row e4 2 "$(efl 0 11223344)" encode "$T" --key "@0=$KA" --fingerprint @0=11223344 --fingerprint @0=11223344
  row e5 2 "$(efl 0 aabbccdd)" descriptor --template "$T" --key "@0=$KA" --fingerprint @0=AABBCCDD --fingerprint @0=0xaabbccdd
  row e6 2 "$(ekl 0 1 2)" descriptor --template "$T" --key "@0=[73c5da0a/84'/1'/0']$KA" --key "@0=[73C5DA0A/84h/1h/0h]$KA"
  return $bad
}

step "1a build md-cli" cargo build --locked -p md-cli
step "1b rows s1-s17 e1-e6" rows

step "2a nextest" cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast --build-jobs 6 -j 6
step "2b doctests" cargo test --locked --workspace --doc --all-features
step "2c clippy" cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
step "2d fmt" cargo fmt --all --check
step "2e no-default-features" cargo build --locked -p md-cli --no-default-features
step "2f doc" env RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps --document-private-items --all-features
step "2g code-docs check" bash ci/code-docs-check.sh

if [ ${#failed[@]} -ne 0 ]; then echo "GATE FAILED steps: ${failed[*]}"; exit 1; fi
echo "GATE OK"
