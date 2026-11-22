# FIX_bh1119_md_placeholder_order: a non-first-occurrence placeholder order mints, hashes and compares correctly (bh-md-C2 Critical, bh-md-I2, bh-md-I12 Important)

Full track (bughunt-1119 FIX table row #5; class B: it changes md-codec's wire canonicalisation). Ships in
one md-codec release with row #8 (`FIX_bh1119_md_encoder_admission`, T9), then fleet pin bumps. Repo
descriptor-mnemonic, branch `fix/md-bh-criticals` from main `5df4cb923e2124be16575207a0050986f0f0d7bc`.
Crates: `md-codec` (C2, I12) and `md-cli` (I2). No new flag, type, command, error variant or printed text.
Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-22.

**No STOP is open.** No holder-visible text is new: every printed line in the rows below already exists on
the base (seen there for the canonical spelling). Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo
1.85.0; `--locked`; nextest `--build-jobs 6 -j 6`; never `--release`. Signet keys only.

## 1. The defects (RED evidence, run by the author on the base)

Placeholder numbering is a name: BIP-388 and md's own canonical form number slots by first occurrence, and
`encode` canonicalises a clone before writing (`encode.rs:228-229`). A template may spell the slots in another
order (`@1` before `@0`); that is legal input.

- **C2** (`canonicalize.rs:213-261`): the descriptor's use-site baseline (`Descriptor::use_site_path`) is the
  use-site of OLD `@0`; the permutation moves the per-slot override of old `@k` to its new index but never
  recomputes the baseline. When the new `@0` had an override, the card carries an override keyed on `@0`,
  which the decoder refuses (`validate.rs:160-173`, D5(a)).
  `md encode 'wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))'` exits 0 with `md1yppqqxppsg2qk3q2zc2pz7ecekxclml`;
  `md decode` of it exits 1 `md: codec error: use-site override keyed on baseline @0; @0 cannot be overridden`.
  `--force-chunked` (the row the triage did not run): exits 0 with `md1fnq0eqqpqggqpsgvzzs95gzsks24jcnv650hxv`,
  decode exits 1 with the same text. The first-occurrence spelling `wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))`
  mints `md1yppq2zcyxppgzcuqqcr8qpnypdll2zu`, which decodes. Keyed, with origins, on signet: same refusal.
- **I2** (`md-cli/src/seat/compose.rs:240-278`): `spend_equal_verdict` compares slot `i` of the two sides,
  but a template side keeps the holder's numbering while a decoded card is canonical. A template's own fresh
  card: `md descriptor --template 'wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))' --key @0=KA --key @1=KB --network
  signet --verify-against <its card>` exits 5 `md: --verify-against: NOT spend-equal — the VALUES half
  differs. ...`; same for `sortedmulti`; the `@0,@1` control exits 0.
- **I12** (`canonicalize.rs:218-235`, library): `remap_indices` mutates the tree before the
  `DivergentPathCountMismatch` guard, so an `Err` return leaves the caller's descriptor renumbered, and a
  repaired retry returns Ok on a different wallet (paths bound to the wrong keys). The triage test, copied in
  and run on the base: `2 tests run: 0 passed, 2 failed` (":40 descriptor mutated despite returned error",
  ":52 retry after repair diverges from one-shot control"), then removed.

RED runner (committed): `bash scripts/fix-bh1119-md-placeholder-order-gate-erows.sh target/debug/md` on the
base: e1-e7 and e11 FAIL, e8-e10, e16, e17 PASS; exit 1, `rows failed: 8` (measured by the fold on b4074415).

## 2. Rules (normative)

**R1 (C2). Canonicalisation recomputes the use-site baseline.** In `canonicalize_placeholder_indices`, on the
non-identity path only (the identity fast path at `canonicalize.rs:213-215` is unchanged and returns `d`
untouched):
0. After the identity fast path and before any mutation: if `d.tlv.use_site_path_overrides` holds two entries
   with the same index `i`, return `Err(Error::OverrideOrderViolation { prev: i, current: i })` (the encoder's
   existing error for this input on the identity path, `tlv.rs:123`), leaving `d` unchanged. With duplicates
   excluded, step 1's lookup is unique.
1. Before any mutation, resolve each OLD slot's use-site: `old[i]` = the override entry for `i` if present,
   else `d.use_site_path`, for every `i` in `0..n`.
2. Map to new slots: `new[perm[i]] = old[i]`.
3. Set `d.use_site_path = new[0]`.
4. Set `d.tlv.use_site_path_overrides` to the entries `(k, new[k])` for every `k` in `1..n` with
   `new[k] != new[0]`, in ascending `k`. If there is none: `None`, except that an input of `Some(vec![])`
   stays `Some(vec![])` (so `encode` still refuses it with `EmptyTlvEntry`, as it does on the identity path).
5. Step 4 replaces the `remap_tlv_vec` call on `use_site_path_overrides` (`canonicalize.rs:249-251`).
   Fingerprints, pubkeys, origin-path overrides and a Divergent `path_decl` are permuted as today.
Consequence (the testable form): for any template, the card minted from any spelling of its slot numbering is
byte-identical to the card minted from its first-occurrence spelling, single-string and `--force-chunked`
(same strings, same chunk-set id), and it decodes.

**R2 (C2, identities).** After R1, `compute_wallet_descriptor_template_id` and `compute_md1_encoding_id` of a
non-first-occurrence spelling equal those of its first-occurrence spelling (both canonicalise a clone first,
`identity.rs:84,195`). `compute_wallet_policy_id` is already equal on the base (it hashes per-slot resolved
values: `md encode --policy-id-fingerprint` prints `0x2f10ddf9` for both spellings of e1, seen) and stays so.
Only inputs whose card today carries an override keyed on `@0` (an undecodable card) or a redundant override
change any output; a card that decodes today is minted byte-identical. So no decodable md1, and no mk1
policy-id stub derived from a decodable md1, changes.

**R3 (I12). `canonicalize_placeholder_indices` is atomic:** on every `Err` it leaves `d` exactly as it was.
The `DivergentPathCountMismatch` length check (`canonicalize.rs:229-234`) moves to after the identity fast
path and before `remap_indices` (`:218`); every other `Err` (including R1 step 0's) already returns before any mutation. The identity
fast path still returns `Ok(())` for a short Divergent vector (pinned by
`canonicalize_identity_short_divergent_not_reached`, `:778`, unchanged). The in-branch guard then cannot
fire and is removed. R1 adds no fallible step after the first mutation.

**R4 (I2). Spend-equality compares canonical numberings.** `spend_equal_verdict(a, b)` first runs today's
three checks, in today's order (values, use-sites, structure), on `a` and `b` as given; if that verdict is
`Equal`, it returns `Equal`. Otherwise it clones both sides, runs `md_codec::canonicalize_placeholder_indices`
on each clone (a codec error maps through the existing `From<md_codec::Error> for CliError`), and returns
today's three checks on the canonical clones. An `Equal` on the inputs as given is kept because it already
required equal stripped `comparison_form` bytes, which mean the same scripts. `spend_equal` follows. On canonical inputs (every decoded
card, every first-occurrence template, every seating composition) the result is unchanged. A side that fails
canonicalisation was unreachable through the CLI before (a template that parses always canonicalises) and
stays so; no text or exit code is added.

**R5 (holder-visible behaviour).** No printed text changes. For the inputs above: `md encode` prints the
first-occurrence spelling's card (R1); `md decode` of it exits 0 with the first-occurrence spelling's
template and notes; `md descriptor --verify-against` exits 0 with the existing line `md: --verify-against:
SPEND-EQUAL — same template structure, per-slot key values and per-slot use-site paths. Origin metadata
(fingerprints, key origins, path declaration) is excluded from this comparison: it is seating/signing
guidance, not script content.`; `md inspect` of the card prints the first-occurrence spelling's ids.

**R6 (release).** This FIX bumps no version. CHANGELOG texts (section 5). The md-codec release that carries
this FIX and row #8 bumps md-codec and md-cli once, in a `release:` commit after both merge (md's
convention); the version numbers are that release's decision, not this FIX's. Pin bumps after the release:
section 6.

## 3. TRACE rows

Signet keys (BIP-39 test vector "abandon x11 about", fingerprint `73c5da0a`, `mnemonic convert --network
signet`): KA = m/48'/1'/0'/2' `tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ`,
KB = m/48'/1'/1'/2' `tpubDEYM1BmQ5rp2PWKvCgvQxNeUrEv8gu5819xRdmu6S23fYpS8x2icwAeoVaBTLyN3fGWJQcWoaiKMduTXWKtG9bXNpVrZPRF7XVxrANtAEcR`.
CLI rows go in a new `crates/md-cli/tests/bh1119_md_placeholder_order.rs`; library rows in
`crates/md-codec/tests/bh1119_md_placeholder_order.rs` (e12, e14, e15, e18) and the `seat/compose.rs` test module
(e13; md-cli has no lib target, so its unit rows live in the module, building `a` with `crate::parse::template::parse_template` and keys from `crate::parse::keys::parse_key`). Each test function is named exactly as its row.

| row | command / call | base (seen unless marked) | fixed (expected) |
|---|---|---|---|
| e1_c2_two_slots | `md encode 'wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))'` then `md decode <card>` | stdout `md1yppqqxppsg2qk3q2zc2pz7ecekxclml`, rc 0; decode rc 1 `md: codec error: use-site override keyed on baseline @0; @0 cannot be overridden` | stdout `md1yppq2zcyxppgzcuqqcr8qpnypdll2zu`, rc 0; decode rc 0, stdout `wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))` |
| e2_c2_force_chunked | `md encode --force-chunked` of e1's template, and of `wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))` | `md1fnq0eqqpqggqpsgvzzs95gzsks24jcnv650hxv` rc 0, decode rc 1 (same text); differs from the first-occurrence set `md1f5dpaqqpqggzskppsg2qk8qqxw2kucj9lvgv06` | stdout `md1f5dpaqqpqggzskppsg2qk8qqxw2kucj9lvgv06`, decode rc 0 |
| e3_c2_three_slots | `md encode 'wsh(multi(2,@1/<2;3>/*,@2/<2;3>/*,@0/<0;1>/*))'`, decode | `md1yzpqqxppsgscp4gs9pvczskeu6m3ealdmrn0`, decode rc 1 (same text) | `md1yzpq2zcyxpzrq9j5qqcsv998hnqcexfa`, decode rc 0, stdout `wsh(multi(2,@0/<2;3>/*,@1/<2;3>/*,@2/<0;1>/*))` |
| e4_c2_baseline_moves | `md encode 'wsh(multi(2,@1/<0;1>/*,@0/<2;3>/*))'`, decode | `md1yppq2zcyxppgzc5qqcpu38g4sky8np2`, decode rc 1 (same text) | `md1yppqqxppsg2qknq2zc2ktzhwekmddzh`, decode rc 0, stdout `wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))` |
| e5_c2_keyed | `md encode "wsh(multi(2,@1/48'/1'/1'/2'/<2;3>/*,@0/48'/1'/0'/2'/<0;1>/*))" --key @0=KA --key @1=KB --fingerprint @0=73c5da0a --fingerprint @1=73c5da0a --network signet`; then `md address --network signet <cards>` | 5 chunks `md1f2h7vzq...`, decode rc 1 (same text) | the 5 chunks of the first-occurrence spelling (`@0`/`@1` swapped in the template and `--key @0=KB --key @1=KA`), first `md1fr6mpzq9q2tvyw89ykcguy4q2zcyxppgzcuqqc9uyw0za5z4eutks2zhqjvuqxvmnuzkpw9lvu`; decode rc 0; first address `tb1q0v66c8vdpjv67fx77penkcx6lealfln60ey7vjte5evmdtc378uq44nq3u` |
| e6_i2_multi_noncanonical | `md encode 'wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))' --key @0=KA --key @1=KB --network signet --out own.md1`; `md descriptor --template <same> --key @0=KA --key @1=KB --network signet --verify-against own.md1` | rc 5, `md: --verify-against: NOT spend-equal — the VALUES half differs. ...` | rc 0, the R5 SPEND-EQUAL line |
| e7_i2_sortedmulti_noncanonical | e6 with `sortedmulti` | rc 5, VALUES half | rc 0, SPEND-EQUAL |
| e8_i2_control_canonical | e6 with `wsh(multi(2,@0/<0;1>/*,@1/<0;1>/*))` | rc 0, SPEND-EQUAL | unchanged |
| e9_control_canonical | `md encode 'wsh(multi(2,@0/<2;3>/*,@1/<0;1>/*))'` | `md1yppq2zcyxppgzcuqqcr8qpnypdll2zu`, decodes | unchanged |
| e10_control_noncanonical_same_use_site | `md encode 'wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))'` | `md1yppqqxppsg2vlumagltz27le`, decodes | unchanged (R2: a decodable card does not change) |
| e11_i2_divergent_use_sites | e6 with `wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))` | encode rc 0; verify rc 1 (the card does not decode) | rc 0, SPEND-EQUAL |
| e12_c2_identities_follow_the_canonical_spelling | md-codec library: `D` = `decode_md1_string("md1yppq2zcyxppgzcuqqcr8qpnypdll2zu")`; `N` = `D` relabelled as e1's template parses (tree `MultiKeys` indices `[1, 0]`, `use_site_path` = `<0;1>`, `use_site_path_overrides` = `Some(vec![(1, <2;3>)])`, all else as `D`); `canonicalize_placeholder_indices(&mut N.clone())`, and `compute_wallet_descriptor_template_id`, `compute_md1_encoding_id`, `compute_wallet_policy_id` of `N` and `D` | not run (by the code: canonicalised `N` keeps baseline `<0;1>` with `[(0, <2;3>)]`, so it differs from `D`; policy ids equal: `0x2f10ddf9` for both spellings, seen via `md encode --policy-id-fingerprint`) | canonicalised `N` == `D`; all three ids equal: template id `ee4e423a9b62fec656e822b2022eafb8`, encoding id `a343d85c7e3fdc1a76a2b1adee39e2c0`, policy id `2f10ddf928ef27bbdbf8a32c5007d61f` (from `md inspect` of `D` on the base, seen) |
| e13_i2_verdict_either_order | md-cli unit: `a` = e6's template parsed with `--key @0=KA`, `--key @1=KB`, signet (not canonicalised); `b` = a clone of `a` passed through `canonicalize_placeholder_indices`; `spend_equal_verdict(a, b)` and `spend_equal_verdict(b, a)` | not run (`Values` expected on the base by the code at `compose.rs:244-252`) | both `SpendEqualVerdict::Equal` |
| e14_i12_error_leaves_descriptor_unchanged | library: the triage t4 test `error_leaves_descriptor_unchanged` (n = 2, `MultiKeys [1, 0]`, `Divergent(vec![])`) | FAIL at `assert_eq!(d, pre)` (seen) | `Err(DivergentPathCountMismatch { n: 2, got: 0 })` and `d == pre` |
| e15_i12_repair_and_retry_matches_one_shot | library: triage t4 `repair_and_retry_matches_one_shot` (Divergent `[84', 48']` after a failed call) | FAIL, retry gives paths `[84', 48']` with tree `[0, 1]` against the control's `[48', 84']` (seen) | retry equals the one-shot control |
| e16_i2_sortedmulti_respelled_same_wallet | `md encode 'wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))' --key @0=KA --key @1=KB --network signet --out own.md1`; `md descriptor --template 'wsh(sortedmulti(2,@1/<0;1>/*,@0/<0;1>/*))' --key @0=KA --key @1=KB --network signet --verify-against own.md1` | rc 0, SPEND-EQUAL (seen) | unchanged (rc 0) |
| e17_i2_noncanonical_replaced_key | `md encode 'wsh(multi(2,@1/<0;1>/*,@0/<0;1>/*))' --key @0=KA --key @1=KB --network signet --out own.md1`; `md descriptor --template <same> --key @0=KA --key @1=KC --network signet --verify-against own.md1` (KC = m/48'/1'/2'/2' `tpubDEPGZXGPF2v3uNGTWHteRSQnZdEFfMvh6Dcp3DykkBE48hTA6pe3Es6nWH5dAThPyvqmZGmZqQhTw5kxqgR5uJW6Ei2VWyAAN92yLcYBvzV`) | rc 5, VALUES (seen) | unchanged (rc 5) |
| e18_c2_duplicate_override_refused | md-codec library: n = 2, tree `Wsh(Multi [1, 0])`, baseline `<0;1>`, `use_site_path_overrides = Some(vec![(1, <2;3>), (1, <4;5>)])`; `canonicalize_placeholder_indices(&mut d)` and `encode_payload(&pre)` | debug: panic at `canonicalize.rs:270` post-condition (seen) | `Err(OverrideOrderViolation { prev: 1, current: 1 })`, `d == pre`; `encode_payload` is `Err` |

Plausible wrong implementations and the row that catches each:
- Recompute the baseline but keep overrides equal to it (redundant): e3 (`@1` equals the new baseline; the
  decoder refuses `RedundantUseSiteOverride`).
- Swap the `@0` override with the old baseline and key the old baseline at `perm[0]`: right for e1, wrong
  for e3 (leaves a redundant override at new `@1`).
- Fix the encoder (or a decode-and-retry in `encode`) instead of `canonicalize`: e12 (identities stay wrong).
- Keep `remap_tlv_vec` and only rewrite an entry keyed on 0: e4 (the old baseline `<2;3>` must become an
  override at new `@1`).
- Canonicalise only the first argument of `spend_equal_verdict`: e13 (`(b, a)` order).
- Canonicalise both sides unconditionally (R4 without the as-given pass): e16 (rc 5).
- Resolve duplicate overrides by taking the first (or last) entry instead of refusing: e18 (the card would be
  minted, bound to a guessed use-site).
- Move the I12 guard before the identity fast path: `canonicalize_identity_short_divergent_not_reached`
  (existing) fails.
- Clone-and-restore only on the Divergent error: e14 still passes, but R3 holds for every `Err`; the other
  errors already precede mutation, so no further row is needed (checked by reading `:157-210`).

Coverage map (rule -> rows): R1 -> e1, e2, e3, e4, e5, e9, e10, e18; R2 -> e10, e12, and the unchanged goldens
(section 4); R3 -> e14, e15, e18, `canonicalize_identity_short_divergent_not_reached`; R4 -> e6, e7, e8, e11, e13, e16, e17;
R5 -> e1-e8, e11 (texts are existing ones); R6 -> section 5 and 6 (no row; review checks the diff).

## 4. Tests that change

- `crates/md-codec/src/canonicalize.rs` `permute_with_use_site_path_overrides` (`:812-834`): today asserts the
  defect (`overrides == vec![(0, custom)]`). New expectation: `d.use_site_path == custom` and
  `d.tlv.use_site_path_overrides == Some(vec![(1, no_multipath())])`. Rewrite it in the RED commit, with a
  one-line comment that it pinned C2.
- `canonicalize.rs` `canonicalize_short_divergent_returns_typed_error` (`:736-769`): add `let pre = d.clone();`
  before the call and `assert_eq!(d, pre);` after the variant check.
- Goldens: none expected to change (R2: only undecodable outputs change). If any wire golden, vector file
  (`crates/md-codec/tests/vectors/`), corpus or proptest regression changes, STOP and report it: that would be
  a decodable card changing.
- New: e1-e18 (section 3).

## 5. CHANGELOG (exact texts)

Under a new `## md-codec [Unreleased]` header placed above `## md-cli [0.20.8]` (row #8 appends to the same
header; whichever lands second merges the two lists), `### Fixed`:
> - **A template that numbers its slots out of first-occurrence order now mints a card that decodes.** `canonicalize_placeholder_indices` recomputes the use-site baseline from the new `@0` and keeps an override only where a later slot differs. Before, a template such as `wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))` minted a card the decoder refused ("use-site override keyed on baseline @0"); it now mints the same card as its first-occurrence spelling. The template id and encoding id of such a spelling now equal its first-occurrence spelling's; no card that decoded before changes.
> - **`canonicalize_placeholder_indices` leaves the descriptor unchanged when it returns an error.** The Divergent path-count check now runs before any index is rewritten.

Under `## md-cli [Unreleased]`, `### Fixed` (shared with row #4's entry if both land):
> - **`md descriptor --verify-against` compares slot numberings in canonical order.** A template that numbers its slots out of first-occurrence order no longer reports its own fresh card as "NOT spend-equal — the VALUES half differs".

No toolkit-manual change: no flag, help or printed text changes (the manual's `--verify-against` row,
`42-md.md:222`, describes the relation, which is unchanged). No mnemonic-gui change.

## 6. Release and pin bumps (after the shared release with row #8)

Consumers that build md-codec and mint or hash md1 (pins at their default heads, read 2026-11-22):

| repo | where | pin today |
|---|---|---|
| mnemonic-toolkit | `crates/mnemonic-toolkit/Cargo.toml:73` | git rev `cf35d61a` (md-codec 0.47.0 era) |
| mnemonic-engrave | `crates/me-cli/Cargo.toml:44` | git rev `c8bb97a5` (md-codec 0.48.5) |
| mnemonic-refugium | `Cargo.toml:26` (mr-cli, mr-codec via workspace) | git rev `c8bb97a5` |
| refugium-wallet | through mr-codec `rev 7fc61f6a` (`Cargo.toml:45`) | follows mnemonic-refugium |
| mnemonic-gui | through mnemonic-toolkit `tag mnemonic-toolkit-v0.105.1` (`Cargo.toml:80`) | follows the toolkit |
| mnemonic-key | `crates/mk-cli/Cargo.toml:34` | crates.io `0.42.0` (reads md1 cards it decodes, which are canonical; bump optional) |

Each bump is its own PR in its repo (with the `[patch]` and `vendor/` moves CLAUDE.md "Pins" requires);
not part of this FIX.

## 7. Interaction with row #8 (`FIX_bh1119_md_encoder_admission`, T9; not authored here)

- #8 makes `encode_payload` decode its own output and return the decoder's error. With #8 and without this
  FIX, e1-e5 change from "mints an undecodable card" to "encode refuses" (`md: codec error: use-site override
  keyed on baseline @0; ...`, exit 1): a fail-closed backstop, still RED against the rows above. With both,
  the rows pass. Either merge order is safe; neither row's RED evidence depends on the other being absent
  except e1-e5's base column, which reads "encode rc 1" if #8 merged first.
- #8 moves four decoder tests (`decode.rs:200-250` area) to a raw bit writer; this FIX touches
  `canonicalize.rs` and `seat/compose.rs`: no shared lines. Both add to `## md-codec [Unreleased]`.
- R1 makes canonicalisation stop producing the `@0` override; #8's self-decode would still catch a redundant
  override on the identity path (a hand-built canonical descriptor), which R1 deliberately leaves alone.

## 8. Gate line

`bash scripts/fix-bh1119-md-placeholder-order-gate.sh` is the gate. The builder commits it with its coordinator catalog row, and adds
a catalog row for `scripts/fix-bh1119-md-placeholder-order-gate-erows.sh`; row #4 has its own gate. Usage: no arguments, run from the
repo root, `$HOME/.cargo/bin` first in PATH, `--locked` everywhere. Every step runs; the wrapper exits 0 only if all passed, else exits 1
after printing the failed step names. There is no quick mode.
- Step 1: `cargo build --locked -p md-cli`, then `bash scripts/fix-bh1119-md-placeholder-order-gate-erows.sh target/debug/md`. That
  script is committed with this FIX: the E block of the author's red.sh (its header, the `erow`, e2, e5 and `vrow` rows) plus e16 and e17;
  it exits 0 only when no E row fails; on a base build it exits 1 with `rows failed: 8`.
- Step 2: the named library and unit rows, one run each, as `cargo nextest run --locked -p <crate> -E 'test(=<path>)'`, where `<path>`
  is the nextest test name: for the integration rows `e12_c2_identities_follow_the_canonical_spelling`,
  `e14_i12_error_leaves_descriptor_unchanged`, `e15_i12_repair_and_retry_matches_one_shot`, `e18_c2_duplicate_override_refused` (`-p md-codec`);
  for the unit tests `canonicalize::tests::permute_with_use_site_path_overrides` and
  `canonicalize::tests::canonicalize_short_divergent_returns_typed_error` (`-p md-codec`) and `seat::compose::tests::e13_i2_verdict_either_order`
  (`-p md-cli`). Before each run, the match count is checked: `cargo nextest list --locked -p <crate> --message-format oneline -E '<same filter>' 2>/dev/null | wc -l`
  must print 1.
- Step 3: the repo CI set, as `.github/workflows/ci.yml` runs it: `cargo nextest run --locked --workspace --all-targets --all-features
  --no-fail-fast --build-jobs 6 -j 6`; `cargo test --locked --workspace --doc --all-features`; `cargo clippy --locked --workspace --all-targets
  --all-features -- -D warnings`; `cargo fmt --all --check`; `cargo build --locked -p md-cli --no-default-features`;
  `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps --document-private-items --all-features`;
  `(cd design && sha256sum -c display-grouping-vectors.tsv.sha256)`; `sha256sum --check --strict ci/shared-ci.sha256`;
  `bash ci/docs-only.test.sh`; `bash ci/code-docs-check.test.sh`; `bash ci/code-docs-check.sh`.
- Step 4: `git diff --exit-code "$(git merge-base HEAD origin/main)" -- crates/md-codec/tests/vectors design/display-grouping-vectors.tsv`
  (no golden moved against the base; a changed golden is a STOP, section 4).

"Shown RED" means the step-1 E-row script on a base build (`rows failed: 8`, exit 1), recorded in the builder's report; the wrapper is not
run on the base. Not covered: the bitcoind-differential and fuzz-smoke workflows (run as CI on the PR), the freebsd and musl/aarch64 jobs,
the consumer pin bumps, the release commit.

## 9. FOLLOWUPS

Closes nothing in `design/FOLLOWUPS.md`; lines to file after merge are listed in the author's report.
