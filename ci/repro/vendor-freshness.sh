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

# Fail CLOSED if a git source ever appears in Cargo.lock: the two-block config
# would not redirect it, so `--offline` would silently reach the live host (or
# mis-resolve) instead of REDing. If this trips, the crate gained a git dep and
# needs the toolkit's three-block form (a per-source git-fork stanza).
if grep -qE '^source = "git\+' Cargo.lock; then
  echo "::error::vendor-freshness: Cargo.lock now has a git source — the codec two-block" \
       "config can't redirect it. Add a per-source git-fork [source] stanza (see the" \
       "toolkit ci/repro/vendor-freshness.sh three-block form)." >&2
  exit 1
fi

# Two-block source-replacement: crates-io -> vendored-sources -> committed vendor/.
SRC_CONFIG=(
  --config 'source.crates-io.replace-with="vendored-sources"'
  --config 'source.vendored-sources.directory="vendor"'
)

echo "vendor-freshness: resolving Cargo.lock against committed vendor/ (offline, locked) ..."
if cargo metadata --format-version 1 --locked --offline "${SRC_CONFIG[@]}" >/dev/null; then
  echo "vendor-freshness: OK — vendor/ satisfies Cargo.lock."
else
  echo "::error::vendor/ is out of sync with Cargo.lock — the --offline --locked reproducible build" \
       "cannot resolve a dependency from the committed vendor/ tree. Run 'cargo vendor vendor/' and" \
       "commit the result (see docs/verify-reproducibility.md). This is the toolkit v0.74.0 release-CI" \
       "failure class, now caught at PR time." >&2
  exit 1
fi
