# QuoteStat auto-count handoff

Narrow change only. `register_pool` now increments the canonical QuoteStat exactly once for each pool that is successfully registered. This is not a deployment and it is not mainnet-ready. The live website still sends 8 accounts and was not modified.

## Pins

| | |
|---|---|
| Base commit | `80060ab46f77c6b46b4c9c1f65918ae9e356dbfc` (`main`) |
| Program id | `AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9` |
| Base `lib.rs` sha256 | `558c68ec6a287d271943e422186d0ec05b16140aa08ad6858612070de95fd366` |
| New `lib.rs` sha256 | `962378fc274a720706a9ddb2aa891c413007fc47d9d057e178981df96d236edf` |
| Toolchain | `cargo-build-sbf --arch v0 --tools-version v1.58 --offline`, Anchor 0.31.1 resolved to locked 0.31.2, profile overflow-checks / lto fat / codegen-units 1 / opt-level z |

Rebuilt base ELF from that commit, on this machine, with the locked dependencies:

| | bytes | sha256 |
|---|---:|---|
| Base full ELF | 521080 | `2028226a2de10dbf2a1c8f9106d67f0ad3681ade7b4b2dcafba453ab5e1949bd` |
| Base, trailing zeros removed | 521065 | `35953c2c65ee8935823eaad0030bdc56fe675e67f9010e50fc21131e60f85a3d` |

Those two hashes are the previously recorded mainnet pair. The rebuild matches both.

New ELF from this branch:

| | bytes | sha256 |
|---|---:|---|
| New full ELF | 520272 | `968c6eb26274082c49603c82d95463bcb620cf9f8b9cf8351eca1da8d66a2001` |
| New, trailing zeros removed | 520257 | `89492f8f20af3b976cc8921720e6bf55882714ae4df998958a7a211dc0a58e64` |

No key was used. The build's generated program keypair was deleted and is not part of this change.

## What was wrong, and the fix

On `80060ab`, `register_pool` treats QuoteStat as optional. Leaving it out still creates the pool record and leaves `pool_count` stale. Passing an empty `["qstat", quote_mint]` PDA creates it at count 1, which hides pools that already exist.

The handler now requires two remaining accounts before it writes the pool:

1. `remaining_accounts[0]` is the quote mint, checked with the existing mint policy.
2. `remaining_accounts[1]` is the writable QuoteStat PDA `["qstat", quote_mint]`.

An existing account must be owned by this program, 49 bytes, the QuoteStat discriminator, and the same mint. Its `pool_count` then increases by 1 with `checked_add`. Overflow returns `Math` (6006). A missing account returns `QuoteStatRequired` (6048). An empty PDA returns `QuoteStatUninitialized` (6049) and is not created. A wrong key, owner, length, tag, or mint returns `QuoteMismatch` (6002). A read-only account returns `BadMint` (6000). Any of those failures rolls back the new pool record, because Anchor `init` runs inside the same instruction.

Registering the same pool again fails at `init` and does not increment. The stored quote on an existing pool record is not rewritten. Register stays permissionless: the payer does not have to be the admin. `sync_quote_stat` stays an admin absolute overwrite. It does not add.

`Cargo.lock` gained the offline test crates. No previously locked package version was removed or replaced, so the program build graph is the same one that reproduced the base ELF.

## One admin baseline, once per quote

The program does not scan old pool records. Pools that already exist are invisible to the counter until the admin writes the count.

`sync_quote_stat(quote_mint, exact_existing_pool_count)` is that write. It is once per quote, not once per launch:

- A quote that already has pools: one admin call with that quote's exact current pool count.
- A quote that has no pools yet: one admin call with 0, before its first `register_pool`.

After that account exists, each successful new registration adds exactly one. A later launch does not need a recount. This repository does not include a chain read of those counts, and this change does not invent one.

## ABI delta

Unchanged:

- `register_pool` discriminator `[85, 229, 114, 47, 75, 145, 166, 100]`
- Arguments, 104 bytes, no length prefixes: discriminator, pool, base mint, quote mint
- The 7 named accounts and their order: payer (signer, writable), config record, pool record (writable), system program, global, DBC pool, DBC config
- QuoteStat layout: 8 byte discriminator `[196, 19, 91, 20, 68, 151, 114, 86]`, mint at offset 8, `pool_count` u64 at offset 40, bump at offset 48. Total 49 bytes
- `sync_quote_stat` discriminator `[130, 168, 89, 25, 125, 164, 73, 104]`, accounts, and arguments
- Every other instruction, including claim, pay, burn, release, and auto-LP
- Error numbers through `AutoLpCpiFailed` (6047)
- Economics: holder 90, burn 20, split 110; creator 8/100; auto-LP 5000 bps. Auto-LP was not activated or edited

Changed:

- `remaining_accounts[0]` (quote mint) and `remaining_accounts[1]` (writable QuoteStat) are required. The website's current 8-account instruction is the 7 named accounts plus the quote mint, so it has no `remaining_accounts[1]`.
- Appended errors: `QuoteStatRequired` = 6048, `QuoteStatUninitialized` = 6049.

## Caller size

Measured offline as a legacy transaction, one signature, program `AZY2…`, 104-byte data, account order taken from the website audit. Wire size is `1 + 64 + bincode(message)`.

| Caller | Accounts | Signed bytes |
|---|---:|---:|
| Current website `register_pool` | 8 | 505 |
| Same instruction plus writable QuoteStat | 9 | 538 |

Both are under the 1232-byte packet cap. The 9th account adds 33 bytes on a register-only transaction. A bundled create, memo, and register that is already near 1232 can cross the cap; this run did not rebuild that bundle. No mainnet-ready claim follows from the register-only measurement.

## Reported website change (not made, not authorized)

One edit in `registerPoolInstruction`: after the quote mint, append a writable non-signer

`findProgramAddressSync([Buffer.from("qstat"), quoteMint], FEE_PROGRAM)`.

Do not change the discriminator or the 104-byte arguments. No file in the website was edited here.

## Rollout order

1. On the program that is live now, the admin calls `sync_quote_stat` once per quote with the exact existing count (0 if that quote has no pools). Do this before any client sends the 9th account. The current website omits QuoteStat, so those baseline writes stay put.
2. Ship the new program and the 9th account together.
3. Do not upgrade the program while clients still send 8 accounts. Those registrations fail with 6048 and create no pool record.
4. Do not send the 9th account to the current program before step 1. The current program increments an existing QuoteStat, and it still creates count 1 when the PDA is empty.
5. After steps 1 and 2, a new pool increments its quote by one. Repeating a pool does not. No per-launch recount.

## Tests

Offline only. `cargo test -p fused_fee --offline --lib` and `--test quotestat_register` against the new ELF. Real Meteora was not called. Claim, pay, and release function bodies are unchanged against `80060ab`; release and one holder payment were executed.

Library: `test_id`, `economics_constants_unchanged`, `quote_stat_discriminator_matches_manual_layout`, `register_pool_instruction_discriminator`. All passed.

Bank evidence, verbatim:

```
new-quote baseline 0, two new pools -> 2
repeat same pool rejected=true count=2
cannot retarget pool quote rejected=true
wrong-quote counter err=Some(6002)
initialized quote baseline 13 plus one register -> 14
existing-account count 7 plus one register -> 8
legacy 8-account err=Some(6048)
empty QuoteStat err=Some(6049)
readonly QuoteStat err=Some(6000)
wrong-owner err=Some(6002)
malformed tag err=Some(6002)
wrong mint bytes err=Some(6002)
short account err=Some(6002)
overflow err=Some(6006) pool_absent=true
stranger sync rejected=true
stranger register ok=true count=3
release ok burn_left=0 operator=2000 holder_before=9000 closed_cursor_paid=0 next_round holder_left=0 dest=9000
signed-bytes website8=505 proposed9=538 packet_cap=1232
```

`stranger sync` fails with Anchor `ConstraintHasOne` (2001). A non-admin who pays can still register, and that registration increments once. The first pay page only creates the cursor; a brand-new cursor is closed for the round it is created on, which is the existing rule. The next round paid 9000 of the 9000 holder pot. The 2000 burn pot was released to the operator.

The experimental vault branch `cursor/exp-vault-compare-a4ab` is still `8de35af7d16d7d28521ecc00c62cd8bea87caa6a`. It was not rebased or changed.
