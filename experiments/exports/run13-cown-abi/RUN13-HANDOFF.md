# RUN13 handoff

The bundle is `run13-cown-abi-bundle.tar`. It contains this note, the full machine ABI, the manifest, the unchanged 3-signer size file, the test rows, and the four ELF files. Nothing was deployed. `mock_meteora` is not a Meteora add or a permanent lock. `b3911bc` is not a deploy candidate: its reverse could omit the creator account, return 9000, leave the creator's 1000 behind, and mark the reverse complete.

## Creator account

Migrate and reverse now require the canonical `["cown", pool]` account, and it must be writable, before any marker or token change. An empty account there means the creator is owed nothing. A populated account must be owned by the program and carry the creator tag and this pool. Omission, a read-only pass, and another pool's creator account each returned 6040. The marker stayed at "in the pool vault", the pool vault stayed at 10000, and the creator figure stayed at 1000. The same reverse with the canonical account then put 10000 back and a second reverse returned 6056. A 1% fee mint still returns 6053 and moves nothing. No fee is taken from the pots.

Shared import uses the same rule, so a backfill cannot book a creator total from a substitute account.

## New pool after the reserve is sealed

One `sync_quote_stat` set the baseline. Two pools were imported and the reserve was sealed. A third pool was then registered with no second sync and no import. QuoteStat went from 2 to 3. The claim on that pool booked holder 9000 into the pool and into the sealed reserve, which went from 0 to 9000. There is no import marker for that pool. The reserve stayed sealed. Config setup is still required for a new quote. A later launch on a quote that already has a QuoteStat does not need an admin recount.

## The six cells that were missing

`mark_graduated`, `check_quote_mint`, and `close_old_checkpoints` are in the ABI twice, once for each design. They are the same on both.

`mark_graduated`, disc `[125, 72, 57, 129, 59, 15, 247, 251]`, no arguments. Accounts: global readonly `["global"]`, pool record writable `["pool", pool]`, dbc pool readonly and equal to that pool. It flips the flag only when DBC byte 305 is 1. Before that it returned 6022. After that the pool record byte 136 was 1. This disc is already in the saved builder. This work did not add an account to it.

`check_quote_mint`, disc `[18, 102, 204, 80, 128, 116, 58, 205]`, no arguments. One account, the mint, not a signer and not writable. Return data is 7 bytes: tier, decimals, a token-2022 flag, and flags as a little-endian u32. It writes nothing. The saved builder does not send it. A classic mint check succeeded on both designs.

`close_old_checkpoints`, disc `[90, 240, 98, 46, 31, 229, 181, 93]`. Argument is a Borsh vector of `{round u64, owner pubkey, bump u8}`. Accounts: signer (admin or `2tUjZV4UX9V8P65RZxFBhTxHTDhYRhWo6MYsAGuTsn1h`), global readonly, pool record readonly, rent receiver writable and exactly that crank address, then one writable checkpoint per entry. The checkpoint seeds are `["ckpt", pool, round, owner]` with the entry's bump. The round must be below the pool's current round. Rent went to the crank address. Sending the rent to the admin instead returned 6027 and left the checkpoint. The saved builder does not send this instruction.

## What the current caller can accept

The saved client pins program `AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9` and the bytes of each instruction. A larger ELF is not acceptance.

Register's discriminator is still `[85, 229, 114, 47, 75, 145, 166, 100]`. The first seven named accounts are unchanged. Instruction index 7 on register is the quote mint. It is not the watermark. The watermark is account index 7 of `open_lp_reservation` only. The live client sends eight accounts and stops at the quote mint. Both designs return 6048. The client has to append a writable QuoteStat. The shared design also has to append the writable quote reserve. That is a client change.

`mark_graduated` keeps the three accounts above. `check_quote_mint` and `close_old_checkpoints` are unchanged and are not in the client's sent list. The 3-signer memo sizes in `SDK-SIZES-3SIGNER.json` are unchanged.

## Prices

Crank's provisional figures for the previous 620032 and 606400 files are an extension of 0.497799, buffers 3.081350 and 3.150601, and 0.763 of headroom above a 0.5 reserve. These ELFs are different. Do not reuse those prices.

| File | Bytes | sha256 |
| --- | --- | --- |
| 3Evo shared | 621672 | `d87ce272c943bb7b115cc7c2555f1977e38244d2beac576d727d4a7347f834b6` |
| 3Evo per-pool | 607824 | `225c7768650666540c5f66d82ea2c4ac2be7567f9189ff998648abc1cc63fba9` |
| Experiment-id shared | 620392 | `cb0b1d4952e8123a29a2e8cd80d243faab4c8be54c90cb07b215ae0de4ada92c` |
| Experiment-id per-pool | 606544 | `321acc6eedf0d8a510c968eac64a6e853e75f3a6138a3625d18eeb7f5fff13bf` |

No deployment authority is added by this package.
