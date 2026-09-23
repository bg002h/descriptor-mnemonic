#!/usr/bin/env bash
# scripts/vendor-liana-evidence.sh -- vendor the Liana-import evidence for
# the tr kind-1 (Liana unspendable xpub) wire kind, from mnemonic-engrave's
# composer-fable-r0 evidence JSONL, into this repo's own fixture at
# crates/md-codec/tests/fixtures/liana/cases.json.
#
# Usage:
#   scripts/vendor-liana-evidence.sh <path-to-mnemonic-engrave>
#
# Reads, from the given mnemonic-engrave checkout:
#   design/evidence/composer-fable-r0/fable-liana-parse-in.jsonl
#   design/evidence/composer-fable-r0/fable-liana-parse-out-v15.jsonl
#
# and extracts the nine `variant == "liana-unspendable-xpub"` records --
# design/SPEC_liana_unspendable_internal_key.md section 1's four ACCEPTed
# shapes (preset-kofn-recovery-tr, preset-tiered-recovery-tr,
# same-seed-two-paths-tr, X19-tr-kofn-nums-older5) plus the four Liana
# refused on policy shape (preset-hashlock-gated-tr,
# preset-decaying-multisig-tr, hashlock-gated-tr-hash160,
# X20-tr-hashlock-known), plus F-449 stage 2's nested ACCEPT
# (nested-2of2-two-recoveries-tr, F-640). Section 2's derivation recipe is
# correct for the refused four too; preset-decaying-multisig-tr (refused)
# and nested-2of2-two-recoveries-tr (accepted) are the two nested taptrees.
#
# ADDING A CASE IS FOUR EDITS, NOT ONE (F-449 stage 2 plan, Task 5 Step 2):
# NAMES and ACCEPTED_NAMES below, plus one record in EACH of the two JSONLs
# above, committed in mnemonic-engrave BEFORE this script runs -- every case
# is stamped with that repo's HEAD as `source_commit`. This script rebuilds
# cases.json wholesale, so a case added by hand is deleted by the next run.
#
# For each, writes one JSON object with exactly the fields
# `crates/md-codec/tests/common/liana.rs`'s `Case` struct expects:
#   name, accepted, leaf_tlv_hex, leaf_pubkeys_hex, expected_xpub,
#   descriptor_with_checksum, liana_receive, liana_change
# plus a `source_commit` field (the mnemonic-engrave HEAD this was vendored
# from) that `Case`'s derive(Deserialize) silently ignores -- there is no
# `#[serde(deny_unknown_fields)]`, and `all_cases()` decodes this file
# directly into a bare `Vec<Case>`, so a wrapping object is not an option.
#
# leaf_tlv_hex/leaf_pubkeys_hex are NOT copied from the evidence -- the
# evidence carries only base58 xpub strings, one per key. This script
# base58check-decodes each LEAF key's xpub found in the descriptor's `{...}`
# tree, in the order it appears left to right in the descriptor text (which
# IS wire/slot order -- SPEC section 2 -- and was independently verified
# while writing this script to reproduce every one of the eight `desc`
# strings' own embedded internal-key xpub, including the nested-taptree
# case; see task-1-report.md). It keeps the 65-byte
# `chain code (32) || compressed pubkey (33)` slice at extended-key payload
# offset [13:78] -- md's own TLV entry layout (validate.rs:331-335,:348) --
# NOT the base58 descriptor's [45:78] pubkey-only slice, and NOT the
# 78-byte payload verbatim (SPEC section 2 step 1 is explicit that md1
# carries no version/depth/fingerprint/child-number bytes). leaf_pubkeys_hex
# is the pubkey half of that same slice, i.e. bytes [32:65] of the 65.
#
# The first xpub in the descriptor (the tr() internal key itself) is NOT a
# leaf -- it IS the value this evidence claims the recipe derives, and is
# vendored verbatim as `expected_xpub`. This script does not recompute it:
# that recipe does not exist in md-codec yet (it is this plan's later
# tasks), so there is nothing yet to check `expected_xpub` against inside
# the crate. It was checked by hand, once, for all eight cases while
# writing this script -- see task-1-report.md -- and every one reproduced
# byte-for-byte.

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <path-to-mnemonic-engrave>" >&2
  exit 1
fi

engrave_repo=$1
in_jsonl="$engrave_repo/design/evidence/composer-fable-r0/fable-liana-parse-in.jsonl"
out_jsonl="$engrave_repo/design/evidence/composer-fable-r0/fable-liana-parse-out-v15.jsonl"

for f in "$in_jsonl" "$out_jsonl"; do
  if [[ ! -f "$f" ]]; then
    echo "vendor-liana-evidence: not found: $f" >&2
    exit 1
  fi
done

source_commit=$(git -C "$engrave_repo" rev-parse HEAD)

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
out_dir="$repo_root/crates/md-codec/tests/fixtures/liana"
out_file="$out_dir/cases.json"
mkdir -p "$out_dir"

python3 - "$in_jsonl" "$out_jsonl" "$source_commit" "$out_file" <<'PY'
import hashlib
import json
import re
import sys

in_path, out_path, source_commit, out_file = sys.argv[1:5]

VARIANT = "liana-unspendable-xpub"

# Order of first appearance in the evidence file -- deterministic, and
# matches the plan brief's listing (the four ACCEPTed shapes named first).
NAMES = [
    "preset-kofn-recovery-tr",
    "preset-tiered-recovery-tr",
    "preset-hashlock-gated-tr",
    "preset-decaying-multisig-tr",
    "hashlock-gated-tr-hash160",
    "same-seed-two-paths-tr",
    "X19-tr-kofn-nums-older5",
    "X20-tr-hashlock-known",
    # F-449 stage 2 (F-640): the nested taptree Liana v15.0 ACCEPTS --
    # the only nested ACCEPT in the corpus. Appended last so the eight
    # existing cases keep their positions.
    "nested-2of2-two-recoveries-tr",
]
ACCEPTED_NAMES = {
    "preset-kofn-recovery-tr",
    "preset-tiered-recovery-tr",
    "same-seed-two-paths-tr",
    "X19-tr-kofn-nums-older5",
    "nested-2of2-two-recoveries-tr",
}

ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def b58decode_check(s: str) -> bytes:
    """Base58Check-decode an extended-key string to its raw 78-byte payload
    (version(4) || depth(1) || parent_fingerprint(4) || child_number(4) ||
    chain_code(32) || key(33)), verifying the trailing 4-byte checksum."""
    num = 0
    for ch in s:
        idx = ALPHABET.find(ch)
        if idx < 0:
            raise ValueError(f"not base58: {s!r}")
        num = num * 58 + idx
    nbytes = (num.bit_length() + 7) // 8
    combined = num.to_bytes(nbytes, "big")
    n_leading = len(s) - len(s.lstrip("1"))
    combined = b"\x00" * n_leading + combined
    payload, checksum = combined[:-4], combined[-4:]
    want = hashlib.sha256(hashlib.sha256(payload).digest()).digest()[:4]
    if want != checksum:
        raise ValueError(f"bad base58check checksum: {s}")
    if len(payload) != 78:
        raise ValueError(
            f"expected a 78-byte extended-key payload, got {len(payload)}: {s}"
        )
    return payload


XPUB_RE = re.compile(r"xpub[1-9A-HJ-NP-Za-km-z]+")


def load_variant(path):
    out = {}
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            if rec.get("variant") == VARIANT:
                out[rec["name"]] = rec
    return out


in_recs = load_variant(in_path)
out_recs = load_variant(out_path)

missing_in = [n for n in NAMES if n not in in_recs]
missing_out = [n for n in NAMES if n not in out_recs]
if missing_in or missing_out:
    raise SystemExit(
        f"vendor-liana-evidence: missing evidence -- in={missing_in} out={missing_out}"
    )

cases = []
for name in NAMES:
    desc = in_recs[name]["desc"]
    xpubs = XPUB_RE.findall(desc)
    if len(xpubs) < 2:
        raise SystemExit(
            f"{name}: expected an internal key plus >=1 leaf key, "
            f"got {len(xpubs)} xpub(s) total"
        )
    expected_xpub, leaf_xpubs = xpubs[0], xpubs[1:]

    leaf_tlv_hex = []
    leaf_pubkeys_hex = []
    for lx in leaf_xpubs:
        payload = b58decode_check(lx)
        chain_code, pubkey = payload[13:45], payload[45:78]
        leaf_tlv_hex.append((chain_code + pubkey).hex())
        leaf_pubkeys_hex.append(pubkey.hex())

    out_rec = out_recs[name]
    accepted = bool(out_rec.get("ok"))
    if accepted != (name in ACCEPTED_NAMES):
        raise SystemExit(
            f"{name}: evidence ok={accepted} disagrees with the plan's "
            "accepted-set membership -- evidence or plan has drifted"
        )
    liana_receive = out_rec.get("receive", []) if accepted else []
    liana_change = out_rec.get("change", []) if accepted else []

    cases.append(
        {
            "name": name,
            "accepted": accepted,
            "leaf_tlv_hex": leaf_tlv_hex,
            "leaf_pubkeys_hex": leaf_pubkeys_hex,
            "expected_xpub": expected_xpub,
            "descriptor_with_checksum": desc,
            "liana_receive": liana_receive,
            "liana_change": liana_change,
            "source_commit": source_commit,
        }
    )

with open(out_file, "w", encoding="utf-8") as fh:
    json.dump(cases, fh, indent=2)
    fh.write("\n")

print(
    f"vendor-liana-evidence: wrote {len(cases)} cases to {out_file} "
    f"(source {source_commit})"
)
PY
