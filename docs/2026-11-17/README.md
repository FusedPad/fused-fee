# 17 November 2026 mainnet record

All times Europe/Jersey (UTC+0). This is a completed-work record, not permission for another deployment. Website rollout is separate and not included here.

## Deployed program and public source

- Program: `AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9` (mainnet).
- Upgrade at 10:59:32, finalized slot `470508982`; full transaction in [upgrade receipt](upgrade-receipt.md).
- Adds the on-chain security.txt card and `crank_pay_page_v4` for plain-SOL holder payouts with a Token-2022 base and wSOL quote. Existing pay paths, fee economics and burn rules remain unchanged.
- Public source commit: `76b2daff84e4d94b6b268c12c04cdc70f6910714`.
- It is byte-for-byte tree-identical to the deployed local source commit `16ce6345799f363b5558de410bf5e143a3d3ae08`: Git tree `332c21a404375b489e604edb22cb2b5406273a41`. Only the two local commits' author/committer identities were replaced with the public noreply identity before publication. The local history remains untouched.
- Full ELF: 552248 bytes, SHA-256 `2b8785843a74e5ca1b65d99477c28027603a99884aaa2041268b308843906bed`.
- Trimmed ELF: 552233 bytes, SHA-256 `d3feac6092d635915ce9b39621beea952dc7976dd88db14069a60136f6c98c08`.
- Authority unchanged. No further program upgrade was performed for verification.

## Official verifiable build and OtterSec

Built from the public source commit above using solana-verify 0.5.2, Docker image `solanafoundation/solana-verifiable-build@sha256:695f890e620db8c39afe5112e048599f8ee395a0cab5a2e572f30a72c6366cb4`, and `--cargo-build-sbf-args="--tools-version v1.58"`.

```bash
solana-verify build --library-name fused_fee \
  --base-image solanafoundation/solana-verifiable-build@sha256:695f890e620db8c39afe5112e048599f8ee395a0cab5a2e572f30a72c6366cb4 \
  --cargo-build-sbf-args="--tools-version v1.58" .
solana-verify get-executable-hash target/deploy/fused_fee.so
solana-verify get-program-hash -u https://api.mainnet-beta.solana.com \
  AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9
```

Docker build ran 14:14:51–14:19:49. Full and trimmed hashes matched the deployed ELF exactly before any verification transaction was signed.

- Verification PDA update (one transaction): `HgifNYCovEXTuJbJxMBP3mNuf7Y9Vp9Xe2sZ56m2nnrczSGYBWf9xve5jW7xXt51Q5S7bVfnL6x3Ye4nUpdmeTj`.
- Cost: 25300 lamports (0.0000253 SOL). Existing PDA updated; no new rent.
- OtterSec job: https://verify.osec.io/job/a772714f-7d41-4979-af7c-6423e594828d
- At 14:24:38 the service reported **is_verified=true**, with both hashes equal to `d3feac60…98c08` and repo commit `76b2daf`. See [saved status](ottersec-status.json).
- Live status: https://verify.osec.io/status/AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9
- Solscan page: https://solscan.io/account/AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9#anchorProgramIdl . Solscan's separately rendered badge has not been independently inspected.

## 57 Token-2022 launch configs

All 57 ConfigRecords now target the new `token_type=1` DBC configs. Full old/new map: [57-row CSV](NEW-CONFIGS-for-Website-57.csv).

- First 56 non-SOL configs: 12:39–12:54; [receipt](non-sol-config-receipt.md), [per-pair transactions](non-sol-config-receipts.csv). Cost 0.335202284 SOL.
- SOL, the 57th: 13:13:45, finalized slot 470546492; [receipt](sol-config-receipt.md). Cost 0.005985589 SOL.
- Total configs cost: 0.341187873 SOL. All finalized read-back checks passed.
- Target curve at creation prices: $4000 opening MC, $9000 quote raise, about $17964 permanently locked LP after migration's 0.2% fee. The enforced bounds are $3000–$5000 open, at least $8000 raised, at least $16000 locked LP; prices can drift. Figures are not fixed USD guarantees after creation.
- DAMM v2, 100% partner permanently locked LP, 1.5% fee, 8% creator share. Fee claimer and leftover receiver are the Global PDA.
- ConfigRecord repoints did not modify the old configs or existing pools. The old SOL config `Fdz9izTjXKicQiosBcQCFgv31mcthJcBhDb6qhERbrbN`, including the existing $FUSED pool, remains intact.

## Crank readiness note

The crank operator reported PAY_V4_WSOL_T22 enabled at 13:03, after taking a backup. Program v4 and the deployed hash are allowed. Non-wSOL Token-2022 pools use v3; wSOL Token-2022 pools use v4. Old pools continue through their existing compatible paths. No crank source or private configuration is published here.

No Token-2022/wSOL pool existed at the flag change. A live Token-2022-base payout has not yet been observed. This record is not a promise of payout timing. New pools still require on-chain `register_pool` and a PoolRecord; changing website metadata or a database alone does not register a pool.
