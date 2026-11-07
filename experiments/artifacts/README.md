# Artifacts

`fused_fee-3evo-LAYOUT-COMPATIBLE-not-a-vault-redesign.so` is a local build of the current fee-program source with only the program id set to `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`. It keeps the existing account seeds (`global`, `pool`, `qstat`, `cown`). It is not a vault redesign.

`fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` is the run11 shared build, 609296 bytes, sha256 `659a899e2e6fdd1edd8dfec2a70a457c6f7d25d795155a7836a1f6751dfe5c93`. The per-pool file is 582240 bytes, sha256 `ee74870f323d797cb6d7b45aabaf4e7b7122333d27255cf14993a81f74f7f4c3`. Do not upload. The handoff is `experiments/exports/run11-handoff/`. Crank's slot 519510577 rents apply to the c2083f7 sizes and to the 82/40/564-byte accounts, not to these ELF lengths. Run9 hashes `9b903d80…` and `1447e565…` remain unsafe.

Do not upload the experimental `xa` / `xb` programs onto the funded 3Evo account. Their seeds are different, so that upgrade would leave the current vaults without a program that can sign for them.

Nothing in this folder has been deployed.
