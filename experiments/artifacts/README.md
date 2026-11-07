# Artifacts

`fused_fee-3evo-LAYOUT-COMPATIBLE-not-a-vault-redesign.so` is a local build of the current fee-program source with only the program id set to `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`. It keeps the existing account seeds (`global`, `pool`, `qstat`, `cown`). It is not a vault redesign.

`fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` is now the run10 shared build, 598008 bytes, sha256 `3deff208a647b64d6fa5fa11a9060821cfacdd16b93f2b012327cb6d8c91ec03`. The per-pool file is 568552 bytes, sha256 `60d036ae8a8a2fd0ffd92cba67378cfc9d6be795bb441434d07a9221d9ba9265`. Do not upload until approved. The run9 hashes `9b903d80…` (594040 bytes) and `1447e565…` (564320 bytes) remain in `experiments/exports/run9-7365d21ef0d2/` and are unsafe: their reservation consume paid an unbound DAMM-authority account. Crank's slot 519505861 rent minima apply only to those unsafe sizes.

Do not upload the experimental `xa` / `xb` programs onto the funded 3Evo account. Their seeds are different, so that upgrade would leave the current vaults without a program that can sign for them.

Nothing in this folder has been deployed.
