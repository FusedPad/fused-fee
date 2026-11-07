# RUN11 Crank handoff

This is a local handoff. Nothing was deployed, no devnet or mainnet transaction was sent, and no design was selected. Real Meteora `add_liquidity` and `permanent_lock` are still unproven: every auto-LP result in this repo calls `mock_meteora`, which only moves tokens and sets two bytes.

## Rent from Crank, slot 519510577

Attributed to Crank's read-only DEVNET RPC. This agent did not query the cluster. These lamports supersede earlier rent rows and apply only to the sizes he named. They are not mainnet prices, and they are not the rents for the run11 ELFs.

| Account | Bytes | Lamports |
| --- | --- | --- |
| Existing programdata `CeFRF4rmjsUh17dfHf835xP9d4yf5iHrFxb8hD3dpFqa` | 522085 | 2652842040 |
| c2083f7 shared programdata | 598053 | 3038759480 |
| c2083f7 per-pool programdata | 568597 | 2889123000 |
| c2083f7 shared buffer | 598045 | 3038718840 |
| c2083f7 per-pool buffer | 568589 | 2889082360 |
| Quote reserve | 82 | 1066800 |
| Import marker | 40 | 853440 |
| LP batch | 564 | 3515360 |

Upgrade authority `HiRkRrxCgGzLV3WqKu7HxibS5kTVtDD6LW1vLKSH9Nxw` balance 5073546881 lamports. Max extension from the existing programdata to the c2083f7 shared programdata is 385917440 lamports. The per-pool extension on that same pair of figures is 2889123000 − 2652842040 = 236280960 lamports. Opening both c2083f7 buffers at once is 3038718840 + 2889082360 = 5927801200 lamports, which is above the reported balance. He still keeps a 0.5 SOL reserve. This run spent nothing.

The run11 ELFs are larger because they add escrow close, existing-pool import, and the fee-mint migration refusal. Their programdata lengths are 609341 (shared) and 582285 (per-pool). Slot 519510577 does not price them.

## What is retained, by design

| Account | Shared | Per-pool | After a finished batch |
| --- | --- | --- | --- |
| Quote reserve, 82 bytes, 1066800 lamports | one per quote | none | kept |
| Import marker, 40 bytes, 853440 lamports | one per pool | none | kept |
| LP batch, 564 bytes, 3515360 lamports | one per batch id | one per batch id | kept on purpose |
| Escrow | one per mint in that batch | one per mint in that batch | closed when empty |

The executed escrow is 165 bytes and is owned by `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`. Crank's slot report does not include a 165-byte rent. A 1% fee-mint token account in the migration test is 178 bytes under `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`. That rent was not reported either.

A new reservation every six hours therefore keeps one 564-byte batch, 3515360 lamports, forever. Closing that account would let the same id be opened again, so `close_lp_escrow` does not close it. The escrow rent can come back. Tests: a stranger is rejected (6023); a close while the escrow still holds 250 is rejected (shared 6055, per-pool 6052) and the 250 stays; after consume, the admin close removes the escrow; a second close is rejected (6038); a later consume of that batch is rejected (shared 6054, per-pool 6051); the batch account is still 564 bytes and still program-owned.

Do not add the quote-reserve and import-marker rents to the per-pool design. Isolated pools have their own vaults and no `qres` or `qimp` accounts.

## Fee-mint migration

Old-layout accounts were written directly: pool record, creator-owed `cown`, and a global vault holding exactly holder 8000 + burn 1000 + owed 1000 = 10000 on a 1% Token-2022 mint.

Shared `import_existing_pool` books 8000 / 1000 / 1000 and leaves the vault at 10000. It does not move tokens, so it does not charge a transfer fee. Seal succeeds. A claim while the program is paused returns 6004 and leaves that vault and those pots unchanged.

Per-pool `migrate_pool_custody` on the same 10000 balance returns 6053 `MigrationFeeShort`. The global vault stays 10000, the pool vault stays 0, and holder, burn, and owed stay 8000 / 1000 / 1000. The program will not lower those liabilities to match what a fee-bearing transfer would deliver. Paying the extra fee so the pool vault still receives the full 10000 is a funding decision. It is not made here.

A classic mint with holder 9000 and burn 2000 moves all 11000 into the pool vault and leaves the pots at 9000 / 2000. After that move the global vault is 0. The current program, which signs only `["global"]`, cannot pay those pots. Rolling the program back after a per-pool custody move strands the tokens in the pool vault. There is no inverse instruction.

## Caller matrix, executed

| Call | Shared | Per-pool |
| --- | --- | --- |
| Builder register, 8 accounts, quote only | 6048 | 6048 |
| Quote plus an empty QuoteStat | 6049 | 6049 |
| Synced QuoteStat, no reserve | 6050 | register succeeds |
| Synced QuoteStat and an initialized reserve | register succeeds | reserve is not an account on this design |

Bootstrap for a new shared quote is `sync_quote_stat`, then `init_quote_reserve`, then register with both accounts, then `import_quote_reserve`, then seal. Pools that already exist use `import_existing_pool` instead, because the old program never incremented the reserve's pool count. A register during backfill still blocks seal until that new pool is imported.

Signed legacy sizes for the register instruction alone, one signature, measured in this run: 8 accounts 505 bytes, plus QuoteStat 538, plus QuoteStat and reserve 571. QuoteStat plus a separate Memo `"fusedpad"` signed by a second key is 678 bytes. All four are under 1232. The create-plus-first-buy transaction is not in that measurement. The saved builder audit names `@meteora-ag/dynamic-bonding-curve-sdk` 1.5.13, and this repo does not contain that package or a saved create transaction. That serialization cannot be reproduced here.

The builder's one-account edit, appending QuoteStat, matches the per-pool register. The shared design needs the reserve as a further account. That is a second client change, not covered by the one edit.

## Contention

A shared quote has one global vault and one `qres`. Every claim, pay, release, burn, and reservation for that quote writes both, so two such transactions cannot land in parallel. Two different quotes do not share those accounts.

A per-pool pay, release, or burn writes that pool's vault and not a quote-wide reserve. Two pools of the same quote can pay in parallel after custody has moved. A claim still writes the global vault first, so claims on the same quote still contend. The 24-account auto-LP measurement is the size of one transaction. It is not a throughput result.

## Meteora devnet

Public docs, not a chain read and not a transaction: [DAMM v2](https://docs.meteora.ag/developer-guides/damm-v2) and [DBC](https://docs.meteora.ag/developer-guides/dbc) publish the same program ids on devnet and mainnet, `cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG` and `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN`. The pool authorities on those pages are `HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC` and `FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM`. A later rehearsal can target those ids. This handoff did not, because that would be a devnet transaction. Until `add_liquidity` and `permanent_lock` run against those programs, the lock is unproven.

## Pinned run11 binaries

Build: `cargo-build-sbf --arch v0 --tools-version v1.58 --offline`, overflow checks, fat LTO, one codegen unit, opt-level `z`. Do not upload.

| Role | Bytes | sha256 |
| --- | --- | --- |
| 3Evo shared | 609296 | `659a899e2e6fdd1edd8dfec2a70a457c6f7d25d795155a7836a1f6751dfe5c93` |
| 3Evo per-pool | 582240 | `ee74870f323d797cb6d7b45aabaf4e7b7122333d27255cf14993a81f74f7f4c3` |
| Experiment-id shared | 608016 | `b0ff39459ece549ace130a9d99e93efdcdbe33d5d9aecda2d04ba378be34a030` |
| Experiment-id per-pool | 580960 | `47086d4d573f05e288196e3962e5c3baec5947004ac32b74ea7af46be8c9a467` |

Run9 hashes `9b903d80…` and `1447e565…` remain unsafe. Run10 priced the binding fix at 598008 and 568552 bytes; this ELF is newer. Machine rows are beside this file. The account contract is `INSTRUCTION-CONTRACT.json`. The lamport table is `RENT-ACCOUNTING.json`.
