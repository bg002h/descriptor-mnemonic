# Fold: md FIX-B whole-diff s1 I-1 + s2 I-1..I-3

Worktree `/scratch/code/shibboleth/dm-worktrees/bughunt-fix-b`, branch fix/bughunt-b, from fe45778f. One commit (single, not RED/GREEN split; RED seen before the fix): see SHA below. Logs in `design/agent-reports/fold-md-fix-b-whole-diff-logs/` (uncommitted).

## Items applied
1. s1 I-1: RED row appended to `crates/md-codec/tests/policy_shape.rs` (plus `use md_codec::tree::{Body, Node};`). On fe45778f: `1 test run: 0 passed, 1 failed`, panic `policy_shape.rs:624:5: sln:older inside thresh is not a disjunction` (stated reason; `red.log`). Then the guarded `Tag::OrI | Tag::AndOr` arm in `crates/md-codec/src/policy_shape.rs` `collect`, before the all-false disjunction arm. FIX doc R-5 row (line 83) amended with the exception text.
2. s2 I-1..I-3: rows `bughunt_md_05b_...`, `bughunt_md_07b_...`, `bughunt_md_08d_...` appended to `crates/md-codec/tests/bughunt.rs`. Helper `tlv_write_err` already existed; none copied.
3. FIX doc: §3 table rows for all four, §4 lists, §5 filter extended and count 14 -> 18 (16 bughunt + 2 policy_shape).

Judgement call (mechanical, not a product point): clippy `type_complexity` rejected the 07b body as written in `bughunt_new.rs`; I added `type TlvSetter = Box<dyn Fn(&mut TlvSection)>;` above it and used it. No behaviour change. `cargo fmt --all` reflowed the new rows.

## Gate (on the fold head)
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --locked --all-targets -- -D warnings`: exit 0 (first run failed on the type_complexity point above; fixed, rerun clean).
- `cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast`: 1595 tests run: 1595 passed, 13 skipped, 0 failed (= 1591 + 4). (A plain `cargo nextest run --locked` gave 1557 passed, 13 skipped before the type alias edit; the feature set differs, as s1 noted.)
- `cargo test --locked --doc`: exit 0.
- FIX §5 filter (extracted from the doc and run): 18 tests run: 18 passed, 1552 skipped.
- Probe cards `md shape-key` on `md1ypfds3cf2qqvzdq3zsu2nnxyjwqqqqqjq6wtjp0p4p6y4d` and `md1ypfds3cf2qqvzfnfg49tsqqqqys3q2rtwl0d8kt3g0`: both exit 0 (run before the test-only type-alias edit; production code unchanged since).
- s2 M-1 not folded, as briefed.

Counts: 0 Critical / 0 Important / 0 STOP; gate 1595 passed, 13 skipped, 0 failed.
