#!/usr/bin/env bash
# scripts/vendor-coord-evidence.sh -- vendor coordinator-compatibility
# EVIDENCE from mnemonic-engrave into this repo (ruling R-2, coordinator-compat
# plan 1b), where `cargo xtask verdicts` builds the verdict table from it.
#
# Usage:
#   scripts/vendor-coord-evidence.sh <path-to-mnemonic-engrave>          # write
#   scripts/vendor-coord-evidence.sh --check <path-to-mnemonic-engrave>  # freshness
#
# Writes crates/md-codec/tests/fixtures/coordinator/evidence.jsonl (one row per
# measurement) and evidence.meta.json (the engrave commit and the sha256 of every
# input). --check regenerates both into a temp dir and fails (exit 1) if either
# the rows or the input hashes differ from what is committed -- i.e. if engrave's
# evidence moved and this copy did not. The engrave commit itself is NOT
# compared: an unrelated engrave commit must not make this copy stale.
#
# THIS SCRIPT NEVER COMPUTES A KEY. It transcribes what each harness printed,
# plus per-SOURCE provenance constants (each cites where it is stated). Keys are
# computed by md-codec's one implementation when the table is built.
#
# Rows, and the one rule for each source:
#   Liana 8.0   composer-fable-r0/fable-liana-parse-{in,out}.jsonl, variant "md"
#   Liana 15.0  composer-fable-r0/fable-liana-parse-{in,out-v15}.jsonl, variants
#               "md" and "liana-unspendable-xpub"
#   Liana 15.0  f449-stage2/liana-live-gate-{in,expected}.jsonl (all rows)
#   Liana 15.0  f449-stage4/liana-probes-{parse-in,out}.jsonl (all rows)
#   Liana 15.0  coord-compat-1b/liana-r0-probes-{in,out}.jsonl (plan 1b R0's probes)
#   Nunchuk     composer-fable-r0/fable-nunchuk-harness-out.txt, "<name>.multipath"
#               sections, descriptor from fable-liana-shapes.json's desc_md
#   Nunchuk     coord-compat-1b/nunchuk-kind1-probe.{tsv,out} (the recon's probe)
#   Core        coord-compat-core-boundary/core-<v>.json: ALL15 (chain0 AND chain1,
#               both were imported), K1 (same), and the multipath refusals
# measured_at is the committer date of the commit that recorded the row's line
# (git blame), so it is mechanical, not typed.
set -euo pipefail

check=0
if [[ "${1:-}" == "--check" ]]; then check=1; shift; fi
if [[ $# -ne 1 ]]; then
  echo "usage: $0 [--check] <path-to-mnemonic-engrave>" >&2
  exit 2
fi
engrave=$1
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
out_dir="$repo_root/crates/md-codec/tests/fixtures/coordinator"
work=$out_dir
if [[ $check -eq 1 ]]; then work=$(mktemp -d); trap 'rm -rf "$work"' EXIT; fi
mkdir -p "$work"

python3 - "$engrave" "$work" <<'PY'
import hashlib, json, os, re, subprocess, sys

engrave, work = sys.argv[1], sys.argv[2]
EV = "design/evidence"

def path(rel):
    p = os.path.join(engrave, EV, rel)
    if not os.path.isfile(p):
        sys.exit(f"vendor-coord-evidence: not found: {p}")
    return p

inputs = {}
def read_lines(rel):
    p = path(rel)
    data = open(p, "rb").read()
    inputs[rel] = hashlib.sha256(data).hexdigest()
    return data.decode().split("\n")

def dates(rel):
    """Committer date (YYYY-MM-DD) of the commit that recorded each line."""
    out = subprocess.run(["git", "-C", engrave, "blame", "--line-porcelain", "--", f"{EV}/{rel}"],
                         check=True, capture_output=True, text=True).stdout
    per_line, t = [], None
    for l in out.split("\n"):
        if l.startswith("committer-time "):
            t = int(l.split()[1])
        elif l.startswith("\t"):
            per_line.append(subprocess.run(["date", "-u", "-d", f"@{t}", "+%Y-%m-%d"],
                                           check=True, capture_output=True, text=True).stdout.strip())
    return per_line

def jsonl(rel):
    lines = read_lines(rel)
    d = dates(rel)
    return [(json.loads(l), f"{rel}:{i+1}", d[i]) for i, l in enumerate(lines) if l.strip()]

rows = []
def row(id_, name, coord, version, lib, tool, tver, form, at, desc, outcome, unkeyable=None):
    r = {"id": id_, "name": name, "coordinator": coord, "version": version, "library_rev": lib,
         "renderer_tool": tool, "renderer_version": tver, "form": form,
         "measured_at": at, "descriptor": desc, "outcome": outcome}
    if unkeyable:
        r["unkeyable"] = unkeyable
    rows.append(r)

def liana_reading(rec):
    p = rec.get("primary")
    if not isinstance(p, dict):
        return None
    return {"wallet_kind": "Liana", "threshold": [p.get("k", 1), len(p["keys"])]}

def liana_outcome(rec):
    if rec.get("ok") is True:
        return {"imported": liana_reading(rec)}
    return {"refused": rec["error"]}

# Liana harness descriptors whose internal key md1 cannot carry (a stale key:
# neither NUMS nor Liana's recipe over these leaves, and origin-less).
LIANA_UNKEYABLE = {"CONTROL-stale-key-nested-B-tree-with-A-internal-key":
                   "internal key is neither NUMS nor Liana's recipe over these leaves"}

# -- Liana, fable r0 (8.0 and 15.0) ----------------------------------------
shapes = {s["name"]: s for s in json.loads("\n".join(read_lines("composer-fable-r0/fable-liana-shapes.json")))}
fin = {(r["name"], r["variant"]): r["desc"] for r, _, _ in jsonl("composer-fable-r0/fable-liana-parse-in.jsonl")}
# Liana v8.0 at 9d2fb742 and md 0.17.0: agent-reports/composer-fable-r0-liana-core.md:17
for rel, ver, lib, variants in [
    ("composer-fable-r0/fable-liana-parse-out.jsonl", "8.0", "9d2fb742", {"md"}),
    ("composer-fable-r0/fable-liana-parse-out-v15.jsonl", "15.0", "4684d5cb", {"md", "liana-unspendable-xpub"}),
]:
    for rec, id_, at in jsonl(rel):
        if rec["variant"] not in variants:
            continue
        tool, tver = ("md", "0.17.0") if rec["variant"] == "md" else ("harnesses/liana unspendable", "v15.0")
        row(id_, rec["name"], "liana", ver, lib, tool, tver, "multipath", at,
            fin[(rec["name"], rec["variant"])], liana_outcome(rec))

# -- Liana, F-449 live gate and stage-4 probes (15.0 at 4684d5cb) -----------
for in_rel, out_rel in [("f449-stage2/liana-live-gate-in.jsonl", "f449-stage2/liana-live-gate-expected.jsonl"),
                        ("f449-stage4/liana-probes-parse-in.jsonl", "f449-stage4/liana-probes-out.jsonl")]:
    din = {r["name"]: r["desc"] for r, _, _ in jsonl(in_rel)}
    for rec, id_, at in jsonl(out_rel):
        if "name" not in rec:
            assert rec.get("liana_tag") == "v15.0", rec  # the header pins the tag
            continue
        row(id_, rec["name"], "liana", "15.0", "4684d5cb", "harnesses/liana unspendable", "v15.0", "multipath",
            at, din[rec["name"]], liana_outcome(rec), LIANA_UNKEYABLE.get(rec["name"]))

# -- Liana 15.0, the plan 1b R0 review's probes (harness at fable-liana-src-v15,
# `git describe` v15.0, 4684d5cb; agent-reports/coord-compat-1b-plan-r0.md).
# Hand-built descriptors over the X24 keys: the 1-of-n fold (I-1) and older(70000) (M-1).
din = {r["name"]: r["desc"] for r, _, _ in jsonl("coord-compat-1b/liana-r0-probes-in.jsonl")}
for rec, id_, at in jsonl("coord-compat-1b/liana-r0-probes-out.jsonl"):
    row(id_, rec["name"], "liana", "15.0", "4684d5cb", "plan 1b r0 probe (hand-built)", "2026-09-23",
        "multipath", at, din[rec["name"]], liana_outcome(rec))

# -- Nunchuk 2.1.1 (libnunchuk a7cfb49), fable r0, md 0.16.2 ----------------
# agent-reports/composer-fable-r0-nunchuk.md:6 (md 0.16.2), recon §3c (a7cfb49 = 2.1.1's pin)
rel = "composer-fable-r0/fable-nunchuk-harness-out.txt"
lines, d = read_lines(rel), dates(rel)
i = 0
while i < len(lines):
    m = re.match(r"^### (.+)\.multipath$", lines[i])
    if not m:
        i += 1
        continue
    name, start, rec, body = m.group(1), i, {}, []
    i += 1
    while i < len(lines) and not lines[i].startswith("### "):
        body.append(lines[i])
        for k, v in re.findall(r"(\w+)=(\S*)", lines[i]):
            rec.setdefault(k, v)
        i += 1
    if rec["ParseWalletDescriptor"] == "ACCEPT":
        kind = rec["wallet_type"]
        thr = [int(rec["m"]), int(rec["n"])] if kind == "MULTI_SIG" else None
        outcome = {"imported": {"wallet_kind": kind, "threshold": thr}}
    else:
        line = next(l.strip() for l in body if l.strip().startswith("ParseWalletDescriptor="))
        outcome = {"refused": line}
    row(f"{rel}:{start+1}", name, "nunchuk", "2.1.1", "a7cfb49", "md", "0.16.2", "multipath",
        d[start], shapes[name]["desc_md"], outcome)

# -- Nunchuk 2.1.1, the recon's kind-1 probe (ruling R-1's evidence) --------
rel_tsv, rel_out = "coord-compat-1b/nunchuk-kind1-probe.tsv", "coord-compat-1b/nunchuk-kind1-probe.out"
descs = {}
for l in read_lines(rel_tsv):
    if l.strip():
        n, desc = l.split("\t")[:2]
        descs[n] = desc
lines, d = read_lines(rel_out), dates(rel_out)
PR1746 = "PR-1746 form: an origin-less real internal key, which md1 cannot encode (recon §4)"
for i, l in enumerate(lines):
    m = re.match(r"^### (\S+)$", l)
    if not m:
        continue
    name = m.group(1)
    end = next((j for j in range(i + 1, len(lines)) if lines[j].startswith("### ")), len(lines))
    block = "\n".join(lines[i + 1:end])
    if "ParseWalletDescriptor=ACCEPT" in block:
        outcome = {"imported": {"wallet_kind": re.search(r"wallet_type=(\S+)", block).group(1), "threshold": None}}
    else:
        outcome = {"refused": re.search(r"ParseWalletDescriptor=REFUSE[^\n]*", block).group(0)}
    row(f"{rel_out}:{i+1}", name, "nunchuk", "2.1.1", "a7cfb49", "recon-1b-nunchuk-probe.py", "2026-09-23",
        "multipath", d[i], descs[name], outcome, PR1746 if name.startswith("pr1746") else None)

# -- Bitcoin Core, official release binaries --------------------------------
live = {r["name"]: r for r, _, _ in jsonl("f449-stage2/liana-live-gate-expected.jsonl") if "name" in r}
k1_desc = live["CONTROL-accept-kofn-recovery-flat"]["liana_desc"]
for v in ["24.2", "25.0", "25.2", "26.0", "26.2", "27.2", "28.4", "29.4", "30.3", "31.1"]:
    rel = f"coord-compat-core-boundary/core-{v}.json"
    rec = json.loads("\n".join(read_lines(rel)))
    at = dates(rel)[0]
    got = re.fullmatch(r"/Satoshi:(\d+)\.(\d+)\.\d+/", rec["version"])
    assert got and f"{got.group(1)}.{got.group(2)}" == v, (v, rec["version"])  # self-reported
    lib = rec["version"]
    def core_outcome(text):
        if text.startswith("ACCEPT"):
            return {"imported": None if "addr==md" in text else {"wallet_kind": "ADDRESSES_DIFFER", "threshold": None}}
        return {"refused": text.split("error message:")[-1].strip()}
    for name, verdict in rec["ALL15"].items():
        for form in ("chain0", "chain1"):
            row(f"{rel}:ALL15.{name}", name, "core", v, lib, "md", "0.17.0", form, at,
                shapes[name]["desc_md"], core_outcome(verdict))
    k1 = rec["K1"]
    k1_text = ("ACCEPT addr==md" if k1["addr_match"] is True else "ACCEPT")\
        if k1["verdict_import"] == "ACCEPT" else k1["getdescriptorinfo_c0"]
    for form in ("chain0", "chain1"):
        row(f"{rel}:K1", "CONTROL-accept-kofn-recovery-flat", "core", v, lib, "harnesses/liana unspendable", "v15.0", form, at,
            k1_desc, core_outcome(k1_text))
    for tag, name, desc, tool, tver in [
            ("REP", "preset-kofn-recovery-tr", shapes["preset-kofn-recovery-tr"]["desc_md"], "md", "0.17.0"),
            ("K1", "CONTROL-accept-kofn-recovery-flat", k1_desc, "harnesses/liana unspendable", "v15.0")]:
        mp = rec[tag]["getdescriptorinfo_multipath"]
        if mp != "ok":  # an ok here is a parse, not an import: no positive row
            row(f"{rel}:{tag}.multipath", name, "core", v, lib, tool, tver, "multipath", at,
                desc, {"refused": mp.split("error message:")[-1].strip()})

with open(os.path.join(work, "evidence.jsonl"), "w") as f:
    for r in rows:
        f.write(json.dumps(r, sort_keys=True, ensure_ascii=False) + "\n")
head = subprocess.run(["git", "-C", engrave, "rev-parse", "HEAD"], check=True,
                      capture_output=True, text=True).stdout.strip()
with open(os.path.join(work, "evidence.meta.json"), "w") as f:
    json.dump({"engrave_commit": head, "inputs": dict(sorted(inputs.items()))}, f, indent=1, sort_keys=True)
    f.write("\n")
print(f"vendor-coord-evidence: {len(rows)} rows from {len(inputs)} inputs")
PY

if [[ $check -eq 1 ]]; then
  stale=0
  cmp -s "$work/evidence.jsonl" "$out_dir/evidence.jsonl" || { echo "STALE: evidence.jsonl differs from a fresh vendoring" >&2; stale=1; }
  inputs() { python3 -c 'import json,sys; print(json.dumps(json.load(open(sys.argv[1]))["inputs"], sort_keys=True))' "$1"; }
  [[ "$(inputs "$work/evidence.meta.json")" == "$(inputs "$out_dir/evidence.meta.json")" ]] \
    || { echo "STALE: an engrave evidence input changed since vendoring" >&2; stale=1; }
  [[ $stale -eq 0 ]] && echo "vendor-coord-evidence: fresh"
  exit $stale
fi
