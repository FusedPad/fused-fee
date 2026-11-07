# Artifacts

`fused_fee-3evo-LAYOUT-COMPATIBLE-not-a-vault-redesign.so` is a local build of the current fee-program source with only the program id set to `3EvoYD7bjAgr5evMJDbsLdyNU32zXgKiBycKruojYxpD`. It keeps the existing account seeds (`global`, `pool`, `qstat`, `cown`). It is not a vault redesign.

`fused_fee-3evo-SHARED-RESERVE-experimental-DO-NOT-UPLOAD-YET.so` and `fused_fee-3evo-PER-POOL-experimental-DO-NOT-UPLOAD-YET.so` are the integrated experiments rebuilt with only `declare_id` set to the rehearsal id. They were executed in local banks. They are not approved to upload. The older bytes that used to sit at these filenames were not executed and have been replaced.

Do not upload the experimental `xa` / `xb` programs onto the funded 3Evo account. Their seeds are different, so that upgrade would leave the current vaults without a program that can sign for them.

Nothing in this folder has been deployed.
