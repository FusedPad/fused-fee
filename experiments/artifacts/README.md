# Artifacts

`fused_fee-3evo-LAYOUT-COMPATIBLE-not-a-vault-redesign.so` is a local build of the current fee-program source with only the program id set to `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`. It keeps the existing account seeds (`global`, `pool`, `qstat`, `cown`). It is not a vault redesign.

`fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` (594040 bytes, sha256 `9b903d8003a460cab19aa584c8774b9ac06c3e688dd741c2389cc5a68bc790a8`) and `fused_fee-3evo-PER-POOL-experimental-DO-NOT-UPLOAD-YET.so` (564320 bytes, sha256 `1447e565641b329eddfe384fb0deb17a82a18e73d667e8157aff1b0050092996`) are the pinned rehearsal-id builds. Immutable copies and the run9 report are in `experiments/exports/run9-7365d21ef0d2/`. A file of 711144 or 678040 bytes at these names is stale. Do not upload until approved. Devnet rent for these exact sizes is the set Crank reported; the old 6960-lamport-per-byte costs are withdrawn.

Do not upload the experimental `xa` / `xb` programs onto the funded 3Evo account. Their seeds are different, so that upgrade would leave the current vaults without a program that can sign for them.

Nothing in this folder has been deployed.
