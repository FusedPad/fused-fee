# RUN10 binding fix

The run9 report at commit `399c08b` is an unsafe experimental record. Its 36 harness-complete rows were not 36 safety passes. Two of those rows recorded `binding_holds=false`: a token account owned by the DAMM pool authority, and not the configured auto-LP vault, received 250 on both reservation designs. Do not upload these hashes:

| Unsafe ELF | Bytes | sha256 |
| --- | --- | --- |
| run9 3Evo shared | 594040 | `9b903d8003a460cab19aa584c8774b9ac06c3e688dd741c2389cc5a68bc790a8` |
| run9 3Evo per-pool | 564320 | `1447e565641b329eddfe384fb0deb17a82a18e73d667e8157aff1b0050092996` |
| run9 experiment-id shared | 592760 | `1aa04cecefde0eb2ed3d6bfdca244bd754e7b395b8db98b6defdf3cfe5671c21` |
| run9 experiment-id per-pool | 563040 | `246f4e7b4cebf4e12516946163e6bd92bfab8fa56f7f5e78d8da5422dad4e398` |

Crank's devnet rent minima of 3.018602040 SOL and 2.867624440 SOL, from finalized slot 519505861, apply only to those 594040 and 564320 byte files. They do not apply to the binaries in this run. The 6960-lamport-per-byte formula remains invalid. These are not mainnet prices. Nothing was deployed or spent. Neither design is selected.

## This build

`cargo-build-sbf --arch v0 --tools-version v1.58 --offline`. Release profile on each experiment crate: overflow checks, fat LTO, one codegen unit, opt-level `z`. Anchor 0.31.1. No Anchor IDL. The account contract is `experiments/exports/run10-binding/INSTRUCTION-CONTRACT.json`.

| File | Program id | Bytes | Programdata bytes | sha256 |
| --- | --- | --- | --- | --- |
| `run10-3evo-shared-598008-3deff208a647b64d6fa5fa11a9060821cfacdd16b93f2b012327cb6d8c91ec03.so` | `3Evo…` | 598008 | 598053 | `3deff208a647b64d6fa5fa11a9060821cfacdd16b93f2b012327cb6d8c91ec03` |
| `run10-3evo-per-568552-60d036ae8a8a2fd0ffd92cba67378cfc9d6be795bb441434d07a9221d9ba9265.so` | `3Evo…` | 568552 | 568597 | `60d036ae8a8a2fd0ffd92cba67378cfc9d6be795bb441434d07a9221d9ba9265` |
| `run10-expid-shared-596728-2e631f6b54e7b718d21a9c4a4d7dbdcb8ec2bea3059f359ab35dcfbea4dd11cc.so` | `C1nBL…` | 596728 | | `2e631f6b54e7b718d21a9c4a4d7dbdcb8ec2bea3059f359ab35dcfbea4dd11cc` |
| `run10-expid-per-567272-af8e96981e01df03a658dc3fee4348ff25d1a5b75e01ec84bfceb1e753070e6f.so` | `C5hUU…` | 567272 | | `af8e96981e01df03a658dc3fee4348ff25d1a5b75e01ec84bfceb1e753070e6f` |

Do not upload until approved. The rent envelope for 598053 and 568597 byte programdata accounts has not been read. Keep the 0.5 SOL reserve. A buffer close returns the buffer only. These programs have no close for a quote reserve (82 bytes), an import marker (40), an LP batch (136-byte entries, account length 564), or an escrow, so that rent stays until a later close exists. Programdata still does not shrink.

## Binding

Open stores the destination vault. That vault's mint is the pool quote, its token owner is the DAMM pool authority, its pubkey is inside the configured DAMM pool account, and that pool account equals `AutoLpConfig.damm_pool` and is owned by the DAMM program. Consume pays only that stored vault, and it checks the same rule again. A rejected consume leaves the escrow, the burn pot, the holder pot, and the configured vault unchanged.

Executed on both designs, on the experiment ids and on the 3Evo id:

| Substitution | Result |
| --- | --- |
| User-owned token account | 6027 `BadDest` |
| Another account with the same DAMM-authority owner, not named in the configured pool | 6027 |
| Vault of the other quote | 6027 |
| A different DAMM pool account | 6027 |
| A different escrow | 6024 `BadVault` |
| The vault named in the configured pool | receives 500 per quote. This transfer is not add-liquidity and not a lock |
| Second consume, second refund, replay of an existing open | rejected |

Holder pots stayed 9000. The omitted pool stayed at holder 9000 and burn 2000.

## Auto-LP mock, separate from the reservation

`crank_auto_lp` still calls the local Meteora stand-in. With 24 pools present and 2 listed, the signed transaction is 24 locks, 1 signature, 872 message bytes, 937 signed bytes. Experiment-id CU is 97677 shared and 101620 per-pool. 3Evo CU is 103689 shared and 121144 per-pool. The unsent full list is 68 locks / 2389 signed bytes shared and 90 / 3115 per-pool.

## Fee mint, cursor, wSOL

Supported Token-2022 policy is unchanged: transfer fee at or below 100 bps inclusive, and rejection of an active hook (6029), a fee above 100 bps (6032), an unknown extension, a paused mint, default-frozen accounts, a non-transferable mint, and confidential mint or burn. Classic transfers use instruction 12. Token-2022 transfers use instruction 26/1 with the computed fee.

The mock claim now uses that Token-2022 instruction when the quote program is Token-2022. A 10000 gross claim at 100 bps:

| Design | Where the tokens land | Pots |
| --- | --- | --- |
| Shared | global vault 9900. Fee 100 withheld on that vault | holder 8100, burn 1800 |
| Per-pool | the sweep of 9900 assesses a second fee. Pool vault 9801. Global vault 0 | holder 8019, burn 1782 |

The seeded 1% pay remains a separate test. It does not replace this claim.

Pay v2, cursor opened at round 0, then one claim: the lower wallet receives 4500, a replay of that wallet while the pot still has funds pays it nothing more, and the higher wallet then receives 4500. The holder pot ends at 0. Same result on both designs and on 3Evo.

wSOL pay v2 unwrap pays 9000 lamports to a rent-exempt system account. Same result on both designs and on 3Evo. The quote mint is the native mint. The destination has to already be rent-exempt or the program carries the share forward instead of paying it.

## Counts

| File | Rows | Harness failures |
| --- | --- | --- |
| experiment-id `integrated-fee.json` | 17 | 0 |
| experiment-id `integrated-rest.json` | 42 | 0 |
| 3Evo shared core | 14 | 0 |
| 3Evo shared rest | 21 | 0 |
| 3Evo per core | 6 | 0 |
| 3Evo per rest | 21 | 0 |

The 42 rows include the binding rejections. They are safety checks with unchanged balances, not a repeat of the run9 observation that counted a failed binding as a completed row.

## Rollout facts for a later decision

The historical 8-account register still returns 6048 on both designs, including 3Evo. Shared register also needs the reserve or it returns 6050. Shared claims stay closed until the reserve is sealed (6052). A second import during backfill is 6053. Import after seal is 6052. A pool registered while the reserve is still in backfill blocks the seal until it is imported. Per-pool has no reserve. Its claim still arrives in the global ATA and is then moved, and on a fee mint that move charges a second fee. There is no account-close path for the new PDAs. The mock lock byte is still not a mainnet Meteora lock.
