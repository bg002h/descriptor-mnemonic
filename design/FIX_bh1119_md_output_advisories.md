# FIX_bh1119_md_output_advisories: keyed cards are labelled watch-only; the no-key warning names what opens the path (bh-md-M5, bh-md-I8; both Important)

Small-fix track (bughunt-1119 Minor/Nit regrade, coordinator `agent-reports/bughunt-1119-mn-regrade/md-grade.md`
L25 and L30; refutes `md-refute-bh-md-M5.md` (not refuted) and `md-refute-bh-md-I8.md` (overruled by the grade, I)).
Repo descriptor-mnemonic, branch `fix/md-advisories` from main `21cf1444d743f5336a00e9aeeeb799017082760f`. Crate
`md-cli` only: no md-codec change, no wire change, no new flag, type or command. Holder-visible changes: the stderr
class note on two verbs (an existing text), the `--experimental` no-key warning texts (new, PROPOSED), the
`md encode --experimental` help paragraph (new, PROPOSED), and the warning now also printed by `md decode --json`.
Author: Claude Opus 5.5 (claude-opus-5-5), 2026-11-24.

Toolchain: `export PATH=$HOME/.cargo/bin:$PATH`; cargo 1.85.0 (`rust-toolchain.toml`); `--locked`;
`CARGO_BUILD_JOBS=6`; nextest `--build-jobs 6 -j 6`; never `--release`. Signet only: the one key is the signet tpub
`KA` of `crates/md-cli/tests/bh1119_md_slot_bindings.rs:9` (public, watch-only); no seed or secret anywhere.

**No STOP is open.** The two defects share the `md decode --json` branch (`crates/md-cli/src/cmd/decode.rs:36-63`)
but not an emitter, so this is one document with two sections. Files this FIX changes: `crates/md-cli/src/cmd/
decode.rs`, `crates/md-cli/src/cmd/bytecode.rs`, `crates/md-cli/src/cmd/encode.rs`, `crates/md-cli/src/main.rs`,
`crates/md-cli/src/parse/template.rs`, new `crates/md-cli/src/keyless_advisory.rs`, new test file
`crates/md-cli/tests/bh1119_md_output_advisories.rs`, `crates/md-cli/tests/cmd_decode.rs`,
`crates/md-cli/tests/cmd_encode.rs`, `CHANGELOG.md`, new `scripts/fix-bh1119-md-output-advisories-gate.sh`; and in
mnemonic-toolkit `docs/manual/src/40-cli-reference/42-md.md` (lockstep, section 5).

Every "on the base" output below was run by the author on base `21cf1444` built with `cargo build --locked -p md-cli`.
Every "after the fix" output was run by the author on a scratch build of the section 2 and 3 behaviour (reverted, not
committed; `cargo nextest run --locked -p md-cli --all-features` on that scratch build: 1112 run, 1111 passed, the one
failure is `cmd_decode::decoding_a_bearer_card_warns_the_restorer`, which section 6 changes).

## 1. Defects (RED evidence, run on the base)

**bh-md-M5.** `md decode --json` (`decode.rs:55-58`) and `md bytecode` text and `--json` (`bytecode.rs:69-72`,
`:84-87`) pass `OutputClass::Template` unconditionally. On a card that carries a public key, stdout carries it
(`--json`: `descriptor.tlv.pubkeys[0]`, 65 bytes; `bytecode`: the payload hex, which includes the Pubkeys TLV) while
stderr says it carries none. `md inspect` and `md repair` already choose by `descriptor.is_wallet_policy()`
(`inspect.rs:84-88`, `repair.rs:122-126`).

```
$ md encode 'wpkh(@0/<0;1>/*)' --network signet --path bip84 --key "@0=$KA"     # two chunks C1 C2, exit 0
$ md decode --json C1 C2
exit=0; stdout JSON with "pubkeys": [[0, "3c8c2037ee4c...a890b4"]]
stderr: note: stdout is a keyless descriptor template (no keys)
$ md bytecode C1 C2          (and: md bytecode --json C1 C2)
exit=0; stdout payload-bits: 597 / payload-bytes: 75 / hex: 200ef521...4485a0
stderr: note: stdout is a keyless descriptor template (no keys)
$ md decode C1 C2
exit=0; stdout wpkh(@0/<0;1>/*)
stderr: note: stdout is a keyless descriptor template (no keys)          <- correct: stdout is the @N template only
```

**bh-md-I8.** The `--experimental` no-key warning (`encode.rs:133-140`, `decode.rs:111-116`) says whoever learns "its
preimage" can spend, for every shape the signature rule refuses. On a path opened by a timelock alone there is no
preimage, and anyone who has the policy and its public keys can spend that path once the lock is met (no signature
commits the transaction, so anyone who sees it can also redirect it). On `1` there is not even a lock. And
`md decode --json` returns at `decode.rs:62` before the warning check, so the restorer reading JSON gets no warning.

```
$ md encode 'wsh(or_i(pk(@0/**),after(100)))' --path bip84 --network signet --experimental
exit=0; stdout md1yq802gggqpsfj2dsqqqqvsf2pjymaa6veav
stderr: warning: --experimental relaxed the signature rule. This descriptor has at least one spend path that needs NO key, so whoever learns its preimage can spend it alone. If that preimage is engraved, THE PLATE IS BEARER ACCESS. Malleability, resource limits, repeated keys and timelock mixing were still checked.
        note: stdout is a keyless descriptor template (no keys)
```

The same warning, byte for byte, on the base for `wsh(or_d(pk(@0/**),older(144)))` (md1yq802gggqpsfs2wqqqqqjqlz0wj33mkg96c),
`wsh(or_d(pk(@0/**),sha256(H)))` (H below), `wsh(or_d(pk(@0/**),1))` (md1yq802gggqpsfs23s7wv9rxmal4u8q) and
`tr(@0/**,{sha256(H),after(100)})` with `--path bip84`, and for `md encode --json` of the first. `md decode` of the
first card prints `warning: this card has at least one spend path that needs NO KEY. Whoever learns its preimage can
spend that path alone — if the preimage is engraved, THE PLATE IS BEARER ACCESS. It was minted with --experimental;
the card carries no record of that, so this is derived from the policy itself.`; `md decode --json` of every one of
these cards prints only `note: stdout is a keyless descriptor template (no keys)` on stderr.
`H = 6c60f404f8167a38fc70eaf8aa17ac351023bef86bcb9d1086a19afe95bd5333`.

## 2. Spec delta, bh-md-M5 (no new text)

M5-S1. `md decode --json` chooses the class note from the decoded card. When the I8-S3 predicate holds on
`text::descriptor_to_template(&descriptor)` (the card has a no-key spend path), it prints the I8-S2 decode line. On a
card with `descriptor.is_wallet_policy()` it then prints NO class note. On a keyless card it then prints `TMPL`.
Otherwise it prints `WATCH` when `descriptor.is_wallet_policy()`, else `TMPL`.
M5-S2. `md bytecode`, text and `--json`, applies the same rule, the I8-S2 decode line included.
M5-S3. Unchanged: `md decode` text output keeps `OutputClass::Template` on every card (its stdout is the `@N` template
and carries no key); stdout of every verb, exit codes, the position of the class note (after stdout, before the
partial note on `decode --json`). A card carrying fingerprints but no Pubkeys TLV stays Template (a fingerprint is not
a key; same predicate as `inspect`/`repair`).

## 3. Spec delta, bh-md-I8 (new texts, PROPOSED (architect confirms))

I8-S1 (the analysis). New module `crates/md-cli/src/keyless_advisory.rs` with `pub enum Verb { Encode, Decode }` and
`pub fn render_keyless_warning(template: &str, verb: Verb) -> String` (one line, no trailing newline), the one
emitter both verbs call. It parses `template` exactly as the `--experimental` branch of `parse_template_ext` does
(`parse/template.rs:2946-2956`: `substitute_synthetic(template, ctx_for_template(template))`, then
`miniscript::expression::Tree::from_str` and `FromTree::from_tree`, no sanity or ext check), through a new
`pub(crate) fn relaxed_ms_descriptor(template: &str) -> Option<MsDescriptor<DescriptorPublicKey>>` in
`parse/template.rs` (any error -> `None`). It lifts the descriptor (`miniscript::policy::Liftable::lift`) to a semantic
policy `P` and computes:
- `K` = `P` with every `Key` leaf replaced by `Unsatisfiable`, then `.normalized()` — the conditions under which the
  descriptor can be spent with no key;
- `T` = `P` with every `Key` and every hash leaf (`Sha256`, `Hash256`, `Ripemd160`, `Hash160`) replaced by
  `Unsatisfiable`, then `.normalized()` — non-`Unsatisfiable` iff some keyless path needs no secret;
- `has_hash` = `K` contains a hash leaf anywhere.
A pure inner function `render_from(lifted: Option<&Policy<DescriptorPublicKey>>, verb: Verb) -> String` does the
`K`/`T`/text work, so the fallback rows are unit-testable. `<K>` below is `K`'s `Display` (rust-miniscript semantic
form: `after(100)`, `older(144)`, `sha256(<64 hex>)`, `and(..)`, `or(..)`, `thresh(k,..)`), printed in full.

I8-S2 (the texts; each is ONE stderr line; the pieces are concatenated in the order shown, each piece after the first
beginning with one space). Common pieces:
- `S_T` = ` A path there needs no secret at all: once its timelock condition is met, anyone who has this policy and its public keys can spend it.`
- `S_H` = ` Whoever learns a preimage named there can spend that path without a key once any timelock beside it is met. If such a preimage is engraved, THE PLATE IS BEARER ACCESS.`
- `TAIL_E` = ` Malleability, resource limits, repeated keys and timelock mixing were still checked.`
- `TAIL_D` = ` It was minted with --experimental; the card carries no record of that, so this is derived from the policy itself.`

Lock pieces (PROPOSED): one piece per distinct lock leaf of `K`, in `K`'s Display order, appended after `S_T`/`S_H` and
before the tail, each beginning with one space:
- `after(N)`, N < 500000000: ` after(N) is block height N, not N blocks from now; md cannot see the chain, so if the chain is already past height N, that path is open today.`
- `after(N)`, N >= 500000000: ` after(N) is Unix time N; once the chain's median time passes it, that path is open.`
- `older(N)`, bit 22 clear: ` older(N) is N blocks after each coin confirms.`
- `older(N)`, bit 22 set: ` older(N) is M seconds after each coin confirms.` (M = (N & 0xffff) * 512)

| case (first that applies) | `md encode` line | `md decode` line |
|---|---|---|
| F: lift failed (`None`) or `K` is `Unsatisfiable` | `warning: --experimental relaxed the signature rule. rust-miniscript could not prove that every spend path here needs a signature, and md could not name a spend path without one. Check every spend path of this descriptor before funding it.` + `TAIL_E` | `warning: this card's policy fails the rule that every spend path needs a signature, and md could not name a spend path without one. Check every spend path before funding it.` + `TAIL_D` |
| Z: `K` is `Trivial` | `warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key, NO secret and NO timelock: anyone who has this policy and its public keys can spend it now.` + `TAIL_E` | `warning: this card can be spent with NO key, NO secret and NO timelock: anyone who has this policy and its public keys can spend it now.` + `TAIL_D` |
| N: otherwise | `warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key when: <K>.` + (`S_T` if `T` is not `Unsatisfiable`) + (`S_H` if `has_hash`) + `TAIL_E` | `warning: this card can be spent with NO key when: <K>.` + (`S_T` if `T` is not `Unsatisfiable`) + (`S_H` if `has_hash`) + `TAIL_D` |

So `THE PLATE IS BEARER ACCESS` prints only when a preimage is named, and the word `preimage` never prints for a path
with no hash. Case F is not known to be reachable from the CLI (no RED row could reach it); it exists so the warning
never claims a keyless path it cannot name.

I8-S3 (when, which stream). Stderr only; nothing is added to stdout or to any JSON object; exit codes unchanged.
- `md encode`: replaces the warning at `encode.rs:133-140`, same trigger (`--experimental` and `needed_the_flag`),
  text and `--json` alike, same position (before the md1 output and the class note). The "not needed" note
  (`encode.rs:142-147`) is unchanged.
- `md decode` text: replaces the warning at `decode.rs:111-116`, same trigger (the two-parse F-610 predicate,
  `decode.rs:87-110`), same position (before the origins note and the class note).
- `md decode --json` (new): after the JSON is printed, the same predicate on `text::descriptor_to_template(&descriptor)`;
  if it holds, the decode line above, then the class note (section 2), then the partial note if any. If
  `descriptor_to_template` fails, no warning is printed and nothing else changes (stdout and exit as today).

I8-S4 (help; PROPOSED). `crates/md-cli/src/main.rs:191-193`, the last sentences of the `md encode --experimental`
doc comment, become: `A keyless path opens on what it names instead of a key: a preimage (whoever learns it can spend
that path, so an engraved preimage makes the plate bearer access) or a timelock alone (once it is met, anyone who has
the policy and its public keys can spend that path). Prints a warning naming those conditions, or a note when the
flag relaxed nothing.` The text before `Whoever` (L184-191) is unchanged. No other verb's help changes (`md address`,
`md descriptor` and `md verify` `--experimental` help, `main.rs:249`/`:317`/`:624`/`:814` area, name no preimage).

I8-S5 (refusal hint; PROPOSED). `parse/template.rs:3006-3009` hint becomes ` -- a spend path that needs no key. \`md compose\` refuses the
same shape without --experimental; pass --experimental here to encode it as EXPERIMENTAL (that path opens with no
key, on a preimage or a timelock it names; the warning --experimental prints says which)`.

## 4. TRACE rows (new file `crates/md-cli/tests/bh1119_md_output_advisories.rs`, unless marked unit)

Fixtures: `KA` (signet tpub above); `KEYED` = the md1 strings `md encode 'wpkh(@0/<0;1>/*)' --network signet --path
bip84 --key @0=KA` prints, minted inside each test (the chunk-set id is random, so not a constant; the two-chunk count
is asserted); `KEYLESS` = `md1yq802gggqpsqwgtua24e7ssf3` (`md encode 'wpkh(@0/<0;1>/*)' --network signet --path
bip84`); `CARD_AFTER` = `md1yq802gggqpsfj2dsqqqqvsf2pjymaa6veav`, `CARD_SHA` =
`md1yq802gggqpsfs2wkcc85qnupv73cl3cw4792z7kr2yprhmuxhjuazzr2rxh7jk74xvc20rflk6gdqmus`, `CARD_ONE` =
`md1yq802gggqpsfs23s7wv9rxmal4u8q` (minted on the base by the `encode` rows below; each test asserts its fixture's
mint reproduces it, provenance stated in the file). `WATCH` = `note: stdout is watch-only — public keys only, cannot
spend`; `TMPL` = `note: stdout is a keyless descriptor template (no keys)`. "line n" = stderr line n.

| row (test fn) | command | expected after the fix | on the base (seen) |
|---|---|---|---|
| R1 `m5_r1_decode_json_keyed_card_is_watch_only` | `md decode --json KEYED...` | exit 0; JSON `descriptor.tlv.pubkeys` has 1 entry; stderr exactly `WATCH\n` | stderr `TMPL` |
| R2 `m5_r2_bytecode_text_keyed_card_is_watch_only` | `md bytecode KEYED...` | exit 0; stdout 3 lines `payload-bits:`/`payload-bytes:`/`hex:`; stderr exactly `WATCH\n` | stderr `TMPL` |
| R3 `m5_r3_bytecode_json_keyed_card_is_watch_only` | `md bytecode --json KEYED...` | exit 0; JSON has `hex`; stderr exactly `WATCH\n` | stderr `TMPL` |
| C1 `m5_c1_decode_text_keyed_card_stays_template` | `md decode KEYED...` | exit 0; stdout `wpkh(@0/<0;1>/*)\n`; stderr exactly `TMPL\n` (passes on the base) | same |
| C2 `m5_c2_keyless_card_stays_template` | `md decode --json KEYLESS`, `md bytecode KEYLESS`, `md bytecode --json KEYLESS` | each exit 0, stderr exactly `TMPL\n` (passes on the base) | same |
| R4 `i8_r1_encode_absolute_timelock_path_names_the_lock` | `md encode 'wsh(or_i(pk(@0/**),after(100)))' --path bip84 --network signet --experimental` | exit 0; stdout `CARD_AFTER\n`; line 1 = E(N, K=`after(100)`, `S_T`, no `S_H`), exactly `warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key when: after(100). A path there needs no secret at all: once its timelock condition is met, anyone who has this policy and its public keys can spend it. after(100) is block height 100, not 100 blocks from now; md cannot see the chain, so if the chain is already past height 100, that path is open today. Malleability, resource limits, repeated keys and timelock mixing were still checked.`; line 2 `TMPL`; stderr contains neither `preimage` nor `BEARER` | line 1 the base warning (says `preimage`, `BEARER ACCESS`) |
| R5 `i8_r2_encode_relative_timelock_path_names_the_lock` | `md encode 'wsh(or_d(pk(@0/**),older(144)))' --path bip84 --network signet --experimental` | exit 0; line 1 as R4 with `older(144)` for `after(100)`, and the `older(144)` lock piece (` older(144) is 144 blocks after each coin confirms.`) for the `after(100)` piece | base warning |
| R6 `i8_r3_encode_hashlock_path_names_the_hash` | `md encode 'wsh(or_d(pk(@0/**),sha256(H)))' --path bip84 --network signet --experimental` | exit 0; stdout `CARD_SHA\n`; line 1 exactly `warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key when: sha256(H). Whoever learns a preimage named there can spend that path without a key once any timelock beside it is met. If such a preimage is engraved, THE PLATE IS BEARER ACCESS. Malleability, resource limits, repeated keys and timelock mixing were still checked.` (H expanded) | base warning (no `when: sha256(`) |
| R7 `i8_r4_encode_no_condition_path_says_no_secret_no_timelock` | `md encode 'wsh(or_d(pk(@0/**),1))' --path bip84 --network signet --experimental` | exit 0; stdout `CARD_ONE\n`; line 1 exactly `warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key, NO secret and NO timelock: anyone who has this policy and its public keys can spend it now. Malleability, resource limits, repeated keys and timelock mixing were still checked.` | base warning (says `preimage`) |
| R8 `i8_r5_encode_hash_or_timelock_names_both` | `md encode 'tr(@0/**,{sha256(H),after(100)})' --path bip84 --network signet --experimental` | exit 0; line 1 exactly `warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key when: or(sha256(H),after(100)).` + `S_T` + `S_H` + the `after(100)` lock piece + `TAIL_E` | base warning (no `after(100)`) |
| R9 `i8_r6_encode_json_prints_the_same_warning` | `md encode --json 'wsh(or_i(pk(@0/**),after(100)))' --path bip84 --network signet --experimental` | exit 0; JSON `phrase == CARD_AFTER`; line 1 identical to R4 line 1 | base warning |
| R10 `i8_r7_decode_text_timelock_card_names_the_lock` | `md decode CARD_AFTER` | exit 0; stdout `wsh(or_i(pk(@0/<0;1>/*),after(100)))\n`; line 1 exactly `warning: this card can be spent with NO key when: after(100). A path there needs no secret at all: once its timelock condition is met, anyone who has this policy and its public keys can spend it. after(100) is block height 100, not 100 blocks from now; md cannot see the chain, so if the chain is already past height 100, that path is open today. It was minted with --experimental; the card carries no record of that, so this is derived from the policy itself.`; last line `TMPL` | line 1 the base decode warning (`preimage`) |
| R11 `i8_r8_decode_json_timelock_card_warns` | `md decode --json CARD_AFTER` | exit 0; stdout JSON with `schema` = `md-cli/2` (byte-identical to the base: the author diffed base and scratch runs, stderr only differs); stderr exactly R10 line 1 + `\n` + `TMPL\n` | stderr only `TMPL` |
| R12 `i8_r9_decode_json_hashlock_card_warns` | `md decode --json CARD_SHA` | exit 0; stderr exactly `warning: this card can be spent with NO key when: sha256(H).` + `S_H` + `TAIL_D` + `\n` + `TMPL\n` | stderr only `TMPL` |
| R13 `i8_r10_decode_no_condition_card_text_and_json` | `md decode CARD_ONE`; `md decode --json CARD_ONE` | both exit 0; line 1 of each exactly `warning: this card can be spent with NO key, NO secret and NO timelock: anyone who has this policy and its public keys can spend it now. It was minted with --experimental; the card carries no record of that, so this is derived from the policy itself.`; `--json` stderr is that line then `TMPL` | text: base decode warning; json: only `TMPL` |
| R14 `i8_r11_encode_help_names_timelock_paths` | `md encode --help` | stdout contains `or a timelock alone (once it is met, anyone who has the policy and its public keys can spend that path)` and does not contain `Whoever learns the preimage of a keyless path` | contains the old sentence |
| R15 `i8_r14_hash_and_timelock_path_says_no_secret_is_not_free` | `md encode 'wsh(or_d(pk(@0/**),and_v(v:sha256(H),after(100))))' --path bip84 --network signet --experimental` | exit 0; line 1 exactly `warning: --experimental relaxed the signature rule. This descriptor can be spent with NO key when: and(sha256(H),after(100)).` + `S_H` + the `after(100)` lock piece + `TAIL_E` (H expanded); line 1 contains no `needs no secret` | base warning; on mutant A (`S_T` keyed on a timelock in `K`) the extra `S_T` |
| R16 `i8_r15_hash_only_on_a_keyed_path_is_not_bearer` | `md encode 'wsh(or_i(and_v(v:pk(@0/**),sha256(H)),after(100)))' --path bip84 --network signet --experimental` | exit 0; line 1 exactly R4's line 1 (K = `after(100)`); stderr contains neither `preimage` nor `BEARER` | base warning (says `preimage`) |
| R17 `m5_r4_keyed_keyless_card_never_says_cannot_spend` | mint `md encode 'wsh(or_d(pk(@0/<0;1>/*),1))' --network signet --path bip84 --key @0=KA --experimental`; then each of `md decode --json C...`, `md bytecode C...`, `md bytecode --json C...` | each exits 0; its stderr is exactly the case-Z decode line + `\n`; its stderr does not contain `cannot spend` | stderr is `TMPL` only (row fails); on the previous green of this doc stderr contains `cannot spend` (row fails) |
| R18 `i8_r12_refusal_hint_names_no_preimage` | `md encode 'wsh(or_i(pk(@0/**),after(100)))' --path bip84 --network signet` (no `--experimental`) | exit 1; stderr contains `a spend path that needs no key` and contains neither `whoever holds the preimage` nor `bearer` (coordinator ruling 2026-11-24: the I8-S5 hint itself says `a preimage or a timelock`, so the bare word `preimage` is allowed; the false claims were the old sentence and `bearer access`) | stderr contains `whoever holds the preimage` (row fails) |
| R19 `i8_r13_relative_time_lock_in_seconds` | `md encode 'wsh(or_d(pk(@0/**),older(4194305)))' --path bip84 --network signet --experimental` | line 1 contains ` older(4194305) is 512 seconds after each coin confirms.` | count of `512 seconds` is 0 (row fails) |
| C3 `i8_c1_encode_not_needed_note_unchanged` | `md encode 'wsh(and_v(v:pk(@0/**),after(100)))' --path bip84 --network signet --experimental` | exit 0; line 1 `note: --experimental was not needed for this descriptor. Every spend path here requires a signature, so the flag relaxed nothing and this card is not bearer access. Re-run without it to get the same plate under the unrelaxed rules.`; no `warning:` line (passes on the base) | same |
| C4 `i8_c2_decode_json_ordinary_card_has_no_warning` | `md decode --json KEYLESS` | stderr exactly `TMPL\n` (passes on the base; also C2) | same |
| U1 (unit, `keyless_advisory.rs`) `i8_u1_fallback_when_policy_cannot_be_lifted` | `render_from(None, Encode)` and `(None, Decode)` | exactly the two case-F texts of I8-S2 | n/a (new function) |
| U2 (unit) `i8_u2_fallback_when_every_path_needs_a_key` | `render_from(Some(&Policy::Key(KA/<0;1>/*)), Encode/Decode)` | exactly the case-F texts | n/a |

RED proof: R1-R3 and R4-R14 were run as commands on the base with the outputs in the last column (section 1 and the
author's report). U1-U2 test a new function and are RED by not compiling. GREEN proof: the scratch build printed every
"expected" text above exactly (author's report, `green_rows`).

Coverage map: M5-S1 -> R1, C2; M5-S2 -> R2, R3, C2; M5-S3 -> C1, R1 (stdout keys), C2. I8-S1 -> R4-R8, R12, U1, U2;
I8-S2 case N with `S_T` only -> R4, R5, R9, R10, R11; case N with `S_H` only -> R6, R12; case N with both -> R8;
case Z -> R7, R13; case F -> U1, U2. I8-S3 encode text/json -> R4, R9; decode text -> R10; decode `--json` (new, with
order) -> R11, R12, R13; unchanged "not needed" note -> C3; no warning on an ordinary card -> C4. I8-S4 -> R14. I8-S2 `S_T` only when a keyless path needs no secret (T) -> R15; `S_H`/`BEARER` only for a hash kept in `K` (not one behind a key) -> R16; lock piece for `older` in seconds -> R19; M5-S1 keyed card with a no-key path (warning, no class note) -> R17; I8-S5 -> R18.

MUTATION notes for the test file: Template unconditional in `decode.rs`/`bytecode.rs` -> R1-R3 fail; `S_H` printed
unconditionally -> R4 fails; `S_T` dropped -> R4, R8 fail; the JSON-branch call removed -> R11-R13 fail; predicate
asked in `Disposition::Warn` (the F-610 trap) -> R10-R13 fail. `S_T` keyed on a timelock in `K` -> R15 fails; `has_hash` on `P` -> R16 fails.

## 5. Manual (toolkit lockstep), CHANGELOG, GUI

Toolkit manual, mnemonic-toolkit master `4bfa645f` `docs/manual/src/40-cli-reference/42-md.md` (read only here; the
builder edits it in a toolkit PR that lands with this one):
- L44 (`md encode` flags, `--experimental` row): replace `Whoever learns the preimage of a keyless path can spend it
  alone, so if that preimage is engraved, the plate is bearer access. Prints a warning on every use` with `A keyless
  path opens on what it names instead of a key: a preimage (whoever learns it can spend that path, so an engraved
  preimage makes the plate bearer access) or a timelock alone (once it is met, anyone who has the policy and its public
  keys can spend that path). Prints a warning naming those conditions, or a note when the flag relaxed nothing`
  (PROPOSED; mirrors I8-S4).
- After the `md decode` paragraph that ends `even though \`md encode\` would no longer mint it.` (L126-127), add (PROPOSED):
  `**A card with a spend path that needs no key warns the restorer.** When the card's policy fails the rule that every
  spend path needs a signature (it was minted with \`--experimental\`), \`md decode\` prints on stderr, in text and
  \`--json\` output alike, \`warning: this card can be spent with NO key when: <condition>.\` (or, for a path with no
  condition at all, \`warning: this card can be spent with NO key, NO secret and NO timelock: ...\`) and what the
  condition means: a timelock alone needs no secret, and a named preimage makes an engraved plate bearer access. A timelock value is printed with what kind of lock it is; md cannot tell whether it has passed. On a card that
  carries public keys, \`md decode --json\` (and \`md bytecode\`) print \`note: stdout is watch-only — public keys
  only, cannot spend\`, except on a card that has a no-key spend path, where the warning replaces the note; the text form, which prints only the \`@N\` template, prints \`note: stdout is a keyless
  descriptor template (no keys)\`.`
- `md bytecode` (after L373 `shape — see the callout under \`md decode\` above.`), add (PROPOSED): `On a card that
  carries public keys the stderr note is \`note: stdout is watch-only — public keys only, cannot spend\`; on a keyless
  card it is \`note: stdout is a keyless descriptor template (no keys)\`.`
- No transcript changes: `transcripts/24-recover-md1.out`, `qs-26-recover-md1.out` and `42-md-repair-text.err` show
  keyless or text-decode output, unchanged. The builder runs the toolkit `manual` job (and doc-flag-lint) on the head.

CHANGELOG (`CHANGELOG.md`, `## md-cli [Unreleased]`, `### Fixed`), PROPOSED (architect confirms):
- `**\`md decode --json\` and \`md bytecode\` call a keyed card watch-only.** On a card that carries public keys they
  printed them (the JSON \`tlv.pubkeys\` entries; the payload hex) under \`note: stdout is a keyless descriptor
  template (no keys)\`. They now print \`note: stdout is watch-only — public keys only, cannot spend\`, as \`md inspect\`
  and \`md repair\` already did, except on a card that has a no-key spend path, where the warning replaces the note. A keyless card, and \`md decode\` text output (the \`@N\` template only), keep the
  template note (bh-md-M5).`
- `**The \`--experimental\` no-key warning names what opens the path.** \`md encode --experimental\` and \`md decode\`
  said whoever learns "its preimage" can spend, even on a path opened by a timelock alone, such as
  \`wsh(or_i(pk(@0/**),after(100)))\`, which has no preimage and which anyone with the policy and its public keys can
  spend once the lock is met. The warning now prints the no-key condition (\`after(100)\`), says when a path needs no
  secret, and keeps \`THE PLATE IS BEARER ACCESS\` only when a preimage is named. \`md decode --json\`, which printed no
  warning, now prints it on stderr, and \`md encode --help\` says the same. The hint a minting verb prints when it refuses such a path no longer says 'whoever holds the preimage' (bh-md-I8).`

GUI mirror: no flag change; the md schema (`gui_schema.rs`) carries no help text. None.

## 6. Tests that change

- `crates/md-cli/tests/cmd_decode.rs:230` `decoding_a_bearer_card_warns_the_restorer`: the assert at L261-264 requires `needs NO KEY`,
  which no longer prints. It changes to assert `can be spent with NO key when: and(sha256(2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881),after(100)).`
  and `BEARER ACCESS`, and also checks `!err.contains("needs no secret")` (its card is `and_v(v:sha256(..),after(100))`, so case N with `S_H` only; lift order checked by
  the author: `and(sha256(..),after(100))`). Its control at L290-293 (L291) changes `contains("NO KEY")` to
  `contains("NO key")` so it still guards the multisig card. Failed on the scratch build only there (panic at `cmd_decode.rs:261:5`), as expected.
- `crates/md-cli/tests/cmd_encode.rs:948` `experimental_does_not_claim_bearer_access_for_a_wallet_that_needs_no_relaxing`:
  its `!err.contains("needs NO key")` (L972-975, L973) would pass vacuously; it changes to `!err.contains("NO key")`.
  `experimental_admits_a_keyless_spend_path` (L880) is unchanged (still prints `--experimental relaxed` and
  `BEARER ACCESS`: its keyless leaf names a sha256). Both pass on the scratch build.
- `crates/md-cli/tests/cli_output_class.rs` `decode_json_emits_template_advisory`, `bytecode_text_emits_template`,
  `bytecode_json_emits_template`: unchanged (their fixture is keyless); R1-R3 and C2 are the keyed/keyless pair.
- No golden, snapshot or vector moves (`json_snapshots.rs` passed on the scratch build).

## 7. FOLLOWUPS

Closes `design/FOLLOWUPS.md:3538` `[md-nokey-warning-says-preimage]` (filed grade M; regraded I): the closing line is
direct-pushed after merge (listed in the author's report; no FOLLOWUPS edit in the PR). M5 has no FOLLOWUPS line.

## 8. Gate line

`bash scripts/fix-bh1119-md-output-advisories-gate.sh` (the builder commits it with its coordinator catalog row), on
the pattern of `scripts/fix-bh1119-md-card-readers-gate.sh`: step 1 every row of section 4 by exact nextest name
(match count 1 each; U1-U2 in the `md-cli` bin unit tests) and the two changed tests of section 6, then run; step 2
the repo CI set (`cargo nextest run --locked --workspace --all-targets --all-features --no-fail-fast`, `cargo test
--locked --workspace --doc --all-features`, `cargo clippy --locked --workspace --all-targets --all-features -- -D
warnings`, `cargo fmt --all --check`, `cargo build --locked -p md-cli --no-default-features`, `RUSTDOCFLAGS=-D warnings
cargo doc --locked --workspace --no-deps --document-private-items --all-features`, the grouping and shared-ci sha
checks, `ci/docs-only.test.sh`, `ci/code-docs-check.test.sh`, `ci/code-docs-check.sh`); step 3 no golden moved against
the merge-base. Every cargo call with `CARGO_BUILD_JOBS=6`. Shown RED once on the base (step 1 fails: R1-R19 assert,
U1-U2 do not compile). Not covered: bitcoind-differential, fuzz-smoke, FreeBSD and musl legs, the toolkit `manual`
job (run in the toolkit PR), the release.

## 9. Overlaps and non-goals

`decode.rs`, `encode.rs`, `main.rs` and `CHANGELOG.md` `## md-cli [Unreleased]` are shared with other md rows; the
later builder rebases. Not in scope (proposed FOLLOWUPS in the author's report): `md inspect`, `md descriptor
--experimental` and `md address --experimental` render a keyless-path card with no keyless warning.
