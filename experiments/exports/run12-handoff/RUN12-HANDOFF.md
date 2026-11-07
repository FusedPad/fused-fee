# RUN12 handoff

Superseded for SDK sizes and the account contract by `RUN12-CORRECTED-HANDOFF.md` in this directory. The memo measurement below signed with the fee payer. The builder uses a distinct launch signer. The ELF hashes are unchanged.



Local only. No deployment, no transaction, no architecture choice. Real Meteora `add_liquidity` and `permanent_lock` are still the mock. Run9 hashes `9b903d80…` and `1447e565…` are still unsafe and were not overwritten. Run11's files were left as they were. Slot 519510577 rents still apply only to the sizes Crank named. These ELFs are larger, so those programdata rents do not apply to them.

## Custody reversal

Classic per-pool custody can be reversed. A pool with holder 9000 and burn 2000 moved 11000 into the pool vault and left the pots at 9000/2000. A second migrate returned 6055 and moved nothing. `reverse_pool_custody` is signed by the admin, and the pool record PDA signs the token transfer. It put 11000 back in the global vault and left the pool vault at 0. The pots stayed 9000/2000. The global vault then covered the full liability. A stranger was rejected (6023). A second reverse was rejected (6056).

A 1% fee mint is not reversible by this instruction. Migrate of an exact 10000 liability still returns 6053 and moves nothing. A reverse attempted against a pool vault that already holds exactly 10000, with owed still 1000, also returns 6053. The pool vault stays 10000, the global vault stays 0, and holder, burn, and owed stay 8000/1000/1000. The program will not lower those figures to match a fee-bearing transfer. Funding extra tokens so the destination still receives the full liability is an open product decision. Until that decision, fee-mint per-pool custody is not reversible.

## LP batch rent

`close_lp_batch` closes a finished batch only when every escrow is empty. A close while an escrow still holds 250 is rejected and the 250 stays. On success the batch account is removed and one 17-byte `["lpnonce"]` watermark records the highest closed id. Opening that id again is rejected (shared 6056, per-pool 6054). The watermark stayed 17 bytes.

Crank's 3515360 lamports is the rent of one 564-byte batch while it exists. After `close_lp_batch` that rent comes back. What stays is the single 17-byte watermark. Its devnet rent was not in the slot 519510577 report. Skipping the close leaves the 564-byte account and its 3515360 lamports. That is the retained-record choice. The close is not available for an open or funded batch, so it is not a way to discard a live reservation.

## SDK sizes

`npm view` returned the published versions named in the builder audit: `@meteora-ag/dynamic-bonding-curve-sdk@1.5.13`, `@solana/web3.js@1.99.0`, `@solana/spl-token@0.4.14`. They were installed in a scratch directory. No site file was edited and nothing was submitted.

The measured transactions come from that SDK's `createPoolWithFirstBuy`, then a Memo of `fusedpad` signed by the same payer, then the experiment `register_pool`. The pool config is a local fixture labelled `LOCAL_FIXTURE_NOT_A_DEPLOYED_CONFIG`. Quote is wSOL. The first buy is 100000000 lamports with minimum out 1. This is the SDK's transaction shape. It is not a byte copy of the live site, because the live config account was not fetched.

| Design | One transaction | Create + first buy + memo | Register alone |
| --- | --- | --- | --- |
| Shared, 10 register accounts | 1286 bytes, 2 signatures, 9 instructions, 25 accounts, over 1232 | 977 bytes, 2 signatures, fits | 571 bytes, 1 signature, fits |
| Per-pool, 9 register accounts | 1253 bytes, 2 signatures, 9 instructions, 24 accounts, over 1232 | 977 bytes, 2 signatures, fits | 538 bytes, 1 signature, fits |

The one-transaction launch does not fit. The two-transaction fallback does: create, first buy, and memo in the first, register in the second. Both signatures on the first transaction are the payer and the new base mint.

## Next devnet steps

These are rehearsal steps, not an upload approval.

1. Crank re-reads rent for the run12 programdata lengths, the 17-byte watermark, and the 165-byte Tokenkeg escrow. Do not reuse slot 519510577 for the new ELF sizes.
2. On devnet, send the two-transaction launch above against the documented DBC program `dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN`. Keep the register on the second transaction.
3. A later auto-LP rehearsal has to call the documented DAMM program `cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG`. The local mock is not that call and is not a permanent lock.
4. For a classic per-pool rehearsal, `reverse_pool_custody` is the restore path. Do not use it as a fee-mint restore. That path is blocked until someone funds the fee without taking it from the pots.
5. The LP crank calls `close_lp_batch` after the batch is consumed or refunded. Leaving it open keeps 3515360 lamports per batch.

## Pinned run12 binaries

Build: `cargo-build-sbf --arch v0 --tools-version v1.58 --offline`. Do not upload.

| Role | Bytes | sha256 |
| --- | --- | --- |
| 3Evo shared | 620032 | `498233a22757af4fd5fc6e6fe05025f64b9ac278fc0691d03e86448c6f448c8e` |
| 3Evo per-pool | 606400 | `a0d1a6167cafd25f132878d83ddddc2067373c3f46d4e70d73db2509d2376723` |
| Experiment-id shared | 618752 | `24578a0e3d1b854d16c1558f11a3cb1c94ec142e0890d550a633a485cf11a45b` |
| Experiment-id per-pool | 605120 | `531e26506443b0d9591a004e42b3ce791dd6a2d852e4588139e4f9bf09ac661c` |

New instructions: `close_lp_batch` disc `[77, 158, 87, 217, 88, 244, 144, 86]`, `reverse_pool_custody` disc `[135, 74, 192, 108, 126, 50, 79, 186]`. Open now takes the 17-byte watermark at account index 7. The result rows and the four `.so` files are beside this note.
