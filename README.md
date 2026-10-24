# fused-fee

Source code of the FusedPad fee program on Solana mainnet:
`AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9`

Anchor 0.31.1, built with Solana/Agave 2.3 platform tools v1.58 (`opt-level = "z"`, fat LTO).

## Verifiable build

```
solana-verify build --library-name fused_fee \
  --base-image solanafoundation/solana-verifiable-build:2.3.0 \
  --cargo-build-sbf-args="--tools-version v1.58"
solana-verify get-executable-hash target/deploy/fused_fee.so
solana-verify get-program-hash AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9
```

The two hashes must match.

## Security

Email hello@fusedpad.com.
