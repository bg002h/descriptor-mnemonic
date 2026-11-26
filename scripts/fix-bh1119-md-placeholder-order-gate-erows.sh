#!/usr/bin/env bash
# E rows of FIX_bh1119_md_placeholder_order (bughunt-1119 row #5): placeholder order (C2) and
# --verify-against numbering (I2). Each row asserts the FIXED behaviour; on the unfixed base the
# defect rows print FAIL, control rows print PASS on both.
# Usage: fix-bh1119-md-placeholder-order-gate-erows.sh <md-binary>   Signet keys only.
# Does NOT cover: the library rows e12-e15, e18 and the unit tests (cargo, FIX section 8), CI.
set -u
MD=$(realpath "$1")
W=$(mktemp -d -p /scratch/cache md-bh-red.XXXXXX); cd "$W" || exit 2
fails=0
pass() { echo "PASS $1"; }
fail() { echo "FAIL $1: $2"; fails=$((fails+1)); }
KA=tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ
KB=tpubDEYM1BmQ5rp2PWKvCgvQxNeUrEv8gu5819xRdmu6S23fYpS8x2icwAeoVaBTLyN3fGWJQcWoaiKMduTXWKtG9bXNpVrZPRF7XVxrANtAEcR
M0=tpubD6NzVbkrYhZ4XYa9MoLt4BiMZ4gkt2faZ4BcmKu2a9te4LDpQmvEz2L2yDERivHxFPnxXXhqDRkUNnQCpZggCyEZLBktV7VaSmwayqMJy1s
K2=tpubDB5GSFrKWykZsJgwdd9ViGUECU8AQvTxig4iZonrS5BJ2k9V4ucfHifnXTNfRExUaDsLvZedPfbEN2iphbq11CMDe4WJQAvQRqXaGJuwhwk
# E rows: md-codec placeholder order.
erow() { # $1 row, $2 template, $3 expected card
  local c; c=$("$MD" encode "$2" 2>/dev/null); local rc=$?
  "$MD" decode $c > /dev/null 2> dec.err; local drc=$?
  if [ $rc -eq 0 ] && [ $drc -eq 0 ] && [ "$c" = "$3" ]; then pass "$1"; else fail "$1" "enc=$rc card=$c decode=$drc $(head -1 dec.err)"; fi
}
erow e1_c2_two_slots 'wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))' md1yppq2zcyxppgzcuqqcr8qpnypdll2zu
erow e3_c2_three_slots 'wsh(multi(2,@1/<2;3>/*,@2/<2;3>/*,@0/<0;1>/*))' md1yzpq2zcyxpzrq9j5qqcsv998hnqcexfa
erow e4_c2_baseline_moves 'wsh(multi(2,@1/<0;1>/*,@0/<2;3>/*))' md1yppqqxppsg2qknq2zc2ktzhwekmddzh
erow e9_control_canonical 'wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))' md1yppq2zcyxppgzcuqqcr8qpnypdll2zu
erow e10_control_noncanonical_same_use_site 'wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))' md1yppqqxppsg2vlumagltz27le
# E2: --force-chunked, compared with the first-occurrence spelling's chunks.
a=$("$MD" encode --force-chunked 'wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))' 2>/dev/null)
b=$("$MD" encode --force-chunked 'wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))' 2>/dev/null)
"$MD" decode $a > /dev/null 2> dec.err; drc=$?
[ $drc -eq 0 ] && [ "$a" = "$b" ] && pass e2_c2_force_chunked || fail e2_c2_force_chunked "decode=$drc $(head -1 dec.err) same=$([ "$a" = "$b" ] && echo yes || echo no)"
# E5: keyed, with origins; must equal the first-occurrence spelling (keys swapped).
FP="--fingerprint @0=73c5da0a --fingerprint @1=73c5da0a --network signet"
a=$("$MD" encode "wsh(multi(2,@1/48'/1'/1'/2'/<2;3>/*,@0/48'/1'/0'/2'/<0;1>/*))" --key "@0=$KA" --key "@1=$KB" $FP 2>/dev/null)
b=$("$MD" encode "wsh(multi(2,@0/48'/1'/1'/2'/<2;3>/*,@1/48'/1'/0'/2'/<0;1>/*))" --key "@1=$KA" --key "@0=$KB" $FP 2>/dev/null)
"$MD" decode $a > /dev/null 2> dec.err; drc=$?
addr=$("$MD" address --network signet $a 2>/dev/null | head -1)
[ $drc -eq 0 ] && [ "$a" = "$b" ] && [ "$addr" = tb1q0v66c8vdpjv67fx77penkcx6lealfln60ey7vjte5evmdtc378uq44nq3u ] && pass e5_c2_keyed || fail e5_c2_keyed "decode=$drc $(head -1 dec.err) same=$([ "$a" = "$b" ] && echo yes || echo no) addr=$addr"
# E6-E8: I2, a template's own fresh card verifies spend-equal.
vrow() { # $1 row, $2 template
  rm -f own.md1
  "$MD" encode "$2" --key "@0=$KA" --key "@1=$KB" --network signet --out own.md1 2>/dev/null || { fail "$1" "fixture encode failed"; return; }
  "$MD" descriptor --template "$2" --key "@0=$KA" --key "@1=$KB" --network signet --verify-against own.md1 > /dev/null 2> v.err; local rc=$?
  [ $rc -eq 0 ] && pass "$1" || fail "$1" "rc=$rc $(grep -- '--verify-against' v.err | head -1 | cut -c1-90)"
}
vrow e6_i2_multi_noncanonical 'wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))'
vrow e7_i2_sortedmulti_noncanonical 'wsh(sortedmulti(2,@1/<0;1>/*,@0/<0;1>/*))'
vrow e8_i2_control_canonical 'wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))'
vrow e11_i2_divergent_use_sites 'wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))'
KC=tpubDEPGZXGPF2v3uNGTWHteRSQnZdEFfMvh6Dcp3DykkBE48hTA6pe3Es6nWH5dAThPyvqmZGmZqQhTw5kxqgR5uJW6Ei2VWyAAN92yLcYBvzV
# E16: sortedmulti, same wallet respelled, same --key lines: SPEND-EQUAL (rc 0).
rm -f own.md1
"$MD" encode 'wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))' --key "@0=$KA" --key "@1=$KB" --network signet --out own.md1 > /dev/null 2>&1 || fail e16_i2_sortedmulti_respelled_same_wallet "fixture encode failed"
"$MD" descriptor --template 'wsh(sortedmulti(2,@1/<0;1>/*,@0/<0;1>/*))' --key "@0=$KA" --key "@1=$KB" --network signet --verify-against own.md1 > /dev/null 2> v.err; rc=$?
[ $rc -eq 0 ] && pass e16_i2_sortedmulti_respelled_same_wallet || fail e16_i2_sortedmulti_respelled_same_wallet "rc=$rc"
# E17: non-canonical template, one key replaced: NOT spend-equal (rc 1, NO; was 5 before the exit-code cutover).
rm -f own.md1
"$MD" encode 'wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))' --key "@0=$KA" --key "@1=$KB" --network signet --out own.md1 > /dev/null 2>&1 || fail e17_i2_noncanonical_replaced_key "fixture encode failed"
"$MD" descriptor --template 'wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))' --key "@0=$KA" --key "@1=$KC" --network signet --verify-against own.md1 > /dev/null 2> v.err; rc=$?
[ $rc -eq 1 ] && pass e17_i2_noncanonical_replaced_key || fail e17_i2_noncanonical_replaced_key "rc=$rc"
echo "rows failed: $fails (workdir $W)"
[ $fails -eq 0 ]
