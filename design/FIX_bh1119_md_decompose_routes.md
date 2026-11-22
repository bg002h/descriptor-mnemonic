# FIX_bh1119_md_decompose_routes: `md decompose --emit commands` prints only routes it has proven (bh-md-C1 Critical, bh-md-I9 Important)

Small-fix track (bughunt-1119 FIX table row #4; class B, funds path). Ruling: coordinator
`agent-reports/architect-bughunt-1119-questions.md` T8 (decided). Repo descriptor-mnemonic,
branch `fix/md-bh-criticals` from main `5df4cb923e2124be16575207a0050986f0f0d7bc`. Crate `md-cli` only;
no md-codec change, no wire change, no new flag, type or command.
Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22.

**Two STOPs are open (section 9): Q1 how route 2 is proven, Q2 the `--emit template` pointer.** Every
other point is decided below; new holder-visible texts are marked PROPOSED (architect confirms, via the
WORDING_REVIEW row this FIX owes).

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (`rust-toolchain.toml`); `--locked`;
nextest `--build-jobs 6 -j 6`; never `--release`. Signet keys only in new rows.

## 1. The defects (RED evidence, run by the author on the base, signet)

Keys: BIP-39 test vector "abandon x11 about" (fingerprint `73c5da0a`), signet, derived with
`mnemonic convert --from phrase=- --to xpub --network signet --path <P>`:

| name | path | depth | tpub |
|---|---|---|---|
| KA | m/48'/1'/0'/2' | 4 | `tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ` |
| K2 | m/84'/1' | 2 | `tpubDB5GSFrKWykZsJgwdd9ViGUECU8AQvTxig4iZonrS5BJ2k9V4ucfHifnXTNfRExUaDsLvZedPfbEN2iphbq11CMDe4WJQAvQRqXaGJuwhwk` |
| M0 | m | 0 | `tpubD6NzVbkrYhZ4XYa9MoLt4BiMZ4gkt2faZ4BcmKu2a9te4LDpQmvEz2L2yDERivHxFPnxXXhqDRkUNnQCpZggCyEZLBktV7VaSmwayqMJy1s` |

- **C1** (`decompose/walk.rs:351-367` `build_template` writes `@i` + origin path + use-site, so a use-site that is
  not a multipath-or-wildcard tail is read by md as part of the ORIGIN): `md decompose
  "wpkh([73c5da0a/48'/1'/0'/2']KA/0/*)" --network signet --emit commands` exits 0 and prints route 1
  `md encode 'wpkh(@0/48'\''/1'\''/0'\''/2'\''/0/*)' --key '@0=KA' --fingerprint '@0=73c5da0a' --network signet`.
  Run as printed it exits 0; its card's first address is `tb1qly3u3wjcuaq086x24mlfl9fxyp8kmxm3aenp53`; the input
  wallet's first address (KA/0/0) is `tb1qpl9q2sgpew5vpxl2cqgxmk5024j87rlry9xgdd`. Route 2 prints the same
  template and composes to the same wrong wallet. The same holds with no use-site at all
  (`...KA)`: the card is a ranged `/*` wallet) and with `/7/<0;1>/*`.
- **I9** (`decompose.rs:124-223` never checks `md encode --key`'s depth rule nor mk's record rules):
  `md decompose "wpkh([73c5da0a]M0/<0;1>/*)" --network signet --emit commands` exits 0; printed route 1
  exits 1 `md: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally
  uses 3), got 0`; printed route 2 exits 64 at `mk encode`: `error: keys.txt:1: origin "73c5da0a" has no
  derivation path; a key card must declare one (e.g. `[73c5da0a/48'/0'/0'/2']`)`.
- Found while authoring (same root as I9, Important): a depth-2 key `wpkh([73c5da0a/84'/1']K2/<0;1>/*)`:
  route 1 fails as printed (`... got 2`); route 2 runs and composes back to the input. This is the one
  case found where one route is withheld and the other printed.

The author's RED runner (scratch; the builder commits it as the gate, section 7):
`bash /scratch/cache/md-bh-criticals-red/red.sh target/debug/md` on the base:
`FAIL d1..d5`, `PASS d6, d7, d8, d9, d6b` (log `/scratch/cache/md-bh-criticals-red/base-5df4cb92.log`).

## 2. Spec delta (behaviour; T8)

Rule (T8): **`--emit commands` prints a route only after proving it.** md runs the route's own encode in
process, re-derives the descriptor from what it minted, and compares that with the input descriptor. A route
that does not reproduce the input is withheld.

2.1 Order of checks in `commands()` (`decompose.rs:124`): (1) today's origin-less refusal, unchanged, first;
(2) build every route's lines exactly as today; (3) prove route 1, then route 2; (4) print.

2.2 **Route 1 proof.** With exactly the printed arguments (the template, every `--key`, every
`--fingerprint`, `--experimental` iff printed, the decompose `--network`, no `--path`, not chunk-forced),
run the same calls `md encode` text mode makes, in the same order: `ctx_for_template`, `parse_key` per key,
`parse_fingerprint` per fingerprint, `parse_template_ext(.., experimental, Disposition::Refuse)`,
`md_codec::validate::validate_relative_timelocks`, `mint_md1_cards(&d, false)` (`cmd/encode.rs:56-76,190,198`; `apply_path_override` at :119 is a no-op with no `--path`).
Then decode the minted strings (`decode_md1_string` for one, `chunk::reassemble` for a set), render with
`md_codec::to_miniscript_descriptor_multipath_with_network(&decoded, network)`, and compare with the input
descriptor (`Decomposition::descriptor`, re-parsed) under normalisation N.
A refusal at any step, or a comparison that differs, withholds route 1. The builder may factor the shared
calls into one `pub(crate)` helper used by both `encode::run` and the proof (preferred, so the proof cannot
drift from `md encode`); `md encode`'s own output must not change (its suite is the check).

**Normalisation N:** on both sides, every extended key's parent fingerprint is set to `00000000`; nothing
else is normalised. Reason: a card does not carry the parent fingerprint and md renders it as zero (F-611,
`md descriptor --help`); the author checked by base58 decode that a reproduced key differs from the input
key only in bytes 5-8. Equality is equality of the two normalised descriptors' Display strings (origin
fingerprint, origin path, depth, child number, chain code, key, use-site, wildcard and script all count).
Author's mock of N (python, `/scratch/cache/md-bh-criticals-red/red.sh` `norm.py`) accepts today's good
routes on the Liana `preset-kofn-recovery-tr` case and the `v-d-rt` pathological fixture.

2.3 **Route 2 proof: STOP Q1** (section 9). Whatever Q1 decides, the D4 row (I9) must end with route 2
withheld (T8 names "route 2 when `mk encode` refuses depth 0") and the D5 row with route 2 printed.

2.4 **Output.**
- Every route proven: stdout and stderr byte for byte as today (row D6b pins the control's stdout hash).
- Some route withheld, at least one proven: exit 0. The withheld route's whole block (its `# ── route N`
  header and all its command lines; for route 2 also the heredoc and the `mk encode` line) is replaced by one
  stdout line; the blank line between the two route blocks stays; the proven route prints as today; the
  `note:` lines on stderr print as today. The line, PROPOSED:

  > `` # ── route <N> withheld: <reason>; see `md decompose --emit template` ── ``

- Every route withheld: exit 1 (`CliError::Decompose`, like the origin-less refusal), stdout empty, no
  `note:` lines (as the origin-less refusal today), stderr one line, PROPOSED:

  > `` md: decompose: --emit commands cannot be produced: no mint route reproduces this descriptor. route 1: <reason>. route 2: <reason>. The emissions that mint nothing still work: `--emit template`, `--emit keys`, `--emit descriptor`. ``

- `<reason>`, PROPOSED, one of:
  - `md encode refuses it: <error>` where `<error>` is the refusing step's `CliError` Display (the text
    `md encode` prints after `md: `), e.g. `md encode refuses it: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally uses 3), got 0`;
  - `the wallet it mints is not this descriptor` (the comparison differs);
  - route 2's refusal reason: by Q1.

2.5 No other `--emit` form changes (subject to Q2). No new flag, exit code or error variant.

## 3. Help, manual, CHANGELOG (exact texts, PROPOSED)

- `crates/md-cli/src/main.rs:837-841` (`--emit` long help), the sentence "`commands` prints runnable `md
  encode` / `mk encode` lines and REFUSES when any key states no origin (an mk1 card binds key to origin by
  design)." becomes:
  > `commands` prints runnable `md encode` / `mk encode` lines for each mint route md has proven reproduces the descriptor, withholding any other route with one line, and REFUSES when any key states no origin (an mk1 card binds key to origin by design) or when no route reproduces the descriptor.
- `crates/md-cli/src/cmd/decompose.rs:25` (`Emit::Commands` doc, shown in `--help` possible values):
  > Runnable `md encode` / `mk encode` command lines for each mint route proven to reproduce the descriptor
- Toolkit manual lockstep (mnemonic-toolkit master `docs/manual/src/40-cli-reference/42-md.md:335`, the
  `--emit` row), the parenthetical after `commands` becomes:
  > `commands` (runnable `md encode` / `mk encode` command lines for each mint route md has proven reproduces the descriptor; a route it cannot prove is withheld with one line, and it REFUSES when any key states no origin, since an mk1 card binds key to origin by design, or when no route reproduces the descriptor)
  No other manual line names `--emit commands` (grep of `docs/manual` at origin/master). mnemonic-gui's
  mirror (`src/schema/md.rs:1102`) paraphrases `--emit` without this clause: no GUI change.
- `CHANGELOG.md` under `## md-cli [Unreleased]`, new `### Fixed`:
  > - **`md decompose --emit commands` prints a mint route only after proving it.** md now mints each route's card in memory and compares the wallet it describes with the input descriptor. A route that would mint a different wallet (a use-site md1 cannot hold, such as `/0/*` or none) or that `md encode`/`mk encode` would refuse (such as a depth-0 key) is withheld with one line; when no route reproduces the descriptor the command refuses (exit 1).

## 4. TRACE rows (RED on the base; GREEN after the build)

`$MD` = the built `md`; every row `--network signet`; `O` = `[73c5da0a/48'/1'/0'/2']`. Tests live in a new
`crates/md-cli/tests/bh1119_md_decompose_routes.rs`, each test function named exactly as its row. "Runs and
reproduces" = the printed block run by `bash` exits 0 and its artifacts compose (route 1: `md descriptor
<cards>`; route 2: `md descriptor <policy.md1> --from-mk1 <mk1 cards>`) to the input under N.

| row | input to `md decompose ... --emit commands` | base (seen) | fixed |
|---|---|---|---|
| d1_c1_fixed_step_use_site | `wpkh(O KA/0/*)` | rc 0, both routes mint a different wallet | rc 1, stdout empty, stderr the 2.4 refusal, route 1 and 2 reason `the wallet it mints is not this descriptor` (route 2 subject to Q1) |
| d2_no_wildcard | `wpkh(O KA)` | rc 0, both routes mint a different wallet | as d1 |
| d3_fixed_step_then_multipath | `wpkh(O KA/7/<0;1>/*)` | rc 0, both routes mint a different wallet | as d1 |
| d4_i9_master_key | `wpkh([73c5da0a]M0/<0;1>/*)` | rc 0; route 1 rc 1 (`got 0`), route 2 rc 64 (mk, no derivation path) | rc 1, stdout empty, route 1 reason `md encode refuses it: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally uses 3), got 0`, route 2 reason by Q1 |
| d5_depth2_route1_withheld | `wpkh([73c5da0a/84'/1']K2/<0;1>/*)` | rc 0; route 1 rc 1 (`got 2`); route 2 runs and reproduces | rc 0; stdout line 1 is the 2.4 withheld line for route 1 with reason `md encode refuses it: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally uses 3), got 2`; route 2 block as today, runs and reproduces |
| d6_control_standard | `wpkh(O KA/<0;1>/*)` | both routes run and reproduce | unchanged; stdout sha256 prefix `db14761d61c955d2` (d6b) |
| d7_control_other_multipath | `wpkh(O KA/<2;3>/*)` | both run and reproduce | unchanged |
| d8_control_bare_wildcard | `wpkh(O KA/*)` | both run and reproduce | unchanged |
| d9_origin_less_unchanged | `wpkh(KA/<0;1>/*)` | rc 1, origin-less refusal text | byte-identical (the check runs first) |

Plausible wrong implementations each row catches: a use-site allow-list that admits one fixed step
before `*` (d1); an allow-list that checks only the tail (d3 ends in `/<0;1>/*`); a check that `md encode`
exits 0 without comparing wallets (d1-d3 all mint); a comparison of raw xpub strings without N (d6-d8 would
be withheld: every reproduced key differs in its parent fingerprint); refusing the whole command when one
route fails (d5); a copied depth rule instead of running `md encode`'s own calls (d1-d3 still mint).

Existing tests that keep passing unchanged (the proof must not withhold their routes): `cmd_decompose.rs`
`emit_commands_prints_runnable_md_and_mk_lines` (:164), `v_d_noorig_*` (:389, :413),
`emit_commands_route1_line_actually_runs` (:514); `cmd_decompose_roundtrip.rs`
`v_d_rt_the_recorded_mint_commands_are_still_the_ones_decompose_emits` (:314); `cli_network_version_bytes.rs`
`decompose_commands_recipe_runs_on_a_test_network` (:158); `keyless_hashlock_reaches_a_descriptor.rs` F-548
row (:208); `printed_remedy_runs.rs` (:434). If any of them changes, STOP (the proof is withholding a good
route).

## 5. Tests that change

None existing. New: the nine rows above. The triage files (`/scratch/cache/bh1119-md-t1/.../bughunt_1119.rs`
C1 test, `/scratch/cache/bh1119-md-t3/.../bh1119_md_t3.rs` I9 test) are evidence only; the rows above
replace them (signet, exact expected outputs).

## 6. FOLLOWUPS

Closes nothing in `design/FOLLOWUPS.md` (C1 and I9 are not filed there; the coordinator's bughunt records
close them). Lines to file after merge (listed in the author's report, not appended here): see report.

## 7. Gate line

`bash scripts/fix-bh1119-md-criticals-gate.sh` (builder commits it, with its coordinator catalog row):
runs the author's red.sh rows d1-d9 and d6b against the built `md` and the installed `mk` (route 2 rows need `mk`),
requires all PASS, then the repo CI set: `cargo nextest run --locked --workspace --all-targets
--all-features --no-fail-fast --build-jobs 6 -j 6`, `cargo test --locked --workspace --doc --all-features`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`,
`cargo build --locked -p md-cli --no-default-features`, `RUSTDOCFLAGS=-D warnings cargo doc --workspace
--no-deps --document-private-items --all-features`, `bash ci/code-docs-check.sh`. Shown RED once on the base.
Not covered: the toolkit manual edit (separate toolkit change at the pin bump), macOS/Windows/FreeBSD/musl
legs, the WORDING_REVIEW row.

## 8. Overlaps

- `fix/single-print` (descriptor-mnemonic, worktree `/scratch/cache/wt-md-single-print`, at `5df4cb92`
  with no commits when checked): it changes how `md encode` prints its card. This FIX reads
  `cmd/encode.rs:56-76,190-198` (and may factor them into a helper); single-print's print path is later in
  the same `run`. Same file, different lines; whichever lands second rebases.
- Row #5 (`FIX_bh1119_md_placeholder_order`, same branch): no shared lines. Row #8 (encoder admission):
  route 1's proof calls `mint_md1_cards`, which after #8 also self-decodes; a #8 refusal then withholds the
  route with reason `md encode refuses it: <error>`, which is correct.

## 9. STOP questions (not chosen)

**Q1. How is route 2 proven in process?** Route 2's own encode is `md encode` (keyless) plus `mk encode
--keys ... --from-md1-set`. md-cli links mk-codec 0.5 (decode and encode) but not mk-cli, and mk-cli's
record admission (no-path refusal `keyfile.rs:61-66`, depth-0/1 fingerprint provability `cmd/encode.rs:262-290`,
membership, network/version help) lives only in mk-cli. Without it, d4's route 2 would be "proven" and still
fail as printed. Options:
- A. md-cli mirrors the mk-cli record rules a decompose record can reach (no derivation path; depth-1
  parent fingerprint vs declared; depth-0 is excluded by the first), plus a data proof (md half minted and
  decoded; each record to a `KeyCard` through `mk_codec::encode`/`decode`; seated with md's own
  `--from-mk1` seating; compared under N). Drift tracked by a cross-repo companion FOLLOWUPS line in mk.
- B. mk-codec (or a new shared crate) exports mk-cli's record admission as a public function; md-cli calls
  it. Drift-free, but a new public API, an mk release and an mk-codec pin bump in md: not small-track.
- C. Data proof only; mk's own refusals stay mk's (fails closed with mk's error). Does not meet T8 for d4.
Recommendation: A for this row (small, meets T8's named case), and file B as a follow-up.

**Q2. The `--emit template` pointer when the template itself is the cause.** For d1-d3,
`md decompose ... --emit template` prints the same folded template the withheld route used
(`wpkh(@0/48'/1'/0'/2'/0/*)`, seen on the base), and `md encode` of it with KA mints the same wrong wallet;
`--emit all` prints it too. T8 says the withheld line names `--emit template`. Options:
- A. Keep T8 literally (both reasons name `--emit template`).
- B. Reason `the wallet it mints is not this descriptor` names `--emit descriptor` instead, and
  `--emit template` / `--emit all` gain the same route-1-style proof (with the input's own keys) and refuse
  a template that does not reproduce the input (a holder-visible change to two more forms; may want its own row).
- C. B's pointer change only, leaving `--emit template` as is.
Recommendation: B; at least C, since A points the holder at the C1 defect again.
