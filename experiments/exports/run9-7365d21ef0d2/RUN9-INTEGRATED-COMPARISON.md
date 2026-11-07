# RUN9 integrated comparison

UNSAFE EXPERIMENTAL. Do not upload these ELFs and do not treat the harness row count as safety passes. Commit `399c08b` recorded `binding_holds=false`: a token account owned by the DAMM pool authority, and not the configured auto-LP vault, received 250 on both reservation designs. The hashes `9b903d8003a460cab19aa584c8774b9ac06c3e688dd741c2389cc5a68bc790a8` (594040 bytes) and `1447e565641b329eddfe384fb0deb17a82a18e73d667e8157aff1b0050092996` (564320 bytes), plus experiment-id `1aa04cecefde0eb2ed3d6bfdca244bd754e7b395b8db98b6defdf3cfe5671c21` and `246f4e7b4cebf4e12516946163e6bd92bfab8fa56f7f5e78d8da5422dad4e398`, are that unsafe build. The replacement is `experiments/exports/run10-binding/`.

Superseded warning. Any earlier `INTEGRATED-COMPARISON.md` that lists binaries of 708808 or 675704 bytes, hashes `47360f8d` or `1f23a817`, 3Evo files of 711144 or 678040 bytes, hashes `2f540943` or `e8c8861a`, buffer figures of 4.95 or 4.72 SOL, or a permanent extension of 0.501120 SOL is stale. The 6960-lamport-per-byte formula used for those costs is not the current devnet rent. The figures below are devnet-only. They are not mainnet prices.

This file is the run9 pin. Machine-readable companion: `experiments/exports/run9-7365d21ef0d2/MANIFEST.json`. ELF source commit `7365d21ef0d2fdb62b76480f695ce52ab770580f`. The ELFs were not rebuilt after that commit.

Nothing in this run was deployed or spent. Neither design is selected.

## Crank's devnet read

Attributed to Crank's paced read-only Helius DEVNET RPC. This agent did not query the cluster.

| Field | Value |
| --- | --- |
| Commitment and slot | finalized slot 519505861 |
| Programdata | `CeFRF4rmjsUh17dfHf835xP9d4yf5iHrFxb8hD3dpFqa` |
| Programdata space | 522085 |
| Programdata lamports | 2.652842040 SOL |
| Upgrade authority | `HiRkRrxCgGzLV3WqKu7HxibS5kTVtDD6LW1vLKSH9Nxw` |
| Wallet balance | 5.073546881 SOL |
| Rent minimum he reported for 594040 + 45 bytes | 3.018602040 SOL |
| Rent minimum he reported for 564320 + 45 bytes | 2.867624440 SOL |

He described the buffer as approximately those same rent minima. Exact loader-buffer overhead is still unverified. He will recompute the envelope from this manifest, keeping a 0.5 SOL reserve and room to restore state.

## What those rent minima imply

Differences of the rent minima he reported:

| Step | SOL |
| --- | --- |
| Permanent top-up if programdata grows to the shared size (3.018602040 − 2.652842040) | 0.365760000 |
| Permanent top-up if programdata grows only to the per-pool size (2.867624440 − 2.652842040) | 0.214782400 |
| Further top-up to grow from the per-pool size to the shared size | 0.150977600 |

Programdata does not shrink. After a rehearsal that has installed the shared build, the permanent extension that remains is 0.365760 SOL even if a later upgrade is the smaller per-pool ELF. That is the sequential maximum extension. It is not 0.365760 plus 0.214782.

A single variant's buffer is not the multi-design budget. Opening both buffers at once is about 3.018602040 + 2.867624440 = 5.886226480 SOL before the extension, fixtures, or fees. That already exceeds the reported wallet balance of 5.073546881 SOL. Closing a buffer returns that buffer's lamports. The extension stays in programdata. Signature fees and new accounts are extra and are not in his rent minima.

One shared upgrade, buffer plus its extension, is about 3.018602040 + 0.365760000 = 3.384362040 SOL before unverified buffer overhead, fixtures, and fees. Holding back his 0.5 SOL reserve leaves 4.573546881 SOL of the reported balance. This run does not call that a fit. Fixture accounts these programs can create, and cannot close, are a quote reserve of 82 bytes, an import marker of 40 bytes, an LP batch of 436 bytes, and token accounts of 165 bytes. Their devnet rent was not priced here.

## Build

`cargo-build-sbf --arch v0 --tools-version v1.58 --offline`. Each experiment crate is its own workspace. Release profile: `overflow-checks = true`, `lto = "fat"`, `codegen-units = 1`, `opt-level = "z"`. Anchor 0.31.1. That profile is what cut the earlier untested artifacts (711144 and 678040 bytes) down to the sizes below. The sbf build did not emit an Anchor IDL. Discriminators are in the manifest.

## Pinned binaries

| File | Program id | Bytes | sha256 |
| --- | --- | --- | --- |
| `run9-3evo-shared-594040-9b903d8003a460cab19aa584c8774b9ac06c3e688dd741c2389cc5a68bc790a8.so` | `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` | 594040 | `9b903d8003a460cab19aa584c8774b9ac06c3e688dd741c2389cc5a68bc790a8` |
| `run9-3evo-per-564320-1447e565641b329eddfe384fb0deb17a82a18e73d667e8157aff1b0050092996.so` | `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD` | 564320 | `1447e565641b329eddfe384fb0deb17a82a18e73d667e8157aff1b0050092996` |
| `run9-expid-shared-592760-1aa04cecefde0eb2ed3d6bfdca244bd754e7b395b8db98b6defdf3cfe5671c21.so` | `C1nBL5ufPcQvjkmj6hAmRgoLntHonMuf5WdxVxJenJaE` | 592760 | `1aa04cecefde0eb2ed3d6bfdca244bd754e7b395b8db98b6defdf3cfe5671c21` |
| `run9-expid-per-563040-246f4e7b4cebf4e12516946163e6bd92bfab8fa56f7f5e78d8da5422dad4e398.so` | `C5hUUPNfyui9sr2EXzVgiZa733W1TRbUdvJcZLMFXdtw` | 563040 | `246f4e7b4cebf4e12516946163e6bd92bfab8fa56f7f5e78d8da5422dad4e398` |

Do not upload the 3Evo files until they are approved. The older filenames `fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` and `fused_fee-3evo-PER-POOL-experimental-DO-NOT-UPLOAD-YET.so` now contain these same bytes. A copy that still has the 711144 or 678040 byte length is the stale artifact.

## Counts

| File | Rows | Harness ok | Harness fail | What it is |
| --- | --- | --- | --- | --- |
| `integrated-fee.json` | 17 | 17 | 0 | Experiment ids. Includes the 8-account register, which returns 6048 |
| `integrated-rest.json` | 36 | 36 | 0 | Experiment ids, this run. Two of those rows are the binding observation below, where the safety property failed |
| `integrated-3evo-shared-core.json` | 14 | 14 | 0 | 3Evo shared, prior execution of the pinned ELF |
| `integrated-3evo-shared-rest.json` | 14 | 14 | 0 | 3Evo shared, prior execution |
| `integrated-3evo-per-core.json` | 6 | 6 | 0 | 3Evo per, prior execution |
| `integrated-3evo-per-rest.json` | 14 | 14 | 0 | 3Evo per, prior execution |

Skip, with no row claiming a pass: fee-mint claim, fee-mint per-pool sweep, fee-mint import, wSOL unwrap, a second pay-page inside the same round, an Anchor IDL, exact buffer overhead, and devnet rent of the fixture accounts.

## Auto-LP mock and the reservation are different tests

`crank_auto_lp` on the experiment ids, with 24 pools present and 2 listed, calls the local Meteora stand-in. The stand-in moves the threshold amounts and sets two bytes on the position. That is the auto-LP mock test. It is not a mainnet lock.

Measured signed transaction, one signature, fee payer included in the account list:

| Design | Locks | Message bytes | Signed transaction bytes | CU | Unsent full-list locks | Unsent full-list signed bytes |
| --- | --- | --- | --- | --- | --- | --- |
| Shared `C1nBL…` | 24 | 872 | 937 | 97551 | 68 | 2389 |
| Per-pool `C5hUU…` | 24 | 872 | 937 | 116494 | 90 | 3115 |

A second run of the same binaries produced the same CU. The 3Evo ELFs were not re-signed in this run. Their earlier auto-LP CU on the pinned files was 103563 (shared) and 121018 (per). The omitted pool stayed at holder 9000 and burn 2000. The executed call fits in 1232 bytes and in 64 locks. The full list does not, and it was not submitted.

`open_lp_reservation`, `consume_lp_reservation`, and `refund_lp_reservation` do not call add-liquidity and do not set the lock bytes. A passing reservation test is an escrow result. It is not evidence of LP.

On both designs the reservation kept holder pots at 9000. A second consume of the same batch was rejected. A second refund was rejected. Replaying an open of an existing batch was rejected with 6027. A consume to a user-owned token account was rejected with 6027 and the escrow stayed in place. A consume to a different token account whose owner is the DAMM pool authority succeeded: that account received 250. The account was not the vault used by the auto-LP instruction. Destination binding to the configured DAMM pool does not hold. That is a blocker for treating consume as a payment to the configured pool.

## Fee-mint gap

The 1% Token-2022 pay debited 10000 and credited 9900, fee 100, on both designs. The pot for that pay was written with `set_account`. The mock claim still uses classic SPL transfer instruction 3, so this run has no fee-mint claim and no fee-mint move from the global vault into a pool vault. Those two paths are open gaps.

## Blockers

- Reservation consume pays any token account owned by the DAMM pool authority. It does not require the configured auto-LP pool or that pool's vault.
- Fee-mint claim and fee-mint migration were not executed.
- wSOL unwrap and a second cursor page in the same round were not executed.
- Exact devnet buffer overhead and fixture rent are waiting on Crank's envelope from this manifest.
- Both ELFs are still above 530000 bytes. Both buffers open together exceed the reported 5.073546881 SOL balance. The 0.365760 SOL extension is permanent.
