# FIX_bh1119_md_decompose_routes: `md decompose --emit commands` prints only routes it has proven (bh-md-C1 Critical, bh-md-I9 Important)

Small-fix track (bughunt-1119 FIX table row #4; class B, funds path). Ruling: coordinator
`agent-reports/architect-bughunt-1119-questions.md` T8 (decided). Repo descriptor-mnemonic,
branch `fix/md-bh-criticals` from main `5df4cb923e2124be16575207a0050986f0f0d7bc`. Crate `md-cli` only;
no md-codec change, no wire change, no new flag, type or command.
Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22.
Folded 2026-11-22 by Claude Sonnet 5.5 (claude-sonnet-5-5) from the architect ruling
`agent-reports/architect-md-bh4-stops.md` (Claude Opus 5.5, claude-opus-5-5) on STOPs Q1 and Q2.

**No STOP is open.** Q1 (route 2 proof) is ruled option A and Q2 (the `--emit template` pointer) is ruled
"B's check, T8's pointer kept" (section 9 records both). Every holder-visible text is PROPOSED (architect
confirms, via the WORDING_REVIEW row #4 owes).

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
- **New-2** (found by the Q2 STOP; graded **Critical** by the architect, the author proposed Important):
  `md decompose "wpkh(O KA/0/*)" --network signet --emit template` (and `--emit all`, under its header
  "`md encode <TEMPLATE>`") exits 0 and prints the folded template `wpkh(@0/48'/1'/0'/2'/0/*)`; `md encode` of it
  with `--key @0=KA --fingerprint @0=73c5da0a --network signet` exits 0, its card's first address is
  `tb1qly3u3wjcuaq086x24mlfl9fxyp8kmxm3aenp53`, the input wallet's is `tb1qpl9q2sgpew5vpxl2cqgxmk5024j87rlry9xgdd`.
  Same inputs and outcome as C1 (adding the keys by hand is one more step, not a different class). Also folds with no
  use-site (`wpkh(@0/48'/1'/0'/2')`) and `/7/<0;1>/*` (`wpkh(@0/48'/1'/0'/2'/7/<0;1>/*)`) and for an origin-less key
  (`wpkh(KA/0/*)` gives `wpkh(@0/0/*)`). md1 cannot hold a fixed use-site step: md-codec `UseSitePath` is
  `{multipath, wildcard_hardened}` (`crates/md-codec/src/use_site_path.rs:63-68`) and `parse_key` keeps only
  `bytes[13..78]` (`parse/keys.rs:163-165`), so no correct template exists. Unaffected: `/<0;1>/*`, `/<2;3>/*`, `/*`,
  `/*h`, `/<0;1;2>/*`, depth-2 `wpkh(@0/84'/1'/<0;1>/*)`, depth-0 `wpkh(@0/<0;1>/*)`.
- **New-1** (graded **Important** by the architect): a depth-1 key `wpkh([deadbeef/84']K1/<0;1>/*)` (K1 = m/84',
  `tpubD97UxEEVXiRvic42NcHLjLAWv5L3CEkuj587dZtEmb4T4Da74DpjbC5tjm7qBq3G4LxjU9SYuTv7edTvTefctXWPkvgSn7MHUmiPCsSyzyJ`):
  `--emit commands` exits 0; printed route 1 exits 1 `md: --key @0: expected an account-level xpub at depth 3 or 4
  (this script context conventionally uses 3), got 1`; printed route 2 exits 64 at `mk encode`: `error: origin fingerprint
  deadbeef does not match the xpub: at depth 1 the xpub's parent is the master key, so the fingerprint must be 73c5da0a.
  The record pairs a key with an origin that is not its own.` With the right fingerprint (`[73c5da0a/84']`) route 1 still
  fails (`got 1`) and route 2 runs and composes to the input under N. Fails closed (no card), so not Critical; same root as
  I9 (depth-2 case above too); closed by the route proofs (d5, d10, d10b).

The author's RED runner (scratch; the builder commits it as the gate, section 7):
`bash /scratch/cache/md-bh-criticals-red/red.sh target/debug/md` on the base:
`FAIL d1..d5`, `PASS d6, d7, d8, d9, d6b` (log `/scratch/cache/md-bh-criticals-red/base-5df4cb92.log`).

## 2. Spec delta (behaviour; T8)

Rule (T8): **`--emit commands` prints a route only after proving it.** md runs the route's own encode in
process, re-derives the descriptor from what it minted, and compares that with the input descriptor. A route
that does not reproduce the input is withheld.

2.0 **Template check (Q2, ruled).** In `decompose` (not only `commands()`), after `build_template` (`decompose/mod.rs:525`)
and before any emission that prints the template, md reads the template back with its own lexer
(`parse::template::lex_placeholders`, no admission rules) and, for every `@i` in index order, requires the read-back
origin path (none = empty) and use-site (multipath alternatives, wildcard, hardening) to equal occurrence i's. A lexer
read-back, not a use-site allow-list, so the check cannot drift from md's parser. The first `@i` that differs fails the
check (the template then never reaches stdout). Result by emission:
- `--emit template` and `--emit all`: exit 1 (`CliError::Decompose`), stdout empty, stderr one line, PROPOSED:

  > `` md: decompose: this descriptor has no md1 template: key @<i> has use-site `<use-site>`, which an md1 card cannot hold (after the origin it holds only a wildcard, optionally after one multipath step such as `/<0;1>/*`), so a template, and any card minted from it, would describe a different wallet. If this is the receive (`/0/*`) or change (`/1/*`) half of a wallet, give the descriptor with `/<0;1>/*` in its place. The emissions that need no template still work: `--emit keys`, `--emit fingerprints`, `--emit descriptor`. ``

  `<use-site>` is the occurrence's use-site as the input states it (`/0/*`, `/7/<0;1>/*`); with no use-site,
  `has use-site `<use-site>`` reads `has no use-site`.
- `--emit commands`: the same refusal (same text, so exit 1, stdout empty), after check (1) below and before the route proofs.
- `--emit keys`, `--emit fingerprints`, `--emit descriptor`: unchanged (rc 0), except the origin-less note on stderr (today's
  note for keys with no origin) whose last sentence, when the template is refused, reads, PROPOSED: `The descriptor
  emission is unaffected; `--emit template` and `--emit commands` refuse.` (it reads as today when the template is not refused).
- Auto-widening `/0/*` to `/<0;1>/*` is not done (it would add a change branch the input lacks); the refusal asks the holder.

2.1 Order of checks in `commands()` (`decompose.rs:124`): (1) today's origin-less refusal, unchanged, first;
(2) the 2.0 template refusal; (3) build every route's lines exactly as today; (4) prove route 1, then route 2; (5) print.

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

2.3 **Route 2 proof (Q1, ruled option A).** md-cli proves route 2 in process; any failing step withholds route 2 with the reason given.
1. md half: the same calls as the route-1 proof (2.2) with no `--key`: the printed template, the printed `--fingerprint`s,
   `--experimental` iff printed, the decompose `--network`. Refusal: `md encode refuses it: <CliError Display>`.
2. mk-cli's record admission, mirrored for the two rules a decompose record can reach, for each heredoc record n (1-based), in order:
   (a) empty origin path: `mk encode refuses it: keys.txt:<n>: origin "<fp>" has no derivation path; a key card must declare one (e.g. `[<fp>/48'/0'/0'/2']`)`;
   (b) one-component origin path and `xpub.parent_fingerprint != fp`: `mk encode refuses it: origin fingerprint <fp> does not match the xpub: at depth 1 the xpub's parent is the master key, so the fingerprint must be <parent fp>. The record pairs a key with an origin that is not its own.`
   Both texts are mk's own. The other mk rules are unreachable from a decompose record: origin-less keys refuse first, depth/child
   mismatch refuses in `decompose/mod.rs:412-452`, repeated keys in `:281-370`, records carry a bare xpub/tpub and no suffix, the
   keyless policy skips mk's membership loop ("Only KEYED policies are checked"), and mk's depth-0 check needs a depth-0 key with
   a path, which (a) excludes.
3. Each record to `mk_codec::KeyCard::new(vec![stub], Some(fp), path, xpub)` with the stub md's own `--from-mk1` seating reads from
   the md half, `mk_codec::encode`, then `mk_codec::decode`. Refusal: `mk encode refuses it: <mk_codec::Error Display>`.
4. Seat the decoded cards on the md half with the function `md descriptor --from-mk1` uses, render with the network, compare
   under N. Differ: `the wallet it mints is not this descriptor`.
The mirror sits next to `check_depth_consistency` (which already mirrors mk's depth rule) with a comment citing mk-cli
`keyfile.rs` `parse_key_record` and `cmd/encode.rs` provable-fingerprint loop @434d789. Drift in either direction fails
closed (mk adds a refusal: mk exits 64, no card; mk drops one: md withholds a good route); the drift is tracked by the two
FOLLOWUPS lines in section 6. The data proof (steps 3-4) uses the linked codec and cannot drift. mk-codec 0.5 `encode`
checks neither the fingerprint nor a missing path (`key_card.rs:53-55` admits a depth-0 / no-path key), so a data-only proof
(option C) would pass d10 and d4 and route 2 would still fail as printed. The D4 row (I9) ends with route 2 withheld and the D5 row with route 2 printed.

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
  - route 2's refusal reasons: the 2.3 texts, `md encode refuses it: <CliError Display>` (md half), `mk encode refuses it: keys.txt:<n>: origin ...` (2.3 a),
    `mk encode refuses it: origin fingerprint <fp> does not match the xpub: ...` (2.3 b), `mk encode refuses it: <mk_codec::Error Display>` (2.3 step 3),
    or `the wallet it mints is not this descriptor` (2.3 step 4).
  Because a withheld line now prints only after the 2.0 template check passed, its pointer `see `md decompose --emit template``
  is true (T8's pointer kept); the all-withheld refusal keeps its list for the same reason.

2.5 `--emit template` and `--emit all` change only by the 2.0 refusal (a passing template prints byte for byte as today); `--emit
keys`, `fingerprints` and `descriptor` change only by the 2.0 note sentence. No new flag, exit code or error variant; one field on
`Decomposition`.

## 3. Help, manual, CHANGELOG (exact texts, PROPOSED)

- `crates/md-cli/src/main.rs:837-841` (`--emit` long help), the sentence "`commands` prints runnable `md
  encode` / `mk encode` lines and REFUSES when any key states no origin (an mk1 card binds key to origin by
  design)." becomes:
  > `commands` prints runnable `md encode` / `mk encode` lines for each mint route md has proven reproduces the descriptor, withholding any other route with one line, and REFUSES when any key states no origin (an mk1 card binds key to origin by design) or when no route reproduces the descriptor.
- `crates/md-cli/src/cmd/decompose.rs:25` (`Emit::Commands` doc, shown in `--help` possible values):
  > Runnable `md encode` / `mk encode` command lines for each mint route proven to reproduce the descriptor
- `crates/md-cli/src/cmd/decompose.rs:17` (`Emit::Template` doc, shown in `--help` possible values), PROPOSED:
  > The keyless BIP-388 template alone, one line, no checksum. REFUSES when a key's use-site is one an md1 card cannot hold (a fixed step such as `/0/*`, or none).
- `crates/md-cli/src/main.rs:837-841` (`--emit` long help) also gains, after the `commands` sentence above, PROPOSED:
  > `template` and `all` REFUSE when a key's use-site is one an md1 card cannot hold (a fixed step such as `/0/*`, or none).
- `design/SPEC_wallet_form_converter.md:434-440` ("Input boundary"), one sentence added after the bullet, PROPOSED:
  > A fixed-path descriptor is accepted, but md1 cannot hold its use-site, so `--emit template`, `--emit all` and `--emit commands` refuse it (there is no md1 template for it); `--emit keys`, `--emit fingerprints` and `--emit descriptor` still work.
- Toolkit manual lockstep (mnemonic-toolkit master `docs/manual/src/40-cli-reference/42-md.md:335`, the
  `--emit` row), the parenthetical after `commands` becomes:
  > `commands` (runnable `md encode` / `mk encode` command lines for each mint route md has proven reproduces the descriptor; a route it cannot prove is withheld with one line, and it REFUSES when any key states no origin, since an mk1 card binds key to origin by design, or when no route reproduces the descriptor)
  The same manual row (`42-md.md:335`) also gains the clause, PROPOSED: `template` and `all` REFUSE when a key's use-site is one an md1 card cannot hold (a fixed step such as `/0/*`, or none).
  No other manual line names `--emit commands` (grep of `docs/manual` at origin/master). mnemonic-gui's
  mirror (`src/schema/md.rs:1102`) paraphrases `--emit` without this clause: no GUI change.
- `CHANGELOG.md` under `## md-cli [Unreleased]`, new `### Fixed`:
  > - **`md decompose --emit commands` prints a mint route only after proving it.** md now mints each route's card in memory and compares the wallet it describes with the input descriptor. A route that would mint a different wallet (a use-site md1 cannot hold, such as `/0/*` or none) or that `md encode`/`mk encode` would refuse (such as a depth-0 key) is withheld with one line; when no route reproduces the descriptor the command refuses (exit 1). `--emit template` and `--emit all` (and `--emit commands`) now refuse a descriptor whose key use-site md1 cannot hold (a fixed step such as `/0/*`, or none), since the template they printed minted a different wallet.

## 4. TRACE rows (RED on the base; GREEN after the build)

`$MD` = the built `md`; every row `--network signet`; `O` = `[73c5da0a/48'/1'/0'/2']`. Tests live in a new
`crates/md-cli/tests/bh1119_md_decompose_routes.rs`, each test function named exactly as its row. "Runs and
reproduces" = the printed block run by `bash` exits 0 and its artifacts compose (route 1: `md descriptor
<cards>`; route 2: `md descriptor <policy.md1> --from-mk1 <mk1 cards>`) to the input under N.

| row | input to `md decompose ... --emit commands` | base (seen) | fixed |
|---|---|---|---|
| d1_c1_fixed_step_use_site | `wpkh(O KA/0/*)` `--emit commands` | rc 0, both routes mint a different wallet | rc 1, stdout empty, stderr the 2.0 template refusal with key @0 use-site `/0/*` (not the route refusal) |
| d2_no_wildcard | `wpkh(O KA)` `--emit commands` | rc 0, both routes mint a different wallet | as d1, `has no use-site` |
| d3_fixed_step_then_multipath | `wpkh(O KA/7/<0;1>/*)` `--emit commands` | rc 0, both routes mint a different wallet | as d1, use-site `/7/<0;1>/*` |
| d1t_c1_fixed_step_template | `wpkh(O KA/0/*)` `--emit template` | rc 0, stdout `wpkh(@0/48'/1'/0'/2'/0/*)` (folded) | rc 1, stdout empty, the 2.0 template refusal, use-site `/0/*` |
| d2t_no_wildcard_template | `wpkh(O KA)` `--emit template` | rc 0, stdout `wpkh(@0/48'/1'/0'/2')` (folded) | rc 1, stdout empty, the 2.0 template refusal, `has no use-site` |
| d3t_fixed_step_then_multipath_template | `wpkh(O KA/7/<0;1>/*)` `--emit template` | rc 0, stdout `wpkh(@0/48'/1'/0'/2'/7/<0;1>/*)` (folded) | rc 1, stdout empty, the 2.0 template refusal, use-site `/7/<0;1>/*` |
| d1a_c1_fixed_step_all | `wpkh(O KA/0/*)` `--emit all` | rc 0, stdout carries the folded template under the `md encode <TEMPLATE>` header | rc 1, stdout empty, the 2.0 template refusal, use-site `/0/*` |
| d4_i9_master_key | `wpkh([73c5da0a]M0/<0;1>/*)` `--emit commands` | rc 0; route 1 rc 1 (`got 0`), route 2 rc 64 (mk, no derivation path) | rc 1, stdout empty, route 1 reason `md encode refuses it: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally uses 3), got 0`, route 2 reason (2.3 a, n=1) `mk encode refuses it: keys.txt:1: origin "73c5da0a" has no derivation path; a key card must declare one (e.g. `[73c5da0a/48'/0'/0'/2']`)` |
| d5_depth2_route1_withheld | `wpkh([73c5da0a/84'/1']K2/<0;1>/*)` `--emit commands` | rc 0; route 1 rc 1 (`got 2`); route 2 runs and reproduces | rc 0; stdout line 1 is the 2.4 withheld line for route 1 with reason `md encode refuses it: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally uses 3), got 2`; route 2 block as today, runs and reproduces |
| d10_depth1_wrong_fingerprint | `wpkh([deadbeef/84']K1/<0;1>/*)` `--emit commands` | rc 0; route 1 rc 1 (`got 1`); route 2 rc 64 (mk: origin fingerprint deadbeef does not match the xpub) | rc 1, stdout empty, route 1 reason `md encode refuses it: --key @0: expected an account-level xpub at depth 3 or 4 (this script context conventionally uses 3), got 1`, route 2 reason (2.3 b) `mk encode refuses it: origin fingerprint deadbeef does not match the xpub: at depth 1 the xpub's parent is the master key, so the fingerprint must be 73c5da0a. The record pairs a key with an origin that is not its own.` (RED for Q1 option C and for a missing 2.3 b) |
| d10b_depth1_right_fingerprint | `wpkh([73c5da0a/84']K1/<0;1>/*)` `--emit commands` | rc 0; route 1 rc 1 (`got 1`); route 2 rc 0 and composes to the input under N | rc 0; stdout line 1 is the 2.4 withheld line for route 1 with reason `md encode refuses it: --key @0: ... got 1`; route 2 block as today, runs (rc 0) and reproduces |
| d11_origin_less_fixed_step | `wpkh(KA/0/*)` | `commands`: rc 1 origin-less refusal; `template`: rc 0 stdout `wpkh(@0/0/*)`; `keys`: rc 0, note ends `The template and descriptor emissions are unaffected; `--emit commands` refuses.` | `commands`: rc 1, origin-less refusal byte-identical to base; `template`: rc 1, stdout empty, the 2.0 template refusal, use-site `/0/*`; `keys`: rc 0, stdout as base, note ends `The descriptor emission is unaffected; `--emit template` and `--emit commands` refuse.` |
| d12_template_controls | `wpkh(O KA/<2;3>/*)`, `wpkh(O KA/*)`, `wpkh(O KA/<0;1;2>/*)`, and depth-0 `wpkh([73c5da0a]M0/<0;1>/*)`, each `--emit template` | rc 0; stdout `wpkh(@0/48'/1'/0'/2'/<2;3>/*)`, `wpkh(@0/48'/1'/0'/2'/*)`, `wpkh(@0/48'/1'/0'/2'/<0;1;2>/*)`, `wpkh(@0/<0;1>/*)` | unchanged, byte-identical |
| d6_control_standard | `wpkh(O KA/<0;1>/*)` | both routes run and reproduce | unchanged; stdout sha256 prefix `db14761d61c955d2` (d6b) |
| d7_control_other_multipath | `wpkh(O KA/<2;3>/*)` | both run and reproduce | unchanged |
| d8_control_bare_wildcard | `wpkh(O KA/*)` | both run and reproduce | unchanged |
| d9_origin_less_unchanged | `wpkh(KA/<0;1>/*)` | rc 1, origin-less refusal text | byte-identical (the check runs first) |

Base column: every row above was run by the folder on base `f4dad6e5` (md-cli built from it, `mk 0.16.0`; log in the fold
report); the d10/d10b/d4 route lines were run as printed. The architect's scratch plant of the 2.0 check
(`/scratch/cache/arch-md-bh4-template-proof.diff`) confirmed the fixed column for d1-d3, d1t-d3t, d1a, d11 (refusal and rc) and
d12; its wording is the plant's, not the PROPOSED text. The route-proof rows (d4, d5, d10, d10b) and the exact PROPOSED texts have no plant: GREEN is the builder's.

Plausible wrong implementations each row catches: a use-site allow-list that admits one fixed step
before `*` (d1); an allow-list that checks only the tail (d3 ends in `/<0;1>/*`); a check that `md encode`
exits 0 without comparing wallets (d1-d3 all mint); a comparison of raw xpub strings without N (d6-d8 would
be withheld: every reproduced key differs in its parent fingerprint); refusing the whole command when one
route fails (d5); a copied depth rule instead of running `md encode`'s own calls (d1-d3 still mint); a use-site allow-list or a 2.0 check wired only into `commands` (d1t-d3t, d1a still print the folded template); a 2.0 check that refuses `/<2;3>/*`, `/*` or depth 0 (d12); a route-2 data proof without the 2.3 mirror (d10 and d4: mk-codec encodes a wrong-fingerprint or no-path card, route 2 then fails as printed); the mirror without 2.3 b (d10); withholding route 2 when route 1 fails, or the reverse (d5, d10b).

Existing tests that keep passing unchanged (the proof must not withhold their routes): `cmd_decompose.rs`
`emit_commands_prints_runnable_md_and_mk_lines` (:164), `v_d_noorig_*` (:389, :413),
`emit_commands_route1_line_actually_runs` (:514); `cmd_decompose_roundtrip.rs`
`v_d_rt_the_recorded_mint_commands_are_still_the_ones_decompose_emits` (:314); `cli_network_version_bytes.rs`
`decompose_commands_recipe_runs_on_a_test_network` (:158); `keyless_hashlock_reaches_a_descriptor.rs` F-548
row (:208); `printed_remedy_runs.rs` (:434). If any of them changes, STOP (the proof is withholding a good
route).

## 5. Tests that change

Two existing tests pin the folded template and change (both found failing by the architect's plant of 2.0: 1014 run, 1012 pass, 2 fail):
- `crates/md-cli/tests/cmd_decompose.rs:297` `v_d_pair_a_single_fixed_path_descriptor_is_still_accepted`: keeps its purpose (the pair
  refuses, a single fixed-path descriptor is accepted) by asserting `--emit descriptor` rc 0 and `--emit template` refusing (rc 1,
  the 2.0 refusal text naming `/0/*`) instead of asserting `@0/48'/0'/0'/2'/0/*` on `--emit template`.
- `crates/md-cli/tests/liana_input_side.rs:331` `decompose_does_not_recognise_a_non_canonical_internal_key_use_site`: for the
  internal-key use-sites `/0/*`, `/0` and `/<0;1>/5/*` it asserts the refusal (rc 1, no `UNSPENDABLE(liana)` in stdout) instead of rc 0
  and `tr(@0/`; the other four cases (`/<2;3>/*`, `/<1;0>/*`, `/<0;1;2>/*`, `/*h`) are unchanged.

New: the rows of section 4. The triage files (`/scratch/cache/bh1119-md-t1/.../bughunt_1119.rs`
C1 test, `/scratch/cache/bh1119-md-t3/.../bh1119_md_t3.rs` I9 test) are evidence only; the rows above
replace them (signet, exact expected outputs).

## 6. FOLLOWUPS

Closes nothing in `design/FOLLOWUPS.md` (C1 and I9 are not filed there; the coordinator's bughunt records
close them). The author's `md-decompose-template-folds-use-site` line is not filed: the 2.0 check closes it in this row.
Lines to file after merge (not appended here; the coordinator direct-pushes them), verbatim from the architect ruling:
- [md-decompose-mk-admission-mirror] status=open grade=M kind=tooling owner=none filed=2026-11-22 src=design/FIX_bh1119_md_decompose_routes.md#9 refs=mk-record-admission-mirrored-in-md repo=md :: md decompose mirrors two mk-cli key-record rules (no derivation path; depth-1 parent fingerprint) to prove route 2; if mk-codec opens for F-C, consider exporting that admission and calling it instead.
- [mk-record-admission-mirrored-in-md] status=open grade=M kind=tooling owner=none filed=2026-11-22 refs=md-decompose-mk-admission-mirror repo=mk :: A change to mk-cli parse_key_record (keyfile.rs) or the provable-fingerprint loop (cmd/encode.rs) updates md-cli decompose's route-2 mirror in lockstep.

## 7. Gate line

`bash scripts/fix-bh1119-md-criticals-gate.sh` (builder commits it, with its coordinator catalog row):
runs the author's red.sh rows d1-d9 and d6b (and the new rows d1t-d3t, d1a, d10, d10b, d11, d12) against the built `md` and the installed `mk` (route 2 rows need `mk`; CI has none), records the `mk --version` it ran against in its output,
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

## 9. Rulings on the two STOPs (architect, `agent-reports/architect-md-bh4-stops.md`)

**Q1, route 2 proof: option A, TECHNICAL.** Folded as 2.3. Rejected: B (cross-repo, full track, would block a Critical fix
on an mk release, and would still drift against the holder's installed `mk`, which the printed line runs) and C (fails T8's
named case d4, and d10). If F-C's mk-codec release wants to export the admission it may ride; it gets no release of its own.

**Q2, the `--emit template` pointer: B's check, T8's pointer kept, TECHNICAL.** Folded as 2.0. md1 cannot hold the use-site, so no
correct template exists and refusal is the only correct result; A would re-send the holder to the wrong wallet, C leaves New-2 open.
T8 change: its example "C1's folded origin ... withheld with one line" becomes "C1 inputs refuse at the template check"; the principle stands.

**Grades (architect):** New-1 (depth-1/2 key, printed route 1 fails as printed) Important; New-2 (`--emit template`/`--emit all`
print a template that mints a different wallet) **Critical** (the author proposed Important). **Track:** row #4 stays small-track
(md-cli plus manual lockstep, no flag, type or command). **PRODUCT points:** none.
