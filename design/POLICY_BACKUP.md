# Wallet Descriptor Mnemonic (WDM) — Design Document

**Status:** v0 draft. Architecture hypothesis only. No commitments. Decisions
flagged with **DECIDE** below; open questions flagged with **OPEN**.

**Format name:** **Wallet Descriptor Mnemonic** (abbreviated **WDM** when
context demands brevity; otherwise spell out in full).

**Purpose:** Define a human-transcribable, engravable-on-steel backup format
for bitcoin miniscript wallet policies. The format must back up the *structure*
of a wallet (the BIP 388 template + foreign xpubs) such that the wallet can be
fully reconstructed from steel-engraved artifacts alone, without requiring any
digital archive.

The Wallet Descriptor Mnemonic format is intended as a generic standard for
human-transcribable, engravable backup of bitcoin wallet policies. It is
designed to be implementation-neutral and adoptable by any miniscript-capable
wallet (Liana, Sparrow, Specter, Coldcard, others) and is a candidate for
BIP standardization.

---

## 1. Goals

1. **Engravable on steel.** A typical wallet's full backup must fit on
   commercially available steel backup plates (Cryptosteel, Blockplate,
   centerpunch + washer DIY, etc.). Single-plate fit for common cases.
2. **Hand-transcribable.** A user reading their steel can re-enter the backup
   into a tool by hand without ambiguity.
3. **Strong error correction.** Engraved metal experiences scratches, dents,
   oxidation, partial obliteration. Format must detect and correct realistic
   damage levels.
4. **Self-describing and versioned.** A backup found in isolation must be
   recognizable as a WDM-format policy backup of a known version.
5. **Separates structure from keys.** Seed-backed key material (BIP 39, etc.)
   handles personal xpubs. Policy backup handles the *script template* and
   *foreign xpubs* (xpubs not derivable from the user's seed).
6. **Lossless round-trip with BIP 388.** Any backup must encode a valid
   BIP 388 wallet policy and decode back to the identical policy string.
7. **Reference-implementable.** Encoding rules must be precise enough that
   independent implementations round-trip identical bytes.
8. **Adversarial-aware.** Threat model is explicit (§4). Format choices
   acknowledge that a finder learns *structure*, not *keys*.

## 2. Non-goals

1. **Not a key backup format.** Seeds are backed up by BIP 39 / SLIP 39 /
   codex32. We do not re-solve key backup.
2. **Not a transport format.** PSBTs, descriptor exchange, and inter-wallet
   communication use existing standards.
3. **Not optimized for arbitrarily large policies.** A 2 MB miniscript is out
   of scope. The format targets policies that compile to reasonable on-chain
   scripts (which already have practical size limits).
4. **Not encrypted by default.** The format is structural-information backup,
   not secret backup. Encryption may be optional but is not the primary mode.
5. **Not a wallet UI specification.** UX for verification, registration, and
   recovery is a separate concern (informed by but not part of this spec).

## 3. Threat model

### Actors and their capabilities

| Actor | Sees | Can do |
|---|---|---|
| Legitimate owner | All steel artifacts + their seed | Recover wallet, sign |
| Cosigner | Their own seed + Template Card + others' Xpub Cards | Sign their share |
| Finder (passive) | Whatever steel they happen to find | Read structure; cannot derive private keys |
| Coercer | Forces owner to reveal artifacts | Can recover wallet if all artifacts compelled |
| Inheritance party | Steel artifacts after owner's death; possibly delayed seed access via timelock | Recover wallet after timelock condition |

### What the format protects

- **Confidentiality of keys:** unconditional. The backup contains no private
  key material. Foreign xpubs are public-key data; their disclosure is not a
  break.
- **Integrity of structure:** ECC defends against transcription errors and
  metal damage. A corrupted backup either decodes correctly (within ECC
  tolerance), fails decode loudly, or signals "ambiguous; do not proceed."
- **Authenticity:** the Wallet ID (Tier 3) lets a user verify a digital bulk
  copy against their steel-stamped ID.

### What the format does NOT protect

- **Privacy of structure.** A finder of a Template Card learns the wallet's
  spending policy structure. For complex miniscripts this can reveal
  inheritance schedules, threshold counts, and the existence of cosigners.
  This is an accepted tradeoff: the backup must be readable to function as a
  backup. **OPEN:** consider an optional encrypted variant where the template
  is encrypted with a seed-derived key (loses some seed/policy separation but
  may be acceptable as an opt-in).
- **Coercion-resistance.** If an attacker compels disclosure of all steel
  artifacts AND the seed, the wallet is compromised. This is true of all
  backup schemes and not a goal here.
- **Non-malleability of the wallet itself.** If the user changes the policy,
  old backups become wrong. Wallet UX must enforce "policy is immutable once
  stamped" or provide explicit re-stamp flows.

---

## 4. Architecture hypothesis: three-tier backup

**Status: hypothesis. Pressure-test before committing.**

### v0 scope (decided 2026-04-26)

The v0 spec targets exactly one use case:

- **A single user** (one human; not a multi-party group)
- **Who owns every seed** referenced by the policy (no foreign xpubs)
- **All keys derive from a single shared path** (e.g., all `@i` use `m/48'/0'/0'/2'`)
- **All `@i` are structurally interchangeable** with respect to derivation —
  only the seed differs

Why this scope:

- It covers the largest real-world self-custody case (geographic-redundancy
  multisig with one owner)
- All xpubs are re-derivable from the owned seeds, so **Tier 2 (Xpub Cards)
  is empty in v0** — the Template Card alone suffices
- Single shared path means the path is encoded once, not per `@i` — meaningful
  size savings
- The spec is small enough to fully implement, test, and pressure-test before
  committing to broader cases

**Included in v0:**
- Multi-string chunking (so arbitrary-length policies are supportable, see §6.8)
- Single-string single-chunk format (the common case for typical policies)

**Deferred to v1+:**
- Foreign xpubs (cosigners whose seeds the user does not hold) — adds
  Tier 2 Xpub Cards
- Per-`@i` paths (mixing derivation paths within one policy)
- Mixed-source policies (some `@i` from seeds, some from hardware-only signers)

The encoding format is designed so that v0 backups are forward-compatible
with v1+ readers, and v1+ adds new opcodes/fields rather than changing v0
semantics.

### Tier 1 — Template Card (always required)

The BIP 388 template — script structure with `@i` placeholders, no keys —
plus origin paths for each placeholder. Encoded as compact bytecode + ECC,
rendered as a codex32-style string.

- Always required, regardless of wallet kind
- Single-plate target for common cases
- Independent of which keys are used

### Tier 2 — Xpub Cards (deferred to v1+)

Each xpub *not derivable from the user's own seeds* would be backed up on its
own small card.

- Empty in v0 (single user, all seeds owned)
- v1+: a cosigner gives you their xpub card; you back it up
- v1+: each card is independent and can be re-supplied by the originating
  cosigner

### Tier 3 — Wallet ID (derived, optional in v0)

A 12-word phrase derived from `hash(canonical Template Card || sorted Xpub Cards)`.
Stamped separately as a "this is wallet X" identifier.

- Allows verification of any digital bulk copy (load digital, hash, compare)
- Optional but recommended for users who want a redundant cloud/paper bulk copy

### Steel-surface estimates (v0 scope, single shared path)

These are first-cut estimates assuming the bytecode sketch in §6.1 and a
shared-path encoding (1 byte from a path dictionary). Refine after canonical
bytecode is implemented.

Format: bytes binary → codex32 chars (with ECC).

| Wallet kind | Without fingerprints | With fingerprints |
|---|---|---|
| `wsh(pk(@0/**))` (1 key) | ~6 B → ~10 chars | ~10 B → ~16 chars |
| `wsh(sortedmulti(2,@0,@1,@2))` (2-of-3) | ~14 B → ~22 chars | ~26 B → ~42 chars |
| Inheritance miniscript (3 keys, 1 timelock) | ~25 B → ~40 chars | ~37 B → ~60 chars |
| Complex miniscript (5 keys, 2 timelocks) | ~40 B → ~64 chars | ~60 B → ~96 chars |

**All cases fit single-plate** for typical commercial steel backup products
(which support 96–192 chars per plate).

**Tier 2 (Xpub Cards) is empty in v0.** All xpubs derive from owned seeds.

**Tier 3 (Wallet ID) is optional in v0.** A user with all seeds + Template
Card can recover without it; the Wallet ID is useful only if the user wants
to verify a digital bulk copy of the Template Card.

### Master fingerprint optionality

The Template Card MAY include a 4-byte master fingerprint per `@i`. Tradeoffs:

- **Without fingerprints:** 12 bytes saved (for 3 keys); restore-time tool
  must try each seed against each `@i` and assign by matching derived xpub.
  Computationally cheap; a few hundred milliseconds even for 10 keys.
- **With fingerprints:** Restore-time tool can directly identify which seed
  goes with which `@i`. Catches user error early ("this seed doesn't match
  any `@i` in this Template Card"). Costs 4 bytes per key.

**DECIDE:** Default to fingerprints **omitted** in v0. The space savings are
worth more than the convenience, and the restore-time matching algorithm is
trivial. Make fingerprint inclusion an optional flag in the Template Card
header for users who explicitly want it.

---

## 5. Encoding layer

**Decision (2026-04-26, after prior-art survey):** Build a codex32-derived
encoding with a WDM-specific HRP and header structure. Do NOT use
codex32 directly — its HRP is fixed to `"ms"` and its header semantics are
hard-coded for BIP 32 seeds (Shamir threshold, share index). Reuse the
cryptographic primitives only.

### What we adopt from codex32 (BIP 93)

- **Alphabet:** the bech32 32-character set
  (`qpzry9x8gf2tvdw0s3jn54khce6mua7l`). No visually confusable characters.
- **BCH polynomials:** both the regular (13-char checksum) and long
  (15-char checksum) polynomials.
- **ECC capabilities:** detect all errors affecting ≤8 chars; correct up
  to 4 substitutions or 8 erasures (regular) or 13–15 consecutive erasures
  (long).
- **General string structure:** HRP + separator + data + checksum.

### What we change

- **HRP:** proposed `"wdm"` (Wallet Descriptor Mnemonic). Alternative
  candidates: `"wp"` (wallet policy), `"dm"` (descriptor mnemonic).
  Final choice deferred until spec is mature enough to register.
- **Header semantics:** codex32's threshold/identifier/share-index fields
  do not apply. Replace with:
  - Payload version byte (1 byte)
  - Payload type tag (1 byte: Template Card v0, Wallet ID, etc.)
  - **OPEN:** wallet name field — included or separate?
- **Multi-string scheme:** required in v0. Single codex32 strings (with
  the 2-character slim header) cap at 48 bytes (regular) or 56 bytes
  (long) of bytecode. Arbitrary-length policies must be supportable.
  Each chunk is an independently-valid codex32-derived string with its
  own BCH ECC; chunks carry index + count + wallet-id for correct
  reassembly; a 4-byte canonical-bytecode hash appended before chunking
  provides cross-chunk integrity. See §6.8 for the chunk format.

### Payload size envelope

- v0 simple policies (1–3 keys, shared path, no fingerprints): 6–25 B →
  fits regular codex32 single-string
- v0 typical policies (3–6 keys): 25–60 B → fits regular or long
  single-string with slim header
- v0 complex policies (6+ keys, deep nesting, fingerprints): 60+ B →
  requires multi-string chunking (defined in §6.8)
- v1+ policies with foreign xpubs: each foreign xpub adds ~80 B →
  always multi-string

**DECIDE:** codex32-derived encoding is the encoding layer. BIP 39
4-letter prefix is no longer the conservative fallback — Bytewords + a
stronger external ECC layer takes its place if codex32-derivation proves
unworkable. See `PRIOR_ART.md` §3 for Bytewords rationale.

---

## 6. Sub-problems to solve, in order

### 6.1 Canonical template bytecode

**Decision (2026-04-26, after prior-art survey):** Build on
`descriptor-codec` (joshdoman, Rust, CC0). It already provides a
miniscript-complete tag table (50 tags, 0x00–0x31) and LEB128 path
encoding. We extend rather than reinvent.

#### Adopted from descriptor-codec

- **Tag table** for all miniscript operators, top-level wrappers, hashes,
  timelocks, key-script types, wildcards (see `PRIOR_ART.md` §2 for the
  complete list)
- **LEB128 varints** for path length and child numbers
- **Hardened/unhardened encoding:** child `c` encoded as `2c+1` hardened
  or `2c` unhardened
- **Tag-based prefix encoding:** root operator first, sub-expressions
  recursive

#### WDM-specific extensions

- **`@i` placeholder tag (proposed: 0x32).** Replaces the 0x26–0x2E key
  tags (which encode embedded keys) in our wallet-policy framing. Followed
  by a single byte `i` (0–255) identifying the placeholder.
- **Shared-path tag (proposed: 0x33).** v0 case where every `@i` uses the
  same derivation path. Encoded once at the start of the template; all
  `@i` references implicitly use it. Followed by a path-dictionary code
  byte (next item) or explicit LEB128 path.
- **Path dictionary (proposed: 0x34 + 1-byte code).** Standard paths
  (BIP 44 = 0x01, BIP 49 = 0x02, BIP 84 = 0x03, BIP 86 = 0x04, BIP 48
  multisig = 0x05, etc.). Codes 0xFE/0xFF reserved for "explicit LEB128
  path follows" and "no path."
- **Header byte (1 byte).** Payload version + Template Card flag in the
  upper/lower nibbles. Allows encoders to identify what they're decoding
  before parsing operator tags.
- **Optional fingerprints block (proposed: 0x35 + N×4 bytes).** When
  present, gives master fingerprints for each `@i` in order. When absent,
  restore-time tool matches seeds to placeholders by trial derivation.

#### Goals (revised)

- v0 typical templates (no fingerprints): 6–40 bytes binary
- v0 typical templates (with fingerprints): 10–60 bytes binary
- All v0 cases fit single codex32 string (regular or long)
- Round-trip with BIP 388 wallet policy string is lossless

**OPEN:** finalize the exact tag values for `@i`, shared-path,
path-dictionary, and fingerprints-block extensions. Coordinate with
descriptor-codec maintainer if upstreaming.

**OPEN:** decide whether to upstream the wallet-policy extensions into
descriptor-codec (preserving its CC0 license) or maintain a separate
fork in this repository. Upstreaming is more useful to the ecosystem.

### 6.2 Xpub card encoding

xpub (78 raw bytes) + origin info (fingerprint 4 bytes + path) → compact form.

- xpub raw bytes are not compressible (they're effectively random)
- Origin info benefits from path dictionary
- Per-card version byte and ECC

**Goals:**
- Single-card target: ~80 codex32 chars
- Round-trip with `[fingerprint/path]xpub` standard form

**OPEN:** whether to include the xpub's intended `@i` index in the card or
require ordering at decode time.

### 6.3 Wallet ID derivation

Canonical hash of (template bytecode || sorted-by-`@i` xpub cards) → 16 bytes
→ 12 BIP-39 words.

- Use SHA-256 truncated to 128 bits, or BLAKE3, or HMAC variant
- Sort key: `@i` index from the template
- Deterministic and reproducible across implementations

**OPEN:** hash function choice; word list (BIP 39 vs codex32-style).

### 6.4 Versioning scheme

- Version byte at start of each card type (Template Card v0, Xpub Card v0,
  etc.)
- Forward compat: decoders reject unknown versions loudly with a clear error
  ("This is a v2 Template Card; this software supports v0–v1")
- Backward compat: minor versions add fields with defaults; major versions
  break

**OPEN:** version space partitioning; whether Template Card and Xpub Card
share a version space or are independent.

### 6.5 Round-trip correctness with BIP 388

Implementation invariant: for every valid BIP 388 wallet policy, the encoded
bytecode decodes back to the *identical* policy string (canonical form).

Define a canonical form for BIP 388 wallet policies (whitespace, key
ordering, multipath shorthand expansion) and require both directions of the
codec to produce/accept canonical form.

**OPEN:** canonical form spec; tests for non-canonical inputs (reject vs.
normalize).

### 6.6 Adversarial analysis (formal)

Document precisely:

- What a Template Card alone reveals
- What a single Xpub Card alone reveals
- What the combination reveals
- What the Wallet ID alone reveals
- What recovery is possible from each subset of artifacts
- Privacy implications for inheritance / coercion scenarios

### 6.7 Reference implementation

A small library (language **DECIDE** but probably Rust or Python) with:
- Encoder: BIP 388 wallet policy → bytecode → one or more codex32 strings
- Decoder: codex32 string(s) → bytecode → BIP 388 wallet policy
- Multi-string assembly + cross-chunk integrity verification
- Wallet ID derivation
- Validator (catches malformed templates before encoding)
- Test vectors (round-trip, decode error cases, ECC stress, chunk
  reassembly stress including out-of-order and missing chunks)

This is the primary deliverable of the project per current scoping.

### 6.8 Multi-string chunking scheme

Required in v0. Supports arbitrary-length policies by splitting bytecode
across multiple independently-valid codex32-derived strings.

#### Chunk structure

```
wdm 1 <ver> <type> <wallet-id> <count> <index> <fragment> <checksum>
└──┘ │  1ch    1ch       4ch       1ch     1ch     N ch       13/15ch
HRP  sep
```

| Field | Length | Present in | Notes |
|---|---|---|---|
| HRP `wdm` | 3 chars | always | Proposed; final value DECIDE |
| separator `1` | 1 char | always | bech32 convention |
| version | 1 char (5 bits) | always | currently `0` |
| type | 1 char | always | `0`=single-string, `1`=chunked, more reserved |
| wallet-id | 4 chars | chunked only | random 20-bit per-wallet identifier |
| count | 1 char | chunked only | total chunks 1–32 (5 bits) |
| index | 1 char | chunked only | this chunk's index 0..count-1 |
| fragment | variable | always | bytes of canonical bytecode (full or partial) |
| checksum | 13 (regular) / 15 (long) chars | always | codex32 BCH ECC over this chunk |

For type=0 (single-string), wallet-id/count/index are absent.
Header overhead: HRP(2) + sep(1) + version(1) + type(1) = 5 chars.
Plus checksum = 18 chars (regular) or 20 chars (long).

For type=1 (chunked), full header present.
Header overhead: HRP(2) + sep(1) + 8 metadata chars = 11 chars.
Plus checksum = 24 chars (regular) or 26 chars (long).

#### Cross-chunk integrity

Before chunking, append a 4-byte truncated hash (BLAKE3 or SHA-256 —
**DECIDE**) of the canonical bytecode to the bytecode itself. This hash
is part of the byte stream that gets chunked.

After reassembling chunks, the decoder:
1. Concatenates fragments in index order
2. Splits off the trailing 4-byte hash
3. Recomputes the hash over the remaining bytecode
4. Rejects if mismatch

This catches: out-of-order reassembly that happens to pass per-chunk BCH;
missing chunks not detected by index gaps; chunks from a different wallet
mixed in (despite wallet-id check).

#### Capacity

| Code | Per-chunk fragment | Max chunks | Max policy bytecode |
|---|---|---|---|
| Regular | 45 bytes | 32 | 1436 bytes |
| Long | 53 bytes | 32 | 1692 bytes |

(Per-chunk fragment = ⌊(93 − 8 − 13) × 5 / 8⌋ = 45 bytes for regular;
⌊(108 − 8 − 15) × 5 / 8⌋ = 53 bytes for long. Max policy bytecode = 32 ×
fragment − 4 bytes for the cross-chunk hash.)

Realistic miniscripts top out at a few hundred bytes; typical wallets
fit in 1–4 chunks.

#### Wallet-id allocation

The 4-char (20-bit) wallet-id is generated randomly at first stamping.
With 2^20 ≈ 1M values, collision probability for a user with N wallets
is negligible. Wallet-id is per-wallet, not per-chunk.

For type=0 (single-string) Template Cards, no wallet-id is needed since
there's only one chunk.

**OPEN:** decide whether single-string Template Cards should also carry
a wallet-id for cross-format consistency (cost: 4 chars per single-string
Template Card).

**OPEN:** is wallet-id derived from Template Card content, or random?
Random has the property that re-stamping the same policy gets a fresh
ID; derived means same policy always produces same ID. Trade-off worth
discussing.

### 6.9 Compression rationale

**Decision (2026-04-26):** No explicit compression layer in v0. The
format relies on the inherent compactness of the bytecode + `@i`
indirection, not on a generic compression pass.

#### What's already compressed

The format provides large compression wins implicitly via its design
choices, before any "compression" in the conventional sense:

- **Key indirection (`@i` placeholders):** ~40× compression on key
  material vs. inline xpubs (2 bytes per reference vs. 78 bytes raw
  xpub). BIP 388 mandates this.
- **Operator opcodes:** 1 byte per fragment vs. plaintext keywords
  (8–12× compression on operator names).
- **Path dictionary:** 1-byte codes for standard paths vs. ~20 chars
  plaintext (~20× compression).
- **LEB128 timelocks:** 2–3 bytes vs. 4–7 char numeric strings.

#### Why no additional compression

1. **Key indirection captures the dominant repetition.** BIP 388 forbids
   true key reuse (malleability + privacy reasons), so the only legitimate
   "repetition" at the policy level is multiple references to the same
   `@i`, already optimally encoded as 1-byte indices.
2. **Payload sizes are too small for generic compression to win.** For
   7–60 byte payloads, LZ-family compressors yield 5–20% savings — often
   offset by dictionary/header overhead.
3. **Each layer adds attack surface.** Decompression bombs and parser
   bugs are a real source of vulnerabilities. The codec stays minimal.
4. **Constrained-device targets.** Coldcard and similar small targets
   parse these policies. Direct opcode interpretation is much friendlier
   than LZ decompression on memory-limited firmware.
5. **Chunking handles arbitrary length cleanly.** Compression's main
   benefit (avoiding chunk boundaries) is moot when chunking is already
   robust.

#### What we'd consider in v1.x if compression became necessary

In rough order of cost/benefit:

1. **Common-timelock dictionary** — 1-byte codes for frequent values
   (1 day, 1 month, 1 year, 2 years). Small spec change, ~1–2 bytes
   saved per timelock. Likely worthwhile if many policies use round
   timelocks.
2. **n-of-n implicit-thresh opcode** — compresses long `and_v` chains
   that bundle multiple keys with a timelock. Modest spec change,
   ~2–4 bytes saved per typical chain.
3. **Generic LZ on bytecode** — last resort. High complexity, marginal
   gain, real attack surface. Probably skip.

#### Forbidden non-compressions

- **Subtree deduplication:** literally identical subtrees would require
  reused keys, which BIP 388 prohibits. Therefore this optimization is
  unreachable for valid policies.
- **Huffman on operators:** ~25 operators yield log2(25) ≈ 4.6 bits;
  we use 5 bits. ~10% theoretical savings at high implementation cost.
  Not worth it.

This section exists to prevent re-litigation. If a future contributor
proposes adding compression, the burden of proof is to show: (a) which
v0 corpus example benefits by what amount, (b) how the spec change
preserves Coldcard implementability, and (c) that the attack surface
is acceptable.

### 6.10 Guided recovery (mandatory tool feature)

**Status:** part of the WDM v0 spec. Any recovery tool claiming WDM
compliance MUST implement guided recovery. This is not an optional
enhancement; it is a required feature of conformant decoders.

**Why required in spec:** the format's structured bytecode and `@i`
indirection enable substantially stronger recovery from heavily damaged
backups than the BCH ECC alone. If implementations don't expose this,
users with damaged plates will fail to recover wallets that the format
*can* recover. The format's robustness story depends on tools surfacing
this capability. Specifying it in-spec ensures all WDM tools provide
consistent recovery semantics.

#### Required user-facing flow

A conformant guided recovery tool MUST walk the user through these
steps (UI presentation may vary; semantic stages must be present):

1. **Damage assessment.** Accept the damaged codex32 string(s). Display
   per-position character status; let the user mark positions as
   unreadable (erasures).
2. **Standard decode attempt.** Try BCH decoding with no user-supplied
   information. If it succeeds, report results per §6.10.4 below. Stop.
3. **Erasure-aware decode.** If standard decode fails, use any
   user-marked erasures to attempt erasure decoding (up to 8 per
   string). Report.
4. **Structure elicitation.** If decoding still fails, prompt the user
   for what they remember about the policy:
   - Wallet kind (single-sig / multisig / inheritance / custom)
   - Number of keys
   - Threshold (if multisig or thresh)
   - Derivation path family (BIP 44/49/84/86/48/87 or custom)
   - Timelock values, if any (with common-value hints: "1 day = 144,
     1 month = 4380, 1 year = 52560")
   - Wallet ID, if separately stamped
5. **Constrained candidate search.** Encode the user's structural
   knowledge as a partial bytecode template. Enumerate candidate
   codewords within an extended Hamming radius (up to ~12) that:
   (a) decode to valid WDM bytecode (syntactic check), and
   (b) match the user's structural assertions.
6. **Candidate verification.** For each surviving candidate:
   - Derive xpubs from the user's available seeds at the candidate's
     declared paths
   - Check that derived xpubs map to the expected `@i` placeholders
   - If multi-chunk and chunks are present, verify cross-chunk hash
   - Reject candidates failing any check
7. **Optional: blockchain verification.** With user consent, derive
   the candidate's first N receive addresses and query the blockchain
   for transaction history. A match against expected on-chain history
   is the strongest available verification. Privacy implications MUST
   be disclosed before this step (see §6.10.6).
8. **Result presentation.** Show the recovered policy in BIP 388
   wallet-policy form, the wallet ID, derived xpubs, and a confidence
   indicator based on which verification steps succeeded.

#### Required decoder capabilities

Tools MUST implement:

- **Standard BCH decode** (regular and long codes) with reporting per
  §6.10.4
- **Erasure-aware decode** with user-supplied erasure positions
- **Known-position decode**: accept user-supplied confirmed-correct
  characters as anchors, with the decoder using them to extend
  correction capacity
- **Structure-aware candidate filtering**: enumerate codewords within
  an extended radius and discard those that don't parse as valid WDM
  bytecode
- **Cross-chunk reassembly** with hash verification (chunked Template
  Cards)
- **Per-step result reporting** so the user understands which level of
  recovery succeeded

#### Decoder reporting requirements

For every decode attempt, the tool MUST report:

- **Outcome:** clean / auto-corrected / erasure-corrected /
  structure-aided / failed
- **Error count and positions:** if errors were corrected, list them
  ("3 errors corrected at positions 14, 47, 52")
- **Correction method:** which technique succeeded (BCH alone / BCH +
  erasures / BCH + structure + erasures)
- **Verification status:** which post-decode checks passed (cross-chunk
  hash, seed-derived xpub match, blockchain history)
- **Confidence indicator:** explicit categorization, e.g.:
  - "Confirmed" = clean BCH + all verification checks pass
  - "High confidence" = some auto-correction + all verification checks
    pass
  - "Probabilistic" = structure-aided recovery + verification checks
    pass; recommend re-stamping
  - "Failed" = could not produce a verified candidate

#### Privacy and security obligations

The tool MUST:

- **Disclose privacy implications** before any blockchain query:
  "Querying address history will reveal which addresses you control to
  your blockchain data source. Use a private node or Tor for full
  privacy."
- **Not transmit recovered policy material** to any external service
  by default; all recovery operations run locally
- **Warn before re-stamping** if the recovery confidence is below
  "Confirmed" — partial recovery should not be trusted to produce a
  fresh canonical backup without independent verification
- **Default to offline operation** — blockchain verification must be
  opt-in per recovery, not enabled by default

#### Recovery confidence calibration

The tool MUST NOT report "Confirmed" unless ALL of:
- BCH decode succeeded with ≤4 errors corrected total
- Cross-chunk hash verified (if multi-chunk)
- All `@i` placeholders successfully matched against user's seeds at
  the recovered paths
- The user has explicitly confirmed the recovered policy structure
  matches their expectation

Lower confidence levels MUST be reported with specific qualifiers so
the user understands which guarantees did or did not hold.

#### Test vectors required

The reference implementation MUST include guided-recovery test vectors
that exercise:

- Heavy damage requiring erasure decoding (8 known-bad positions)
- Damage exceeding BCH bounds, recoverable via structure (10–12 errors
  with user-supplied template)
- Damage exceeding all algebraic recovery (BCH + structure both
  insufficient) — must fail loudly
- Multi-chunk recovery with one chunk damaged beyond per-chunk BCH
- Multi-chunk recovery with cross-chunk hash mismatch (must reject)
- Maliciously-crafted "valid-looking" damage attempting silent
  acceptance

These test vectors are part of the conformance suite for any
implementation claiming WDM compliance.

#### Why this matters

Without guided recovery:
- Damaged plates that *could* be recovered will be reported as
  "unrecoverable" by inadequate tools, leading to lost funds
- Different WDM tools could produce different recovery outcomes for
  the same damaged input, undermining the format's reliability
- The format's "10⁸× better than BIP 39" robustness claim depends on
  exposing the algebraic recovery capability to users

Specifying guided recovery in the spec ensures every WDM tool meets
the same recovery standard and that users have consistent expectations
regardless of which implementation they happen to be using.

---

## 7. Prior art

See **`PRIOR_ART.md`** for the full survey (populated 2026-04-26). Summary
of what was adopted vs. rejected:

- **Adopted (bytecode layer):** `descriptor-codec` (joshdoman, CC0). Tag
  table + LEB128 path encoding. Extend with BIP 388 placeholder tags.
- **Adopted (encoding layer):** codex32 (BIP 93) primitives — bech32
  alphabet + BCH polynomial. Replace HRP and header with
  WDM-specific framing.
- **Reference, not adopted:** BIP 39, SLIP 39 (seed-only); Bytewords / UR
  (weak ECC); Liana `.bed` (encrypted, not engravable); Coldcard multisig
  file (multisig-only, verbose); seedhammer PSBT-style (no ECC, heavier
  framing).
- **Conservative fallback:** Bytewords + external ECC layer if
  codex32-extension proves unworkable.

**Still to investigate** (deferred):
- Sparrow's wallet export format
- Specter's wallet config format
- Nunchuk's backup format
- Active bitcoin-dev discussions on descriptor encoding (post-2024)

---

## 8. Open questions / decisions deferred

### Resolved 2026-04-26 (after prior-art survey and recovery design)

- **RESOLVED:** Encoding base — codex32-derived (alphabet + BCH ECC),
  with new HRP and WDM-specific header. See §5.
- **RESOLVED:** Bytecode layer — extend descriptor-codec (CC0) with
  BIP 388 placeholder tags. See §6.1.
- **RESOLVED:** Master fingerprints — optional, default omitted in v0.
  See §4.
- **RESOLVED:** Multi-string chunking — required in v0 (not deferred).
  See §6.8.
- **RESOLVED:** Compression — none in v0; rationale documented to
  prevent re-litigation. See §6.9.
- **RESOLVED:** Guided recovery — mandatory in conformant tools, not
  optional. See §6.10.

### Still open

- **RESOLVED (2026-04-26):** Hash function for Wallet ID derivation is **SHA-256 truncated to 16 bytes**. Decision matches `bitcoin::hashes::sha256` (already a transitive dependency); avoids adding BLAKE3 or HMAC.
- **DECIDE:** Reference implementation language. Rust is leading
  candidate (descriptor-codec is Rust; rust-miniscript is Rust;
  rust-bitcoin is the reference ecosystem). Python second choice for
  reach.
- **DECIDE:** HRP value — proposed `"wdm"`. Alternatives: `"wp"`, `"dm"`.
  Defer until spec is mature enough to register.
- **DECIDE:** Whether to upstream BIP 388 extensions to descriptor-codec
  or maintain a fork.
- **DECIDE:** Whether to support an optional encrypted Template Card
  variant for users who want structural privacy.
- **OPEN:** Path dictionary — exact code values for BIP 44/49/84/86/48
  and any other "standard enough" paths.
- **OPEN:** Taproot tree encoding — descriptor-codec has a TapTree tag
  (0x08), but how it nests with `@i` placeholders for our wallet-policy
  framing needs design.
- **OPEN:** Threshold operators (`thresh(k,...)`) — encoding for arbitrary
  k and arbitrarily many sub-expressions.
- **OPEN:** MuSig2 — does the format support `musig(...)` placeholders,
  and if so, how is the participant set encoded? Coldcard does not
  currently support MuSig2 in miniscript, which may justify deferring.
- **OPEN:** Wallet name — is a human-readable name part of the Template
  Card payload, or stored separately?
- **OPEN:** Policy mutability — what's the spec story when a user
  changes a timelock by one block? Force re-stamp? Allow versioning of
  "the same wallet" across policies?
- **OPEN:** finalize tag values for chunk-header fields (version, type,
  wallet-id format, count/index encoding); see §6.8.
- **OPEN:** wallet-id — derived from content vs. random; whether
  single-string cards carry one.
- **OPEN:** cross-chunk hash function — BLAKE3 vs. truncated SHA-256.
- **OPEN:** Interop / BIP candidacy — pursue as a BIP from the start, or
  let the reference implementation mature first?

---

## 9. Glossary

- **Template** — the BIP 388 script template with `@i` placeholders, no keys
- **Wallet policy** — BIP 388 (template + key information vector + name)
- **Template Card** — engraved Tier 1 backup of the template + origin paths
- **Xpub Card** — engraved Tier 2 backup of one foreign xpub + its origin
- **Wallet ID** — Tier 3 derived 12-word identifier
- **Foreign xpub** — an xpub that is *not* derivable from the user's own seed
  (typically belonging to a cosigner)
- **Codex32** — BIP 93 encoding (bech32-style alphabet + BCH error correction)
  designed for hand-transcribed engravable bitcoin secrets
- **Canonical form** — the unique string representation of a wallet policy
  that the codec produces and accepts
- **Guided recovery** — the spec-mandatory tool flow (§6.10) that combines
  BCH ECC, erasure decoding, user-supplied structural knowledge, and
  seed-derived xpub verification to recover damaged backups beyond what
  the BCH ECC alone could correct
- **Structure-aware candidate filtering** — the technique of enumerating
  codewords within an extended Hamming radius and discarding those that
  don't parse as valid WDM bytecode; used in guided recovery
- **Recovery confidence** — categorical assessment of how trustworthy a
  recovered policy is, ranging from "Confirmed" (all checks passed) to
  "Probabilistic" (structure-aided, recommend re-stamping)

---

## 10. Next-session work queue (v0 scope)

Roughly in dependency order:

1. **Survey prior art** in §7 — read codex32 in particular, plus Liana's
   backup, Coldcard's multisig file, others. Output: a `PRIOR_ART.md`
   summary. Codex32 is the most load-bearing because it determines whether
   our default encoding layer is viable.
2. **Pick 4–6 example miniscripts** that match the v0 scope (single user,
   shared path, owned seeds) and represent the realistic complexity range
   from `pk(@0)` to a 5-key inheritance miniscript. Use these as the
   pressure-test corpus through every subsequent step.
3. **Draft the canonical template bytecode** (§6.1) for v0 scope. Define
   opcodes, varint rules, single-path encoding, path dictionary entries.
   Defer per-`@i` paths and foreign-xpub support to v1+. Produce concrete
   byte counts for the corpus from (2) and verify the size table in §4.
4. **Define Wallet ID derivation** (§6.3) — hash choice, word list choice.
   Optional in v0 but worth specifying now.
5. **Reference implementation** start — encoder/decoder for the bytecode
   layer, then wrap with codex32. Round-trip with BIP 388 wallet policy
   strings for the corpus.
6. **Guided recovery implementation** (§6.10) — known-position decoding,
   structure-aware candidate filtering, confidence calibration, all
   reporting fields. Required for spec conformance.
7. **Test vectors** — round-trip suite for the corpus. Include ECC stress
   tests (single-char and multi-char errors, recovery success rate) AND
   the guided-recovery test vectors enumerated in §6.10.
8. **Adversarial analysis** (§6.6) — formalize what a Template Card reveals
   in the v0 scope (no Xpub Cards to analyze).
9. **v1 scope spec** — extend the format for foreign xpubs and per-`@i`
   paths. Verify v0 backups still decode under v1 readers.
10. **Decision: BIP candidacy** — at this point we'll know whether the
    format is general enough to propose as a BIP or whether it stays
    WDM-specific.
