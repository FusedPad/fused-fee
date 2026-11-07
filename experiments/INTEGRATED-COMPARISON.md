# Integrated local comparison

https://github.com/FusedPad/fused-fee/pull/3

These are experimental copies of `fused_fee`. Neither design is selected. Nothing here was deployed, spent, or sent to a public cluster. The earlier toy programs and `INTEGRATION-GAP.md` stay as prior evidence. The hashes in the previous revision of this note (`47360f8d…` / `1f23a817…`, and the untested 3Evo files `2f540943…` / `e8c8861a…`) are superseded by the binaries below.

Both copies keep the seeds `global`, `pool`, `qstat`, and `cown`. The global PDA is still the Meteora fee-claimer and still signs the claim into the global quote vault. Pool records stay at `["pool", pool]`. Economics are unchanged: curve claims split 20/110 to burn and the rest to holders; a graduated claim sets aside 8/100 for the creator when that creator has no DAMM LP, then applies 20/110 to the rest; auto-LP still takes 5000 bps of the burn pot.

## What the bank actually ran

SPL Token and Token-2022 inside the bank are the real programs. Meteora is `harness/mock_meteora.so`, loaded at both the DBC and DAMM ids. The mock claim moves the requested quote amount. The mock position-fee claim always moves 11000. The mock add-liquidity moves the two thresholds and sets a byte on the position. The mock permanent-lock sets a second byte. That is not a mainnet Meteora lock, and an escrow transfer is not permanent LP.

The same cases were executed twice: once on the experiment ids, and once on separate local banks whose only program id was `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`. The 3Evo runs are not inferred from the other ids. Compute units differ between those builds.

### Experiment ids

| Case | Result | Compute units |
| --- | --- | --- |
| Shared: seal, mock claim 9000/2000, pay v3, release v2 | pass | 15161, 39247, 43764, 30889 |
| Shared: register without the reserve | 6050 `ReserveMissing` | |
| Shared: historical 8-account register (quote only, no QuoteStat) | 6048 `QuoteStatRequired` | |
| Shared: claim before seal | 6052 `ReserveNotLive` | |
| Shared: second import of the same pool during backfill | 6053 `AlreadyImported` | |
| Shared: register during backfill, then seal | 6051 until that pool is imported, then seal succeeds | |
| Shared: import after seal | 6052 | |
| Shared: claim with the reserve omitted, and with a substituted account | 6050, then 6051 | |
| Shared: two claims in one transaction, the second names a missing pool | both roll back; pot and vault stay 0 | |
| Shared: pay v2 pays 18000 from two claims | pass | 41277 |
| Shared: vault lowered by 1 with `set_account`, then release v1 | 6034 `VaultShort`, burn pot unchanged. This shortfall was manufactured. It is not a normal-sequence hole | |
| Shared: direct burn clears the burn pot | pass | 23746 |
| Shared: mock DAMM claim | holder 8280, burn 1840, creator owed 880 | 57058 |
| Shared: creator claim pays 880; release v2 pays the 1840 burn | pass | 40409, 29387 |
| Shared: DAMM claim without the tail accounts | rejected | |
| Shared: Token-2022 pay of 10000 at 1% | source debited 10000, destination received 9900, fee 100 | 35755 |
| Shared: auto-LP with 24 pools present, 2 listed | 24 accounts, 872 message bytes, 97551 CU. A 24-pool list is 68 accounts and 2324 bytes and was not sent | |
| Shared: duplicate pool in that auto-LP list | 6045 | |
| Shared: open a two-quote LP reservation, refund, refund again | second refund 6054. Vaults and burn pots return to the pre-open amounts | |
| Shared: consume that reservation, then consume again | second consume rejected. Each quote's 500 units landed on a DAMM-authority token account | |
| Per-pool: claim leaves global at 0 and the pool vault at 11000; pay v3 pays from the pool vault | pass | 47705, 41230 |
| Per-pool: historical 8-account register | 6048 | |
| Per-pool: two claims in one transaction | pool vault stays 0 | |
| Per-pool: pay v2 pays 9000 from the pool vault; global stays 0 | pass | 40068 |
| Per-pool: manufactured 1-unit short, then release v1 | 6034, burn stays 2000. Preexisting fixture | |
| Per-pool: release v1 and direct burn both leave the global vault at 0 | pass | 17586, 18089 |
| Per-pool: mock DAMM claim, creator claim of 880, release v2 of 1840 | pass, and the global vault is 0 after the claim | 56507, 27399, 20874 |
| Per-pool: Token-2022 1% pay | debited 10000, received 9900, fee 100, from the pool vault | 45219 |
| Per-pool: auto-LP with 24 pools present, 2 listed | 24 accounts, 872 bytes, 105994 CU. A full list is 90 accounts and 3050 bytes | |
| Per-pool: reservation refund twice, then consume twice, across two quotes | same exactly-once result. Omitted pool burn stayed 2000 | |
| Token-2022 mint with no extension | `check_quote_mint` passes | 2908 |
| Token-2022 fee above 1%, and an active hook | 6032 and 6029 | |

The Token-2022 pay seeded the pot with `set_account` because the mock claim uses classic SPL `Transfer` and cannot fund a fee mint. The debit, the 100-unit fee, and the 9900 credit are the real Token-2022 program.

### 3Evo id, executed locally

Same cases, one design per bank. Shared auto-LP on this id used 103563 CU. Per-pool auto-LP used 121018 CU. The 1% fee pay debited 10000 and credited 9900 on both. The 8-account register returned 6048 on both. Rows are in `experiments/results/integrated-3evo-shared-core.json`, `integrated-3evo-shared-rest.json`, `integrated-3evo-per-core.json`, and `integrated-3evo-per-rest.json`.

## Binaries

Release profile on both experiment crates: `overflow-checks`, `lto = fat`, `codegen-units = 1`, `opt-level = z`. That is the same shape as the production profile. Guards and tests were not removed to chase a smaller file.

| Build | Program id | sha256 | Bytes |
| --- | --- | --- | --- |
| Shared reserve | `C1nBL5ufPcQvjkmj6hAmRgoLntHonMuf5WdxVxJenJaE` | `1aa04cecefde0eb2ed3d6bfdca244bd754e7b395b8db98b6defdf3cfe5671c21` | 592760 |
| Per-pool vault | `C5hUUPNfyui9sr2EXzVgiZa733W1TRbUdvJcZLMFXdtw` | `246f4e7b4cebf4e12516946163e6bd92bfab8fa56f7f5e78d8da5422dad4e398` | 563040 |
| Shared, 3Evo id, executed | `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` | `9b903d8003a460cab19aa584c8774b9ac06c3e688dd741c2389cc5a68bc790a8` | 594040 |
| Per-pool, 3Evo id, executed | `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` | `1447e565641b329eddfe384fb0deb17a82a18e73d667e8157aff1b0050092996` | 564320 |

The 3Evo files are `experiments/artifacts/fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` and `fused_fee-3evo-PER-POOL-experimental-DO-NOT-UPLOAD-YET.so`. They replace the older untested artifacts of the same names. Do not upload them.

Buffer rent is `(128 + elf_bytes) * 3480 * 2`. Programdata extend past the recorded 522085 bytes is `(elf + 45 - 522085) * 3480 * 2`. Against the previously recorded rehearsal balance of 5.073546881 SOL, which was not read again in this run:

| File | Buffer | Extend | Peak if both are funded together |
| --- | --- | --- | --- |
| 3Evo shared | 4.135409 SOL | 0.501120 SOL (72000 bytes) | 4.636529 SOL |
| 3Evo per-pool | 3.928558 SOL | 0.294269 SOL (42280 bytes) | 4.222827 SOL |

One of those peaks, by itself, is under the old recorded balance. Both buffers at the same time are about 8.06 SOL and do not fit. The extend is not refundable. The 530000-byte / 3.70 SOL buffer plan still does not cover either file. No buffer was created and no upgrade was sent.

## Account ceiling

`crank_auto_lp` no longer treats a full quote-group scan as the way to scale. Each call accepts 1 to 4 pools. The shared copy reads the quote's liability from `["qres", mint]`, not from the pools that happened to be listed. The per-pool copy moves only the listed pools' LP budgets out of those pools' own vaults. An omitted pool's burn pot stayed 2000 on both designs while 24 pools existed.

A synthetic message that listed every pool was measured and not submitted: 68 accounts / 2324 bytes on the shared layout, 90 accounts / 3050 bytes on the per-pool layout. Both are past the 1232-byte packet and the 64-account lock limit. The executed call was 24 accounts and 872 bytes.

## Reservation

`open_lp_reservation`, `consume_lp_reservation`, and `refund_lp_reservation` are experimental. A batch id is `["lpbatch", id]`. Open moves at most four pools' LP budgets (5000 bps of each burn pot) into escrow ATAs owned by that batch, and it reduces those burn pots immediately so the same tokens cannot be paid out. Consume sends each escrow amount once to a token account owned by the DAMM pool authority. Refund sends it back and restores the pots. A second consume and a second refund fail. One batch in the test held two quote mints; each mint's vault plus escrow plus destination was conserved, and a pool that was not in the batch was unchanged.

Consume does not call the mock add-liquidity. `crank_auto_lp` does. The reservation transfer is classic SPL `Transfer`. It is not the Token-2022 fee path, and it is not a permanent lock.

## Caller matrix

Historical register is 7 named accounts plus the quote mint at `remaining_accounts[0]`. Both builds reject that call with 6048. That was executed on the experiment ids and on the 3Evo id. A shared register that includes QuoteStat and omits the reserve fails 6050.

Shared claims, pay v2, pay v3, both releases, direct burn, the DAMM tail, and creator claim take the reserve account in the position named below. Until `seal_quote_reserve`, a claim fails 6052. Pay v3 of an empty page still returns before it reads the reserve; a page that would move tokens checks the reserve. The empty-page case was not the rejection under test.

| Instruction | Shared accounts that changed | Per-pool accounts that changed |
| --- | --- | --- |
| `initialize` | unchanged: admin, global, system | same |
| `init_quote_reserve` | new. admin, global, `["qres", mint]`, mint, system. State is backfill and no tokens move | not present |
| `import_quote_reserve` | admin must be writable. Remaining: `["qimp", pool]` and system. A second import while backfilling is 6053. Import after seal is 6052 | not present |
| `seal_quote_reserve` | requires `pool_count == imported` and vault >= holder + burn + owed. A pool registered during backfill blocks the seal until it is imported | not present |
| `register_pool` | remaining: quote, QuoteStat, reserve | remaining: quote, QuoteStat. No reserve |
| `crank_claim` | remaining[0] is the live reserve | remaining[0] is the pool vault. The claim still lands in the global ATA, then the delta moves | 
| `crank_claim_damm` | the old 5 tail accounts, then the reserve | the old 5, then the pool vault |
| `claim_creator_share` | the old 8, then the reserve. Pays the global vault | the old 8, and the vault must be the pool vault |
| `crank_pay_page_v2` | `quote_reserve` after the native mint, before holder pairs | `quote_vault` is the pool vault. Transfers sign with `["pool", pool, bump]` |
| `crank_pay_page_v3` | `quote_reserve` is the last named account | no reserve. The vault owner is the pool record |
| `release_burn_pot` | remaining[0] is the reserve | the vault owner is the pool record |
| `release_burn_pot_v2` | reserve is the last account | the vault is the pool ATA |
| `crank_burn_direct` | remaining[0] is the reserve | the vault owner is the pool record |
| `crank_auto_lp` | reserves at indexes 18 and 19, then pairs of pool and creator-owed. 1 to 4 pools | triples of pool, pool vault, creator-owed. 1 to 4 pools |
| `open_lp_reservation` | groups of pool, global ATA, mint, escrow, reserve | groups of pool, pool vault, mint, escrow |
| `consume_lp_reservation` | pairs of escrow and a DAMM-authority destination | same |
| `refund_lp_reservation` | groups of escrow, source, pool, reserve | groups of escrow, source, pool |

`init_auto_lp` and `set_auto_lp` keep their old accounts. The new reservation reads the auto-LP share from that PDA.

## Unresolved

- Neither design is chosen. Auto-LP stays off on production. Production `declare_id` was not edited.
- The mock is not mainnet Meteora. A later rehearsal would still be required before a lock is treated as real.
- The LP reservation and `crank_auto_lp` are both bounded to 4 pools. The reservation does not itself call add-liquidity.
- The 1% fee transfer was a real Token-2022 pay. The pot it paid was seeded, because the mock claim cannot fund a fee mint.
- Import was tested on local fixtures, including a pool registered while the reserve was still in backfill. The 19 live quote vaults were not imported.
- Both ELFs are still over 530000 bytes. The 3.70 SOL buffer plan does not cover them. The previously recorded 5.07 SOL balance was not read again. No deployment was attempted.
- The two designs cannot be buffered at the same time on that balance. An extend is permanent.
