# R0 Review — Cycle E bitcoind differential (round 3)

Reviewer: Fable 5 architect agent (acf1aa53bf7fcf384), 2026-06-12.
Target: design/BRAINSTORM_stress_cycle_e_bitcoind_differential.md (R2 fold).
Persisted verbatim per CLAUDE.md convention.

## Verdict: YELLOW

The differential is already proven (20/20 byte-identical vs pinned v27.0, rounds 1-2). The two round-2 Importants are substantially folded: E-I1's operational regtest residuals are gone from §Home+shape (now uniformly offline `-chain=main`, connect-only), and the E-I2 wiring contract is present and internally consistent between §Home+shape and §CI on datadir, rpcport, chain, flags, cookie auth, and all three env var names. BUT the fold introduced one env-var-name inconsistency in the §Oracle self-test that breaks the fail-loud guard's wiring (I3(b) read `BITCOIND_BIN`, a var CI never exports), plus a stale regtest mention in §Recon facts. One Important + one Minor; both folded post-review.

## Critical
- (none)

## Important
- **E3-I1 — `BITCOIND_BIN` at line 164 contradicts the three-var contract (fold-introduced typo; defeats the I3(b) fail-loud guard).** The E-I2 contract names exactly three vars (`BITCOINCLI_BIN`/`BITCOIND_DATADIR`/`BITCOIND_RPCPORT`) and CI exports exactly those; the fail-loud trigger is correctly stated at the contract as "the three env vars ARE set but bitcoin-cli doesn't answer." But the §Oracle self-test I3(b) restatement read "if `BITCOIND_BIN` (or the binary path) is provided…" — `BITCOIND_BIN` is in NO contract and NEVER exported (a round-1 leftover) → an implementer keying fail-loud off it would have the panic never fire → the green-by-skip hole I3(b) exists to close stays open in CI. Fold: rewrite I3(b) to key off the three contract vars + `bitcoin-cli getblockchaininfo` failure. [FOLDED]

## Minor
- **E3-m1 — stale regtest mention at §Recon facts** ("need a RUNNING bitcoind, but `-regtest` is instant") — pre-C1 framing presenting regtest favorably with no DEAD caveat. Not the operational E-I1 (no surviving "start -regtest"), but recon prose that could mislead. Fold: replace with the offline-mainnet rationale + the regtest-is-DEAD caveat. [FOLDED]

## Checks
1. [E-I1] regtest residuals — RESOLVED operationally; §Home+shape clean of regtest; lines explaining regtest is DEAD are fine; the lone stale favorable mention (recon) = E3-m1. [folded]
2. [E-I2] wiring contract — consistent + complete between §Home+shape and §CI (chain/datadir/rpcport/flags/cookie/3 env names all match; CI exports exactly what the test reads; connect-only unambiguous; skip/fail branches mutually exclusive). The §Oracle restatement broke env-var parity → E3-I1. [folded]
3. 3 minors — all present: E-m1 range `[0,N]` mandatory; E-m2 bare depth-0 xpubs; E-m3 qualified to_miniscript path + pinned bitcoin-cli not PATH.
4. NEW-ERROR sweep (§Home+shape + §CI): rpcport 18999 + datadir $RUNNER_TEMP/bcd clean; skip-vs-fail non-contradictory; only the §Oracle line-164 `BITCOIND_BIN` mismatch = E3-I1.
5. Completeness: after E3-I1 + E3-m1 fold, zero further design decisions.

## Evidence log
- grep regtest: §Home+shape (74-92) clean; only stale non-(a)/(b) = the recon line = E3-m1.
- Env-var trace: contract names 3 vars; line 164 read `BITCOIND_BIN` (never defined/exported) = the parity break.
- Cross-section wiring (Home+shape vs CI): chain=main, datadir=$RUNNER_TEMP/bcd, rpcport=18999, flags -connect=0/-listen=0/-blocksonly=1, cookie auth, 3 var names — all match.
- Differential NOT re-run (proven 20/20 twice). Read-only; tree as found.

Both findings folded post-review; round 4 = fast confirmation.
