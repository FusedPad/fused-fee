# Artifacts

`fused_fee-3evo-LAYOUT-COMPATIBLE-not-a-vault-redesign.so` is a local build of the current fee-program source with only the program id set to `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`. It keeps the existing account seeds (`global`, `pool`, `qstat`, `cown`). It is not a vault redesign.

`fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` is the run12 shared build, 620032 bytes, sha256 `498233a22757af4fd5fc6e6fe05025f64b9ac278fc0691d03e86448c6f448c8e`. The per-pool file is 606400 bytes, sha256 `a0d1a6167cafd25f132878d83ddddc2067373c3f46d4e70d73db2509d2376723`. Do not upload. The handoff is `experiments/exports/run12-handoff/`. Run9 and run11 files are unchanged. Run9 hashes `9b903d80…` and `1447e565…` remain unsafe.

Do not upload the experimental `xa` / `xb` programs onto the funded 3Evo account. Their seeds are different, so that upgrade would leave the current vaults without a program that can sign for them.

Nothing in this folder has been deployed.
