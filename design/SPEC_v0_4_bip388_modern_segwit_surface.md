# v0.4 Design Spec: BIP 388 Modern Post-Segwit Surface Subset

**Brainstormed**: 2026-04-27 via `superpowers:brainstorming` skill
**Status**: Approved by user; ready for writing-plans handoff
**Per-section agent reviews**: Sections 2, 3, 4, 5 reviewed by Opus 4.7 peer agents (revisions folded inline)
**Closes FOLLOWUPS**: `v0-4-bip-388-surface-completion` (with closure-note correction — see §6)
**Files NEW FOLLOWUPS at release**: `v0-5-multi-leaf-taptree`, `legacy-pkh-permanent-exclusion`, `legacy-sh-multi-permanent-exclusion`, `legacy-sh-sortedmulti-permanent-exclusion`, `bip48-nested-name-table-entry-followup` (if deferred)

---

## §1. Scope and Goals

**Goals.** v0.4.0 of `md-codec` lifts the encode/decode rejection of three currently-rejected top-level descriptor types, completing the **modern post-segwit subset of BIP 388**. MD is deliberately narrower than BIP 388 — see §FAQ for the rejected-by-design types.

Newly accepted top-level types:

| Type | What | Address prefix | BIP |
|---|---|---|---|
| `wpkh(@0/**)` | Native segwit single-sig | `bc1q...` | BIP 84 |
| `sh(wpkh(@0/**))` | Nested-segwit single-sig | `3...` | BIP 49 |
| `sh(wsh(SCRIPT))` where `SCRIPT` is any valid wsh inner | Nested-segwit multisig | `3...` | BIP 48/1' |

Wire format is strictly additive at the type level — no new tags, no new header bits, no schema bump. `Pkh = 0x02`, `Sh = 0x03`, `Wpkh = 0x04` are already allocated in `bytecode/tag.rs` but currently rejected at the encode/decode layer. The `Wsh` inner subtree machinery used for `wsh(sortedmulti)` is reused inside `sh(wsh(...))`.

**Default path-tier convention**: when neither caller-supplied path (Tier 0) nor key-origin-extracted path (Tier 1) is available, the encoder picks a per-top-level-type default (e.g., `wpkh` → BIP 84, `sh(wpkh)` → BIP 49). Caller `--path` flag overrides per existing semantics. This **strong default + explicit override** behavior is consistent with how existing `wsh` and `tr` types resolve their tiers; no asymmetry between old and new top-level types.

**Non-goals (out of scope, deferred to future releases):**

- Subsystem (2): **multi-leaf TapTree** (`tr(KEY, TREE)` with non-trivial tree). Deferred to v0.5+. Filed as new FOLLOWUPS `v0-5-multi-leaf-taptree` at release.
- Subsystem (4): **full miniscript expressivity** beyond BIP 388 wallet-policy grammar.
- Subsystem (5): **inline xpubs / foreign keys** (descriptor-codec tag range 0x24–0x31).

**Rejected-by-design (permanently EXCLUDED, narrower than BIP 388):**

- `pkh(KEY)` — legacy P2PKH single-sig. BIP 388 admits; MD rejects. See §FAQ.
- `sh(multi(...))`, `sh(sortedmulti(...))`, `sh(KEY)` — legacy P2SH. BIP 388 admits; MD rejects. See §FAQ for the address-prefix-ambiguity rationale.
- `bare(SCRIPT)` — pre-2014 raw script. BIP 388 also excludes this; MD inherits.

**Sh restriction matrix** (3-cell):

- `Sh -> Wpkh(K)` ALLOWED
- `Sh -> Wsh(...)` ALLOWED
- `Sh -> {anything else}` REJECTED with `Error::PolicyScopeViolation` (diagnostic names the offending inner type)

---

## §2. Wire Format and Restriction Matrix

### Bytecode shapes (composition over existing tags; placeholder index is single-byte, NOT LEB128)

| Policy | Bytecode (after header) | Bytes |
|---|---|---|
| `wpkh(@0/**)` | `[Wpkh=0x04][Placeholder=0x32][index=0x00]` | 4 |
| `sh(wpkh(@0/**))` | `[Sh=0x03][Wpkh=0x04][Placeholder=0x32][index=0x00]` | 5 |
| `sh(wsh(sortedmulti(2, @0/**, @1/**, @2/**)))` | `[Sh=0x03][Wsh=0x05][SortedMulti=0x09][k=2][n=3][Placeholder][0][Placeholder][1][Placeholder][2]` | 12 |

**Capacity sanity check**: `wpkh` with fingerprints block ≈ 12 bytes; `sh(wsh(sortedmulti 2-of-3))` ≈ 13 bytes. Both well within Regular's 48-byte single-string capacity. No chunking impact.

### Wsh subtree handler refactor (CRITICAL pre-implementation requirement)

`decode_wsh_inner` at `crates/md-codec/src/bytecode/decode.rs:97-157` currently returns `Descriptor::Wsh(...)` — the *top-level* descriptor. For `sh(wsh(...))` we need the inner `Wsh<DescriptorPublicKey>`. Refactor BEFORE implementation begins:

- Split into `decode_wsh_body(cur, keys) -> Wsh<DescriptorPublicKey>` (the work)
- Thin wrapper `decode_wsh_inner(cur, keys) -> Descriptor::Wsh(decode_wsh_body(...))` (preserves existing call site)
- `Sh -> Wsh` calls `decode_wsh_body` directly, then wraps via `Descriptor::new_sh_with_wsh(wsh)` (NOT `new_sh_wsh` which takes Miniscript)

Same on encode side: `Wsh::encode_template` at `encode.rs:147-155` already only emits inner script (good as-is); the `Sh` arm matches against miniscript v13's `ShInner` enum (3 variants: `Wpkh`, `Wsh`, `Ms` — `ShInner::SortedMulti` does NOT exist, `Ms` covers all legacy P2SH).

### Restriction matrix (enforced symmetrically on encode AND decode)

```
Sh -> Wpkh(KEY)              ALLOWED  → BIP 49 single-sig
Sh -> Wsh(<wsh-inner>)       ALLOWED  → BIP 48/1' multisig
Sh -> Multi/SortedMulti      REJECTED → "v0.4 does not support sh(<legacy P2SH>);
                                         use sh(wsh(sortedmulti(...))) for modern
                                         nested-segwit multisig"
Sh -> Pkh                    REJECTED → "v0.4 does not support sh(<inner type>)"
Sh -> Tr                     REJECTED → "sh(tr(...)) is not valid"
Sh -> Sh                     REJECTED → "nested sh wrappers are not valid"
Sh -> Bare                   REJECTED → "sh(bare(...)) is not valid"
Sh -> {any inner-script tag} REJECTED → "sh(...) requires wpkh or wsh wrapper"
```

### Hostile-input invariants

**Recursion-bomb defense**: `decode_sh_inner` MUST peek the next tag byte BEFORE recursive descent. A trivial misimplementation calling `decode_descriptor` recursively from inside `decode_sh_inner` would admit `Sh -> Sh -> Sh -> ...` bombs and stack-overflow on a 100-byte hostile payload. Same hostile-input class as v0.2.2 `MalformedPayloadPadding` defense.

**Wpkh length contract**: Top-level `Wpkh` consumes EXACTLY `[0x04][0x32][index_byte]` (3 bytes after dispatch). After consuming the placeholder, the wpkh decoder MUST NOT accept additional tags. `sh(wpkh(...))` consumes EXACTLY 4 bytes. Enforced at the wpkh subtree's exit point, before the top-level `cur.require_empty()`.

### Layering invariant for forward-compatibility (subsystem 5 prep)

Three tag families exist; the dispatcher layers MUST NOT cross them:
- **Wrapper/top-level family**: `Pkh, Sh, Wpkh, Wsh, Tr, Bare` (0x02–0x07)
- **Inner-script family**: `SortedMulti, Multi, MultiA, AndV, …, RawPkH, After, Older, hash-locks` (0x09–0x23)
- **Key-slot family**: `Placeholder` (0x32), and future `Reserved{Origin,…,XPriv}` (0x24–0x31) when subsystem 5 lands

The `Sh -> {any inner-script tag} REJECTED` row is correct *only because* key-slot-family tags never appear directly under `Sh` — they only appear inside placeholder positions within inner-script tags. This invariant MUST be locked in a comment at `decode.rs::decode_sh_inner` so subsystem 5 implementers don't accidentally widen `Sh -> ReservedXPub`.

### Header bits unchanged. Error reporting via existing `Error::PolicyScopeViolation`.

---

## §3. Encoder + Decoder Implementation Surface

### Encoder changes (`crates/md-codec/src/bytecode/encode.rs`)

Lift the rejection at lines 98-106. Replace with structured dispatch:

```rust
Descriptor::Wpkh(wpkh) => {
    out.push(Tag::Wpkh.as_byte());
    wpkh.as_inner().encode_template(out, placeholder_map)
}
Descriptor::Sh(sh) => {
    out.push(Tag::Sh.as_byte());
    match sh.as_inner() {
        ShInner::Wpkh(wpkh) => {
            out.push(Tag::Wpkh.as_byte());
            wpkh.as_inner().encode_template(out, placeholder_map)
        }
        ShInner::Wsh(wsh) => {
            out.push(Tag::Wsh.as_byte());
            wsh.encode_template(out, placeholder_map)
        }
        ShInner::Ms(_) => Err(Error::PolicyScopeViolation(
            "v0.4 does not support sh(<legacy P2SH>) including sh(multi/sortedmulti); \
             use sh(wsh(sortedmulti(...))) for modern nested-segwit multisig".to_string()
        )),
    }
}
// Note: §2's restriction matrix lists "Sh -> Sh REJECTED" but miniscript v13's
// ShInner enum has only 3 variants (Wpkh, Wsh, Ms); sh(sh(...)) is structurally
// unreachable on the encoder side. The §2 matrix row applies to the DECODER only,
// where hostile inputs CAN present arbitrary tag bytes. No encoder arm needed.
Descriptor::Pkh(_) | Descriptor::Bare(_) => Err(Error::PolicyScopeViolation(
    "v0.4 does not support top-level pkh()/bare() (legacy non-segwit out of scope)".to_string()
)),
```

(Tone matches existing `decode.rs:65-91` "v0.X does not support …" prefix.)

### Decoder changes (`crates/md-codec/src/bytecode/decode.rs`)

**Naming convention**: keep `decode_wsh_inner` as the existing top-level wrapper (no churn for existing call sites). Add `decode_wsh_body` returning `Wsh<DescriptorPublicKey>` for reuse inside `sh(wsh)`. Add `decode_wpkh_inner`, `decode_sh_inner` symmetric to existing `decode_tr_inner`.

```rust
fn decode_wsh_inner(cur: &mut Cursor, keys: &[DescriptorPublicKey])
    -> Result<Descriptor<DescriptorPublicKey>, Error> {
    Ok(Descriptor::Wsh(decode_wsh_body(cur, keys)?))
}

fn decode_wsh_body(cur: &mut Cursor, keys: &[DescriptorPublicKey])
    -> Result<Wsh<DescriptorPublicKey>, Error> { ... }

fn decode_wpkh_inner(cur: &mut Cursor, keys: &[DescriptorPublicKey])
    -> Result<Descriptor<DescriptorPublicKey>, Error> {
    let key = decode_placeholder(cur, keys)?;
    Ok(Descriptor::new_wpkh(key)?)
}

fn decode_sh_inner(cur: &mut Cursor, keys: &[DescriptorPublicKey])
    -> Result<Descriptor<DescriptorPublicKey>, Error> {
    let inner_byte = cur.peek_byte()?;  // peek_byte exists at cursor.rs:111-119
    match Tag::from_byte(inner_byte) {
        Some(Tag::Wpkh) => {
            cur.consume_byte();
            let key = decode_placeholder(cur, keys)?;
            Ok(Descriptor::new_sh_wpkh(key)?)
        }
        Some(Tag::Wsh) => {
            cur.consume_byte();
            let wsh = decode_wsh_body(cur, keys)?;
            Ok(Descriptor::new_sh_with_wsh(wsh))  // takes Wsh<Pk>, infallible
        }
        Some(other) => Err(Error::PolicyScopeViolation(format!(
            "v0.4 does not support sh({other:?}); only sh(wpkh(...)) and sh(wsh(...)) allowed"
        ))),
        None => Err(Error::InvalidBytecode {
            offset: cur.position(),
            kind: BytecodeErrorKind::UnknownTag(inner_byte),
        }),
    }
}
```

Top-level dispatch at `decode.rs:62-91`:

```rust
match Tag::from_byte(top_byte) {
    Some(Tag::Wsh)  => decode_wsh_inner(cur, keys),
    Some(Tag::Tr)   => decode_tr_inner(cur, keys),
    Some(Tag::Wpkh) => decode_wpkh_inner(cur, keys),  // NEW
    Some(Tag::Sh)   => decode_sh_inner(cur, keys),    // NEW
    Some(Tag::Pkh) | Some(Tag::Bare) => Err(Error::PolicyScopeViolation(
        "v0.4 does not support top-level pkh()/bare() (legacy non-segwit out of scope)".to_string()
    )),
    Some(other) => Err(Error::PolicyScopeViolation(format!(
        "v0.4 does not support top-level tag {other:?}"
    ))),
    None => Err(Error::InvalidBytecode {
        offset: 0,
        kind: BytecodeErrorKind::UnknownTag(top_byte),
    }),
}
```

### Default path-tier selection (`crates/md-codec/src/policy.rs`) — STRUCTURAL ADDITION (scoped)

The existing tier-1 logic at `policy.rs:390-397` falls through to `shared_path()` (extracts first key's origin) and hardcodes `m/84'/0'/0'` as the FINAL fallback when no origin is present. **This existing behavior is preserved unchanged for `wsh` and `tr` top-level types** to avoid wire-format-changing existing v0.3.x-shaped inputs (a wsh-no-origin policy that currently encodes with the BIP-84 fallback indicator MUST continue to do so, even though BIP 48/2' would arguably be the more natural default).

v0.4 adds a NEW per-descriptor selector helper that fires ONLY for the new top-level types (`wpkh`, `sh(wpkh)`, `sh(wsh)`):

```rust
/// Returns the natural default-tier indicator for v0.4-introduced top-level
/// types when no Tier 0 / Tier 1 path is available. Intentionally scoped:
/// existing wsh/tr types preserve their pre-v0.4 fallback behavior to avoid
/// changing the bytecode of v0.3.x-shaped no-origin inputs.
fn default_indicator_for_v0_4_types(d: &Descriptor<DescriptorPublicKey>) -> Option<u8> {
    match d {
        Descriptor::Wpkh(_) => Some(0x03),                  // BIP 84
        Descriptor::Sh(sh) => match sh.as_inner() {
            ShInner::Wpkh(_) => Some(0x02),                 // BIP 49
            ShInner::Wsh(_)  => Some(0x06),                 // BIP 48/1'
            _ => None,                                       // unreachable in v0.4 (rejected upstream)
        },
        Descriptor::Wsh(_) | Descriptor::Tr(_) => None,     // preserve pre-v0.4 behavior
        _ => None,                                           // unreachable in v0.4
    }
}
```

Wire this into the existing fall-through chain at `policy.rs:390-397` BETWEEN Tier 1 (origin-extracted) and the final BIP 84 fallback: if `default_indicator_for_v0_4_types` returns `Some(indicator)`, use it; otherwise fall through to the existing BIP 84 hard-coded fallback (which fires for wsh/tr no-origin cases unchanged).

**Regression test required**: `wsh_no_origin_default_unchanged_from_v0_3` — pin a wsh-no-origin policy's encoded bytecode at v0.3 and verify byte-identity at v0.4. Catches accidental scope creep of the new selector.

### CLI surface

`md encode <policy>` accepts the three new top-level types automatically (the encoder dispatches on policy type; no CLI parsing changes).

`--path bip48-nested` (NEW) — add to `bin/md/main.rs::NAME_TABLE` mapping `"bip48-nested" → 0x06` (and testnet variant `"bip48-nestedt" → 0x16`). Pure CLI ergonomics — wire format unaffected. Hex (`--path 0x06`) and literal-path (`--path "m/48'/0'/0'/1'"`) forms also work.

### Public API surface

No additions, no removals. `Error::PolicyScopeViolation(String)` is `#[non_exhaustive]` already; no new variants needed.

---

## §4. BIP Doc Changes

### Edit 1 — REPLACE existing scope paragraph at `bip/bip-mnemonic-descriptor.mediawiki:67-73`

Current "Format overview" prose ("script type is wsh() or tr()") becomes a forward reference: "*See §"Top-level descriptor scope" for the normative allow-list and §"FAQ" for design choices.*"

### Edit 2 — NEW normative section §"Top-level descriptor scope"

```mediawiki
==Top-level descriptor scope==

Conformant encoders MUST accept exactly the following top-level descriptor 
templates as input:

* '''<code>wpkh(KEY)</code>''' — BIP 84 native-segwit single-sig.
* '''<code>wsh(SCRIPT)</code>''' — native-segwit script (multisig and miniscript 
  script trees per BIP 388 wallet-policy grammar).
* '''<code>sh(wpkh(KEY))</code>''' — BIP 49 nested-segwit single-sig.
* '''<code>sh(wsh(SCRIPT))</code>''' — BIP 48/1' nested-segwit multisig. The 
  inner SCRIPT is subject to the same constraints as a top-level <code>wsh</code>.
* '''<code>tr(KEY)</code>''' or '''<code>tr(KEY, SCRIPT)</code>''' — BIP 86 
  single-key taproot, optionally with a single tap-leaf script. RESERVED — 
  admission deferred for: multi-leaf TapTree (bytecode tag <code>0x08</code>); 
  decoders MUST reject inputs containing tag <code>0x08</code> with 
  <code>Error::PolicyScopeViolation</code>.

Conformant MD encoders MUST reject the following BIP-388-permitted top-level 
forms; MD's accepted scope is narrower than BIP 388:

* '''<code>pkh(KEY)</code>''' — legacy P2PKH single-sig (BIP 388 admits; MD rejects)
* '''<code>sh(multi(K, ...))</code>''', '''<code>sh(sortedmulti(K, ...))</code>''', 
  '''<code>sh(KEY)</code>''' — legacy P2SH (BIP 388 admits; MD rejects)
* '''<code>bare(SCRIPT)</code>''' — pre-2014 raw script (BIP 388 also excludes; 
  MD inherits)

See §"FAQ: Why is MD narrower than BIP 388?" for the rationale.

Decoders MUST reject these forms with a structured error distinguishable from 
generic checksum failure and from generic bytecode-parse failure. The reference 
implementation surfaces all such rejections via 
<code>Error::PolicyScopeViolation(String)</code>; cross-implementations MUST 
provide a semantically equivalent variant. The diagnostic string SHOULD name 
the offending top-level type (or <code>sh(...)</code> inner type) so consumers 
can disambiguate "top-level rejection" from "sh-inner rejection" from 
"reserved-tag rejection".

All accepted forms follow the existing canonical form rules (no whitespace, 
<code>/**</code> expansion, ascending placeholder order, <code>'</code> for 
hardened). No type-specific canonicalization.
```

(Framing keyword distinction: **RESERVED** = admission deferred for future spec versions. **EXCLUDED** = no admission planned. **NARROWER THAN BIP 388** = explicit subset relationship.)

### Edit 3 — NEW normative subsection §"Sh wrapper restriction matrix"

```mediawiki
====Sh wrapper restriction matrix====

When a decoder consumes a top-level <code>sh</code> bytecode tag 
(<code>0x03</code>), the next bytecode tag determines admission:

{|
! Inner tag !! Inner type !! Action
|-
| <code>0x04</code> || <code>wpkh</code> || Decoder MUST recurse — admit as BIP 49 single-sig
|-
| <code>0x05</code> || <code>wsh</code> || Decoder MUST recurse — admit as BIP 48/1' multisig
|-
| Any other || any other || Decoder MUST reject with <code>Error::PolicyScopeViolation</code>
|}

Decoders MUST peek the inner tag BEFORE recursive descent into the sh subtree. 
This prevents recursion-bomb hostile inputs (e.g., <code>sh(sh(sh(...)))</code> 
of arbitrary depth) — same hostile-input class as the malformed-payload-padding 
defense in §"Payload".
```

### Edit 4 — Tag table inline "v0.4 disposition" column

Add a new column to the existing tag table at `bip/bip-mnemonic-descriptor.mediawiki:295-377`:

| Tag | Name | … | Disposition |
|---|---|---|---|
| `0x02` | Pkh | … | Top-level: REJECTED. Not used in any nested context in MD's accepted surface. |
| `0x03` | Sh | … | Top-level: ACTIVE per §"Sh wrapper restriction matrix". |
| `0x04` | Wpkh | … | Top-level: ACTIVE. As `sh(wpkh)` inner: ACTIVE. |
| `0x05` | Wsh | … | Top-level: ACTIVE. As `sh(wsh)` inner: ACTIVE. |
| `0x07` | Bare | … | Top-level: REJECTED. Not used in any nested context in MD's accepted surface. |
| `0x08` | TapTree | … | RESERVED — admission deferred (multi-leaf taproot). |

### Edit 5 — Extend §"Default derivation paths" table

```mediawiki
{|
! Top-level !! Default tier-1 indicator !! Default path
|-
| <code>wsh</code> || <code>0x05</code> || <code>m/48'/0'/0'/2'</code>
|-
| <code>tr</code> || <code>0x04</code> || <code>m/86'/0'/0'</code>
|-
| <code>wpkh</code> || <code>0x03</code> || <code>m/84'/0'/0'</code>
|-
| <code>sh(wpkh)</code> || <code>0x02</code> || <code>m/49'/0'/0'</code>
|-
| <code>sh(wsh)</code> || <code>0x06</code> || <code>m/48'/0'/0'/1'</code>
|}
```

### Edit 6 — NEW §"Frequently Asked Questions"

Full Q&A drafted at end of this spec doc as §FAQ. Inserted in BIP as peer to §Rationale.

### Edits 7-8 — TODO Phase 6 markers, version-qualifier cleanup

Same pattern as v0.3 rename Phase 2 → Phase 6:
- `<!-- TODO Phase 6 -->` at v0.2.json SHA reference (will change at regen)
- `<!-- TODO Phase 6 -->` at line 744 family-stable note (`"md-codec 0.3"` → `"md-codec 0.4"`)
- Drop `v0.4 decoders MUST` framing in normative MUST clauses — BIP IS the v0.4 spec, "Decoders MUST" suffices

### Edits NOT in scope

- §"Length envelope" math unchanged
- §"Status" preamble unchanged
- §"Bytecode layer" tag definitions unchanged (only Disposition column added)

---

## §5. Test Corpus + Hostile-Input Fixtures

### Positive corpora (added to `crates/md-codec/src/vectors.rs::POSITIVE_FIXTURES`)

| ID | Policy | Coverage |
|---|---|---|
| `S1` | `wpkh(@0/**)` | BIP 84 bare single-sig |
| `S2` | `wpkh(@0/**)` + fingerprints block | + origin record |
| `S3` | `sh(wpkh(@0/**))` | BIP 49 nested-segwit single-sig |
| `S4` | `sh(wpkh(@0/**))` + fingerprints block | + origin |
| `M1` | `sh(wsh(sortedmulti(1, @0/**, @1/**)))` | BIP 48/1' 1-of-2 minimal |
| `M2` | `sh(wsh(sortedmulti(2, @0/**, @1/**, @2/**)))` | BIP 48/1' 2-of-3 representative |
| `M3` | `sh(wsh(sortedmulti(2, @0/**, @1/**, @2/**)))` + fingerprints | + origins for all 3 |
| `Cs` | `sh(wsh(sortedmulti(2, @0/**, @1/**, @2/**)))` formatted as Coldcard export | Hardware-wallet shape parity |

`Cs` MUST cite a specific Coldcard firmware version source in its provenance comment (parallel to existing `coldcard` fixture pattern).

### Negative corpora — DECODE-side restriction matrix (one fixture per REJECTED row)

| ID | Bytecode shape | Expected error |
|---|---|---|
| `n_sh_multi` | `[Sh][Multi][...]` | `PolicyScopeViolation: "v0.4 does not support sh(<legacy P2SH>)..."` |
| `n_sh_sortedmulti` | `[Sh][SortedMulti][...]` | same |
| `n_sh_pkh` | `[Sh][Pkh][...]` | `PolicyScopeViolation: "v0.4 does not support sh(<inner type>)..."` |
| `n_sh_tr` | `[Sh][Tr][...]` | same |
| `n_sh_bare` | `[Sh][Bare][...]` | same |
| `n_sh_inner_script` | `[Sh][AndV][...]` | "sh(...) requires wpkh or wsh wrapper" |
| `n_sh_key_slot` | `[Sh][Placeholder][0]` | layering-invariant defense |
| `n_top_pkh` | `[Pkh][Placeholder][0]` | top-level rejection |
| `n_top_bare` | `[Bare][...]` | top-level rejection |

`n_sh_inner_script` and `n_sh_key_slot` CANNOT be constructed via the policy parser (rust-miniscript rejects them upstream). They MUST be hand-rolled bytecode buffers fed through `WalletPolicy::from_bytecode` directly. Same provenance class as existing `n12`/`n17`/`n30` (empty `input_strings`, "lower-level API" note in provenance recipe).

### Negative corpora — ENCODE-side restriction matrix (symmetric to decode)

Without symmetric encode-side tests, an implementation could regress the encoder to silently produce strings the decoder rejects (CI passes, prod breaks). Add parallel `encode_rejects_*` table:

| ID | Encoder input | Expected error |
|---|---|---|
| `enc_sh_multi` | `Descriptor::Sh::new_p2sh_multi(...)` | `PolicyScopeViolation` |
| `enc_sh_sortedmulti` | `Descriptor::Sh::new_sortedmulti(...)` | same |
| `enc_top_pkh` | `Descriptor::Pkh(...)` | `PolicyScopeViolation: top-level pkh()...` |
| `enc_top_bare` | `Descriptor::Bare(...)` | same |
| `enc_sh_via_inner_ms` | `Descriptor::new_sh(<arbitrary miniscript>)` (catches `ShInner::Ms(_)` arm) | `PolicyScopeViolation` |

### Hostile-input fixtures

Pattern matches v0.2.2 `MalformedPayloadPadding` test (`tests/conformance.rs::rejects_malformed_payload_padding`):

- `rejects_sh_recursion_bomb` — construct hostile `[Sh][Sh][Sh]...×100` with valid BCH polymod. Decoder MUST reject at depth 1, NOT panic.
- `rejects_sh_recursion_minimal` — `[Sh][Sh][Wpkh][Placeholder][0]` at depth 1. Confirms peek-before-recurse rejects ANY depth.
- `rejects_wpkh_trailing_bytes` — wpkh with garbage tail. Length contract trip.
- `rejects_sh_wpkh_trailing_bytes` — same for sh-wpkh.
- `rejects_sh_wpkh_non_placeholder` — `sh(wpkh(<not-Placeholder>))` distinct diagnostic test.

### Default-tier-selection tests (`crates/md-codec/src/policy.rs::tests`)

- `wpkh_default_tier_is_bip84` — `wpkh(@0/**)` no override → indicator `0x03`
- `sh_wpkh_default_tier_is_bip49` — `sh(wpkh(@0/**))` → `0x02`
- `sh_wsh_default_tier_is_bip48_nested` — `sh(wsh(sortedmulti(...)))` → `0x06`
- 3 override tests confirming `--path m/<custom>` flows through Tier 0 unchanged

### Round-trip property tests

`encode_then_decode_round_trip(policy)` for each new positive (S1-Cs). Bytecode-bytes assertions don't catch a bug where `to_bytecode` produces a value `from_bytecode` then rejects.

### Existing-corpus regression coverage — explicit gating (Section-5 acceptance criteria)

- `tests/vectors_schema.rs:225` — `V0_2_SHA256` constant updated to new SHA at v0.4.0 regen
- `tests/vectors_schema.rs:41-57` — `build_test_vectors_has_expected_corpus_count` hardcoded counts (`10` positive + `>= 18` negatives) bumped to ~18 + ~30
- `tests/vectors_schema.rs:300-334` — `schema_2_*_additions` tests verify schema-2-specific fields cover all new fixtures

### Test corpus growth — count (precise arithmetic)

- Decode-side restriction-matrix: 9 fixtures (n_sh_multi, n_sh_sortedmulti, n_sh_pkh, n_sh_tr, n_sh_bare, n_sh_inner_script, n_sh_key_slot, n_top_pkh, n_top_bare)
- Encode-side restriction-matrix: 5 fixtures (enc_sh_multi, enc_sh_sortedmulti, enc_top_pkh, enc_top_bare, enc_sh_via_inner_ms — `sh_pkh`/`sh_tr`/`sh_bare`/`sh_inner` not added because `ShInner` only has 3 variants and these are structurally unreachable)
- Hostile-input: 5 (rejects_sh_recursion_bomb, rejects_sh_recursion_minimal, rejects_wpkh_trailing_bytes, rejects_sh_wpkh_trailing_bytes, rejects_sh_wpkh_non_placeholder)
- Default-tier-selection: 6 (3 named-default tests + 3 override tests)
- Wsh/Tr regression preservation: 1 (`wsh_no_origin_default_unchanged_from_v0_3` — required per §3 default-tier scoping)
- Positive round-trip property tests: 8 (one per S1, S2, S3, S4, M1, M2, M3, Cs)
- Hash-pin / corpus-count infrastructure bumps: 3 (V0_2_SHA256, expected_corpus_count, schema_2_*_additions)
- **Total new tests: 9 + 5 + 5 + 6 + 1 + 8 + 3 = 37**
- **Final test count target: 565 + 37 = 602 passing** (was 565 at v0.3.0)

### Vectors file regeneration (Approach A — locked at brainstorming)

Extend `crates/md-codec/tests/vectors/v0.2.json` in place with new fixtures. Family token bumps `"md-codec 0.3"` → `"md-codec 0.4"`. New SHA. CHANGELOG documents the SHA migration.

---

## §6. Migration + Release

### SemVer + wire-format compatibility framing

v0.3.x → v0.4.0 follows pre-1.0 convention (second component is the breaking-change axis). But **wire-format compatibility semantics differ from v0.3's rename**:

- v0.3.0 was wire-format-INCOMPATIBLE: HRP changed; v0.2.x strings REJECTED
- v0.4.0 is wire-format-ADDITIVE: HRP unchanged; v0.4.0 accepts a STRICT SUPERSET of what v0.3.x accepts

Asymmetry: NEW decoders accept all OLD strings; OLD decoders reject NEW strings. Standard "additive feature" semantics.

### NO past-release deprecation banners

v0.3.0's banners on v0.2.x existed because v0.2.x was REPLACED. v0.4.0 ships no banners — v0.3.x is not replaced; it's a smaller-surface subset. Users can stay on v0.3.x indefinitely.

### `MIGRATION.md` new section `v0.3.x → v0.4.0`

```markdown
## v0.3.x → v0.4.0

v0.4.0 is wire-format-additive over v0.3.x. Three previously-rejected
top-level descriptor types are now accepted: `wpkh(@0/**)`, `sh(wpkh(@0/**))`,
and `sh(wsh(...))`. v0.3.x-produced strings continue to validate identically
in v0.4.0; v0.4.0-produced strings using the new types will be rejected by
v0.3.x decoders with `PolicyScopeViolation`.

1. **Cargo dependency**: bump `md-codec = "0.3"` → `md-codec = "0.4"`. No
   API changes; no library `use` statement updates needed.
2. **CLI**: `md encode <policy>` now accepts the three new top-level types.
   Existing `wsh(...)`, `tr(...)` invocations unchanged.
3. **Test vector SHAs**: `v0.2.json` SHA changes from `18804929…` (v0.3.x
   family-stable) to `<new SHA>` at v0.4.0 (one-time migration). Future
   v0.4.x patches will produce byte-identical SHAs (family token bumps to
   `"md-codec 0.4"`).
4. **No public API changes**: `MdBackup`, `EncodeOptions`, `WalletPolicy`,
   `Error::PolicyScopeViolation` all unchanged. `PolicyScopeViolation` simply
   fires for fewer inputs.
5. **CLI `--path` ergonomics**: new optional name `bip48-nested` maps to
   indicator `0x06` (BIP 48/1' nested-segwit multisig). Hex (`--path 0x06`)
   and literal-path (`--path "m/48'/0'/0'/1'"`) forms also work.
6. **Restriction matrix is normative**: hardware wallets and other implementers
   producing `sh(...)` strings MUST adhere to the §"Sh wrapper restriction
   matrix" in the BIP — `sh(multi(...))`, `sh(sortedmulti(...))`,
   `sh(pkh(...))`, etc. are permanently REJECTED.
```

### CHANGELOG entry template

```markdown
## [0.4.0] — <date>

The v0.4 release adds the three remaining post-segwit BIP 388 surface
descriptor types (`wpkh`, `sh(wpkh)`, `sh(wsh(...))`) per design at
`design/SPEC_v0_4_bip388_modern_segwit_surface.md`. MD remains narrower
than BIP 388 by design — see BIP §FAQ "Why is MD narrower than BIP 388?"
for the rejected-by-design types.

### Added — top-level descriptor types
- `wpkh(@0/**)` — BIP 84 native-segwit single-sig
- `sh(wpkh(@0/**))` — BIP 49 nested-segwit single-sig
- `sh(wsh(SCRIPT))` — BIP 48/1' nested-segwit multisig

### Wire format
- ADDITIVE expansion. v0.3.x-produced strings continue to validate identically.
- v0.4.0-produced strings using new types are rejected by v0.3.x decoders
  with `PolicyScopeViolation`.
- Restriction matrix on `sh(...)` admits only `sh(wpkh)` and `sh(wsh)`;
  legacy `sh(multi/sortedmulti)` permanently EXCLUDED (see BIP §FAQ).
- HRP `md`, header bits, tag space ALL unchanged from v0.3.

### Test vectors
- `crates/md-codec/tests/vectors/v0.2.json` regenerated with ~8 new positive
  + ~9 new negative fixtures. New SHA: `<value>`.
- Family token bumps `"md-codec 0.3"` → `"md-codec 0.4"`. v0.4.x patches
  will produce byte-identical SHAs.

### CLI
- `md encode <policy>` now accepts `wpkh`, `sh(wpkh)`, `sh(wsh)` policies.
- `--path bip48-nested` (NEW) maps to indicator `0x06`.

### Notes
- MSRV: 1.85 (unchanged)
- Test count: ~605 passing (was 565 at v0.3.0)
- Repository URL: unchanged
- Workspace `[patch]` block: unchanged (still waiting on
  apoelstra/rust-miniscript#1)

### Closes FOLLOWUPS
- `v0-4-bip-388-surface-completion` — this release.

### Files NEW FOLLOWUPS
- `v0-5-multi-leaf-taptree` (deferred BIP 388 surface item)
- `legacy-pkh-permanent-exclusion` (wont-fix)
- `legacy-sh-multi-permanent-exclusion` (wont-fix)
- `legacy-sh-sortedmulti-permanent-exclusion` (wont-fix)
```

### FOLLOWUPS housekeeping at release (ORDER MATTERS — file new entries BEFORE closing old)

1. **File first** (so closure note can cite real existing IDs):
   - NEW: `v0-5-multi-leaf-taptree` (v0.5+ tier) — `tr(KEY, TREE)` multi-leaf taproot, BIP 388 §"Taproot tree" expansion
   - NEW: `legacy-pkh-permanent-exclusion` (`wont-fix`) — with rationale cross-reference to BIP §FAQ
   - NEW: `legacy-sh-multi-permanent-exclusion` (`wont-fix`)
   - NEW: `legacy-sh-sortedmulti-permanent-exclusion` (`wont-fix`)
   - NEW: `bip48-nested-name-table-entry-followup` (only if NAME_TABLE update is deferred; otherwise close in same release)

2. **Then close the umbrella entry**:
   - `v0-4-bip-388-surface-completion`: close as `resolved <SHA>` with closure note: "Stated scope (wpkh + sh(wsh)) addressed; v0.4 also adds sh(wpkh) (BIP-388-required, omitted from original entry). Entry name imprecise — v0.4 is the modern post-segwit SUBSET of BIP 388, narrower than BIP 388 itself. Multi-leaf TapTree filed as new entry `v0-5-multi-leaf-taptree`. Legacy exclusions filed as `legacy-{pkh,sh-multi,sh-sortedmulti}-permanent-exclusion` (wont-fix)."

3. Existing open entries (slip-0173-register-md-hrp, bch-known-vector-repin-with-md-hrp, bip-preliminary-hrp-disclaimer-tension, etc.) untouched.

### Release sequence (parallels v0.3.0)

1. Implementation per §§2-5 in worktree branch `feature/v0.4-bip388-modern-surface`
2. Per-phase implementer + reviewer cycles via `superpowers:subagent-driven-development`
3. Final cumulative reviewer pass over branch diff
4. Bump `crates/md-codec/Cargo.toml` 0.3.0 → 0.4.0
5. CHANGELOG entry + MIGRATION section landed before tag
6. Annotated tag `md-codec-v0.4.0`
7. Push commit + tag
8. Watch CI 3-OS green
9. Draft GitHub Release `--prerelease --latest`
10. NO past-release deprecation banners
11. Close FOLLOWUPS per housekeeping above

---

## §FAQ (proposed for BIP §FAQ insertion)

### Why is MD narrower than BIP 388?

BIP 388 admits six top-level wallet-policy shapes: `pkh(KEY)`, `wpkh(KEY)`, `sh(wpkh(KEY))`, `wsh(SCRIPT)`, `sh(wsh(SCRIPT))`, and `tr(KEY[, TREE])`. It also admits `sh(multi(...))` and `sh(sortedmulti(...))` as legacy P2SH multisig forms. MD accepts a strict subset, rejecting:

- `pkh(KEY)` — legacy P2PKH single-sig
- `sh(KEY)`, `sh(multi(...))`, `sh(sortedmulti(...))` — legacy P2SH

**Rationale (three reasons, weighted):**

1. **Address-prefix ambiguity creates a recovery footgun.** Both `sh(multi(K, ...))` (legacy P2SH multisig, 2014–2017 era) and `sh(wsh(...))` (BIP 48/1' modern nested-segwit multisig) produce addresses with the `3...` prefix. A user looking at a `3...` address from cold storage often cannot disambiguate which descriptor type produced it. Engraving the wrong type would derive the wrong addresses on recovery — silent fund loss. MD eliminates this footgun by rejecting `sh(multi)` at the spec level.
2. **Engravable steel backup is overkill for legacy single-sig.** BIP 39 seed words alone are sufficient backup for `pkh(KEY)` wallets; the policy template adds no recovery information. MD's value proposition (engravable backup of policies hard to reconstruct) doesn't apply.
3. **No deployment of new pre-segwit wallets.** Modern hardware wallets (Coldcard, Trezor, Ledger) emit modern post-segwit forms by default. Realistic users wanting to engrave 2015-era cold storage are vanishingly few; most pre-segwit wallets have either been swept to native segwit or sit dormant.

The three reasons combine: a category with high footgun risk, low marginal recovery value, and negligible new deployment is permanently excluded by design. This is a deliberate narrowing of BIP 388, not a future deferral.

### Why is multi-leaf TapTree deferred (vs excluded)?

`tr(KEY, TREE)` with multi-leaf TapTree (BIP 388 §"Taproot tree") is a substantive feature requiring its own design pass: TapTree depth/balancing rules, leaf-wrapper subset re-evaluation (Coldcard tap-leaf subset is currently restrictive), per-leaf miniscript context validation. Deferring to a future MD revision (v0.5+) keeps the v0.4 release focused. Unlike the legacy exclusions above, multi-leaf TapTree IS planned admission; see FOLLOWUPS entry `v0-5-multi-leaf-taptree`.

### Why is `bare(SCRIPT)` rejected?

BIP 388 itself excludes `bare(SCRIPT)` (pre-2014 raw script output type, no address). MD inherits this exclusion.

### Why no inline xpubs (foreign keys)?

MD's `@i` placeholder framing assumes named keys provided separately at recovery time (BIP 388 wallet-policy structure). Inline xpubs (descriptor-codec reserved tags 0x24–0x31) would change the recovery model: the engraved card would carry key material, defeating the steel-engravable backup form factor's value (key material is in the seeds, not the engraved card). Deferred indefinitely; see FOLLOWUPS entry `p2-inline-key-tags`.

### What about `tr(KEY)` single-sig taproot?

Single-leaf taproot is supported (`tr(KEY)` and `tr(KEY, single-leaf-script)`). Multi-leaf TapTree is deferred (see "Why is multi-leaf TapTree deferred (vs excluded)?" above).

### Why is the HRP `md` rather than something more descriptive?

Matches the 2-character convention of BIP 93's `ms` HRP for codex32. Shorter HRPs leave more capacity in the BCH-bounded data part for policy bytecode. The earlier `wdm` HRP was renamed to `md` at v0.3.0 — see CHANGELOG.

### Why is the family-stable generator string `"md-codec X.Y"` rather than the full crate version?

Family-stability across patch versions: any v0.4.x build regenerating `v0.2.json` produces a byte-identical SHA, so downstream conformance suites pinning the SHA see no churn within a v0.4.x line. Patch-version traceability is preserved in `gen_vectors --output`'s stderr log. Family token bumps at minor version boundaries (v0.3 → v0.4 shifts `"md-codec 0.3"` to `"md-codec 0.4"`).

---

## Appendix: Per-section brainstorming review history

| Section | Reviewer | Verdict | Revisions folded |
|---|---|---|---|
| §1 Scope | none | approved | n/a |
| §2 Wire format | Opus 4.7 | APPROVED-WITH-REVISIONS | Critical (wsh body refactor) + Important (rejection wording, dispatch ordering, recursion-bomb) |
| §3 Implementation | Opus 4.7 | APPROVED-WITH-REVISIONS | Critical (ShInner has 3 not 4 variants; new_sh_with_wsh not new_sh_wsh) + Important (naming, structural addition for tier selector, error tone) |
| §4 BIP doc | Opus 4.7 | APPROVED-WITH-REVISIONS | Critical (replace lines 67-73, tr framing, PolicyScopeViolation ambiguity) + Important (rationale placement, RESERVED vs EXCLUDED keywords) |
| §5 Test corpus | Opus 4.7 | APPROVED-WITH-REVISIONS | Critical (encode-side coverage missing, n_sh_inner provenance) + Important (count recalibration, V0_2_SHA256 + corpus-count tests as gating, Cs realism) |
| §6 Migration | none + user redirect | approved-after-correction | FOLLOWUPS closure framing fix (BIP-388-narrower posture) + new FAQ section |

---

## Implementation handoff

This spec is ready for `superpowers:writing-plans` to produce a per-phase implementation plan. Suggested 11-phase plan structure with explicit dependencies:

- **Phase 0**: Pre-implementation refactor (`decode_wsh_inner` body/wrapper split per §2). MUST complete before any encoder/decoder change because Phases 1+2 both depend on `decode_wsh_body` existing.
- **Phase 1**: Encoder changes (per §3 encoder block). Independent of Phase 2 once Phase 0 lands.
- **Phase 2**: Decoder changes (per §3 decoder block). Independent of Phase 1 once Phase 0 lands.
- **Phase 3**: Default path-tier selector (per §3 policy.rs structural addition; SCOPED to wpkh/sh-wpkh/sh-wsh only). Depends on Phase 1 (encoder must accept new types before policy can encode them).
- **Phase 4**: NAME_TABLE addition for `bip48-nested` (per §3 CLI surface). Independent.
- **Phase 5**: BIP doc edits (per §4). Independent — can land before or after code phases.
- **Phase 6**: Test corpus expansion (per §5; both encode + decode sides; hostile-input + property tests + regression test). Depends on Phases 1, 2, 3.
- **Phase 7**: Vectors regeneration + SHA pin update + family token bump (per §5). Depends on Phases 1-4 being byte-stable.
- **Phase 8**: Cargo bump 0.3.0 → 0.4.0 + Cargo.lock refresh. Trivial; lands as part of release-prep.
- **Phase 9**: Documentation (CHANGELOG, MIGRATION) (per §6). Lands after Phase 8 so version numbers are concrete.
- **Phase 10**: FOLLOWUPS housekeeping (per §6 ORDER: file new entries first, then close umbrella).
- **Phase 11**: Final review + release sequence (per §6).

**Dependency summary**:

```
Phase 0 (refactor)
   ├──→ Phase 1 (encoder) ──┐
   └──→ Phase 2 (decoder) ──┤
                            ├──→ Phase 3 (selector) ──→ Phase 6 (tests) ──→ Phase 7 (regen) ──→ Phase 8 (bump) ──→ Phase 9 (docs) ──→ Phase 10 (FOLLOWUPS) ──→ Phase 11 (release)
                            ├──→ Phase 4 (NAME_TABLE) ─┤
                            └──→ Phase 5 (BIP doc) ────┘
```

Each phase passes 4 mandatory gates (build, test, clippy, fmt) per the v0.3 workflow precedent. Phase 7 also requires `gen_vectors --verify` PASS for both v0.1.json and v0.2.json.
