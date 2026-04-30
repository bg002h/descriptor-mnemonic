# Pre-Brainstorm: v0.13 wallet-policy-with-keys mode

Captured during pre-brainstorm conversation, 2026-04-30. This is the input to a
future formal brainstorm that will produce
`design/BRAINSTORM_v0_13_wallet_policy.md` and a SPEC + plan in the v0.11
mold.

## Goal

Extend md1 to encode BIP 388 wallet **policies** (templates + concrete keys),
not just templates. The wire must:

- preserve enough information to find the next address and sign all
  addresses;
- let users elide as much as possible to keep engravings small;
- keep "more effort = more information" as a graceful-degradation principle.

## The 16-cell design space

Per `@N` placeholder, four independently elidable axes:

| Axis | Assumed (compact) | Explicit (verbose) |
|---|---|---|
| `master_fp` | absent (signer guesses seed) | 4 bytes |
| `path-from-master` | BIP-canonical for top-level wrapper, coin=0, account=0 | variable BIP 32 path |
| `pubkey` | placeholder `@N` (receiver supplies) | concrete 65-byte xpub |
| `use-site path` | `<0;1>/*` (`standard_multipath`) | per-`@N` override |

2⁴ = 16 cells. Different `@N` may sit in different cells (per-`@N` granularity,
matching v0.11's TLV pattern).

## Locked design pillars

1. **Assumed-origin = BIP-canonical for the top-level wrapper.** Standard-map
   below; gap-filling open.
2. **16 cells, each axis independently elidable per `@N`.**
3. **Coin = mainnet (0) only as assumed.** Testnet/signet/regtest must encode
   origin explicitly.
4. **Account = 0 only as assumed.** Non-zero accounts must encode origin
   explicitly.
5. **xpub on the wire = 65 bytes** = 32 chain code + 33 compressed pubkey.
   - parent_fp dropped: cosmetic, not used by any restore/spend operation.
     Re-rooted xpubs (parent_fp=0) are interoperable with Bitcoin Core and all
     BIP 32 implementations.
   - depth / child_index implied by origin path.
   - version implied by network (mainnet locked).
   - 33-byte pubkey is the floor for general use — 32-byte x-only would break
     non-taproot BIP 32 child derivation (HMAC takes 33-byte input including
     parity).
6. **Optimize the wire for cell 7** (`explicit fp + explicit path + concrete
   xpub + assumed use-site`). This is the standard `[fp/path]xpub/<0;1>/*`
   shape and dominates real-world wallets. A dedicated cell-7-shaped per-`@N`
   block should pay no TLV framing overhead.

## BIP-canonical origin map (assumed `path-from-master`)

| Wrapper | Standard | Assumed origin |
|---|---|---|
| `pkh(@N)` | BIP 44 | `m/44'/0'/0'` |
| `wpkh(@N)` | BIP 84 | `m/84'/0'/0'` |
| `tr(@N)` (key-path only) | BIP 86 | `m/86'/0'/0'` |
| `wsh(multi/sortedmulti)` | BIP 48 script-type 2 | `m/48'/0'/0'/2'` |
| `sh(wsh(multi/sortedmulti))` | BIP 48 script-type 1 | `m/48'/0'/0'/1'` |
| `sh(sortedmulti)` legacy | — | **gap (open)** |
| `tr(@N, TapTree)` taproot multi | — | **gap (open)** |
| inside `TapTree` leaves | — | **gap (open)** |

## Open questions for the brainstorm

1. **Gap-filling for non-canonical wrappers** — accept "must be explicit" or
   define an ad-hoc convention? Three wrappers affected: legacy P2SH multisig,
   taproot script-path multi, taptree leaves.
2. **Cell-7 optimization mechanics** — dedicated per-`@N` packed block, or
   keep the orthogonal TLV decomposition with cell-7 as the implicit default
   (emit only deviations)?
3. **TLV vs mode-bit dispatch for "wallet-policy mode"** — header has 3
   reserved bits per D9 lock; do we burn one for "keys present" or use
   TLV-only signaling?
4. **Identity hash** — `WalletPolicyId` (deferred from v0.11 §4d) hashes the
   full keyed wire, distinct from `WalletDescriptorTemplateId` (γ-flavor) and
   `Md1EncodingId` (full-wire SHA-256[0..16]).
5. **Chunk-budget ergonomics** — a 2-of-3 multisig fully expanded is
   ~1900–2400 bits → 5–7 codex32 chunks. Worth flagging engraving friction
   for users.

## Cross-references

- BIP 32 (extended keys): https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki
- BIP 380 (descriptors, key-origin syntax): https://github.com/bitcoin/bips/blob/master/bip-0380.mediawiki
- BIP 388 (wallet policies): https://github.com/bitcoin/bips/blob/master/bip-0388.mediawiki
- BIP 389 (multipath descriptors): https://github.com/bitcoin/bips/blob/master/bip-0389.mediawiki
- v0.11 brainstorm §4d (WalletPolicyId deferral): `design/BRAINSTORM_v0_11_wire_format.md`
- Upstream miniscript blockers (will gate v0.13 if wallet-policy materializes
  through `miniscript::WalletPolicy`):
  - rust-miniscript#935 — hash-terminal support
  - rust-miniscript#936 — `template()` / `key_info()` accessors
  - rust-miniscript#934 — `set_key_info` AST-vs-placeholder ordering
