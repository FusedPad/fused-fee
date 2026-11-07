# Integrated local comparison

https://github.com/FusedPad/fused-fee/pull/3

These are experimental copies of `fused_fee`. They are not selected and they were not deployed. The earlier toy programs and `INTEGRATION-GAP.md` stay as prior evidence.

Both copies keep the seeds `global`, `pool`, `qstat`, and `cown`. The global PDA is still the Meteora fee-claimer and still signs the shared quote vault on a claim. Pool records stay at `["pool", pool]`.

## What the bank actually ran

SPL Token in the bank is real. The DBC claim is the local stand-in `mock_meteora`, which transfers 11000 quote tokens. It is not mainnet Meteora and it is not a permanent DAMM lock.

| Case | Result | Compute units |
| --- | --- | --- |
| Shared reserve: seal an empty quote after one imported pool | pass | 11738 |
| Shared reserve: mock claim books holder 9000, burn 2000, vault 11000, and the reserve matches | pass | 29138 |
| Shared reserve: pay page v3 pays 9000 and the reserve holder total goes to 0 | pass | 35767 |
| Shared reserve: release v2 sends burn 2000 to the admin token account | pass | 25358 |
| Shared reserve: a new pool register without the reserve account | fails 6050 `ReserveMissing` | |
| Per-pool: mock claim leaves the global vault at 0 and the pool vault at 11000 | pass | 36176 |
| Per-pool: pay page v3 pays 9000 from the pool vault; the global vault stays 0 | pass | 33800 |
| Token-2022 mint with no extensions, `check_quote_mint` on the shared build | pass | 1648 |
| Token-2022 transfer fee above 1% | fails 6032 `TransferFeeTooHigh` | |
| Token-2022 active transfer hook, on the per-pool build | fails 6029 `HookActive` | |

Numbers are in `experiments/results/integrated-fee.json`.

## Binaries that were tested

| Build | Program id | sha256 | Bytes |
| --- | --- | --- | --- |
| Shared reserve | `C1nBL5ufPcQvjkmj6hAmRgoLntHonMuf5WdxVxJenJaE` | `47360f8d231b3b641f5e7ff731a640e70964ea78839cb03e033cfd1badb9012b` | 708808 |
| Per-pool vault | `C5hUUPNfyui9sr2EXzVgiZa733W1TRbUdvJcZLMFXdtw` | `1f23a817023547178a42caa8dfb3c4d961e987b5e339507c685cd7def07be56d` | 675704 |

Source is `experiments/integrated/shared_fee` and `experiments/integrated/per_pool_fee`. Economics are unchanged: 20/110 burn, the rest holders, and 8/100 creator on a graduated claim when the creator has no DAMM LP.

Shared design adds `["qres", quote_mint]`. Register requires it as `remaining_accounts[2]`, after the quote mint and QuoteStat. Claims, pay v2, pay v3, both burn releases, direct burn, creator claim, and auto-LP were patched to update that account and to refuse a payout when the vault is below the reserve total. This run executed register, claim, pay v3, and release v2.

Per-pool design keeps the claim landing in the global associated token account, then moves that delta into a token account owned by the pool record. Pay v3, release v2, release v1, direct burn, and creator claim were patched to sign with `["pool", pool, bump]` and to require that vault owner. This run executed the claim move and pay v3.

## Not executed in this run

The handlers below are in the built files. This bank did not call them, so they are not passes:

- `crank_claim_damm` on either copy
- pay page v2
- `claim_creator_share`
- `crank_burn_direct`
- `crank_auto_lp` with the two reserve accounts at indexes 18 and 19 on the shared copy
- a real Token-2022 transfer that charges a fee. The policy rejection above 1% ran. A transfer at or under 1% did not.
- a manufactured short vault against the integrated reserve
- two claims in one transaction

`crank_auto_lp` still calls the Meteora stand-in for add and permanent lock when it is invoked. That CPI was not invoked here. A later devnet rehearsal is still required before anyone treats a lock as real. The per-pool claim test moves tokens with real SPL. It does not lock liquidity.

## 3Evo offline files

Built after the tests, by changing only `declare_id` to `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`. They were not executed and not deployed. Seeds were not changed. Do not upload either one onto the funded rehearsal account until a migration is reviewed: claims and payouts fail closed until the new reserve is sealed, or until each pool vault exists.

| File | sha256 | Bytes | Buffer rent | Programdata extend past 522085 |
| --- | --- | --- | --- | --- |
| `experiments/artifacts/fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` | `2f5409438600c378f5e3bc926d3efcee77cdd9bb762f0f5ca2d99f8ff20ea319` | 711144 | 4.950453 SOL | 189104 bytes, 1.31616384 SOL |
| `experiments/artifacts/fused_fee-3evo-PER-POOL-experimental-DO-NOT-UPLOAD-YET.so` | `e8c8861a91f333d71c785152b6ae1c537766d0ecc56dfde6e95d37239a3353aa` | 678040 | 4.720049 SOL | 156000 bytes, 1.08576 SOL |

Buffer rent uses `(128 + bytes) * 3480 * 2` and is refundable only if the buffer is closed. The extend is not. Both files are larger than the 530000-byte planning cap, so the 3.70 SOL buffer ceiling does not cover them. That is a real size blocker for uploading these builds onto the current rehearsal programdata under that plan. It is not a reason to upload the smaller toy programs instead. Those toys use different seeds and would leave the current vaults without a signer.

The smaller layout-compatible file from the previous note, sha256 `db42525e…`, is still the current fee program with the 3Evo id and no vault redesign. It is also not approved to deploy.
