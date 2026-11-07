# Artifacts

`fused_fee-3evo-LAYOUT-COMPATIBLE-not-a-vault-redesign.so` is a local build of the current fee-program source with only the program id set to `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`. It keeps the existing account seeds (`global`, `pool`, `qstat`, `cown`). It is not a vault redesign.

`fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` is the run13 shared build, 621672 bytes, sha256 `d87ce272c943bb7b115cc7c2555f1977e38244d2beac576d727d4a7347f834b6`. The per-pool file is 607824 bytes, sha256 `225c7768650666540c5f66d82ea2c4ac2be7567f9189ff998648abc1cc63fba9`. Do not upload. `b3911bc` is not a deploy candidate. The handoff is `experiments/exports/run13-cown-abi/`. Run9 hashes `9b903d80…` and `1447e565…` remain unsafe.

Do not upload the experimental `xa` / `xb` programs onto the funded 3Evo account. Their seeds are different, so that upgrade would leave the current vaults without a program that can sign for them.

Nothing in this folder has been deployed.
