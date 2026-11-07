# Register caller compatibility

This is local evidence. Nothing was deployed, spent, or sent to a public network. The saved builder audit in `experiments/evidence/WEB-BUILDER-AUDIT-20261107.md` is the pasted client report. It is not a rollout order. Section 8’s “program first” sentence is not adopted here.

The website tree was not modified. There is still one builder, `registerPoolInstruction`, and it still sends 8 accounts and 104 bytes. Account index 7 is the quote mint. In the program that account is `remaining_accounts[0]`, not an eighth named account. A 9th account, when a caller adds one, is `remaining_accounts[1]`, the QuoteStat PDA `["qstat", quote_mint]`.

## Binaries that were actually executed

| Label | What it is | ELF sha256 | Bytes |
| --- | --- | --- | --- |
| `base-80060ab` | Local SBF rebuild of source commit `80060ab46f77c6b46b4c9c1f65918ae9e356dbfc`. The string `quote_stat omitted at rem[1]` is in the file. | `3e98bf91e9a9ddca4e70f97f2c6ee8fe0f88f9c9c9a33e9d547ab799e543ddac` | 522384 |
| `pr-ae170b8` | Existing mainnet-id build already in `target/deploy/fused_fee.so`, matching the documented PR hash. Source tip `ae170b8cd7785fe80e32acea7ed08dee95e95e99`. | `1fdd85b47f76775c5937a21edf08e4e5db69b2c1f1df91185a653e53c8fa0968` | 522968 |

The baseline rebuild is not the deployed mainnet file. Deployed full ELF `2028226a2de10dbf2a1c8f9106d67f0ad3681ade7b4b2dcafba453ab5e1949bd` was not re-executed. The rebuild used platform-tools v1.58 and `anchor-lang` 0.31.1. The compile log also shows some Anchor helper crates at 0.31.2. Program id inside both files is mainnet `AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9`. Rows are in `experiments/results/register-caller-base-80060ab.json` and `experiments/results/register-caller-pr-ae170b8.json`.

## What each program does

`80060ab` register, source lines 147–182: if account index 7 is present it must be the quote mint. If a writable QuoteStat is also present, the program creates it at count 1 when it is empty, or adds 1 when it already exists. If it is omitted, register still succeeds and the counter stays absent.

PR `ae170b8` register, source lines 160–167 and 1376–1400: fewer than 2 remaining accounts returns `QuoteStatRequired` (6048) and creates no pool. An empty QuoteStat returns `QuoteStatUninitialized` (6049) and creates neither the pool nor the counter. An existing counter is incremented by 1. The program does not create the counter inside register. `sync_quote_stat` is the create/overwrite. It writes the absolute `u64` it is given. It does not add, and it does not count pool records.

## Executed matrix

`ok` is the transaction result. `pool` is whether the new pool record existed afterwards. Failures below left no new pool.

| Case | 80060ab rebuild | PR `ae170b8` |
| --- | --- | --- |
| 8 accounts, no QuoteStat | ok, pool created, counter absent. CU 21130 | fail 6048, no pool, counter absent. CU 22437 |
| 9th account, writable, empty | ok, pool created, counter created at 1. CU 31053 | fail 6049, no pool, counter still absent. CU 26628 |
| 9th account read-only | fail 6000, no pool, counter absent | fail 6000, no pool, counter absent |
| Index 7 is not the quote, only 8 accounts | fail 6000, no pool | fail 6048, no pool. This build stops on the missing 9th account before the mint check |
| Index 7 is not the quote, 9th account present, counter already 0 | fail 6000, pool absent, counter stays 0 | fail 6000, pool absent, counter stays 0 |
| Counter already 4, then 9th account | ok, pool created, counter 5 | ok, pool created, counter 5 |
| 8-account register, then sync to 0, then 9th account | first register ok and counter absent; after sync 0 the next register sets counter to 1 while two pools exist | the 8-account register never lands, so this sequence cannot create the first pool |
| sync to 7 with zero pools, then one register | ok, counter becomes 8 | ok, counter becomes 8 |
| sync of a new quote to 0 | ok, no pool, counter 0, 49 bytes | ok, no pool, counter 0, 49 bytes |
| register after that sync, second transaction | ok, counter 1 | ok, counter 1 |
| sync and register in one transaction | ok, counter 1. CU 47139 | ok, counter 1. CU 45635 |
| two registers, one transaction, same quote, counter starts at 0 | ok, both pools, counter 2. CU 50806 | ok, both pools, counter 2. CU 50798 |
| same, but the second register has the wrong base mint | fail 6000, neither pool, counter stays 0 | fail 6000, neither pool, counter stays 0 |
| 9th account is the wrong address | fail 6002, no pool, counter absent | fail 6002, no pool, counter absent |
| a 10th readonly account after a valid 9th | ok, ignored, counter 2 | ok, ignored, counter 2 |
| sync overwrite after the stale 1-vs-2 case | ok, counter set to 2 | not reached, because the 8-account setup did not land |

The 9th account is not rejected by the 80060ab source. The builder note that the old program rejects a 9th account does not match this rebuild. A read-only or wrong 9th account is rejected by both. An extra 10th account is not rejected by either when the 9th is valid.

A failed transaction rolls the pool and the counter back together. Two registers in one transaction share the QuoteStat account, run in order, and add 2. They are not parallel. A bad second instruction undoes the first. Solana account locking is what serializes two transactions that both write the same QuoteStat. This bank did not run two transactions at the same time.

Neither program recounts pool records. A counter at 7 becomes 8 after one register even if no pools existed. On the old source, an 8-account register plus a later sync to 0 leaves the counter at 1 after the next 9-account register, with two pools on that quote. Repair is another absolute sync. A wrong sync can also lower a correct counter. That is the same overwrite on both builds.

QuoteStat account data is 49 bytes (8 discriminator, 32 mint, 8 count, 1 bump). The “33 bytes” figure is the legacy message cost of one new key, measured below, not the account size.

## Message sizes

Local serialization with `solana_transaction` 2.2.3. Not broadcast. One signature unless the memo row, which has two. Distinct pubkeys, so nothing is merged away except the shared program ids inside a batch.

| Shape | Bytes | Unique keys in the message |
| --- | --- | --- |
| Register, 8 accounts | 505 | 9 |
| Register, 9 accounts | 538 | 10 |
| Difference | 33 | 1 |
| Compute price 100000 microlamports plus a limit, 8 accounts | 557 | |
| Same, 9 accounts | 590 | |
| Memo `fusedpad` plus price plus limit, 8 accounts | 697 | |
| Same, 9 accounts | 730 | |
| `sync_quote_stat(0)` and register in one legacy transaction | 593 | 10 |

All of those single-register shapes are under 1232. A DBC create plus first buy plus memo plus register was not serialized. That combined launch size is still unmeasured.

`/admin` as described sends one legacy transaction, no compute budget, one register per missing coin. Worst case measured here is a different quote for every coin:

| Coins | 8-account bytes | 9-account bytes |
| --- | --- | --- |
| 3 | 959 | 1058 |
| 4 | 1170 | 1302 |
| 5 | 1381 | 1546 |

Four 8-account registers fit. Five do not. Three 9-account registers fit. Four do not. Locks at those sizes stay under 64. The live admin path has no size check. This measurement does not change that path.

## What this does not decide

Shipping the PR program before the client changes makes every current 8-account register fail with 6048. The pool is not created. That is the measured result of “program first” against this PR binary. It is not a recommendation.

Sending a 9th writable QuoteStat while the 80060ab program is still the one running succeeds on this rebuild. An empty counter is created at 1. An existing counter increases by 1. The current client does not send that account. Adding it is a client change, and this file does not make that change.

The PR program will not create QuoteStat inside register. A new quote needs `sync_quote_stat` of 0 first. On this bank that sync and the register fit in one 593-byte legacy transaction. The API already uses a separate create transaction and a separate register transaction. Whether sync belongs in the register transaction is a later client decision. An existing quote with pools and no counter, which is the shape of the 6GmAFS group, must be given the real pool count by sync before any PR register. Sync to 0 there would record a lie. The PR register would then increment the lie. The old program’s 8-account path does not touch the counter at all.

Per-pool vaults and a shared aggregate are not in this commit. No prototype program was built or executed here. Both ideas need a writable account the current 8-key builder does not pass. This matrix shows the program cannot update an account the transaction omits: the old source leaves QuoteStat absent, and the PR source rejects the registration. That constraint is a prerequisite for either design. It is not a choice between them, and it is not a reason to change the production instruction yet.
