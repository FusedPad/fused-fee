# Mainnet create: 56 Token-2022 (token_type = 1) FusedPad DBC configs, 17 Nov 2026

Operator approval was given on 17 Nov 2026 at 12:31 Europe/Jersey (UTC+0). This receipt covers the first 56 pairs only; the SOL/wSOL pair followed at 13:13 (see sol-config-receipt.md).

## Result

- **56 of 56 non-SOL pairs PASS.** None were skipped and none failed. ANSEM (Token-2022 quote, no TokenBadge) went through without a badge account. BONK used the 16-weight curve fallback and two transactions (create, then update_config), because the atomic tx was 1,273 B, over the 1,232 B limit.
- Signer: admin D8pJVn2TTnEy4MVYQkPYyG4jS4oBb2CvrCq3jEET2rHx on the operator machine. The key was referenced only by path. It was never printed, copied or hashed.
- **D8pJ balance:** 3.230195030 SOL before (12:39:31) and **2.894992746 SOL after** (12:54:13; checked again on Helius at finalized).
- **Total spent: 0.335202284 SOL.** That's 56 × ~0.0059856 SOL (5,974,080 lamports rent per 1,048 B config, plus fees with a 20,000 µlamport/CU priority).
- Window: test pair USDC 12:39:31–12:39:43, then 36 pairs (until PYTH) by about 12:47, then the last 19 at 12:49:30–12:54:13, all Europe/Isle_of_Man (UTC+0). The middle run stopped between pairs when the shell session was cut off. No pair was left half-sent. The final run resumed from the receipts and used the operator's pacing rule (3 s after each tx, plus 10 s after every 5). The first 37 pairs ran with about 1.2 s between RPC calls, before that rule was applied.

## Build (box, fresh)

- Prices: Jupiter v3 at 17 Nov 2026 12:36:10 Europe/Isle_of_Man, cross-checked with DexScreener. All gaps were under 1.1%. LINK was cross-checked against any pool, because it has no ≥$20k pool. Decimals and the mint program were read from chain.
- Curve: the house builder `buildCurveWithMarketCap` (SDK 1.5.13), $4,000 open and $9,000 raise, R = 3, so a $36,000 graduation MC. token_type = 1, DAMM v2, migration fee option 6, migrated fee 150 bps, dynamic fee 0, 1.5% fee (cliff numerator 15,000,000), creator 8%, 100% partner permanent-locked LP, Immutable, fee claimer and leftover = Global F3sw….
- All 56 had an unsigned mainnet simulation of [CU, create_config, fused_fee update_config]. The decoded account passed the rule, and the non-curve house fields equalled each pair's live config (only token_type and the curve differ).

## Rule enforcement (script refuses otherwise)

At build time, and again at send time with a fresh Jupiter price: open between $3,000 and $5,000, raise ≥ $8,000, **locked LP after Meteora's 0.2% migration fee ≥ $16,000**, and price drift ≤ 5% since the build. Send-time ranges across the 56: open $3,964.75–$4,073.42, raise ≥ $8,920.69, locked LP ≥ $17,805.70.

## Verification

- Laptop, per pair: the signed simulation had to give bytes identical to the box-verified bytes. After confirmation, the on-chain config bytes had to match, and the ConfigRecord had to be repointed.
- Box, independent read on **Helius at finalized** (`helius-verify.json`): 56/56 PASS. It checked owner DBC, 1,048 B, full byte match, token_type = 1 and byte 237 = 1, quote mint, fee claimer and leftover = Global, DAMM v2, fee option 6, 150 bps, dynamic fee 0, 100% partner permanent lock with 0 other LP, sqrt_start and threshold, and that the ConfigRecord points at the new config.

## fused_fee registration

This follows the existing flow (22 Oct BONK/ORE/SHIB, devnet 3GCb). Each quote's ConfigRecord already existed, so `update_config` repointed it to the new config. It ran atomically with create_config (BONK: a separate second tx). No `register_config` or `sync_quote_stat` was needed: all 56 ConfigRecords and QuoteStats already exist, and no new fused_fee accounts were created.
- `register_pool` does not require the ConfigRecord's current target (it records the pool's real config), so launches on the old configs still register.
- The crank only reports old configs as "not current target" (BLOCK_AUTO_OPEN_NONCURRENT_CONFIG=false), so pay and burn are unaffected.
- The SOL ConfigRecord 3ZgxFFgN… still points at Fdz9izTj… (unchanged).

Not touched: the website / QUOTE_CONFIGS, the crank and its flags, the live crank server, the upgrade authority, the SOL pair.

## Pairs (send-time price)

| Pair | Quote dec | New config | Open | Raise | Locked LP | Laptop / Helius |
|---|---|---|---|---|---|---|
| AAPLx | 8 | `Gxztpc4NjYJtpY1n8xJry87dXfDe9dFiXXk8caSEMZwd` | $4,003.26 | $9,007.34 | $17,978.65 | PASS / PASS |
| AMZNx | 8 | `dDotDvyc38TnANLcu1a28D5e3u36pxXUXSLqd122ikX` | $4,000.00 | $9,000.00 | $17,964.00 | PASS / PASS |
| ANSEM | 6 | `A6PW7eDRW4i7TUC6XZpdWyopCHdg2H5kDsojnh9Nfa8j` | $4,008.17 | $9,018.39 | $18,000.71 | PASS / PASS |
| ARC | 6 | `EV4eTyDUjjUJojyRGYEtgqs3rx2Ar1JrAmDLM9Hghtma` | $4,000.00 | $9,000.00 | $17,964.00 | PASS / PASS |
| BOME | 6 | `Q2WQU1x5CWZTsya8EcsNuDkf5eT7C2s3UTFuWHzwjTb` | $3,993.58 | $8,985.56 | $17,935.18 | PASS / PASS |
| BONK | 5 | `BXibKbUm2fGKQgYj7ySs7qAPDR8vjfSJH1T8KkcnQF9L` | $4,002.20 | $9,004.95 | $17,973.89 | PASS / PASS |
| BP | 9 | `YeXpP4TQtUTN4pkjkapp12Cy77k3bXUDxtc7vNCaK6K` | $3,979.59 | $8,954.07 | $17,872.33 | PASS / PASS |
| CARDS | 6 | `HpQpU9cdR5H3V1ADzSLt3rNEwMZZUpqNBhwpbPvTJVSk` | $4,011.06 | $9,024.88 | $18,013.67 | PASS / PASS |
| COINx | 8 | `6SxUkEMGsEks8qGMNEcuuYRTyaysGTPzosMUSSCXqySB` | $4,012.21 | $9,027.48 | $18,018.85 | PASS / PASS |
| CRCLx | 8 | `FE37nonqwbTCK3PhZqoD5xZhm18eYJfmdvhJSfM7KKsk` | $4,013.37 | $9,030.09 | $18,024.07 | PASS / PASS |
| ETH | 8 | `3gqgurNjxtKjwcj2k7yWVwAeKnLqDk7iDj1rHtUnVJ6L` | $4,002.26 | $9,005.08 | $17,974.14 | PASS / PASS |
| Fartcoin | 6 | `FJqorZaADDjaR1py98Ku6MpcCshtkbjv9Lq1UnEYtzVb` | $3,993.60 | $8,985.61 | $17,935.27 | PASS / PASS |
| GLDx | 8 | `6AJ3aJQN9zPBDr6FhaNQ8YjkTaD4LsCqeHm59PjL1QPp` | $3,984.96 | $8,966.15 | $17,896.44 | PASS / PASS |
| GMEx | 8 | `A4eipt8X6eDSkCvKtpivsaLLsTLvaKcB4ozDQBbrpoPE` | $4,009.47 | $9,021.31 | $18,006.54 | PASS / PASS |
| GOAT | 6 | `BkrJ8hQz4KGKvRVPTkUbJmbSsRiAEWUCHK7cyEMS4UjL` | $3,987.93 | $8,972.85 | $17,909.81 | PASS / PASS |
| GOOGLx | 8 | `4Doscv49KaRQhrYk4r48BXZRUJf1nMAq3fVktdP7zaba` | $4,006.32 | $9,014.21 | $17,992.37 | PASS / PASS |
| HOODx | 8 | `BwbMUfgyRt1s9ASuvw9fuqiF56QpG5B5grTj45wZqvX6` | $4,007.82 | $9,017.60 | $17,999.14 | PASS / PASS |
| HYPE | 9 | `5FJiyFaiHERzfzU8Tev9iKNnfuRtnfu6rjPcLmWEzgaD` | $4,011.86 | $9,026.67 | $18,017.24 | PASS / PASS |
| JLP | 6 | `4KbUKSitVtwyftNZENowE9MpoDaUMerFZwu5UAbeq57t` | $4,011.57 | $9,026.04 | $18,015.98 | PASS / PASS |
| JTO | 9 | `5RUYaXfVJwXCwHBPxKzEyau7rvSUFei87GfCMSXgPJvD` | $3,988.41 | $8,973.92 | $17,911.95 | PASS / PASS |
| JUP | 6 | `AdFbbHXw1pjGLgfxmvwMGtXWHimjK3GAJa9Yc9kqXays` | $4,018.14 | $9,040.82 | $18,045.48 | PASS / PASS |
| KMNO | 6 | `HZHUx68yKXGfpBfGPvGnw1CVHYBwSAXx1Mi2VMQrzod8` | $3,995.47 | $8,989.81 | $17,943.66 | PASS / PASS |
| LINK | 9 | `6bcmBvfCzmAh7wdsX6iHpwDd1d2t58Ek2P5FWUxSLZ3b` | $4,004.92 | $9,011.07 | $17,986.10 | PASS / PASS |
| MET | 6 | `4aA244eCvcTieyh5BLbeAC22UwJxSemAeYdh4G5aBpyn` | $4,017.35 | $9,039.04 | $18,041.93 | PASS / PASS |
| METAx | 8 | `25Bh7AD1pRuLq2wnLWJfMUuUQ41jTxMV9vxw8SCkBynL` | $4,000.02 | $9,000.03 | $17,964.07 | PASS / PASS |
| MSFTx | 8 | `CHNuhnbbjFn5FrDe7zNy94aaCSuWPWhPtsA3erG7MGRP` | $4,023.25 | $9,052.30 | $18,068.40 | PASS / PASS |
| MSTRx | 8 | `9igr1gkuKArVLAMpevJmvpF1ng1paj7HxpmYigdxdaBz` | $4,004.59 | $9,010.32 | $17,984.61 | PASS / PASS |
| MU | 6 | `2AiWtgEAuibTV2TKHK7DeR6qPGpC5yPGjFLwXJcj433j` | $4,000.06 | $9,000.11 | $17,964.22 | PASS / PASS |
| NVDAx | 8 | `HVrwsaxrqvPVuL1J8kPM5VGzywnXKyBdstcpu7KscnXC` | $3,998.16 | $8,995.87 | $17,955.75 | PASS / PASS |
| ORCA | 6 | `AyyyxQitUx66R1CanDVt8hXYDDkJcjPnfxDizJnyYj6u` | $3,995.43 | $8,989.71 | $17,943.47 | PASS / PASS |
| ORE | 11 | `A811DsJ6pXJgvP3mcva3YyvZHryv4ZRKnw5Bi1yMkx3R` | $4,024.46 | $9,055.03 | $18,073.84 | PASS / PASS |
| PENGU | 6 | `99Qk6KURbk8s7fQMk6QYBGUBVezZFrtcjp6jGj8ykc14` | $3,989.53 | $8,976.44 | $17,916.97 | PASS / PASS |
| PLTRx | 8 | `8dDbcMbV3SXhKSXyyPUYh69qVZX5hpurvKXkeqehwu9D` | $4,000.50 | $9,001.13 | $17,966.25 | PASS / PASS |
| PNUT | 6 | `iMKhtSoxnFJNRroWRK8q8zQTQbVVpXAYwBXh2RP1t2n` | $3,994.78 | $8,988.26 | $17,940.57 | PASS / PASS |
| PUMP | 6 | `AJEkdJobRE4MqZ1rznTvmCMspGBEFsEdZNBrC48FZ6vs` | $4,001.40 | $9,003.16 | $17,970.30 | PASS / PASS |
| PYTH | 6 | `HNdtguZqRMCq5eoKBzxmopWgUtrkYFRSPeY2SGdN1LNA` | $4,000.94 | $9,002.11 | $17,968.20 | PASS / PASS |
| QQQx | 8 | `4B3FReiz3QM77H7obMWpBvMx7ygm4P3Z9RgKco9D17UN` | $4,002.00 | $9,004.50 | $17,972.98 | PASS / PASS |
| RAY | 6 | `hVingKMEM8sxN2yGuLYdeC3pAKu3Bm5gDQw1kanSMKm` | $4,005.91 | $9,013.30 | $17,990.55 | PASS / PASS |
| RENDER | 8 | `E2gEw8QS82fjfpiTwhBupC1jToKewYpx76V1NpE9E3BF` | $4,001.13 | $9,002.55 | $17,969.09 | PASS / PASS |
| SHIB | 4 | `EBPk88kDVuVkLWiNbZJbsA8hg3EHrzsjW3aXf4p4m8CU` | $4,073.42 | $9,165.19 | $18,293.72 | PASS / PASS |
| SKHY | 6 | `dayx5Z8qBY6cwa6dAqECHrudpccaz9ynCe26scLWdpW` | $3,977.85 | $8,950.16 | $17,864.52 | PASS / PASS |
| SPCX | 6 | `FeSepx1ZiU5GX856rUNrtCwKLKXdMKRUWN8ihj8GqQ3M` | $4,013.40 | $9,030.15 | $18,024.18 | PASS / PASS |
| SPX | 8 | `5DDhcRDWpG9u4f8Fs95ytsPQbKwqHegy8vsyWyLpfN8E` | $4,005.37 | $9,012.08 | $17,988.11 | PASS / PASS |
| SPYx | 8 | `6kmLDanTfuAF42D1MwnM2r9j32fFRHwkfzFC4EGhRMo3` | $4,001.67 | $9,003.76 | $17,971.50 | PASS / PASS |
| STONK | 9 | `35PzKS4sZGwNa51vH3irUp1A4ASRMq1tzxG9RC4j2H1z` | $4,014.02 | $9,031.55 | $18,026.98 | PASS / PASS |
| TRUMP | 6 | `EjHMPYF371We86Z9DU8ZGB7fTbmMLH7jQ8TPcLPGkXh6` | $3,964.75 | $8,920.69 | $17,805.70 | PASS / PASS |
| TRX | 6 | `GjXp9ALDyNb1kTJqGaUTqJgUeahitHuSFFco1CC6tB6D` | $4,004.09 | $9,009.21 | $17,982.39 | PASS / PASS |
| TSLAx | 8 | `8YgEXsx2SxRDdzqstC2T6qAbUeBvTpDi8t6LGJrUJKRr` | $4,001.75 | $9,003.94 | $17,971.87 | PASS / PASS |
| TTWO | 6 | `91EcQBDduFLyciLF2rV7wS9Y1aAhchcYc7ePsBAjRvX6` | $4,010.48 | $9,023.56 | $18,011.04 | PASS / PASS |
| USDC | 6 | `4Jv2iJqsS1zNoE5oGNDUuY8UBwqmsmjjcX5uAubhKf41` | $4,000.10 | $9,000.22 | $17,964.44 | PASS / PASS |
| USDT | 6 | `EV8NZnGJ1uHPGFRfjorjkU8YSfec738VJZ6LAgzohPnR` | $4,000.03 | $9,000.06 | $17,964.11 | PASS / PASS |
| USELESS | 6 | `CSDtH4uH5uqPBJtCXBLhkJKUaP8fF2JA8DjfH1aTgBEb` | $3,991.58 | $8,981.05 | $17,926.18 | PASS / PASS |
| WBTC | 8 | `Ac2ou1sCrxnwwsoRo8QRjfyDPZyBDqDbGebrom8GFTkr` | $4,014.79 | $9,033.24 | $18,030.37 | PASS / PASS |
| WIF | 6 | `7a9DLasWjrCy4mEQEnvfyXZW2g6RJogGf9cFgqytbEDD` | $4,008.72 | $9,019.61 | $18,003.15 | PASS / PASS |
| wNEAR | 9 | `6iWST3jaAnyx4zDPPBYNV1hYLcHWX34cUMqM2sP8ExvT` | $3,970.56 | $8,933.75 | $17,831.77 | PASS / PASS |
| ZEC | 8 | `7VAi7TxMa3dCASFnkMEnpAFTzRpEJEMjeKZMcKbvekh4` | $4,009.92 | $9,022.31 | $18,008.53 | PASS / PASS |
