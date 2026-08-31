# IMPLEMENTATION PLAN — post-converter md-cli mini-cycle

Status: DRAFT p0 — plan R0 not yet run. Spec:
`SPEC_mdcli_mini.md`, **GREEN at `b8a64938`** (R0 loop closed
2026-08-31 at 0C/0I over four rounds). Baseline revision for
staleness re-validation: **`b8a64938`** — every citation in this plan
was verified against that tree, and every measurement in it was run
this session. The plan contains NO speculative code blocks, so there
is nothing for a build-gate script to extract; its machine-checkable
surface is commands and citations, all already run.

## The gate (every phase; closes `phase-gate-omits-cargo-doc`)

From P1 onward (P1 itself widens CI to match):

```
cargo nextest run --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features
```

Until P1 lands, the same four lines without `--all-features`.
Machine-verified at baseline: clippy and doc are exit 0 WITH
`--all-features`; nextest `--all-features --no-fail-fast` fails only
the fired tripwire P1 deletes (1106 run / 1105 passed / 1 failed / 2
skipped). Every phase's implementer runs the gate before its final
commit and pastes the summary lines into the commit message.

**A plan's GREEN expires.** Before dispatching each phase's
implementer, the controller re-validates this plan against the tree,
scoped to *"what did the last phase falsify here?"* — not a fresh
audit.

## Phase order and rationale

P1 first so every later phase runs the widened gate. P2/P3 are the
normative core (risk-set; spec N1). P4–P6 are surface work on ground
P2/P3 stabilise. P7 closes out. One implementer per phase; the
controller folds small post-review fixes inline rather than spawning
more agents.

### P1 — R5: the all-features closure

1. Delete `render_tr_template` and the fired tripwire
   (`crates/md-cli/src/compile.rs:338`,
   `upstream_display_is_still_broken_delete_local_renderer_when_this_fails`);
   render via upstream `Display`. The tripwire IS the failing test —
   after the deletion the all-features suite must be fully green.
2. Widen `.github/workflows/ci.yml`: `--all-features` on the test
   job (`cargo test --workspace --all-targets`), the clippy job
   (line 65) and the doc job (line 93), in the same commit that
   widens the gate above. Sequencing (1) before (2) keeps CI green
   throughout (never-skip-jobs).
3. Sweep `compile.rs` for imports/helpers orphaned by the deletion
   (clippy `--all-features` now sees them).

Closes FOLLOWUPS `all-features-suite-is-red-and-ungated-by-ci`.

### P2 — N1 mint/compose refusals (template path)

The classifier, per the spec's placement constraints: input is
(i) the per-placeholder occurrence list and (ii) the resolved
per-`@i` key bindings; each predicate has ONE implementation;
per-verb disposition (refuse/warn) is a parameter; NOTHING new inside
`encode_payload`'s validator set; the codec floor
(`validate_no_duplicate_key_slots` at `encode.rs:120`) and the S-row
`check_no_repeated_xpub` stay as shipped.

TDD order — rows first, red before green:

1. **Fixture mint FIRST, at baseline:** mint the R-N1a card
   (`wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))`, chunk-set-id
   `0xed813` measured) and the R-N1d delta card
   (`wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))` with one key, chunk-set-id
   `0x00ee4` measured, 4 chunks) from the BASELINE binary, commit as
   fixtures with provenance headers in the V-BOUND-REF pattern
   (`tests/fixtures/seating/generate.sh` documents it). After this
   phase no shipped binary can mint them — the fixtures are the
   read-side and card-input rows' input forever.
2. Family-1 rows: R-N1a refusal at `encode`, `descriptor
   --template`, `address --template`; R-N1b; R-N1c full rendered
   line (new error variant — NOT `CliError::TemplateParse` — body
   names BIP-legality, one-path-per-key/F-417, and the runnable
   escape `me sysw pack --as descriptor --in <your export file>`);
   R-N1-origin (names the origin axis; cites no BIP rule);
   R-N1-hardening reachability probe — pin the row if reachable past
   the single-site hardened-wildcard refusal, otherwise record
   unreachability in the phase report.
3. R-N1d delta rows: template-path refusal with full rendered line
   (attributes the pairwise-distinct violation to the SPELLING's key
   vector; wallet-expressible-as-descriptor; same escape; never the
   shipped same-use-site wording). Flip
   `duplicate_key_slots.rs::one_key_at_two_different_use_sites_is_not_a_duplicate`
   and `::t_row_one_key_at_two_disjoint_use_sites_still_composes`
   to refusal rows.
4. Must-COMPOSE control: same fingerprint, different accounts,
   different xpubs — composes (the operator-probe row; catches a
   fingerprint-keyed misimplementation).
5. Correct the stale comments: `cmd/build.rs:280-283` ("BIP 388
   permits it") and `md-codec/src/validate.rs:353-355`.

### P3 — N1 card paths + read side

1. Card-input composing refusals: `descriptor` and `address` refuse
   the P2 fixture cards (both the R-N1a card and the R-N1d delta
   card) — the r3 I-1 rows; measured today both read at exit 0
   through the card branch (`build.rs:69-77`, no reuse check).
2. Read-side rows on both fixture cards: `decode`, `inspect`,
   `bytecode`, `verify` complete at exit 0 WITH a warning naming the
   BIP-388 violation (Acceptance 5's newly-refuses scope). `verify`'s
   `--template` argument warns rather than refuses.
3. Seating: the V-BOUND-REF different-paths sibling row (same xpub
   declared at two paths refuses at `check_no_repeated_xpub` —
   measured 2026-08-31, previously unpinned).
4. T-row/S-row parity is now row-pinned from both sides (spec r3
   M-1): the delta wallet refuses at T (P2) and S (shipped);
   the legitimate same-seed wallet composes at T (P2 control) and
   seats at S (shipped control).

Closes FOLLOWUPS `md-repeated-placeholder-inverts-bip388`.

### P4 — N3 bracket-as-source + R9 arity (the descriptor/address input surface)

1. N3: the last-resort arm in
   `resolve_keys_fingerprints_and_precedence` plus the per-slot
   override via `apply_path_override_per_slot`. Rows: the
   different-accounts wallet composes AND equals the
   inline-origins composition byte-for-byte; a disagreeing bracket
   with `--path` present still refuses; a slot with no path from any
   source still refuses.
2. R9: `num_args = 1..` on `--from-mk1`; BOTH guards (mk1-prefix in
   the positional → names `--from-mk1`; md1-prefix among
   `--from-mk1` values → names the positional); the flag-first
   ordering must produce the symmetric guard's diagnostic, not
   clap's missing-required error — mechanics are this phase's to
   choose under that outcome. Four rows: positional-first composes;
   flag-first trailing-md1 → named refusal; mk1-in-positional →
   named refusal; `--from-mk1` with no policy card anywhere → named
   refusal (nothing falls through to seating with an empty policy).

Closes FOLLOWUPS `descriptor-key-bracket-path-as-a-last-resort-source`
and `from-mk1-arity-spills-card-strings-into-the-md1-positional`.

### P5 — N2: `--emit md1`

1. Emission from the seating result on `md descriptor --from-mk1`;
   carries the origin metadata learned from seating; depth rule on
   `md encode --key` untouched.
2. Input-mode refusals: `--emit md1` with `--template` refuses
   naming `md encode`; on a keyed-card positional refuses as a
   re-emit, by name.
3. Oracle rows: PRIMARY — byte-identity against `md encode` invoked
   with inline per-slot origins + per-slot `--fingerprint @i=HEX`
   matching the policy declarations (measured executable at
   baseline); SECONDARY — `spend_equal` and address-0 equality
   against the keyed fixture; plus one row pinning a seating refusal
   surviving under `--emit md1`.
4. Matrix: flip the S→K cell in all 4 homes in the same commit;
   `scripts/matrix-identity-check.sh` green in the gate run.

Closes FOLLOWUPS `md-cannot-mint-a-keyed-card-from-a-split-set`.

### P6 — R3 + R6 + R7 (small riders)

1. R3 `--verify-against <md1|FILE>` on `md descriptor`: wires
   `spend_equal` (delete its `#[allow(dead_code)]` and the
   now-false comment); output per spec; **exit 0 equal / 5 not / 1-2
   errors**. Four rows: equal cross-form pair (0); one-xpub-off (5,
   names the values half); origins-differ (0, EQUAL); garbage
   argument (1, decode error, no verdict).
2. R6: one desugar core serving both spellings (generalise off the
   `@`-anchor or two thin front-ends over one component — no
   keep-in-sync regex pair); `--help` names `/**`; row: `/**`
   decomposes identically to its `/<0;1>/*` rewrite.
3. R7: `-` on decompose's positional (≡ `--in /dev/stdin`); the
   not-a-descriptor refusal names `--in` and `-`.

Closes FOLLOWUPS `md-verify-against-flag-for-cross-form-comparison`,
`md-decompose-rejects-double-wildcard-input`,
`md-decompose-does-not-read-stdin`.

### P7 — close-out

1. R8: append the parked trigger to FOLLOWUPS
   `md-decompose-has-no-json-output` ("the first front-end consumer
   doing the listdescriptors-extraction job designs the envelope").
2. Cross-repo docs pass in `bg002h/mnemonic-toolkit`:
   `docs/manual/src/40-cli-reference/42-md.md` catches up on the
   converter cycle's surface plus this cycle's
   (`--verify-against`, `--emit md1`, bracket-source, `-` on
   decompose, `/**`, R9 arity); `tests/lint.sh flag-coverage` green
   there. Also closes FOLLOWUPS
   `sibling-toolkit-md-manual-lockstep-for-the-converter` (ruled
   into this phase by the operator, 2026-08-31).
3. FOLLOWUPS reconciliation sweep: all nine owned entries
   dispositioned (the eight originals + the walk-discovered R9
   entry); each closure cites its phase's commit.
4. **Whole-diff independent adversarial review (mandatory,
   non-deferrable)** over the full cycle diff before merge; report
   persisted; fold loop to 0C/0I.
5. Push via `scripts/push-via-staging.sh`; `main` frozen for the
   window.

## Follow-up burndown (slug → owning phase)

| slug | phase |
| --- | --- |
| `all-features-suite-is-red-and-ungated-by-ci` | P1 |
| `md-repeated-placeholder-inverts-bip388` | P2+P3 |
| `descriptor-key-bracket-path-as-a-last-resort-source` | P4 |
| `from-mk1-arity-spills-card-strings-into-the-md1-positional` | P4 |
| `md-cannot-mint-a-keyed-card-from-a-split-set` | P5 |
| `md-verify-against-flag-for-cross-form-comparison` | P6 |
| `md-decompose-rejects-double-wildcard-input` | P6 |
| `md-decompose-does-not-read-stdin` | P6 |
| `md-decompose-has-no-json-output` | P7 (parked, trigger recorded) |
| `sibling-toolkit-md-manual-lockstep-for-the-converter` | P7 |
| `phase-gate-omits-cargo-doc` | closed by this plan's gate |

## Review protocol

Plan R0 to 0C/0I before P1's implementer dispatches. Rust-primary:
every admission change lands here with vectors; no Go port leads.
Reviewer tiers sonnet/opus; agents persist their own reports to
`design/agent-reports/`; persist-before-fold, two commits, gate
output in the fold commit's message.
