# IMPL report — `emit-md1-has-no-transcribe-ready-form`

Closes FOLLOWUPS `emit-md1-has-no-transcribe-ready-form`, filed
2026-08-31 from `REVIEW-mdcli-mini-whole-diff-r1` M2. Implementation
commit `c8c3a4fd`; FOLLOWUPS closure commit `01aa0412`; this report is
the final commit on branch `emit-md1-form` (base `c589054d`).

## The defect (M2, as filed)

`md descriptor <keyless md1…> --from-mk1 … --emit md1` minted the keyed
card the S→K bridge exists for, but offered no `--out`, no
`--group-size`, no `--separator`, and printed no engraving card at all
on stderr — the transcribe-ready form `md encode` supplies, on the one
card `md encode` cannot mint (the depth-3/4 rule). An operator minting
for a plate had to pipe and format the bare strings by hand.

## `md encode`'s contract, as measured (not assumed)

Measured against the built binary (`./target/debug/md`, pre-fix
source), fixture material from
`crates/md-cli/tests/fixtures/pathological/keys.txt` and the same
template the N2 oracle test already uses (`B1_TPL` in
`n2_emit_md1.rs`):

```
$ md encode "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))" \
    --key @0=<xpub1> --key @1=<xpub2> \
    --fingerprint @0=73c5da0a --fingerprint @1=73c5da0a
exit=0
STDOUT (unbroken, one card per line, newline-terminated):
md1f0ghpps9q2tvyyy5jmpprj5qqcy8ppgtcgu79mg9tnchdq59wpyhwsv0jskp2rsal4egz4eqdccu772e060rs
md1f0ghppsf5859p875x67p5s3wem7sgluxl3d2a3syx3m7halwd7s7d5e8l2xm3y3xzfmadfjcjukwzsuw7pydp
md1f0ghppsje20ur0anz7jwkzae8efejcxy50llpx82qfmryv7l68w6hzragnj3g5qrl85zeapccg28cpyh2qcaz
md1f0ghppse8wq0vdczfyy55tqsd5576trsa3p40nfpd7hsyjyf7vlx6hk2j6ckr4wf0m3sq5klzdk64u37vh
STDERR (chunk-set-id, then the grouped card, then group size / separator, then notes):
chunk-set-id: 0x7a2e1
md1f0 ghpps 9q2tv yyy5j mpprj 5qqcy 8ppgt cgu79 mg9tn chdq5 9wpyh wsv0j skp2r sal4e gz4eq dccu7 72e06 0rs
md1f0 ghpps f5859 p875x 67p5s 3wem7 sglux l3d2a 3syx3 m7hal wd7s7 d5e8l 2xm3y 3xzfm adfjc jukwz suw7p ydp
md1f0 ghpps je20u r0anz 7jwkz ae8ef ejcxy 50llp x82qf mryv7 l68w6 hzrag nj3g5 qrl85 zeapc cg28c pyh2q caz
md1f0 ghpps e8wq0 vdczf yy55t qsd55 76trs a3p40 nfpd7 hsyjy f7vlx 6hk2j 6ckr4 wf0m3 sq5kl zdk64 u37vh
group size: 5
separator: space
note: stdout is watch-only — public keys only, cannot spend
```

`--group-size 0`, `--group-size 8` and `--separator space` (explicit)
change only the grouped-card lines and the `group size:`/`separator:`
values; stdout is unaffected in every case (always unbroken). `--out
FILE` moves the whole artifact off stdout (stdout empty) into FILE,
created 0600 (verified `stat -c '%a'` → `600`), and does NOT suppress
the chunk-set-id, the engraving card or the notes on stderr.
`--force-chunked` and `--policy-id-fingerprint` were measured but are
out of this FOLLOWUP's scope (the entry names only `--out`,
`--group-size`, `--separator`) and were not added to `--emit md1`.

Source confirms the measurement: `crates/md-cli/src/cmd/encode.rs`
`run()` — mint → (chunk-set-id to stderr if chunked) → write body
(stdout or `--out` file) → `emit_engraving_card` (stderr: grouped
card(s), `group size:`, `separator:`) → advisories.

## What was mirrored, and where it single-sources

- `crates/md-cli/src/cmd/encode.rs`: `emit_engraving_card` changed from
  private to `pub(crate)` — it was already the sole renderer for `md
  encode`'s stderr card; nothing about its body changed.
- `crates/md-cli/src/cmd/descriptor.rs`: `DescriptorArgs` gained
  `out_file: Option<&'a Path>`, `group_size: usize`, `separator: char`.
  `emit_md1_card` gained the same three parameters and now: writes the
  body via `crate::cmd::write_artifact` when `out_file` is `Some`
  (identical to `md encode`'s own `--out` branch) or `print!`s it
  otherwise; then calls `crate::cmd::encode::emit_engraving_card` with
  the SAME cards/group_size/separator, in the SAME relative position
  (immediately after the artifact write) `md encode` uses. Card
  minting itself was already single-sourced through
  `crate::cmd::encode::mint_md1_cards` (P5); this fold reuses it
  unchanged.
- `crates/md-cli/src/main.rs`: `Command::Descriptor` gained `--out`,
  `--group-size` (`default_value_t = 5`), `--separator`
  (`default_value = "space"`, same `parse_separator` fn `md encode`
  uses) — same flag names, defaults and value parser as `md encode`.
  All three carry `requires = "emit"` (clap-structural, not a runtime
  check): without `--emit md1` they refuse at parse time rather than
  being silently accepted and discarded — the I4 defect class this
  cycle's review keeps naming on this verb's other flags
  (`--key`/`--fingerprint`/`--path` vs `--template`).
- No wire-format, minting or seating logic touched. The minted STRINGS
  are unchanged (same `mint_md1_cards` call, same arguments).

## Divergence from `md encode`'s stderr shape (deliberate, not a gap)

`emit_seating_notes` (an S-row-only insertion — composed-wallet-id,
WALLET-CONFIRMED/stub, address-0 notes — with no `md encode`
counterpart) is emitted AFTER the mirrored engraving-card block and
BEFORE the shared advisories (legacy-P2SH, pathless, output-class),
preserving its pre-existing relative position (it already ran between
the body-write and the advisories before this fold; the engraving
card was inserted ahead of it, not after).

## Row evidence

New tests, `crates/md-cli/tests/n2_emit_md1.rs`, section "step 3b — the
transcribe-ready form":

| Row | Test |
| --- | --- |
| default engraving-card shape matches `md encode`'s | `n2_emit_md1_default_engraving_card_matches_md_encodes_default_shape` |
| `--group-size` differential (0, 8, 3) | `n2_emit_md1_group_size_behaves_identically_to_md_encodes` |
| `--separator` differential (explicit `space`) | `n2_emit_md1_separator_behaves_identically_to_md_encodes` |
| `--out`: stdout empty, file contents match, 0600 | `n2_emit_md1_out_writes_the_file_and_stdout_matches_md_encodes_out_contract` |
| all three refuse without `--emit` | `n2_group_size_separator_and_out_require_emit` |

**Red-then-green, verified by `git stash` on the source (not the
tests), not assumed.** With the three source files stashed back to
pre-fix (tests unchanged), `cargo nextest run -p md-cli --test
n2_emit_md1` reported 13 passed / 4 failed:

- `n2_emit_md1_default_engraving_card_matches_md_encodes_default_shape`
  — FAILED (`left: []`, oracle had 6 lines: pre-fix stderr carried no
  engraving card at all).
- `n2_emit_md1_group_size_behaves_identically_to_md_encodes` — FAILED
  (`unexpected argument '--group-size'`: the flag did not exist yet).
- `n2_emit_md1_separator_behaves_identically_to_md_encodes` — FAILED
  (same cause).
- `n2_emit_md1_out_writes_the_file_and_stdout_matches_md_encodes_out_contract`
  — FAILED (`unexpected argument '--out'`).
- `n2_group_size_separator_and_out_require_emit` — PASSED, vacuously:
  pre-fix, any of the three flags was an unknown-flag clap error (exit
  2), which coincidentally satisfies the same assertion the post-fix
  `requires = "emit"` refusal (also exit 2) does. Not a discriminating
  row before the fold; a real regression guard after it.

After `git stash pop` (restoring the fix), all 17 tests in the file
pass, including the 6 new ones.

**No existing P5 row pinned the old bare-strings default; none needed
flipping.** Checked by grep across `n2_emit_md1.rs` and
`r3_verify_against.rs` (the only other file invoking `--emit md1`
against `md descriptor`) for full-stderr equality assertions or
negative "engraving card absent" checks: none exist. Every existing
row either checks `status.success()`, filters stdout for `md1`-prefixed
lines, or checks stderr with `.contains(...)` — all unaffected by
additional stderr content, and confirmed by the full-suite run below
(0 regressions).

## Gate output (`scripts/phase-gate.sh`, run twice — once red on `cargo
fmt --check` after the first pass, clean on the second after `cargo
fmt`)

```
=== cargo nextest run --locked --all-features ===
Summary [ 0.850s] 1191 tests run: 1191 passed, 2 skipped

=== cargo test --workspace --doc ===
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

=== cargo clippy --locked --all-targets --all-features -- -D warnings ===
Finished `dev` profile [optimized + debuginfo] target(s) in 0.75s

=== cargo fmt --check ===
(clean, after `cargo fmt` fixed 3 line-wrap diffs in descriptor.rs / n2_emit_md1.rs)

=== cargo doc --workspace --no-deps --document-private-items --all-features ===
Generated /scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini/target/doc/md/index.html and 1 other file

=== design/display-grouping-vectors.tsv.sha256 ===
display-grouping-vectors.tsv: OK

phase-gate: all six steps passed
```

## Files changed

- `crates/md-cli/src/main.rs` — `--out`/`--group-size`/`--separator` on
  `Command::Descriptor`, `requires = "emit"`; threaded through
  `dispatch()`.
- `crates/md-cli/src/cmd/descriptor.rs` — `DescriptorArgs` gains the
  three fields; `emit_md1_card` writes via `--out` and calls the
  shared `emit_engraving_card`.
- `crates/md-cli/src/cmd/encode.rs` — `emit_engraving_card` visibility
  `fn` → `pub(crate) fn`; doc comment updated to name the new caller.
  No behavioural change to `md encode`.
- `crates/md-cli/tests/n2_emit_md1.rs` — 6 new tests (167 added
  lines), TDD red-then-green as above.
- `CHANGELOG.md` — extended the existing `[Unreleased]` "`md descriptor
  --emit md1`" bullet (the cycle has not shipped a release since that
  entry landed, so this is an amendment to the same entry, not a new
  dated section).
- `design/FOLLOWUPS.md` — closure paragraph on
  `emit-md1-has-no-transcribe-ready-form`, citing `c8c3a4fd` (separate
  commit `01aa0412`, matching this file's own convention of a
  dedicated `followups:` commit after the implementation).

## Commits on `emit-md1-form` (base `c589054d`)

1. `c8c3a4fd` — implementation + tests + CHANGELOG, gate output in the
   message.
2. `01aa0412` — FOLLOWUPS closure citing `c8c3a4fd`.
3. this report (final commit).
