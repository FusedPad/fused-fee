# Mainnet create: SOL/wSOL Token-2022 (token_type = 1) FusedPad DBC config, 17 Nov 2026

This is the 57th pair. The other 56 were created earlier today. Operator approval was given for all 57 pairs at the creation-time prices, after the crank flag PAY_V4_WSOL_T22 was turned on.

## Result

- **PASS.** One atomic transaction created the new config and repointed the existing fused_fee ConfigRecord. Nothing was left half-sent.
- New config: `E1f6qMU5817ksstswxPsqbtxgurVmned8iQBoTze4m9F`
- Signature: `5DgSQcQK4E5GQdBuvvppaYtHNYSg7fEYpNDN6b1sT8oRxV8UMYzjQXcuQKQzhjsb2BgxoG4fr9c5ZY2ED3rLd3kF`
- Slot: **470546492** (finalized). Block time 17 Nov 2026, 13:13:45 Europe/Isle_of_Man (UTC+0).
- Signer: admin D8pJVn2TTnEy4MVYQkPYyG4jS4oBb2CvrCq3jEET2rHx on the operator machine. The key was referenced only by path. It was never printed, copied or hashed.
- Laptop send window: 13:13:38–13:13:51 Europe/Isle_of_Man. RPC: public mainnet (`https://api.mainnet-beta.solana.com`).
- **D8pJ balance:** 2.894992746 SOL before and **2.889007157 SOL after** (checked again on Helius at finalized).
- **Cost: 0.005985589 SOL** (5,985,589 lamports: 5,974,080 rent for the 1,048 B config, plus 11,509 lamports in fees with a 20,000 µlamport/CU priority).

## Build (box, earlier the same day)

- Built and unsigned-simulated on Helius at 13:05:51–13:05:59 Europe/Isle_of_Man. Dry run was DRY-OK. Nothing was sent then.
- Prices at build: Jupiter v3 $73.88621109141216, DexScreener $73.82 (gap 0.09%).
- Curve: the house builder `buildCurveWithMarketCap` (SDK 1.5.13), $4,000 open and $9,000 raise, R = 3, so a $36,000 graduation MC. token_type = 1, DAMM v2, migration fee option 6, migrated fee 150 bps, dynamic fee 0, 1.5% fee (cliff numerator 15,000,000), creator 8%, 100% partner permanent-locked LP, fee claimer and leftover = Global F3sw….
- Quote is wSOL `So11111111111111111111111111111111111111112`, 9 decimals, SPL. Atomic tx 825 bytes (under 1,232). Signed simulation used 44,170 CU and matched the box-verified bytes.

## Rule at send time (fresh Jupiter price)

Open $3k–$5k, raise ≥ $8k, locked LP after Meteora's 0.2% fee ≥ $16k, drift since build ≤ 5%. The script would have refused otherwise.

| | |
|---|---|
| Send-time Jupiter | $73.85816174220307 (drift −0.038%) |
| Open MC | $3,998.48 |
| Raise | $8,996.58 (121.808925739 SOL) |
| Locked LP after 0.2% | $17,957.18 |
| sqrt_start_price | `135727555095865343` |
| migration_quote_threshold (raw) | `121808925739` |

## Helius verification (finalized), 17 Nov 2026, 13:14:33 Europe/Isle_of_Man

**PASS.** Independent read on Helius (`helius-verify.json`):

- New config is DBC-owned, 1,048 bytes, full byte match to the box-verified account.
- token_type = 1 and byte 237 = 1. Quote mint is wSOL. Fee claimer and leftover = Global.
- DAMM v2, fee option 6, 150 bps, dynamic fee 0, 100% partner permanent lock with 0 other LP.
- sqrt_start_price and migration_quote_threshold match the build.
- ConfigRecord `3ZgxFFgNbobmRYcx9vkxXjJiC169K9MPZpkcQmi9Cnse` now points at `E1f6qMU5817ksstswxPsqbtxgurVmned8iQBoTze4m9F`.
- Old config `Fdz9izTjXKicQiosBcQCFgv31mcthJcBhDb6qhERbrbN` is untouched: still DBC-owned, 1,048 bytes, token_type = 0, threshold still 78 SOL (`78000000000`), same sqrt_start as this morning (`93437093511147724`).
- Transaction succeeded, fee 11,509 lamports. Admin cost and closing balance match the laptop receipt.

## fused_fee registration

Same flow as the 56 pairs. The SOL ConfigRecord already existed, so `update_config` ran in the same transaction as `create_config` and repointed it. No `register_config` or `sync_quote_stat` was needed. QuoteStat was already in place. No new fused_fee accounts were created.

Old-config pools can still register with `register_pool`. The crank will report the old config as "not current target".

