# IMPL — mdcli-mini P2 (N1 mint/compose refusals, template path)

Implementer report. Worktree `/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`,
branch `mdcli-mini`, from `35e616ce` (P1 landed).

**Outcome: P2 steps 1–8 executed in full. Phase gate GREEN — all six steps
passed, 1139 tests run / 1139 passed / 2 skipped. No deviation from the plan.
One site outside the plan's enumeration was found by running the suite and is
recorded below.**

---

## 1. The fixture mint (plan step 1) — transcript

Binary: `/scratch/code/shibboleth/descriptor-mnemonic/target/debug/md`
(`b8a64938` code surface — the plan's baseline revision; the `md encode` path
is untouched by P1). Key material: `crates/md-cli/tests/fixtures/pathological/keys.txt`
record 1, `[73c5da0a/48'/0'/0'/2']`.

```
$ md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" \
    --key @0=xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf \
    --path "48'/0'/0'/2'" --group-size 0
chunk-set-id: 0xed813          <- stderr
md1fakqnqspqztvyyy4qqxppcgg4gythgx8egtq4pcwl6u5p2us6r6zsnl2rd0q6gghvalgywfyx3z0nn28m7t
md1fakqnqs0cdlz64mrqgdrha0m7umapumfj075dhzfzvynh66n94j5lcxlmx9ayav9mj0jjqpx5yl5n7q5v9j
exit 0, 2 chunks

$ md encode "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" \
    --key @0=<KEY 1> --key @1=<KEY 1> --path "48'/0'/0'/2'" --group-size 0
chunk-set-id: 0x00ee4          <- stderr
md1fqrhypspq2tvyyy4qqxppsg2qknq2zc2uzfwaqcl9pvz58pmltjs9tjrg0g2z0agd4ufp7hduum6kwk5
md1fqrhypsgxjz9m806prlsm794tkxqs6806lhaeh6reknylagmwyjycf8044xtt9flsduceq9hcyzfej5t
md1fqrhypshvch5n4shwf72wa6p372zc9gwrh7h9q2hyxs7s5yl6smtcxjz9m806prlsmup3ltrsqzp8y40
md1fqrhypsut2hvvpp5wl4l0mn058ndxfl63kufyfsjwlt2vkk2nlqmlvch5n4shwf72gq83vs87hd9720k
exit 0, 4 chunks
```

**Both identities match the plan's measured values: `0xed813` / 2 chunks and
`0x00ee4` / 4 chunks.** No STOP condition.

Written to `crates/md-cli/tests/fixtures/n1/r-n1a-keyed.txt` and
`.../r-n1d-delta.txt`, provenance headers in the V-BOUND-REF pattern of
`crates/md-cli/tests/fixtures/seating/generate.sh`, each stating that it has no
generator by design and that regenerating it requires checking out `b8a64938`.

**Verified KEYED, not keyless** (r1 I7's hazard — the keyless spelling mints a
one-chunk card that composes nothing). At the baseline binary each fixture
composes a concrete wallet at exit 0:

```
$ md descriptor <the two r-n1a chunks>
wsh(sortedmulti(2,xpub661My…/<0;1>/*,xpub661My…/<0;1>/*))#kuunuuvp     exit 0
$ md descriptor <the four r-n1d chunks>
wsh(multi(2,xpub661My…/<0;1>/*,xpub661My…/<2;3>/*))#3sxca8l0            exit 0
$ md address <the two r-n1a chunks> --count 1
bc1ql5j095gqvdv6ugccf956pduc2e0vevtfnf9r72nhmln9lf8tlmmsd9ujlz          exit 0
```

That is r3 I-1's premise for P3's card-input refusals, measured rather than
inherited. The keyless contrast is a single 34-character string
(`md1yqfdsssj5qqcy8ppy4ztqaj4nwzw7`), which is what the keyed 2-chunk output
distinguishes itself from.

---

## 2. What changed, file by file

### New

| file | what |
| --- | --- |
| `crates/md-cli/src/parse/reuse.rs` (539 lines) | the classifier. `classify(occs, keys) -> Option<Finding>`, `Finding::message()`, `Disposition{Refuse,Warn}`, `check()`. 11 unit rows. |
| `crates/md-cli/tests/n1_admission_taxonomy.rs` (530 lines) | 20 integration rows: the six taxonomy diagnostics as full rendered lines, three anti-over-refusal controls, the hardening reachability probe, the shipped-floor boundary, `verify` warn, and `md compile`'s duplicate-key refusal. |
| `crates/md-cli/tests/fixtures/n1/r-n1a-keyed.txt` | the R-N1a card, `0xed813`. |
| `crates/md-cli/tests/fixtures/n1/r-n1d-delta.txt` | the R-N1d delta card, `0x00ee4`. |

### Changed — source

| file | what |
| --- | --- |
| `crates/md-cli/src/parse/mod.rs` | `pub mod reuse;` |
| `crates/md-cli/src/error.rs` | new `CliError::Unsupported(String)` → `md: unsupported: …`, exit 1. Doc states why `TemplateParse` could not carry R-N1c. |
| `crates/md-cli/src/parse/template.rs` | `parse_template_ext` gains `reuse: Disposition` and calls `reuse::check` between `lex_placeholders` and `resolve_placeholders`. `parse_template` passes `Refuse` and its doc names the verbs that inherit it. |
| `crates/md-cli/src/cmd/encode.rs` | passes `Refuse`. |
| `crates/md-cli/src/cmd/verify.rs` | passes `Warn`. |
| `crates/md-cli/src/cmd/build.rs` | plan step 5 — the stale "BIP 388 permits it and `md encode` mints it" comment corrected. |
| `crates/md-codec/src/validate.rs` | plan step 5 — says plainly that "not a duplicate" is about *this comparison*, not a licence to mint, and why the wire floor keeps the narrower boundary. |
| `crates/md-codec/src/test_vectors.rs` | the three MANIFEST vector replacements (§5). |
| `crates/md-cli/src/cmd/vectors.rs` | a row for step 6's ruled invocation point: the generator is fail-closed. |

### Changed — tests and artifacts

`crates/md-cli/tests/duplicate_key_slots.rs` (2 rows flipped),
`sortedmulti_a_taproot_leaf.rs`, `cli_unhardened_origin_note.rs`,
`cli_keyed_excess_origin_note.rs`, `cmd_encode.rs`,
`crates/md-cli/tests/fixtures/seating/generate.sh`, and the 15 regenerated
corpus files under `crates/md-codec/tests/vectors/`.

Whole-phase diff: **34 files, +1750 / −219**.

---

## 3. Placement, and the one design decision the plan left open

The spec fixes the classifier's *input* (occurrence list + per-`@i` key
bindings), one implementation per predicate, disposition as a parameter, and
nothing new inside `encode_payload`'s validator set. It leaves the invocation
point to the plan, which rules only that `md vectors`' `parse_template` path
must hit it.

**Chosen: inside `parse_template_ext`, between `lex_placeholders` and
`resolve_placeholders`.**

It *must* precede `resolve_placeholders`, which is not a free choice: that
function collapses same-`@i` occurrences and already refuses four of the five
Family-1 cases first, with one generic message naming neither the axis nor the
authority —

```
$ md encode "wsh(multi(2,@0/<0;1>/*,@0/<2;3>/*))" --key @0=<K> --path 48'/0'/0'/2'
md: template parse error: @0 appears with inconsistent path/multipath/hardening
```

— which is exactly the prefix R-N1c's mandate forbids. Only R-N1a survives that
collapse today, which is why R-N1a is the case that *mints*.

Dispositions at the call sites: `parse_template` → `Refuse` (so `md descriptor
--template`, `md address --template` via `cmd::build`, and `md vectors`' own
generator loop all inherit it); `md encode` → `Refuse`; `md verify --template`
→ `Warn`.

**`md verify` is the decision the placement forced.** Any single-source
placement that `verify` shares makes its disposition a P2 question, and the
spec puts `verify` in the WARN set (Acceptance 5: a plate carrying a
newly-refused shape must stay checkable — and both such plates now exist, in
`tests/fixtures/n1/`). Passing `Refuse` there would have broken Acceptance 5;
inventing a third, silent disposition would have put a variant in the type that
the spec does not have. So `Warn` is wired and implemented, and per the
Acceptance-4 obligation it carries its own rendered-line row
(`verify_template_warns_and_completes_on_a_refused_shape`). P3 step 2 still owns
the CARD-side warnings on `decode`/`inspect`/`bytecode`/`verify`; what it finds
already done is the `--template` half.

This is **not** a scope change and needs no ruling: it is the minimum required
to leave the tree consistent with the spec at the end of P2. Flagged here so
P3's implementer is not surprised by it.

---

## 4. R-N1-hardening reachability — DETERMINED: **REACHABLE**, row lands

The plan required a probe past "the single-site hardened-wildcard refusal".
Measured against the baseline binary:

```
$ md descriptor --template "wpkh(@0/<0;1>/*')" --key @0=<KEY 1> --path 48'/0'/0'/2'
wpkh(xpub661My…/<0;1>/*h)#hs2ar46p                                     exit 0
$ md encode    "wpkh(@0/<0;1>/*')" --key @0=<KEY 1> --path 48'/0'/0'/2' --group-size 0
                                                                        exit 0
$ md encode "wsh(multi(2,@0/<0;1>/*,@0/<0;1>/*'))" --key @0=<KEY 1> --path 48'/0'/0'/2'
md: template parse error: @0 appears with inconsistent path/multipath/hardening   exit 1
```

**There is no single-site hardened-wildcard refusal to get past** — the
single-site form composes and mints. The two-occurrence differing-hardening
form lexes cleanly and reaches `resolve_placeholders`, i.e. reaches exactly
where the classifier now sits. The case is therefore reachable and its row
lands, as `r_n1_hardening_refuses_naming_the_hardening_axis_and_cites_no_bip`.

The probe itself is kept as a row
(`r_n1_hardening_reachability_probe_single_site_still_composes`) so the
determination cannot rot silently: if the single-site form ever starts
refusing, that row goes red and says which ground moved.

---

## 5. Blast-radius dispositions (plan step 6) — every site, what was done, test state

Measured by running the full suite after the classifier landed: **10 failures,
9 of them the plan's enumeration, 1 outside it.**

| site | plan's disposition | what was done | state |
| --- | --- | --- | --- |
| `test_vectors.rs::keyed_tr_sortedmulti_a` | REPLACE — internal key gets a placeholder appearing nowhere in the leaf | `tr(@0/48'/0'/0'/2'/<0;1>/*,sortedmulti_a(2,@1/48'/0'/1'/2'/<0;1>/*,@2/48'/0'/2'/2'/<0;1>/*))`, 3 keys/fingerprints | green |
| `test_vectors.rs::keyed_tr_multi_a` | REPLACE, same, order-sensitivity role preserved | `tr(@0/…0'…,multi_a(2,@1/…1'…,@2/…2'…))`. The leaf still holds **two distinct keys in written order**, which is all a reversal test needs; the comment was updated to say "two keys IN THE LEAF". | green |
| `test_vectors.rs::keyed_wsh_timelock_hashlock` | REPLACE — fresh placeholders in the recovery clause | recovery becomes `multi(1,@3/48'/0'/3'/2'/<0;1>/*,@4/48'/0'/0'/2'/<0;1>/*)`, 5 keys. `@4` is under the **second** master (`b8688df1`) because `73c5da0a`'s records stop at `48'/0'/3'/2'`. | green |
| `vector_corpus.rs` (`diff -r` vs committed corpus) | regenerate the committed corpus | `md vectors --out crates/md-codec/tests/vectors` → **exactly 15 files** changed ({3 vectors} × {template, bytes.hex, phrase.txt, descriptor.json, conformance.json}); directory still holds **131** files, no additions, no orphans | green |
| `conformance_vectors_roundtrip.rs` | same | — | green |
| `corpus_origin_consistency.rs` (never red, but the binding constraint) | never bind one `[fingerprint/path]` to two xpubs | the timelock descriptor's five origins are `[73c5da0a/48'/0'/0'/2']`, `[…/1'/2']`, `[…/2'/2']`, `[…/3'/2']`, `[b8688df1/48'/0'/0'/2']` — all distinct, each bound to its real xpub | green |
| `sortedmulti_a_taproot_leaf.rs:77` (order invariance) | rewrite to distinct placeholders | internal key `@2`; `address_of` gains a third binding | green |
| `sortedmulti_a_taproot_leaf.rs:139` (encode/derive agree) | rewrite to distinct placeholders | internal key `@2` | green |
| `sortedmulti_a_taproot_leaf.rs:106` (nested is refused) | message assertion updates to whichever refusal now fires first, verified by running it | **both**: the row keeps the positional rule on a distinct-placeholder template, and a NEW sibling row pins that the repeated-placeholder spelling refuses at **N1 first**. See the note below. | green |
| `cli_unhardened_origin_note.rs:134` | rewrite to `@0`/`@1`, preserving the note | tier 1 joins its slots, so `count == 1` and the test's name still hold; comment records that the per-occurrence spelling is now unconstructible | green |
| `cli_keyed_excess_origin_note.rs:169` | rewrite to `@0`/`@1`, preserving the note | tier 2 emits **one line per firing slot** (its own emitter says so), so with both slots firing the count is 2. Renamed `note_is_said_once_per_firing_slot`; the per-declaration half is what remains observable and is what it now measures. | green |
| `duplicate_key_slots.rs::one_key_at_two_different_use_sites_is_not_a_duplicate` (step 3) | flip to a refusal row | → `..._refuses_as_the_r_n1d_delta`. The single-slot address contrast is KEPT — it is what makes the refusal's own claim ("the wallet is a legal descriptor") true rather than asserted. The two-slot `addr()` comparison was deleted: both spellings now refuse, so an `assert_ne!` over them would have compared two refusals. | green |
| `duplicate_key_slots.rs::t_row_one_key_at_two_disjoint_use_sites_still_composes` (step 3) | flip to a refusal row | → `..._refuses_as_the_r_n1d_delta`. Its ground (descriptor must not refuse what encode mints) is intact — N1 moved both verbs at once. | green |

**Not silently absorbing the positional rule.** The plan's letter for `:106`
(update the message assertion to whatever fires) would have left "sortedmulti_a
is valid only as a taproot leaf" asserted by nothing, because R-N1a now refuses
that template before the tree is walked. Both facts are kept instead, in two
rows. This is a coverage-preserving addition, not a departure from the ruling.

### The site OUTSIDE the plan's enumeration

`crates/md-cli/tests/cmd_encode.rs::experimental_still_enforces_the_other_sanity_rules`.
Its "repeated keys" case used `tr(NUMS,{pk(@0/<0;1>/*),and_v(v:pk(@1/<0;1>/*),pk(@1/<0;1>/*))})`
and asserted rust-miniscript's `repeated pubkeys` refusal. R-N1a now fires
first.

This is a permanent, correct consequence, not an accident:
`substitute_synthetic` gives every `@i` a DISTINCT synthetic key, so the only
way a repeated pubkey can reach rust-miniscript through md's template surface
is a repeated PLACEHOLDER — which N1 refuses upstream, with or without
`--experimental`. Miniscript's repeated-keys rule is therefore no longer
reachable from that surface.

Disposition: the case is not deleted. The table keeps its timelock-mixing case
(the "other rules still apply" property), and the repeated-keys case moves to
its own row, `experimental_does_not_admit_a_repeated_placeholder`, which pins
**which layer answers**. A cheaper gate absorbing an older one is exactly the
shape that passes green while proving less, so the move is recorded in a row
rather than inferred.

### Rust-primary lockstep — FLAGGED, follows, never leads

The 15 regenerated files under `crates/md-codec/tests/vectors/` are the
cross-language artifact vendored byte-for-byte into the Go port
(`crates/md-cli/tests/corpus_origin_consistency.rs` reads them here). The change
lands in Rust **with this phase**, with vectors. **The Go vendor sync is
outstanding and follows.** Three vector templates and all five per-vector files
changed for each; the Go port will re-vendor and re-run its conformance pass.

---

## 6. The generator (plan step 7)

`crates/md-cli/tests/fixtures/seating/generate.sh`'s V-R5M1 block would, after
P2, fail at `md encode` — and under `set -e` it would fail *after* `header`'s
`> "$f"` had already truncated `v-r5m1.txt`, leaving one fixture emptied and
the 17 written after it unregenerated.

**Measured first, before touching the block:** a full run of the generator
against the BASELINE binary reproduced **every** fixture byte-identically
(`git status` over the fixture directory: empty). So the committed
`v-r5m1.txt` already *is* this block's final output, and "regenerate one final
time" is a measured no-op rather than an unrun step.

The block is now an EXISTENCE ASSERT that requires the fixture to still carry
both halves it is used for (`^md1` and `^mk1` lines) — deliberately stronger
than `-e`, because an empty or half-written file is precisely the failure it
replaces. It writes nothing.

**Proven non-vacuous:** with `v-r5m1.txt` truncated to its header, the
generator exits **1** after writing only the 4 fixtures that precede the block.
(Exit code read directly from the process, not through a pipe.)

**Re-run end to end with the POST-P2 binary:** exit 0, 21 fixtures written,
`kept (frozen, not regenerable): …/v-r5m1.txt`, and `git status` over the
fixture directory **empty** — byte-identical over all 17 files written after
the block, which is r2 M-b's check.

---

## 7. `md compile` (plan step 8) — pinned

`md_compile_refuses_a_duplicate_key_policy` pins four `(expr, --context)` pairs
— `thresh(2,pk(@0),pk(@0),pk(@1))` at `segwitv0` and `tap`, `and(pk(@0),pk(@0))`,
`or(pk(@0),pk(@0))` — each at exit 1 with the exact rendered line

```
md: compile error: compile: Policy contains duplicate keys
```

with a distinct-key control (`md_compile_still_accepts_a_distinct_key_policy`)
so the rows cannot pass on a broken verb. The refusal is rust-miniscript's and
was pinned locally by nothing; an upstream bump can no longer silently open a
mint path for a refused shape. Both rows are `#[cfg(feature = "cli-compiler")]`
and run under the gate's `--all-features`.

---

## 8. Phase gate

```
$ ./scripts/phase-gate.sh          # exit 0

=== cargo nextest run --locked --all-features ===
     Summary [0.845s] 1139 tests run: 1139 passed, 2 skipped

=== cargo test --workspace --doc ===
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

=== cargo clippy --locked --all-targets --all-features -- -D warnings ===
    Finished `dev` profile

=== cargo fmt --check ===
    (clean)

=== cargo doc --workspace --no-deps --document-private-items --all-features ===
    Finished `dev` profile

=== design/display-grouping-vectors.tsv.sha256 ===
display-grouping-vectors.tsv: OK

phase-gate: all six steps passed
```

Baseline for comparison (plan "## The gate", machine-verified at P1): 1106 run
/ 1105 passed / 1 failed / 2 skipped, the failure being the tripwire P1 deleted.
P2 adds 33 tests (1106 → 1139) and holds the suite fully green.

Blind spot the script states itself and this run does not cover: the freebsd
and musl jobs, and the windows/macos legs of the CI test matrix. The
push-ritual staging run covers them before anything reaches `main`.

---

## 9. Acceptance obligations discharged in this phase

- **Acceptance 1** — every P2 vector row exists as an executable test in the
  same commit as its implementation. R-N1-hardening's row lands because the
  reachability condition was met (§4).
- **Acceptance 2** — green under the full gate including `--all-features`.
- **Acceptance 4** — every diagnostic P2 introduces has a RENDERED-line row.
  Six refusals + one warning. `rendered_line()` additionally asserts there is
  **exactly one** `md:` line, so no row can pass while a second diagnostic sits
  beside it. No diagnostic contains "invalid", asserted per row and again over
  every `Finding` in a unit row.
- **Acceptance 5** — not yet discharged; it is P3's, and the two fixture cards
  it needs exist and are pinned. The `verify --template` half is done (§3).

## 10. Commits

| SHA | what |
| --- | --- |
| `f996bc8c` | P2.1 — the two fixture cards, minted from the baseline binary |
| `3f8dd4c5` | P2.2–4 — the taxonomy rows, RED (20 run, 7 passed, 13 failed) |
| `c0fa1ab3` | P2 — the classifier (suite 1136/1126/10 at this commit) |
| `8a71594a` | P2.5–6 — blast radius, the two flips, the stale comments, MANIFEST + corpus |
| `fa0384f4` | P2.7 — freeze generate.sh's V-R5M1 block; `md vectors` fail-closed row |
| `156240bf` | P2 — rustfmt |

## 11. Deviations

**None.** Two decisions are recorded above because the plan left them to the
implementer and a reviewer should see them named rather than discover them:

1. `md verify --template` gets the WARN disposition **in P2** (§3), forced by
   the single-source placement and required by Acceptance 5. Its rendered-line
   row lands with it.
2. `sortedmulti_a_taproot_leaf.rs:106` keeps the positional rule AND gains a
   row for the N1-first refusal, rather than replacing the former with the
   latter (§5).

One site outside the plan's enumeration was found by running the suite
(`cmd_encode.rs`, §5) and dispositioned in the same spirit as the enumerated
ones.

## 12. For P3

- Both fixture cards are in `crates/md-cli/tests/fixtures/n1/` and both compose
  at exit 0 through the card branch today — r3 I-1's premise, re-measured here.
- The classifier is `crates/md-cli/src/parse/reuse.rs`. The card path needs the
  same two inputs reconstructed from a decoded `Descriptor`: the occurrence
  triples (from `use_site_path` + `use_site_path_overrides` + `path_decl`) and
  the per-`@i` key bindings (from `tlv.pubkeys`).
- P3.3b's door-check unification (`check_no_repeated_placeholder` at
  `seat/satisfy.rs:188`) is **untouched** by P2, as ruled. Its two pinning sites
  (`seating_vectors.rs:679-687`, `satisfy.rs:530-548`) are likewise untouched
  and green.
- `CliError::Unsupported` renders `md: unsupported: …` at exit 1; the WARN
  disposition renders `md: warning: <the same body>`.
