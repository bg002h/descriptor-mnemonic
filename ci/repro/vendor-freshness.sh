#!/usr/bin/env bash
# vendor/ freshness guard — the LEADING (PR-time) gate. CODEC (fork-free) form.
#
# REDs iff the committed `vendor/` tree cannot satisfy the current `Cargo.lock`
# under the reproducible build's `--offline --locked` source-replacement config.
# This is the v0.74.0 failure class that hit the toolkit: a dep bump that updates
# Cargo.lock but forgets `cargo vendor vendor/`, so the release `--offline`
# reproducible build can't resolve the bumped dep and publishes NO musl binary.
# That gate is LAGGING (fires only at the release tag); this makes the same
# failure surface on the PR.
#
# Cheap by design: `cargo metadata` does FULL-workspace, all-target resolution
# with NO compile / NO musl toolchain / NO Docker. With vendored-sources
# replacement active, resolution validates EVERY Cargo.lock entry against vendor/
# regardless of target cfg (proven in the toolkit R0 — no musl-only false
# negative). Ported verbatim from mnemonic-toolkit:ci/repro/vendor-freshness.sh.
#
# CODEC TWO-BLOCK FORM: this crate is fork-free (no miniscript `[patch.crates-io]`
# git dep — Cargo.lock has zero `source = "git+…"` entries), so the source config
# is the TWO-block form (crates-io + vendored-sources) with NO git-fork stanza and
# NO MINISCRIPT_REV. (The toolkit form adds a third miniscript git-fork block.)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# This crate carries a miniscript git fork (`[patch.crates-io]`), so the source
# config is the THREE-block form: crates-io + the git fork + vendored-sources,
# all redirected at the committed vendor/ tree.
#
# It used to be the two-block CODEC form with a guard that failed closed the
# moment any git source appeared -- which it did when the tr/wsh cycle pinned
# miniscript at ff4732e. Correct behaviour from the guard, wrong config for the
# crate: from that day the gate could no longer pass.
#
# It DID fire and it DID say so -- this workflow ran on the pin commit itself
# (5b4d20ad, 2026-08-20) and failed. The failure then sat unacted-on for two
# days. So the lesson is not "a silent gate": a red gate nobody watches costs
# exactly as much as one that cannot run (F-226, 2026-08-21).

# Derive the fork rev from Cargo.lock -- authoritative and comment-free -- so
# the config auto-tracks the pin instead of drifting from it. Fail CLOSED on an
# empty match: a missing rev would silently drop the git-fork stanza and let
# resolution mis-resolve into a false GREEN.
MINISCRIPT_REV="$(grep -oE 'rust-miniscript\?rev=[0-9a-f]{40}' Cargo.lock | head -1 | grep -oE '[0-9a-f]{40}' || true)"
if [ -z "$MINISCRIPT_REV" ]; then
  echo "::error::vendor-freshness: could not derive the miniscript fork rev from Cargo.lock" \
       "(expected a 'rust-miniscript?rev=<40-hex>' source line). Failing closed." >&2
  exit 1
fi

# P3: `md-cli` pins `mnemonic-io-lib` (the constellation's shared IO crate) by
# exact rev out of `bg002h/mnemonic-engrave`. It is a SECOND git source, so it
# needs its own stanza or `--offline` resolution is not constrained by vendor/
# for it -- exactly what the guard below caught the day the pin landed. Derived
# from Cargo.lock for the same reason the miniscript rev is: so the config
# tracks the pin instead of drifting from it, and fails CLOSED when it cannot.
IOLIB_REV="$(grep -oE 'mnemonic-engrave\?rev=[0-9a-f]{40}' Cargo.lock | head -1 | grep -oE '[0-9a-f]{40}' || true)"
if [ -z "$IOLIB_REV" ]; then
  echo "::error::vendor-freshness: could not derive the mnemonic-io-lib rev from Cargo.lock" \
       "(expected a 'mnemonic-engrave?rev=<40-hex>' source line). Failing closed." >&2
  exit 1
fi

# Fail CLOSED on any git source this config does NOT redirect.
#
# The form below covers exactly TWO git sources: the miniscript fork and
# mnemonic-io-lib. A THIRD git dependency would not be redirected, so
# `--offline` would mis-resolve or reach the live host -- the same false-GREEN
# the original two-block guard existed to prevent, one dependency further along.
# Keeping the guard means gaining a git dep still trips a loud error instead of
# silently weakening the gate. It did exactly that when P3 added the
# mnemonic-io-lib pin, which is how that stanza came to be written.
UNCOVERED="$(grep -oE '^source = "git\+[^"]+"' Cargo.lock \
  | grep -v "rust-miniscript?rev=${MINISCRIPT_REV}" \
  | grep -v "mnemonic-engrave?rev=${IOLIB_REV}" || true)"
if [ -n "$UNCOVERED" ]; then
  echo "::error::vendor-freshness: Cargo.lock has a git source this config does not" \
       "redirect, so --offline resolution would not be constrained by vendor/:" >&2
  printf '%s\n' "$UNCOVERED" >&2
  echo "::error::Add a per-source [source] stanza for it to SRC_CONFIG below." >&2
  exit 1
fi

# 4-block source-replacement: crates-io + the miniscript git fork +
# mnemonic-io-lib + vendored-sources -> the committed vendor/ tree. These are
# the blocks `cargo vendor` itself emits; they are not invented here.
SRC_CONFIG=(
  --config 'source.crates-io.replace-with="vendored-sources"'
  --config "source.\"git+https://github.com/rust-bitcoin/rust-miniscript?rev=${MINISCRIPT_REV}\".git=\"https://github.com/rust-bitcoin/rust-miniscript\""
  --config "source.\"git+https://github.com/rust-bitcoin/rust-miniscript?rev=${MINISCRIPT_REV}\".rev=\"${MINISCRIPT_REV}\""
  --config "source.\"git+https://github.com/rust-bitcoin/rust-miniscript?rev=${MINISCRIPT_REV}\".replace-with=\"vendored-sources\""
  --config "source.\"git+https://github.com/bg002h/mnemonic-engrave?rev=${IOLIB_REV}\".git=\"https://github.com/bg002h/mnemonic-engrave\""
  --config "source.\"git+https://github.com/bg002h/mnemonic-engrave?rev=${IOLIB_REV}\".rev=\"${IOLIB_REV}\""
  --config "source.\"git+https://github.com/bg002h/mnemonic-engrave?rev=${IOLIB_REV}\".replace-with=\"vendored-sources\""
  --config 'source.vendored-sources.directory="vendor"'
)

echo "vendor-freshness: resolving Cargo.lock against committed vendor/ (offline, locked; miniscript rev ${MINISCRIPT_REV}, mnemonic-io-lib rev ${IOLIB_REV}) ..."
if cargo metadata --format-version 1 --locked --offline "${SRC_CONFIG[@]}" >/dev/null; then
  echo "vendor-freshness: OK — vendor/ satisfies Cargo.lock."
else
  echo "::error::vendor/ is out of sync with Cargo.lock — the --offline --locked reproducible build" \
       "cannot resolve a dependency from the committed vendor/ tree. Run 'cargo vendor vendor/' and" \
       "commit the result (see docs/verify-reproducibility.md). This is the toolkit v0.74.0 release-CI" \
       "failure class, now caught at PR time." >&2
  exit 1
fi
