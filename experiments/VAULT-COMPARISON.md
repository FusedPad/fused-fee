# Experimental vault comparison

Local SBF only. Neither design is selected. `fused_fee` production instructions were not changed. Nothing was deployed, spent, or sent to a public network. The register-caller matrix in `REGISTER-COMPATIBILITY.md` still stands. The current 8-account website builder is a rollout constraint, not a reason these prototypes were skipped.

## Binaries

| Program | Id in the file | ELF sha256 | Bytes |
| --- | --- | --- | --- |
| Shared reserve | `6URwbPipuA4MJLG7LCRRZuWnms3JZ9cRG3z9indXWz8G` | `6827f082ef2880c718deb440063e216ce608287b8f0febb20cfbb5cae66959cd` | 125040 |
| Per-pool vault | `6YMEjhBqVTMaSRWcmVkLrnHZ22FWEDJEpTeonAg8GKSy` | `c582180c224ad753c3f23fa817058940dd44fe458a50b498517435b12809b5e0` | 115304 |

Built with `cargo-build-sbf --tools-version v1.58`. These ids are not mainnet `AZY2…` and not devnet `3Evo…`. A loader will reject them if they are uploaded onto a program with a different id. Rows and compute units are in `experiments/results/vault-compare.json`. The bank test `experiments/compare/tests/both.rs` passed.

Seeds are `xb` for the shared program and `xa` for the per-pool program. They do not alias fused_fee `global`, `pool`, `qstat`, `cown`, or `autolp`.

There is no Anchor IDL. The interface is the tag byte plus the account order below. Hand-written `experiments/interface.json` matches those tags.

## What was executed

SPL Token in the bank is the real program-test build. Deposits are ordinary token transfers into the vault. They are not Meteora claims. The 11000-unit curve split booked holder 9000 and burn 2000. The graduated split with the 8% creator carve booked creator 880, holder 8280, burn 1840. Those are the existing ratios.

Shared reserve:

- A deposit while the quote is still frozen fails `105`. No pot is booked.
- Seal fails `102` if a pool was registered during backfill and not imported. After that pool is imported, seal succeeds.
- Curve and graduated deposits update the pool and the reserve by the same amounts. After both, reserve holder 17280, burn 5840, owed 880, vault 24000.
- A 100-unit holder pay leaves the vault at 23900 and the destination at 100. CU 13450.
- Two outs in one transaction, sharing the reserve, pay 90 and leave the reserve consistent. CU 26886.
- A stranger fails `106`. Pause fails `110`.
- A vault balance lowered by 1 with a direct bank write, not by an instruction, makes the burn release fail `103`. The burn pot stays 1840. That shortfall was manufactured.
- Auto-LP moves half of one burn pot, 2000, into an escrow. CU 21913. Consume moves it once. A second consume does not move it again. Refund of a later batch restores the burn pot and the vault.
- A request above the half-burn budget fails `111` and leaves the pot.
- An 8-account register fails `107`.
- 19 quote groups, 43 pools, were registered, imported, and sealed. The first group is 13 pools and includes a 250 creator-owed slice inside the 1000-unit pool. One group seals with a 7-unit surplus. One group is zero pots. First-group seal CU 4710.
- 55 pools on one quote: one pay uses 10 locked accounts and 13431 CU.

Per-pool vault:

- Curve 11000 lands in that pool's vault only. The other pool's vault stays 0. CU 11423.
- A pay of 100 does not change the other vault.
- An 8-account register fails `107`.
- Four pools migrate by real transfers, then a 4-pool LP batch moves 20000 into escrow. CU 58006. Consume-once and a replay leave the destination at 20000. Refund restores a later batch.
- A 5-pool batch fails `108` before any move.
- Zeroing one pool's vault by a direct bank write makes that pool's pay fail `103`. The other pool's pay still succeeds. CU 13021.
- 13 real migrations, then a pay with 9 locked accounts. CU 13021.
- 43 real migrations from one source account empty that source.
- Closing a pool that still has a pot fails `100`. Closing an empty pool removes the account. CU 3457.

A single shared pay serializes to 475 bytes and 9 keys before the fee payer is rewritten. A made-up instruction that appends 55 extra pool accounts serializes to 2290 bytes and 64 keys. That instruction was not sent. The program does not have that instruction.

## 3Evo sequential upgrade

`3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` is the historical devnet rehearsal program. Its programdata length was recorded earlier as 522085 bytes. It was not read again here, and nothing was upgraded.

These two ELFs are about 125KB and 115KB, so each is smaller than that programdata. Using the rent formula `(128 + bytes) * 3480 * 2`, one buffer is about 0.871 SOL for the shared ELF and 0.803 SOL for the per-pool ELF. One buffer at a time fits under the 3.70 SOL buffer ceiling in Crank's plan. The two buffers are not needed at once. That rent is refundable only if the buffer is closed. This session did not close or create one.

The tested files cannot be the upgrade bytes. Their `declare_id` is not `3Evo…`. A rehearsal upgrade needs a new build with that id, then a new hash. That build was not made.

If that rebuild is later approved:

- The upgrade replaces the code. Existing fused_fee instructions on that program stop until the old ELF is upgraded back. Old accounts stay. These prototypes do not read `["global"]`, `["pool", …]`, `["qstat", …]`, or `["cown", …]`, and they do not sign `["global"]`, so they do not move the current shared vault.
- Running A and then B on the same program id does not mix their accounting, because the seeds differ. A's accounts remain after the upgrade to B unless they were closed while A was still the code. Empty per-pool close was executed. A non-empty pool close is rejected. Shared `close_reserve` is in the source and was not in the executed matrix.
- This does not migrate the tokens already in 3Evo's vault. That would need an instruction that signs the old global seeds. These prototypes do not have it.

## Not tested

- No Meteora DBC or DAMM instruction. No permanent LP lock. The LP destination is a normal token account.
- No Token-2022 transfer-fee mint was created. The fee parser is source only.
- No holder pages, cursors, wSOL unwrap, or 6-hour crank window.
- No quote-stat account. The shared reserve is a different account.
- The manufactured 1-unit shortfall is not a sequence the instructions produced.
- The 55-account tail was serialized locally and not executed.
- Devnet was not used. No website file was edited.
- Rows whose compute unit is 0 are state checks after earlier transactions, not a measured call of their own.

Neither design is ready for a devnet rehearsal until a `3Evo` id build exists and Michael approves the spend. Nothing here authorizes that.
