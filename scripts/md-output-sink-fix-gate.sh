#!/usr/bin/env bash
# Gate for the md output-sink FIX document (design/FIX_*output_sink.md; crash sweep
# F1 family, md).
#
# What it does:
#   1. Extracts the document's blocks (section 3 `diff` blocks, section 4 `diff`
#      rows and the `sh` RED commands) and checks that the code blocks and the rows
#      hash to the sha256 the document states. If the architect's files exist, it also
#      cmp's them byte for byte (fix-with-docfix.diff, rows.diff).
#   2. In its own detached worktree at the base commit: applies the rows, runs every
#      RED command as printed and requires each to FAIL with its `## expect:`
#      substring; the whole-file run must be "11 tests run: 2 passed, 9 failed" and
#      the two guard rows must PASS on the base.
#   3. Self-test: the checker must reject a wrong expectation and a passing run, or
#      the gate dies.
#   4. Applies the code blocks; requires the 11 rows green, the unit test green, the
#      full md-cli suite green twice (943 with --features cli-compiler, 905 with
#      default features), doc tests, fmt, clippy, ci/code-docs-check.sh, and md's CI `cargo doc` job
#      command as ci.yml prints it (RUSTDOCFLAGS=-D warnings), which must exit 0.
#
# What it does NOT cover: the grade's 14 plants and the architect's two (not re-run);
# the macOS, Windows, FreeBSD, musl and other CI legs, nor the workspace-wide test run (the
# suite is -p md-cli); the toolkit manual paragraph; the coordinator's
# mutation-gate.sh; any review. fmt runs on the repo's pinned toolchain (the grade
# ran it on stable).
#
# Usage: scripts/md-output-sink-fix-gate.sh [DOC]   (run from the repo root)
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
REPO=$(git rev-parse --show-toplevel)
DOC=${1:-$(ls "$REPO"/design/FIX_*output_sink.md)}
DOC=$(realpath "$DOC")
BASE=${BASE:-6b70d5d964d9dafcae0c1cfef926a0a7887ec6a9}
W=${W:-/scratch/cache/md-output-sink-gate-wt}
OUT=${OUT:-/scratch/cache/md-output-sink-gate-out}
GRADE=${GRADE:-/scratch/cache/architect-output-sink-ro/md}
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-/scratch/cache/md-output-sink-gate-target}
case "$W$OUT$CARGO_TARGET_DIR" in /tmp*) echo "refusing /tmp" >&2; exit 2;; esac
mkdir -p "$OUT"
die() { echo "GATE FAIL: $*" >&2; exit 1; }
cargo --version | grep -q '^cargo 1\.85\.' || die "cargo is not the 1.85 pin: $(cargo --version)"
R3='r3_a_not_equal_verdict_whose_stdout_is_lost_exits_1_and_keeps_the_verdict_line'
ROWS_FILTER="binary(output_errors) | test(/(^|::)${R3}\$/)"

# 1. extract
python3 -I - "$DOC" "$OUT" <<'PY'
import sys,re,hashlib
doc,out=sys.argv[1:3]
t=open(doc).read()
def section(a,b):
    i=t.index(a); j=t.index(b,i); return t[i:j]
def blocks(s,tag):
    return re.findall(r'(?m)^```'+tag+r'\n(.*?)^```\n',s,re.S)
s3=section('\n## 3. ','\n## 4. ')
s4=section('\n## 4. ','\n## 5. ')
code=''.join(blocks(s3,'diff'))
rows=''.join(blocks(s4,'diff'))
red=''.join(blocks(s4,'sh'))
want=re.search(r'sha256 `([0-9a-f]{64})`, checked',s3).group(1)
wantr=re.search(r'sha256 `([0-9a-f]{64})`\)',s4).group(1)
assert hashlib.sha256(code.encode()).hexdigest()==want,'code blocks differ from the stated sha256'
assert hashlib.sha256(rows.encode()).hexdigest()==wantr,'rows blocks differ from the stated sha256'
assert len(blocks(s3,'diff'))==5 and len(blocks(s4,'diff'))==2
assert len([l for l in red.splitlines() if l.startswith('cargo nextest')])==9
for n,v in (('code.diff',code),('rows.diff',rows),('red.sh',red)):
    open(out+'/'+n,'w').write(v)
PY
if [ -f "$GRADE/fix.diff" ]; then
  cmp "$OUT/code.diff" "$GRADE/fix-with-docfix.diff" || die "code blocks differ from the architect's fix-with-docfix.diff"
  cmp "$OUT/rows.diff" "$GRADE/rows.diff" || die "rows differ from the architect's rows.diff"
  echo "cmp: blocks identical to the architect's files"
else echo "cmp: grade files absent, sha256 only"; fi

# worktree at the base
[ -e "$W/.git" ] || git -C "$REPO" worktree add --detach "$W" "$BASE" >/dev/null
reset() { git -C "$W" reset -q --hard "$BASE"; git -C "$W" clean -qfd -e target; }
reset
git -C "$W" rev-parse HEAD | grep -q "^$BASE" || die "worktree is not at the base"

# checker: 0 only if the run FAILED and its output holds the substring
red_check() { # $1 log, $2 rc, $3 expect
  [ "$2" -ne 0 ] && grep -qF -- "$3" "$1"
}
# 2. RED on the base
git -C "$W" apply "$OUT/rows.diff"
n=0; first_log=; first_rc=
while IFS= read -r line; do
  case "$line" in cargo\ nextest*) ;; *) continue;; esac
  cmd=${line%% ## expect: *}; exp=${line#* ## expect: }
  n=$((n+1)); log="$OUT/red-$n.log"
  rc=0; (cd "$W" && eval "$cmd") >"$log" 2>&1 || rc=$?
  red_check "$log" "$rc" "$exp" || die "RED row $n did not fail for its stated reason: $cmd (rc=$rc, expect: $exp)"
  echo "RED $n ok (rc=$rc): ${cmd#*-E }"
  [ -n "$first_log" ] || { first_log=$log; first_rc=$rc; }
done < "$OUT/red.sh"
[ "$n" -eq 9 ] || die "expected 9 RED commands, ran $n"
rc=0; (cd "$W" && cargo nextest run --locked -j 8 --no-fail-fast -p md-cli -E "$ROWS_FILTER") >"$OUT/red-all.log" 2>&1 || rc=$?
grep -q '11 tests run: 2 passed, 9 failed' "$OUT/red-all.log" || die "whole-file RED is not 2 passed, 9 failed"
grep -q 'PASS .*only_out_rs_names_stdout' "$OUT/red-all.log" || die "the scan guard did not pass on the base"
grep -q 'PASS .*full_stdout_with_nothing_to_print_keeps_the_verbs_code' "$OUT/red-all.log" || die "the nothing-to-print guard did not pass on the base"
echo "RED whole set: $(grep -o '11 tests run: 2 passed, 9 failed' "$OUT/red-all.log"); both guards PASS"
# self-test of the checker
red_check "$first_log" "$first_rc" "NO-SUCH-TEXT-XYZ" && die "self-test: a wrong expectation was accepted"
red_check "$OUT/red-all.log" 0 "11 tests run" && die "self-test: a passing run was accepted as RED"
echo "self-test ok: the checker rejects a broken row"

# 3. GREEN
git -C "$W" apply "$OUT/code.diff"
grep -rq '^### `md-output-sink`' "$W/design" || die "the md-output-sink entry is missing"
run() { # name cmd...
  local name=$1; shift
  (cd "$W" && "$@") >"$OUT/$name.log" 2>&1 || { tail -n 30 "$OUT/$name.log" >&2; die "$name failed"; }
}
run rows cargo nextest run --locked -j 8 -p md-cli -E "$ROWS_FILTER"
grep -q '11 tests run: 11 passed' "$OUT/rows.log" || die "rows are not 11/11"
run unit cargo nextest run --locked -j 8 -p md-cli -E 'test(/^out::tests::/)'
grep -q '2 tests run: 2 passed' "$OUT/unit.log" || die "the out::tests unit test did not run twice and pass"
run suite_cli cargo nextest run --locked -j 8 -p md-cli --features cli-compiler
grep -q '943 tests run: 943 passed, 3 skipped' "$OUT/suite_cli.log" || die "cli-compiler suite is not 943/943: $(grep 'tests run' "$OUT/suite_cli.log")"
run suite_def cargo nextest run --locked -j 8 -p md-cli
grep -q '905 tests run: 905 passed, 3 skipped' "$OUT/suite_def.log" || die "default suite is not 905/905: $(grep 'tests run' "$OUT/suite_def.log")"
run doc cargo test --locked --workspace --doc --all-features
run fmt cargo fmt --all --check
run cidoc env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features
run clippy cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
git -C "$W" add -N .
run docscheck bash ci/code-docs-check.sh
echo "GREEN rows: $(grep -o '11 tests run: 11 passed' "$OUT/rows.log"); unit: $(grep -o '2 tests run: 2 passed' "$OUT/unit.log")"
echo "GREEN suite cli-compiler: $(grep -o '943 tests run: 943 passed, 3 skipped' "$OUT/suite_cli.log")"
echo "GREEN suite default: $(grep -o '905 tests run: 905 passed, 3 skipped' "$OUT/suite_def.log")"
echo "GREEN doc, fmt, cargo doc (CI command), clippy, code-docs check: ok"
reset
echo "GATE PASS"
