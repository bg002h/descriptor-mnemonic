# R0 Review — Cycle E bitcoind differential (round 4)

Reviewer: Fable 5 architect agent (a69759dab06fae73b), 2026-06-12.
Target: design/BRAINSTORM_stress_cycle_e_bitcoind_differential.md (R3 fold).
Persisted verbatim per CLAUDE.md convention.

## Verdict: GREEN

The two round-3 folds are clean, the env-var contract is consistent across §Home+shape, §CI, and §Oracle self-test, and the differential is already proven (20/20 byte-identical vs pinned v27.0, rounds 1-2). Final consistency pass. 0 Critical / 0 Important.

## Critical / Important
- (none)

## Minor
- (none new) — E3-m1 confirmed folded.

## Checks
1. **[E3-I1] RESOLVED.** `grep -n BITCOIND_BIN` over the whole spec → ZERO hits. The §Oracle self-test I3(b) fail-loud guard now keys off the three contract vars `BITCOINCLI_BIN`/`BITCOIND_DATADIR`/`BITCOIND_RPCPORT` SET + a `bitcoin-cli getblockchaininfo` failure → `panic!`. Green-by-skip hole closed.
2. **[E3-m1] RESOLVED.** `grep -n regtest` → 5 hits, every one a DEAD/rejected caveat or quoted error; the former stale-favorable §Recon line now reads "regtest is DEAD here, it rejects mainnet xpubs". No favorable framing survives.
3. **Final consistency — CONSISTENT.** Three env var names match across §Home+shape (names + test invocation), §Oracle I3(b), §CI (exports). Fail-loud trigger identical at the contract + I3(b). Datadir `$RUNNER_TEMP/bcd`, rpcport 18999, chain `-chain=main`, flags `-connect=0`/`-listen=0`/`-blocksonly=1`, cookie auth all match Home+shape↔CI. No contradiction.
4. **Implementer-ready — YES.** Home (md-codec tests/bitcoind_differential.rs, cfg(feature="derive") #[ignore]), wiring contract (3 vars, connect-only, skip/fail), corpus (10 proven shapes), oracle (per-chain to_miniscript_descriptor(d,chain).to_string() + range [0,N] + 3 self-tests), CI (pinned v27.0 + sha + offline -chain=main + triggers) all specified, zero open design decisions.

## Evidence log
- `grep -n BITCOIND_BIN` → no output (zero hits).
- `grep -n regtest` → 50/105/106/107/197, all DEAD-caveat or quoted-error.
- Env-var trace: BITCOINCLI_BIN/BITCOIND_DATADIR/BITCOIND_RPCPORT consistent at §Home+shape, §Oracle I3(b), §CI.
- Differential NOT re-run (proven 20/20 twice). Read-only; tree as found.

GREEN (0C/0I) — cleared for implementation.
