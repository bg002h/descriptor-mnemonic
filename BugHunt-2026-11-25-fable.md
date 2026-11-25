# Bug hunt 2026-11-25 (Fable): descriptor-mnemonic

Report-only review of `descriptor-mnemonic` at `c8cde23`, part of a constellation-wide hunt across all 11 bg002h repositories. Nothing in this repository was changed by the hunt; this file is the only addition.

**How it was done.** Eight Fable reviewers each audited an area (one of them the boundaries between repositories), reproducing what they could with locally built binaries or scratch tests that were not committed. Three further Fable verifiers then tried to disprove every High and Medium finding (and several Low ones) and re-graded them. Grades below are the verifiers' final grades where they ran.

**Grades.** Critical: can lose funds or leak a seed in ordinary use. High: wrong wallet, addresses or keys, or secret exposure, plausible in normal use, or recovery broken. Medium: needs an unusual but realistic input, or fails in a way a careful user might not notice. Low: fails closed, hardening, or a small exposure. Info: context, not a bug.

**Categories.** security (secret or key exposure, injection, integrity of what is trusted), funds (wrong address, amount, descriptor or recovery that could lose or strand coins), other.

**Status.** "Confirmed (reproduced)" means a command or test showed it. "Confirmed by reading" means the code path was traced but not executed. "Reviewer-confirmed, not re-verified" means the first reviewer reproduced or read it and no verifier re-checked it.

**Totals:** 5 Medium, 4 Low. No Critical findings.

## Summary

| ID | Grade | Category | Status | Finding |
|---|---|---|---|---|
| md#1 | Medium | funds | Confirmed (reproduced) | An elided (depth-0) origin is rendered as the master node, against the BIP's MUST |
| md#2 | Medium | funds | Confirmed (reproduced) | `md encode` accepts an xpub whose header contradicts the declared origin |
| md#3 | Medium | funds | Confirmed (reproduced) | A placeholder with no wildcard is silently minted as a ranged `/*` key |
| md#4 | Medium | funds | Confirmed (reproduced) | `c:` over a non-key child mints a card md cannot turn back into a descriptor |
| X4 | Medium | funds | Confirmed (reproduced) | A WIF slot's zero-chain-code "xpub" on a card is not refused by md (toolkit mints it) |
| md#5 | Low | funds | Reviewer-confirmed, not re-verified | A hardened wildcard (`/*h`) mints at exit 0 and can never derive an address |
| md#6 | Low | security | Reviewer-confirmed, not re-verified | `--fingerprint` minted and shown with no privacy warning, against the BIP's MUST |
| X2 | Low | other | Confirmed (reproduced) | "md-codec 0.48.7" names two different codecs (fix added under the released heading, no bump) |
| X7 | Low | other | Reviewer-confirmed, not re-verified | Older md-codec pins can mint a card the decoder refuses (out-of-order template numbering) |

IDs: `md#`, `mk#`, `ms#`, `toolkit#`, `G`, `refugium#`, `wallet#`, `E`, `T`, `SH#`, `M` are per-repository findings; `X` findings are at a boundary between repositories and appear in every report they touch.

## Findings

## md#1: An elided (depth-0) origin is rendered as the master node, against the BIP's MUST

**Grade:** Medium · **Category:** funds · **Status:** Confirmed (reproduced)

**Reviewer write-up** (source: `md`)

### 1. An elided (depth-0) origin is rendered as the master node

**Severity: High. CONFIRMED.**

**What goes wrong.** When the operator gives no origin path for a canonical wrapper (`wpkh`, `pkh`, `sh(wpkh)`, key-path `tr`, `wsh(multi/sortedmulti)`, `sh(wsh(multi/sortedmulti))`), `md encode` writes a depth-0 origin. The BIP says that is an *elided* origin and that reading it as the master node is wrong:

> "A depth-0 origin does not mean 'derive at the master node m' … A decoder MUST resolve a depth-0 origin, on any wrapper shape listed in this table, through the corresponding canonical origin before deriving keys. Deriving at the literal master node instead would recover the wrong wallet." (`bip/bip-mnemonic-descriptor.mediawiki:181-207`, again at `:449`)

The reference implementation resolves it only when hashing the wallet-policy id. Everywhere else it treats it as the master node:

- `md descriptor` emits `[fingerprint]xpub661My…` — an origin with no path and a depth-0 xpub header, which states that the key is the master key.
- `md inspect` prints `@0: [aabbccdd/m]`.
- `md decode` prints `@0: m` and advises `--path m`.
- Seating (`--from-mk1`) treats the slot's declared origin as empty (`@0 []`), so it fails closed rather than seating wrongly.

The result is that the default keyed mint (no `--path`) restores to a descriptor whose key origins are wrong. Addresses are still right, because origins do not enter address derivation. But a signer given `[fp]xpub661…` cannot match the key to its seed, so the restored wallet watches and cannot sign until someone works out the path by hand.

The two representations also disagree with each other: an elided card and the explicit-origin card for the same wallet have the **same** wallet-policy id but produce **different** descriptors.

**Evidence (code).**

- `crates/md-codec/src/canonicalize.rs:527-548` — `expand_per_at_n` returns the empty path as-is when the wrapper is canonical.
- `crates/md-codec/src/to_miniscript.rs:220-242` — `assemble_origin_and_xkey` builds origin, `depth` and `child_number` from that empty path.
- `crates/md-codec/src/identity.rs:236-241` — the only place the canonical fill is applied.
- `crates/md-cli/src/cmd/decode.rs:129-160`, `crates/md-cli/src/cmd/inspect.rs:122` — the displays.

**Reproduction.**

```
$ md encode "wpkh(@0/<0;1>/*)" --key @0=$X0 --fingerprint @0=aabbccdd      # exit 0, no warning
md1ffnw2qspqqgqpsq95z4thnxaz4q3waqcl9pvz58pmltjs9tjrg0g2z0agd4urfpzanhaqss2gv2atqyddp4
md1ffnw2qs0lphut2hvvpp5wl4l0mn058ndxfl63kufyfsjwlt2vkk2nlqmlvch5n4shwf72gve4z667e0g9xp

$ md inspect <those two>
origins:
  @0: [aabbccdd/m]
wallet-policy-id: a3d38b48d8aca89d8fe6eb1d2ed6e431

$ md descriptor <those two>
wpkh([aabbccdd]xpub661MyMwAqRbcGQnC8zMGwRc4EXYJCgXrxx9kXtw1RXqu4TcW26PxHqssgp6sU4N5CR5o9QcUZPG31fzPeUHEVPkFit1WpVZTmqZLzpvZG2s/<0;1>/*)#xc98st9h
```

Same wallet, origin written explicitly (`--path bip84`):

```
$ md inspect <card>
  @0: [aabbccdd/84'/0'/0']
wallet-policy-id: a3d38b48d8aca89d8fe6eb1d2ed6e431        # identical id
$ md descriptor <card>
wpkh([aabbccdd/84'/0'/0']xpub6BemYiVNp19a1zxN9QsTb21SjdjQVxGn3JpqaxSgqbY2f77oaK97QJtSpr2GhQQv5ArCXJnnBnnA9iaCVwpw8Fy2KXusdivQ9fR73CTbuwt/<0;1>/*)#4x2mjzfy
```

Multisig, and a mix of elided and explicit slots, behave the same way:

```
$ md encode "wsh(sortedmulti(2,@0/<0;1>/*,@1/48'/0'/1'/2'/<0;1>/*))" --key @0=$X0 --key @1=$X1 \
     --fingerprint @0=aabbccdd --fingerprint @1=11223344
$ md descriptor <cards>
wsh(sortedmulti(2,[aabbccdd]xpub661MyMw…/<0;1>/*,[11223344/48'/0'/1'/2']xpub6DXuQW1…/<0;1>/*))
```

Keyless card: `md decode md1yqqqq2wy43c0uqcl29` prints `@0: m` and "`md address --template <the line above> --key @i=<xpub> --path m`".

**Fix direction.** Resolve an empty origin through `canonical_origin` in one shared place (for example in `expand_per_at_n`, or a resolved-origin accessor used by `to_miniscript`, the CLI displays and seating), so that rendering agrees with what the policy id already hashes. Alternatively stop eliding at mint for keyed cards.

**Cross-repo.** Any consumer that calls `expand_per_at_n` or `to_miniscript_descriptor*` on an elided-origin card inherits the master-node reading: mnemonic-toolkit (bundle, restore, verify-bundle), mnemonic-refugium, refugium-wallet, mnemonic-engrave, and the Go port in the seedhammer fork. A restore-from-seed path that derives at the card's origin should be checked first, because there the wrong origin would produce wrong keys and not only wrong metadata.

**Verifier re-check** (source: `verify-1`)

### md#1: elided depth-0 origin rendered as the master node

**CONFIRMED-but-downgraded. Medium. funds.**

Reproduced exactly: `md encode "wpkh(@0/<0;1>/*)" --key @0=$X0 --fingerprint @0=aabbccdd` exits 0 with no warning; `md inspect` prints `@0: [aabbccdd/m]`; `md descriptor` prints `wpkh([aabbccdd]xpub661MyMw…/<0;1>/*)#xc98st9h`; `md decode` advises `--path m`. The BIP text is as quoted (a decoder MUST resolve a depth-0 origin through the canonical table), and `md encode --help` itself calls the origin "inferred". The only place the canonical fill is applied is `crates/md-codec/src/identity.rs:236-241`; `to_miniscript.rs:220-242` builds origin, depth and child number from the empty path. Not recorded in `design/FOLLOWUPS.md`.

Why not High: no address is wrong. `md address` on the elided card and on the `--path bip84` card for the same key both give `bc1qtncy45g8qy8laah6jvxn6ppvj0s9rluxj5uxf6`. What is wrong is the key-origin metadata and the re-stamped xpub header in the restored descriptor, so a signer cannot match the key until someone supplies the (standard) path by hand. That is impaired recovery of signing, not a wrong wallet. It is the default keyed mint for the most common shapes, which keeps it at the top of Medium.

**Verifier re-check** (source: `verify-1`)

### Are md#1, toolkit#1 and toolkit#2 the same root cause?

They are one design gap with three separate code defects, not one bug. The gap: nobody has fixed what an empty origin means on a *keyed* card with a canonical wrapper. Three readings are live at once:

- The md BIP says it is elided and MUST be resolved to the canonical origin (`bip/bip-mnemonic-descriptor.mediawiki:181-207`, `:449`).
- md-codec's renderer and the toolkit's descriptor-mode mint both read it as the master node (md#1, toolkit#1). These two agree with each other.
- The toolkit's plain-multisig restore reads it as the template default `m/87'/0'/0'` (toolkit#2), which is neither of the above (md's canonical origin for `wsh(sortedmulti)` is `m/48'/0'/0'/2'`).

They must be fixed together. If md#1 alone is fixed by resolving to the canonical origin, every card minted by toolkit#1 (which really does hold the master key) would start rendering as `[fp/84'/0'/0']<master key>`, which is toolkit#2's defect in a new place.

## md#2: `md encode` accepts an xpub whose header contradicts the declared origin

**Grade:** Medium · **Category:** funds · **Status:** Confirmed (reproduced)

**Reviewer write-up** (source: `md`)

### 2. `md encode` accepts an xpub whose header contradicts the declared origin

**Severity: Medium. CONFIRMED.**

**What goes wrong.** `md encode --key` checks only that the xpub's depth is 3 or 4. It never compares the xpub's depth or child number with the origin the card will declare for that slot. A depth-4 key with child number `2'` can be minted under `m/84'/0'/0'`, or under `m/48'/0'/0'/1'`, at exit 0 with no warning.

On restore, `md descriptor` then rebuilds the xpub header (depth and child number) from the declared origin. The output is a self-consistent-looking xpub that is not the one the operator supplied, sitting under an origin the key was provably not derived at. A signer deriving at that origin gets a different key and cannot sign.

`md decompose` refuses the same contradiction ("key @0 is depth-inconsistent IN THE INPUT: the extended key states depth 4, but its origin path … has 3 component(s)"), so the two entry points disagree. mnemonic-key's hazard list calls this check out for mk-codec on both depth and child number.

**Evidence (code).** `crates/md-cli/src/parse/keys.rs:107-146` — the comment states that the invariant `depth == origin_path.len()` "is NOT reachable here" and was left out as a stopgap. `crates/md-codec/src/to_miniscript.rs:230-239` re-stamps `depth` and `child_number`.

**Reproduction.**

```
$ md encode "wpkh(@0/84'/0'/0'/<0;1>/*)" --key @0=$X0 --fingerprint @0=aabbccdd
chunk-set-id: 0xdddc2
md1fmhwzpqpqpm6jzzqqvqpdq42auehg4gythgx8egtq4pcwl6u5pg79kgdrygvzzjz
…                                                           [exit 0, no warning]

$ md encode "wsh(sortedmulti(2,@0/48'/0'/0'/1'/<0;1>/*,@1/48'/0'/7'/2'/<0;1>/*))" \
     --key @0=$X0 --key @1=$X1 --fingerprint @0=aabbccdd --fingerprint @1=11223344
chunk-set-id: 0xed0af                                        [exit 0, no warning]
```

`X0` is depth 4, child `0x80000002`; the first command declares a depth-3 origin, the second a last step of `1'`.

**Fix direction.** Once the per-slot origin is resolved in `encode` (and in `build_descriptor`), refuse or at least warn when the xpub's depth differs from the origin's length or its child number differs from the origin's last step. Finding 1 should be fixed first, or the check will compare against an empty path.

**Cross-repo.** mnemonic-key refuses these records at `mk encode`, so a keyed md1 card minted this way has no matching mk1 card.

**Verifier re-check** (source: `verify-1`)

### md#2: xpub header contradicting the declared origin is accepted

**CONFIRMED. Medium. funds.**

`md encode "wpkh(@0/84'/0'/0'/<0;1>/*)" --key @0=$X0 --fingerprint @0=aabbccdd` (X0 is depth 4, child `2'`) exits 0 with only the chunk-set-id and watch-only note. `md descriptor` on the result prints `wpkh([aabbccdd/84'/0'/0']xpub6BemYiVNp19a1…)`, a different xpub string from the one supplied, with the header rebuilt to depth 3. `md decompose` on the same key and origin refuses ("key @0 is depth-inconsistent IN THE INPUT … depth 4 … 3 component(s)"), so the two entry points disagree. Addresses are unaffected; the origin is provably wrong for the key. Needs an operator error (wrong path for the key), which is realistic for BIP-48 keys.

## md#3: A placeholder with no wildcard is silently minted as a ranged `/*` key

**Grade:** Medium · **Category:** funds · **Status:** Confirmed (reproduced)

**Reviewer write-up** (source: `md`)

### 3. A placeholder with no wildcard is silently minted as a ranged key

**Severity: Medium. CONFIRMED.**

**What goes wrong.** In the template lexer the wildcard group is optional. `@0` and `@0/<0;1>` are accepted and stored exactly as `@0/*` and `@0/<0;1>/*`. In descriptor terms those are different wallets: `wpkh(KEY)` is one fixed key with one address, and `wpkh(KEY/<0;1>)` is two fixed keys. md mints a ranged wallet, derives child-index addresses, and `md verify` against the original spelling says `OK`.

The tool's own words say this must not happen. `md decompose` refuses the same shapes on a concrete descriptor: "key @0 has no use-site, which an md1 card cannot hold … so a template, and any card minted from it, would describe a different wallet". `crates/md-codec/src/use_site_path.rs:13-24` says consumers "must REFUSE" them and never widen.

`md compile` emits bare `@N` (for example `tr(@1,pk(@0))`), so bare `@N` may be an intended shorthand for `/*`. If so, it is undocumented at the point of use, and it yields a single-chain wallet with no change branch, with no note. `@0/<0;1>` without the wildcard has no such excuse.

**Evidence (code).** `crates/md-cli/src/parse/template.rs:183` (the regex; group 4 is optional), `:345-355` and `:951-968` (`make_use_site_path` records only alternatives and wildcard hardening; a missing wildcard is not represented).

**Reproduction.**

```
$ md encode "wpkh(@0)"            -> md1yqqqq2wy43c0uqcl29     [exit 0]
$ md encode "wpkh(@0/*)"          -> md1yqqqq2wy43c0uqcl29     (same card)
$ md encode "wpkh(@0/<0;1>)"      -> md1yqpqqxqq8xtwhw4xwn4qh  [exit 0]
$ md encode "wpkh(@0/<0;1>/*)"    -> md1yqpqqxqq8xtwhw4xwn4qh  (same card)

$ md address --template "wpkh(@0)" --key @0=$X0
bc1q8qydrp3penwz2aaurncw6qfa85ef7ywn047wqw                    # the address of X0/0, not of X0
$ md descriptor --template "wpkh(@0)" --key @0=$X0
wpkh(xpub661MyMw…/*)#fn4fyxff

$ md decompose "wsh(multi(2,[aabbccdd/48'/0'/0'/2']$X0/<0;1>,[11223344/48'/0'/0'/2']$X1/<0;1>))"
md: decompose: this descriptor has no md1 template: key @0 has use-site `/<0;1>`, which an md1 card cannot hold …   [exit 1]
```

**Fix direction.** Refuse `@N/<…>` with no trailing `/*` in `lex_placeholders`, with the same message `decompose` gives. For bare `@N`, either refuse or print a note saying it was read as `/*` (single chain, no change branch), and document it beside `md compile`.

**Verifier re-check** (source: `verify-1`)

### md#3: placeholder without a wildcard minted as a ranged key

**CONFIRMED. Medium. funds.**

`wpkh(@0)` and `wpkh(@0/*)` both encode to `md1yqqqq2wy43c0uqcl29`; `wpkh(@0/<0;1>)` and `wpkh(@0/<0;1>/*)` both encode to `md1yqpqqxqq8xtwhw4xwn4qh`. `md verify <card> --template "wpkh(@0/<0;1>)"` prints `OK`. `md address --template "wpkh(@0)" --key @0=$X0` prints `bc1q8qydrp3…`, the address of `X0/0`, not of `X0`. The regex's wildcard group is optional at `crates/md-cli/src/parse/template.rs:183`. The toolkit manual calls exactly this `@0/<0;1>` case a silent wrong-wallet encoding and now refuses it (`41-mnemonic.md:322-329`); md-cli still accepts it. A card restored from such a template derives a different address space than the non-ranged wallet the text describes. The input is unusual (non-ranged wallets are rare), hence Medium.

## md#4: `c:` over a non-key child mints a card md cannot turn back into a descriptor

**Grade:** Medium · **Category:** funds · **Status:** Confirmed (reproduced)

**Reviewer write-up** (source: `md`)

### 4. Valid miniscript with `c:` over a non-key child mints a card md cannot restore

**Severity: Medium. CONFIRMED.** The decode half is recorded as deferred work ("Deferred A2", `design/FOLLOWUPS.md:82-84`; related `md-encode-typecheck-gate`). The mint half is not recorded: those entries describe a rendering gap and hand-built trees, not that the CLI mints these cards from valid input at exit 0.

**What goes wrong.** The walker writes both `pk_k(K)` (type K) and `c:pk_k(K)` (type B) as a bare `Tag::PkK`. The decoder always turns a bare `Tag::PkK` back into `c:pk_k`. So any tree with `Tag::Check` over a non-key child — `c:or_i(pk_k,pk_k)`, `c:and_v(v:pk,pk_k)`, `c:or_i(pk_k,pk_h)`, `c:andor(…)` — decodes to `c:` over a type-B fragment and fails the type check. Compilers do emit these forms.

`md encode` accepts the valid template and prints a card. `md address` and `md descriptor` on that card fail. `md decode` prints a template (`c:or_i(pk(…),pk(…))`) that is not the input and that no miniscript parser accepts. `md verify` against the original template still says `OK`, so the usual post-mint check does not catch it. The BIP says `Tag::Check` "remains valid on the wire only when wrapping a non-key child" (`:828`), which is the one case the decoder cannot convert.

It fails closed (an error, never a wrong address), which is why this is Medium and not higher. But it is an engrave-and-cannot-restore card.

**Evidence (code).** `crates/md-cli/src/parse/template.rs:1585-1590` and `:1633-1654` (walker), `crates/md-codec/src/to_miniscript.rs:753-788` (decoder), `crates/md-codec/src/encode.rs:283-290` (the mint-time self-decode does not type-check).

**Reproduction.**

```
$ md encode "wsh(c:or_i(pk_k(@0/<0;1>/*),pk_k(@1/<0;1>/*)))" --path bip48
md1ypfdsssj5qqcyvvjs495khr3xmjp6j6                             [exit 0]

$ md address --template "wsh(c:or_i(pk_k(@0/<0;1>/*),pk_k(@1/<0;1>/*)))" --key @0=$X0 --key @1=$X1 --path bip48
md: codec error: address derivation failed: typecheck: fragment «c:or_i(pk(xpub…/0/*),pk(xpub…/0/*))» cannot wrap a fragment of type B   [exit 1]

$ md decode md1ypfdsssj5qqcyvvjs495khr3xmjp6j6
wsh(c:or_i(pk(@0/<0;1>/*),pk(@1/<0;1>/*)))                     [exit 0]
```

`wsh(c:and_v(v:pk(@0/<0;1>/*),pk_k(@1/<0;1>/*)))` and `wsh(c:or_i(pk_k(@0/<0;1>/*),pk_h(@1/<0;1>/*)))` behave the same.

**Fix direction.** Either implement the type-directed conversion already sketched as A2 (render keys as type K under a `Check` parent, through `or_i`, the right arm of `and_v`, and arms 2 and 3 of `andor`), or refuse at mint any template that would put `Tag::Check` on the wire, by running `to_miniscript` in the mint-time self-check.

**Cross-repo.** The Go port in the seedhammer fork and the toolkit walker share the wire shape; check both once Rust is fixed.

**Verifier re-check** (source: `verify-1`)

### md#4: `c:` over a non-key child mints a card md cannot restore

**CONFIRMED. Medium. funds.**

`md encode "wsh(c:or_i(pk_k(@0/<0;1>/*),pk_k(@1/<0;1>/*)))" --path bip48` exits 0 and prints `md1ypfdsssj5qqcyvvjs495khr3xmjp6j6`; `md verify` against the same template prints `OK`; the keyed mint also exits 0. `md address` and `md descriptor` on the keyed card fail with `typecheck: fragment «c:or_i(pk(…),pk(…))» cannot wrap a fragment of type B`. Cause is at `crates/md-codec/src/to_miniscript.rs:753-788`. The decode half is a recorded deferral ("Deferred A2" in `design/FOLLOWUPS.md`, under `to-miniscript-check-pkh-double-wrap`); the fact that the CLI mints these at exit 0 and verifies them `OK` is not recorded. It fails closed and the wire still holds the full tree, so a fixed decoder could read the card; until then the card yields no address. Medium because the failure shows up only at restore time.

## X4: A WIF slot's zero-chain-code "xpub" on a card is not refused by md (toolkit mints it)

**Grade:** Medium · **Category:** funds · **Status:** Confirmed (reproduced)

**Reviewer write-up** (source: `cross`)

### X4. Toolkit WIF slots write a "fixed key" (depth 0, all-zero chain code) into mk1 and md1; only unreleased mk-cli knows not to derive from it

- **Severity:** Medium (addresses no wallet watches), needs confirmation by the md and toolkit reviewers
- **Status:** SUSPECTED (reading only)
- **Repos:** mnemonic-toolkit, mnemonic-key, descriptor-mnemonic, mnemonic-gui (pinned mk-cli), mnemonic-refugium, seedhammer
- **Evidence:** `mnemonic-toolkit/crates/mnemonic-toolkit/src/cmd/bundle.rs:785-822` builds an `Xpub` with
  `depth: 0`, `chain_code: [0; 32]`, fingerprint `00000000` from a `--slot @N.wif=` and emits it in the bundle's
  cards. mk-cli HEAD (Unreleased, bh-mk-C1, `mnemonic-key/crates/mk-cli/CHANGELOG.md` "`mk address` and `mk derive`
  refuse a fixed-key card") refuses to derive children from such a card, because "it has no child keys any wallet
  watches". No other reader has that rule: `grep -rni "zero chain code\|all-zero chain\|fixed-key"` over
  descriptor-mnemonic, mnemonic-refugium, mnemonic-engrave and seedhammer (`mk/`, `gui/`, `address/`) finds nothing.
  The GUI and the installer pin `mk-cli-v0.17.0` (`mnemonic-gui/pinned-upstream.toml:70`), which is before the fix.
- **Impact:** `mk address` 0.17.0 (the pinned release), and any md-side or device-side address derivation over an
  md1 that carries such a key under a `/<0;1>/*` use-site, prints child addresses of a key that is not an HD key.
  Coins sent there are spendable only by software that treats the WIF key as an xprv with a zero chain code.
- **Fix direction:** put the fixed-key predicate in md-codec (derive and render paths) and port it to the Go
  `md`/`mk` packages; release mk-cli 0.18.0 and move the GUI and installer pins.

**Verifier re-check** (source: `verify-1`)

### cross X4: WIF slot writes a depth-0, zero-chain-code xpub onto cards

**CONFIRMED. Medium. funds.**

`mnemonic bundle --network mainnet --template bip84 --slot @0.wif=KwDiBf89…` exits 0 and emits an mk1 with `origin_path: m/` and an md1 `wpkh(@0/<0;1>/*)`. The toolkit's own `restore --md1` prints `first recv: bc1qgl57xjd595rrxmpwys85q40xh4nmqunqfacg4j`, and `md address` prints the same plus `bc1qcw7uxw07…`. Those are children of the WIF public key under an all-zero chain code, not the WIF key's own address. `mk address` at HEAD refuses the card ("holds one fixed key (depth 0, all-zero chain code; the toolkit writes a WIF key this way) … mk derives nothing from it"); md, the toolkit and the GUI's pinned `mk-cli-v0.17.0` (`mnemonic-gui/pinned-upstream.toml:70`) have no such rule. The toolkit's own comment says the key is "non-derivable" (`cmd/bundle.rs:786-789`) while the template gives it a ranged use-site. Coins sent to the printed address are invisible to any wallet that imports the WIF normally, and are spendable only by treating the WIF as an xprv with a zero chain code. Unusual input, recoverable with custom tooling, hence Medium. I did not run the Go device or mnemonic-refugium against this card.

## md#5: A hardened wildcard (`/*h`) mints at exit 0 and can never derive an address

**Grade:** Low · **Category:** funds · **Status:** Reviewer-confirmed, not re-verified

**Reviewer write-up** (source: `md`)

### 5. A keyed card with a hardened wildcard mints and can never derive an address

**Severity: Low. CONFIRMED.**

**What goes wrong.** The lexer refuses a hardened multipath alternative with "hardened derivation is impossible on a watch-only (xpub) card", but accepts a hardened wildcard. `md encode "wpkh(@0/<0;1>/*h)" --key @0=XPUB` exits 0 with no warning. The card verifies `OK`, `md descriptor` prints `wpkh(xpub…/<0;1>/*h)`, and `md address` fails with `HardenedPublicDerivation`. Fails closed.

**Evidence (code).** `crates/md-cli/src/parse/template.rs:297-309` (alternatives refused) against `:345-348` (wildcard accepted).

**Reproduction.**

```
$ md encode "wpkh(@0/<0;1>/*h)" --key @0=$X0 --path bip84       [exit 0; stderr: chunk-set-id and the watch-only note only]
$ md address <cards>
md: codec error: hardened public-key derivation: use-site path requires hardened component, which BIP 32 forbids on xpub-only restore   [exit 1]
```

**Fix direction.** Apply the same refusal to a hardened wildcard when minting a keyed card, or warn that the card cannot derive addresses from its xpubs.

## md#6: `--fingerprint` minted and shown with no privacy warning, against the BIP's MUST

**Grade:** Low · **Category:** security · **Status:** Reviewer-confirmed, not re-verified

**Reviewer write-up** (source: `md`)

### 6. Fingerprints are minted and shown with no privacy warning

**Severity: Low. CONFIRMED.**

**What goes wrong.** The BIP says: "Recovery tools MUST warn the user before including fingerprints … Decoders that surface parsed fingerprints to the user MUST flag the data as privacy-sensitive" (`bip/bip-mnemonic-descriptor.mediawiki:665`). `md encode --fingerprint` prints no such warning (stderr holds only the chunk-set-id and the watch-only note; see the first command in finding 1). `md inspect` and `md decode` print fingerprints with no flag. A separate advisory actively recommends adding them ("Pass one --fingerprint @N=HEX per slot to make them distinguishable").

**Fix direction.** Either print the warning and the flag, or change the BIP text to match the decision that fingerprints are wanted for seating.

## X2: "md-codec 0.48.7" names two different codecs (fix added under the released heading, no bump)

**Grade:** Low · **Category:** other · **Status:** Confirmed (reproduced)

**Reviewer write-up** (source: `cross`)

### X2. "md-codec 0.48.7" names two different codecs: the release commit lacks a fix the CHANGELOG lists under 0.48.7

- **Severity:** Medium
- **Status:** CONFIRMED (source diff)
- **Repos:** descriptor-mnemonic, mnemonic-toolkit (pins it), and every consumer that reads the md CHANGELOG to
  decide what a pin holds (refugium, engrave, refugium-wallet, gui)
- **Evidence:**
  - Release commit `e11b4ce` "release: md-cli 0.21.1 + md-codec 0.48.7". The toolkit pins exactly that
    (`mnemonic-toolkit/Cargo.lock`, commit `8ed773b` "pin e11b4ce1 (= md-codec-v0.48.7)").
  - `4b763d7` (same day, after the release in history: `git log e11b4ce..HEAD -- crates/md-codec/src`) adds
    `check_chain_in_range` to `to_miniscript_descriptor_with_network` and puts its CHANGELOG bullet inside the
    already released `## md-codec [0.48.7]` section (`descriptor-mnemonic/CHANGELOG.md:86-89`). The crate version
    stayed `0.48.7` (`crates/md-codec/Cargo.toml:3`).
  - `diff mnemonic-toolkit/vendor/md-codec/src/to_miniscript.rs descriptor-mnemonic/crates/md-codec/src/to_miniscript.rs`:
    the vendored 0.48.7 has no `check_chain_in_range`; HEAD's 0.48.7 has it (lines 98, 123-143). `git show
    e11b4ce:CHANGELOG.md` has one bullet under 0.48.7; HEAD has two.
- **Impact:** with any pinned md-codec in the suite (0.47.0, 0.48.4, 0.48.5, and the tagged 0.48.7),
  `to_miniscript_descriptor(d, chain)` for a descriptor where no key has a multipath ignores `chain` and returns the
  chain-0 descriptor for every chain; a caller asking for "change" gets the receive descriptor at `Ok`. A reader of
  the CHANGELOG concludes the toolkit's 0.48.7 is safe; it is not. Current callers: the toolkit calls chain 0 only
  (`synthesize.rs:1004`, `template.rs:598,699`); mr-codec calls chains 0 and 1 but first overwrites the use-site
  with a two-alternative multipath (`mnemonic-refugium/crates/mr-codec/src/render.rs:171,182-187`), so no shipped
  caller was found that hits it today. The released md-cli 0.21.1 CHANGELOG also lists `md descriptor --change`
  refusing such a wallet; check whether tag `md-cli-v0.21.1` holds `4b763d7` (the CLI fix needs this codec change).
- **Reproduction:** the `diff` command above; `grep -c check_chain_in_range mnemonic-toolkit/vendor/md-codec/src/to_miniscript.rs` gives 0.
- **Fix direction:** release the change as 0.48.8 and move its CHANGELOG bullet; add a CI check that a released
  section's text does not change after its release commit.

**Verifier re-check** (source: `verify-3`)

### X2 — "md-codec 0.48.7" names two different codecs

- Verdict: **CONFIRMED-but-downgraded**. Grade: **Low**. Category: **other** (release hygiene).
- `e11b4ce` "release: md-cli 0.21.1 + md-codec 0.48.7" (2026-11-22 11:40) is an ancestor of `4b763d7`
  (12:52, `git merge-base --is-ancestor e11b4ce 4b763d7`). `4b763d7` changes `crates/md-codec/src/{derive,error,
  to_miniscript}.rs` and adds bullets under the already released headings: `CHANGELOG.md:86` (md-codec [0.48.7]) and
  `:42` (md-cli [0.21.1]); `git show e11b4ce:CHANGELOG.md` has one bullet under 0.48.7. `crates/md-codec/Cargo.toml`
  is `0.48.7` at both commits. The toolkit pins `e11b4ce` (`mnemonic-toolkit/Cargo.lock:686-688`);
  `grep -c check_chain_in_range mnemonic-toolkit/vendor/md-codec/src/to_miniscript.rs` = 0, HEAD = 2.
- Why Low: no shipped caller hits the missing check. The toolkit calls `to_miniscript_descriptor(_, 0)` only
  (`synthesize.rs:1004`, `template.rs:598,699`). The harm is a CHANGELOG that says a pinned 0.48.7 has a fix it lacks
  (md-cli 0.21.1 likewise). Fix: release as 0.48.8 / 0.21.2 and move the bullets.

## X7: Older md-codec pins can mint a card the decoder refuses (out-of-order template numbering)

**Grade:** Low · **Category:** other · **Status:** Reviewer-confirmed, not re-verified

**Reviewer write-up** (source: `cross`)

### X7. md-codec pins behind 0.48.6/0.48.7: a template numbered out of first-occurrence order can still mint a card the decoder refuses

- **Severity:** Low
- **Status:** SUSPECTED (0.48.5 read from `mnemonic-refugium/vendor/md-codec/src/canonicalize.rs:217-251`; not executed)
- **Repos:** mnemonic-refugium and mnemonic-engrave (0.48.5), refugium-wallet (0.48.4), seedhammer Go port
  (`md/bits.go:6`, tracks 0.47.0), descriptor-mnemonic
- **Evidence:** in 0.48.5 `canonicalize_placeholder_indices` remaps use-site overrides but keeps the old `@0`
  baseline, so `wsh(multi(2,@1/<2;3>/*,@0/<0;1>/*))` mints a card with an override keyed on `@0`, which decode
  refuses (0.48.6 CHANGELOG, bughunt-1119 C2). 0.48.7 makes `encode_payload` decode its own output. The failure
  is fail-closed at decode, so the risk is an engraved card that cannot be read back, not a wrong wallet.
  mr-codec mints through `encode_payload_unadmitted` and its own templates (`render.rs`), which set one use-site
  for all keys, so `mr` is unlikely to reach it; `me` does not mint md1. The Go port's `md/canonicalize.go`
  (364 lines) should be checked against the 0.48.6 rule under the Rust-primary rule.
- **Fix direction:** move the pins to the next md-codec release (see X2); port the 0.48.6 vector to Go.

---

_Generated by [Claude Code](https://claude.ai/code) on 2026-11-25. Findings were not fixed; each fix needs its own review under this repository's gates._
