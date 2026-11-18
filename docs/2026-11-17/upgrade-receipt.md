# MAINNET upgrade of fused_fee AZY2 → 16ce634, 17 Nov 2026 — DONE, verified

- **Approval:** Operator (10:57 Europe/Jersey / UTC) approved runbook steps 1–5 only.
- **At upgrade time:** no configs or re-verification had run. See README.md for the later same-day completed work.
- **Where it ran:** on the operator machine.
- **Signer:** D8pJ, used only as the `--keypair` path; the key file was never opened, read, copied or hashed.
- **RPC:** pinned to https://api.mainnet-beta.solana.com, with the mainnet genesis checked.
- **Crank windows:** every write ran between 10:59:09 and 10:59:32 UTC, outside the crank windows.

## Preflight (10:58:19 UTC)

| Check | Result |
|---|---|
| Package `.so` sha256 | 2b878584…6bed, matches SHA256SUMS |
| Program data length | 521,080 |
| Authority | D8pJ |
| D8pJ balance | 3,391,444,050 lamports (≥ 2.98 SOL) |
| Buffers for D8pJ | none |

ProgramData: `2xjp8tUswiUWYzH5DkDn4zTm6ZRtA9z6Eq9Bx6bAbH7D`. It was last deployed in slot 466,736,401.

## Transactions

| Step | Signature | Slot | Time | D8pJ after (lamports) |
|---|---|---|---|---|
| Extend +31,168 B → 552,248 | 23Hcus2WGPFVQafuNBT2zwtqu237F5EbnEdi9UmK33eg1zjy5MMUiuh9HLkpEntdCjcBPzHTWP1DjZcgbiQZmVM9 | 470,508,886 | 10:59:10 | 3,233,105,610 |
| Buffer create + writes (577 tx, 0 errors) | list in `mainnet-sigs-ours.txt` | 470,508,8xx–973 | 10:59:10–10:59:30 | 423,901,349 |
| Upgrade | mgsC3rbrGKP4jJfrfxXXSGzBFwjbWX83xzSC6JLj76YW1SQ3rTn468zsCH2dn222qH6jNxqq2sAniiehFT9BVi2 | **470,508,982** (finalized) | 10:59:32 | 3,230,195,029 |

- The buffer was `DNoCPzyEHHUC3rqQNFhmGGFFEPqyKc7LT81FZtt5rhpN`.
  - It had a fresh keypair.
  - Before the upgrade, its dumped bytes matched the build: sha 2b878584….
  - The upgrade consumed it.
  - **No buffer is left:** the D8pJ buffer list is empty.
- In total, D8pJ signed 579 transactions from 10:59:00 to 10:59:35 UTC, and none failed.

## Verify (10:59:41 UTC)

| Check | Result |
|---|---|
| Data length | 552,248, with ProgramData balance 2.80629868 SOL |
| Dump size | 552,248 B |
| Full sha256 | **2b8785843a74e5ca1b65d99477c28027603a99884aaa2041268b308843906bed** = build; 0 nonzero tail bytes |
| Trimmed sha256 | **d3feac6092d635915ce9b39621beea952dc7976dd88db14069a60136f6c98c08** = the crank's PROGRAM_SHA_V4 / PAY_V4 entry |
| Authority | still D8pJ |

The dump was re-checked on the box with the same results.

security.txt is present and its fields read:
- name = FusedPad fee program
- project_url = https://fusedpad.com
- contacts = email:hello@fusedpad.com
- policy = Please report security issues privately to hello@fusedpad.com. Do not exploit or publicly disclose a vulnerability before it is fixed.
- source_code = https://github.com/FusedPad/fused-fee

## SOL

| | Lamports | SOL |
|---|---|---|
| D8pJ before | 3,391,444,050 | |
| D8pJ after | 3,230,195,029 | |
| **Total spent** | **161,249,021** | **0.161249** |
| Extend (rent + fee, permanent, now in ProgramData) | 158,338,440 | |
| Buffer tx fees, net of the full buffer refund at the upgrade | 2,910,581 | |

At 10:59:54 UTC an unrelated transaction (2EVT8RJC…, paid by EnQEnS7a…) sent D8pJ 1 lamport, so D8pJ now shows
3,230,195,030. This was not ours; it looks like dust.

