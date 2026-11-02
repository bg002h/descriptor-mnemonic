**⚠** r1: 0C/2I/0M/3N

# FIX review r1: `design/FIX_bughunt_md_b.md` (md bug hunt FIX-B, md-codec)

Reviewer: independent adversarial review, Opus, effort medium. Head `347160b5`, base `b2bfdd63`.
Scratch worktree `dm-worktrees/review-md-b` (removed after this report). It was the first round, so 0 findings come from a previous fix commit.

## What was executed

§2 was applied literally to a scratch tree: R-2, R-4/2.5, R-5, R-8, R-16, 2.1 BIP row, 2.2 doc paragraph and 2.3 variant. R-7 was written from its prose, because it is the only row with no exact code. Logs were kept in the scratch `rv/`.

| step | command (as printed in §5) | result |
|---|---|---|
| 1 | `cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast` | 1600 run, 1591 pass, 9 fail, 4 skipped. The 9 failures are exactly the md-cli rows 01, 03, 04, 10, 11, 12, 14, 06 and 13 that the doc lists. Every md-codec test passes. |
| 2 | `cargo test --locked --workspace --doc --all-features` | exit 0 |
| 3 | `cargo fmt --all --check` | **exit 1** on the literal §2 text (see I-2) |
| 4 | `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| 5 | the 14-row filter | "14 tests run: 14 passed", exit 0 |

- **RED on base:** the controller log `/scratch/cache/bughunt/md/fixb-red2.log` shows 14 matched, 13 failing and #9 passing. I accept this as settled.
- **R-16 through the CLI:**
  - The scratch build of `md decode 'md1Źq9dsssqrqq20vuhf9agdphs'` prints `md: codec error: codex32 decode error: character 'Ź' not in codex32 alphabet` and exits 1.
  - The installed md 0.20.3 (base behaviour) prints a `wpkh(@0/<0;1>/*)` card with a wrong origin `m/48'/0'` and exits 0. This matches doc lines 174-178.
- **Q1 text through the CLI:** the scratch `md encode 'wpkh(@0/<0;536870912>/*)'` prints `md: codec error: varint value 536870912 exceeds single-extension range (max 2^29 - 1)` and exits 1. This matches Q1.

## Case tables

**R-2** (doc §2.4 R-2: "`older(n)` under `thresh` (`Body::Variable`) is truncation-checked")

| input | output |
|---|---|
| `thresh(..,older(n))` where n truncates | `RelativeTimelockTruncated` (row 02 passes) |
| non-truncating n under thresh | unchanged Ok (suite green) |

**R-4** (doc: "any origin component, or any use-site multipath alternative, whose value is > 2^29-1 is refused with ... `VarintOverflow { value }` by every `to_miniscript` entry")

| input | entries | output |
|---|---|---|
| alt 2^31, selected (04b) | `to_miniscript_descriptor`, `_multipath`, `derive_address` | `VarintOverflow{2147483648}` |
| origin comp 2^31 (04c) | all three | `VarintOverflow{2147483648}` |
| alt 2^29, unselected (04f) | all three | `VarintOverflow{536870912}` |
| alt 2^29-1, control (04f) | all three | Ok |

- **Guard placement:** the guard is in `assemble_origin_and_xkey`, which both the single-path and the multipath builders call. I found no other `ChildNumber` construction from a wire value that bypasses it.
- **Removed fallbacks:** the three fallbacks become direct constructors, and these are safe once the guard has run.
- **Error ordering:** `NetworkRequiredForUnspendable`, `HardenedPublicDerivation`, `ChainIndexOutOfRange` and `MissingPubkey` are checked earlier and can win. All of them fail closed.

**R-5** (doc: "`collect` returns `false` on `AndOr`, `OrB`, `OrC`, `OrD`, `OrI` inside a branch")

| input | output |
|---|---|
| `wsh(and_v(v:pk,or_d(pk,older)))` (row 05, policy_shape row) | `complete == false`, `branches` empty |
| a top-level or | split as before (suite green) |

**R-7** (doc: "every TLV entry idx ... must be `< d.n`; the first that is not returns ... `PlaceholderIndexOutOfRange { idx, n: d.n }`")

| input | output |
|---|---|
| n=2, fingerprints idx 5 (row 07) | `PlaceholderIndexOutOfRange{5,2}`, no panic |
| in-range idx | unchanged (suite green) |

The check sits before the identity fast path. An identity-permutation descriptor with a bad TLV idx is therefore now refused too. That is correct, and the suite is green.

**R-8** (doc: "a tag outside `0x04..=0x1f` returns `TlvUnknownTagOutOfRange { tag }`; a tag equal to an earlier unknown entry's tag returns ... `TlvOrderingViolation`... Both before anything is written")

| input | output | bits written |
|---|---|---|
| unknown tag 0x21 (08) | "unknown TLV tag 0x21 is outside the reserved range 0x04..=0x1f" | 0 |
| unknown tag 0x01 (08b) | same text with 0x01 | 0 |
| tag 5 twice (08c) | `TlvOrderingViolation{5,5}` | 0 |

**R-9:** no change. Row 09 pins the lenient decoder, as BIP :752 requires. It passes on both base and fix.

**R-16** (doc: "a non-ASCII char never aliases a codex32 symbol ... same text as an ASCII non-alphabet char")

| input | path | output |
|---|---|---|
| U+01xx alias, single string (16) | `char_to_symbol` | `codex32 decode error: character 'Ź' not in codex32 alphabet`; the `'b'` control gives the same wording |
| alias in chunk 0 (16b) | `reassemble` (string decoder) | same text, no chunk prefix |
| alias in chunk 0 (16c) | `correct_chunks` / `parse_chunk_symbols` | `codex32 decode error: chunk 0: character .. not in codex32 alphabet` |

- `chunk.rs:528` and `codex32.rs:73` are the only alias sites in md-codec src.
- md-cli has no own char→u8 truncation (grep).

## Mandatory checks 3-5

- **3. Create-input table.** §3/§1 add no new printed holder command or flag. The only CLI forms cited are `md decode <string>` and `md encode <template>`: positional arguments only, and both were run above with the expected output. No row fails.
- **4. Deferred-minors.** md `design/FOLLOWUPS.md` has one open `deferred-minor`, `fuzz-oracle-unadmitted-nits`. This doc does not touch its text, so there is nothing to re-grade.
- **5. BIP facts.** These were checked against `bip/bip-mnemonic-descriptor.mediawiki`:
  - :459 says values up to 2^29-1 are representable and larger ones are rejected with VarintOverflow.
  - :639 says duplicates are rejected.
  - :654 gives the reserved range 0x04-0x1F.
  - :752 covers canonicality and leniency.
  - The 2.1 row inserts after :957 and renders correctly as a table row.

## Findings

### I-1 (Important): FIX-B cannot merge first. Its own branch's workspace CI is red by design, and the doc does not decide how that is handled

**Location.**
- `design/FIX_bughunt_md_b.md` §1 line 20, "FIX-B builds and merges first".
- §5, the paragraph after the filter (lines 227-232), "Still failing, and owed elsewhere (not a FIX-B failure): the md-cli bughunt rows ...".

**Failure scenario.**
1. FIX-B's base `b2bfdd63` is on `bughunt/triage`, not in master. That base already carries the nine md-cli RED rows:
   - `crates/md-cli/tests/bughunt.rs` 01, 03, 04, 10, 11, 12, 14
   - `src/seat/compose_bughunt.rs` 06
   - `src/seat/input_bughunt.rs` 13
2. CI's required check `cargo test (ubuntu-latest)` runs `cargo nextest run --workspace --all-targets --all-features --no-fail-fast` (`.github/workflows/ci.yml:83`, no `--run-ignored`). On FIX-B's head it therefore fails on those nine. The md-cli musl jobs (`ci.yml:314`, `:323`) fail too.
3. The merge rule requires CI green on the final head, so FIX-B cannot be merged. FIX-A is the doc that touches those rows, and it bases on "FIX-B's merged head" (§1 line 21). The order deadlocks.
4. A builder or merger has to choose: merge red, ignore the rows, or re-order. That is an undecided point.

**Fix text** (anchor: replace the §5 paragraph that begins "Expected workspace result for step 1:" through "...FIX-A `#[ignore]`s 06 and 13)."):

> Expected workspace result for step 1: every test passes. FIX-B adds `#[ignore = "owed to FIX-A (md bug hunt row NN)"]` (NN = the row number), on the line after `#[test]`, to the nine md-cli rows its base already carries and that FIX-A owns: `crates/md-cli/tests/bughunt.rs` rows 01, 03, 04, 10, 11, 12, 14, `crates/md-cli/src/seat/compose_bughunt.rs` row 06 and `crates/md-cli/src/seat/input_bughunt.rs` row 13. Step 1 then reports 1591 passed, 13 skipped. FIX-A removes the ignore from rows 01, 03, 04, 10, 11, 12 and 14 as its RED step and keeps 06 and 13 ignored. Any failure is a FIX-B FAIL.

Also add to §4 ("Tests that change"): `crates/md-cli/tests/bughunt.rs`, `compose_bughunt.rs`, `input_bughunt.rs`: the ignore attribute above. FIX-A's doc needs the matching line "remove the FIX-B ignore on rows 01, 03, 04, 10, 11, 12, 14".

**Run.** I applied it on the scratch tree with the §2 fix. The step 1 command as printed gave `1591 tests run: 1591 passed, 13 skipped`, exit 0.

### I-2 (Important): §2's exact Rust texts fail the doc's own gate step 3 (`cargo fmt --all --check`) when pasted literally

**Location.** Two §2 texts trigger it:
- §2.4 "R-8 loop, exact", doc line 97: `return Err(Error::TlvOrderingViolation { prev: *tag, current: *tag });`. This is 102 columns at its indentation.
- The §2.4 R-16 row, `chunk.rs` text: `if !c.is_ascii() { return Err(Error::Codex32DecodeError(format!("chunk {chunk_index}: character {c:?} not in codex32 alphabet"))); }`, given on one line.

**Failure scenario.**
1. A builder pastes the exact texts and runs §5 step 3 as printed.
2. rustfmt reports `Diff in .../tlv.rs:99` and `Diff in .../chunk.rs:525`, and the check exits 1.
3. The build stops, or the builder has to decide on reformatting. The doc gives no instruction for this.
4. The tool fails closed and no wrong result follows. Even so, it is a printed gate command that fails as printed when the doc is followed literally, so under the likelihood test it stays Important.

**Fix text** (anchor: §5, insert before "1. `cargo nextest run ...`"):

> 0. `cargo fmt --all` (§2's inline Rust texts are not rustfmt-formatted; this reflows the R-8 `return Err(...)` and the R-16 `chunk.rs` statement without changing them).

**Run.** I applied both texts literally:
1. `cargo fmt --all --check` showed diffs in tlv.rs:99 and chunk.rs:525, exit 1.
2. I ran `cargo fmt --all`.
3. `cargo fmt --all --check` then exited 0.

### Minors

None.

### Nits

- **N-1:** §2.4 R-4 / §2.5 says "`assemble_origin_and_xkey`, line 172". Line 172 is mid-comment, and `let path = origin_path_to_derivation` is at line 175. The doc's own "(line 175)" resolves it.
- **N-2:** §2.5 gives `/// Translate` at "line 349". It is at line 348 on `b2bfdd63`.
- **N-3:** §2.5 places the multipath non-hardened alternative at "line 258". It is at lines 261-262, and it is already `ChildNumber::Normal { index: a.value }`, so the edit there is a no-op.

## Design concerns (ungraded)

- R-5 refuses an `or_*` inside a branch, but `thresh(k<n)` inside a branch still reports `complete == true` with a single branch. Equivalent policies can therefore get different completeness flags. The toolkit and engraver consume `policy_shape` through pinned revs and pick this up only on a pin bump.

C / I / M / N: 0 / 2 / 0 / 3
