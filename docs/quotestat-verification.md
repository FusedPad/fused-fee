# QuoteStat registration fix

Verdict: **NOT READY for mainnet.** Do not merge this branch and do not deploy it.

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

## Hashes

| File | sha256 |
| --- | --- |
| `programs/fused_fee/src/lib.rs` | `b0612738206b48c929ff25068feac224f9cc23b1eec357ce176f8ff495d4a63d` |
| `programs/fused_fee/tests/runtime_suite.rs` | `ac001312892a57a2110fc23ee19bbda9d403732658c32b2eeb910003a77c2ef0` |
| `harness/mock_meteora/src/lib.rs` | `8ce1be67263c2e9bf29099698ba5e95479ff347bb734d683412cc38183071fb6` |
| `target/deploy/fused_fee.so` (521576 bytes) | `f11b273747e636b783ae0dacb97f50054ba39a4112eb0296ba003e663ff9b107` |
| `harness/mock_meteora/target/deploy/mock_meteora.so` (25936 bytes) | `950c41b6ddcb0d8c30e173e34c87ddc52c6aa5489658f55ffee90172c59d6d99` |

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
| Passing one pool twice satisfies the count and spends another pool's balance | PASS as a known gap |
| Omitting `CreatorOwed` lets that balance be treated as surplus | PASS as a known gap |

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
2. For each quote that already has pools, admin `sync_quote_stat(quote, exact_count)` of pools that already exist.
3. For a brand-new quote, admin `sync_quote_stat(quote, 0)` and then `register_pool` (the same transaction is enough).
4. Every later registration increments. Do not sync again unless the value is a fresh exact total. Sync overwrites. It does not add.

`/api/launch`, Matchbox, and the coin-page register path are the callers named in the previous source comment. They are not in this repository. This change does not edit them. Upgrading first will reject those launches with `QuoteStatRequired` or `QuoteStatUninitialized`.

## Economics

Not changed.

- Curve claim: burn = amount * 20 / 110, holders get the rest. 11000 became 2000 and 9000. That is the 0.90 / 0.20 split of a 1.10 partner claim. The 0.30 Meteora share and the curve's separate creator share are not paid by this instruction.
- Graduated claim with no creator LP: 8% of the claim is reserved for the creator, then 20/110 and 90/110 apply to the remainder. 11000 became creator 880, burn 1840, holder 8280. 880 is 8% of the claim. If that claim is 1.10% of volume, 8% of it is 0.088% of volume, not 0.10%. The code and its own comment describe 8% and 0.096% in different places. This patch does not pick a new formula.
- Auto-LP share stays 5000 bps of the burn pot, so half of the 0.20 burn share would be burn and half would be locked liquidity after `set_auto_lp`. The local test took 5000 from a 10000 burn pot and left the holder pot alone. `init_auto_lp` still leaves the DAMM target unset, and `crank_auto_lp` failed closed until `set_auto_lp`. Nothing in this patch turns that on.

## Other findings, not fixed

These are in the current program. They were not silently patched.

1. **Auto-LP pool list can repeat one pool.** `crank_auto_lp` counts accounts, not unique pool keys. The runtime test passed the same pool twice, matched `QuoteStat.pool_count`, and moved lamports that belonged to a pool that was never passed. Confirmed on this binary.
2. **Auto-LP does not require every CreatorOwed account.** An omitted owed balance was treated as surplus and could be spent. Confirmed on this binary.
3. **`release_burn_pot` checks only this pool's burn pot against the shared vault.** A 5000 burn was released from a vault of 8000 while another pool still had an 8000 holder pot. Confirmed on this binary.
4. **`set_paused` does not cover `register_pool`.** A registration succeeded while paused. Confirmed on this binary.
5. **`sync_quote_stat` can set any `u64`, including a lower one.** It is an admin overwrite. A wrong sync after the upgrade puts the counter back.
6. **`claim_creator_share` reserves this pool's owed + holder + burn, not every pool on the shared vault.** The source already says so. The new test covers the single-pool shortfall only.
7. **`crank_burn_direct` is classic SPL.** Token-2022 burns are not this instruction.
8. **`eligible_total` on holder pay is caller-supplied.** The crank is trusted. An inflated total underpays and can strand the remainder for that round.
9. **Devnet program id in source** (`8U45yvkgMohJBqCrd3uepXEsuVTfFY9Ak8Lu3ZU9Rm9F`) is not the historical devnet program (`3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`).

Auto-LP must stay off until the counter is fixed on the deployed program, the live counts have been synced and checked, findings 1 and 2 are fixed or explicitly accepted, and a separate authorization turns it on.

## Devnet rehearsal, not executed

Do not run this.

Public reads only, on 6 Nov 2026, no signature:

| Account | Observation |
| --- | --- |
| Mainnet program `AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9` | Executable, upgradeable loader. Programdata `2xjp8tUswiUWYzH5DkDn4zTm6ZRtA9z6Eq9Bx6bAbH7D` space 521125. |
| Historical devnet program `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` | Executable, upgradeable loader. Programdata `CeFRF4rmjsUh17dfHf835xP9d4yf5iHrFxb8hD3dpFqa` space 522085. |
| Source devnet id `8U45yvkgMohJBqCrd3uepXEsuVTfFY9Ak8Lu3ZU9Rm9F` | Program account exists. Its programdata account does not. Not a usable program. |
| Devnet wallet `HiRkRrxCgGzLV3WqKu7HxibS5kTVtDD6LW1vLKSH9Nxw` | 5073546881 lamports (5.073546881 SOL). The earlier note was 3.073546881 SOL. This read did not spend anything. |

This ELF is 521576 bytes. Upgradeable programdata for it is 45 + 521576 = 521621 bytes.

- Rent-exempt programdata is about 3.63137304 SOL, plus about 0.00114144 SOL for the program account. Steady total about **3.63251448 SOL**.
- If a buffer of that size and the programdata account exist at the same time, peak rent is about **7.26388752 SOL**.
- 5.073546881 SOL covers the steady figure and does not cover that peak. 3.073546881 SOL covers neither.
- Mainnet programdata is 496 bytes short of 521621. The extra rent is about 0.00345216 SOL, plus whatever buffer the upgrade holds.
- The historical devnet programdata (522085) is large enough for this ELF. That is not permission to write it, and the deployed bytes were not compared to this source.

Later rehearsal, only after a separate go-ahead and coordination with the crank:

1. Build with the command above and check the `.so` hash.
2. Confirm the devnet wallet can pay the buffer the CLI actually creates. Airdrop on devnet if it cannot. Do not use mainnet SOL.
3. On a throwaway devnet program, or only after an explicit choice to touch `3EvoY...`, deploy with a buffer and no `set_auto_lp`.
4. `sync_quote_stat` for every quote that already has pools, using a counted total.
5. Register one new pool with QuoteStat at rem[1] and check the count increased by 1. Repeat the same pool and check it did not.
6. Run one claim, one holder page, and one burn or release on devnet with amounts above the dust floor. The local suite is not that run.
7. Leave auto-LP unset.

No mainnet step belongs in that rehearsal.
