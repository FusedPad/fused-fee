# QuoteStat registration fix

Verdict: **NOT READY for mainnet.** Do not merge this branch and do not deploy it. Real Meteora was not part of the local suite, and no devnet rehearsal was run.

## Live parity (read-only, 7 Nov 2026)

Evidence archive sha256 `6acb1067f76eedf743089d8b6bb38ff09ea83759765ca3f9d0db6e4a19cdbbad` matches the attached file. It contains no key, seed, or wallet.

Current mainnet program `AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9` was read from the public RPC. No transaction was sent.

| Check | Result |
| --- | --- |
| Programdata | `2xjp8tUswiUWYzH5DkDn4zTm6ZRtA9z6Eq9Bx6bAbH7D`, 521125 bytes, 2647965240 lamports |
| ELF `programdata[45:]` length | 521080 |
| Full ELF sha256 | `2028226a2de10dbf2a1c8f9106d67f0ad3681ade7b4b2dcafba453ab5e1949bd` |
| ELF with trailing zeros removed, sha256 | `35953c2c65ee8935823eaad0030bdc56fe675e67f9010e50fc21131e60f85a3d` |
| Upgrade authority | `D8pJVn2TTnEy4MVYQkPYyG4jS4oBb2CvrCq3jEET2rHx` |
| Last upgrade slot in the header | 463425198 |

Those two hashes are the 30 Oct OtterSec pair. `VERIFY-RESULT-20261030.json` records `is_verified: true` for commit `80060ab46f77c6b46b4c9c1f65918ae9e356dbfc` at 2026-10-30 21:43 UTC, with both hashes equal to `35953c2c…`. The chain still holds that bytecode. This branch's `.so` is a different file and is not deployed.

Source copies in the archive, compared with `git show 80060ab:programs/fused_fee/src/lib.rs` (`558c68ec6a287d271943e422186d0ec05b16140aa08ad6858612070de95fd366`):

| Copy | sha256 | Versus GitHub `80060ab` |
| --- | --- | --- |
| Nov 1 readonly clone | `558c68ec…fd366` | Byte-identical |
| Review fix-copy | `17a0be7c…c0c8` | One comment line. The devnet crank pubkey is the same. No logic change. The earlier "comment-only" note is true for this file. |
| `build-fix-mainnetid` | `3990a795…0325` | Not comment-only. About 312 lines removed and 46 added. It has no `QuoteStat`, no `sync_quote_stat`, and `claim_creator_share` only checks `owed <= vault` (the later reserved check is absent). Its `crank_auto_lp` does not debit `burn_pot` by `lp_share_bps` or require the pool set. |

`PERFECT-REPORT.json` hashes `b77c107a…` / `644a361f…` are an earlier local build. They are not the bytes on mainnet. The old upgrade plan also tells the crank to pin `b77c107a…` in one section and `2028226a…` in another. Only `2028226a…` / `35953c2c…` match the chain.

Live accounts derived for this program:

| PDA | State |
| --- | --- |
| `global` `F3swKL7FKLtNfo9ckummnkrjnoQna2UCGzAErx5VBcLU` | Present, 75 bytes, not paused, burn mint set |
| `autolp` `989UWgwA1mei9ZHet6PZfXRe7yiusUhgeMq1mrwqiZ9d` | Absent. Auto-LP has not been initialized. |
| wSOL `QuoteStat` `4oJkn2NHDc31zd8e9biikVEMD8n2wkaauWBBRuE98xyf` | Present, 49 bytes, mint wSOL, `pool_count` **13** |

So the duplicate-pool and omitted-owed paths are in the deployed source, and they are not callable today: there is no auto-LP account, and `crank_auto_lp` additionally requires the admin or the crank signer. `release_burn_pot` is on the live path and has no crank signer.

The devnet wallet balance of 5.073546881 SOL matches a later deposit. The earlier 3.073546881 figure was from before that deposit. It is not a conflicting read.

The program change is local only. No devnet or mainnet transaction was sent. Auto-LP is not armed.

## Commits

| Item | Value |
| --- | --- |
| Base | `80060ab46f77c6b46b4c9c1f65918ae9e356dbfc` |
| Branch | `cursor/quotestat-autocount-a4ab` |
| Base archive | `/opt/cursor/artifacts/fused-fee-base-80060ab46f77c6b46b4c9c1f65918ae9e356dbfc-source.tar.gz` |
| Base archive sha256 | `ef319d938800c274eaabad326303b79b067279d00be2e4458fe5f4d00a87758d` |

The archive is `git archive` of that base commit. It contains only the eight tracked source files. No keypair, seed, or env file was opened or copied.

Final tip SHA is the commit that adds this file. Read it with `git rev-parse HEAD` on the branch.

## What changed

`register_pool` still has the same seven named accounts. QuoteStat stays off that list, at `remaining_accounts[1]`, with the quote mint at `remaining_accounts[0]`.

A registration now succeeds only when that QuoteStat PDA already exists, is writable, is the PDA `["qstat", quote_mint]`, and can be incremented by exactly one. The increment happens in the same instruction as the new `PoolRecord`. If the account is missing, read-only, the wrong address, the wrong owner, the wrong mint, empty, or the counter would overflow, the instruction returns an error and the new pool is not committed.

`sync_quote_stat` is unchanged in layout and still sets an absolute count. It is how a quote is initialized:

- a quote with no pools yet: `sync_quote_stat(mint, 0)`, then each `register_pool` adds one
- a quote that already has pools: `sync_quote_stat(mint, exact_live_count)` before any further registration

An empty QuoteStat is not created at count 1. That would hide pools registered before the counter existed. The same pool cannot be registered twice, because `PoolRecord` is still `init`, so a retry does not increment.

Auto-LP targets are still unset after `init_auto_lp`. This patch does not call `set_auto_lp`.

Anchor-lang and anchor-spl stay on 0.31.1. The manifest pin is exact so the resolver cannot move to 0.31.2. New error codes are appended: `QuoteStatRequired` is 6048, `QuoteStatUninitialized` is 6049. Older codes are unchanged.

## Why the first diff was large

Against `80060ab`, the first commit touched 8 files and about +11601/−1462 lines. Almost all of that is generated or test-only:

| Path | Role |
| --- | --- |
| `Cargo.lock` | About +7068/−1416. Resolver output for `solana-program-test` and the Solana 2.3 crates the runtime suite links. Not hand-edited. |
| `harness/mock_meteora/Cargo.lock` | About 2009 lines. Separate lockfile for the Meteora stand-in. |
| `programs/fused_fee/tests/runtime_suite.rs` | About 2100 lines. The runtime suite. |
| `docs/quotestat-verification.md` | This note. |
| `programs/fused_fee/src/lib.rs` | The program change. The first commit was +97/−44. The auto-LP pair check is a further small edit on top. |
| `programs/fused_fee/Cargo.toml` | Dev-dependencies for that suite, plus an exact `=0.31.1` pin so the resolver stays on the already-used Anchor release. |

No formatting sweep was applied to the program. The lockfile churn is the test toolchain, not a logic rewrite.

## Hashes

| File | sha256 |
| --- | --- |
| `programs/fused_fee/src/lib.rs` | `ead75cedffef3ded7e693818b0ef4ea789177d623508e9400c480be72bdbcd5f` |
| `programs/fused_fee/tests/runtime_suite.rs` | `e86f3df3ac22ba5d4783ef1022d342a2a21e3bf3648bbc0feedb11c01b5802ee` |
| `harness/mock_meteora/src/lib.rs` | `8ce1be67263c2e9bf29099698ba5e95479ff347bb734d683412cc38183071fb6` |
| Mainnet-id `fused_fee.so`, 522968 bytes, not committed | full `1fdd85b47f76775c5937a21edf08e4e5db69b2c1f1df91185a653e53c8fa0968`; zeros trimmed `b62bccd7b2309c8162c5942e5d20de711f321a4a3605d4446b3e47f7c359af15` |
| 3Evo-id devnet-feature build, 522968 bytes, temp tree only | full `e22aa044bd7c6ef99173473a02100185a44674cd8f111689a5a7eb399576bb79`; zeros trimmed `3978f3ed0ee6cc3b770388b338c52c5f4d263d90e18ea7babcfe3c07ecadb5e2` |
| `mock_meteora.so`, 25936 bytes, not committed | `950c41b6ddcb0d8c30e173e34c87ddc52c6aa5489658f55ffee90172c59d6d99` |

The `.so` files are build outputs and are not committed. Rebuild them with the commands below and check these hashes.

Host tests used rustc 1.88.0. The SBF build used Agave `cargo-build-sbf` 2.3.13 and platform-tools v1.58.

## Commands

```
git rev-parse HEAD
git archive --format=tar.gz --prefix="fused-fee-80060ab46f77c6b46b4c9c1f65918ae9e356dbfc/" \
  80060ab46f77c6b46b4c9c1f65918ae9e356dbfc \
  -o fused-fee-base-80060ab46f77c6b46b4c9c1f65918ae9e356dbfc-source.tar.gz
sha256sum fused-fee-base-80060ab46f77c6b46b4c9c1f65918ae9e356dbfc-source.tar.gz

cargo test --manifest-path programs/fused_fee/Cargo.toml --lib
cargo-build-sbf --manifest-path programs/fused_fee/Cargo.toml --tools-version v1.58
cargo-build-sbf --manifest-path harness/mock_meteora/Cargo.toml --tools-version v1.58
cargo test --manifest-path programs/fused_fee/Cargo.toml --test runtime_suite -- --test-threads=1
```

`cargo-build-sbf` for the mock is run from `harness/mock_meteora` (that crate is its own workspace so it is not part of the program build).

## Results

Unit tests, host, against the library: **4 passed, 0 failed.** That includes the QuoteStat discriminator check, the instruction discriminator check, and the economics constants. These are not runtime proof.

Runtime suite, one local banks session per test, executing `target/deploy/fused_fee.so`: **3 passed, 0 failed.** SPL Token 3.5.0 and Token-2022 8.0.0 are the BPF builds shipped inside solana-program-test 2.3.13. DBC and DAMM are `mock_meteora.so`, not the Meteora programs. Play SOL only. No keypair file was read.

| Case | Result |
| --- | --- |
| Sync 0 then register, one transaction, count becomes 1 | PASS |
| Second pool increments by exactly 1 | PASS |
| Duplicate pool does not increment | PASS |
| Omitted QuoteStat (`QuoteStatRequired` 6048), pool not created | PASS |
| Old client shape (7 accounts + mint only) rejected the same way | PASS |
| Read-only QuoteStat rejected (`BadMint` 6000), count unchanged | PASS |
| Wrong PDA rejected (`QuoteMismatch` 6002) | PASS |
| Other mint's QuoteStat rejected | PASS |
| Extra trailing account does not double-count | PASS |
| QuoteStat placed at rem[0] rejected | PASS |
| Two registers in one transaction add 2 | PASS |
| Failed second instruction rolls the first pool and the count back | PASS |
| `sync_quote_stat` overwrites; the next register adds 1 to that baseline | PASS |
| Non-admin `sync_quote_stat` rejected, count unchanged | PASS |
| Non-admin `register_pool` still counts once | PASS |
| Count `u64::MAX` then register fails (`Math` 6006), pool absent | PASS |
| Empty QuoteStat rejected (`QuoteStatUninitialized` 6049) | PASS |
| Historical pool is not counted until `sync_quote_stat(exact)` | PASS |
| After that sync, the next register is exact+1 | PASS |
| Prefunded empty address is not a counter | PASS |
| Wrong mint bytes inside the right PDA rejected, count unchanged | PASS |
| System-owned garbage at the PDA rejected | PASS |
| Freeze-authority mint still registers | PASS |
| Token-2022 mint with no extensions registers and increments | PASS |
| Unknown extension, transfer fee above 1%, active hook rejected | PASS |
| `check_quote_mint` accepts the plain Token-2022 mint and rejects the unknown extension | PASS |
| Seeded 20-step fuzz: successes add 1, omissions and bad accounts do not | PASS |
| `set_paused` does **not** block `register_pool` | PASS as a known gap |
| Curve claim of 11000 splits holder 9000 / burn 2000 | PASS |
| `mark_graduated` before migration rejected | PASS |
| Direct burn of that 2000 debits supply and the burn pot; vault left 9000 | PASS |
| Holder pay of 9000 happens once; a second page is `EmptyPot` | PASS |
| A non-crank payer cannot pay | PASS |
| Burn when the vault is short is `VaultShort` and the pot stays | PASS |
| `release_burn_pot` can spend a shared vault past another pool's holder pot | PASS as a known gap |
| Graduated DAMM claim: creator 880, holder 8280, burn 1840 on a 11000 claim | PASS |
| DAMM claim without the 5 creator accounts rejected | PASS |
| Creator claim pays 880 once; a second claim is `NoCreatorShare` | PASS |
| Creator claim below the reserved vault is `VaultShort` | PASS |
| Paused program rejects holder pay | PASS |
| `crank_auto_lp` before `set_auto_lp` is `AutoLpNotReady` | PASS |
| Threshold above the burn budget rejected before the liquidity CPI | PASS |
| Add plus permanent-lock: half of each 10000 burn pot is taken, holders stay 5000, position marked added then locked | PASS |
| Pool list shorter than `QuoteStat.pool_count` rejected, pots unchanged | PASS |
| Passing one pool twice, or skipping its CreatorOwed PDA | PASS, rejected, vault unchanged |

## Not run

| Case | Why |
| --- | --- |
| `crank_pay_page_v3` Token-2022 transfer | Not executed. Token-2022 coverage is mint policy plus registration. SPL pay and burn used the real token program. |
| wSOL unwrap inside `crank_pay_page_v2` | The lamport path was not executed. The classic token pay path was. |
| Real Meteora DBC and DAMM bytecode | The stand-in transfers with SPL Token and writes a position flag. It does not run Meteora. |
| On-chain buy before auto-LP | There is no buy instruction in this program. |
| Devnet deploy, upgrade, or any signed transaction | Not allowed for this change. |
| Mainnet transaction | Not allowed. |
| Live website, crank service, or authority | Not in this repo and not touched. |

## Caller rollout

BLOCKER. Do not upgrade the live program until launch clients send the new accounts.

The seven named accounts and the instruction discriminator are unchanged, so an IDL that only lists those accounts does not need a new account order. Clients that build the instruction by hand must keep the quote mint at `remaining_accounts[0]` and append the writable QuoteStat PDA at `remaining_accounts[1]`.

They must also initialize that PDA first:

1. Halt launches, or ship the client and the program in the same window so an old client cannot register after the baseline is written.
2. For each quote that already has pools, admin `sync_quote_stat(quote, exact_count)` of pools that already exist. wSOL already has a QuoteStat and its `pool_count` is 13. Do not sync that mint to 0. Pass the existing PDA and let new registrations add to 13.
3. For a brand-new quote, admin `sync_quote_stat(quote, 0)` and then `register_pool` (the same transaction is enough).
4. Every later registration increments. Do not sync again unless the value is a fresh exact total. Sync overwrites. It does not add.

`/api/launch`, Matchbox, and the coin-page register path are the callers named in the previous source comment. They are not in this repository. This change does not edit them. Upgrading first will reject those launches with `QuoteStatRequired` or `QuoteStatUninitialized`.

## Economics

Not changed.

- Curve claim: burn = amount * 20 / 110, holders get the rest. 11000 became 2000 and 9000. That is the 0.90 / 0.20 split of a 1.10 partner claim. The 0.30 Meteora share and the curve's separate creator share are not paid by this instruction.
- Graduated claim with no creator LP: 8% of the claim is reserved for the creator, then 20/110 and 90/110 apply to the remainder. 11000 became creator 880, burn 1840, holder 8280. 880 is 8% of the claim. If that claim is 1.10% of volume, 8% of it is 0.088% of volume, not 0.10%. The code and its own comment describe 8% and 0.096% in different places. This patch does not pick a new formula.
- Auto-LP share stays 5000 bps of the burn pot, so half of the 0.20 burn share would be burn and half would be locked liquidity after `set_auto_lp`. The local test took 5000 from a 10000 burn pot and left the holder pot alone. `init_auto_lp` still leaves the DAMM target unset, and `crank_auto_lp` failed closed until `set_auto_lp`. Nothing in this patch turns that on.

## Holes: where they live, and who can hit them

Reproduced on the previous binary of this branch. `crank_auto_lp` in that binary is the same function as GitHub `80060ab`, which is the verified live source. The duplicate-pool and omitted-owed cases are therefore in the deployed program. They are not reachable until an admin creates the auto-LP account and sets a pool. That account is absent on mainnet.

| Hole | Where | Who | Impact | This branch |
| --- | --- | --- | --- | --- |
| Repeat one pool account so `QuoteStat.pool_count` matches while another pool is left out | Live source and the previous build. Dormant: no `autolp` account. Signer must be admin or crank. | Trusted operator, not a random wallet | Can spend another pool's tokens into LP | Rejected. Each pool key may appear once. |
| Leave out `CreatorOwed` so that balance looks like surplus | Same | Trusted operator | Can spend creator reserves into LP | Rejected. Every pool is followed by its `["cown", pool]` PDA. Empty means zero. |
| `release_burn_pot` compares only this pool's burn pot with the shared vault | Live source. Callable now. No crank signer. Destination must be the admin token account. | Any caller | Can send this pool's burn pot to the admin while another pool's holder pot is no longer covered. Not a transfer to the caller. | Not changed. A full fix needs every pool on a shared vault, which is a live crank change. Left as a decision. |

The 8% graduated creator share is unchanged. On a 11000 claim the program still books creator 880, holder 8280, burn 1840. If that claim is 1.10% of volume, 8% of it is 0.088% of volume, not a flat 0.10%. That difference is a product decision, not a silent edit.
4. **`set_paused` does not cover `register_pool`.** A registration succeeded while paused. Confirmed on this binary.
5. **`sync_quote_stat` can set any `u64`, including a lower one.** It is an admin overwrite. A wrong sync after the upgrade puts the counter back.
6. **`claim_creator_share` reserves this pool's owed + holder + burn, not every pool on the shared vault.** The source already says so. The new test covers the single-pool shortfall only.
7. **`crank_burn_direct` is classic SPL.** Token-2022 burns are not this instruction.
8. **`eligible_total` on holder pay is caller-supplied.** The crank is trusted. An inflated total underpays and can strand the remainder for that round.
9. **Devnet program id in source** (`8U45yvkgMohJBqCrd3uepXEsuVTfFY9Ak8Lu3ZU9Rm9F`) is not the historical devnet program (`3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`).

Auto-LP must stay off until the counter is fixed on the deployed program, the live counts have been synced and checked, findings 1 and 2 are fixed or explicitly accepted, and a separate authorization turns it on.

## Crank artifacts for a later rehearsal

Not deployed. The archived IDL at `mainnet-final-upgrade/fused_fee.idl.json` is the previous devnet-address IDL (`3EvoY…`). This branch does not regenerate that file. Account and instruction changes versus that IDL and versus live `80060ab`:

| Item | Change |
| --- | --- |
| `register_pool` named accounts | Unchanged. Seven accounts. Discriminator unchanged. |
| `register_pool` remaining | `[0]` quote mint, required. `[1]` writable QuoteStat PDA `["qstat", quote_mint]`, required, and it must already exist. |
| `sync_quote_stat` | Unchanged. Absolute `pool_count`. Seeds `["qstat", quote_mint]`. |
| `QuoteStat` layout | Unchanged: discriminator, mint, `u64` count, bump. 49 bytes. |
| `crank_auto_lp` tail | Was a list of pool records plus optional owed accounts. Now strict pairs: pool record, then `["cown", pool]`. Empty owed PDA means zero. Duplicate pool keys fail. |
| Errors | `QuoteStatRequired` 6048 and `QuoteStatUninitialized` 6049 appended. |
| Program id | Committed mainnet id stays `AZY2…`. Devnet feature id in source stays `8U45…`, which is not a usable program. A 3Evo rehearsal build has to set `declare_id` to `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` or the loader will reject it. |
| Hashes for the crank pin | Mainnet-id full `1fdd85b47f76775c5937a21edf08e4e5db69b2c1f1df91185a653e53c8fa0968`, trimmed `b62bccd7b2309c8162c5942e5d20de711f321a4a3605d4446b3e47f7c359af15`. 3Evo-id feature build full `e22aa044bd7c6ef99173473a02100185a44674cd8f111689a5a7eb399576bb79`, trimmed `3978f3ed0ee6cc3b770388b338c52c5f4d263d90e18ea7babcfe3c07ecadb5e2`. |
| `PROGRAM_SHA` | Do not pin the old `b77c107a…` or `644a361f…`. Those are not on mainnet. Live pin today is `2028226a…` / `35953c2c…`. |

QuoteStat counts registrations only. Claims and trades do not touch it.

The crank regression the owner listed (current crank first, then auto-LP, holder gate, DBC buy/burn, permanent lock, the 56 adversarial cases, dry-run with zero sends) needs the real crank tree and Helius. This cloud session did not run it. Local tests did not sleep to imitate RPC pacing.

## Devnet rehearsal, not executed

Do not run this.

Public reads only, on 6 Nov 2026, no signature:

| Account | Observation |
| --- | --- |
| Mainnet program `AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9` | Executable, upgradeable loader. Programdata `2xjp8tUswiUWYzH5DkDn4zTm6ZRtA9z6Eq9Bx6bAbH7D` space 521125. |
| Historical devnet program `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` | Executable, upgradeable loader. Programdata `CeFRF4rmjsUh17dfHf835xP9d4yf5iHrFxb8hD3dpFqa` space 522085. |
| Source devnet id `8U45yvkgMohJBqCrd3uepXEsuVTfFY9Ak8Lu3ZU9Rm9F` | Program account exists. Its programdata account does not. Not a usable program. |
| Devnet wallet `HiRkRrxCgGzLV3WqKu7HxibS5kTVtDD6LW1vLKSH9Nxw` | 5073546881 lamports (5.073546881 SOL). The earlier note was 3.073546881 SOL. This read did not spend anything. |

This mainnet-id ELF is 522968 bytes. Upgradeable programdata for it is 45 + 522968 = 523013 bytes. Rent below uses `(128 + len) * 3480 * 2` lamports.

Two different bills:

| Scenario | New SOL required | Fits in 5.073546881? |
| --- | --- | --- |
| Brand-new program: program account + programdata, no second buffer | about 3.6422028 | Yes |
| Brand-new program while a full buffer and the programdata account both exist | about 7.28326416 | No |
| In-place upgrade: existing programdata is extended, plus one temporary buffer of the ELF | extend is small; buffer is about 3.64 | Yes for the buffer. The extend is extra and tiny. |

Mainnet programdata is 521125 bytes, 1888 bytes short of 523013. The extend is about 0.01314048 SOL on top of the SOL already locked in that account (2.64796524 SOL). Devnet `3EvoY…` programdata is 522085 bytes, 928 bytes short of 523013, about 0.00645888 SOL to extend. It is not large enough for this ELF without an extend. The source devnet id `8U45…` still has no programdata; do not treat it as the rehearsal program.

The 3Evo hash above is a throwaway `--features devnet` build whose `declare_id` was edited to `3EvoY…` in `/tmp` only. That feature also selects the devnet crank pubkey. It is not the mainnet-id twin, and it was not deployed. A rehearsal binary has to use `declare_id` `3EvoY…` or the upgradeable program will reject calls. The committed source still declares mainnet `AZY2…` and the devnet feature id `8U45…`.

Later rehearsal, only after a separate go-ahead and coordination with the crank:

1. Build with the command above and check the `.so` hash.
2. Confirm the devnet wallet can pay the buffer the CLI actually creates. Airdrop on devnet if it cannot. Do not use mainnet SOL.
3. On a throwaway devnet program, or only after an explicit choice to touch `3EvoY...`, deploy with a buffer and no `set_auto_lp`.
4. `sync_quote_stat` for every quote that already has pools, using a counted total.
5. Register one new pool with QuoteStat at rem[1] and check the count increased by 1. Repeat the same pool and check it did not.
6. Run one claim, one holder page, and one burn or release on devnet with amounts above the dust floor. The local suite is not that run.
7. Leave auto-LP unset.

No mainnet step belongs in that rehearsal.
