# REVIEW — SPEC_mdcli_mini.md, R0 round 1 (independent adversarial architect)

| field | value |
| --- | --- |
| artifact | `design/SPEC_mdcli_mini.md` |
| commit | `889720c4` (main tip at review time) |
| date | 2026-08-31 |
| reviewer | independent architect agent (opus tier), no authorship of the artifact |
| repo | `/scratch/code/shibboleth/descriptor-mnemonic` |

**The one question asked:** if this spec were implemented exactly as written,
would it produce wrong results, unmet guarantees, unsound admission semantics,
or unexecutable/vacuous vector rows?

**Lenses run**

1. **BIP-388 fidelity** — verified against the authoritative text
   (`https://raw.githubusercontent.com/bitcoin/bips/master/bip-0388.mediawiki`,
   fetched 2026-08-31, version 1.1.0, 351 lines). Quotes checked against source
   lines, not against the spec.
2. **Vector-row executability** — every named row asked "can it run, and can it
   fail if the implementation is wrong?"
3. **N1's single-source invariant** — traced against the real call graph of
   every `md` subcommand.
4. **N2's byte-oracle soundness** — traced against what `encode_payload` writes
   and what `seat::compose` produces.
5. **N3 precedence** — traced against
   `resolve_keys_fingerprints_and_precedence`.
6. **R9 clap semantics** — reasoned from clap 4 multi-value-option behaviour
   against the real arg definitions.
7. **Falsified-elsewhere sweep** — what the spec's own text makes false in the
   shipped tree.

**Machine-checked by this reviewer before writing (so no finding below rests on
a described fact):**

- `cargo nextest run --locked --all-features --no-fail-fast` → **1106 tests
  run: 1105 passed, 1 failed, 2 skipped**; the single failure is
  `md-cli::bin/md compile::tests::upstream_display_is_still_broken_delete_local_renderer_when_this_fails`.
  R5's premise ("one failing test") is CONFIRMED, and its (a)-then-(b)
  sequencing is sound. (Note for future measurers: a plain `--all-features` run
  fail-fasts at 426/1106 and proves nothing; `--no-fail-fast` is required.)
- The full `md` subcommand set, read from the `Cmd` enum in
  `crates/md-cli/src/main.rs`: `encode`, `decode`, `verify`, `inspect`,
  `bytecode`, `vectors`, `compile`, `descriptor`, `address`, `decompose`,
  `gui-schema`, `repair`, `gen-man`.
- Live measurements on `target/debug/md` at this tree (all three at **exit 0**):
  - `md descriptor --template "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" --key @0=<K0> --path "48'/0'/0'/2'"` → composes a checksummed descriptor.
  - `md address` with the same arguments `--count 1` → prints `bc1ql5j095gqvdv6ugccf956pduc2e0vevtfnf9r72nhmln9lf8tlmmsd9ujlz`.
  - `md encode` with the same arguments → mints `chunk-set-id: 0xed813`, two md1 chunks.
  - `md decode` and `md inspect` on those two chunks → both exit 0 and print the
    forbidden template.
  - `md descriptor --template "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" --key @0=<K0> --key @1=<K0> --path "48'/0'/0'/2'"` → composes at exit 0.

**BIP-388 fidelity result, stated up front because it is a clean pass.** The
three taxonomy rows' BIP claims are all real and correctly attributed *on the
multipath-set axis*:

- Row 1's quote "Repeated keys with the same path expression" is verbatim
  bip-0388.mediawiki **line 308**, in the invalid-example list.
- Row 2's disjointness rule is verbatim **line 195**: "If two `KEY` are
  `KP/<M;N>/*` and `KP/<P;Q>/*` for the same key placeholder `KP`, then the
  sets `{M, N}` and `{P, Q}` must be disjoint."
- Row 3's "LEGAL" is confirmed by the BIP's own *valid* example at **line 291**:
  `tr(@0/**,{sortedmulti_a(1,@0/<2;3>/*,@1/**),or_b(pk(@2/**),s:pk(@3/**))})` —
  `@0` at `{0,1}` and at `{2,3}`.

No finding is filed against the quotes. The findings against N1 (I1, M1) are
about axes the taxonomy's classification key **cannot see**, and I3 is about a
BIP-388 rule the taxonomy's scope excludes.

---

# CRITICAL

## C1 — N1's named enforcement home makes N1's own read-side rule unimplementable: `md inspect` and `md verify` re-enter `encode_payload`

**Severity:** Critical (unmet guarantee; an already-engraved plate stops being
readable, which is the exact hazard `encode.rs`'s own comment says the
encode-only placement exists to prevent).

**Evidence.**

The spec, N1 "Single-source invariant":

> The taxonomy is enforced by ONE implementation reachable from every verb
> above, extending the `validate_no_duplicate_key_slots` discipline (its two
> call sites are the model).

and N1 "Read side (walk-confirmed)":

> `decode`, `inspect`, `bytecode` still READ a card carrying R-N1a's shape,
> printing a warning that names the BIP-388 violation

The brainstorm this was authored from states the premise explicitly
(`design/BRAINSTORM_mdcli_mini.md:95-96`): *"Reading verbs do NOT run the
encode-path validators, which is what keeps already-minted cards readable"* —
and it is **false** for two reading verbs:

- `crates/md-cli/src/cmd/inspect.rs:32` — `let md1 = compute_md1_encoding_id(&descriptor)?;`
- `crates/md-codec/src/identity.rs:40` — `let (bytes, _bit_len) = encode_payload(d)?;`
- `crates/md-codec/src/encode.rs:99-121` — `encode_payload` runs
  `validate_placeholder_usage`, `validate_multipath_consistency`,
  `validate_tap_script_tree`, `validate_origin_key_consistency` and
  `validate_no_duplicate_key_slots` before writing a bit.

and independently:

- `crates/md-cli/src/cmd/verify.rs:52` — `let (decoded_bytes, decoded_bits) = encode_payload(&decoded)?;`
  — `verify` re-encodes the **decoded card**, not just the expected template.

The `?` on each call propagates the validator's `Err` straight out. There is no
branch that suppresses validation for a read.

**Failure construction (measured baseline, projected post-change).**

Today, at this tree, `md encode "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" --key @0=<K0> --path "48'/0'/0'/2'"` mints:

```
md1fakqnqspqztvyyy4qqxppcgg4gythgx8egtq4pcwl6u5p2us6r6zsnl2rd0q6gghvalgywfyx3z0nn28m7t
md1fakqnqs0cdlz64mrqgdrha0m7umapumfj075dhzfzvynh66n94j5lcxlmx9ayav9mj0jjqpx5yl5n7q5v9j
```

and `md inspect` on that pair exits 0, printing
`template: wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))`,
`md1-encoding-id: ed813834dbfb11ee887f56c385db2814`, and the origins block.

Place the R-N1a validator "beside `validate_no_duplicate_key_slots`" as the
spec and the brainstorm both direct, and that same `md inspect` call becomes
`md: codec error: …` at exit 1 — because the identity computation cannot
complete. Same for `md verify <those two chunks> --template … --key …`. An
operator holding a plate cut before this cycle can no longer inspect or verify
it; the spec promises they can, and the promise is what the
already-minted-cards-stay-readable argument in `encode.rs:110-117` rests on.

The row named in N1's Vectors — "a decode-warn row from a hand-built md1 vector
string carrying R-N1a" — is written against `decode` only. `decode` does *not*
touch `encode_payload` (verified: no `identity` or `encode_payload` import in
`crates/md-cli/src/cmd/decode.rs`), so **the row passes while `inspect` and
`verify` are broken**. This is the "passes in both worlds" class for the two
verbs that actually regress.

Compounding it: the placement that satisfies *both* halves of N1 exists and is
unnamed in the spec. `crates/md-cli/src/parse/template.rs:723`
`resolve_placeholders` is already the single funnel every template-parsing verb
passes through — `md encode` and `md verify` via `parse_template_ext`
(`cmd/encode.rs:68`, `cmd/verify.rs:48`), `md descriptor` and `md address` via
`build_descriptor` → `parse_template` (`cmd/build.rs:64`) — and it is
unreachable from the read side. The spec points the plan at the one home that
breaks the read-side rule.

**Direction (one line):** name the template-surface funnel, not the encode-path
validators, as the taxonomy's home — and add read-side rows for `inspect` and
`verify`, not `decode` alone.

---

## C2 — N1's verb enumeration names a verb that does not exist and omits a spendable-artifact verb; two of its vector rows are correspondingly unexecutable and unwritten

**Severity:** Critical (a named row that cannot exist; and an unpinned path that
produces a receive address for the exact wallet the cycle exists to refuse).

**Evidence.**

The spec, N1:

> Every verb that parses a template with placeholders toward a mintable or
> spendable artifact (`md encode`, `md build`, `md descriptor --template`) …

and N1 "Vectors":

> R-N1a refusal at `encode`, at `descriptor --template`, and at `build`

**There is no `md build`.** The `Cmd` enum in `crates/md-cli/src/main.rs`
declares exactly: `Encode` (101), `Decode` (190), `Verify` (204), `Inspect`
(239), `Bytecode` (251), `Vectors` (261), `Compile` (266), `Descriptor` (289),
`Address` (447), `Decompose` (620), `GuiSchema` (648), `Repair` (~662),
`GenMan` (671). `crates/md-cli/src/cmd/build.rs` is the shared
`build_descriptor` **helper**, whose module doc says so in its first line:
*"Building a `Descriptor` from either input shape, in ONE place. Two commands
now need it — `md address` and `md descriptor`."* The spec's
"Machine-verified" block cites `crates/md-cli/src/cmd/build.rs:301` as a call
site — correct as a *line*, but the spec then promotes the file name to a verb
name. Acceptance criterion 1 requires *"Every vector row named above exists as
an executable test"*; the `build` row cannot exist, so criterion 1 is
unsatisfiable as written and will be closed by writing something that is not
the named row.

**`md address --template` is missing from both lists.** It is a real
subcommand (`main.rs:447-570`, with `--template` at 452, `--key` at 462,
`--path` at 530), it produces a spendable artifact (a receive address), and the
spec's read-side sentence even names `address` as a composing verb that must
refuse — while the mint-side enumeration and the vector list both omit it.

**Failure construction (measured at this tree, exit 0):**

```
$ md address --template "wsh(sortedmulti(2,@0/<0;1>/*,@0/<0;1>/*))" \
      --key @0=xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf \
      --path "48'/0'/0'/2'" --count 1
bc1ql5j095gqvdv6ugccf956pduc2e0vevtfnf9r72nhmln9lf8tlmmsd9ujlz
note: stdout is watch-only — public keys only, cannot spend
```

That is a receive address for a "2-of-2" one key satisfies alone — the exact
shape R-N1a exists to refuse. An implementer who follows the enumeration
literally (and who has no `build` verb to write a row against) lands the check
in `md encode` and in `md descriptor`, and this invocation keeps printing an
address, with no vector row demanding otherwise. It happens to be *likely* that
an implementer puts the check in `build_descriptor` (which is shared by
descriptor and address, `cmd/build.rs:66`), but "likely" is what a vector row
exists to replace, and `md encode` does **not** go through `build_descriptor`,
so a `build_descriptor`-only placement misses `md encode` instead. Either
reading of the enumeration leaves a spendable-artifact verb uncovered and
unpinned.

`md verify --template` (`main.rs:204-238`, `cmd/verify.rs:48`) is a third
template-parsing verb absent from the enumeration; see C1 for why it is the
verb that most sharply exposes the placement question.

**Direction (one line):** derive the enumeration from the `Cmd` enum rather
than from file names, and give every template-parsing verb a row.

---

# IMPORTANT

## I1 — R6's central factual claim is false: the shipped desugar cannot be applied to decompose's input at all

**Severity:** Important (a false machine-checkable fact that a plan will
inherit; the named row either fails or is closed by a second, divergent
implementation).

**Evidence.** The spec, R6:

> decompose applies the same desugar the template surface ships

and the brainstorm it folds (`BRAINSTORM_mdcli_mini.md:274-275`): *"The fix
reuses the shipped desugar."*

The shipped desugar is `desugar_double_wildcard`,
`crates/md-cli/src/parse/template.rs:67`, and its regex (line 73) is:

```rust
Regex::new(r"@\d+(?:/\d+'?)*/\*\*").expect("static regex compiles")
```

It is anchored on a literal `@` followed by a placeholder index. `md decompose`
takes a **concrete descriptor with real extended keys** — `md decompose
"wpkh([73c5da0a/48'/0'/0'/2']xpub6DkFA…/**)"` — which contains no `@i`
placeholder anywhere. The function's own fast path (`if !template.contains("**")`)
is not even the one that fires; the regex simply finds zero matches, `last`
stays 0, and it returns `Cow::Borrowed(input)` **unchanged**.

**Failure construction.** Implement R6 by calling `desugar_double_wildcard` on
decompose's input, as the spec says. `md decompose "wpkh([73c5da0a/48'/0'/0'/2']xpub…/**)"`
hands `/**` to rust-miniscript unchanged (the pin at `ff4732e` does not parse
it), decompose still refuses, and R6's named row — *"a `/**` descriptor
decomposes identically to its `/<0;1>/*` rewrite"* — fails. The likely repair
under time pressure is a second, key-shaped desugar written next to the first —
which is precisely the "two regexes carrying a standing keep-these-in-sync
obligation" hazard that `template.rs:53-62` documents as its own reason for
existing.

**Direction (one line):** state that decompose needs its own key-shaped
rewrite, or that the shared piece must be generalised off the `@i` anchor
first.

---

## I2 — the taxonomy's classification key is coarser than what the parser already distinguishes, so at least two non-repetition shapes fall into R-N1a and are refused with a BIP-388 citation that does not apply to them

**Severity:** Important (unsound classification; a diagnostic that falsely
accuses BIP 388, under an operator ruling that governs exactly the accuracy of
these diagnostics — and the R-N1a row asserting the repeated-key wording
*passes* on it).

**Evidence.** The spec, N1:

> classifies a placeholder appearing at more than one use site by its use-site
> multipath sets A and B

The parser this replaces draws a **three-way** distinction, not a one-way one.
`crates/md-cli/src/parse/template.rs:730-741`:

```rust
if prev.multipath_alts != occ.multipath_alts
    || prev.wildcard_hardened != occ.wildcard_hardened
    || prev.origin_path != occ.origin_path
{
    return Err(CliError::TemplateParse(format!(
        "@{} appears with inconsistent path/multipath/hardening",
        occ.i
    )));
}
```

Classifying on `multipath_alts` alone collapses the other two axes into
"A = B".

**Failure construction, case 1 (differing inline origins).**
`wsh(multi(2,@0/48'/0'/0'/2'/<0;1>/*,@0/48'/0'/1'/2'/<0;1>/*))`. Both
occurrences lex with `multipath_alts = [0,1]` (the origin steps are consumed by
the lexer regex's group 2, `template.rs:130`), so A = B = {0,1} and the spec
routes it to **R-N1a**, whose wording is mandated to name "BIP 388's repeated-key
rule" and the invalid-example string "Repeated keys with the same path
expression". The wallet is not that: it is one placeholder claiming two
different *origins*, an md-side representability contradiction. BIP-388 does not
have a rule about it — in fact `@0/48'/0'/…` is outside BIP-388's `KEY` grammar
entirely (bip-0388 line 152-154: a `KP` is *always* followed by `/**` or
`/<NUM;NUM>/*`; line 305 lists `pkh(@0/0/**)` — "Key placeholder with an
explicit path present" — as **invalid**). The refusal's disposition is right;
its stated reason is a fabricated BIP violation. Today's message
("inconsistent path/multipath/hardening") is at least accurate about the axis.

**Failure construction, case 2 (differing wildcard hardening).**
`wsh(multi(2,@0/<0;1>/*,@0/<0;1>/*'))`. Again `multipath_alts` matches, so
A = B, so R-N1a, so the same false citation — for a shape whose actual defect is
that a hardened wildcard is un-derivable on an xpub (`template.rs:119-127`).

In both cases the R-N1a vector row — which asserts the BIP-repeated-key wording
and the absence of "invalid" — **passes**, so nothing in the acceptance set can
detect the misclassification.

Note also that BIP-388's disjointness rule (line 195) is stated *only* for two
`KP/<M;N>/*` `KEY` expressions. It has nothing to say about two use sites that
differ in origin steps or hardening, so no row of the taxonomy has authority
over them.

**Direction (one line):** make the classification key the same triple the
parser compares, and give the non-multipath axes their own disposition.

---

## I3 — the same xpub in two slots at disjoint use-sites: the T row mints it, the S row refuses it, both citing BIP 388 — and this spec pins the refusing half while leaving the minting half open

**Severity:** Important (unsound admission of a BIP-388-forbidden wallet; one
binary gives two answers for one wallet, and after this cycle both answers are
vector-pinned as correct).

**Evidence.**

BIP-388, "Additional rules", **line 193**, verbatim from the authoritative
source:

> The public keys obtained by deserializing elements of the key information
> vector must be pairwise distinct

The key information vector for `wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))` with
`@0 = @1 = X` holds **two** elements, both `X`. Not pairwise distinct.
**Forbidden by BIP 388**, and the disjointness rule (line 195) does not rescue
it — that rule is about one placeholder's two `KEY` expressions, i.e. one
key-info entry, not two.

The **S row refuses it**, citing exactly that rule:
`crates/md-cli/src/seat/satisfy.rs:294` `check_no_repeated_xpub` compares
`public_key` and `chain_code` only — declared paths and use sites never enter —
and its message cites `crate::bip388::PAIRWISE_DISTINCT_RULE`.

The **T row mints it**, and the tree asserts that this is correct:

- `crates/md-cli/src/cmd/build.rs:280-283`: *"one xpub at `<0;1>` and at
  `<2;3>` derives a different child at every index, which is two wallets and not
  a duplicate — **BIP 388 permits it** and `md encode` mints it"*.
- `crates/md-cli/tests/duplicate_key_slots.rs:82`
  `one_key_at_two_different_use_sites_is_not_a_duplicate`.
- `crates/md-cli/tests/duplicate_key_slots.rs:~314`
  `t_row_one_key_at_two_disjoint_use_sites_still_composes`, whose doc says *"the
  DISJOINT use-site form BIP 388 permits"*.
- `crates/md-codec/src/validate.rs:353-355` — the boundary comment inside
  `validate_no_duplicate_key_slots` itself.

Measured at this tree, exit 0:

```
$ md descriptor --template "wsh(multi(2,@0/<0;1>/*,@1/<2;3>/*))" \
      --key @0=<K0> --key @1=<K0> --path "48'/0'/0'/2'"
wsh(multi(2,xpub661MyMwAqRbcG…/<0;1>/*,xpub661MyMwAqRbcG…/<2;3>/*))#3sxca8l0
```

**Why this is a defect in *this* spec rather than a converter re-audit.** The
spec reaches onto this exact axis and pins only the side that already refuses.
N1 "Vectors":

> a V-BOUND-REF sibling row pinning same-xpub-at-DIFFERENT-declared-paths
> refusing at seating (measured 2026-08-31, currently unpinned)

After this cycle the repository will carry a vector row asserting the S row
refuses same-xpub, alongside two existing vector rows asserting the T row mints
same-xpub, with the T-row rows justified by a claim ("BIP 388 permits it") that
the BIP text contradicts. The operator ruling the spec opens with — *"Bad ideas
can be valid, but we don't want to support BIP forbidden wallets"* — decides
this case, and the spec's "verbs must not be able to diverge" invariant is the
one it violates.

**Direction (one line):** either bring the T row's pairwise-distinct handling
into the taxonomy's scope, or state explicitly that this axis is out of scope
and correct the "BIP 388 permits it" rationale that the spec's row 3 sits next
to.

---

## I4 — N2's byte oracle omits the two inputs that decide byte-identity, so the primary row cannot pass for a correct implementation

**Severity:** Important (the row named as N2's *primary* oracle is unexecutable
as specced; the natural repair silently degrades the minted card).

**Evidence.** The spec, N2 "Oracle":

> The minted card must be byte-identical to the card `md encode` mints given
> **the same template and the fixtures' real account-level keys with the same
> origins** — the primary row.

Byte-identity of an md1 payload is decided by more than template + keys +
origins. `crates/md-codec/src/encode.rs:122-135` writes, in order: the header,
`path_decl`, `use_site_path`, the tree, and the **TLV section** — which carries
`fingerprints`, `pubkeys` and `origin_path_overrides`
(`crates/md-codec/src/tlv.rs:24-51`).

The seating side always inherits the policy card's fingerprints:
`crates/md-cli/src/seat/compose.rs:219` asserts
`seated.tlv.fingerprints == policy.tlv.fingerprints`, and the module doc
(`compose.rs:10`) states the rule — *"the mk1 card's fingerprint never
overwrites a fingerprint-free declaration: the policy [declaration wins]"*.

`md encode` writes a `Fingerprints` TLV only from `--fingerprint @i=HEX`
(`main.rs:145`); an account-level xpub's own parent fingerprint is not a master
fingerprint and is not used. So for any fixture whose keyless policy card
declares fingerprints — which is the normal shape, and the one
`md decompose --emit` exists to reproduce — the two sides differ in the
`Fingerprints` TLV, hence in the payload bytes, hence in the md1 string.

The same argument applies to the origin-path declaration shape: a policy card
carrying per-slot origins is `PathDeclPaths::Divergent`, while `md encode
--path P` produces `Shared` (this is the flattening
`validate_origin_key_consistency`'s doc comment at `validate.rs:~400` records as
having produced 9-of-9 contradictory conformance vectors). The spec's phrasing
"with the same origins" does not distinguish them.

**Failure construction.** Implement N2 exactly as written; write the primary
row as `assert_eq!(minted_md1, md_encode(template, keys, origins))`. On a
fingerprint-bearing fixture the row fails against a *correct* implementation.
The cheapest way to make it pass is to stop carrying the seating fingerprints
into the minted card — which contradicts the spec's own sentence three lines
earlier ("The minted card carries the origin metadata learned from seating") and
degrades the card into a fingerprint-free declaration, a shape SPEC A3(c)
treats differently at seating time.

**Direction (one line):** state the oracle's full input set — template, keys,
origins, **fingerprints, and the path-declaration shape** — or name a
fingerprint-free fixture as the one the primary row uses.

---

## I5 — R3's exit-code scheme conflates "not spend-equal" with every input error, in the one place the FOLLOWUP says a false signal invites re-cutting a good plate

**Severity:** Important (a funds-shaped check whose caller cannot distinguish
the answer from a failure to answer).

**Evidence.** The spec, R3:

> Exit 0 equal / 1 not.

`crates/md-cli/src/main.rs:689-703`:

```rust
match dispatch(cli.command) {
    Ok(code) => ExitCode::from(code),
    Err(CliError::BadArg(m)) => { eprintln!("md: {m}"); ExitCode::from(2) }
    Err(e) => { eprintln!("md: {e}"); ExitCode::from(1) }
}
```

Every `CliError` except `BadArg` already exits **1** — a mistyped md1 string
(`Codec`), an unreadable `FILE`, a template that will not parse
(`TemplateParse`), a seating refusal (`Seat`). R3 proposes to add "the two
wallets are not spend-equal" to that same bucket.

**Failure construction.** An operator scripts the check the FOLLOWUP was
written for:

```sh
md descriptor <split set> --from-mk1 … --verify-against <keyed md1> || recut_plate
```

A single mistranscribed character in the `--verify-against` argument exits 1
with a decode error, the script re-cuts a plate that is fine, and the entry's
stated motivation — *"the worst direction for a funds-shaped check, since it
invites re-cutting plates that are fine"* — is reproduced by the remedy. The
same binary already solved this: `md repair` reserves a distinct code (**5**,
`REPAIR_APPLIED`) for a non-error non-default outcome and documents it in
`main.rs:~652-660` as "D26 cross-CLI parity".

Related, same section, smaller: R3 does not say which of `md descriptor`'s
input modes accept `--verify-against` (`--template`, phrases, `--from-mk1`), so
the flag's admissibility matrix is undefined.

**Direction (one line):** give "NOT spend-equal" its own exit code, as
`md repair` already does for its own third answer.

---

## I6 — R9(a) moves the arity edge rather than closing it, and the spec pins only the ordering that works

**Severity:** Important (a missing case on the converter's most-trodden
entrance, created by the fix).

**Evidence.** The spec, R9:

> (a) `num_args = 1..` so the natural paste works — the plan verifies the
> positional/greedy-flag interaction with a mixed vector (md1 positionals and
> mk1 strings in one invocation); (b) the md1 positional refuses an
> `mk1…`-prefixed string BY NAME, pointing at `--from-mk1`.

The arg definitions are `main.rs:291` (`phrases: Vec<String>`,
`#[arg(num_args = 0..)]`, a variadic positional) and `main.rs:400-401`
(`from_mk1: Vec<String>`, `#[arg(long = "from-mk1", value_name = "STRING", conflicts_with = "template")]`).

In clap 4, a multi-value option with `num_args = 1..` consumes values greedily
until it meets a token beginning with `-` (absent `allow_hyphen_values`) or the
end of argv. It does not stop to leave one for a positional. Both `md1…` and
`mk1…` strings are bare bech32-ish tokens with no leading `-`.

**Failure construction.** After R9(a), the operator pastes in the other natural
order — flag first, then the policy card:

```
md descriptor --from-mk1 mk1qq…A mk1qq…B md1qq…POLICY
```

`from_mk1` receives all three strings, `phrases` stays empty, and the
`descriptor_input` `ArgGroup` (`main.rs:288`, `.required(true)`) fails →
`error: the following required arguments were not provided: <PHRASES|--template>`
at exit 2, for an invocation containing a perfectly good policy card. If the
group is satisfied by other means, the md1 string reaches the mk1 decoder and
draws a bare mk-side codec error — the F-420 class the rider exists to close,
in the mirror direction.

R9(b) adds the guard for `mk1…` in the md1 positional but **not** the symmetric
guard for `md1…` in `--from-mk1`, and the vector obligation names "a mixed
vector … in one invocation" without fixing which ordering, so the row that gets
written will be the one that passes.

**Direction (one line):** pin both orderings, and add the symmetric
`md1…`-in-`--from-mk1` refusal alongside R9(b).

---

## I7 — R-N1c's stated goal ("the text stops reading as a user error") is not reachable through the specified row, because the error *prefix* is not part of the message

**Severity:** Important (the diagnostic contract this cycle is built around can
ship self-contradicting, with the acceptance row green).

**Evidence.** The spec, R-N1c:

> (today's "@0 appears with inconsistent path/multipath/hardening") is rewritten
> to state: the wallet is BIP-legal; md1 deliberately cannot express it … Behavior
> unchanged — message only.

and Acceptance 4:

> No diagnostic introduced by this cycle contains the word "invalid" … asserted
> by the vector rows, not by convention.

The refusal is raised as `CliError::TemplateParse`
(`crates/md-cli/src/parse/template.rs:735`), and `CliError`'s `Display`
(`crates/md-cli/src/error.rs:87`) is:

```rust
CliError::TemplateParse(m) => write!(f, "template parse error: {m}"),
```

`main.rs:700` prints `md: {e}`.

**Failure construction.** "Message only" is implemented by changing the string
argument and leaving the variant. The operator sees:

```
md: template parse error: this wallet is legal under BIP 388, but md1 cannot
express one key at two different use-site paths (one path per key slot) — keep
it as a descriptor and engrave with `me sysw pack --as descriptor`.
```

The body says "your wallet is fine, our format is narrow"; the prefix says
"your input failed to parse". The row asserts the body's three phrases and the
absence of the token `invalid`, all of which hold, so the row is green. The
brainstorm's own words for the goal (`BRAINSTORM_mdcli_mini.md:149`) — *"the
text stops reading as a user error"* — are unmet, and nothing in the acceptance
set can see it.

**Direction (one line):** make the error *variant* (and therefore the prefix)
part of what R-N1c specifies and what its row asserts.

---

# MINOR

## M1 — "a fixed derivation counts as its singleton set" describes a shape md's use-site grammar cannot represent

The spec, N1: *"by its use-site multipath sets A and B, where a fixed
derivation counts as its singleton set"*.

md1's `UseSitePath` carries a multipath group plus a wildcard and nothing else.
A fixed step *after* the multipath is rejected outright —
`crates/md-cli/src/parse/template.rs:562` `lex_rejects_post_multipath_fixed_step`
and `:576` `lex_rejects_post_multipath_normal_step`, with the comment at
`:553-559` stating *"md1's UseSitePath cannot represent post-multipath fixed
steps, so the form is REJECTED (fail-closed)"*. A fixed step *before* the
multipath is consumed by the lexer's group 2 as **origin path**
(`template.rs:130`), not as a use-site derivation, and
`make_use_site_path` (`template.rs:795-807`) builds `Alternative`s from
`multipath_alts` alone.

So the singleton clause is either vacuous — nothing ever reaches the classifier
carrying a fixed use-site derivation, and any row exercising it is unbuildable —
or it is read as applying to the *origin* steps, which produces I2's case 1 with
the opposite (and equally wrong) disposition: `@0/0/<0;1>/*` vs `@0/1/<0;1>/*`
would classify as disjoint singletons {0} vs {1} → R-N1c → a message asserting
the wallet is "BIP-388-legal", for a shape BIP-388 lists as invalid
(`pkh(@0/0/**)`, bip-0388 line 305).

## M2 — R5(b) widens the test job only; clippy and doc stay at default features, in CI and in the phase gate

`.github/workflows/ci.yml:65` runs `cargo clippy --workspace --all-targets -- -D warnings`
and `:93` runs `cargo doc --workspace --no-deps --document-private-items` (with
`RUSTDOCFLAGS: "-D warnings"` set as job env at `:81-82`). Neither carries
`--all-features`, and the spec's phase gate mirrors that omission —
`--all-features` is attached only to the nextest line. R5(a) deletes
`render_tr_template` from `crates/md-cli/src/compile.rs`, which is gated behind
`cli-compiler`; any dead import or now-unused helper left behind in that module
is invisible to both. The "ungated by CI" defect class is closed for tests and
left open for lint and doc.

(For the record, machine-checked above: the tripwire really is the only
`--all-features` failure — 1106 run, 1105 passed, 1 failed, 2 skipped — so
R5's sequencing itself is sound.)

## M3 — `--emit md1`'s admissible input modes are undefined, and the spelling collides with `md decompose --emit`

N2 shows one invocation (`md descriptor <keyless md1…> --from-mk1 … --emit md1`)
and states `--emit md1` "composes with `--seat`", but says nothing about
`--template` (where it would duplicate `md encode`) or a plain keyed-card
positional (where it would be a re-emit). Separately, `md decompose --emit`
already exists with an unrelated value set (`template|keys|commands`,
`main.rs:~636`), so one flag name will carry two disjoint vocabularies on two
verbs.

## M4 — R-N1c's escape hatch is named with a spelling that does not exist

The spec: *"keep it as a descriptor (engraving path: `me … --as descriptor`)"*.
The real surface is `me sysw pack --as <descriptor|md1> --in <file>` — its own
guidance line, `mnemonic-engrave/crates/me-cli/src/main.rs:723`. A diagnostic
whose whole purpose is to redirect the operator should carry the runnable
spelling.

---

# NITS

## N1 — the `spend_equal` dead-code comment becomes false when R3 lands

`crates/md-cli/src/seat/compose.rs:~138`: *"Nothing on the C2 CLI surface calls
it, because C2 ships no channel for supplying a keyed card alongside a split
set."* R3 is that channel. Grep for the comment at fold time; the
`#[allow(dead_code)]` and its justification both go.

## N2 — the "two call sites" citation is accurate but names the wrapper, not the reachability

`crates/md-cli/src/cmd/build.rs:301` is the `validate_no_duplicate_key_slots`
call, inside `fn refuse_key_reuse_across_slots` (declared at `:300`). What
matters for the single-source invariant is its *caller*, `build.rs:66`, inside
`build_descriptor`'s `--template` branch — which is what makes the check reach
`md descriptor` **and** `md address`, and what makes C2's omission visible.

---

COUNTS: 2C / 7I / 4M / 2N
