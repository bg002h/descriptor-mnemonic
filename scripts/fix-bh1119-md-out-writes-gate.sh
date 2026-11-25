#!/usr/bin/env bash
# Gate for FIX_bh1119_md_out_writes (bughunt-1119 bh-md-C4, bh-md-I1, bh-md-I7), FIX section 7.
# Usage: bash scripts/fix-bh1119-md-out-writes-gate.sh   (no arguments; run from the repo root;
#   $HOME/.cargo/bin first in PATH; --locked everywhere; every cargo call with CARGO_BUILD_JOBS=6).
#   MD=<path> runs step 1 against that md binary instead of building one (used to show it RED on the
#   base). Every step runs; exit 0 only if all passed, else exit 1 after printing the failed step names.
# Steps: 1 build md-cli, then rows o1 o3 o4 o6 o8 o10 o11 o13 o15 as printed commands against the built
#   md (exit code, stdout, exact stderr, file bytes; FSIZE rows by prlimit); 2 the repo CI set (ci.yml);
#   3 the vendor-freshness gate (network for its check 4).
# Does NOT cover: the toolkit manual edit (M1, M2; separate toolkit change), the man-pages and release
#   workflows, the macOS/Windows/FreeBSD/musl legs, the WORDING_REVIEW rows for H1, M1, M2.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=6
cd "$(git rev-parse --show-toplevel)" || exit 2
failed=()
step() { local name=$1; shift; echo "=== $name"; "$@" || { echo "FAILED: $name"; failed+=("$name"); }; }

T='wpkh(@0/<0;1>/*)'
P='pkh(@0/<0;1>/*)'
W='wsh(pk(@0/<0;1>/*))'
CARD=$'md1yqpqqxqq8xtwhw4xwn4qh'
NOTE='note: stdout is a keyless descriptor template (no keys)'
CODEC='md: codec error: non-canonical wrapper requires explicit origin for @0, but none provided'

if [ -n "${MD:-}" ]; then
  build() { echo "using MD=$MD"; }
else
  MD="${CARGO_TARGET_DIR:-target}/debug/md"
  build() { cargo build --locked -p md-cli --bin md; }
fi

rows() {
  local bad=0 D out err code ef
  D=$(mktemp -d)
  ef="$D/stdout.txt"
  # run <cmd...>: sets out, err, code
  # stderr goes through a pipe and stdout to a file read after: a file on stderr would itself hit prlimit
  run() { err=$("$@" 2>&1 >"$ef"); code=$?; out=$(cat "$ef"); }
  fs() { local lim=$1; shift; bash -c "trap '' XFSZ; exec prlimit --fsize=$lim \"\$@\"" _ "$MD" "$@"; }
  chk() { # name ok
    if [ "$2" = 1 ]; then echo "PASS $1"; else echo "FAIL $1 (rc $code, stdout [$out], stderr [$err])"; bad=1; fi
  }
  local F ok exp
  # o1
  F="$D/o1"; "$MD" encode "$T" --out "$F" 2>/dev/null
  run fs 8 encode "$P" --out "$F"
  exp="md: cannot write the output: $F: File too large (os error 27). $F is unchanged."
  ok=0; [ "$code" = 1 ] && [ -z "$out" ] && [ "$err" = "$exp" ] && cmp -s "$F" <(printf '%s\n' "$CARD") && ok=1
  chk o1 $ok
  # o3
  F="$D/nodir/x.md1"
  run "$MD" encode "$T" --out "$F"
  exp="md: cannot write the output: $F: No such file or directory (os error 2). Nothing was written."
  ok=0; [ "$code" = 1 ] && [ -z "$out" ] && [ "$err" = "$exp" ] && ok=1
  chk o3 $ok
  # o4 (skipped for root)
  if [ "$(id -u)" = 0 ]; then echo "SKIP o4 (euid 0)"; else
    mkdir "$D/ro"; F="$D/ro/F"; "$MD" encode "$T" --out "$F" 2>/dev/null; chmod 0500 "$D/ro"
    run "$MD" encode "$P" --out "$F"
    chmod 0700 "$D/ro"
    exp="md: cannot write the output: $F: Permission denied (os error 13). $F is unchanged."
    ok=0; [ "$code" = 1 ] && [ -z "$out" ] && [ "$err" = "$exp" ] && cmp -s "$F" <(printf '%s\n' "$CARD") && ok=1
    chk o4 $ok
  fi
  # o6
  F="$D/o6"
  run "$MD" encode "$T" --json --out "$F"
  local plain; plain=$("$MD" encode "$T" --json 2>/dev/null)
  ok=0; [ "$code" = 0 ] && [ "$out" = "$plain" ] && [ "$err" = "$NOTE" ] && cmp -s "$F" <(printf '%s\n' "$CARD") 2>/dev/null \
    && [ "$(stat -c %a "$F" 2>/dev/null)" = 600 ] && ok=1
  chk o6 $ok
  # o8
  F="$D/nodir/x.md1"
  run "$MD" encode "$T" --json --out "$F"
  exp="md: cannot write the output: $F: No such file or directory (os error 2). Nothing was written."
  ok=0; [ "$code" = 1 ] && [ -z "$out" ] && [ "$err" = "$exp" ] && ok=1
  chk o8 $ok
  # o10
  F="$D/o10"; printf 'PRIOR\n' > "$F"
  run "$MD" encode "$W" --policy-id-fingerprint --out "$F"
  ok=0; [ "$code" = 1 ] && [ -z "$out" ] && [ "$err" = "$CODEC" ] && [ "$(cat "$F")" = PRIOR ] && ok=1
  chk o10 $ok
  # o11
  run "$MD" encode "$W" --policy-id-fingerprint
  ok=0; [ "$code" = 1 ] && [ -z "$out" ] && [ "$err" = "$CODEC" ] && ok=1
  chk o11 $ok
  # o13
  run "$MD" encode "$W" --policy-id-fingerprint --force-chunked
  ok=0; [ "$code" = 1 ] && [ -z "$out" ] && [ "$err" = "$CODEC" ] && ok=1
  chk o13 $ok
  # o15 (Linux: /dev/stdout appended onto a file, RLIMIT_FSIZE 0)
  F="$D/s.txt"; printf 'OLD BACKUP\n' > "$F"
  err=$(bash -c "trap '' XFSZ; exec prlimit --fsize=0 \"\$0\" encode \"\$1\" --out /dev/stdout" "$MD" "$T" 2>&1 >>"$F"); code=$?; out=""
  exp="md: cannot write the output: /dev/stdout: File too large (os error 27)"
  ok=0; [ "$code" = 1 ] && [ "$err" = "$exp" ] && [ "$(cat "$F")" = "OLD BACKUP" ] && ok=1
  chk o15 $ok
  rm -r "$D"
  return $bad
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
