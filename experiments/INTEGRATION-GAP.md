# Integration status for Crank

PR: https://github.com/FusedPad/fused-fee/pull/3

Commit this note lands on: the branch `cursor/exp-vault-compare-a4ab`. No design is selected. Nothing was deployed, spent, merged, or sent. The website was not edited. The earlier toy programs and the register-caller evidence are still in this branch.

The toy shared-reserve and per-pool programs are not rehearsal builds. Their program ids are not `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`, and their seeds are `xb` and `xa`. Uploading either one onto the funded rehearsal program would replace the fee program with code that does not sign `["global"]` and does not read `["pool", pool]`. The quote vaults, whose authority is the rehearsal global PDA, would stay on chain and the new code could not move them. That is stranded state. Those toys must not be used on that account. No binary of either toy was built with the 3Evo id.

## What the real program did in this run

`cargo test --manifest-path programs/fused_fee/Cargo.toml --test runtime_suite -- --test-threads=1` passed 3 tests against the existing mainnet-id file `target/deploy/fused_fee.so` (`1fdd85b47f76775c5937a21edf08e4e5db69b2c1f1df91185a653e53c8fa0968`, 522968 bytes):

- `quotestat_registration_matrix`
- `claims_payouts_burn_and_creator`
- `auto_lp_allocation_and_known_gaps`

Meteora in that suite is the local stand-in `harness/mock_meteora`, not mainnet DBC or DAMM. A passing auto-LP case there is a mock transfer plus two flag bytes. It is not a permanent Meteora lock. The toy escrow transfer is also not a permanent lock.

Token-2022 in that same suite, on the real program:

- A Token-2022 mint with no extensions registers and `check_quote_mint` succeeds.
- An unknown extension is rejected at `register_config`.
- A transfer fee above the 1% cap (`MAX_TRANSFER_FEE_BPS` = 100) is rejected.
- An active transfer hook is rejected.
- The source also rejects a paused mint, a default-frozen mint, non-transferable, and confidential mint/burn. Those four were not re-executed as their own rows in this run. The cap and the hook and the unknown extension were.

Holder pages, creator owed, claim, burn, and release in that suite are the real `fused_fee` instructions. They use the shared global vault. They do not implement either experimental design.

## Address check

Computed locally, not on a cluster:

| Account | Address |
| --- | --- |
| Rehearsal global `["global"]` under 3Evo | `2KwUVpnSKUbix7CGwLHsAJzFdrt1gmEKBomT9ggN3LkF` |
| Toy shared global `["xb","global"]` | `FQqMPRz2KKE8atBtXTRokUE9DCYdtS5qULEp62FkfU9w` |
| Toy per-pool global `["xa","global"]` | `GiSFZ5RrUo9r9xYxXxxuDpp6KE7f7VbAoAxjdaTwVAdX` |

A pool PDA `["pool", pool]` under 3Evo is also a different address from `["xb","pool", pool]` and from the same seed under the mainnet program id. The test is `toy_seeds_do_not_match_rehearsal_accounts` in `experiments/compare`.

## 3Evo file that keeps the current layout

`experiments/artifacts/fused_fee-3evo-LAYOUT-COMPATIBLE-not-a-vault-redesign.so`

| | |
| --- | --- |
| sha256 | `db42525e06e4a0810ef0f753db312e8a4ddad152dcb96eb20f80b79775c7d01c` |
| bytes | 522968 |
| program id bytes inside the file | `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` (one occurrence). Mainnet `AZY2…` bytes are absent. |
| source | Current branch source with only that id changed, in `/tmp` only. The committed `programs/fused_fee/src/lib.rs` still declares `AZY2…`. |
| what it is | The current fee program, including the QuoteStat requirement from this branch. Not the live `80060ab` program. Not either vault design. |

It was not deployed. Deploying it would keep the same seeds, so existing vaults would still have a signer. It would also make `register_pool` require an initialized QuoteStat, which the current 8-account website does not send. That is a behavior change, not a stranded vault. It is not approved.

Recorded 3Evo programdata length from the earlier note is 522085 bytes. This file needs 522968 + 45 = 523013 bytes of programdata. The shortfall is 928 bytes. At `(bytes) * 3480 * 2` lamports, that extend is 0.00645888 SOL and is not refunded by closing a buffer. The upgrade buffer for this 522968-byte file is `(128 + 522968) * 3480 * 2` = 3.64074816 SOL, refundable if the buffer is closed and not left allocated. One buffer at a time. This session did not extend, upload, or close anything. The 3.70 SOL buffer ceiling in the planning note still covers 3.64074816. The 0.00645888 extend is extra permanent rent if that upgrade is later approved. It is not spent now.

## Where the two designs stand against the real lifecycle

Neither design has been wired into `fused_fee`. The toys do not have these production instructions, and the production program does not have their reserve or per-pool vault accounts.

| Production instruction | Real program | Toy shared reserve | Toy per-pool vault |
| --- | --- | --- | --- |
| `initialize`, config, pause, burn mint | yes | init and pause only | init and pause only |
| `register_pool` 7 named + quote at index 7 | yes | different accounts and seeds; 8-key shape fails | same, fails |
| `sync_quote_stat` | yes | no | no |
| `crank_claim` / `crank_claim_damm` | yes, mock Meteora in tests | a direct deposit, not a claim | same |
| holder `crank_pay_page_v2` / `v3` | yes | one destination, no cursor or page | same |
| `release_burn_pot` / `v2`, `crank_burn_direct` | yes | one burn debit from the shared vault | one burn debit from that pool's vault |
| `claim_creator_share` | yes | one owed debit | one owed debit |
| `crank_auto_lp` permanent lock | yes, mock only | escrow transfer, not a lock | escrow transfer, not a lock |
| Token-2022 policy | yes, suite above | not executed | not executed |
| migration of an existing global vault | not started | import onto a new PDA | `migrate_in` from an admin token account, not from the live global |

Bootstrap and retries that were executed are only inside the toys: frozen-until-import, same-transaction rollback, consume-twice, and refund. They do not run through `fused_fee`.

## What a real integration still requires

An upgrade that does not strand the rehearsal has to keep program id `3Evo…` only for that rehearsal, keep signing `["global"]`, and keep `PoolRecord` at `["pool", pool]` with the same field layout or an explicit in-place migration. New seeds are a new program, or they strand the old vaults.

Shared reserve, if built inside `fused_fee`: one `["reserve", quote]` account updated in `crank_claim`, `crank_claim_damm`, both pay pages, both burn releases, direct burn, creator claim, and auto-LP. Backfill must run while claims are stopped, then every new register must update it. The 8-account website call still cannot pass that account until the builder changes.

Per-pool vaults, if built inside `fused_fee`: each pool needs its own token account, a move from today's global ATA into that account, and every claim, pay, burn, and creator path pointed at it. Auto-LP has to batch those vaults. The website register call has no vault account today.

Neither of those edits was made. Doing both inside one program, with the full instruction set, was not finished. The toys remain the bounded accounting evidence from the previous commit. They are not the rehearsal.

## Untested

- Real mainnet or devnet Meteora DBC and DAMM. Every Meteora result so far is the local stand-in or is absent.
- Permanent liquidity lock on a real DAMM position.
- A Token-2022 transfer that actually charges a fee inside either toy. The real program's policy rejects a fee above 1% and an active hook. It does not, in this run, show the lamports or token units of a fee-on-transfer pay.
- Paused mint, default-frozen mint, non-transferable, and confidential mint/burn as separate executed rows.
- Holder paging, wSOL unwrap, and the 6-hour crank on either design.
- Any transaction on devnet or mainnet.

No architecture is chosen. No spend is approved.
