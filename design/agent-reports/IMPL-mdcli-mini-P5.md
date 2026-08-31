# IMPL report — mdcli-mini P5 (N2: `md descriptor --emit md1`, the S → K cell)

Worktree: `/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`, branch
`mdcli-mini`. Started at `02a09157` (P1–P4 landed, gate green at 1158 tests).
Three commits, one per plan step group:

- `85bac534` — P5.1–2: the flag, the emission, the input-mode refusals
- `a13b4738` — P5.3: the oracle rows
- `cd7785a2` — P5.4: the matrix flip in all four homes (final SHA)

Plan: `design/IMPLEMENTATION_PLAN_mdcli_mini.md`, "P5 — N2: `--emit md1`"
steps 1–4. Spec: `design/SPEC_mdcli_mini.md`, "N2 — mint a keyed card from a
seating result". FOLLOWUPS slug: `md-cannot-mint-a-keyed-card-from-a-split-set`
(owning phase P5; the entry itself is left for P7's reconciliation sweep, which
is where the plan's burndown puts every closure).

**No deviations. No contradiction between plan and spec was found.** Two
judgment calls the plan left open are reasoned below (§6).

---

## 1. Changes, file by file

### New

| file | what |
| --- | --- |
| `crates/md-cli/tests/n2_emit_md1.rs` (502 lines) | the 11 P5 rows: emission, both input-mode refusals as rendered lines, the `--json` conflict, the P4 guard-scope discharge, both oracles, the refusal-survives row, the `--seat` composition row, and a fixture-provenance pin. |
| `design/agent-reports/IMPL-mdcli-mini-P5.md` | this file. |

### Changed — source

| file | what |
| --- | --- |
| `crates/md-cli/src/main.rs` | `--emit <FORM>` on `Command::Descriptor`, `value_enum`, `conflicts_with = "json"`; the field threaded into `DescriptorArgs`. |
| `crates/md-cli/src/cmd/descriptor.rs` | new `pub enum Emit { Md1 }` (clap `ValueEnum`); `DescriptorArgs::emit`; the input-mode check called first in `run` after P4's `check_from_mk1_arity`; the emission branch after the wallet-policy check; new private `check_emit_md1_input_mode` and `emit_md1_card`. |
| `crates/md-cli/src/cmd/encode.rs` | `mint_md1_cards(descriptor, force_chunked) -> (Vec<String>, Option<u32>)` extracted from `encode::run` verbatim (the F-136 rationale comment moved onto it as a doc comment); `run` now calls it and prints the `chunk-set-id` line when the second element is `Some`. No behaviour change to `md encode` — its stdout, its stderr order and its `--json` branch are untouched. |
| `crates/md-cli/src/seat/mod.rs` | the matrix cell + the prose the flip falsified (§5). |

### Changed — design docs

`design/BRAINSTORM_wallet_form_converter.md`,
`design/SPEC_wallet_form_converter.md`,
`design/IMPLEMENTATION_PLAN_wallet_form_converter.md` — the matrix cell and its
surrounding prose (§5).

---

## 2. Step 1 — emission from the seating result

`md descriptor <keyless md1…> --from-mk1|--from-mk1-file … --emit md1` puts the
KEYED md1 card on stdout instead of the concrete descriptor. `--seat` composes
with it. The `chunk-set-id` line goes to stderr, where `md encode` puts it, so
stdout stays the artifact and nothing else.

**It is not the blocked bridge, and does not relax it.** `md encode --key`'s
depth-3/4 admission rule is untouched and nothing routes through it. The card is
minted from the seating result directly; a keyed card's `Pubkeys` TLV is 65
bytes (chain code ‖ compressed point) with no depth field, so the depth-0 keys a
card composes lose nothing. The origin metadata is the policy card's own,
carried through because `seat::compose::compose` fills the `Pubkeys` TLV and
touches nothing else — no new code was needed for "carries the origin metadata
learned from seating"; the row set measures that it does.

Both card channels reach it: `collect_mk1` (`main.rs:724` at this tree) merges `--from-mk1`
and `--from-mk1-file` before `descriptor::run` sees either, and the
`--from-mk1-file` row asserts the two spellings mint byte-identically.

---

## 3. The oracle (plan step 3) — PRIMARY form, measured, no fallback taken

The spec's PRIMARY form is executable at this tree, so the TLV-field-by-field
fallback was **not** taken and needs no controller sign-off.

Inputs, both run against the P5 tree's `target/debug/md`:

```
A  md descriptor <v-b1-wallet.txt's md1 policy card> \
     --from-mk1 <each of its 5 mk1 chunks> --emit md1

B  md encode "wsh(sortedmulti(2,@0/48'/0'/0'/2'/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))" \
     --key @0=<keys.txt record 1 xpub> --key @1=<keys.txt record 2 xpub> \
     --fingerprint @0=73c5da0a --fingerprint @1=73c5da0a
```

B is the spec's form exactly: the template carrying INLINE per-slot origins,
plus one `--fingerprint @i=HEX` per fingerprint the policy card declares. The
template and both fingerprint flags are v-b1-wallet.txt's own recorded mint
command; `n2_fixture_header_still_records_the_mint_the_oracle_reproduces`
asserts the fixture still records them, so a regenerated fixture cannot leave
the oracle reproducing a mint nobody performed.

Result — **byte-identical**:

```
=== A stdout ===
md1f0ghpps9q2tvyyy5jmpprj5qqcy8ppgtcgu79mg9tnchdq59wpyhwsv0jskp2rsal4egz4eqdccu772e060rs
md1f0ghppsf5859p875x67p5s3wem7sgluxl3d2a3syx3m7halwd7s7d5e8l2xm3y3xzfmadfjcjukwzsuw7pydp
md1f0ghppsje20ur0anz7jwkzae8efejcxy50llpx82qfmryv7l68w6hzragnj3g5qrl85zeapccg28cpyh2qcaz
md1f0ghppse8wq0vdczfyy55tqsd5576trsa3p40nfpd7hsyjyf7vlx6hk2j6ckr4wf0m3sq5klzdk64u37vh
=== A stderr ===
chunk-set-id: 0x7a2e1
note: composed wallet id 6a801edb · policy shape id aad0e0e0
note: 2 card(s) WALLET-CONFIRMED — stub matches this exact composed wallet: 20933, 8392c
note: address 0 (chain 0, index 0) is bc1qf4jpv99wj36eqez9fzxrzww6sdy97uw5gmgp38t6trqpk5lre8qsv3ttqz — compare against your wallet software before trusting.
note: stdout is watch-only — public keys only, cannot spend
=== B stdout ===  (identical to A's, 4 strings)
=== B stderr ===  chunk-set-id: 0x7a2e1

$ cmp A B
IDENTICAL: 353 bytes, 4 card strings
sha256 A: 65862456755d46aa9cb59e3a66655fc9b3d79fe2e4d3d05ee46b8f1cca57e229
sha256 B: 65862456755d46aa9cb59e3a66655fc9b3d79fe2e4d3d05ee46b8f1cca57e229
```

The chunk-set id matches too (`0x7a2e1` on both), which is the label an operator
writes beside the plate.

Two pre-checks, run before any code was written, established that this would be
a real comparison rather than a tautology:

- `md encode "<B1_TPL>" --fingerprint @0=73c5da0a --fingerprint @1=73c5da0a`
  reproduces v-b1-wallet.txt's committed keyless policy card byte-for-byte
  (`md15pfdsssjjtvyyw2sqrqsuy9p0prnchdq4w0za5zst076dhvcvrcvh`), so the keyed
  mint differs from the fixture's policy card only by the `Pubkeys` TLV;
- composing the split set and composing the B card yield the identical concrete
  descriptor, ending `#d48r7auj`.

**SECONDARY.** The minted card is spend-equal to `v-spendeq-keyed.txt` — the
same wallet minted with DIFFERENT declared fingerprints (b8688df1 vs 73c5da0a) —
and derives the same address 0. Both relations are computed in
`tests/common/facts.rs` from a rust-miniscript parse of the emitted descriptor
STRING, never by asking `src/seat` whether it succeeded.

---

## 4. Row evidence, red → green

### RED, before any implementation existed

All 11 rows written first; the binary at `02a09157` had no `--emit`:

```
$ cargo nextest run --locked --all-features -p md-cli --test n2_emit_md1
    error: unexpected argument '--emit' found
      tip: to pass '--emit' as a value, use '-- --emit'
    Usage: md descriptor <PHRASES|--template <TEMPLATE>|--from-mk1 <STRING>...>
Summary 11 tests run: 1 passed, 10 failed, 0 skipped
```

The one passing row is `n2_fixture_header_still_records_the_mint_the_oracle_reproduces`,
which reads fixture text and calls no binary — correct, and worth naming so the
count is not mistaken for a partial implementation.

### GREEN, after

```
$ cargo nextest run --locked --all-features -p md-cli --test n2_emit_md1
Summary 11 tests run: 11 passed, 0 skipped
```

### MUTATION-CHECKED — the oracles can fail, and fail at different things

A green row proves little on its own, so both oracles were broken deliberately
and the failure observed:

| mutation (in `emit_md1_card`, reverted after each) | result |
| --- | --- |
| `mutant.tlv.fingerprints = None` before minting | **PRIMARY fails** — a genuinely different card (`md1f4axwps…` vs `md1f0ghpps…`), not a formatting difference. **SECONDARY passes, correctly**: spend-equality excludes origin metadata by definition, which is exactly why N2 specifies two relations rather than one. |
| seat slot 0's key into every slot | 5 rows fail, **both oracles among them** (`…is_byte_identical…` and `…is_spend_equal…`). |

### Row → obligation map

| plan/spec obligation | row |
| --- | --- |
| mints the keyed card from the seating result | `n2_emit_md1_mints_a_keyed_card_from_the_seating_result` |
| one row uses the `--from-mk1-file` spelling | `n2_emit_md1_mints_the_same_card_from_the_from_mk1_file_spelling` |
| P4 guard-scope note (r1 M6): the literal `--emit md1` must not trip the md1-prefix guard | `n2_emit_md1_is_not_swallowed_or_refused_by_the_from_mk1_arity_guard` |
| `--emit md1` + `--template` refuses naming `md encode` | `n2_emit_md1_with_a_template_refuses_naming_md_encode` |
| `--emit md1` on a keyed-card positional refuses as a re-emit | `n2_emit_md1_on_a_keyed_card_positional_refuses_as_a_re_emit` |
| PRIMARY oracle: byte-identity against `md encode` | `n2_emit_md1_is_byte_identical_to_the_md_encode_mint_of_the_same_wallet` |
| SECONDARY: spend_equal + address-0 against the keyed fixture | `n2_emit_md1_is_spend_equal_to_the_keyed_fixture_card_and_shares_address_zero` |
| a seating refusal survives unchanged under `--emit md1` | `n2_emit_md1_leaves_a_seating_refusal_exactly_as_it_was` |
| "composes with `--seat`" | `n2_emit_md1_composes_with_seat` |
| (this phase's own) `--json` is not silently discarded | `n2_emit_md1_and_json_are_declared_mutually_exclusive` |
| (this phase's own) the oracle still reproduces the fixture's mint | `n2_fixture_header_still_records_the_mint_the_oracle_reproduces` |

**Acceptance 4.** Both diagnostics this phase introduces are asserted as the
full RENDERED stderr line from the `md: ` prefix onward, with exactly one such
line per run, via `assert_one_rendered_line` (`assert_eq!` on the literal text).
Neither contains the word "invalid". Both exit 2 as `CliError::BadArg`. The
`--json` conflict introduces no diagnostic of md's own — it is clap's
"cannot be used with", which is why that row asserts clap's text and not a
rendered md line.

---

## 5. Step 4 — the matrix flip (`cd7785a2`)

The cell now reads, byte-identical in all four homes:

```
| **S** keyless card + mk1 strings | ✓ P2 (the seating engine, shipped this cycle) | ✓ P2 (shipped this cycle) | ✓ `md descriptor --emit md1` (mdcli-mini P5 — minted from the seating result, never through the depth-3/4 bridge) | — |
```

```
$ scripts/matrix-identity-check.sh
matrix identical across 4 homes, 6 lines each:
  design/BRAINSTORM_wallet_form_converter.md
  design/SPEC_wallet_form_converter.md
  design/IMPLEMENTATION_PLAN_wallet_form_converter.md
  crates/md-cli/src/seat/mod.rs
sha256: 9e0cd908f1e7f34de9412241534ab6e2c90f5fae162406f0dcb17698eeb70d02
```

(The pre-flip sha was `0527deee4a60b60385b07d7bfda8d5987672d2d5195ced56db0780836a5baab1`.)

**The prose moved with the cell, in the same commit**, because a diff falsifies
text it never touches and the identity check only compares six table lines:

1. the "Flipped at C4 close" paragraph's present tense — "One cell … **is left**
   ✗" — becomes a record of that close ("**was left** ✗ at that close"), and a
   second paragraph states the P5 flip and that the depth-3/4 bridge is
   unchanged and unused by it. Both paragraphs are identical in all four homes.
2. `seat/mod.rs`'s "A keyed card is an output of the D row via the
   `md encode --key` bridge, but **NOT of this engine's own S row**" — flatly
   false as of `85bac534` — now names both routes.
3. the spec's and plan's "Output forms: … keyed card (via the existing
   `md encode --key` bridge)" now names the S-row route too.
4. `SPEC_wallet_form_converter.md`'s "The S → keyed-card matrix cell **stays** ✗
   with that reason" now says where it closed.

Left deliberately untouched: the C4-era MEASUREMENTS in all three docs (the
brainstorm's decision 2, the spec's P2 paragraph, the plan's §6 filing list).
They are true statements about the `md encode --key` bridge, which P5 does not
use, and rewriting history to match a later flip would destroy the record of why
the cell was ✗.

---

## 6. Judgment calls (the plan deferred the mechanics; neither is a deviation)

**(a) The authoring gate travels to the new minting surface.**
`emit_md1_card` runs `md_codec::validate::validate_relative_timelocks` before it
mints, for the reason `cmd/encode.rs` gives at its own call site: BIP-68 reads
only bits 31, 22 and 0–15 of a relative locktime, so a plate can assert a
four-year lock the chain releases in three months, and `md encode` gates that on
its authoring surface. `--emit md1` is an authoring surface too, and which
command engraved the plate makes no difference to what the plate claims. It
cannot regress anything: the gate is reachable only through a flag that did not
exist before this commit. Not specified by plan or spec — recorded here as the
one behaviour added beyond the letter of step 1.

**(b) `--emit` conflicts with `--json` at the clap surface, rather than growing
a JSON envelope or silently ignoring the flag.** `md descriptor --json` emits
`{schema, network, chain, descriptor}` and `--emit md1` produces no descriptor;
the spec's Non-goals say the cycle ships no new JSON envelope (R8 is parked); and
a flag accepted and silently discarded on this exact verb is
REVIEW-converter-whole-diff-r1 I4's own finding. A `conflicts_with` declaration
costs one attribute, introduces no diagnostic of md's own, and is the same
mechanism `--key`/`--fingerprint`/`--path` already use here.

**(c) Both input-mode refusals are `CliError::BadArg` (exit 2), not exit-1
content refusals.** Every flag is spelled correctly and every value parses; what
is wrong is the COMBINATION, which is the class clap itself reports at exit 2.
Nothing is being said about the material, so `Seat`/`Unsupported` would misfile
it. This matches P4's own guard 3, which chose exit 2 for the same reason.

A note on the second refusal's wording: it is reached with a card on the
positional and no key cards, which covers a KEYED card (the re-emit the spec
names) and also a keyless one. The sentence "These md1 phrases are a card
already, and re-emitting a card you are holding would hand back what you pasted
in" is true of both, so one message serves the whole branch; the row asserts it
on the keyed card, which is the case the spec names.

---

## 7. Gate (final, at `cd7785a2`)

```
$ ./scripts/phase-gate.sh            # exit 0
cargo nextest run --locked --all-features: 1169 tests run, 1169 passed, 2 skipped
    (P4 closed at 1158; +11 = this phase's row file)
cargo test --workspace --doc: ok. 0 passed; 0 failed; 0 ignored
cargo clippy --locked --all-targets --all-features -- -D warnings: clean
cargo fmt --check: clean
cargo doc --workspace --no-deps --document-private-items --all-features: clean
    under RUSTDOCFLAGS="-D warnings"
design/display-grouping-vectors.tsv: OK
phase-gate: all six steps passed

$ scripts/matrix-identity-check.sh   # exit 0
matrix identical across 4 homes, 6 lines each
sha256: 9e0cd908f1e7f34de9412241534ab6e2c90f5fae162406f0dcb17698eeb70d02
```

The gate's stated blind spot is unchanged and untouched by this phase: the
freebsd/musl jobs and the windows/macos legs of the test matrix are CI-only and
are covered by the push-ritual staging run.

## 8. Notes for the phases after this one

- **P7 owns the FOLLOWUPS closure.** `md-cannot-mint-a-keyed-card-from-a-split-set`
  is answered by `85bac534`; its entry in `design/FOLLOWUPS.md` is untouched
  here, matching P2–P4's practice and the plan's burndown, which routes every
  closure through P7's reconciliation sweep.
- **P7's cross-repo docs pass** must list `--emit md1` on `md descriptor` among
  this cycle's new surface (the plan already names it).
- **P6 note.** `cmd::encode::mint_md1_cards` is now the single mint; if R3's
  `--verify-against` grows any minting behaviour it should go through it rather
  than around it.
- **No Go-port lockstep is triggered.** This phase changes no normative codec
  behaviour: no wire format, no identity/stub algorithm, no validation and no
  admission rule. It adds a CLI output form over an existing composition and one
  extracted helper. `md encode`'s and `md descriptor`'s existing outputs are
  byte-unchanged, which the 1158 pre-existing tests still passing is the check
  on.

## 9. Final SHA

`cd7785a2` — HEAD of `mdcli-mini` in this worktree, before this report's own
commit.
