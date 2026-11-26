# descriptor-mnemonic bug-hunt fix branch: adversarial whole-diff review, round 1

Branch `claude/project-thread-utm1aa` (4 commits, `92b40d5f..5398e118`) against `origin/main`. Only md-cli source, the BIP, the CHANGELOG and tests changed. md-codec is untouched.

**Verdict: 0 Critical, 0 Important, 8 Minor, 4 Nit.** I found no funds-level defect. Every new refusal sits on a mint path. On every existing card I tried, the read verbs keep their exit codes and their stdout. The only stdout change is the origins display in `md inspect`, and Brian's ruling asks for it. One verb does change its result on an existing card: `md verify` on a keyed card with an elided origin now reports MISMATCH. Brian's ruling says no such cards are held, so I graded it Minor (m1).

## Method

- I built the branch `md`. I also built a second `md` from a `git archive origin/main` copy in its own target directory, so the two could be compared on the same inputs.
- To make the cases needed, I re-headered real xpubs with a stdlib Python helper. It sets depth, child number and version, or zeroes the chain code. The scripts are in `scratchpad/rev/`.
- I ran the scoped suites (`CARGO_INCREMENTAL=0 cargo test --locked -j 2 -p md-cli --test …`): `bh1125_md_empty_origin`, `bh1125_md_mint_refusals`, `cli_keyed_excess_origin_note`, `cli_unseatable_template_advisory`, `cmd_verify`, `h13_*`, `m5_*`, `printed_remedy_runs`, `single_print`, `baseline_gapb`, `bh1119_md_decompose_routes`, `cmd_decompose`, `cmd_decompose_roundtrip`, `n2_emit_md1`, `seating_vectors`. All passed; log in `scratchpad/rev/branch-tests.log`.
- I ran the two new test files against origin/main source. Log in `scratchpad/rev/main-newtests.log`.
- I ran one mutation build, with the verify fill and the prove.rs fill both removed. Log in `scratchpad/rev/mut.log`.
- Housekeeping: I deleted my scratch copies and their target directory. To make sure no shared-inode fingerprint from those builds could leak into your cache, I also ran `cargo clean -p md-cli -p md-codec` in `/home/user/descriptor-mnemonic/target`. That freed about 6.8 GiB, so the next build there recompiles md-codec and md-cli. The working tree is clean and no tracked file was touched.

## Q1. Admission versus existing cards

I minted cards with origin/main's `md` and read them with the branch `md` (`decode`, `decode --json`, `inspect`, `inspect --json`, `descriptor`, `address`, `bytecode`, `repair`). I used three kinds of card:
- a keyed elided canonical `wsh(sortedmulti)`, with and without fingerprints;
- a keyed non-canonical `tr(@0,multi_a(..))` with no origin, which partial-decodes with exit 4;
- an all-zero-chain-code keyed card (X4).

Results:
- **Exit codes are identical on every verb**: 0/0, 4/4, 1/1, 2/2.
- **stdout is identical everywhere except one place.** For a keyed slot with no origin, `md inspect`'s `origins:` block now prints `@0: [fp] (origin path not on the card)` instead of `@0: m` / `[fp/m]`. Brian's ruling asks for this.
- **stderr gains warnings only**: the missing-origin warning and the md#6 fingerprint note. `md decode` drops the `--path m` advice line.
- **X4, md#4 and md#5 checks are not on any read path.** An existing zero-chain-code card decodes, inspects, renders, derives addresses, verifies and dumps bytecode at exit 0. `md4_existing_card_still_decodes` covers the `c:` case.
- **md#3 (`refuse_multipath_without_wildcard`) cannot change a read.** It runs under `Disposition::Refuse` only (`parse/template.rs:2979`). Read verbs use `Warn`. `keyless_advisory::needs_no_key` parses twice with `Refuse`, but rendered card templates always end in `/*`, and the refusal is the same in both parses, so the predicate cannot change.
- **Seating** (`crate::seat::run`, `seat/compose.rs`) gains no new check. Its seated `path_decl` is the policy card's, and a keyless elided policy card did not seat on origin/main either (unfilled slots, `@0 []`). So the `--emit md1` mint (`refuse_unrestorable_card`) never sees a seated empty origin. A keyed card seated from a `/*h` keyless policy is now refused at `--emit md1` (md#5). That is a new mint; the existing card still renders.
- **`md verify` (route "apply the same fill") does change its result** on an existing keyed elided card. See m1.
- **decompose route 1** builds from a concrete descriptor, not from a card. Its fill changes nothing observable (see m6).

## Q2. md#1 mint

- **Mainnet: the cards are byte-identical** to the `--path` cards for `wpkh`/`bip84`, `tr`/`bip86`, `pkh`/`bip44`, `sh(wpkh)`/`bip49` and `wsh(sortedmulti)`/`bip48`. I compared md5 of stdout; the wsh case also matched an inline `48'/0'/0'/2'` template byte for byte.
- **Testnet, signet and regtest: coin `1'` is written.** The no-path card equals the `--path "m/48'/1'/0'/2'"` card on all three networks. It does **not** equal `--path bip48 --network testnet`, because named paths always write coin `0'` (m2).
- **Wallet-policy id, keyless template**: unchanged (`465a5fc1…` on both).
- **Wallet-policy id, explicit-origin keyed card**: unchanged (`602b520e…` on both).
- **Wallet-policy id, keyed no-origin mint, mainnet**: unchanged. Old elided card, new filled card and `--path bip48` card all give `52bcbf43…`. A mixed template (one slot inline, one elided) with reversed slot order also gives the same id old and new (`ede5d624…`); the fill and the md#2 index lookup survive placeholder canonicalisation.
- **Wallet-policy id, keyed no-origin mint, testnet**: it **changes**. Old gives `52bcbf43…` (hashed as coin 0'), new gives `8ff60878…`. This follows from coin 1', so it is correct, but the CHANGELOG says the id is unchanged (m3).
- **Shapes newly refused**: every one is either a key the canonical origin contradicts or a keyed card with no origin, which is the ruling. Examples:
  - a BIP-87 depth-3 key on `wsh` without `--path`; it mints with `--path "m/87'/0'/0'"`, byte-identical to before;
  - an account-1 key without a path;
  - 2' keys on `sh(wsh)`;
  - keyed `tr` with a script tree, `wsh(and_v…)` or `multi_a` without a path;
  - `--path m`.

  Each gets an actionable message that asks for `--path`. I found nothing legitimate that now fails to mint once the correct path is given.

## Q3. md#2, md#3, md#4, md#5 and X4: anything legitimate refused?

Still minting, with the same bytes as old unless noted:
- BIP-48 depth-4 `2'` keys on `wsh`, and `1'` keys on `sh(wsh)`;
- `--path bip48` `c:pk_k` and `c:pk_h` under `and_v` (identical cards);
- `tr(@0,multi_a(1,@1))` with a path (identical);
- `wsh(pkh(..))`;
- `@N/**`. The card is the canonical-origin card, because `/**` desugars before the md#3 lexer;
- F-411 unhardened excess (`wpkh(@0/84'/0'/0'/0/<0;1>/*)` with a depth-3 key). It keeps the note and the same bytes;
- keyless `/*h`, the same bytes.

Newly refused, all correctly in my judgment:
- **md#4**: `c:or_i(pk_k,pk_k)`, keyed or keyless. A `K`-typed fragment always holds bare `pk_k`/`pk_h`, which the decoder reads back as `c:pk_k`/`c:pk_h`, so the card can never render.
- **md#5**: keyed `/*h`.
- **md#3**: `@0/<0;1>` with no wildcard, keyed or keyless. BIP-388 key expressions always end in `/*`.
- **X4**: an all-zero chain code.
- **md#2**: a hardened step past the key's depth.

Mint-only: yes for md#2, md#4, md#5 and X4. They run only from `md encode` and from `mint_md1_cards` (which serves `md encode`, `md descriptor --emit md1` and the decompose proofs). md#3 also runs in the `md descriptor --template` / `md address --template` build path (m5).

## Q4. Changed fixtures

- **`cli_keyed_excess_origin_note`.** `excess_hardened_suffix_is_refused_not_noted` is a correct new behaviour: the key cannot be the one a hardened 4th step names. The narrow-note guard now seats a key really derived at `m/0/0/0`, and a new row pins the shallow-origin refusal. Correct.
- **`cli_unseatable_template_advisory`, `cmd_verify`, `h13_*`, `m5_*`.** These fixtures had keys whose header contradicted the canonical origin (child `1'` under `48'/…/2'`, or depth 3). They were replaced with re-headered xpubs: the same chain code and point, depth 4, child `2'`. The assertions of these tests do not depend on derivation, so nothing is hidden. See Nit n4 on how they are labelled.
- **`printed_remedy_runs::c4`.** The template gained inline `48'/1'/0'/3'` origins because keyed `tr` with a tree now needs an origin. The row still tests what it names, the printed `--key` flag. Correct.
- **`single_print`.** The expected stderr gains the md#6 warning, because `encode()` passes `--fingerprint`. Correct.
- **`baseline_gapb`.** The two removed round-trip rows asserted that `c:and_v(v:pk,pk_k)` decodes to `c:and_v(v:pk,pk)`, which is exactly the md#4 defect (unparseable). Moving the mutation kill to a refusal assertion is correct. The bare `PkK`/`PkH` walker arms are now reachable only in refused templates.

## Q5. Do the new tests fail without the fix?

Yes. Against origin/main source:
- `bh1125_md_empty_origin`: 10 of 11 fail; the control `keyless_mint_still_elides` passes.
- `bh1125_md_mint_refusals`: 10 of 15 fail; the 5 controls pass (`md2_consistent_origin_still_mints`, `md4_existing_card_still_decodes`, `md4_check_over_key_still_mints`, `md5_keyless_hardened_wildcard_still_mints`, `md6_encode_without_fingerprint_is_silent`).

This matches the PR body.

With the mutation (both fills removed), only `cmd_verify::a_mismatch_caused_by_missing_fingerprints_says_so` failed. No test fails when the prove.rs fill is removed (m6).

## Findings

### Critical
None.

### Important
None.

### Minor

**m1. `md verify` now reports MISMATCH on an existing keyed elided card, and the message gives no reason.** `crates/md-cli/src/cmd/verify.rs:63-68`.
- Scenario: a card minted by origin/main `md encode "wsh(sortedmulti(2,@0/<0;1>/*,@1/<0;1>/*))" --key … --key …`. Before, `md verify` with the same arguments printed `OK`. Now it prints `md: MISMATCH: expected 1144-bit payload, got 1116-bit (143 vs 140 bytes)` and exits 1, with no hint.
- The only way to make it pass is `--path m`, which md now calls invalid everywhere else.
- Brian's ruling says no holders exist, so this is Minor. Even so, the verb whose job is to check an engraved plate now fails it with a reason it does not state.
- Suggestion: when the card is keyed and `keyed_slots_without_origin(&decoded)` is non-empty, add a hint. It should say the card predates md#1, that its origin is not on the card, and that `md verify --path m` compares it as minted.

**m2. Off mainnet, the no-path default and the named paths write different origins.** `crates/md-cli/src/cmd/encode.rs:448-470` against `crates/md-cli/src/parse/path.rs:150`.
- Scenario: on testnet, `md encode T --key …` writes `48'/1'/0'/2'`, but `md encode T --key … --path bip48` writes `48'/0'/0'/2'`. So there are two md-blessed "standard" cards with different ids for the same testnet wallet.
- The doc comment and the CHANGELOG say the fill gives "the same card `--path bip84` (etc.) mints". That holds only on mainnet.
- Verifying a no-path testnet card with `--path bip48` misfires the F-582 hint: "The card is LARGER … Fingerprints … Do not re-mint". No fingerprints are involved (1145 vs 1144 bits).
- Suggestion: either make the named paths network-aware, or reword the docs and the hint.

**m3. The CHANGELOG says the wallet-policy id is unchanged, but it changes off mainnet.** `CHANGELOG.md` md#1 bullet. On testnet a keyed no-origin mint moves from `52bcbf43…` to `8ff60878…`, because coin `1'` is now written. The change itself is correct; the sentence should say "unchanged on mainnet".

**m4. The fill turns an assumption into an explicit claim, and only the child number is checked.** `encode.rs:347-447` (md#2) and `encode.rs:457`.
- md#2 compares only the step at the key's depth. A depth-3 BIP-49 account key (child `0'`) seated in `wpkh` with no path passes, and the card now states `84'/0'/0'` explicitly. A testnet key derived at coin `0'` gets coin `1'` written.
- Before, the origin was an elision the BIP resolved; now it reads as a recorded fact.
- Addresses are unaffected; signer matching is wrong.
- The ruling asks for exactly this, but nothing on stderr says a path was written that the user never gave. The toolkit half of the ruling prints an `info:` line for this case.
- Suggestion: add a one-line stderr note naming the origin written for each filled slot.

**m5. md#3 refuses on two render verbs, while the PR body says refusals run only when minting.** `parse/template.rs:2979`, reached through `cmd/build.rs:77`.
- `md address --template "wpkh(@0/<0;1>)" --key …` used to print `bc1q…` (silently ranged); now it exits 1. The refusal says "a card minted from this template", but nothing is being minted.
- Refusing is safer, and the CHANGELOG does list `descriptor` and `address`. The PR body's claim and the message wording should match what the code does.

**m6. The fill in decompose route 1 is untested, and as far as I can find it changes nothing observable.** `decompose/prove.rs:190-194`. Removing it breaks no test in `cmd_decompose`, `cmd_decompose_roundtrip` or `bh1119_md_decompose_routes`.
- A decompose template lacks an origin only when the descriptor key has none. Route 2 and `--emit commands` already refuse those, and route 1's strict `normalised` comparison would withhold the route anyway.
- The `md verify` fill is covered only incidentally, by `cmd_verify`'s fingerprint-hint row.
- Suggestion: add a direct row, "a no-path keyed card verifies OK against its template and keys on mainnet and testnet". Either add a route-1 row or drop the parity code.

**m7. The BIP now says MUST, but the reference codec does not enforce it.** `bip/bip-mnemonic-descriptor.mediawiki` §"Elided origins".
- The text says an encoder MUST NOT write a depth-0 origin on a keyed card, and a reader MUST NOT present the key at the canonical origin.
- md-codec (unchanged) still mints keyed depth-0 cards through `encode_payload`. That is the toolkit and refugium path, and the toolkit's `bundle --descriptor` is the original md#1 source.
- md-codec also still hashes a keyed depth-0 origin as the canonical one in the wallet-policy id, which the warning text ("not assumed to be the standard path") now contradicts.
- The PR body lists the toolkit side as in progress, but nothing in `design/FOLLOWUPS.md` records the codec-level gap or the cross-repo items (toolkit manual `42-md.md` printed text, the Go port, the codec gate). The repo convention asks for mirrored FOLLOWUPS entries.

**m8. `md descriptor` still renders a legacy keyed no-origin key as `[fp]xpub661…`, with a depth-0 master header.** `cmd/descriptor.rs:198-203`. A coordinator reading `[73c5da0a]xpub661…` takes the key to be that master. The warning is on stderr only. This is the PR's own open question (a)/(b); with no holders, Minor. I agree with the PR's recommendation (b): refuse `md descriptor` on such a card and send the user to `md address`.

### Nit

- **n1.** md#6 is not printed by `md descriptor`, which prints fingerprints in its `[fp/…]` origins, nor by `md bytecode`. The missing-origin warning is not printed by `md bytecode` either (`output_advisory.rs`).
- **n2.** For a keyless elided card, `md decode`/`md inspect` still print `@0: m`. The amended BIP says a depth-0 origin is not the master node. This predates the branch, and the PR lists it as open.
- **n3.** In `refuse_key_origin_contradiction` (`encode.rs:351-354`), any `expand_per_at_n` error makes it return `Ok(())` for every slot. Today the only such case is a non-canonical empty origin, which `refuse_unrestorable_card` refuses later. A future expansion error would skip md#2 silently.
- **n4.** The replacement fixtures in `cmd_verify`, `cli_unseatable_template_advisory`, `h13_*` and `m5_*` are re-headered keys: their child number is one they were not derived at. The comments say so. Labelling them "synthetic header, not derivable" would stop a later reader from reusing them as real BIP-48 vectors.
