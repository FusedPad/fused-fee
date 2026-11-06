//! Runtime tests for the QuoteStat auto-count fix and the existing fee paths.
//!
//! The fused_fee program under test is the SBF binary at target/deploy/fused_fee.so
//! unless FUSED_FEE_NATIVE=1. SPL Token and Token-2022 are the real BPF builds that
//! ship inside solana-program-test (token 3.5.0, token-2022 8.0.0). Meteora DBC and
//! DAMM are local stand-ins: they are not the mainnet programs. Auto-LP is armed
//! only inside this process.

use solana_account::{Account, AccountSharedData};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_signer::Signer;
use solana_system_interface::{instruction as system_ix, program as system_program};
use solana_transaction::Transaction;
use std::path::PathBuf;

const BPF_LOADER: Pubkey = solana_pubkey::pubkey!("BPFLoader2111111111111111111111111111111111");
const DBC_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
const DBC_POOL_AUTHORITY: Pubkey =
    solana_pubkey::pubkey!("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");
const DAMM_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
const TOKEN_2022_ID: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const NATIVE_MINT: Pubkey = solana_pubkey::pubkey!("So11111111111111111111111111111111111111112");

const DBC_POOL_DISC: [u8; 8] = [213, 224, 5, 209, 98, 69, 119, 92];
const DBC_CONFIG_DISC: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];
const POOL_RECORD_DISC: [u8; 8] = [117, 217, 136, 145, 181, 104, 107, 17];
const QUOTE_STAT_DISC: [u8; 8] = [196, 19, 91, 20, 68, 151, 114, 86];

const IX_INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const IX_REGISTER_CONFIG: [u8; 8] = [32, 247, 82, 131, 35, 183, 7, 57];
const IX_REGISTER_POOL: [u8; 8] = [85, 229, 114, 47, 75, 145, 166, 100];
const IX_SET_BURN_MINT: [u8; 8] = [245, 243, 127, 223, 152, 205, 137, 210];
const IX_SET_PAUSED: [u8; 8] = [91, 60, 125, 192, 176, 225, 166, 218];
const IX_RELEASE_BURN: [u8; 8] = [224, 64, 74, 119, 186, 164, 250, 47];
const IX_MARK_GRADUATED: [u8; 8] = [125, 72, 57, 129, 59, 15, 247, 251];
const IX_CRANK_CLAIM: [u8; 8] = [193, 62, 163, 14, 168, 236, 179, 103];
const IX_CRANK_CLAIM_DAMM: [u8; 8] = [129, 173, 59, 184, 182, 112, 147, 238];
const IX_CLAIM_CREATOR: [u8; 8] = [223, 73, 251, 122, 3, 19, 20, 33];
const IX_PAY_V2: [u8; 8] = [187, 255, 119, 89, 187, 17, 96, 79];
const IX_PAY_V3: [u8; 8] = [26, 92, 113, 244, 121, 99, 140, 121];
const IX_BURN_DIRECT: [u8; 8] = [118, 75, 25, 157, 201, 143, 78, 137];
const IX_INIT_AUTO_LP: [u8; 8] = [166, 15, 73, 164, 58, 151, 116, 19];
const IX_SET_AUTO_LP: [u8; 8] = [77, 69, 121, 149, 171, 199, 117, 85];
const IX_SET_AUTO_LP_SHARE: [u8; 8] = [52, 19, 206, 114, 85, 2, 243, 52];
const IX_SYNC_QSTAT: [u8; 8] = [130, 168, 89, 25, 125, 164, 73, 104];
const IX_CRANK_AUTO_LP: [u8; 8] = [105, 243, 28, 82, 221, 253, 201, 200];
const IX_CHECK_MINT: [u8; 8] = [18, 102, 204, 80, 128, 116, 58, 205];

const CLAIM_TRADING_FEE_DISC: [u8; 8] = [8, 236, 89, 49, 152, 125, 177, 81];
const CLAIM_POSITION_FEE_DISC: [u8; 8] = [180, 38, 154, 17, 133, 33, 162, 211];
const ADD_LIQUIDITY_DISC: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72];
const PERMANENT_LOCK_DISC: [u8; 8] = [165, 176, 125, 6, 231, 171, 186, 213];

const ERR_BAD_MINT: u32 = 6000;
const ERR_QUOTE_MISMATCH: u32 = 6002;
const ERR_MATH: u32 = 6006;
const ERR_EMPTY_POT: u32 = 6010;
const ERR_NOT_CRANK: u32 = 6023;
const ERR_BAD_ROUND: u32 = 6026;
const ERR_VAULT_SHORT: u32 = 6034;
const ERR_NOT_ADMIN: u32 = 6037;
const ERR_USE_SPLIT: u32 = 6039;
const ERR_NO_CREATOR: u32 = 6041;
const ERR_AUTO_LP_NOT_READY: u32 = 6043;
const ERR_BAD_AUTO_LP: u32 = 6045;
const ERR_BAD_AUTO_LP_AMT: u32 = 6046;
const ERR_QSTAT_REQUIRED: u32 = 6048;
const ERR_QSTAT_UNINIT: u32 = 6049;

fn pid() -> Pubkey {
    Pubkey::new_from_array(fused_fee::ID.to_bytes())
}

fn pda(seeds: &[&[u8]]) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, &pid())
}

fn rent_lamports(space: usize) -> u64 {
    Rent::default().minimum_balance(space).max(1)
}

fn owned(owner: Pubkey, data: Vec<u8>) -> Account {
    Account {
        lamports: rent_lamports(data.len()),
        data,
        owner,
        executable: false,
        rent_epoch: 0,
    }
}

fn spl_mint_bytes(decimals: u8, authority: &Pubkey, freeze: bool) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    d[0..4].copy_from_slice(&1u32.to_le_bytes());
    d[4..36].copy_from_slice(authority.as_ref());
    d[44] = decimals;
    d[45] = 1;
    if freeze {
        d[46..50].copy_from_slice(&1u32.to_le_bytes());
        d[50..82].copy_from_slice(authority.as_ref());
    }
    d
}

fn t22_mint_bytes(decimals: u8, authority: &Pubkey, tlv: &[u8]) -> Vec<u8> {
    let mut d = spl_mint_bytes(decimals, authority, false);
    if tlv.is_empty() {
        return d;
    }
    d.resize(165, 0);
    d.push(1); // AccountType::Mint
    d.extend_from_slice(tlv);
    d
}

fn tlv(tag: u16, value: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&tag.to_le_bytes());
    v.extend_from_slice(&(value.len() as u16).to_le_bytes());
    v.extend_from_slice(value);
    v
}

fn token_account_bytes(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = vec![0u8; 165];
    d[0..32].copy_from_slice(mint.as_ref());
    d[32..64].copy_from_slice(owner.as_ref());
    d[64..72].copy_from_slice(&amount.to_le_bytes());
    d[108] = 1;
    d
}

fn dbc_pool_bytes(config: &Pubkey, creator: &Pubkey, base: &Pubkey, migrated: u8) -> Vec<u8> {
    let mut d = vec![0u8; 400];
    d[..8].copy_from_slice(&DBC_POOL_DISC);
    d[72..104].copy_from_slice(config.as_ref());
    d[104..136].copy_from_slice(creator.as_ref());
    d[136..168].copy_from_slice(base.as_ref());
    d[305] = migrated;
    d
}

fn dbc_config_bytes(quote: &Pubkey, fee_claimer: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 256];
    d[..8].copy_from_slice(&DBC_CONFIG_DISC);
    d[8..40].copy_from_slice(quote.as_ref());
    d[40..72].copy_from_slice(fee_claimer.as_ref());
    d
}

fn pool_record_bytes(
    pool: &Pubkey,
    base: &Pubkey,
    quote: &Pubkey,
    config: &Pubkey,
    bump: u8,
    graduated: bool,
    round: u64,
    holder: u64,
    burn: u64,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(8 + 32 * 4 + 1 + 8 + 8 + 8 + 1);
    d.extend_from_slice(&POOL_RECORD_DISC);
    d.extend_from_slice(pool.as_ref());
    d.extend_from_slice(base.as_ref());
    d.extend_from_slice(quote.as_ref());
    d.extend_from_slice(config.as_ref());
    d.push(u8::from(graduated));
    d.extend_from_slice(&round.to_le_bytes());
    d.extend_from_slice(&holder.to_le_bytes());
    d.extend_from_slice(&burn.to_le_bytes());
    d.push(bump);
    d
}

fn quote_stat_bytes(mint: &Pubkey, count: u64, bump: u8) -> Vec<u8> {
    let mut d = Vec::with_capacity(8 + 32 + 8 + 1);
    d.extend_from_slice(&QUOTE_STAT_DISC);
    d.extend_from_slice(mint.as_ref());
    d.extend_from_slice(&count.to_le_bytes());
    d.push(bump);
    d
}

fn ix(disc: [u8; 8], data: &[u8], accounts: Vec<AccountMeta>) -> Instruction {
    let mut buf = disc.to_vec();
    buf.extend_from_slice(data);
    Instruction {
        program_id: pid(),
        accounts,
        data: buf,
    }
}

fn pk_bytes(p: &Pubkey) -> [u8; 32] {
    p.to_bytes()
}

struct Rt {
    ctx: ProgramTestContext,
    slot: u64,
}

impl Rt {
    async fn send(
        &mut self,
        ixs: &[Instruction],
        extra: &[&Keypair],
    ) -> Result<(Vec<String>, Option<Vec<u8>>), String> {
        self.slot += 1;
        let _ = self.ctx.warp_to_slot(self.slot);
        let bh = self
            .ctx
            .banks_client
            .get_latest_blockhash()
            .await
            .map_err(|e| format!("blockhash {e:?}"))?;
        let payer = self.ctx.payer.insecure_clone();
        let mut signers: Vec<&Keypair> = vec![&payer];
        for k in extra {
            if k.pubkey() != payer.pubkey() {
                signers.push(*k);
            }
        }
        // new_signed_with_payer wants the fee payer inside `signers`.
        let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), &signers, bh);
        match self
            .ctx
            .banks_client
            .process_transaction_with_metadata(tx)
            .await
        {
            Ok(r) => match r.result {
                Ok(()) => {
                    let logs = r.metadata.as_ref().map(|m| m.log_messages.clone()).unwrap_or_default();
                    let ret = r.metadata.and_then(|m| m.return_data.map(|d| d.data));
                    Ok((logs, ret))
                }
                Err(e) => {
                    let logs = r.metadata.map(|m| m.log_messages).unwrap_or_default();
                    Err(format!("{e:?} logs={logs:?}"))
                }
            },
            Err(e) => Err(format!("client {e:?}")),
        }
    }

    async fn send_payer(
        &mut self,
        ixs: &[Instruction],
        payer: &Keypair,
        extra: &[&Keypair],
    ) -> Result<(Vec<String>, Option<Vec<u8>>), String> {
        self.slot += 1;
        let _ = self.ctx.warp_to_slot(self.slot);
        let bh = self
            .ctx
            .banks_client
            .get_latest_blockhash()
            .await
            .map_err(|e| format!("blockhash {e:?}"))?;
        let mut signers: Vec<&Keypair> = vec![payer];
        for k in extra {
            if k.pubkey() != payer.pubkey() {
                signers.push(k);
            }
        }
        let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), &signers, bh);
        match self
            .ctx
            .banks_client
            .process_transaction_with_metadata(tx)
            .await
        {
            Ok(r) => match r.result {
                Ok(()) => Ok((
                    r.metadata.as_ref().map(|m| m.log_messages.clone()).unwrap_or_default(),
                    r.metadata.and_then(|m| m.return_data.map(|d| d.data)),
                )),
                Err(e) => Err(format!(
                    "{e:?} logs={:?}",
                    r.metadata.map(|m| m.log_messages).unwrap_or_default()
                )),
                }
            Err(e) => Err(format!("client {e:?}")),
        }
    }

    async fn account(&mut self, key: &Pubkey) -> Option<Account> {
        self.ctx.banks_client.get_account(*key).await.unwrap()
    }

    fn store(&mut self, key: &Pubkey, account: Account) {
        self.ctx
            .set_account(key, &AccountSharedData::from(account));
    }
}

fn custom_code(err: &str) -> Option<u32> {
    for marker in ["Custom(", "custom program error: "] {
        if let Some(i) = err.find(marker) {
            let rest = &err[i + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse() {
                return Some(n);
            }
        }
    }
    if let Some(i) = err.find("Error Number: ") {
        let rest = &err[i + "Error Number: ".len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse() {
            return Some(n);
        }
    }
    None
}

fn assert_err(res: &Result<(Vec<String>, Option<Vec<u8>>), String>, code: u32, what: &str) {
    match res {
        Ok((logs, _)) => panic!("{what}: expected error {code}, succeeded logs={logs:?}"),
        Err(e) => {
            let got = custom_code(e);
            assert_eq!(got, Some(code), "{what}: {e}");
        }
    }
}

fn assert_ok(res: &Result<(Vec<String>, Option<Vec<u8>>), String>, what: &str) {
    if let Err(e) = res {
        panic!("{what}: {e}");
    }
}

async fn qstat_count(rt: &mut Rt, mint: &Pubkey) -> Option<u64> {
    let (key, _) = pda(&[b"qstat", mint.as_ref()]);
    let acct = rt.account(&key).await?;
    if acct.data.len() < 48 || acct.owner != pid() {
        return None;
    }
    Some(u64::from_le_bytes(acct.data[40..48].try_into().unwrap()))
}

async fn pool_exists(rt: &mut Rt, pool: &Pubkey) -> bool {
    let (key, _) = pda(&[b"pool", pool.as_ref()]);
    rt.account(&key).await.is_some()
}

fn token_amount_of(acct: &Account) -> u64 {
    u64::from_le_bytes(acct.data[64..72].try_into().unwrap())
}

fn pool_pots(acct: &Account) -> (u64, u64, u64) {
    let round = u64::from_le_bytes(acct.data[137..145].try_into().unwrap());
    let holder = u64::from_le_bytes(acct.data[145..153].try_into().unwrap());
    let burn = u64::from_le_bytes(acct.data[153..161].try_into().unwrap());
    (round, holder, burn)
}

fn locate_so() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../../target/deploy/fused_fee.so"),
        manifest.join("target/deploy/fused_fee.so"),
        PathBuf::from("target/deploy/fused_fee.so"),
        PathBuf::from("/workspace/target/deploy/fused_fee.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "fused_fee.so not found. Build it with cargo-build-sbf before the runtime suite. Looked in {candidates:?}"
    );
}

fn program_test() -> ProgramTest {
    let mut pt = ProgramTest::default();
    pt.prefer_bpf(false);
    pt.set_compute_max_units(1_400_000);
    // The suite loads the SBF binary. A native entrypoint wrapper does not match
    // the program-test lifetime and would not be the binary under test.
    let path = locate_so();
    let elf = std::fs::read(&path).expect("read so");
    eprintln!("FUSED_FEE_RUNTIME=sbf bytes={} path={}", elf.len(), path.display());
    pt.add_account(
        pid(),
        Account {
            lamports: rent_lamports(elf.len()),
            data: elf,
            owner: BPF_LOADER,
            executable: true,
            rent_epoch: 0,
        },
    );
    let mock = locate_mock();
    let elf = std::fs::read(&mock).expect("read mock_meteora.so");
    eprintln!("MOCK_METEORA bytes={} path={}", elf.len(), mock.display());
    for id in [DBC_PROGRAM_ID, DAMM_PROGRAM_ID] {
        pt.add_account(
            id,
            Account {
                lamports: rent_lamports(elf.len()),
                data: elf.clone(),
                owner: BPF_LOADER,
                executable: true,
                rent_epoch: 0,
            },
        );
    }
    pt
}

fn locate_mock() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest.join("../../harness/mock_meteora/target/deploy/mock_meteora.so"),
        PathBuf::from("/workspace/harness/mock_meteora/target/deploy/mock_meteora.so"),
        PathBuf::from("/workspace/target/deploy/mock_meteora.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("mock_meteora.so missing. Build harness/mock_meteora with cargo-build-sbf. Looked in {candidates:?}");
}

struct QuoteFix {
    mint: Pubkey,
    config: Pubkey,
    base: Pubkey,
    pools: Vec<Pubkey>,
    cursor: usize,
}

impl QuoteFix {
    fn new(rt_global: &Pubkey, base: Pubkey, mint_owner: Pubkey, mint_data: Vec<u8>, n_pools: usize) -> (Self, Vec<(Pubkey, Account)>) {
        let mint = Pubkey::new_unique();
        let config = Pubkey::new_unique();
        let mut accounts = vec![
            (mint, owned(mint_owner, mint_data)),
            (
                config,
                owned(DBC_PROGRAM_ID, dbc_config_bytes(&mint, rt_global)),
            ),
        ];
        let mut pools = Vec::new();
        for _ in 0..n_pools {
            let pool = Pubkey::new_unique();
            let creator = Pubkey::new_unique();
            accounts.push((
                pool,
                owned(
                    DBC_PROGRAM_ID,
                    dbc_pool_bytes(&config, &creator, &base, 0),
                ),
            ));
            pools.push(pool);
        }
        (
            Self {
                mint,
                config,
                base,
                pools,
                cursor: 0,
            },
            accounts,
        )
    }

    fn take(&mut self) -> Pubkey {
        let p = self.pools[self.cursor];
        self.cursor += 1;
        p
    }
}

async fn bootstrap(pt: ProgramTest) -> Rt {
    let mut ctx = pt.start_with_context().await;
    // The simulated bank starts at slot 0. First warp must move forward.
    let _ = ctx.warp_to_slot(1);
    let mut rt = Rt { ctx, slot: 1 };
    let (global, _) = pda(&[b"global"]);
    let admin = rt.ctx.payer.pubkey();
    let init = ix(
        IX_INITIALIZE,
        &[],
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new(global, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    );
    assert_ok(&rt.send(&[init], &[]).await, "initialize");
    rt
}

fn register_config_ix(admin: Pubkey, global: Pubkey, quote: Pubkey, config: Pubkey) -> Instruction {
    let (config_record, _) = pda(&[b"config", quote.as_ref()]);
    let mut data = Vec::new();
    data.extend_from_slice(&pk_bytes(&quote));
    ix(
        IX_REGISTER_CONFIG,
        &data,
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new_readonly(global, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(config_record, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(quote, false),
        ],
    )
}

fn register_pool_ix(
    payer: Pubkey,
    global: Pubkey,
    quote: Pubkey,
    base: Pubkey,
    config: Pubkey,
    pool: Pubkey,
    quotestat: Option<(Pubkey, bool)>,
    extra_tail: &[AccountMeta],
) -> Instruction {
    let (config_record, _) = pda(&[b"config", quote.as_ref()]);
    let (pool_record, _) = pda(&[b"pool", pool.as_ref()]);
    let mut metas = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(config_record, false),
        AccountMeta::new(pool_record, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new_readonly(config, false),
    ];
    assert_eq!(metas.len(), 7, "named register_pool prefix changed");
    metas.push(AccountMeta::new_readonly(quote, false));
    if let Some((qs, writable)) = quotestat {
        metas.push(if writable {
            AccountMeta::new(qs, false)
        } else {
            AccountMeta::new_readonly(qs, false)
        });
    }
    metas.extend_from_slice(extra_tail);
    let mut data = Vec::new();
    data.extend_from_slice(&pk_bytes(&pool));
    data.extend_from_slice(&pk_bytes(&base));
    data.extend_from_slice(&pk_bytes(&quote));
    ix(IX_REGISTER_POOL, &data, metas)
}

fn sync_ix(admin: Pubkey, global: Pubkey, quote: Pubkey, count: u64) -> Instruction {
    let (qs, _) = pda(&[b"qstat", quote.as_ref()]);
    let mut data = Vec::new();
    data.extend_from_slice(&pk_bytes(&quote));
    data.extend_from_slice(&count.to_le_bytes());
    ix(
        IX_SYNC_QSTAT,
        &data,
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new_readonly(global, false),
            AccountMeta::new(qs, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    )
}

async fn fund(rt: &mut Rt, to: &Pubkey, lamports: u64) {
    let from = rt.ctx.payer.pubkey();
    let ix = system_ix::transfer(&from, to, lamports);
    assert_ok(&rt.send(&[ix], &[]).await, "fund");
}

#[tokio::test]
async fn quotestat_registration_matrix() {
    let (global, _) = pda(&[b"global"]);
    let admin_auth = Pubkey::new_unique();
    let base_kp_mint = Pubkey::new_unique();
    let mut pt = program_test();
    pt.add_account(
        base_kp_mint,
        owned(spl_token::ID, spl_mint_bytes(6, &admin_auth, false)),
    );

    let (mut quote_a, acc_a) = QuoteFix::new(&global, base_kp_mint, spl_token::ID, spl_mint_bytes(9, &admin_auth, false), 24);
    let (mut quote_b, acc_b) = QuoteFix::new(&global, base_kp_mint, spl_token::ID, spl_mint_bytes(9, &admin_auth, true), 4);
    let (mut quote_t22, acc_t22) = QuoteFix::new(
        &global,
        base_kp_mint,
        TOKEN_2022_ID,
        t22_mint_bytes(6, &admin_auth, &[]),
        2,
    );
    let bad_unknown = tlv(99, &[1, 2, 3, 4]);
    let (quote_bad, acc_bad) = QuoteFix::new(
        &global,
        base_kp_mint,
        TOKEN_2022_ID,
        t22_mint_bytes(6, &admin_auth, &bad_unknown),
        1,
    );
    let mut high_fee = vec![0u8; 108];
    high_fee[88] = 101;
    let (quote_fee, acc_fee) = QuoteFix::new(
        &global,
        base_kp_mint,
        TOKEN_2022_ID,
        t22_mint_bytes(6, &admin_auth, &tlv(1, &high_fee)),
        1,
    );
    let mut hook = vec![0u8; 64];
    hook[32] = 1;
    let (quote_hook, acc_hook) = QuoteFix::new(
        &global,
        base_kp_mint,
        TOKEN_2022_ID,
        t22_mint_bytes(6, &admin_auth, &tlv(14, &hook)),
        1,
    );
    // Historical pool: registered before the counter existed.
    let (quote_old, acc_old) = QuoteFix::new(&global, base_kp_mint, spl_token::ID, spl_mint_bytes(9, &admin_auth, false), 2);
    let historical_pool = quote_old.pools[0];
    let (hist_rec, hist_bump) = pda(&[b"pool", historical_pool.as_ref()]);
    let (old_cfg_rec, old_cfg_bump) = pda(&[b"config", quote_old.mint.as_ref()]);

    for (k, a) in acc_a
        .into_iter()
        .chain(acc_b)
        .chain(acc_t22)
        .chain(acc_bad)
        .chain(acc_fee)
        .chain(acc_hook)
        .chain(acc_old)
    {
        pt.add_account(k, a);
    }
    pt.add_account(
        hist_rec,
        owned(
            pid(),
            pool_record_bytes(
                &historical_pool,
                &base_kp_mint,
                &quote_old.mint,
                &quote_old.config,
                hist_bump,
                false,
                0,
                0,
                0,
            ),
        ),
    );
    pt.add_account(
        old_cfg_rec,
        owned(
            pid(),
            {
                let mut d = vec![176, 40, 195, 139, 54, 176, 82, 177];
                d.extend_from_slice(quote_old.mint.as_ref());
                d.extend_from_slice(quote_old.config.as_ref());
                d.push(old_cfg_bump);
                d
            },
        ),
    );

    // Corrupt QuoteStat: right PDA, program owner, wrong mint bytes.
    let (bad_qs, bad_qs_bump) = pda(&[b"qstat", quote_a.mint.as_ref()]);
    // Don't pre-seed quote_a stat; use a dedicated mint for the corrupt case.
    let (quote_corrupt, acc_corrupt) = QuoteFix::new(&global, base_kp_mint, spl_token::ID, spl_mint_bytes(9, &admin_auth, false), 1);
    for (k, a) in acc_corrupt {
        pt.add_account(k, a);
    }
    let (corrupt_rec, corrupt_bump) = pda(&[b"config", quote_corrupt.mint.as_ref()]);
    pt.add_account(
        corrupt_rec,
        owned(pid(), {
            let mut d = vec![176, 40, 195, 139, 54, 176, 82, 177];
            d.extend_from_slice(quote_corrupt.mint.as_ref());
            d.extend_from_slice(quote_corrupt.config.as_ref());
            d.push(corrupt_bump);
            d
        }),
    );
    let (corrupt_qs, corrupt_qs_bump) = pda(&[b"qstat", quote_corrupt.mint.as_ref()]);
    pt.add_account(
        corrupt_qs,
        owned(pid(), quote_stat_bytes(&Pubkey::new_unique(), 7, corrupt_qs_bump)),
    );
    let _ = (bad_qs, bad_qs_bump);

    // Wrong owner at the PDA: system-owned non-empty data.
    let (quote_owner, acc_owner) = QuoteFix::new(&global, base_kp_mint, spl_token::ID, spl_mint_bytes(9, &admin_auth, false), 1);
    for (k, a) in acc_owner {
        pt.add_account(k, a);
    }
    let (owner_cfg, owner_cfg_bump) = pda(&[b"config", quote_owner.mint.as_ref()]);
    pt.add_account(
        owner_cfg,
        owned(pid(), {
            let mut d = vec![176, 40, 195, 139, 54, 176, 82, 177];
            d.extend_from_slice(quote_owner.mint.as_ref());
            d.extend_from_slice(quote_owner.config.as_ref());
            d.push(owner_cfg_bump);
            d
        }),
    );
    let (owner_qs, _) = pda(&[b"qstat", quote_owner.mint.as_ref()]);
    pt.add_account(
        owner_qs,
        Account {
            lamports: rent_lamports(10),
            data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    );

    // Prefunded empty PDA (system owner, zero data, some lamports).
    let (mut quote_pre, acc_pre) = QuoteFix::new(&global, base_kp_mint, spl_token::ID, spl_mint_bytes(9, &admin_auth, false), 1);
    for (k, a) in acc_pre {
        pt.add_account(k, a);
    }
    let (pre_qs, _) = pda(&[b"qstat", quote_pre.mint.as_ref()]);
    pt.add_account(
        pre_qs,
        Account {
            lamports: 50_000,
            data: vec![],
            owner: system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    );

    let mut rt = bootstrap(pt).await;
    let admin = rt.ctx.payer.pubkey();
    for q in [&quote_a, &quote_b, &quote_t22, &quote_pre] {
        assert_ok(
            &rt.send(&[register_config_ix(admin, global, q.mint, q.config)], &[])
                .await,
            "register_config",
        );
    }
    // quote_old, quote_corrupt, quote_owner configs were pre-seeded.

    // Empty QuoteStat must not be invented as count=1.
    let p0 = quote_a.take();
    let (qs_a, _) = pda(&[b"qstat", quote_a.mint.as_ref()]);
    let uninit = rt
        .send(
            &[register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p0, Some((qs_a, true)), &[])],
            &[],
        )
        .await;
    assert_err(&uninit, ERR_QSTAT_UNINIT, "empty quotestat");
    assert!(!pool_exists(&mut rt, &p0).await);
    assert!(qstat_count(&mut rt, &quote_a.mint).await.is_none());
    // Launch shape: absolute baseline 0, then this pool increments to 1, one transaction.
    assert_ok(
        &rt.send(
            &[
                sync_ix(admin, global, quote_a.mint, 0),
                register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p0, Some((qs_a, true)), &[]),
            ],
            &[],
        )
        .await,
        "sync 0 then register",
    );
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(1));
    assert!(pool_exists(&mut rt, &p0).await);

    // Second pool increments.
    let p1 = quote_a.take();
    assert_ok(
        &rt.send(
            &[register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p1, Some((qs_a, true)), &[])],
            &[],
        )
        .await,
        "second register",
    );
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(2));

    // Duplicate does not double count. Pool record stays the first one.
    let before = qstat_count(&mut rt, &quote_a.mint).await;
    let dup = rt
        .send(
            &[register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p0, Some((qs_a, true)), &[])],
            &[],
        )
        .await;
    assert!(dup.is_err(), "duplicate must fail: {dup:?}");
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, before);

    // Omission fails and does not create the pool.
    let p_omit = quote_a.take();
    let omit = rt
        .send(
            &[register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p_omit, None, &[])],
            &[],
        )
        .await;
    assert_err(&omit, ERR_QSTAT_REQUIRED, "omit quotestat");
    assert!(!pool_exists(&mut rt, &p_omit).await);
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(2));

    // Mint only, no quotestat (the old live client shape: 7 named + rem[0]).
    let p_old_client = quote_a.take();
    let old_client = rt
        .send(
            &[register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p_old_client, None, &[])],
            &[],
        )
        .await;
    assert_err(&old_client, ERR_QSTAT_REQUIRED, "old client shape");
    assert!(!pool_exists(&mut rt, &p_old_client).await);

    // Read-only QuoteStat.
    let p_ro = quote_a.take();
    let ro = rt
        .send(
            &[register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p_ro, Some((qs_a, false)), &[])],
            &[],
        )
        .await;
    assert_err(&ro, ERR_BAD_MINT, "readonly quotestat");
    assert!(!pool_exists(&mut rt, &p_ro).await);
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(2));

    // Wrong PDA.
    let p_wrong = quote_a.take();
    let wrong_key = Pubkey::new_unique();
    let wrong = rt
        .send(
            &[register_pool_ix(
                admin,
                global,
                quote_a.mint,
                quote_a.base,
                quote_a.config,
                p_wrong,
                Some((wrong_key, true)),
                &[],
            )],
            &[],
        )
        .await;
    assert_err(&wrong, ERR_QUOTE_MISMATCH, "wrong pda");
    assert!(!pool_exists(&mut rt, &p_wrong).await);
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(2));

    // QuoteStat for a different mint (right account, wrong seeds for this quote).
    let (qs_b, _) = pda(&[b"qstat", quote_b.mint.as_ref()]);
    let p_cross = quote_a.take();
    let cross = rt
        .send(
            &[register_pool_ix(
                admin,
                global,
                quote_a.mint,
                quote_a.base,
                quote_a.config,
                p_cross,
                Some((qs_b, true)),
                &[],
            )],
            &[],
        )
        .await;
    assert_err(&cross, ERR_QUOTE_MISMATCH, "cross-mint quotestat");
    assert!(qstat_count(&mut rt, &quote_b.mint).await.is_none());

    // Extra trailing account does not double-increment.
    let p_extra = quote_a.take();
    assert_ok(
        &rt.send(
            &[register_pool_ix(
                admin,
                global,
                quote_a.mint,
                quote_a.base,
                quote_a.config,
                p_extra,
                Some((qs_a, true)),
                &[AccountMeta::new_readonly(Pubkey::new_unique(), false)],
            )],
            &[],
        )
        .await,
        "trailing account",
    );
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(3));

    // QuoteStat placed at rem[0] and junk at rem[1] is not accepted as the counter.
    let p_swapped = quote_a.take();
    let mut swapped = register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p_swapped, None, &[]);
    swapped.accounts.push(AccountMeta::new(qs_a, false));
    swapped.accounts.push(AccountMeta::new_readonly(quote_a.mint, false));
    // The helper already pushed the mint at rem[0] when quotestat is None... wait, the helper
    // always pushes the mint. So accounts are 7 + mint + qs + mint. That's a valid layout plus
    // a tail. Rebuild explicitly.
    let _ = swapped;
    let mut metas_swap = register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p_swapped, None, &[])
        .accounts;
    // helper with None still pushed the mint. Replace rem layout.
    metas_swap.truncate(7);
    metas_swap.push(AccountMeta::new(qs_a, false));
    metas_swap.push(AccountMeta::new_readonly(quote_a.mint, false));
    let mut data = Vec::new();
    data.extend_from_slice(&pk_bytes(&p_swapped));
    data.extend_from_slice(&pk_bytes(&quote_a.base));
    data.extend_from_slice(&pk_bytes(&quote_a.mint));
    let swapped_ix = ix(IX_REGISTER_POOL, &data, metas_swap);
    let swapped_res = rt.send(&[swapped_ix], &[]).await;
    assert_err(&swapped_res, ERR_BAD_MINT, "swapped rem accounts");
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(3));

    // Two successful registers in one transaction increment by 2.
    let p2 = quote_a.take();
    let p3 = quote_a.take();
    assert_ok(
        &rt.send(
            &[
                register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p2, Some((qs_a, true)), &[]),
                register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p3, Some((qs_a, true)), &[]),
            ],
            &[],
        )
        .await,
        "two in one tx",
    );
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(5));
    assert!(pool_exists(&mut rt, &p2).await && pool_exists(&mut rt, &p3).await);

    // Second instruction fails: whole transaction rolls back, including the first pool.
    let p4 = quote_a.take();
    let rolled = rt
        .send(
            &[
                register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p4, Some((qs_a, true)), &[]),
                register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p4, Some((qs_a, true)), &[]),
            ],
            &[],
        )
        .await;
    assert!(rolled.is_err(), "rollback tx must fail");
    assert!(!pool_exists(&mut rt, &p4).await, "rolled-back pool must not exist");
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(5));

    // Absolute sync overwrites, then a new register increments from that baseline.
    assert_ok(&rt.send(&[sync_ix(admin, global, quote_a.mint, 9)], &[]).await, "sync overwrite");
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(9));
    let p5 = quote_a.take();
    assert_ok(
        &rt.send(
            &[register_pool_ix(admin, global, quote_a.mint, quote_a.base, quote_a.config, p5, Some((qs_a, true)), &[])],
            &[],
        )
        .await,
        "register after sync",
    );
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(10));

    // Unauthorized sync.
    let stranger = Keypair::new();
    fund(&mut rt, &stranger.pubkey(), 2_000_000_000).await;
    let unauth = rt
        .send_payer(&[sync_ix(stranger.pubkey(), global, quote_a.mint, 1)], &stranger, &[])
        .await;
    assert!(unauth.is_err(), "stranger sync must fail: {unauth:?}");
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(10));

    // Permissionless register still counts once. Not an admin gate.
    let p6 = quote_a.take();
    assert_ok(
        &rt.send_payer(
            &[register_pool_ix(
                stranger.pubkey(),
                global,
                quote_a.mint,
                quote_a.base,
                quote_a.config,
                p6,
                Some((qs_a, true)),
                &[],
            )],
            &stranger,
            &[],
        )
        .await,
        "permissionless register",
    );
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(11));

    overflow_case(&mut rt, &global, &admin, &admin_auth, &base_kp_mint).await;

    // Historical pool is not visible to a fresh counter, and register must not invent one.
    assert!(pool_exists(&mut rt, &historical_pool).await);
    assert!(qstat_count(&mut rt, &quote_old.mint).await.is_none());
    let (qs_old, _) = pda(&[b"qstat", quote_old.mint.as_ref()]);
    let p_before_sync = quote_old.pools[1];
    let early = rt
        .send(
            &[register_pool_ix(
                admin,
                global,
                quote_old.mint,
                quote_old.base,
                quote_old.config,
                p_before_sync,
                Some((qs_old, true)),
                &[],
            )],
            &[],
        )
        .await;
    assert_err(&early, ERR_QSTAT_UNINIT, "historical quote has no baseline");
    assert!(!pool_exists(&mut rt, &p_before_sync).await);
    // Absolute baseline is the one pool that already exists. The new register makes 2.
    assert_ok(&rt.send(&[sync_ix(admin, global, quote_old.mint, 1)], &[]).await, "sync historical");
    assert_ok(
        &rt.send(
            &[register_pool_ix(
                admin,
                global,
                quote_old.mint,
                quote_old.base,
                quote_old.config,
                p_before_sync,
                Some((qs_old, true)),
                &[],
            )],
            &[],
        )
        .await,
        "register after historical baseline",
    );
    assert_eq!(qstat_count(&mut rt, &quote_old.mint).await, Some(2));

    // Freeze-authority mint is accepted (tier 1, not rejected). Baseline first.
    assert_ok(&rt.send(&[sync_ix(admin, global, quote_b.mint, 0)], &[]).await, "sync quote b");
    let pb = quote_b.take();
    assert_ok(
        &rt.send(
            &[register_pool_ix(admin, global, quote_b.mint, quote_b.base, quote_b.config, pb, Some((qs_b, true)), &[])],
            &[],
        )
        .await,
        "freeze-authority quote",
    );
    assert_eq!(qstat_count(&mut rt, &quote_b.mint).await, Some(1));

    // Token-2022 mint with no extensions registers and increments the counter.
    assert_ok(&rt.send(&[sync_ix(admin, global, quote_t22.mint, 0)], &[]).await, "sync t22");
    let (qs_t, _) = pda(&[b"qstat", quote_t22.mint.as_ref()]);
    let pt22 = quote_t22.take();
    assert_ok(
        &rt.send(
            &[register_pool_ix(
                admin,
                global,
                quote_t22.mint,
                quote_t22.base,
                quote_t22.config,
                pt22,
                Some((qs_t, true)),
                &[],
            )],
            &[],
        )
        .await,
        "token-2022 register",
    );
    assert_eq!(qstat_count(&mut rt, &quote_t22.mint).await, Some(1));

    // Bad Token-2022 policy rolls back. Config was not created via register_config
    // because that path also rejects the mint. Craft is not done for quote_bad, so
    // check_quote_mint is the runtime policy check, plus a register attempt that
    // fails at register_config.
    let bad_cfg = rt
        .send(&[register_config_ix(admin, global, quote_bad.mint, quote_bad.config)], &[])
        .await;
    assert!(bad_cfg.is_err(), "unknown extension must fail config: {bad_cfg:?}");
    assert!(qstat_count(&mut rt, &quote_bad.mint).await.is_none());
    let fee_cfg = rt
        .send(&[register_config_ix(admin, global, quote_fee.mint, quote_fee.config)], &[])
        .await;
    assert!(fee_cfg.is_err(), "high transfer fee must fail: {fee_cfg:?}");
    let hook_cfg = rt
        .send(&[register_config_ix(admin, global, quote_hook.mint, quote_hook.config)], &[])
        .await;
    assert!(hook_cfg.is_err(), "active hook must fail: {hook_cfg:?}");

    let check = ix(
        IX_CHECK_MINT,
        &[],
        vec![AccountMeta::new_readonly(quote_t22.mint, false)],
    );
    assert_ok(&rt.send(&[check], &[]).await, "check good t22");
    let check_bad = ix(
        IX_CHECK_MINT,
        &[],
        vec![AccountMeta::new_readonly(quote_bad.mint, false)],
    );
    assert!(rt.send(&[check_bad], &[]).await.is_err(), "check bad mint");

    // Wrong mint bytes inside the correct PDA.
    let p_cor = quote_corrupt.pools[0];
    let corrupt = rt
        .send(
            &[register_pool_ix(
                admin,
                global,
                quote_corrupt.mint,
                quote_corrupt.base,
                quote_corrupt.config,
                p_cor,
                Some((corrupt_qs, true)),
                &[],
            )],
            &[],
        )
        .await;
    assert_err(&corrupt, ERR_QUOTE_MISMATCH, "wrong mint bytes");
    assert!(!pool_exists(&mut rt, &p_cor).await);
    let corrupt_acct = rt.account(&corrupt_qs).await.unwrap();
    assert_eq!(u64::from_le_bytes(corrupt_acct.data[40..48].try_into().unwrap()), 7);

    // Wrong owner.
    let p_own = quote_owner.pools[0];
    let (qs_own, _) = pda(&[b"qstat", quote_owner.mint.as_ref()]);
    let own = rt
        .send(
            &[register_pool_ix(
                admin,
                global,
                quote_owner.mint,
                quote_owner.base,
                quote_owner.config,
                p_own,
                Some((qs_own, true)),
                &[],
            )],
            &[],
        )
        .await;
    assert!(own.is_err(), "wrong owner must fail: {own:?}");
    assert!(!pool_exists(&mut rt, &p_own).await);

    // A prefunded empty address is still uninitialized. Rent on the address is not a count.
    let (qs_pre, _) = pda(&[b"qstat", quote_pre.mint.as_ref()]);
    let p_pre = quote_pre.take();
    let prefunded = rt
        .send(
            &[register_pool_ix(
                admin,
                global,
                quote_pre.mint,
                quote_pre.base,
                quote_pre.config,
                p_pre,
                Some((qs_pre, true)),
                &[],
            )],
            &[],
        )
        .await;
    assert_err(&prefunded, ERR_QSTAT_UNINIT, "prefunded empty pda");
    assert!(!pool_exists(&mut rt, &p_pre).await);

    // Paused does not block register_pool. Known gap, asserted so it stays visible.
    let pause = ix(
        IX_SET_PAUSED,
        &[1],
        vec![
            AccountMeta::new_readonly(admin, true),
            AccountMeta::new(global, false),
        ],
    );
    assert_ok(&rt.send(&[pause], &[]).await, "pause");
    let p_paused = quote_a.take();
    assert_ok(
        &rt.send(
            &[register_pool_ix(
                admin,
                global,
                quote_a.mint,
                quote_a.base,
                quote_a.config,
                p_paused,
                Some((qs_a, true)),
                &[],
            )],
            &[],
        )
        .await,
        "register while paused",
    );
    assert_eq!(qstat_count(&mut rt, &quote_a.mint).await, Some(12));
    let unpause = ix(
        IX_SET_PAUSED,
        &[0],
        vec![
            AccountMeta::new_readonly(admin, true),
            AccountMeta::new(global, false),
        ],
    );
    assert_ok(&rt.send(&[unpause], &[]).await, "unpause");

    // Seeded fuzz on quote_b (already count 1 from the freeze register).
    seeded_fuzz(&mut rt, &mut quote_b, global, admin).await;
}

async fn overflow_case(rt: &mut Rt, global: &Pubkey, admin: &Pubkey, auth: &Pubkey, base: &Pubkey) {
    let mint = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    let creator = Pubkey::new_unique();
    rt.store(&mint, owned(spl_token::ID, spl_mint_bytes(9, auth, false)));
    rt.store(
        &config,
        owned(DBC_PROGRAM_ID, dbc_config_bytes(&mint, global)),
    );
    rt.store(
        &pool,
        owned(DBC_PROGRAM_ID, dbc_pool_bytes(&config, &creator, base, 0)),
    );
    assert_ok(
        &rt.send(&[register_config_ix(*admin, *global, mint, config)], &[])
            .await,
        "overflow quote config",
    );
    assert_ok(
        &rt.send(&[sync_ix(*admin, *global, mint, u64::MAX)], &[]).await,
        "sync max",
    );
    let (qs, _) = pda(&[b"qstat", mint.as_ref()]);
    let res = rt
        .send(
            &[register_pool_ix(*admin, *global, mint, *base, config, pool, Some((qs, true)), &[])],
            &[],
        )
        .await;
    assert_err(&res, ERR_MATH, "overflow increment");
    assert!(!pool_exists(rt, &pool).await);
    assert_eq!(qstat_count(rt, &mint).await, Some(u64::MAX));
}

async fn seeded_fuzz(rt: &mut Rt, quote: &mut QuoteFix, global: Pubkey, admin: Pubkey) {
    let (qs, _) = pda(&[b"qstat", quote.mint.as_ref()]);
    let mut expected = qstat_count(rt, &quote.mint).await.unwrap();
    let mut registered: Vec<Pubkey> = Vec::new();
    let mut rng = 0xC0FFEE_u64;
    let mut steps = 0u32;
    while quote.cursor < quote.pools.len() && steps < 20 {
        steps += 1;
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let action = (rng >> 33) % 5;
        match action {
            0 | 4 => {
                let pool = quote.take();
                assert_ok(
                    &rt.send(
                        &[register_pool_ix(admin, global, quote.mint, quote.base, quote.config, pool, Some((qs, true)), &[])],
                        &[],
                    )
                    .await,
                    "fuzz success",
                );
                expected += 1;
                registered.push(pool);
            }
            1 => {
                let pool = quote.take();
                let res = rt
                    .send(
                        &[register_pool_ix(admin, global, quote.mint, quote.base, quote.config, pool, None, &[])],
                        &[],
                    )
                    .await;
                assert_err(&res, ERR_QSTAT_REQUIRED, "fuzz omit");
                assert!(!pool_exists(rt, &pool).await);
            }
            2 => {
                if registered.is_empty() {
                    continue;
                }
                let pool = registered[0];
                let res = rt
                    .send(
                        &[register_pool_ix(admin, global, quote.mint, quote.base, quote.config, pool, Some((qs, true)), &[])],
                        &[],
                    )
                    .await;
                assert!(res.is_err(), "fuzz duplicate");
            }
            _ => {
                let pool = quote.take();
                let res = rt
                    .send(
                        &[register_pool_ix(
                            admin,
                            global,
                            quote.mint,
                            quote.base,
                            quote.config,
                            pool,
                            Some((qs, false)),
                            &[],
                        )],
                        &[],
                    )
                    .await;
                assert_err(&res, ERR_BAD_MINT, "fuzz readonly");
                assert!(!pool_exists(rt, &pool).await);
            }
        }
        assert_eq!(qstat_count(rt, &quote.mint).await, Some(expected));
    }
}

const ATA_PROGRAM: Pubkey = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const ERR_PAUSED: u32 = 6004;
const ERR_NOT_MIGRATED: u32 = 6022;

fn ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[owner.as_ref(), token_program.as_ref(), mint.as_ref()], &ATA_PROGRAM).0
}

fn u64_at(data: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(data[off..off + 8].try_into().unwrap())
}

#[tokio::test]
async fn claims_payouts_burn_and_creator() {
    let (global, _) = pda(&[b"global"]);
    let auth = Pubkey::new_unique();
    let mut rt = bootstrap(program_test()).await;
    let admin = rt.ctx.payer.pubkey();

    let quote = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    let creator = Keypair::new();
    rt.store(&quote, owned(spl_token::ID, spl_mint_bytes(9, &auth, false)));
    rt.store(&base, owned(spl_token::ID, spl_mint_bytes(6, &auth, false)));
    rt.store(&config, owned(DBC_PROGRAM_ID, dbc_config_bytes(&quote, &global)));
    rt.store(
        &pool,
        owned(DBC_PROGRAM_ID, dbc_pool_bytes(&config, &creator.pubkey(), &base, 0)),
    );
    let vault = ata(&global, &quote, &spl_token::ID);
    let base_dest = Pubkey::new_unique();
    rt.store(&vault, owned(spl_token::ID, token_account_bytes(&quote, &global, 0)));
    rt.store(&base_dest, owned(spl_token::ID, token_account_bytes(&base, &global, 0)));
    let base_vault = Pubkey::new_unique();
    let quote_pool_vault = Pubkey::new_unique();
    let event = Pubkey::new_unique();
    rt.store(&base_vault, owned(system_program::ID, vec![]));
    // Source of the mocked DBC claim. Global already signs that CPI, so the
    // stand-in can transfer into the quote vault through the real token program.
    rt.store(
        &quote_pool_vault,
        owned(spl_token::ID, token_account_bytes(&quote, &global, 100_000)),
    );
    rt.store(&event, owned(system_program::ID, vec![]));
    rt.store(&DBC_POOL_AUTHORITY, owned(system_program::ID, vec![]));

    assert_ok(
        &rt.send(&[register_config_ix(admin, global, quote, config)], &[]).await,
        "flow register_config",
    );
    assert_ok(&rt.send(&[sync_ix(admin, global, quote, 0)], &[]).await, "flow sync 0");
    let (qs, _) = pda(&[b"qstat", quote.as_ref()]);
    assert_ok(
        &rt.send(
            &[register_pool_ix(admin, global, quote, base, config, pool, Some((qs, true)), &[])],
            &[],
        )
        .await,
        "flow register_pool",
    );
    assert_eq!(qstat_count(&mut rt, &quote).await, Some(1));

    let (pool_rec, _) = pda(&[b"pool", pool.as_ref()]);
    let (pay_cur, _) = pda(&[b"paycur", pool.as_ref()]);
    let (sol_vault, _) = pda(&[b"sol_vault"]);
    let (unwrap, _) = pda(&[b"unwrap", pool.as_ref()]);
    let pay_accounts = vec![
        AccountMeta::new(admin, true),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new(pool_rec, false),
        AccountMeta::new(vault, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(pay_cur, false),
        AccountMeta::new(sol_vault, false),
        AccountMeta::new(unwrap, false),
        AccountMeta::new_readonly(NATIVE_MINT, false),
    ];
    let mut eligible = 0u64.to_le_bytes().to_vec();
    assert_ok(
        &rt.send(&[ix(IX_PAY_V2, &eligible, pay_accounts.clone())], &[]).await,
        "open cursor at round 0",
    );

    let not_migrated = rt
        .send(
            &[ix(
                IX_MARK_GRADUATED,
                &[],
                vec![
                    AccountMeta::new_readonly(global, false),
                    AccountMeta::new(pool_rec, false),
                    AccountMeta::new_readonly(pool, false),
                ],
            )],
            &[],
        )
        .await;
    assert_err(&not_migrated, ERR_NOT_MIGRATED, "mark before migration");

    let mut claim_data = Vec::new();
    claim_data.extend_from_slice(&1u64.to_le_bytes());
    claim_data.extend_from_slice(&11_000u64.to_le_bytes());
    let claim = ix(
        IX_CRANK_CLAIM,
        &claim_data,
        vec![
            AccountMeta::new_readonly(global, false),
            AccountMeta::new(pool_rec, false),
            AccountMeta::new_readonly(DBC_POOL_AUTHORITY, false),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(pool, false),
            AccountMeta::new(base_dest, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(base_vault, false),
            AccountMeta::new(quote_pool_vault, false),
            AccountMeta::new_readonly(base, false),
            AccountMeta::new_readonly(quote, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(event, false),
            AccountMeta::new_readonly(DBC_PROGRAM_ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
    );
    assert_ok(&rt.send(&[claim], &[]).await, "crank_claim");
    let prec = rt.account(&pool_rec).await.unwrap();
    let (round, holder, burn) = pool_pots(&prec);
    assert_eq!((round, holder, burn), (1, 9_000, 2_000), "90/20 split of 11000");
    assert_eq!(token_amount_of(&rt.account(&vault).await.unwrap()), 11_000);

    // Real SPL burn needs supply >= amount. The mock credited tokens without minting.
    let mut mint_acct = rt.account(&quote).await.unwrap();
    mint_acct.data[36..44].copy_from_slice(&11_000u64.to_le_bytes());
    rt.store(&quote, mint_acct);

    let set_burn = ix(
        IX_SET_BURN_MINT,
        &pk_bytes(&quote),
        vec![
            AccountMeta::new_readonly(admin, true),
            AccountMeta::new(global, false),
        ],
    );
    assert_ok(&rt.send(&[set_burn], &[]).await, "set_burn_mint");
    let burn_ix = ix(
        IX_BURN_DIRECT,
        &[],
        vec![
            AccountMeta::new_readonly(global, false),
            AccountMeta::new(pool_rec, false),
            AccountMeta::new(quote, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
    );
    assert_ok(&rt.send(&[burn_ix], &[]).await, "crank_burn_direct");
    let prec = rt.account(&pool_rec).await.unwrap();
    assert_eq!(pool_pots(&prec).2, 0);
    assert_eq!(token_amount_of(&rt.account(&vault).await.unwrap()), 9_000);
    assert_eq!(u64_at(&rt.account(&quote).await.unwrap().data, 36), 9_000);

    let holder_wallet = Pubkey::new_unique();
    let holder_base = Pubkey::new_unique();
    let holder_quote = Pubkey::new_unique();
    rt.store(&holder_base, owned(spl_token::ID, token_account_bytes(&base, &holder_wallet, 100)));
    rt.store(&holder_quote, owned(spl_token::ID, token_account_bytes(&quote, &holder_wallet, 0)));
    eligible = 100u64.to_le_bytes().to_vec();
    let mut pay = pay_accounts.clone();
    pay.push(AccountMeta::new_readonly(holder_base, false));
    pay.push(AccountMeta::new(holder_quote, false));
    let stranger = Keypair::new();
    fund(&mut rt, &stranger.pubkey(), 1_000_000_000).await;
    let mut stranger_pay = pay.clone();
    stranger_pay[0] = AccountMeta::new(stranger.pubkey(), true);
    let denied = rt.send_payer(&[ix(IX_PAY_V2, &eligible, stranger_pay)], &stranger, &[]).await;
    assert_err(&denied, ERR_NOT_CRANK, "pay not crank");

    assert_ok(&rt.send(&[ix(IX_PAY_V2, &eligible, pay.clone())], &[]).await, "pay holder");
    assert_eq!(token_amount_of(&rt.account(&holder_quote).await.unwrap()), 9_000);
    assert_eq!(token_amount_of(&rt.account(&vault).await.unwrap()), 0);
    assert_eq!(pool_pots(&rt.account(&pool_rec).await.unwrap()).1, 0);
    let again = rt.send(&[ix(IX_PAY_V2, &eligible, pay)], &[]).await;
    assert_err(&again, ERR_EMPTY_POT, "second pay");
    assert_eq!(token_amount_of(&rt.account(&holder_quote).await.unwrap()), 9_000);

    // Vault-short burn on a second pool sharing nothing with the first.
    let pool2 = Pubkey::new_unique();
    let config2 = config;
    rt.store(&pool2, owned(DBC_PROGRAM_ID, dbc_pool_bytes(&config2, &creator.pubkey(), &base, 0)));
    let (qs2_unused, _) = pda(&[b"qstat", quote.as_ref()]);
    assert_ok(
        &rt.send(
            &[register_pool_ix(admin, global, quote, base, config2, pool2, Some((qs2_unused, true)), &[])],
            &[],
        )
        .await,
        "second pool",
    );
    assert_eq!(qstat_count(&mut rt, &quote).await, Some(2));
    let (rec2, bump2) = pda(&[b"pool", pool2.as_ref()]);
    rt.store(
        &rec2,
        owned(pid(), pool_record_bytes(&pool2, &base, &quote, &config2, bump2, false, 1, 0, 5_000)),
    );
    // Vault is empty, so this pool's burn cannot be covered.
    let short = rt
        .send(
            &[ix(
                IX_BURN_DIRECT,
                &[],
                vec![
                    AccountMeta::new_readonly(global, false),
                    AccountMeta::new(rec2, false),
                    AccountMeta::new(quote, false),
                    AccountMeta::new(vault, false),
                    AccountMeta::new_readonly(spl_token::ID, false),
                ],
            )],
            &[],
        )
        .await;
    assert_err(&short, ERR_VAULT_SHORT, "burn vault short");
    assert_eq!(pool_pots(&rt.account(&rec2).await.unwrap()).2, 5_000);

    // Known gap: release_burn_pot only checks this pool's burn against the shared vault.
    let op = Pubkey::new_unique();
    rt.store(&op, owned(spl_token::ID, token_account_bytes(&quote, &admin, 0)));
    rt.store(&vault, owned(spl_token::ID, token_account_bytes(&quote, &global, 8_000)));
    rt.store(
        &pool_rec,
        owned(pid(), pool_record_bytes(&pool, &base, &quote, &config, {
            let (_, b) = pda(&[b"pool", pool.as_ref()]);
            b
        }, false, 1, 8_000, 0)),
    );
    rt.store(
        &rec2,
        owned(pid(), pool_record_bytes(&pool2, &base, &quote, &config2, bump2, false, 1, 0, 5_000)),
    );
    let release = ix(
        IX_RELEASE_BURN,
        &[],
        vec![
            AccountMeta::new_readonly(global, false),
            AccountMeta::new(rec2, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(op, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
    );
    assert_ok(&release_res(&mut rt, release).await, "known gap release ignores other holder pot");
    assert_eq!(token_amount_of(&rt.account(&vault).await.unwrap()), 3_000);
    assert_eq!(pool_pots(&rt.account(&pool_rec).await.unwrap()).1, 8_000);
    assert_eq!(pool_pots(&rt.account(&rec2).await.unwrap()).2, 0);

    // Graduated creator path. New quote so the burn mint lock does not force this quote.
    creator_path(&mut rt, &global, &admin, &auth, &base).await;

    let pause = ix(IX_SET_PAUSED, &[1], vec![
        AccountMeta::new_readonly(admin, true),
        AccountMeta::new(global, false),
    ]);
    assert_ok(&rt.send(&[pause], &[]).await, "pause");
    let paused_pay = rt.send(&[ix(IX_PAY_V2, &eligible, pay_accounts)], &[]).await;
    assert_err(&paused_pay, ERR_PAUSED, "paused pay");
}

async fn release_res(rt: &mut Rt, ix: Instruction) -> Result<(Vec<String>, Option<Vec<u8>>), String> {
    rt.send(&[ix], &[]).await
}

async fn creator_path(rt: &mut Rt, global: &Pubkey, admin: &Pubkey, auth: &Pubkey, base: &Pubkey) {
    let quote = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    let creator = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, spl_mint_bytes(9, auth, false)));
    rt.store(&config, owned(DBC_PROGRAM_ID, dbc_config_bytes(&quote, global)));
    rt.store(&pool, owned(DBC_PROGRAM_ID, dbc_pool_bytes(&config, &creator, base, 1)));
    let vault = ata(global, &quote, &spl_token::ID);
    rt.store(&vault, owned(spl_token::ID, token_account_bytes(&quote, global, 0)));
    assert_ok(
        &rt.send(&[register_config_ix(*admin, *global, quote, config)], &[]).await,
        "creator register_config",
    );
    assert_ok(&rt.send(&[sync_ix(*admin, *global, quote, 0)], &[]).await, "creator sync 0");
    let (qs, _) = pda(&[b"qstat", quote.as_ref()]);
    assert_ok(
        &rt.send(&[register_pool_ix(*admin, *global, quote, *base, config, pool, Some((qs, true)), &[])], &[])
            .await,
        "creator register_pool",
    );
    let (pool_rec, _) = pda(&[b"pool", pool.as_ref()]);
    let marked = rt
        .send(
            &[ix(
                IX_MARK_GRADUATED,
                &[],
                vec![
                    AccountMeta::new_readonly(*global, false),
                    AccountMeta::new(pool_rec, false),
                    AccountMeta::new_readonly(pool, false),
                ],
            )],
            &[],
        )
        .await;
    assert_ok(&marked, "mark_graduated");

    let position = Pubkey::new_unique();
    let tok_a = ata(global, base, &spl_token::ID);
    let nft = Pubkey::new_unique();
    let ta_vault = Pubkey::new_unique();
    let tb_vault = Pubkey::new_unique();
    let event = Pubkey::new_unique();
    rt.store(&position, owned(DAMM_PROGRAM_ID, vec![0; 8]));
    rt.store(&tok_a, owned(spl_token::ID, token_account_bytes(base, global, 0)));
    rt.store(&nft, owned(spl_token::ID, token_account_bytes(&Pubkey::new_unique(), global, 1)));
    rt.store(&ta_vault, owned(spl_token::ID, token_account_bytes(base, global, 0)));
    // Source for the mocked DAMM claim. Global signs that CPI.
    rt.store(&tb_vault, owned(spl_token::ID, token_account_bytes(&quote, global, 100_000)));
    rt.store(&event, owned(system_program::ID, vec![]));
    let (owed, _) = pda(&[b"cown", pool.as_ref()]);
    let mut accts = vec![
        AccountMeta::new_readonly(*global, false),
        AccountMeta::new(pool_rec, false),
        AccountMeta::new(vault, false),
        AccountMeta::new_readonly(DBC_POOL_AUTHORITY, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new(position, false),
        AccountMeta::new(tok_a, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(ta_vault, false),
        AccountMeta::new(tb_vault, false),
        AccountMeta::new_readonly(*base, false),
        AccountMeta::new_readonly(quote, false),
        AccountMeta::new_readonly(nft, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(event, false),
        AccountMeta::new_readonly(DAMM_PROGRAM_ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new_readonly(config, false),
        AccountMeta::new(owed, false),
    ];
    // Missing the 5 tail accounts must fail closed.
    let short_tail = accts[..accts.len() - 5].to_vec();
    let missing = rt.send(&[ix(IX_CRANK_CLAIM_DAMM, &[], short_tail)], &[]).await;
    assert_err(&missing, ERR_USE_SPLIT, "damm claim without creator accounts");

    assert_ok(&rt.send(&[ix(IX_CRANK_CLAIM_DAMM, &[], accts.split_off(0))], &[]).await, "damm claim");
    // 8% of 11000 = 880 creator, rest 10120, burn 1840, holder 8280.
    let pots = pool_pots(&rt.account(&pool_rec).await.unwrap());
    assert_eq!(pots, (1, 8_280, 1_840), "creator 8% then 90/20 on the rest");
    let owed_acct = rt.account(&owed).await.unwrap();
    assert_eq!(u64_at(&owed_acct.data, 40), 880);

    let creator_dest = ata(&creator, &quote, &spl_token::ID);
    rt.store(&creator_dest, owned(spl_token::ID, token_account_bytes(&quote, &creator, 0)));
    // Solvency: owed 880 + holder 8280 + burn 1840 = 11000, vault is 11000. Pay succeeds.
    let claim = ix(
        IX_CLAIM_CREATOR,
        &[],
        vec![
            AccountMeta::new_readonly(*global, false),
            AccountMeta::new_readonly(pool_rec, false),
            AccountMeta::new(owed, false),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(quote, false),
            AccountMeta::new(creator_dest, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
    );
    assert_ok(&rt.send(&[claim.clone()], &[]).await, "claim_creator_share");
    assert_eq!(token_amount_of(&rt.account(&creator_dest).await.unwrap()), 880);
    assert_eq!(u64_at(&rt.account(&owed).await.unwrap().data, 40), 0);
    let second = rt.send(&[claim], &[]).await;
    assert_err(&second, ERR_NO_CREATOR, "creator paid twice");

    // Drop the vault under holder+burn and show a fresh owed cannot be paid.
    // Re-seed owed by a second damm claim, then shrink the vault.
    rt.store(&vault, owned(spl_token::ID, token_account_bytes(&quote, global, 100)));
    let mut again = vec![
        AccountMeta::new_readonly(*global, false),
        AccountMeta::new(pool_rec, false),
        AccountMeta::new(vault, false),
        AccountMeta::new_readonly(DBC_POOL_AUTHORITY, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new(position, false),
        AccountMeta::new(tok_a, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(ta_vault, false),
        AccountMeta::new(tb_vault, false),
        AccountMeta::new_readonly(*base, false),
        AccountMeta::new_readonly(quote, false),
        AccountMeta::new_readonly(nft, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(event, false),
        AccountMeta::new_readonly(DAMM_PROGRAM_ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new(*admin, true),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new_readonly(config, false),
        AccountMeta::new(owed, false),
    ];
    assert_ok(&rt.send(&[ix(IX_CRANK_CLAIM_DAMM, &[], again.split_off(0))], &[]).await, "second damm claim");
    // Vault was 100, mock added 11000, now 11100. Owed increased. Shrink vault below reserved.
    rt.store(&vault, owned(spl_token::ID, token_account_bytes(&quote, global, 10)));
    let creator_dest2 = creator_dest;
    let short_claim = ix(
        IX_CLAIM_CREATOR,
        &[],
        vec![
            AccountMeta::new_readonly(*global, false),
            AccountMeta::new_readonly(pool_rec, false),
            AccountMeta::new(owed, false),
            AccountMeta::new_readonly(pool, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(quote, false),
            AccountMeta::new(creator_dest2, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
    );
    let short = rt.send(&[short_claim], &[]).await;
    assert_err(&short, ERR_VAULT_SHORT, "creator claim vault short");
}

#[tokio::test]
async fn auto_lp_allocation_and_known_gaps() {
    let (global, _) = pda(&[b"global"]);
    let auth = Pubkey::new_unique();
    let mut rt = bootstrap(program_test()).await;
    let admin = rt.ctx.payer.pubkey();
    let (auto, _) = pda(&[b"autolp"]);
    assert_ok(
        &rt.send(
            &[ix(
                IX_INIT_AUTO_LP,
                &[],
                vec![
                    AccountMeta::new(admin, true),
                    AccountMeta::new_readonly(global, false),
                    AccountMeta::new(auto, false),
                    AccountMeta::new_readonly(system_program::ID, false),
                ],
            )],
            &[],
        )
        .await,
        "init_auto_lp",
    );
    let mut not_ready = vec![
        AccountMeta::new_readonly(global, false),
        AccountMeta::new_readonly(auto, false),
    ];
    for _ in 0..14 {
        not_ready.push(AccountMeta::new_readonly(admin, false));
    }
    let early = rt
        .send(&[ix(IX_CRANK_AUTO_LP, &auto_lp_args(1, 1, 1), not_ready)], &[])
        .await;
    assert_err(&early, ERR_AUTO_LP_NOT_READY, "auto-lp stays off until set_auto_lp");

    let (mint_a, mint_b, pool_a, pool_b) = setup_pair(&mut rt, &global, &admin, &auth, 10_000, 5_000, 10_000, 5_000).await;
    let vault_a = ata(&global, &mint_a, &spl_token::ID);
    let vault_b = ata(&global, &mint_b, &spl_token::ID);
    rt.store(&vault_a, owned(spl_token::ID, token_account_bytes(&mint_a, &global, 15_000)));
    rt.store(&vault_b, owned(spl_token::ID, token_account_bytes(&mint_b, &global, 15_000)));
    let damm_pool = Pubkey::new_unique();
    let position = Pubkey::new_unique();
    let nft = Pubkey::new_unique();
    rt.store(&damm_pool, owned(DAMM_PROGRAM_ID, vec![0; 8]));
    rt.store(&position, owned(DAMM_PROGRAM_ID, vec![0; 8]));
    rt.store(&nft, owned(spl_token::ID, token_account_bytes(&Pubkey::new_unique(), &global, 1)));
    let damm_va = Pubkey::new_unique();
    let damm_vb = Pubkey::new_unique();
    rt.store(&damm_va, owned(spl_token::ID, token_account_bytes(&mint_a, &damm_pool, 0)));
    rt.store(&damm_vb, owned(spl_token::ID, token_account_bytes(&mint_b, &damm_pool, 0)));
    arm_auto_lp(&mut rt, &global, &admin, &auto, &damm_pool, &position, &nft, &mint_a, &mint_b).await;

    let (rec_a, _) = pda(&[b"pool", pool_a.as_ref()]);
    let (rec_b, _) = pda(&[b"pool", pool_b.as_ref()]);
    let (qs_a, _) = pda(&[b"qstat", mint_a.as_ref()]);
    let (qs_b, _) = pda(&[b"qstat", mint_b.as_ref()]);
    let event = Pubkey::new_unique();
    let too_big = auto_lp_call(
        &global, &auto, &admin, &damm_pool, &position, &nft, &vault_a, &vault_b, &damm_va, &damm_vb,
        &mint_a, &mint_b, &event, &qs_a, &qs_b, &[pool_a, pool_b], 1, 50_000, 50_000,
    );
    let over = rt.send(&[too_big], &[]).await;
    assert_err(&over, ERR_BAD_AUTO_LP_AMT, "threshold above burn budget");
    assert_eq!(rt.account(&position).await.unwrap().data[0], 0, "cpi must not run");

    let ok = auto_lp_call(
        &global, &auto, &admin, &damm_pool, &position, &nft, &vault_a, &vault_b, &damm_va, &damm_vb,
        &mint_a, &mint_b, &event, &qs_a, &qs_b, &[pool_a, pool_b], 1, 5_000, 5_000,
    );
    assert_ok(&rt.send(&[ok], &[]).await, "auto-lp add and permanent lock");
    let pos = rt.account(&position).await.unwrap();
    assert_eq!(&pos.data[..2], &[1, 1], "add then permanent lock");
    assert_eq!(pool_pots(&rt.account(&rec_a).await.unwrap()), (0, 5_000, 5_000));
    assert_eq!(pool_pots(&rt.account(&rec_b).await.unwrap()), (0, 5_000, 5_000));
    assert_eq!(token_amount_of(&rt.account(&vault_a).await.unwrap()), 10_000);
    assert_eq!(token_amount_of(&rt.account(&vault_b).await.unwrap()), 10_000);

    let mismatch = auto_lp_call(
        &global, &auto, &admin, &damm_pool, &position, &nft, &vault_a, &vault_b, &damm_va, &damm_vb,
        &mint_a, &mint_b, &event, &qs_a, &qs_b, &[], 1, 1, 1,
    );
    let miss = rt.send(&[mismatch], &[]).await;
    assert_err(&miss, ERR_BAD_AUTO_LP, "stale or short pool list");
    assert_eq!(pool_pots(&rt.account(&rec_a).await.unwrap()).2, 5_000);

    // Known gap: duplicate the same pool account to satisfy QuoteStat and omit another pool,
    // and omit CreatorOwed so its balance is treated as surplus.
    known_gap_duplicate_and_owed(&mut rt, &global, &admin, &auth, &auto).await;
}

fn auto_lp_args(liq: u128, a: u64, b: u64) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&liq.to_le_bytes());
    d.extend_from_slice(&a.to_le_bytes());
    d.extend_from_slice(&b.to_le_bytes());
    d
}

async fn setup_pair(
    rt: &mut Rt,
    global: &Pubkey,
    admin: &Pubkey,
    auth: &Pubkey,
    burn_a: u64,
    holder_a: u64,
    burn_b: u64,
    holder_b: u64,
) -> (Pubkey, Pubkey, Pubkey, Pubkey) {
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let cfg_a = Pubkey::new_unique();
    let cfg_b = Pubkey::new_unique();
    let pool_a = Pubkey::new_unique();
    let pool_b = Pubkey::new_unique();
    let creator = Pubkey::new_unique();
    rt.store(&mint_a, owned(spl_token::ID, spl_mint_bytes(9, auth, false)));
    rt.store(&mint_b, owned(spl_token::ID, spl_mint_bytes(9, auth, false)));
    rt.store(&base, owned(spl_token::ID, spl_mint_bytes(6, auth, false)));
    rt.store(&cfg_a, owned(DBC_PROGRAM_ID, dbc_config_bytes(&mint_a, global)));
    rt.store(&cfg_b, owned(DBC_PROGRAM_ID, dbc_config_bytes(&mint_b, global)));
    rt.store(&pool_a, owned(DBC_PROGRAM_ID, dbc_pool_bytes(&cfg_a, &creator, &base, 0)));
    rt.store(&pool_b, owned(DBC_PROGRAM_ID, dbc_pool_bytes(&cfg_b, &creator, &base, 0)));
    assert_ok(&rt.send(&[register_config_ix(*admin, *global, mint_a, cfg_a)], &[]).await, "cfg a");
    assert_ok(&rt.send(&[register_config_ix(*admin, *global, mint_b, cfg_b)], &[]).await, "cfg b");
    assert_ok(&rt.send(&[sync_ix(*admin, *global, mint_a, 0)], &[]).await, "sync a");
    assert_ok(&rt.send(&[sync_ix(*admin, *global, mint_b, 0)], &[]).await, "sync b");
    let (qs_a, _) = pda(&[b"qstat", mint_a.as_ref()]);
    let (qs_b, _) = pda(&[b"qstat", mint_b.as_ref()]);
    assert_ok(
        &rt.send(&[register_pool_ix(*admin, *global, mint_a, base, cfg_a, pool_a, Some((qs_a, true)), &[])], &[]).await,
        "pool a",
    );
    assert_ok(
        &rt.send(&[register_pool_ix(*admin, *global, mint_b, base, cfg_b, pool_b, Some((qs_b, true)), &[])], &[]).await,
        "pool b",
    );
    let (rec_a, bump_a) = pda(&[b"pool", pool_a.as_ref()]);
    let (rec_b, bump_b) = pda(&[b"pool", pool_b.as_ref()]);
    rt.store(&rec_a, owned(pid(), pool_record_bytes(&pool_a, &base, &mint_a, &cfg_a, bump_a, false, 0, holder_a, burn_a)));
    rt.store(&rec_b, owned(pid(), pool_record_bytes(&pool_b, &base, &mint_b, &cfg_b, bump_b, false, 0, holder_b, burn_b)));
    (mint_a, mint_b, pool_a, pool_b)
}

async fn arm_auto_lp(
    rt: &mut Rt,
    global: &Pubkey,
    admin: &Pubkey,
    auto: &Pubkey,
    damm: &Pubkey,
    position: &Pubkey,
    nft: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) {
    let mut data = Vec::new();
    for k in [damm, position, nft, mint_a, mint_b] {
        data.extend_from_slice(k.as_ref());
    }
    assert_ok(
        &rt.send(
            &[ix(
                IX_SET_AUTO_LP,
                &data,
                vec![
                    AccountMeta::new_readonly(*global, false),
                    AccountMeta::new_readonly(*admin, true),
                    AccountMeta::new(*auto, false),
                ],
            )],
            &[],
        )
        .await,
        "set_auto_lp",
    );
    assert_ok(
        &rt.send(
            &[ix(
                IX_SET_AUTO_LP_SHARE,
                &5_000u16.to_le_bytes(),
                vec![
                    AccountMeta::new_readonly(*global, false),
                    AccountMeta::new_readonly(*admin, true),
                    AccountMeta::new(*auto, false),
                ],
            )],
            &[],
        )
        .await,
        "set share 5000",
    );
}

fn auto_lp_call(
    global: &Pubkey,
    auto: &Pubkey,
    admin: &Pubkey,
    damm_pool: &Pubkey,
    position: &Pubkey,
    nft: &Pubkey,
    tok_a: &Pubkey,
    tok_b: &Pubkey,
    vault_a: &Pubkey,
    vault_b: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    event: &Pubkey,
    qs_a: &Pubkey,
    qs_b: &Pubkey,
    pools: &[Pubkey],
    liq: u128,
    thresh_a: u64,
    thresh_b: u64,
) -> Instruction {
    let mut metas = vec![
        AccountMeta::new_readonly(*global, false),
        AccountMeta::new_readonly(*auto, false),
        AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(*damm_pool, false),
        AccountMeta::new(*position, false),
        AccountMeta::new_readonly(*nft, false),
        AccountMeta::new(*tok_a, false),
        AccountMeta::new(*tok_b, false),
        AccountMeta::new(*vault_a, false),
        AccountMeta::new(*vault_b, false),
        AccountMeta::new_readonly(*mint_a, false),
        AccountMeta::new_readonly(*mint_b, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(*event, false),
        AccountMeta::new_readonly(DAMM_PROGRAM_ID, false),
        AccountMeta::new_readonly(*qs_a, false),
        AccountMeta::new_readonly(*qs_b, false),
    ];
    for p in pools {
        let (rec, _) = pda(&[b"pool", p.as_ref()]);
        metas.push(AccountMeta::new(rec, false));
    }
    ix(IX_CRANK_AUTO_LP, &auto_lp_args(liq, thresh_a, thresh_b), metas)
}

async fn known_gap_duplicate_and_owed(rt: &mut Rt, global: &Pubkey, admin: &Pubkey, auth: &Pubkey, auto: &Pubkey) {
    let base = Pubkey::new_unique();
    let mint_a = Pubkey::new_unique();
    let mint_b = Pubkey::new_unique();
    let cfg_a = Pubkey::new_unique();
    let cfg_b = Pubkey::new_unique();
    let pool_a1 = Pubkey::new_unique();
    let pool_a2 = Pubkey::new_unique();
    let pool_b = Pubkey::new_unique();
    let creator = Pubkey::new_unique();
    rt.store(&base, owned(spl_token::ID, spl_mint_bytes(6, auth, false)));
    rt.store(&mint_a, owned(spl_token::ID, spl_mint_bytes(9, auth, false)));
    rt.store(&mint_b, owned(spl_token::ID, spl_mint_bytes(9, auth, false)));
    rt.store(&cfg_a, owned(DBC_PROGRAM_ID, dbc_config_bytes(&mint_a, global)));
    rt.store(&cfg_b, owned(DBC_PROGRAM_ID, dbc_config_bytes(&mint_b, global)));
    for (pool, cfg, mint) in [(&pool_a1, &cfg_a, &mint_a), (&pool_a2, &cfg_a, &mint_a), (&pool_b, &cfg_b, &mint_b)] {
        rt.store(pool, owned(DBC_PROGRAM_ID, dbc_pool_bytes(cfg, &creator, &base, 0)));
    }
    assert_ok(&rt.send(&[register_config_ix(*admin, *global, mint_a, cfg_a)], &[]).await, "gap cfg a");
    assert_ok(&rt.send(&[register_config_ix(*admin, *global, mint_b, cfg_b)], &[]).await, "gap cfg b");
    assert_ok(&rt.send(&[sync_ix(*admin, *global, mint_a, 0)], &[]).await, "gap sync a");
    assert_ok(&rt.send(&[sync_ix(*admin, *global, mint_b, 0)], &[]).await, "gap sync b");
    let (qs_a, _) = pda(&[b"qstat", mint_a.as_ref()]);
    let (qs_b, _) = pda(&[b"qstat", mint_b.as_ref()]);
    for (pool, mint, cfg, qs) in [
        (pool_a1, mint_a, cfg_a, qs_a),
        (pool_a2, mint_a, cfg_a, qs_a),
        (pool_b, mint_b, cfg_b, qs_b),
    ] {
        assert_ok(
            &rt.send(&[register_pool_ix(*admin, *global, mint, base, cfg, pool, Some((qs, true)), &[])], &[]).await,
            "gap register",
        );
    }
    assert_eq!(qstat_count(rt, &mint_a).await, Some(2));
    assert_eq!(qstat_count(rt, &mint_b).await, Some(1));
    let (rec_a1, b1) = pda(&[b"pool", pool_a1.as_ref()]);
    let (rec_a2, b2) = pda(&[b"pool", pool_a2.as_ref()]);
    let (rec_b, bb) = pda(&[b"pool", pool_b.as_ref()]);
    rt.store(&rec_a1, owned(pid(), pool_record_bytes(&pool_a1, &base, &mint_a, &cfg_a, b1, false, 0, 0, 1_000)));
    rt.store(&rec_a2, owned(pid(), pool_record_bytes(&pool_a2, &base, &mint_a, &cfg_a, b2, false, 0, 20_000, 0)));
    rt.store(&rec_b, owned(pid(), pool_record_bytes(&pool_b, &base, &mint_b, &cfg_b, bb, false, 0, 0, 10_000)));
    // Creator owed for A2, intentionally NOT passed.
    let (owed, obump) = pda(&[b"cown", pool_a2.as_ref()]);
    let mut owed_data = vec![83, 80, 110, 53, 75, 132, 102, 33];
    owed_data.extend_from_slice(pool_a2.as_ref());
    owed_data.extend_from_slice(&5_000u64.to_le_bytes());
    owed_data.extend_from_slice(&0u64.to_le_bytes());
    owed_data.push(obump);
    rt.store(&owed, owned(pid(), owed_data));

    let vault_a = ata(global, &mint_a, &spl_token::ID);
    let vault_b = ata(global, &mint_b, &spl_token::ID);
    // A vault covers A1 burn 1000 + A2 holder 20000. The omitted owed is extra and not in this balance;
    // the duplicate makes A2's holder look like surplus.
    rt.store(&vault_a, owned(spl_token::ID, token_account_bytes(&mint_a, global, 21_000)));
    rt.store(&vault_b, owned(spl_token::ID, token_account_bytes(&mint_b, global, 10_000)));
    let damm_pool = Pubkey::new_unique();
    let position = Pubkey::new_unique();
    let nft = Pubkey::new_unique();
    rt.store(&damm_pool, owned(DAMM_PROGRAM_ID, vec![0; 8]));
    rt.store(&position, owned(DAMM_PROGRAM_ID, vec![0; 8]));
    rt.store(&nft, owned(spl_token::ID, token_account_bytes(&Pubkey::new_unique(), global, 1)));
    let dva = Pubkey::new_unique();
    let dvb = Pubkey::new_unique();
    rt.store(&dva, owned(spl_token::ID, token_account_bytes(&mint_a, &damm_pool, 0)));
    rt.store(&dvb, owned(spl_token::ID, token_account_bytes(&mint_b, &damm_pool, 0)));
    arm_auto_lp(rt, global, admin, auto, &damm_pool, &position, &nft, &mint_a, &mint_b).await;
    let event = Pubkey::new_unique();
    let mut ixn = auto_lp_call(
        global, auto, admin, &damm_pool, &position, &nft, &vault_a, &vault_b, &dva, &dvb,
        &mint_a, &mint_b, &event, &qs_a, &qs_b, &[pool_a1], 1, 19_000, 5_000,
    );
    // Pass pool A1 a second time so the declared count of 2 is satisfied without A2.
    let (rec_dup, _) = pda(&[b"pool", pool_a1.as_ref()]);
    // auto_lp_call already pushed rec for pool_a1. Insert B as well and duplicate A1.
    ixn.accounts.push(AccountMeta::new(rec_dup, false));
    let (rec_b_meta, _) = pda(&[b"pool", pool_b.as_ref()]);
    ixn.accounts.push(AccountMeta::new(rec_b_meta, false));
    let res = rt.send(&[ixn], &[]).await;
    assert_ok(&res, "known gap: duplicate pool alias and omitted creator owed");
    assert_eq!(token_amount_of(&rt.account(&vault_a).await.unwrap()), 2_000);
    assert_eq!(pool_pots(&rt.account(&rec_a2).await.unwrap()).1, 20_000, "omitted holder pot still booked");
    assert_eq!(u64_at(&rt.account(&owed).await.unwrap().data, 40), 5_000, "owed still recorded");
    // Vault can no longer cover the omitted holder.
    assert!(token_amount_of(&rt.account(&vault_a).await.unwrap()) < 20_000);
}
