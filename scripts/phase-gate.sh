#!/usr/bin/env bash
# phase-gate.sh -- the mdcli-mini "## The gate" section, as a command.
#
# design/IMPLEMENTATION_PLAN_mdcli_mini.md ("## The gate"): from P1 onward,
# every phase's implementer runs this before its final commit and pastes the
# summary lines into the commit message, so the gate is a script rather than
# a from-memory list. P1 widened the four all-features lines below to match
# the same widening it made in .github/workflows/ci.yml (test/clippy/doc
# jobs); P1 also added the two lines that were already ungated
# (`phase-gate-omits-cargo-doc`, r1 I3: the gate must name every CI job or
# the command that runs them).
#
# WHAT THIS DOES NOT COVER. The freebsd and musl compile/test jobs
# (ci.yml:95+) and the windows/macos legs of the test matrix (ci.yml:31-49
# runs three OS contexts; this script reproduces one) are CI-only gates a
# local run cannot reproduce -- the push-ritual staging run
# (scripts/push-via-staging.sh) covers them before anything reaches main. A
# gate that hides its own blind spot is worse than no gate.
#
#   scripts/phase-gate.sh          # exit 0 = every step passed
set -euo pipefail
cd "$(dirname "$0")/.."

step() { echo; echo "=== $* ==="; }

step "cargo nextest run --locked --all-features"
cargo nextest run --locked --all-features

step "cargo test --workspace --doc --all-features"
cargo test --workspace --doc --all-features

step "cargo clippy --locked --all-targets --all-features -- -D warnings"
cargo clippy --locked --all-targets --all-features -- -D warnings

step "cargo fmt --check"
cargo fmt --check

step "cargo doc --workspace --no-deps --document-private-items --all-features"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features

step "design/display-grouping-vectors.tsv.sha256"
( cd design && sha256sum -c display-grouping-vectors.tsv.sha256 )

echo
echo "phase-gate: all six steps passed"
