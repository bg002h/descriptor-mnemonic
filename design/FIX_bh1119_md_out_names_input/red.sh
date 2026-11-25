#!/usr/bin/env bash
# RED/GREEN probe for FIX_bh1119_md_out_names_input (rows n1-n27).
# Usage: red.sh <md binary> <scratch dir, not under /tmp>
# Runs each row as printed in the FIX doc section 4, in a fresh subdirectory,
# and prints: rc, stdout byte count, stderr (first 2 lines), and the state of
# the file the row protects (line count + first line).
# Not covered: Windows/macOS (io-lib same_file limits), the --json envelope
# shape (md prints every error on stderr), the help/manual texts.
set -u
MD=$(realpath "$1")
ROOT=$(realpath "$2")
HERE=$(cd "$(dirname "$0")/../.." && pwd)
FIX="$HERE/crates/md-cli/tests/fixtures/seating/v-b1-wallet.txt"
TPL='wpkh(@0/<0;1>/*)'
DESC="wpkh([73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf/<0;1>/*)"
M=$(grep '^md1' "$FIX")
mkdir -p "$ROOT"

setup() {
  local d="$ROOT/$1"
  mkdir -p "$d" && cd "$d" || exit 9
  find . -mindepth 1 -delete
  printf '%s\n' "$TPL" > tpl.txt
  "$MD" encode "$TPL" > card.txt 2>/dev/null
  printf '%s\n' "$DESC" > desc.txt
  grep '^mk1' "$FIX" > K.txt
  "$MD" descriptor "$M" --from-mk1-file K.txt --emit md1 > V.txt 2>/dev/null
}

state() { # file -> "<lines> lines, first: <line>"
  if [ -e "$1" ]; then
    printf '%s: %s lines, first: %s\n' "$1" "$(wc -l < "$1")" "$(head -c 60 "$1" | head -1)"
  else
    printf '%s: absent\n' "$1"
  fi
}

row() { # id, file-to-show, shell command (run with bash -c in the row dir)
  local id=$1 f=$2 cmd=$3
  setup "$id"
  bash -c "$cmd" > "$ROOT/$id.stdout" 2> "$ROOT/$id.stderr"
  local rc=$?
  # A row whose stdout is redirected inside cmd leaves this file empty.
  printf '== %s: %s\n   rc=%s stdout=%sB\n' "$id" "$cmd" "$rc" "$(wc -c < "$ROOT/$id.stdout")"
  grep -v '^note:\|^  \|^chunk-set-id' "$ROOT/$id.stderr" | head -2 | sed 's/^/   stderr: /'
  printf '   %s\n' "$(state "$f")"
}

export MD M TPL
# X1: --out names an input
row n1 tpl.txt  '"$MD" encode --in tpl.txt --out tpl.txt'
row n2 tpl.txt  'ln tpl.txt hl && "$MD" encode --in tpl.txt --out hl'
row n3 tpl.txt  'ln -s tpl.txt sl && "$MD" encode --in tpl.txt --out sl'
row n4 tpl.txt  '"$MD" encode --in ./tpl.txt --out tpl.txt'
row n5 tpl.txt  '"$MD" encode --json --in tpl.txt --out tpl.txt'
row n6 K.txt    '"$MD" descriptor "$M" --from-mk1-file K.txt --emit md1 --out K.txt'
row n7 K.txt    'ln -s K.txt kl && "$MD" descriptor "$M" --from-mk1-file K.txt --emit md1 --out kl'
row n8 V.txt    '"$MD" descriptor "$M" --from-mk1-file K.txt --emit md1 --group-size 5 --verify-against V.txt --out V.txt'
row n9 tpl.txt  '"$MD" encode --in tpl.txt --out /dev/stdout >>tpl.txt'
# X2: standard output names an input
row n10 tpl.txt  '"$MD" encode --in tpl.txt >>tpl.txt'
row n11 tpl.txt  '"$MD" encode --in tpl.txt >tpl.txt'
row n12 card.txt '"$MD" decode --in card.txt >>card.txt'
row n13 card.txt '"$MD" verify --in card.txt --template "$TPL" >>card.txt'
row n14 card.txt '"$MD" inspect --in card.txt >>card.txt'
row n15 card.txt '"$MD" bytecode --in card.txt >>card.txt'
row n16 card.txt '"$MD" repair --in card.txt >>card.txt'
row n17 desc.txt '"$MD" decompose --in desc.txt >>desc.txt'
row n18 card.txt '"$MD" decode - <card.txt >>card.txt'
row n19 desc.txt '"$MD" decompose - <desc.txt >>desc.txt'
row n20 K.txt    '"$MD" descriptor "$M" --from-mk1-file K.txt >>K.txt'
row n21 K.txt    '"$MD" address "$M" --from-mk1-file K.txt >>K.txt'
row n22 V.txt    '"$MD" descriptor "$M" --from-mk1-file K.txt --verify-against V.txt >>V.txt'
row n23 tpl.txt  '"$MD" encode --json --in tpl.txt --out G >>tpl.txt'
# X3: --out and standard output are one file (md encode --json --out only)
row n24 F        'printf "keep\n" >F && "$MD" encode "$TPL" --json --out F >>F'
row n25 F        'printf "keep\n" >F && "$MD" encode "$TPL" --json --out F >F'
# order: X2 before X1 before X3
row n26 tpl.txt  '"$MD" encode --json --in tpl.txt --out tpl.txt >>tpl.txt'
row n27 card.txt '"$MD" decode - <card.txt >card.txt'
# an absent --in and --out naming one place; first-input order
row n28 x        '"$MD" encode --in x --out x'
row n29 K.txt    '"$MD" descriptor "$M" --from-mk1-file K.txt --emit md1 --verify-against K.txt --out K.txt'
# controls (same rc on base and fixed; nothing refused)
row c1 new.txt  '"$MD" encode --in tpl.txt --out new.txt'
row c2 card.txt '"$MD" decode --in card.txt >out.txt'
row c3 card.txt '"$MD" decode --in card.txt | cat >/dev/null'
row c4 K.txt    '"$MD" descriptor "$M" --from-mk1-file K.txt --emit md1 --out G >>K.txt'
row c5 "$M"     '"$MD" descriptor "$M" --from-mk1-file K.txt --emit md1 --verify-against "$M" --out "$M"'
row c6 tpl.txt  '"$MD" encode "$TPL" --out F >>tpl.txt'
row c7 F        'printf "keep\n" >F && "$MD" encode "$TPL" --out F >>F'
