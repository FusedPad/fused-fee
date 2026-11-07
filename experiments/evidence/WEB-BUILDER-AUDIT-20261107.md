Read-only audit of the current tree. Nothing was changed or deployed. No key material is included. This workspace did not return a git commit (no HEAD), so the source revision below is the code as it sits now, not a tagged release.
0. Scope and labels
Verified means it is in this tree. Assumption means it is inferred and not proven by a chain read or an IDL. There is no fee-program IDL or Rust source in the repo. The client account list in registerPoolInstruction is the whole interface this app sends.
The Matchbox game (/game) is an iframe. It does not build, sign, or send launch or register transactions. The Matchbox launch page is /launch. My Cache is /lit (title “My Cache”).
1. Entry points
EntryWhat it doesShared buildersDifferences/launch MatchboxCreates and registersprepareLaunchTx → buildLaunchTransaction; register via bundleRegister or later prepareRegisterTx → registerIfMissing → buildRegisterPoolBrowser. One legacy tx if the register instruction fits; otherwise create, then a second register tx. Mint keypair is generated in the browser and passed in as baseMint. Launch-signer memo is added only if it fits, then re-signed server-side by cosignLaunchTx.POST /api/launch (no action)Returns unsigned/partially signed txs. Does not submit.prepareLaunch → buildLaunchTransaction + buildRegisterPoolAlways two legacy txs. Mint keypair is created on the server and the create tx is partialSigned by that mint and, if it fits, the launch signer. Register is not simulated (simulated: false). First buy capped at 1000. Rate limit 4/min/wallet. Writes the token file before the user signs.POST /api/launch action: "register"Register onlyprepareRegister → registerIfMissingSame builder as the coin page. Returns { already: true } if the pool record exists. Rate limit 4/min.POST /api/launch action: "record"Indexes a confirmed create. No transaction.recordLaunchConfirms the signature, checks mint+pool are in the tx and the pool creator is the wallet, inserts fuse_mints. Does not require a pool record. Rate limit 8/min.Coin page RegisterRegister onlyprepareRegisterTx → registerIfMissingShown only when readPoolRecord says the record is missing. Wallet signs; sendSigned → submitSigned.My Cache /lit RegisterMineRegister onlysame prepareRegisterTxShown when stashFor says registered === false. Signs locally, submits with submitSigned. Does not wait for confirmation before saying “Registered.”/adminBatch register of listed coins missing a recordbuildRegisterPool directlyNot prepareRegisterTx. One legacy tx, every missing coin, no priority fee, no compute limit, no size check. Config comes from readDbcConfig (on-chain pool), not the catalog.
/game is not an entry point.
All live register instructions are built only in registerPoolInstruction. Callers: bundleRegister, registerIfMissing, prepareLaunch in bot.ts, signAllLaunch in launch.tsx, and /admin.
2. Build, sign, submit
Instruction and legacy wrapper
src/lib/fuse/fee.ts
language-146:185:src/lib/fuse/fee.tsexport function registerPoolInstruction(
  payer: PublicKey,
  pool: PublicKey,
  baseMint: PublicKey,
  quoteMint: PublicKey,
  dbcConfig: PublicKey,
): TransactionInstruction {
  installBuffer();
  const configRecord = PublicKey.findProgramAddressSync([Buffer.from("config"), quoteMint.toBuffer()], PROGRAM)[0];
  const poolRecord = poolRecordAddress(pool);
  const data = Buffer.alloc(8 + 96);
  REGISTER_POOL.copy(data, 0);
  pool.toBuffer().copy(data, 8);
  baseMint.toBuffer().copy(data, 40);
  quoteMint.toBuffer().copy(data, 72);
  return new TransactionInstruction({
    programId: PROGRAM,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: configRecord, isSigner: false, isWritable: false },
      { pubkey: poolRecord, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: globalAddress(), isSigner: false, isWritable: false },
      { pubkey: pool, isSigner: false, isWritable: false },
      { pubkey: dbcConfig, isSigner: false, isWritable: false },
      { pubkey: quoteMint, isSigner: false, isWritable: false },
    ],
    data,
  });
}
export function buildRegisterPool(...) {
  return new Transaction().add(registerPoolInstruction(payer, pool, baseMint, quoteMint, dbcConfig));
}
PROGRAM is new PublicKey(FEE_PROGRAM).
Bundle, priority, simulate, “already registered”
src/lib/fuse/launch-build.ts
withPriority: if no compute-budget instruction, prepend setComputeUnitPrice({ microLamports: 100_000 }).
addSimulatedComputeLimit: min(1_400_000, ceil(unitsConsumed * 1.25)), inserted after the price instruction. If the legacy tx then serializes over 1232 bytes, that limit instruction is removed.
bundleRegister: appends the register instruction only if fits() says so.
fits: 1232 - 16, and another - 64 if the tx or the extra instruction is the Memo program. A dummy blockhash is used only for the size check.
simulateSignedOff: stamps a blockhash, compiles a v0 message with no lookup tables solely for simulateTransaction (sigVerify: false, replaceRecentBlockhash: true), then stamps a fresh blockhash on the legacy tx. “Already in use” on the pool-record address returns { already: true }.
registerIfMissing: if poolRecordReady (owner is the fee program and data.length >= 162), return { already: true } and no tx. Otherwise build, simulate, set the compute limit, return the legacy tx.
Matchbox prepare (prepareLaunchTx)
src/lib/fuse/launch-prepare.ts
Rejects when LAUNCHES_PAUSED.
Checks name, ticker, description, quote, first buy >= 0 (no 1000 cap).
Requires balance >= 30_000_000 lamports (MIN_LAUNCH_LAMPORTS).
buildLaunchTransaction.
Unless noLaunchSigner, adds a Memo "fusedpad" signed by the launch signer only if txSize(trial) + 64 <= 1232.
withPriority, then bundleRegister.
Simulates the whole legacy tx (create, optional memo, optional register).
Returns { combined, raw, pool, mint, quoteMint, quote, launchSigner }. raw is base64 of the legacy tx, signatures not required.
prepareRegisterTx loads the pool with the Meteora SDK, takes poolState.config from chain, then registerIfMissing. It does not use the catalog config.
API prepare (prepareLaunch / prepareRegister)
src/lib/fuse/bot.ts
Create and register are separate legacy transactions. The server partialSigns the new mint. If a launch signer exists it adds the memo and partialSigns that too. If the signed create is over 1232 bytes, the memo is removed and only the mint is signed again. Register is withPriority(buildRegisterPool(...)) with the same blockhash, fee payer set, not simulated.
prepareRegister is the API twin of prepareRegisterTx.
What the wallet signs
src/routes/launch.tsx
Create signers: user wallet (fee payer) and the base-mint keypair. Optional third signer: launch signer, only on the memo, not writable.
If combined is false and the wallet has signAllTransactions, signAllLaunch puts create and register in one approval. Create is broadcast immediately. The signed register is held and broadcast in settleRegister after the board row is saved.
Otherwise only create is signed and sent. Register is a later prompt via registerLaunch → prepareRegisterTx.
askWallet / signCosigned: after the user is about to sign, cosignLaunchTx checks the tx and adds the launch-signer signature. If that check fails, and the mint does not already exist, it rebuilds with noLaunchSigner: true and signs without the memo.
Broadcast: sendRawTransaction, skipPreflight: false, maxRetries: 5, to the wallet RPC and https://solana-rpc.publicnode.com.
Coin page and My Cache submit register through submitSigned (src/lib/fuse/broadcast.ts): sendRawTransaction, skipPreflight: false, maxRetries: 3, server RPCs only. My Cache does not poll for confirmation.
/admin uses the connected wallet’s connection.sendRawTransaction with library defaults. No extra signers.
Create itself is not hand-built. buildLaunchTransaction in src/lib/fuse/meteora.ts calls @meteora-ag/dynamic-bonding-curve-sdk:
client.creator.createPool when first buy is 0.
client.creator.createPoolWithFirstBuy otherwise (buyer = payer, minimumAmountOut = 0, referralTokenAccount = null).
Pool pubkey is keys[5] of the DBC init instruction.
A token_badge PDA ["token_badge", quoteMint] under the DBC program is passed only if that account already exists.
3. register_pool as sent today
Program ID: AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9 (FEE_PROGRAM in src/lib/fuse/devnet.ts). The file name says devnet; DEVNET.cluster is "mainnet-beta". This is the id every register path uses.
Discriminator (8 bytes): [85, 229, 114, 47, 75, 145, 166, 100].
Args, little-endian raw bytes, no Borsh length prefixes:
OffsetWidthValue08discriminator832pool pubkey4032base mint7232quote mint
Total data length 104. No other arguments.
Accounts (fixed list, not remaining_accounts):
#AccountFlags0payersigner, writable1config record PDAreadonly2pool record PDAwritable3System programreadonly4global PDAreadonly5DBC poolreadonly6DBC configreadonly7quote mintreadonly
PDAs (program = fee program, findProgramAddressSync):
config record: ["config", quoteMint]
pool record: ["pool", pool]
global: ["global"]
Not passed on register: creator-owed ["cown", pool], quote vault, token program, base mint as an account (base mint is only in the instruction data).
QuoteStat is not passed. A repo search finds no QuoteStat, qstat, or remainingAccounts on this instruction. The quote mint is account 7, the last account. It is not a remaining account.
Assumption: the on-chain program expects exactly these 8 accounts. That is what this client sends. It is not confirmed against an IDL.
Other fee-program discriminators in the same file, not part of register: mark_graduated [125, 72, 57, 129, 59, 15, 247, 251], claim_creator_share [223, 73, 251, 122, 3, 19, 20, 33].
4. IDs, configs, token programs, versions
DBC program (hardcoded in several files): dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN.
Create discriminators this app recognizes (meteora.ts, launch-cosign.ts, recover.ts):
SPL create: [140, 85, 215, 176, 102, 54, 104, 79]
Token-2022 create: [169, 118, 51, 78, 145, 110, 220, 155]
Swap: [248, 198, 158, 145, 225, 117, 135, 200]
Swap2: [65, 75, 63, 76, 235, 91, 91, 136]
Cosign account indexes on the create instruction: config keys[0], base mint keys[3], pool keys[5].
Quote selection. quoteConfig(ticker) in src/lib/fuse/configs.ts reads QUOTE_CONFIGS. Each row is { ticker, mint, config, decimals }. New launches use only that list. PREVIOUS_CONFIGS is for discovering older pools (loadChainMint), not for new creates. LAUNCHES_PAUSED is false. There are 56 current quote rows (SOL, USDC, USDT, plus the rest of that array, including the *x mints whose addresses start with Xs). The full pubkeys are in that file. They are public addresses, not secrets.
buildLaunchTransaction loads the DBC config and throws if its on-chain quoteMint is not the catalog mint.
Token program is not stored. quoteTokenProgram (fee.ts) returns the mint account owner. Register does not pass it. Claims do. The DBC SDK picks the base-token program from the config’s tokenType (0 → SPL TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA, else TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb). The quote token program is whatever the SDK reads off the quote mint. fusedCurve() builds an SPL, 6-decimal, 1e9-supply curve, but only buildConfigTransaction uses it. Live launches do not create configs; they use the catalog config pubkeys.
launch-cosign.ts allows auxiliary instructions only for the SPL token program, not Token-2022. A create that contains a Token-2022 instruction fails that check. The Matchbox page then rebuilds without the launch signer. The API path does not run that allowlist; the server signs the tx it just built.
Board rule (board-rule.ts) also requires the DBC config fee claimer F3swKL7FKLtNfo9ckummnkrjnoQna2UCGzAErx5VBcLU, config length 1048, data[243] === 6, data[361] === 0, u16 at 362 === 150, and the pool record’s bytes 104–136 equal that config.
No fee-program IDL is in the repo. Meteora’s DBC IDL is inside the SDK package, not copied here.
Versions installed, matching package.json:
@solana/web3.js 1.99.0
@solana/spl-token 0.4.14
@meteora-ag/dynamic-bonding-curve-sdk 1.5.13
@tanstack/react-start 1.168.60
Source revision: not available. git rev-parse HEAD returned nothing in this sandbox.
Launch-signer key: env FUSEDPAD_LAUNCH_SIGNER, or a server module. Not copied here.
5. Transaction shape and size
Submitted launch and register txs are legacy. v0 is used only to simulate, with no address lookup tables. Lookup tables exist only on Jupiter routed swaps (src/lib/fuse/route.ts), not on launch or register.
Compute budget on Matchbox and API register-only: price 100_000 microlamports, then a limit from simulation when it still fits in 1232. /admin sets neither.
First buy is an extra DBC swap in the create tx (minimumAmountOut = 0). Matchbox allows any finite amount >= 0. The API allows 0..1000 in the quote token.
Memo "fusedpad", Memo program MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr, one signer key (the launch signer), not writable. The user must not strip existing signatures.
Packet cap used by the app: 1232. Near the cap:
first buy + memo + bundled register (the fits budget drops to 1152 once a memo is present)
API create after mint + launch-signer signatures (memo stripped if over 1232)
/admin batching every missing register into one tx with no size check
No launch or register path uses a versioned message or a lookup table to get under the cap. If bundleRegister returns false, behaviour changes from one signature to two. That is the intended overflow valve.
6. Create succeeds, register fails
They are not always one transaction.
combined === true (Matchbox only, when the extra instruction fits): one legacy tx. If register fails, create does not land. The user sees the program or simulation error. Nothing is indexed.
combined === false, or any API client: create and register are separate. Create can land and register can fail.
Duplicate protection:
Same Matchbox attempt reuses the mint keypair (attemptRef and sessionStorage key fused-open-launch) until the fingerprint changes. If the mint account already exists, it does not create another.
poolRecordReady / simulation “already in use” on the pool-record PDA. User text: “Already registered, all good.”
fuse_mints insert is on conflict (mint) do nothing (API) or an update that does not mint a second row (saveMint).
Retries:
Matchbox settleRegister: broadcast a pre-signed register if it has one; then registerLaunch (new prepareRegisterTx); if the blockhash expired, one more registerLaunch; if it failed for another reason and was not a user reject, one more registerLaunch. A user reject returns without throwing.
If the board save fails, Matchbox stays on the page with Register {TICKER}. Do not pay again. That retries saveMint and then register. It does not rebuild the create.
Coin page and My Cache can register later. /admin can batch them.
API clients are told to send create, wait, then register, and to call action: "register" rather than prepare a second coin. record returns 409 if the create is not confirmed yet.
What the user sees:
Register failure after a paid create does not throw out of finish. The app still navigates to the coin page.
The coin page shows Register this coin while readPoolRecord.ready is false, and polls every 20s.
sessionStorage fused-register-error is read on the coin page and cleared on success, but nothing in this tree sets it. The “Not registered.” banner is currently unreachable. Verified.
My Cache shows Register this coin when stashFor says the record is missing, including coins that are not on the public board yet.
7. How a launch shows up on the Board, coin page, and My Cache
Matchbox publishLaunch writes the coin into the browser store, then saveMint inserts fuse_mints (and the token file / picture). API record does the same insert after it verifies the signature. Neither step requires the pool record.
Public Board (listBoard, homepage) and listPublicCoins then run keepBoardCoins. A coin stays off the board until fuse_board_rule.ok is true.
That verdict (board-rule.ts) needs a DBC pool, a config that passes the fee-claimer / fee / decimals checks, and a fee-program pool record whose bytes 104–136 equal that config. A missing record is a retry, not a stored fail. Fails are retried for 24 hours, at most every 5 minutes. recheckBoardCoin runs after saveMint / record.
Coin page: local store can show it immediately after Matchbox finish (route id is ticker + first 4 mint chars, lowercased). Other visitors use findReadyCoin, which requires fuse_board_rule.ok = true. If that is still false, the page says “Opening this mint.” for 30s, then “Gone out.” A direct mint address can also be loaded by loadChainMint, which scans catalog and previous configs and may insert fuse_mints itself.
My Cache uses listCacheMints, which does not apply the board rule, then stashFor to keep only pools this wallet created. Unregistered rows get the register button. Registration is poolRecordAddress owned by the fee program with data.length >= 162.
So: the SQL row can exist before registration; the public board and findReadyCoin wait until the pool record matches the config.
8. What would have to change for a writable QuoteStat PDA
Proposed, not implemented. Append a writable, non-signer account immediately after account 7:
PublicKey.findProgramAddressSync([Buffer.from("qstat"), quoteMint.toBuffer()], FEE_PROGRAM)[0]
with { isSigner: false, isWritable: true }.
Do not change the discriminator or the 104-byte argument blob.
One code change covers every current caller: add that key at the end of the keys array in registerPoolInstruction (src/lib/fuse/fee.ts). buildRegisterPool, Matchbox bundle, signAllLaunch, prepareRegisterTx, /api/launch prepare and action: "register", coin page, My Cache, and /admin all use that function.
Also expect these effects, even with no other intended behaviour change:
Program first. This client has no IDL. Sending a 9th account before the program accepts it will fail simulation (Matchbox combined create, and every registerIfMissing) or submission (API register, which is not simulated). Roll the program out before this client, or keep the old client until the program is live. Confirm whether QuoteStat must already exist. This app never creates one. Today the only create-looking account on the instruction is the pool record, and System program is in the list so the program can CPI-create it. QuoteStat would not be created unless the program does that.
Size. One new account is a new 32-byte key if it is not already in the message, plus one index byte. bundleRegister will stop bundling on txs that no longer pass fits. Those Matchbox launches become two signatures instead of one. First buy + memo is the tight case. API create size is unchanged if register stays a second tx. /admin batches get one account per coin and can cross 1232 with fewer coins than today.
Cosign. instructionsOk checks the register discriminator only, not the account count. A bundled register with an extra account still passes that check. No cosign change is required unless you want it to reject a register that omits QuoteStat.
Do not change create discriminators, memo text, first-buy accounts, quote catalog, or the pool-record seeds. Board visibility still depends on the pool record, not on QuoteStat. poolRecordReady (length >= 162) is unchanged unless the program also changes that account.
There is no second copy of this account list under src/. backups/ has an older launch.tsx and must not be edited for a rollout.
9. Test checklist after a future program change
Do not run these until the final interface and rollout order exist.
SPL quote, no first buy (USDC or USDT, 6 decimals) from /launch. Expect either one combined tx or create-then-register. Pool record owner is the fee program, length >= 162. Account 8 is the QuoteStat PDA, writable. Board row appears only after the record matches. Coin page does not show Register.
SPL quote, with first buy (SOL, 9 decimals, and a 6-decimal quote). Same, plus a DBC swap in the create tx, minimumAmountOut = 0. Watch the 1232 cap: combined vs two prompts must match what fits does with the extra account.
Token-2022 quote, one catalog mint whose mint owner is TokenzQd… (the Xs* rows are the candidates; confirm owner on chain first). Create may include Token-2022 instructions. Matchbox should still launch: cosign may skip the launch signer and rebuild. Register must still append QuoteStat. Repeat with and without first buy. Program errors 6028–6033 are the fee program rejecting the quote token itself; those are not a QuoteStat bug.
API POST /api/launch: two legacy txs, create partially signed, register unsigned, register account 8 present, register not simulated. Send create, wait, send register. Then action: "record". Second record returns already: true. action: "register" on an existing record returns already: true and no tx.
Retry / duplicate: same ticker and mint after a landed create must not create a second pool. Register again from the coin page, from My Cache, and from action: "register" must no-op with “Already registered, all good.” Expired blockhash on the second Matchbox signature must rebuild register, not create.
Create lands, register does not: coin stays off the public board (keepBoardCoins) and off findReadyCoin, but My Cache still lists it and offers Register. After register, board and coin page show it without a second create.
Each entry point once: /launch, /api/launch, coin-page Register, My Cache Register, /admin batch of two missing records (size). /game must still do nothing.
Dark path on quotes that already work: a current SOL launch with no first buy, after the upgrade, must match today’s create accounts except the new trailing QuoteStat on register. Memo "fusedpad" still optional and still dropped when over 1232.
No code was changed.
