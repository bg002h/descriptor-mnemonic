# IMPL-mdcli-mini-P1 — R5: the all-features closure

Implementer: sonnet subagent, single implementer, worktree
`/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`, branch
`mdcli-mini`. Executed
`design/IMPLEMENTATION_PLAN_mdcli_mini.md` section "### P1 — R5: the
all-features closure" exactly as written; **no deviation from the plan.**

## RED evidence (step 1)

```
cargo nextest run --locked --all-features --no-fail-fast
```

```
     Summary [   0.834s] 1106 tests run: 1105 passed, 1 failed, 2 skipped
        FAIL [   0.004s] ( 404/1106) md-cli::bin/md compile::tests::upstream_display_is_still_broken_delete_local_renderer_when_this_fails
error: test run failed
```

Exactly 1 failure, the tripwire named in the plan
(`crates/md-cli/src/compile.rs:338` at the pre-change line numbering). Its
sibling, `render_tr_template_pins_every_topology_class`, passed alone in the
same run (line 504 of the captured log:
`PASS [ 0.004s] ( 403/1106) md-cli::bin/md compile::tests::render_tr_template_pins_every_topology_class`),
confirming the plan's disambiguation (P1.1: genuine upstream PR #953 landing,
not an ordering change) without re-deriving it.

The panic message pinned exactly what upstream's `Display` now produces:

```
left:  "tr(@4,{{pk(@3),pk(@2)},{pk(@1),pk(@0)}})"   (upstream, this run)
right: "tr(@4,{{pk(@3),pk(@2),pk(@1),pk(@0)}})"      (expected, old-broken shape)
```

The `left` value is byte-identical to the local renderer's expected output
for the same policy in `render_tr_template_pins_every_topology_class`,
confirming upstream's fix is complete for this case before any code was
touched.

**Root cause, confirmed from the tree, not assumed:** `Cargo.toml`
`[patch.crates-io]` already pins miniscript to git rev `ff4732e` "for PR
#953 (taptree Display)" — that patch predates P1; P1's job was cleaning up
the code now that the pin has already made the local workaround redundant.

## Changes, file by file

**`crates/md-cli/src/compile.rs`** (commit `1b36bf6`)
- Deleted `render_tr_template` (the ~110-line doc comment + function) and
  its fired tripwire test `upstream_display_is_still_broken_delete_local_renderer_when_this_fails`.
- `compile_policy_to_template`'s `Tap` branch now renders via
  `desc.to_string()` directly, keeping the `split_once('#')` checksum strip
  (checksum note, plan P1.1: upstream's `Descriptor::to_string()` appends a
  BIP-380 checksum the deleted `format!` build never produced).
- Renamed `render_tr_template_pins_every_topology_class` →
  `tap_template_pins_every_topology_class` (r1 N3: its old name cited the
  deleted helper); its doc comment and `compile_strips_descriptor_checksum`'s
  doc comment were rewritten to describe the current single-code-path
  rendering (both keypath-only and taptree shapes go through the same
  `desc.to_string()` + strip, not the old two-path split the stale comments
  described).

**`crates/md-codec/tests/bitcoind_differential.rs`** (commit `1b36bf6`)
- Fixed the comment at (pre-change) line 671 that named `render_tr_template`
  as what "md stopped depending on" — the opposite is now true: md depends
  on upstream's `Display` directly via the patched pin, and the local port
  is gone.

**Orphan sweep (plan P1.3):** `cargo build --locked --all-features` and
`cargo clippy --locked --all-targets --all-features -- -D warnings` both
exit clean with no orphaned imports/helpers from the deletion — nothing else
in `compile.rs` referenced the deleted function or its now-unused
machinery.

**`.github/workflows/ci.yml`** (commit `400a2f7`) — exactly 3 lines changed,
nothing else touched (verified by `git diff --stat` and reading the full
diff):
- test job: `cargo test --workspace --all-targets` →
  `cargo test --workspace --all-targets --all-features`
- clippy job: `cargo clippy --workspace --all-targets -- -D warnings` →
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- doc job: `cargo doc --workspace --no-deps --document-private-items` →
  `cargo doc --workspace --no-deps --document-private-items --all-features`

The doc job's `cargo test --workspace --doc` line was deliberately left
unwidened, matching the plan's own gate-section listing of that command
without `--all-features` (0 doctests in the workspace today per the plan's
baseline measurement).

**`scripts/phase-gate.sh`** (new, executable, commit `400a2f7`) — runs, in
order, failing fast (`set -euo pipefail`):
1. `cargo nextest run --locked --all-features`
2. `cargo test --workspace --doc`
3. `cargo clippy --locked --all-targets --all-features -- -D warnings`
4. `cargo fmt --check`
5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`
6. `( cd design && sha256sum -c display-grouping-vectors.tsv.sha256 )`

Header comment states the blind spot verbatim per the plan's gate section:
the freebsd and musl compile/test jobs and the windows/macos legs of CI's
test matrix are CI-only gates a local run cannot reproduce; the push-ritual
staging run (`scripts/push-via-staging.sh`) covers them before anything
reaches `main`.

## GREEN evidence — full gate run (step 6)

`./scripts/phase-gate.sh`, run to completion after both code commits, exit 0.
No `error`, `FAIL`, or `warning:` anywhere in the captured output (grepped).

```
=== cargo nextest run --locked --all-features ===
     Summary [   0.961s] 1105 tests run: 1105 passed, 2 skipped

=== cargo test --workspace --doc ===
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

=== cargo clippy --locked --all-targets --all-features -- -D warnings ===
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.11s

=== cargo fmt --check ===
(no output — clean)

=== cargo doc --workspace --no-deps --document-private-items --all-features ===
    Finished `dev` profile [optimized + debuginfo] target(s) in 1.39s
   Generated .../target/doc/md/index.html and 1 other file

=== design/display-grouping-vectors.tsv.sha256 ===
display-grouping-vectors.tsv: OK

phase-gate: all six steps passed
```

Test count: 1105 run / 1105 passed / 0 failed / 2 skipped — one fewer than
the RED baseline's 1106 run (the deleted tripwire), and the 1 failure is
gone. All 13 `compile::tests` verified individually
(`cargo nextest run --locked --all-features -E 'test(compile::tests)'`):
13 run, 13 passed, including the renamed `tap_template_pins_every_topology_class`.

## Deviation from the plan

None. Every step in P1.1–P1.3 executed as written; the checksum note and
disambiguation were used as given, not re-derived. The only additions beyond
the plan's literal text were two doc-comment fixes inside `compile.rs`
(the `tap_template_pins_every_topology_class` doc comment and
`compile_strips_descriptor_checksum`'s doc comment) — both directly
downstream of the P1.1 deletion (they described the deleted function's
two-path behavior, which the new single-path code no longer has), judged to
be within the spirit of the "orphan sweep" rather than scope creep, since
leaving them would have left a factually false in-repo comment about code
that no longer exists.

## Commits, this branch (`mdcli-mini`)

- `1b36bf6` — P1.1/P1.3: delete `render_tr_template`, route Tap rendering
  through upstream `Display`
- `400a2f7` — P1.2/gate: widen CI to `--all-features`, add
  `scripts/phase-gate.sh` (carries the full gate output above)
- (this report's commit, final action)

**Final commit SHA:** recorded by the controller after this report's commit
lands; at report-write time the branch tip is `400a2f7`.
