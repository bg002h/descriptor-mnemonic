# IMPL — mdcli-mini P3 (N1 card paths + read side)

Implementer report. Worktree `/scratch/code/shibboleth/descriptor-mnemonic-mdcli-mini`,
branch `mdcli-mini`, from `83885768` (P1+P2 landed).

**Outcome: P3 steps 1, 2, 3, 3b and 4 executed in full. Phase gate GREEN — all
six steps passed, 1154 tests run / 1154 passed / 2 skipped. No deviation from
the plan.** One design decision the plan left implicit is recorded in §2
(`md verify` gets no second check, and why that is a decision rather than an
omission).

---

## 1. What changed, file by file

Whole-phase diff, `git diff --stat 83885768..HEAD`: **10 files, +628 / −103**.

### New

| file | what |
| --- | --- |
| `crates/md-cli/tests/fixtures/seating/v-bound-ref-paths.txt` | V-BOUND-REF's sibling: the same xpub offered as two cards declaring DIFFERENT origin paths (step 3). |

### Changed — source

| file | what |
| --- | --- |
| `crates/md-cli/src/parse/reuse.rs` | `+162`. `check_descriptor(&Descriptor, Disposition)` — the CARD entrance to the shipped classifier — plus `card_occurrences`, `card_key_bindings`, and `count_occurrences` (moved here from `seat::satisfy`). Module doc gains a "Two entrances, one classifier" section. |
| `crates/md-cli/src/cmd/build.rs` | the phrase (card) branch now decodes into a binding and calls `check_descriptor(.., Refuse)` before returning — `md descriptor` and `md address` on a card (step 1). |
| `crates/md-cli/src/cmd/decode.rs` | `check_descriptor(.., Warn)` after decode, before the `--json` branch so both surfaces emit it (step 2). |
| `crates/md-cli/src/cmd/inspect.rs` | same. |
| `crates/md-cli/src/cmd/bytecode.rs` | same. |
| `crates/md-cli/src/seat/satisfy.rs` | `check_no_repeated_placeholder` becomes a one-line invocation of `parse::reuse::check_descriptor` (step 3b); `count_occurrences` and the `md_codec::tree::{Body, Node}` import leave with it; the V-R5M1 unit row is rewritten; the V-BOUND-REF-PATHS unit row is added. |

### Changed — tests and artifacts

| file | what |
| --- | --- |
| `crates/md-cli/tests/n1_admission_taxonomy.rs` | `+260`. A "P3 — the CARD input and the READ side" section: 13 rows (§3). Module doc rewritten — the file is now both halves of the taxonomy, because they must share the rendered line. |
| `crates/md-cli/tests/seating_vectors.rs` | `v_r5m1_reaches_the_command` rewritten to pin the new RENDERED line as a literal; `v_bound_ref_paths_reaches_the_command` added. |
| `crates/md-cli/tests/fixtures/seating/generate.sh` | a V-BOUND-REF-PATHS block, beside its sibling's. |

---

## 2. Placement, and the one decision the plan left to the implementer

The spec fixes the classifier's INPUT (occurrence list + per-`@i` key
bindings), one implementation per predicate, disposition as a parameter, and
**nothing new inside `encode_payload`'s validator set**. P3's card paths
classify the DECODED template at the CLI layer, which is what keeps that
constraint: `md inspect` and `md verify` re-enter `encode_payload` on a
decoded card, so a check there would have made the two frozen fixture plates
uninspectable and unverifiable — the exact outcome Acceptance 5 forbids.

`check_descriptor` reconstructs the two inputs from the wire:

* **occurrences** — the triple is built **once per slot** (default
  `use_site_path`, overridden by `tlv.use_site_path_overrides`) and then
  repeated for each position the slot occupies in the tree. That is the
  structural form of the ruling: a card records ONE origin, ONE multipath set
  and ONE hardening per key slot (F-417), so two occurrences of `@i` cannot
  differ, and **Family 1 has exactly one reachable outcome on a card,
  R-N1a**. The four axis-divergence rows are template-only by construction,
  not by omission.
* **key bindings** — from `tlv.pubkeys`. Family 2 IS reachable here; the
  delta fixture card is a minted example.

Two choices inside that, both commented at their site:

* `origin_path` is left `None` on every reconstructed occurrence rather than
  decoded out of `path_decl`. Its only role in the classifier is
  `same_triple`, which compares occurrences of the **same** slot — where a
  constant is exactly right — and no message on this path quotes it. Decoding
  it would mean running the strict per-`@N` expansion, which fails closed on a
  partial card; the reading verbs must still read one, so the classifier must
  not be the thing that stops them.
* A slot with ZERO tree positions still contributes ONE occurrence, so Family
  2's per-slot lookup is total. A floor of one can never manufacture a
  Family-1 finding, which needs two.

### `md verify` gets NO second check — a decision, recorded

Plan step 2 names `verify` in the read set, and the P2 report notes that the
`--template` half already Warns. **No check is wired on `verify`'s decoded
side**, and the reason is not that it was overlooked:

`verify` returns `MISMATCH` unless the decoded card and the parsed template
encode to the **identical payload**. Equal payloads mean equal descriptors, so
the only card that can reach exit 0 there is one whose shape the template also
carries — and the template side already warns. A second invocation would print
the identical line twice on every such run, which is noise, and
`rendered_line()`'s "exactly one `md:` line" assertion would then be asserting
a defect.

The two verify rows (§3) pin the CARD's journey rather than the mechanism, so
if that reasoning ever stops holding they go red. This is recorded here
because a reviewer should see it named rather than discover it.

---

## 3. Step-by-step row evidence

### Steps 1 and 2 — RED first

The 13 new rows landed in commit `7a25c941`, before any card path existed.
Measured (`cargo nextest run --locked --all-features --test
n1_admission_taxonomy --no-fail-fast`): **33 tests run, 23 passed, 10 failed.**

| row | RED state at `7a25c941` |
| --- | --- |
| `r_n1a_card_refuses_at_descriptor` | FAIL — printed `wsh(sortedmulti(2,xpub661My…/<0;1>/*,xpub661My…/<0;1>/*))#kuunuuvp` at exit 0 |
| `r_n1a_card_refuses_at_address` | FAIL — printed `bc1ql5j095…` at exit 0 |
| `r_n1d_card_refuses_at_descriptor` | FAIL — printed the delta wallet at exit 0 |
| `r_n1d_card_refuses_at_address` | FAIL — printed `bc1qsa6qqv…` at exit 0 |
| `r_n1a_card_decodes_at_exit_0_with_a_warning` | FAIL — 0 `md:` lines |
| `r_n1a_card_inspects_at_exit_0_with_a_warning` | FAIL — 0 `md:` lines |
| `r_n1a_card_bytecodes_at_exit_0_with_a_warning` | FAIL — 0 `md:` lines |
| `r_n1d_card_decodes_at_exit_0_with_a_warning` | FAIL — 0 `md:` lines |
| `r_n1d_card_inspects_at_exit_0_with_a_warning` | FAIL — 0 `md:` lines |
| `r_n1d_card_bytecodes_at_exit_0_with_a_warning` | FAIL — 0 `md:` lines |
| `r_n1a_card_verifies_at_exit_0_with_a_warning` | **PASS on arrival** — P2's `--template` Warn (see §2) |
| `r_n1d_card_verifies_at_exit_0_with_a_warning` | **PASS on arrival** — same |
| `control_a_clean_card_still_composes_and_reads_without_a_diagnostic` | PASS — the anti-over-refusal control |

GREEN at `a445df55`: all 33 pass, and the **whole suite** is 1152 run / 1152
passed / 2 skipped — P2's 1139 plus this phase's 13, **no blast radius
anywhere else in the tree**.

The refusal and warning rows quote the SAME two message constants as P2's
template rows (`MSG_N1A`, `MSG_N1D`), which is why the P3 section lives in the
P2 file rather than a new one. "Each predicate has ONE implementation; the
disposition is a parameter" is a claim about the code that only a shared
constant can falsify — a card path with its own copy of the predicate would
drift, and these `assert_eq!`s are what would say so.

Measured before the phase (the r3 I-1 premise, re-measured here rather than
inherited): all ten card invocations exited 0 and printed a wallet, a
template, an id block or a hex payload, with no diagnostic of any kind.

### Step 3 — the V-BOUND-REF sibling (`359c2d73`)

A row over **shipped** behaviour, so it is green on arrival; non-vacuity is
established by structure rather than by a red run.

`v_bound_ref_paths_same_xpub_at_different_paths_still_refuses`
(`seat/satisfy.rs`) asserts the fixture's own preconditions (two slots, two
declared paths that DIFFER, two cards whose declared paths DIFFER), then
asserts **each upstream check is `Ok`** —
`check_no_repeated_placeholder`, `check_no_identical_fp_bearing_declarations`,
`check_no_impossible_card_pair` — and only then that
`check_no_repeated_xpub` refuses. So the row states **which check answers**,
and cannot pass on a fixture that was wrong in some other way.
`v_bound_ref_paths_reaches_the_command` (`tests/seating_vectors.rs`) pins the
same refusal end to end at the CLI.

Together with the shipped V-BOUND-REF rows (same xpub, SAME declared path)
this pins that `check_no_repeated_xpub` compares key material and nothing
else.

**Fixture provenance.** Generated by the committed generator, run END TO END
with `MD=/scratch/code/shibboleth/descriptor-mnemonic/target/debug/md` (the
plan's `b8a64938` baseline, read-only) and `MK` = mnemonic-key's `mk`.
`git status` over the fixture directory afterwards showed **only the new
file**: the run reports `wrote:` 22 times (21 pre-existing fixtures plus this
one) and `kept (frozen, not regenerable)` for `v-r5m1.txt`, and all 22 other
files reproduced byte-identically. A further end-to-end re-run after the
commit leaves `git status` over the directory **empty**. Re-run with the
CURRENT worktree binary, `v-bound-ref-paths.txt` is byte-identical — the
fixture is **not** baseline-dependent, because its shape carries no reuse and
stays mintable.

Shipped-behaviour measurement, both binaries, identical output:

```
md: seating refused: cards 06dae (stub 5b48af35) and 69f0e (stub 5b48af35)
carry the SAME extended public key. …  exit 1
```

### Step 3b — the door-check unification (`99405eb6`)

`check_no_repeated_placeholder` is now:

```rust
pub fn check_no_repeated_placeholder(policy: &Descriptor) -> Result<(), CliError> {
    crate::parse::reuse::check_descriptor(policy, crate::parse::reuse::Disposition::Refuse)
}
```

**IN PLACE.** `crates/md-cli/src/seat/mod.rs` is untouched by this phase
(`git diff` over it is empty); the call is still the first door check at
`seat/mod.rs:134`, ahead of A3.

`count_occurrences` moved to `parse::reuse` with the predicate, and
`md_codec::tree::{Body, Node}` left `satisfy.rs` with it — one walker under
one predicate where there were two implementations of the same rule.

#### Before / after, run end to end against `v-r5m1.txt`

**Before** (`CliError::Seat`, exit 1):

```
md: seating refused: this policy uses the same placeholder at more than one position — @1 (2 positions), @2 (2 positions). Seating it would bind ONE key to several positions with the same path expression, which is forbidden by BIP 388 ("the public keys obtained by deserializing elements of the key information vector must be pairwise distinct", and two key expressions on one placeholder must have disjoint multipath sets). That shape is UNSUPPORTED here: the script would be well-formed, the POLICY is one this tool declines to reconstruct. Re-mint the policy with one placeholder per distinct key.
```

**After** (`CliError::Unsupported`, exit 1 — the R-N1a message verbatim):

```
md: unsupported: @1 appears at 2 use sites in this template with the same path expression, so ONE key would fill every one of them. That is forbidden by BIP 388 ("the public keys obtained by deserializing elements of the key information vector must be pairwise distinct"), whose forbidden-example list names sh(multi(1,@0/**,@0/**)) — "Repeated keys with the same path expression". md declines to mint or compose this shape: give each distinct key its own placeholder.
```

The one visible loss is that the classifier reports the FIRST finding in its
deterministic order, so the message names `@1` where the old one also named
`@2`. The plan ruled the wording becomes the taxonomy's R-N1a message; the
domain argument for why nothing else is lost is now IN the function's doc, so
a future reader does not have to re-derive it:

* the policy at this door is **keyless by construction** (`seat::run` refuses
  a wallet-policy card first), so Family 2 has no key material to fire on;
* the wire carries one origin, one multipath set and one hardening per key
  slot, so two occurrences of `@i` cannot differ.

Reachable domain: exactly R-N1a. The unit row now asserts
`!policy.is_wallet_policy()` as the ground for the first bullet.

#### The two pinned wording sites, updated in the same commit

| site | what it pins now |
| --- | --- |
| `seat/satisfy.rs::v_r5m1_repeated_placeholder_refuses_as_bip388_forbidden` | that the door check's message **IS** `Finding::SamePathExpression { i: 1, sites: 2 }.message()` — the same string, not two strings that agree today — plus the keyless precondition and the no-"invalid" rule |
| `tests/seating_vectors.rs::v_r5m1_reaches_the_command` | the whole RENDERED line as a literal, and that there is **exactly one** `md:` line (Acceptance 4: a rewritten diagnostic re-earns its row) |

A tree-wide grep for the old wording (`same placeholder at more than one
position`, `declines to reconstruct`) leaves only the deliberate quotation of
it in the new row's doc comment and the frozen historical agent reports.

### Step 4 — T-row/S-row parity: confirmed, nothing new asserted

The plan's step 4 asserts nothing beyond confirming the row set covers it.
The four cells, **measured end to end against the post-P3 binary** (exit codes
read from the process, not through a pipe):

| cell | measured | rows that pin it |
| --- | --- | --- |
| **T refuses the delta wallet** — `--template "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))"` with one key in both slots | exit 1, `md: unsupported: @0 and @1 were given the SAME extended public key at DIFFERENT use sites …` | P2 `r_n1d_delta_refuses_at_{encode,descriptor_template,address_template}`, `r_n1d_message_meets_its_mandate`; **P3 adds the card side**: `r_n1d_card_refuses_at_{descriptor,address}` |
| **S refuses the same wallet** — keyless policy with two differing use sites + two cards carrying one xpub | exit 1, `md: seating refused: cards … carry the SAME extended public key.` | shipped `check_no_repeated_xpub`, pinned by `v_bound_ref_*` (same declared path) and P3's `v_bound_ref_paths_*` (DIFFERENT declared paths) — the pair is what shows the check is blind to the declared path, which is why the S side refuses the delta |
| **T composes the legitimate wallet** — one master, two accounts, two xpubs | exit 0 | P2 `control_same_fingerprint_different_accounts_still_composes`, `control_distinct_keys_at_disjoint_use_sites_still_compose` |
| **S seats the legitimate wallet** — the same two accounts as cards | exit 0, wallet on stdout | shipped `v_bound_ref_control_different_masters_at_one_path_pass`, `v_r5m1_control_a_reuse_free_policy_passes_the_same_check`, `v_dup_the_full_split_set_supplied_twice_over_still_seats` |

Parity holds and is row-pinned from both sides. P3 adds no parity assertion of
its own, per the plan.

---

## 4. Acceptance obligations discharged in this phase

* **Acceptance 1** — every P3 vector row named in the spec exists as an
  executable test in the same commit as its implementation: the card-input
  composing refusals for `descriptor` and `address` on BOTH cards, the
  read-side rows on `decode`/`inspect`/`bytecode`/`verify` on BOTH cards, and
  the V-BOUND-REF sibling row.
* **Acceptance 4** — the two diagnostics this phase introduces or rewrites
  each have a RENDERED-line row. The card-path refusal and the read-side
  warning are asserted as full `md:` lines (and as byte-equal to the template
  path's, via the shared constants); the rewritten door-check line is
  asserted as a literal in `seating_vectors.rs`. `rendered_line()`'s
  exactly-one-`md:`-line assertion carries over to every P3 row. No
  diagnostic contains "invalid" — asserted per row.
* **Acceptance 5 — DISCHARGED.** `decode`, `inspect`, `bytecode` and `verify`
  each complete at **exit 0** on BOTH already-engraved plates carrying shapes
  this cycle newly refuses, each **with** a warning naming the BIP-388
  violation, and each still producing its output (the rows assert stdout is
  non-empty, so "warned but printed nothing" cannot pass). Nothing was added
  to `encode_payload`'s validator set.

---

## 5. Phase gate

```
$ ./scripts/phase-gate.sh          # exit 0

=== cargo nextest run --locked --all-features ===
     Summary [0.958s] 1154 tests run: 1154 passed, 2 skipped

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

P2 closed at 1139; P3 adds 15 tests (13 in §3 plus the two V-BOUND-REF-PATHS
rows) and rewrites two, holding the suite fully green.

Blind spot the script states itself and this run does not cover: the freebsd
and musl jobs, and the windows/macos legs of the CI test matrix. The
push-ritual staging run covers them before anything reaches `main`.

---

## 6. Rust-primary lockstep

**Nothing to flag.** P3 changes no wire format, no identity or stub algorithm,
and no committed conformance vector — the 15 corpus files P2 regenerated are
untouched here (`git diff --name-only 83885768..HEAD` lists nothing under
`crates/md-codec/tests/vectors/`). What changed is md-cli's ADMISSION on the
card input and its read-side diagnostics, in Rust, with vectors. P2's
outstanding Go vendor sync is unaffected by this phase.

---

## 7. Commits

| SHA | what |
| --- | --- |
| `7a25c941` | P3.1–2 — the card-input and read-side rows, RED (33 run, 23 passed, 10 failed) |
| `a445df55` | P3.1–2 — `check_descriptor` and the four invocation points (suite 1152/1152) |
| `359c2d73` | P3.3 — V-BOUND-REF-PATHS: the same xpub at two DIFFERENT declared paths |
| `99405eb6` | P3.3b — the door check becomes an invocation of the shared classifier; both pinned wording sites updated |

## 8. Deviations

**None.** One decision is recorded above because the plan left it to the
implementer and a reviewer should see it named rather than discover it:
`md verify` gets no second, card-side classifier invocation (§2), because the
only card that can reach exit 0 there is one whose shape the template also
carries — and the template side has warned since P2. Its two card rows pin the
outcome, so the reasoning cannot rot silently.

## 9. For P4

* The card branch of `cmd/build.rs` now ends in
  `reuse::check_descriptor(&descriptor, Refuse)?` before `Ok(descriptor)`.
  P4's N3 work is in `resolve_keys_fingerprints_and_precedence` and
  `apply_path_override_per_slot`, both on the TEMPLATE branch, so they do not
  meet.
* `parse::reuse` now has two public entrances, `check` (template) and
  `check_descriptor` (card). Any new composing surface takes one of them; a
  third implementation of either family is what the spec's single-source rule
  forbids.
* The R9 arity work touches `main.rs:400`/`:560` and the `--from-mk1` groups;
  nothing in P3 moved either.
