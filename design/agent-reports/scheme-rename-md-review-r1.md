# Review r1: `md compose --preset` -> `--scheme` (f37f433)

Diff: `origin/claude/project-thread-utm1aa..origin/claude/project-thread-0jg5n7` (one commit, f37f433, 13 files). Brief: `cli-review/scheme-origin-path-plan-2026-11-26.md`.

## Verdict: 0 Critical / 0 Important / 4 Minor

## Critical
None.

## Important
None.

## Minor

**M1. A typo near the old spelling makes clap suggest the hidden alias.** `md compose --wrapper wsh --prese plain-multisig,2of3` prints `tip: a similar argument exists: '--preset'`, which breaks the plan's rule that everything printed names `--scheme`. clap's suggestion code counts hidden aliases. The likely case is an old script with a typo, and that run still exits 2, so nothing is accepted that should not be. A fix needs custom error handling, so file a follow-up rather than block (main.rs:321).

**M2. md-codec's `preset:` prefix still reaches compose's stderr.** `--scheme decaying-multisig,2of2,2of3,older1=2000,...` prints `md: preset: decaying tiers must unlock ...` (md-codec/src/compose/mod.rs:521), while every md-cli refusal now says `scheme <name>`. FOLLOWUPS `compose-preset-shape-error-prefix` defers it to a library round. But md-codec 0.49.0 is still unreleased (CHANGELOG:151), so it could change in this PR at no version cost. The only text-matching dependents are the two asserts at cli_compose_preset.rs:341/354; the toolkit has no `"preset: "` match. Either way is defensible. If it stays deferred, the follow-up should say why.

**M3. Stderr changed beyond the flag name.** The `preset <name>` refusal prefixes became `scheme <name>`, and `presets cannot express` became `schemes cannot express` (compose.rs:333, 384, 405, 509). The CHANGELOG states this. I searched mnemonic-toolkit, mnemonic-gui, mnemonic-engrave and refugium-wallet and found nothing that matches these strings. Low risk.

**M4. The tests could be tighter.**
- cli_compose_scheme_alias.rs:101 checks that stderr contains `--scheme`. The Usage line alone satisfies that, so only the `!contains("--preset")` check has force.
- The `--json` help (main.rs:328) says "the resolved scheme name" but does not say the key is `preset`. The README and json-schema doc do say it.

## Checked and sound
- The `ArgGroup` id is now `scheme`. No other `conflicts_with` or `requires` names `preset`.
- `--preset`, `--preset=` and the alias mixed with `--scheme` all work:
  - repeated, or mixed with `--scheme`: exit 2, `'--scheme <SCHEME>' cannot be used multiple times`
  - with no value: names `--scheme`
  - with `--path`: the conflict names `--scheme`
- `--json` still uses the key `preset`, and the JSON code is untouched.
- `--preset` is absent from `md compose -h/--help`, `gen-man` (md-compose.1) and `gui-schema`, which lists `--scheme` only.
- Replaying the golden through the alias is sound. The alias resolves to the same arg id, and the golden still checks byte identity with md-cli 0.18.0 for all 13 rows. If the alias is ever dropped, that test fails loudly.

## Commands run
- `CARGO_BUILD_JOBS=2 cargo test --locked --workspace --all-targets --all-features --no-fail-fast`: 155 result lines, 1968 passed, 0 failed, 6 ignored, exit 0. The 3 tests in `cli_compose_scheme_alias` passed. Log: /home/user/scratch-md-review/test.log
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`: exit 0
- `cargo fmt --all --check`: exit 0
- Ran the built `md` by hand: the alias edge cases, `gui-schema`, `gen-man --out`, and help.

## Fold (controller)

- M2 folded: md-codec `ComposeError::PresetShape` displays `scheme: <why>`; two asserts in cli_compose_preset.rs follow; md-codec 0.49.0 CHANGELOG notes it; FOLLOWUPS `compose-preset-shape-error-prefix` marked resolved.
- M4 folded: the alias test now asserts clap's `'--scheme <SCHEME>'` naming, and the `--json` help says the key stays `preset`.
- M1 (clap typo tip suggests the hidden alias): accepted as clap behaviour, same in every repo of this rename.
- M3: intended and recorded in CHANGELOG.
