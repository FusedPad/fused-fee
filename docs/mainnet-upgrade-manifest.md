# Mainnet upgrade manifest

Public record of the QuoteStat counter upgrade. Docker verification and the Solscan badge are pending. This file does not authorize another deployment.

## Deployed build

| | |
|---|---|
| Program | `AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9` |
| Source | `29d0811b703f435a8e98723c547c0a3f5ef3d9e7` |
| Repository | https://github.com/FusedPad/fused-fee/commit/29d0811b703f435a8e98723c547c0a3f5ef3d9e7 |
| Full ELF | 520272 bytes, `968c6eb26274082c49603c82d95463bcb620cf9f8b9cf8351eca1da8d66a2001` |
| Trimmed ELF | 520257 bytes, `89492f8f20af3b976cc8921720e6bf55882714ae4df998958a7a211dc0a58e64` |
| Upgrade transaction | `5dw36191AEABqR6UimUPc4dEDKqxfVgnEwL6UyLewVGYoU4VoQUSssUQukJmXa6WZYLAnh2adgcMMiDXTW9ipXHC` |
| Slot | 466736401 |

The live program account is 521080 bytes. Its first 520272 bytes match the full ELF. The tail is zeros. The trimmed hash is the first 520257 bytes. A public read of `https://verify.osec.io/status/AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9` on 8 Nov 2026 returned `is_verified: false`, on-chain hash `89492f8f20af3b976cc8921720e6bf55882714ae4df998958a7a211dc0a58e64`, and executable hash `35953c2c65ee8935823eaad0030bdc56fe675e67f9010e50fc21131e60f85a3d` for commit `80060ab46f77c6b46b4c9c1f65918ae9e356dbfc`. That is the previous verification record. It is not a Solscan verified badge for this upgrade.

The candidate hashes above came from a local `cargo-build-sbf` build. They are not a Docker attestation. No Docker build was run for this manifest.

## Verification, not yet run

Do this on the operator's own Ubuntu machine after that machine is available. Do not use another builder. Do not sign, and do not spend, until the hashes match and a later decision approves the PDA write.

Install Docker and `solana-verify` 0.5.2 (`cargo install solana-verify --version 0.5.2`). The historical build inputs, from an older baseline note, are image `solanafoundation/solana-verifiable-build:2.3.0` digest `695f890e620db8c39afe5112e048599f8ee395a0cab5a2e572f30a72c6366cb4` and `--tools-version v1.58`. Those inputs are not proof that this source will hash to `968c6eb2…` or `89492f8f…`.

```bash
solana-verify build \
  --library-name fused_fee \
  --base-image solanafoundation/solana-verifiable-build:2.3.0 \
  --cargo-build-sbf-args="--tools-version v1.58"
solana-verify get-executable-hash target/deploy/fused_fee.so
solana-verify get-program-hash -u https://api.mainnet-beta.solana.com \
  AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9
```

Stop if the executable hash is not `89492f8f20af3b976cc8921720e6bf55882714ae4df998958a7a211dc0a58e64` or the full file is not `968c6eb26274082c49603c82d95463bcb620cf9f8b9cf8351eca1da8d66a2001`. Do not deploy again to obtain a badge.

The current public workflow is documented at https://solana.com/docs/programs/verified-builds and https://github.com/solana-foundation/solana-verifiable-build. `solana-verify` 0.5.2 is the current crates.io release: https://crates.io/crates/solana-verify/0.5.2. Remote status is `GET https://verify.osec.io/status/<PROGRAM_ID>` from https://github.com/otter-sec/solana-verified-programs-api. Solscan shows that remote result. A matching dump is not the badge.

`verify-from-repo` defaults to the latest commit on the default branch. This commit is not on `main` yet. A later upload has to pass `--commit-hash 29d0811b703f435a8e98723c547c0a3f5ef3d9e7`, `--library-name fused_fee`, and `--mount-path programs/fused_fee`. The command then asks whether to write the verification PDA. Only the program upgrade authority can sign that write. The PDA is owned by `verifycLy8mB96wd9wqq3WDXQwM4oU6r42Th37Db9fC`. Creating it spends rent. The guide does not state the lamport amount. Do not sign until that cost is approved. After the PDA exists, `solana-verify remote submit-job --program-id AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9 --uploader <upgrade-authority>` queues the OtterSec job. The legacy `--remote` flag on `verify-from-repo` is deprecated.

Until that job reports verified, this program is not Solscan-verified for commit `29d0811`.
