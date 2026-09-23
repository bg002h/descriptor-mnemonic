#!/usr/bin/env bash
# gen-compose-golden.sh -- the PRE-`--unspendable` regression floor for
# `md compose` (F-449 stage 2, Task 1 Step 0; mnemonic-engrave
# design/IMPLEMENTATION_PLAN_f449_stage2_compose.md).
#
# Runs a PRE-FLAG `md` (md-cli 0.18.0) over the six presets x four wrappers
# and writes every EXIT-0 cell's stdout, byte-exact, to
#   crates/md-cli/tests/golden/compose_pre_unspendable.json
# as a JSON object {"<argv joined by single spaces>": "<stdout>"}.
#
# Provenance is the point: the golden is a fact about the release BEFORE the
# flag, so a parse bug in a later binary cannot move both sides of
# `cli_compose_unspendable.rs`'s comparison. For that reason this script
# REFUSES to run against a binary whose `md compose --help` mentions
# `--unspendable` -- re-running it after the flag exists would silently
# re-bless whatever that binary does, including a live mutation.
#
# Only exit-0 invocations are recorded (the floor asserts code == 0 for every
# row). Measured at 25acb33c: 14 of the 24 cells exit 0 -- all six presets
# under tr and wsh, plus plain-multisig under sh-wsh and sh. The script
# refuses to write anything else.
#
# The parameter fixtures are the presets' own valid-parameter fixtures from
# crates/md-cli/src/cmd/compose.rs (`every_preset_name_parses_with_some_valid_parameters`).
#
#   MD=/path/to/pre-flag/md scripts/gen-compose-golden.sh
#   (default MD: ${CARGO_TARGET_DIR:-target}/debug/md)
set -euo pipefail
cd "$(dirname "$0")/.."

MD="${MD:-${CARGO_TARGET_DIR:-target}/debug/md}"
[ -x "$MD" ] || { echo "gen-compose-golden: no md binary at $MD (set MD=)" >&2; exit 2; }

if "$MD" compose --help 2>&1 | grep -q -- '--unspendable'; then
  echo "gen-compose-golden: REFUSED -- $MD already knows --unspendable." >&2
  echo "  This golden is Task 1 Step 0's pre-flag floor and must come from a" >&2
  echo "  PRE-flag md (md-cli 0.18.0). Regenerating it from a flag-aware binary" >&2
  echo "  re-blesses the code it exists to guard." >&2
  exit 3
fi

OUT=crates/md-cli/tests/golden/compose_pre_unspendable.json
mkdir -p "$(dirname "$OUT")"

python3 - "$MD" "$OUT" <<'PY'
import json, subprocess, sys
md, out = sys.argv[1], sys.argv[2]
presets = [
    ("plain-multisig", "2of3"),
    ("simple-timelocked-inheritance", "older=26280"),
    ("kofn-recovery", "2of3,older=26280"),
    ("tiered-recovery", "2of2,1of2,older=26280"),
    ("hashlock-gated",
     "sha256=a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8,older=26280"),
    ("decaying-multisig", "2of2,1of1,older1=13140,older2=26280,after=1000000"),
]
wrappers = ["tr", "wsh", "sh-wsh", "sh"]
golden, refused = {}, []
for w in wrappers:
    for name, params in presets:
        args = ["compose", "--wrapper", w, "--preset", f"{name},{params}"]
        assert all(" " not in a for a in args), args
        p = subprocess.run([md] + args, capture_output=True)
        if p.returncode == 0:
            golden[" ".join(args)] = p.stdout.decode("utf-8")
        else:
            refused.append((" ".join(args), p.returncode))
expected_ok = {f"compose --wrapper {w} --preset {n},{pa}"
               for w in ("tr", "wsh") for n, pa in presets}
expected_ok |= {f"compose --wrapper {w} --preset plain-multisig,2of3" for w in ("sh-wsh", "sh")}
if set(golden) != expected_ok:
    sys.exit(f"gen-compose-golden: exit-0 set moved from the measured 14 cells:\n"
             f"  extra={sorted(set(golden) - expected_ok)}\n"
             f"  missing={sorted(expected_ok - set(golden))}")
with open(out, "w") as f:
    json.dump(golden, f, indent=2, sort_keys=True)
    f.write("\n")
print(f"gen-compose-golden: wrote {len(golden)} exit-0 rows to {out} "
      f"({len(refused)} non-zero cells not recorded)")
PY
