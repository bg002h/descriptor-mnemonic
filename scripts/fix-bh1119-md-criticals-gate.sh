#!/usr/bin/env bash
# Gate for FIX_bh1119_md_decompose_routes (row #4). Signet keys only.
# Usage: bash scripts/fix-bh1119-md-criticals-gate.sh <md-binary> [mk-binary] [--ci]
#   rows: runs the TRACE rows d1-d12 (+d3h) against <md-binary> and the installed
#         `mk`, recording `mk --version`; route-2 "runs and reproduces" is checked
#         HERE (nextest never spawns mk; CI has no mk). Every row must PASS.
#   --ci: then the repo CI set (nextest, doc tests, clippy, fmt, no-default-features
#         build, rustdoc -D warnings, ci/code-docs-check.sh).
# Shown RED once on the base (see the build report).
# Does NOT cover: the toolkit manual edit (separate toolkit change at the pin bump),
# macOS/Windows/FreeBSD/musl legs, the WORDING_REVIEW row.
set -u
MD=$(realpath "$1"); shift
MK=""; CI=0
for a in "$@"; do case "$a" in --ci) CI=1;; *) MK=$a;; esac; done
MK=${MK:-$(command -v mk)}
[ -x "$MK" ] || { echo "no mk on PATH (route-2 rows need it)"; exit 2; }
echo "mk: $("$MK" --version)   md: $("$MD" --version)"
W=$(mktemp -d -p /scratch/cache md-bh4-gate.XXXXXX); cd "$W" || exit 2
fails=0
pass() { echo "PASS $1"; }
fail() { echo "FAIL $1: $2"; fails=$((fails+1)); }
KA=tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ
M0=tpubD6NzVbkrYhZ4XYa9MoLt4BiMZ4gkt2faZ4BcmKu2a9te4LDpQmvEz2L2yDERivHxFPnxXXhqDRkUNnQCpZggCyEZLBktV7VaSmwayqMJy1s
K2=tpubDB5GSFrKWykZsJgwdd9ViGUECU8AQvTxig4iZonrS5BJ2k9V4ucfHifnXTNfRExUaDsLvZedPfbEN2iphbq11CMDe4WJQAvQRqXaGJuwhwk
K1=tpubD97UxEEVXiRvic42NcHLjLAWv5L3CEkuj587dZtEmb4T4Da74DpjbC5tjm7qBq3G4LxjU9SYuTv7edTvTefctXWPkvgSn7MHUmiPCsSyzyJ
cat > norm.py <<'PY'
import sys, re, hashlib
A = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
def dec(s):
    n = 0
    for c in s: n = n * 58 + A.index(c)
    return n.to_bytes(82, 'big')
def enc(b):
    n = int.from_bytes(b, 'big'); s = ''
    while n: n, r = divmod(n, 58); s = A[r] + s
    return s
def norm(x):
    b = bytearray(dec(x)[:78]); b[5:9] = b'\0\0\0\0'
    return enc(bytes(b) + hashlib.sha256(hashlib.sha256(bytes(b)).digest()).digest()[:4])
t = [re.sub(r'[xt]pub[1-9A-HJ-NP-Za-km-z]{100,112}', lambda m: norm(m.group(0)),
            open(f).read().strip().split('#')[0]) for f in sys.argv[1:3]]
sys.exit(0 if t[0] == t[1] else 1)
PY
route_ok() { # $1 descriptor, $2 commands output; echoes problems
  local d=$1 out=$2 p=""
  "$MD" decompose "$d" --network signet --emit descriptor > in.desc 2>/dev/null
  awk '/^md encode .*\\$/ && !b {b=1} b {print} b && !/\\$/ {exit}' "$out" > r1.sh
  if grep -q -- "--key '" r1.sh; then
    PATH="$(dirname "$MD"):$PATH" bash r1.sh > r1.cards 2> r1.err || p="$p route1-fails-as-printed($(grep -v '^note' r1.err | head -1));"
    if [ -s r1.cards ] && ! grep -q route1 <<<"$p"; then
      "$MD" descriptor --network signet $(cat r1.cards) 2>/dev/null | tail -1 > r1.desc
      python3 -I norm.py r1.desc in.desc || p="$p route1-mints-a-different-wallet;"
    fi
  fi
  if grep -q '^mk encode' "$out"; then
    awk '/^md encode/ {buf=""} {buf=buf $0 "\n"} /--out policy.md1/ {printf "%s", buf; exit}' "$out" > r2.sh
    awk '/--out policy.md1/ {f=1} f' "$out" | tail -n +2 >> r2.sh
    rm -f policy.md1 keys.txt
    PATH="$(dirname "$MD"):$(dirname "$MK"):$PATH" bash r2.sh > r2.cards 2> r2.err || p="$p route2-fails-as-printed($(grep -m1 -i 'error' r2.err));"
    if ! grep -q route2 <<<"$p"; then
      "$MD" descriptor --network signet "$(cat policy.md1)" --from-mk1 $(grep '^mk1' r2.cards | tr -d ' ') 2>/dev/null | tail -1 > r2.desc
      python3 -I norm.py r2.desc in.desc || p="$p route2-mints-a-different-wallet;"
    fi
  fi
  echo "$p"
}
drow() { # $1 row, $2 descriptor, $3 expect: refuse | r2only | both ; $4 emit (default commands)
  local emit=${4:-commands}
  "$MD" decompose "$2" --network signet --emit "$emit" > out.txt 2> err.txt; local rc=$?
  if [ "$emit" != commands ]; then
    if [ $rc -ne 0 ] && [ ! -s out.txt ]; then pass "$1"; else fail "$1" "rc=$rc stdout=$(head -c 80 out.txt)"; fi
    return
  fi
  local probs; probs=$(route_ok "$2" out.txt)
  local has1=no has2=no
  grep -q -- "--key '" out.txt && has1=yes; grep -q '^mk encode' out.txt && has2=yes
  case "$3" in
    refuse) if [ $rc -ne 0 ] && [ ! -s out.txt ]; then pass "$1"; else fail "$1" "rc=$rc route1=$has1 route2=$has2 $probs"; fi ;;
    r2only) if [ $rc -eq 0 ] && [ $has1 = no ] && [ $has2 = yes ] && grep -q 'route 1 withheld' out.txt && [ -z "$probs" ]; then pass "$1"; else fail "$1" "rc=$rc route1=$has1 route2=$has2 $probs"; fi ;;
    both)   if [ $rc -eq 0 ] && [ $has1 = yes ] && [ $has2 = yes ] && [ -z "$probs" ]; then pass "$1"; else fail "$1" "rc=$rc route1=$has1 route2=$has2 $probs"; fi ;;
  esac
}
O="[73c5da0a/48'/1'/0'/2']"
drow d1_c1_fixed_step_use_site "wpkh($O$KA/0/*)" refuse
drow d2_no_wildcard "wpkh($O$KA)" refuse
drow d3_fixed_step_then_multipath "wpkh($O$KA/7/<0;1>/*)" refuse
drow d1t_c1_fixed_step_template "wpkh($O$KA/0/*)" refuse template
drow d2t_no_wildcard_template "wpkh($O$KA)" refuse template
drow d3t_fixed_step_then_multipath_template "wpkh($O$KA/7/<0;1>/*)" refuse template
drow d3u_trailing_step_after_multipath_template "wpkh($O$KA/<0;1>/5/*)" refuse template
drow d3h_hardened_multipath_alt_template "wpkh($O$KA/<0h;1h>/*)" refuse template
drow d1a_c1_fixed_step_all "wpkh($O$KA/0/*)" refuse all
drow d4_i9_master_key "wpkh([73c5da0a]$M0/<0;1>/*)" refuse
drow d5_depth2_route1_withheld "wpkh([73c5da0a/84'/1']$K2/<0;1>/*)" r2only
drow d10_depth1_wrong_fingerprint "wpkh([deadbeef/84']$K1/<0;1>/*)" refuse
drow d10b_depth1_right_fingerprint "wpkh([73c5da0a/84']$K1/<0;1>/*)" r2only
drow d11_origin_less_fixed_step "wpkh($KA/0/*)" refuse
drow d11_origin_less_fixed_step_template "wpkh($KA/0/*)" refuse template
drow d9_origin_less_unchanged "wpkh($KA/<0;1>/*)" refuse
drow d6_control_standard "wpkh($O$KA/<0;1>/*)" both
drow d7_control_other_multipath "wpkh($O$KA/<2;3>/*)" both
drow d8_control_bare_wildcard "wpkh($O$KA/*)" both
"$MD" decompose "wpkh($O$KA/<0;1>/*)" --network signet --emit commands > d6.out 2>/dev/null
[ "$(sha256sum < d6.out | cut -c1-16)" = db14761d61c955d2 ] && pass d6b_control_stdout_unchanged || fail d6b_control_stdout_unchanged "sha=$(sha256sum < d6.out | cut -c1-16)"
# d12: templates that must still print, byte for byte.
d12() { local got; got=$("$MD" decompose "$1" --network signet --emit template 2>/dev/null); [ "$got" = "$2" ] && pass "d12 $2" || fail "d12 $2" "got=$got"; }
d12 "wpkh($O$KA/<2;3>/*)" "wpkh(@0/48'/1'/0'/2'/<2;3>/*)"
d12 "wpkh($O$KA/*)" "wpkh(@0/48'/1'/0'/2'/*)"
d12 "wpkh($O$KA/<0;1;2>/*)" "wpkh(@0/48'/1'/0'/2'/<0;1;2>/*)"
d12 "wpkh([73c5da0a]$M0/<0;1>/*)" "wpkh(@0/<0;1>/*)"
echo "rows failed: $fails (workdir $W)"
[ $fails -eq 0 ] || exit 1
if [ $CI -eq 1 ]; then
  cd "$(dirname "$0")/.." || exit 2
  export PATH="$HOME/.cargo/bin:$PATH"
  set -e
  cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast --build-jobs 6 -j 6
  cargo test --locked --workspace --doc --all-features
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo fmt --all --check
  cargo build --locked -p md-cli --no-default-features
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features
  bash ci/code-docs-check.sh
fi
