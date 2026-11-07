# RUN13 ABI correction

Pay cursors are writable. `init_if_needed` does not use the word `mut`, and the previous extractor treated that as read-only. That was an export bug. The program was not wrong and was not changed.

Anchor 0.31.2, which expands the accounts for anchor-lang 0.31.1, does this in `ConstraintGroupBuilder::build`: if the attribute is `init` or `init_if_needed` and `mut` is absent, it inserts `ConstraintMut`. `is_mutable()` is then true. `__client_accounts.rs` emits `AccountMeta::new` for that field. The program also runs the generated check `if !account.is_writable`. A read-only compiled meta is rejected by that check. It is not a separate check in the pay handler. `realloc` does not insert `mut`; the field must already say `mut`, and the named payer must already be `mut`, or Anchor refuses to compile. A payer is not given writability by being named. Our payers already have `mut`.

A PDA with `init` or `init_if_needed` is not made a signer. Both pay cursors and the wSOL vault are PDAs, so they are writable and not signers. The same rule makes the initialized global account, the pool record, the auto-LP config, and QuoteStat writable. The compiled flag and the Anchor runtime check are the same fact. A handler `require!(is_writable)` is a different check and is not what marks the cursor.



This package does not change the program. The four run13 ELF hashes stay:

| File | Bytes | sha256 |
| --- | --- | --- |
| 3Evo shared | 621672 | `d87ce272c943bb7b115cc7c2555f1977e38244d2beac576d727d4a7347f834b6` |
| 3Evo per-pool | 607824 | `225c7768650666540c5f66d82ea2c4ac2be7567f9189ff998648abc1cc63fba9` |
| Experiment-id shared | 620392 | `cb0b1d4952e8123a29a2e8cd80d243faab4c8be54c90cb07b215ae0de4ada92c` |
| Experiment-id per-pool | 606544 | `321acc6eedf0d8a510c968eac64a6e853e75f3a6138a3625d18eeb7f5fff13bf` |

Source pin remains `380a54d`. The previous bundle `run13-cown-abi-bundle.tar` is `ee634954e991cc4eb5d16e234c649f854a335b3fc9480994942d0f6922f835c4`. No new binary was built. Crank's rent table for those exact sizes still applies. Fee-mint migration still returns 6053 and is blocked. `mock_meteora` is not `add_liquidity` and not `permanent_lock`.

The schema is a custom raw ABI read from the experimental Rust. `cargo-build-sbf` does not emit an Anchor IDL for this crate, so none is claimed. Every named account list was regenerated from the `Accounts` struct in that source and checked back against it. Instructions whose context has no fields are marked that way, and their order comes from the handler.

## What the earlier export got wrong

`crank_claim_damm` has 18 named accounts on both designs, not 3. In order: global, pool record writable, quote vault writable, pool authority, damm pool, position writable, token A account writable, token B account writable and equal to the quote vault, token A vault writable, token B vault writable, token A mint, token B mint, position NFT account, token A program, token B program, event authority, damm program, token program. After those, both designs require six remaining accounts. The first five are the creator extras: payer signer and writable, system program, DBC pool, DBC config, creator owed writable. The sixth is the deliberate design split. Shared requires the writable quote reserve. Per-pool requires the writable pool vault. A caller that sends 18 named accounts plus only those five extras is one account short on both designs. That is a source incompatibility, not an export abbreviation.

`crank_pay_page_v3` on per-pool has 9 named accounts: payer writable signer, global, pool record writable, quote vault writable, quote mint, quote token program, base mint, system program, pay cursor writable. The field is `quote_token_program`, not `token_program`. Shared has those same nine and a tenth, the writable quote reserve. The current caller header of nine matches per-pool. It does not match shared until the reserve is appended as that named account. Holder pairs are remaining accounts, two per holder, and the destination must be writable.

`crank_auto_lp` has an empty Anchor context. All accounts are remaining. Both designs use the same first 18: global, auto-LP config, payer signer, damm pool writable, position writable, position NFT, token A and B accounts writable, both vaults writable, both mints, both token programs, event authority, DAMM program, then the two QuoteStat accounts. Shared then has the two writable quote reserves and pairs of pool record writable plus creator owed. Per-pool has no reserves; each pool is pool record writable, pool vault writable, creator owed. Both stop at 4 pools. The CPI in this repository is the mock.

`crank_pay_page_v2` names the quote reserve as account index 10 on shared. Per-pool has no reserve, and its quote vault must be owned by the pool record. Shared pay v2's vault is the global ATA.

`check_quote_mint` is the crank's simulate-only pair gate. `close_old_checkpoints` is sent by the separate maintenance command. The earlier sentence that called them unused was wrong. Their wire shape was already the one the caller uses. The program does not cap the checkpoint vector. The command's chunk of 12 is the command's limit.

Register index 7 is still the quote mint. The watermark is still only `open_lp_reservation` index 7.

## Errors

Shared codes run 6000 `BadMint` through 6056 `BatchRetired`. Per-pool matches through 6049, then 6050 is `PoolVault` rather than `ReserveMissing`, and 6053 is `MigrationFeeShort`. The full names, numbers, and messages are in `ERROR-TABLE.json`. `check_quote_mint` rejects with the same mint errors as claim and pay: 6035, 6000, 6028, 6029, 6030, 6031, 6032, and 6033. A freeze authority or a transfer fee at or below 100 bps sets a flag and tier 1. It does not reject the mint.

No program defect was found in this pass. The gaps were in the export.
