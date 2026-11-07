# RUN12 corrected handoff

Program source is still `b3911bc32a1f92f3bfd9ae1a91b7a2856e0426dc`. The four ELFs were not rebuilt. Crank is pricing 620032 and 606400, and those are the files in this bundle. Nothing was deployed. Neither design is selected. `mock_meteora` is not Meteora `add_liquidity` and not `permanent_lock`.

The bundle file is `run12-corrected-handoff-bundle.tar`. It contains this note, `INSTRUCTION-CONTRACT.json` for the current account list, `MANIFEST.json`, `SDK-SIZES-3SIGNER.json`, the test rows, and the four `.so` files.

## SDK sizes, corrected

The earlier run12 figure signed the memo with the fee payer. That was two signatures. The saved builder adds Memo `fusedpad` with a distinct launch signer, and only when `txSize + 64 <= 1232`. That is a third signature. The launch signer is not writable.

Packages checked with `npm view` and installed in scratch: `@meteora-ag/dynamic-bonding-curve-sdk@1.5.13`, `@solana/web3.js@1.99.0`, `@solana/spl-token@0.4.14`. `createPool` and `createPoolWithFirstBuy` came from that SDK. `minimumAmountOut` is 0. Priority is `setComputeUnitPrice` of 100000 microlamports. The limit instruction is `setComputeUnitLimit` of 1400000, which is the builder cap, not a simulation. The instruction is 5 bytes, so a different unit count does not change the packet size. The builder drops that instruction if the legacy packet would exceed 1232. It was kept in every row below. Register is appended only when the packet fits in 1232−16, and in 1152 when a memo is present. It did not fit, so every launch below is two transactions.

This is a local fixture. The config account was encoded in this process. It is not a catalog config and it is not the live site. No transaction was sent.

| Case | Create packet | Signatures | Register packet |
| --- | --- | --- | --- |
| wSOL, first buy 100000000, shared or per-pool | 1125 | 3 | shared 623, per-pool 590 |
| wSOL, no first buy, shared or per-pool | 905 | 3 | shared 623, per-pool 590 |
| 6-decimal SPL fixture, first buy 1000000 | 1096 | 3 | shared 623, per-pool 590 |
| 6-decimal SPL fixture, no first buy | 905 | 3 | shared 623, per-pool 590 |

The rejected one-transaction size, wSOL plus first buy plus memo plus priority plus limit plus register, is 1426 shared and the same shape on a 6-decimal quote is 1364 per-pool. Both are over the memo budget of 1152. The three signers are the fee payer, the new base mint, and the launch signer.

## Safety review, no program change

Classic reverse with the creator account passed: holder 8000, burn 1000, owed 1000. The global vault started at 12000, so 2000 was not part of this pool. After migrate and reverse the global vault was 12000 again, the pool vault was 0, pots stayed 8000/1000, and owed stayed 1000. A different token account holding 7000 of the same mint was not touched. Reverse does not read or write any other pool. The global check is this pool's liability only. It does not prove the shared vault covers every pool on that quote.

Omitting the owed account is not safe for the creator. The same 10000 position reversed without the owed account put 9000 back in the global vault and left 1000 in the pool vault. Owed stayed 1000. The marker still flipped to reversed, and a second reverse returned 6056. Pass the `cown` account. The program was not changed, because Crank is pricing these bytes. Until it is changed, a reverse without `cown` is not a creator restore.

A 1% mint that holds exactly the liability still returns 6053 in both directions and moves nothing. Funding the fee so the destination still receives the full liability is an open decision. It is not taken from the pots.

Refund accepts any signer. The destination must be the source vault stored at open. A stranger refund returned 250 to that vault and the stranger's account stayed empty. Close is admin or crank only. A stranger close returned 6023 and the 564-byte batch stayed. Rent from a successful close goes to that admin or crank.

Two batches, 13 and 15, were open together. Closing 15 raised the one 17-byte watermark to 15. Id 14, which was never opened, was then rejected. Batch 13 was still open through that and was closed afterward. Reopening 13 was rejected. The watermark stayed 17 bytes. Closing a higher id retires every lower id. A lower batch that is still open can still be finished and closed. It cannot be opened again.

## Still unresolved

Fee-mint per-pool custody needs a separate funding decision. The program will not haircut holder, burn, or owed.

A shared quote writes one global vault and one `qres` on every claim, pay, release, and reservation for that quote. Those transactions cannot land in parallel. Two quotes do not share those accounts.

Per-pool pay writes that pool's vault. Two pools can pay in parallel after custody has moved. A claim still writes the global vault first, so claims on the same quote still contend.

The current 8-account register returns 6048 on both designs. An empty QuoteStat returns 6049. Shared register without the reserve returns 6050. Per-pool register with a synced QuoteStat succeeds and does not take a reserve.

Bootstrap for a new shared quote: `sync_quote_stat` with the absolute count, `init_quote_reserve`, register with QuoteStat and the reserve, `import_quote_reserve`, then seal. Pools that already exist use `import_existing_pool`, because the old program never incremented the reserve. The builder's one-account edit adds QuoteStat only. Shared register needs the reserve as a second new account. Open's account index 7 is now the watermark.

No devnet transaction was sent. The documented DBC and DAMM ids are still only public docs. A lock is not proven until those programs are called.

## Files and hashes

| File | Bytes | sha256 |
| --- | --- | --- |
| 3Evo shared | 620032 | `498233a22757af4fd5fc6e6fe05025f64b9ac278fc0691d03e86448c6f448c8e` |
| 3Evo per-pool | 606400 | `a0d1a6167cafd25f132878d83ddddc2067373c3f46d4e70d73db2509d2376723` |
| Experiment-id shared | 618752 | `24578a0e3d1b854d16c1558f11a3cb1c94ec142e0890d550a633a485cf11a45b` |
| Experiment-id per-pool | 605120 | `531e26506443b0d9591a004e42b3ce791dd6a2d852e4588139e4f9bf09ac661c` |

Run9 `9b903d80…` and `1447e565…` were not overwritten and remain unsafe.
