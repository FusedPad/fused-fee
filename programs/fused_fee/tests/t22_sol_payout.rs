//! Offline bank tests for crank_pay_page_v4 (Token-2022 base coins, plain-SOL payout for SOL pairs),
//! plus regression checks that v2 / v3 / release_burn_pot / crank_burn_direct behave as before.
//! Loads the real built .so (FUSED_SO, default target/deploy/fused_fee.so) and the real SPL Token,
//! Token-2022 and ATA programs that solana-program-test ships. No network, no real keys.
use solana_account::{Account, AccountSharedData};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_signer::Signer;
use solana_system_interface::program as system_program;
use solana_transaction::Transaction;

const PROGRAM: Pubkey = solana_pubkey::pubkey!("AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9");
const DBC: Pubkey = solana_pubkey::pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
const DBC_POOL_AUTHORITY: Pubkey = solana_pubkey::pubkey!("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");
const BPF: Pubkey = solana_pubkey::pubkey!("BPFLoader2111111111111111111111111111111111");
const TOKEN: Pubkey = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const T22: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const ATA_PROG: Pubkey = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const WSOL: Pubkey = solana_pubkey::pubkey!("So11111111111111111111111111111111111111112");
const FUSED: Pubkey = solana_pubkey::pubkey!("7SLA53amAEFJxaFJ4VLcS8RqxMg7Bmav1HnsCz7vpqhr");

const IX_INIT: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const IX_REG_CFG: [u8; 8] = [32, 247, 82, 131, 35, 183, 7, 57];
const IX_REG_POOL: [u8; 8] = [85, 229, 114, 47, 75, 145, 166, 100];
const IX_SYNC: [u8; 8] = [130, 168, 89, 25, 125, 164, 73, 104];
const IX_SET_BURN_MINT: [u8; 8] = [245, 243, 127, 223, 152, 205, 137, 210];
const IX_SET_PAUSED: [u8; 8] = [91, 60, 125, 192, 176, 225, 166, 218];
const IX_RELEASE: [u8; 8] = [224, 64, 74, 119, 186, 164, 250, 47];
const IX_BURN_DIRECT: [u8; 8] = [118, 75, 25, 157, 201, 143, 78, 137];
const IX_PAY2: [u8; 8] = [187, 255, 119, 89, 187, 17, 96, 79];
const IX_PAY3: [u8; 8] = [26, 92, 113, 244, 121, 99, 140, 121];
const IX_PAY4: [u8; 8] = [110, 93, 44, 108, 85, 172, 61, 155];

// FusedError codes (6000 + enum index; unchanged by this branch).
const E_BAD_MINT: u32 = 6000;
const E_PAUSED: u32 = 6004;
const E_BAD_TOTAL: u32 = 6008;
const E_BAD_PAGE: u32 = 6009;
const E_EMPTY_POT: u32 = 6010;
const E_OWNER_MISMATCH: u32 = 6011;
const E_NOT_CRANK: u32 = 6023;
const E_BAD_VAULT: u32 = 6024;
const E_BAD_ROUND: u32 = 6026;
const E_BAD_DEST: u32 = 6027;
const E_VAULT_SHORT: u32 = 6034;
const E_WRONG_TOKEN_PROGRAM: u32 = 6035;
const E_BAD_TOKEN_ACCOUNT: u32 = 6038;
/// Anchor: AccountOwnedByWrongProgram (what v2 gives for a Token-2022 holder account).
const E_ANCHOR_WRONG_OWNER: u32 = 3007;

// PoolRecord byte offsets (8 disc + pool + base + quote + config + graduated).
const PR_ROUND: usize = 137;
const PR_HOLDER: usize = 145;
const PR_BURN: usize = 153;
// PayCursor byte offsets (8 disc + pool + round + last_owner + paid_base + paid_amount).
const PC_ROUND: usize = 40;
const PC_LAST: usize = 48;
const PC_PAID_BASE: usize = 80;
const PC_PAID_AMOUNT: usize = 88;

struct Env {
    ctx: ProgramTestContext,
    slot: u64,
}
struct Sent {
    cu: u64,
    ret: Option<Vec<u8>>,
}
impl Env {
    async fn send_with(&mut self, disc: [u8; 8], data: &[u8], accounts: Vec<AccountMeta>, extra: &[&Keypair]) -> Result<Sent, String> {
        self.slot += 1;
        let _ = self.ctx.warp_to_slot(self.slot);
        let bh = self.ctx.banks_client.get_latest_blockhash().await.unwrap();
        let payer = self.ctx.payer.insecure_clone();
        let mut buf = disc.to_vec();
        buf.extend_from_slice(data);
        let ix = Instruction { program_id: PROGRAM, accounts, data: buf };
        let mut signers: Vec<&Keypair> = vec![&payer];
        signers.extend(extra);
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &signers, bh);
        match self.ctx.banks_client.process_transaction_with_metadata(tx).await {
            Ok(r) => match r.result {
                Ok(()) => {
                    let m = r.metadata.unwrap();
                    Ok(Sent { cu: m.compute_units_consumed, ret: m.return_data.map(|d| d.data) })
                }
                Err(e) => Err(format!("{e:?} logs={:?}", r.metadata.map(|m| m.log_messages))),
            },
            Err(e) => Err(format!("{e:?}")),
        }
    }
    async fn send(&mut self, disc: [u8; 8], data: &[u8], accounts: Vec<AccountMeta>) -> Result<Sent, String> {
        self.send_with(disc, data, accounts, &[]).await
    }
    fn store(&mut self, k: &Pubkey, a: Account) {
        self.ctx.set_account(k, &AccountSharedData::from(a));
    }
    async fn acct(&mut self, k: &Pubkey) -> Option<Account> {
        self.ctx.banks_client.get_account(*k).await.unwrap()
    }
    async fn lamports(&mut self, k: &Pubkey) -> u64 {
        self.acct(k).await.map(|a| a.lamports).unwrap_or(0)
    }
    async fn tok(&mut self, k: &Pubkey) -> u64 {
        amt(&self.acct(k).await.unwrap())
    }
    async fn u64_at(&mut self, k: &Pubkey, off: usize) -> u64 {
        u64::from_le_bytes(self.acct(k).await.unwrap().data[off..off + 8].try_into().unwrap())
    }
    async fn set_u64(&mut self, k: &Pubkey, off: usize, v: u64) {
        let mut a = self.acct(k).await.unwrap();
        a.data[off..off + 8].copy_from_slice(&v.to_le_bytes());
        self.store(k, a);
    }
    fn payer(&self) -> Pubkey {
        self.ctx.payer.pubkey()
    }
}

fn rent(n: usize) -> u64 {
    Rent::default().minimum_balance(n).max(1)
}
fn owned(owner: Pubkey, data: Vec<u8>) -> Account {
    Account { lamports: rent(data.len()), data, owner, executable: false, rent_epoch: 0 }
}
fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &PROGRAM).0
}
/// Classic-layout mint (82 bytes). `auth` None = no mint authority. supply set for burn tests.
fn mint_data(dec: u8, auth: Option<&Pubkey>, supply: u64) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    if let Some(a) = auth {
        d[0..4].copy_from_slice(&1u32.to_le_bytes());
        d[4..36].copy_from_slice(a.as_ref());
    }
    d[36..44].copy_from_slice(&supply.to_le_bytes());
    d[44] = dec;
    d[45] = 1;
    d
}
/// Token-2022 mint like a DBC token_type=1 launch: base mint + AccountType::Mint + MetadataPointer TLV.
fn t22_launch_mint(dec: u8) -> Vec<u8> {
    let mut d = mint_data(dec, None, 1_000_000_000_000_000);
    d.resize(165, 0);
    d.push(1); // AccountType::Mint
    d.extend_from_slice(&18u16.to_le_bytes()); // MetadataPointer
    d.extend_from_slice(&64u16.to_le_bytes());
    d.extend_from_slice(&[7u8; 64]);
    d
}
fn token(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = vec![0u8; 165];
    d[0..32].copy_from_slice(mint.as_ref());
    d[32..64].copy_from_slice(owner.as_ref());
    d[64..72].copy_from_slice(&amount.to_le_bytes());
    d[108] = 1;
    d
}
/// Token-2022 account with the ImmutableOwner extension (170 bytes, byte 165 = AccountType::Account).
fn token_t22_ext(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = token(mint, owner, amount);
    d.push(2);
    d.extend_from_slice(&7u16.to_le_bytes());
    d.extend_from_slice(&0u16.to_le_bytes());
    d
}
/// Token-2022 account with MemoTransfer { require_incoming_transfer_memos: true }.
fn token_t22_memo(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = token(mint, owner, amount);
    d.push(2);
    d.extend_from_slice(&8u16.to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.push(1);
    d
}
fn frozen(mut d: Vec<u8>) -> Vec<u8> {
    d[108] = 2;
    d
}
/// Native (wSOL) token account: is_native = Some(rent reserve), lamports = reserve + amount.
fn wsol_account(owner: &Pubkey, amount: u64) -> Account {
    let mut d = token(&WSOL, owner, amount);
    let reserve = rent(165);
    d[109..113].copy_from_slice(&1u32.to_le_bytes());
    d[113..121].copy_from_slice(&reserve.to_le_bytes());
    Account { lamports: reserve + amount, data: d, owner: TOKEN, executable: false, rent_epoch: 0 }
}
fn amt(a: &Account) -> u64 {
    u64::from_le_bytes(a.data[64..72].try_into().unwrap())
}
fn ata(owner: &Pubkey, mint: &Pubkey, prog: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[owner.as_ref(), prog.as_ref(), mint.as_ref()], &ATA_PROG).0
}
fn dbc_pool(config: &Pubkey, creator: &Pubkey, base: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 400];
    d[..8].copy_from_slice(&[213, 224, 5, 209, 98, 69, 119, 92]);
    d[72..104].copy_from_slice(config.as_ref());
    d[104..136].copy_from_slice(creator.as_ref());
    d[136..168].copy_from_slice(base.as_ref());
    d
}
/// DBC PoolConfig. byte 237 = token_type (0 SPL, 1 Token-2022); the program does not read it.
fn dbc_cfg(quote: &Pubkey, global: &Pubkey, token_type: u8) -> Vec<u8> {
    let mut d = vec![0u8; 1048];
    d[..8].copy_from_slice(&[26, 108, 14, 123, 116, 230, 129, 43]);
    d[8..40].copy_from_slice(quote.as_ref());
    d[40..72].copy_from_slice(global.as_ref());
    d[237] = token_type;
    d
}
fn custom(err: &str) -> Option<u32> {
    let marker = "Custom(";
    err.find(marker).and_then(|i| err[i + marker.len()..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
}
fn code(r: &Result<Sent, String>) -> Option<u32> {
    match r {
        Ok(_) => None,
        Err(e) => custom(e),
    }
}
fn ret_u64(s: &Sent) -> u64 {
    // the runtime trims trailing zero bytes of return data, so pad back to 8
    let mut d = s.ret.clone().unwrap_or_default();
    d.resize(8, 0);
    u64::from_le_bytes(d[..8].try_into().unwrap())
}

struct Quote {
    mint: Pubkey,
    prog: Pubkey,
    cfg: Pubkey,
    qs: Pubkey,
    dbc_config: Pubkey,
    vault: Pubkey,
}
struct Pool {
    #[allow(dead_code)]
    pool: Pubkey,
    prec: Pubkey,
    cursor: Pubkey,
    unwrap: Pubkey,
    base: Pubkey,
    bprog: Pubkey,
}

async fn start() -> (Env, Pubkey) {
    let so_path = std::env::var("FUSED_SO").unwrap_or_else(|_| "target/deploy/fused_fee.so".into());
    let so = std::fs::read(&so_path).unwrap_or_else(|e| panic!("read {so_path}: {e}"));
    let mut pt = ProgramTest::default();
    pt.prefer_bpf(false);
    pt.set_compute_max_units(1_400_000);
    pt.add_account(PROGRAM, Account { lamports: rent(so.len()), data: so, owner: BPF, executable: true, rent_epoch: 0 });
    let mut env = Env { ctx: pt.start_with_context().await, slot: 1 };
    // the real wSOL mint (no authority, 9 decimals)
    env.store(&WSOL, owned(TOKEN, mint_data(9, None, 0)));
    let g = pda(&[b"global"]);
    env.send(IX_INIT, &[], vec![
        AccountMeta::new(env.payer(), true),
        AccountMeta::new(g, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ])
    .await
    .map_err(|e| e)
    .expect("init");
    (env, g)
}

/// register_config + QuoteStat for a quote mint that already exists in the bank; opens Global's vault.
async fn add_quote(env: &mut Env, g: &Pubkey, mint: Pubkey, prog: Pubkey, vault_amount: u64, token_type: u8) -> Quote {
    let dbc_config = Pubkey::new_unique();
    env.store(&dbc_config, owned(DBC, dbc_cfg(&mint, g, token_type)));
    let cfg = pda(&[b"config", mint.as_ref()]);
    env.send(IX_REG_CFG, mint.as_ref(), vec![
        AccountMeta::new(env.payer(), true),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new_readonly(dbc_config, false),
        AccountMeta::new(cfg, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(mint, false),
    ])
    .await
    .expect("register_config");
    let qs = pda(&[b"qstat", mint.as_ref()]);
    let mut data = mint.to_bytes().to_vec();
    data.extend_from_slice(&0u64.to_le_bytes());
    env.send(IX_SYNC, &data, vec![
        AccountMeta::new(env.payer(), true),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new(qs, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ])
    .await
    .expect("sync");
    let vault = ata(g, &mint, &prog);
    if mint == WSOL {
        env.store(&vault, wsol_account(g, vault_amount));
    } else {
        env.store(&vault, owned(prog, token(&mint, g, vault_amount)));
    }
    Quote { mint, prog, cfg, qs, dbc_config, vault }
}

/// A launch: base mint under `bprog` (Token-2022 launches use the DBC token_type=1 shape), register_pool
/// with the website's 9-account order, then set round / pots directly.
async fn add_pool(env: &mut Env, g: &Pubkey, q: &Quote, bprog: Pubkey, round: u64, holder_pot: u64, burn_pot: u64) -> Pool {
    let base = Pubkey::new_unique();
    if bprog == T22 {
        env.store(&base, owned(T22, t22_launch_mint(6)));
    } else {
        env.store(&base, owned(TOKEN, mint_data(6, None, 1_000_000_000_000_000)));
    }
    let pool = Pubkey::new_unique();
    let payer = env.payer();
    env.store(&pool, owned(DBC, dbc_pool(&q.dbc_config, &payer, &base)));
    let prec = pda(&[b"pool", pool.as_ref()]);
    let mut data = Vec::new();
    data.extend_from_slice(pool.as_ref());
    data.extend_from_slice(base.as_ref());
    data.extend_from_slice(q.mint.as_ref());
    env.send(IX_REG_POOL, &data, vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(q.cfg, false),
        AccountMeta::new(prec, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new_readonly(q.dbc_config, false),
        AccountMeta::new_readonly(q.mint, false),
        AccountMeta::new(q.qs, false),
    ])
    .await
    .expect("register_pool");
    env.set_u64(&prec, PR_ROUND, round).await;
    env.set_u64(&prec, PR_HOLDER, holder_pot).await;
    env.set_u64(&prec, PR_BURN, burn_pot).await;
    Pool {
        pool,
        prec,
        cursor: pda(&[b"paycur", pool.as_ref()]),
        unwrap: pda(&[b"unwrap", pool.as_ref()]),
        base,
        bprog,
    }
}

fn v3_accounts(payer: Pubkey, g: &Pubkey, q: &Quote, p: &Pool) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new(p.prec, false),
        AccountMeta::new(q.vault, false),
        AccountMeta::new_readonly(q.mint, false),
        AccountMeta::new_readonly(q.prog, false),
        AccountMeta::new_readonly(p.base, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(p.cursor, false),
    ]
}
fn v4_accounts(payer: Pubkey, g: &Pubkey, q: &Quote, p: &Pool) -> Vec<AccountMeta> {
    let mut a = v3_accounts(payer, g, q, p);
    a.push(AccountMeta::new(pda(&[b"sol_vault"]), false));
    a.push(AccountMeta::new(p.unwrap, false));
    a
}
fn v2_accounts(payer: Pubkey, g: &Pubkey, q: &Quote, p: &Pool) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new(p.prec, false),
        AccountMeta::new(q.vault, false),
        AccountMeta::new_readonly(TOKEN, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(p.cursor, false),
        AccountMeta::new(pda(&[b"sol_vault"]), false),
        AccountMeta::new(p.unwrap, false),
        AccountMeta::new_readonly(WSOL, false),
    ]
}
fn with(mut a: Vec<AccountMeta>, pairs: &[(Pubkey, Pubkey)]) -> Vec<AccountMeta> {
    for (b, d) in pairs {
        a.push(AccountMeta::new_readonly(*b, false));
        a.push(AccountMeta::new(*d, false));
    }
    a
}

/// Holder: a wallet (system account), its base token account (under the pool's base program),
/// and optionally a quote token account.
struct Holder {
    wallet: Pubkey,
    base_acct: Pubkey,
}
fn wallets(n: usize) -> Vec<Pubkey> {
    let mut w: Vec<Pubkey> = (0..n).map(|_| Pubkey::new_unique()).collect();
    w.sort();
    w
}
fn add_holder(env: &mut Env, p: &Pool, wallet: Pubkey, base_amount: u64, extended: bool, wallet_lamports: u64) -> Holder {
    let base_acct = Pubkey::new_unique();
    let data = if extended { token_t22_ext(&p.base, &wallet, base_amount) } else { token(&p.base, &wallet, base_amount) };
    env.store(&base_acct, owned(p.bprog, data));
    if wallet_lamports > 0 {
        env.store(&wallet, Account { lamports: wallet_lamports, data: vec![], owner: system_program::ID, executable: false, rent_epoch: 0 });
    }
    Holder { wallet, base_acct }
}

/// open the cursor (fresh cursors are closed for the round they are created in), then move to round 2.
async fn open_cursor(env: &mut Env, disc: [u8; 8], accts: Vec<AccountMeta>, p: &Pool) {
    let r = env.send(disc, &1u64.to_le_bytes(), accts).await;
    assert!(r.is_ok(), "open cursor: {:?}", r.err());
    env.set_u64(&p.prec, PR_ROUND, 2).await;
}

const ONE_SOL: u64 = 1_000_000_000;

// ---------------------------------------------------------------------------------------------
// 1. Token-2022 base + SOL quote: holders paid plain SOL by v4; token path, carry-forward,
//    duplicate protection across v2/v3/v4, burn pot untouched, release_burn_pot unchanged.
// ---------------------------------------------------------------------------------------------
#[tokio::test]
async fn t22_base_sol_quote_paid_plain_sol() {
    let (mut env, g) = start().await;
    let mut ev: Vec<String> = Vec::new();
    let pot = 8 * ONE_SOL;
    let burn = 2 * ONE_SOL;
    let q = add_quote(&mut env, &g, WSOL, TOKEN, pot + burn, 1).await;
    let p = add_pool(&mut env, &g, &q, T22, 1, pot, burn).await;
    assert_eq!(env.acct(&q.qs).await.unwrap().data[40..48], 1u64.to_le_bytes(), "register_pool counted the T22-base pool");
    let payer = env.payer();
    open_cursor(&mut env, IX_PAY4, v4_accounts(payer, &g, &q, &p), &p).await;
    assert_eq!(env.u64_at(&p.cursor, PC_ROUND).await, 1);

    // eligible 10_000 base: shares are pot * amount / 10_000
    let w = wallets(5);
    let h0 = add_holder(&mut env, &p, w[0], 1_000, false, ONE_SOL); // plain SOL, 165-byte T22 account
    let h1 = add_holder(&mut env, &p, w[1], 3_000, true, ONE_SOL); // plain SOL, T22 account with extension
    let h2 = add_holder(&mut env, &p, w[2], 1, false, 0); // share 800_000 lamports < rent-exempt for an empty wallet: carry forward
    let h3 = add_holder(&mut env, &p, w[3], 2_000, false, ONE_SOL); // paid as wSOL into its token account (token path)
    let h3_wsol = ata(&w[3], &WSOL, &TOKEN);
    env.store(&h3_wsol, wsol_account(&w[3], 0));
    let h4 = add_holder(&mut env, &p, w[4], 0, true, ONE_SOL); // zero balance: skipped
    let eligible = 10_000u64;
    let share = |a: u64| ((pot as u128 * a as u128) / eligible as u128) as u64;
    let rent0 = Rent::default().minimum_balance(0);
    assert!(share(1) < rent0);

    let sol_vault = pda(&[b"sol_vault"]);
    let sv_before = env.lamports(&sol_vault).await;
    let payer_before = env.lamports(&payer).await;
    let vault_tok_before = env.tok(&q.vault).await;
    let vault_lam_before = env.lamports(&q.vault).await;
    let w0_before = env.lamports(&w[0]).await;
    let w1_before = env.lamports(&w[1]).await;

    let pairs = vec![(h0.base_acct, h0.wallet), (h1.base_acct, h1.wallet), (h2.base_acct, h2.wallet), (h3.base_acct, h3_wsol), (h4.base_acct, h4.wallet)];
    let r = env.send(IX_PAY4, &eligible.to_le_bytes(), with(v4_accounts(payer, &g, &q, &p), &pairs)).await;
    let sent = r.unwrap_or_else(|e| panic!("v4 pay: {e}"));
    let expected_sol = share(1_000) + share(3_000);
    let expected_tok = share(2_000);
    let total = expected_sol + expected_tok;
    assert_eq!(env.lamports(&w[0]).await - w0_before, share(1_000));
    assert_eq!(env.lamports(&w[1]).await - w1_before, share(3_000));
    assert_eq!(env.lamports(&w[2]).await, 0, "carried forward, nothing sent");
    assert_eq!(env.tok(&h3_wsol).await, expected_tok);
    assert_eq!(env.u64_at(&p.prec, PR_HOLDER).await, pot - total);
    assert_eq!(env.u64_at(&p.prec, PR_BURN).await, burn, "v4 never touches the burn pot");
    assert_eq!(env.tok(&q.vault).await, vault_tok_before - total);
    assert_eq!(env.lamports(&q.vault).await, vault_lam_before - total);
    assert_eq!(env.lamports(&sol_vault).await, sv_before, "sol_vault ends where it started");
    assert!(env.acct(&p.unwrap).await.is_none(), "temp unwrap account closed");
    let payer_cost = payer_before - env.lamports(&payer).await;
    assert!(payer_cost <= 10_000, "crank only pays the tx fee (temp rent refunded), cost={payer_cost}");
    let paid_base = 1_000 + 3_000 + 2_000;
    assert_eq!(ret_u64(&sent), paid_base);
    assert_eq!(env.u64_at(&p.cursor, PC_PAID_BASE).await, paid_base);
    assert_eq!(env.u64_at(&p.cursor, PC_PAID_AMOUNT).await, total);
    assert_eq!(env.acct(&p.cursor).await.unwrap().data[PC_LAST..PC_LAST + 32], w[4].to_bytes());
    ev.push(format!(
        "T22 base + SOL: v4 paid plain SOL {} + {} lamports, wSOL token path {}, carry-forward {} (below rent), zero-balance skipped; holder_pot {} -> {}; burn_pot {} unchanged; sol_vault unchanged; unwrap closed; crank cost {} lamports (fee only); cu={}",
        share(1_000), share(3_000), expected_tok, share(1), pot, pot - total, burn, payer_cost, sent.cu
    ));

    // duplicate protection: the same page again (v4), then v3 and v2 for the same wallets, same round: nothing moves.
    let again = env.send(IX_PAY4, &eligible.to_le_bytes(), with(v4_accounts(payer, &g, &q, &p), &pairs)).await.expect("repeat v4");
    assert_eq!(ret_u64(&again), 0);
    let v3 = env.send(IX_PAY3, &eligible.to_le_bytes(), with(v3_accounts(payer, &g, &q, &p), &[(h3.base_acct, h3_wsol)])).await.expect("v3 same round");
    assert_eq!(ret_u64(&v3), 0);
    assert_eq!(env.u64_at(&p.prec, PR_HOLDER).await, pot - total);
    assert_eq!(env.lamports(&w[0]).await - w0_before, share(1_000));
    // v2 cannot even read a Token-2022 holder account (why v4 exists)
    let v2 = env.send(IX_PAY2, &eligible.to_le_bytes(), with(v2_accounts(payer, &g, &q, &p), &[(h0.base_acct, h0.wallet)])).await;
    assert_eq!(code(&v2), Some(E_ANCHOR_WRONG_OWNER), "{:?}", v2.err());
    ev.push("same round: repeat v4 page paid_base=0, v3 page paid_base=0, v2 rejects Token-2022 holder account (3007)".into());

    // next round: the carried-forward wallet now has SOL and is paid; wallet order restarts.
    env.set_u64(&p.prec, PR_ROUND, 3).await;
    env.store(&w[2], Account { lamports: ONE_SOL, data: vec![], owner: system_program::ID, executable: false, rent_epoch: 0 });
    env.store(&h2.base_acct, owned(T22, token(&p.base, &w[2], 1_000)));
    let pot2 = env.u64_at(&p.prec, PR_HOLDER).await;
    let r3 = env.send(IX_PAY4, &1_000u64.to_le_bytes(), with(v4_accounts(payer, &g, &q, &p), &[(h2.base_acct, h2.wallet)])).await.expect("round 3");
    assert_eq!(ret_u64(&r3), 1_000);
    assert_eq!(env.lamports(&w[2]).await, ONE_SOL + pot2);
    assert_eq!(env.u64_at(&p.prec, PR_HOLDER).await, 0);
    ev.push(format!("round 3: carried-forward wallet paid {pot2} lamports, holder_pot -> 0"));

    // buy-and-burn path unchanged: release_burn_pot moves exactly burn_pot to the admin's wSOL account.
    let operator = ata(&payer, &WSOL, &TOKEN);
    env.store(&operator, wsol_account(&payer, 0));
    let rel = env
        .send(IX_RELEASE, &[], vec![
            AccountMeta::new_readonly(g, false),
            AccountMeta::new(p.prec, false),
            AccountMeta::new(q.vault, false),
            AccountMeta::new(operator, false),
            AccountMeta::new_readonly(TOKEN, false),
        ])
        .await;
    assert!(rel.is_ok(), "{:?}", rel.err());
    assert_eq!(env.tok(&operator).await, burn);
    assert_eq!(env.u64_at(&p.prec, PR_BURN).await, 0);
    assert_eq!(env.tok(&q.vault).await, 0, "vault fully accounted: holders + burn");
    ev.push(format!("release_burn_pot: operator got burn_pot {burn} (the 0.20% slice for the $FUSED buy-and-burn); vault left 0"));
    write_evidence("t22_sol", &ev);
}

// ---------------------------------------------------------------------------------------------
// 2. Token-2022 base + non-SOL quotes (classic USDC-like and Token-2022 TTWO-like): v4 == v3.
// ---------------------------------------------------------------------------------------------
#[tokio::test]
async fn t22_base_non_sol_quotes() {
    let (mut env, g) = start().await;
    let mut ev: Vec<String> = Vec::new();
    let payer = env.payer();
    for (label, qprog) in [("classic USDC-like quote", TOKEN), ("Token-2022 TTWO-like quote", T22)] {
        let qmint = Pubkey::new_unique();
        env.store(&qmint, owned(qprog, mint_data(6, Some(&payer), 0)));
        let pot = 900_000u64;
        let q = add_quote(&mut env, &g, qmint, qprog, pot, 1).await;
        // two identical pools: one paid by v3, one by v4
        let mut results = Vec::new();
        for disc in [IX_PAY3, IX_PAY4] {
            let p = add_pool(&mut env, &g, &q, T22, 1, pot / 2, 0).await;
            let accts = |e: &Env| if disc == IX_PAY3 { v3_accounts(e.payer(), &g, &q, &p) } else { v4_accounts(e.payer(), &g, &q, &p) };
            let a = accts(&env);
            open_cursor(&mut env, disc, a, &p).await;
            let w = wallets(4);
            let hs: Vec<Holder> = (0..4).map(|i| add_holder(&mut env, &p, w[i], 1_000 * (i as u64 + 1), i % 2 == 1, ONE_SOL)).collect();
            let mut dests = Vec::new();
            for (i, h) in hs.iter().enumerate() {
                let d = ata(&h.wallet, &qmint, &qprog);
                let data = match (i, qprog == T22) {
                    (2, _) => frozen(token(&qmint, &h.wallet, 0)), // frozen: carried forward
                    (3, true) => token_t22_memo(&qmint, &h.wallet, 0), // memo-required: carried forward
                    _ => token(&qmint, &h.wallet, 0),
                };
                env.store(&d, owned(qprog, data));
                dests.push(d);
            }
            let pairs: Vec<(Pubkey, Pubkey)> = hs.iter().zip(dests.iter()).map(|(h, d)| (h.base_acct, *d)).collect();
            let a = with(accts(&env), &pairs);
            let s = env.send(disc, &10_000u64.to_le_bytes(), a).await.unwrap_or_else(|e| panic!("{label}: {e}"));
            let got: Vec<u64> = {
                let mut v = Vec::new();
                for d in &dests {
                    v.push(env.tok(d).await);
                }
                v
            };
            let left = env.u64_at(&p.prec, PR_HOLDER).await;
            results.push((got, left, ret_u64(&s)));
            // a wallet (system account) as destination is refused on a non-SOL pool (v4 only has that path)
            if disc == IX_PAY4 {
                env.set_u64(&p.prec, PR_ROUND, 5).await;
                let r = env.send(IX_PAY4, &10_000u64.to_le_bytes(), with(accts(&env), &[(hs[0].base_acct, hs[0].wallet)])).await;
                assert_eq!(code(&r), Some(E_BAD_DEST), "{label}: {:?}", r.err());
            }
        }
        assert_eq!(results[0], results[1], "{label}: v3 and v4 pay identically");
        let (got, left, pb) = &results[0];
        assert_eq!(got[0], 450_000 * 1_000 / 10_000);
        assert_eq!(got[1], 450_000 * 2_000 / 10_000);
        assert_eq!(got[2], 0);
        ev.push(format!("T22 base + {label}: v3 == v4 payouts {got:?}, holder_pot left {left}, paid_base {pb}; frozen/memo dests carried forward; wallet-as-dest -> BadDest (6027)"));
    }
    write_evidence("t22_non_sol", &ev);
}

// ---------------------------------------------------------------------------------------------
// 3. Classic SPL regression: v2 (plain SOL + token) unchanged; v4 gives the same result for a
//    classic base on SOL; v3 unchanged for a classic USDC-like pool.
// ---------------------------------------------------------------------------------------------
#[tokio::test]
async fn classic_spl_regression_v2_v3_v4() {
    let (mut env, g) = start().await;
    let mut ev: Vec<String> = Vec::new();
    let payer = env.payer();
    let pot = 5 * ONE_SOL;
    let q = add_quote(&mut env, &g, WSOL, TOKEN, 2 * pot, 0).await;
    let mut outcomes = Vec::new();
    for disc in [IX_PAY2, IX_PAY4] {
        let p = add_pool(&mut env, &g, &q, TOKEN, 1, pot, 0).await;
        let accts = |e: &Env| if disc == IX_PAY2 { v2_accounts(e.payer(), &g, &q, &p) } else { v4_accounts(e.payer(), &g, &q, &p) };
        let a = accts(&env);
        open_cursor(&mut env, disc, a, &p).await;
        let w = wallets(3);
        let h0 = add_holder(&mut env, &p, w[0], 4_000, false, ONE_SOL);
        let h1 = add_holder(&mut env, &p, w[1], 1, false, 0);
        let h2 = add_holder(&mut env, &p, w[2], 5_000, false, ONE_SOL);
        let h2_wsol = ata(&w[2], &WSOL, &TOKEN);
        env.store(&h2_wsol, wsol_account(&w[2], 0));
        let before = env.lamports(&w[0]).await;
        let a = with(accts(&env), &[(h0.base_acct, h0.wallet), (h1.base_acct, h1.wallet), (h2.base_acct, h2_wsol)]);
        let s = env.send(disc, &10_000u64.to_le_bytes(), a).await.unwrap_or_else(|e| panic!("{e}"));
        outcomes.push((
            env.lamports(&w[0]).await - before,
            env.lamports(&w[1]).await,
            env.tok(&h2_wsol).await,
            env.u64_at(&p.prec, PR_HOLDER).await,
            ret_u64(&s),
            env.acct(&p.unwrap).await.is_none(),
        ));
    }
    assert_eq!(outcomes[0], outcomes[1], "v2 and v4 identical on a classic SOL pool");
    assert_eq!(outcomes[0].0, pot * 4_000 / 10_000);
    ev.push(format!("classic base + SOL: v2 == v4 (sol paid, carry-forward wallet, wSOL token paid, holder_pot left, paid_base, unwrap closed) = {:?}", outcomes[0]));

    // v3 on a classic USDC-like pool (same as the repo's existing release_and_pay check)
    let usdc = Pubkey::new_unique();
    env.store(&usdc, owned(TOKEN, mint_data(6, Some(&payer), 0)));
    let qu = add_quote(&mut env, &g, usdc, TOKEN, 9_000, 0).await;
    let p = add_pool(&mut env, &g, &qu, TOKEN, 1, 9_000, 0).await;
    open_cursor(&mut env, IX_PAY3, v3_accounts(payer, &g, &qu, &p), &p).await;
    let w = Pubkey::new_unique();
    let h = add_holder(&mut env, &p, w, 1_000, false, ONE_SOL);
    let d = ata(&w, &usdc, &TOKEN);
    env.store(&d, owned(TOKEN, token(&usdc, &w, 0)));
    env.send(IX_PAY3, &1_000u64.to_le_bytes(), with(v3_accounts(payer, &g, &qu, &p), &[(h.base_acct, d)])).await.expect("v3 classic");
    assert_eq!(env.tok(&d).await, 9_000);
    // v2 still refuses a system-owned destination on a non-SOL pool, and v3 still refuses it
    env.set_u64(&p.prec, PR_ROUND, 3).await;
    env.set_u64(&p.prec, PR_HOLDER, 9_000).await;
    env.store(&qu.vault, owned(TOKEN, token(&usdc, &g, 9_000)));
    let r2 = env.send(IX_PAY2, &1_000u64.to_le_bytes(), with(v2_accounts(payer, &g, &qu, &p), &[(h.base_acct, w)])).await;
    assert_eq!(code(&r2), Some(E_BAD_DEST), "{:?}", r2.err());
    let r3 = env.send(IX_PAY3, &1_000u64.to_le_bytes(), with(v3_accounts(payer, &g, &qu, &p), &[(h.base_acct, w)])).await;
    assert_eq!(code(&r3), Some(E_BAD_DEST), "{:?}", r3.err());
    ev.push("classic base + USDC-like: v3 paid 9000; system-owned dest refused by v2 and v3 (6027) as before".into());
    write_evidence("classic", &ev);
}

// ---------------------------------------------------------------------------------------------
// 4. $FUSED pair: a Token-2022 base quoted in $FUSED is paid in $FUSED by v4, and
//    crank_burn_direct still burns exactly the burn pot of $FUSED (supply drops).
// ---------------------------------------------------------------------------------------------
#[tokio::test]
async fn fused_pair_pay_and_burn() {
    let (mut env, g) = start().await;
    let mut ev: Vec<String> = Vec::new();
    let payer = env.payer();
    let supply = 1_000_000_000_000u64;
    env.store(&FUSED, owned(TOKEN, mint_data(6, None, supply)));
    env.send(IX_SET_BURN_MINT, FUSED.as_ref(), vec![AccountMeta::new_readonly(payer, true), AccountMeta::new(g, false)])
        .await
        .expect("set_burn_mint");
    let holder_pot = 90_000u64;
    let burn_pot = 20_000u64;
    let q = add_quote(&mut env, &g, FUSED, TOKEN, holder_pot + burn_pot, 1).await;
    let p = add_pool(&mut env, &g, &q, T22, 1, holder_pot, burn_pot).await;
    open_cursor(&mut env, IX_PAY4, v4_accounts(payer, &g, &q, &p), &p).await;
    let w = wallets(2);
    let h0 = add_holder(&mut env, &p, w[0], 600, true, ONE_SOL);
    let h1 = add_holder(&mut env, &p, w[1], 400, false, ONE_SOL);
    let d0 = ata(&w[0], &FUSED, &TOKEN);
    let d1 = ata(&w[1], &FUSED, &TOKEN);
    env.store(&d0, owned(TOKEN, token(&FUSED, &w[0], 0)));
    env.store(&d1, owned(TOKEN, token(&FUSED, &w[1], 0)));
    env.send(IX_PAY4, &1_000u64.to_le_bytes(), with(v4_accounts(payer, &g, &q, &p), &[(h0.base_acct, d0), (h1.base_acct, d1)]))
        .await
        .expect("v4 $FUSED pay");
    assert_eq!(env.tok(&d0).await, 54_000);
    assert_eq!(env.tok(&d1).await, 36_000);
    assert_eq!(env.u64_at(&p.prec, PR_BURN).await, burn_pot);
    let burned = env
        .send(IX_BURN_DIRECT, &[], vec![
            AccountMeta::new_readonly(g, false),
            AccountMeta::new(p.prec, false),
            AccountMeta::new(FUSED, false),
            AccountMeta::new(q.vault, false),
            AccountMeta::new_readonly(TOKEN, false),
        ])
        .await;
    assert!(burned.is_ok(), "{:?}", burned.err());
    let new_supply = u64::from_le_bytes(env.acct(&FUSED).await.unwrap().data[36..44].try_into().unwrap());
    assert_eq!(new_supply, supply - burn_pot);
    assert_eq!(env.u64_at(&p.prec, PR_BURN).await, 0);
    assert_eq!(env.tok(&q.vault).await, 0);
    ev.push(format!("$FUSED (7SLA53…pqhr) pair, T22 base: v4 paid holders 54000/36000 $FUSED; crank_burn_direct burned burn_pot {burn_pot}; supply {supply} -> {new_supply}"));

    // a non-$FUSED pool cannot burn directly (still must swap to $FUSED first)
    let other = Pubkey::new_unique();
    env.store(&other, owned(TOKEN, mint_data(6, Some(&payer), 0)));
    let qo = add_quote(&mut env, &g, other, TOKEN, 1_000, 1).await;
    let po = add_pool(&mut env, &g, &qo, T22, 1, 0, 1_000).await;
    let r = env
        .send(IX_BURN_DIRECT, &[], vec![
            AccountMeta::new_readonly(g, false),
            AccountMeta::new(po.prec, false),
            AccountMeta::new(FUSED, false),
            AccountMeta::new(qo.vault, false),
            AccountMeta::new_readonly(TOKEN, false),
        ])
        .await;
    assert!(r.is_err());
    ev.push("non-$FUSED pool: crank_burn_direct refused (burn target stays $FUSED only)".into());
    write_evidence("fused", &ev);
}

// ---------------------------------------------------------------------------------------------
// 5. Cursor, duplicates, malformed holders and guard rails for v4.
// ---------------------------------------------------------------------------------------------
#[tokio::test]
async fn v4_guards_cursor_and_malformed() {
    let (mut env, g) = start().await;
    let mut ev: Vec<String> = Vec::new();
    let payer = env.payer();
    let pot = 10 * ONE_SOL;
    let q = add_quote(&mut env, &g, WSOL, TOKEN, pot, 1).await;
    let p = add_pool(&mut env, &g, &q, T22, 1, pot, 0).await;
    // round 0 pool refuses a non-empty page; a fresh cursor is closed for its first round
    let p0 = add_pool(&mut env, &g, &q, T22, 0, pot, 0).await;
    let h = add_holder(&mut env, &p0, Pubkey::new_unique(), 10, false, ONE_SOL);
    let r = env.send(IX_PAY4, &10u64.to_le_bytes(), with(v4_accounts(payer, &g, &q, &p0), &[(h.base_acct, h.wallet)])).await;
    assert_eq!(code(&r), Some(E_BAD_ROUND), "{:?}", r.err());
    let w = wallets(8);
    let hs: Vec<Holder> = (0..8).map(|i| add_holder(&mut env, &p, w[i], 1_000, i % 2 == 0, ONE_SOL)).collect();
    let fresh = env.send(IX_PAY4, &8_000u64.to_le_bytes(), with(v4_accounts(payer, &g, &q, &p), &[(hs[0].base_acct, hs[0].wallet)])).await.expect("fresh");
    assert_eq!(ret_u64(&fresh), 0, "fresh cursor is closed for the round it is created in");
    env.set_u64(&p.prec, PR_ROUND, 2).await;
    let acc = |env: &Env| v4_accounts(env.payer(), &g, &q, &p);
    let e = |pairs: &[(Pubkey, Pubkey)], env: &Env| with(acc(env), pairs);

    let check = |name: &str, r: Result<Sent, String>, want: u32, ev: &mut Vec<String>| {
        assert_eq!(code(&r), Some(want), "{name}: {:?}", r.as_ref().err());
        ev.push(format!("{name} -> {want}"));
    };
    // odd number of remaining accounts
    let mut odd = e(&[(hs[0].base_acct, hs[0].wallet)], &env);
    odd.pop();
    check("odd remaining accounts", env.send(IX_PAY4, &8_000u64.to_le_bytes(), odd).await, E_BAD_PAGE, &mut ev);
    // base account of another mint
    let wrong_mint = Pubkey::new_unique();
    env.store(&wrong_mint, owned(T22, token(&Pubkey::new_unique(), &w[0], 1_000)));
    check("holder base of another mint", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(wrong_mint, w[0])], &env)).await, E_BAD_MINT, &mut ev);
    // classic account presented for a Token-2022 base
    let classic = Pubkey::new_unique();
    env.store(&classic, owned(TOKEN, token(&p.base, &w[0], 1_000)));
    check("classic account for a Token-2022 base", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(classic, w[0])], &env)).await, E_WRONG_TOKEN_PROGRAM, &mut ev);
    // malformed Token-2022 accounts: long without account-type byte, short, uninitialized
    let mut bad = token(&p.base, &w[0], 1_000);
    bad.extend_from_slice(&[9, 0, 0, 0, 0]);
    let m1 = Pubkey::new_unique();
    env.store(&m1, owned(T22, bad));
    check("T22 account, byte 165 != 2", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(m1, w[0])], &env)).await, E_BAD_TOKEN_ACCOUNT, &mut ev);
    let m2 = Pubkey::new_unique();
    env.store(&m2, owned(T22, token(&p.base, &w[0], 1_000)[..100].to_vec()));
    check("short account", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(m2, w[0])], &env)).await, E_BAD_TOKEN_ACCOUNT, &mut ev);
    let mut un = token(&p.base, &w[0], 1_000);
    un[108] = 0;
    let m3 = Pubkey::new_unique();
    env.store(&m3, owned(T22, un));
    check("uninitialized account", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(m3, w[0])], &env)).await, E_BAD_TOKEN_ACCOUNT, &mut ev);
    let mut trunc = token(&p.base, &w[0], 1_000);
    trunc.push(2);
    trunc.extend_from_slice(&[7, 0, 50, 0]); // TLV length runs past the end
    let m4 = Pubkey::new_unique();
    env.store(&m4, owned(T22, trunc));
    check("TLV overrun", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(m4, w[0])], &env)).await, E_BAD_TOKEN_ACCOUNT, &mut ev);
    // plain-SOL dest that is not the holder's wallet
    check("SOL dest != holder wallet", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(hs[0].base_acct, w[1])], &env)).await, E_OWNER_MISMATCH, &mut ev);
    // plain-SOL dest not writable
    let mut ro = e(&[(hs[0].base_acct, hs[0].wallet)], &env);
    let last = ro.len() - 1;
    ro[last] = AccountMeta::new_readonly(hs[0].wallet, false);
    check("SOL dest read-only", env.send(IX_PAY4, &8_000u64.to_le_bytes(), ro).await, E_BAD_DEST, &mut ev);
    // system-owned dest with data
    let wd = w[0];
    env.store(&wd, Account { lamports: ONE_SOL, data: vec![1, 2, 3], owner: system_program::ID, executable: false, rent_epoch: 0 });
    check("SOL dest with data", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(hs[0].base_acct, wd)], &env)).await, E_BAD_DEST, &mut ev);
    env.store(&wd, Account { lamports: ONE_SOL, data: vec![], owner: system_program::ID, executable: false, rent_epoch: 0 });
    // token dest owned by someone else
    let other_wsol = Pubkey::new_unique();
    env.store(&other_wsol, wsol_account(&w[5], 0));
    check("token dest of another wallet", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(hs[0].base_acct, other_wsol)], &env)).await, E_OWNER_MISMATCH, &mut ev);
    // token dest of another mint
    let usdc = Pubkey::new_unique();
    env.store(&usdc, owned(TOKEN, mint_data(6, Some(&payer), 0)));
    let wrong_q = Pubkey::new_unique();
    env.store(&wrong_q, owned(TOKEN, token(&usdc, &w[0], 0)));
    check("token dest of another mint", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(hs[0].base_acct, wrong_q)], &env)).await, E_BAD_MINT, &mut ev);
    // wrong quote token program / wrong vault
    let mut wp = acc(&env);
    wp[5] = AccountMeta::new_readonly(T22, false);
    check("quote_token_program != quote mint owner", env.send(IX_PAY4, &8_000u64.to_le_bytes(), with(wp, &[(hs[0].base_acct, hs[0].wallet)])).await, E_WRONG_TOKEN_PROGRAM, &mut ev);
    let fake_vault = Pubkey::new_unique();
    env.store(&fake_vault, wsol_account(&g, pot));
    let mut wv = acc(&env);
    wv[3] = AccountMeta::new(fake_vault, false);
    check("not Global's vault", env.send(IX_PAY4, &8_000u64.to_le_bytes(), with(wv, &[(hs[0].base_acct, hs[0].wallet)])).await, E_BAD_VAULT, &mut ev);
    // eligible_total 0
    check("eligible_total 0", env.send(IX_PAY4, &0u64.to_le_bytes(), e(&[(hs[0].base_acct, hs[0].wallet)], &env)).await, E_BAD_TOTAL, &mut ev);
    // not the admin / crank
    let stranger = Keypair::new();
    env.store(&stranger.pubkey(), Account { lamports: ONE_SOL, data: vec![], owner: system_program::ID, executable: false, rent_epoch: 0 });
    let mut sa = v4_accounts(stranger.pubkey(), &g, &q, &p);
    sa.push(AccountMeta::new_readonly(hs[0].base_acct, false));
    sa.push(AccountMeta::new(hs[0].wallet, false));
    check("stranger signer", env.send_with(IX_PAY4, &8_000u64.to_le_bytes(), sa, &[&stranger]).await, E_NOT_CRANK, &mut ev);
    // vault short (vault holds less than this page)
    let vsave = env.acct(&q.vault).await.unwrap();
    env.store(&q.vault, wsol_account(&g, 1));
    check("vault short", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(hs[0].base_acct, hs[0].wallet)], &env)).await, E_VAULT_SHORT, &mut ev);
    env.store(&q.vault, vsave);
    // paused
    env.send(IX_SET_PAUSED, &[1], vec![AccountMeta::new_readonly(payer, true), AccountMeta::new(g, false)]).await.expect("pause");
    check("paused", env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(hs[0].base_acct, hs[0].wallet)], &env)).await, E_PAUSED, &mut ev);
    env.send(IX_SET_PAUSED, &[0], vec![AccountMeta::new_readonly(payer, true), AccountMeta::new(g, false)]).await.expect("unpause");
    // nothing moved during all the refusals
    assert_eq!(env.u64_at(&p.prec, PR_HOLDER).await, pot);

    // out-of-order and duplicate wallets are skipped;
    // a pre-funded unwrap PDA (donation) does not block the page and the donation goes to the crank.
    env.store(&p.unwrap, Account { lamports: 12_345, data: vec![], owner: system_program::ID, executable: false, rent_epoch: 0 });
    let w1_before = env.lamports(&w[1]).await;
    let w2_before = env.lamports(&w[2]).await;
    let w3_before = env.lamports(&w[3]).await;
    let payer_before = env.lamports(&payer).await;
    // order: w1, w1 (dup), w0 (below cursor), w2, w3, then the DBC authority wherever it sorts
    let pairs = vec![
        (hs[1].base_acct, hs[1].wallet),
        (hs[1].base_acct, hs[1].wallet),
        (hs[0].base_acct, hs[0].wallet),
        (hs[2].base_acct, hs[2].wallet),
        (hs[3].base_acct, hs[3].wallet),
    ];
    let s = env.send(IX_PAY4, &8_000u64.to_le_bytes(), e(&pairs, &env)).await.unwrap_or_else(|x| panic!("{x}"));
    let each = pot * 1_000 / 8_000;
    assert_eq!(env.lamports(&w[1]).await - w1_before, each);
    assert_eq!(env.lamports(&w[2]).await - w2_before, each);
    assert_eq!(env.lamports(&w[3]).await - w3_before, each);
    assert_eq!(ret_u64(&s), 3_000);
    assert_eq!(env.u64_at(&p.prec, PR_HOLDER).await, pot - 3 * each);
    assert!(env.acct(&p.unwrap).await.is_none());
    let net = env.lamports(&payer).await as i128 - payer_before as i128;
    assert!(net > -10_000 && net <= 12_345, "crank net {net}");
    ev.push(format!("page [w1, w1 dup, w0 out of order, w2, w3]: paid w1/w2/w3 {each} each, dup + out-of-order skipped; pre-funded unwrap PDA handled; crank net {net} lamports"));
    // next page continues from the cursor: w0..w3 are now below it, w4.. still payable
    let s2 = env
        .send(IX_PAY4, &8_000u64.to_le_bytes(), e(&[(hs[3].base_acct, hs[3].wallet), (hs[4].base_acct, hs[4].wallet)], &env))
        .await
        .expect("page 2");
    assert_eq!(ret_u64(&s2), 1_000);
    ev.push("page 2: w3 (already paid) skipped, w4 paid; cursor carries across pages".into());

    // excluded holders (DBC pool authority, Global) are skipped, others in the same page are paid
    let px = add_pool(&mut env, &g, &q, T22, 1, ONE_SOL, 0).await;
    open_cursor(&mut env, IX_PAY4, v4_accounts(payer, &g, &q, &px), &px).await;
    let mut ws = vec![DBC_POOL_AUTHORITY, g, Pubkey::new_unique()];
    ws.sort();
    let mut xp = Vec::new();
    let mut normal = Pubkey::default();
    for wk in &ws {
        let fund = if *wk == DBC_POOL_AUTHORITY || *wk == g { 0 } else { ONE_SOL };
        let hh = add_holder(&mut env, &px, *wk, 1_000, true, fund);
        if fund > 0 {
            normal = *wk;
        }
        xp.push((hh.base_acct, *wk));
    }
    let nb = env.lamports(&normal).await;
    let sx = env.send(IX_PAY4, &3_000u64.to_le_bytes(), with(v4_accounts(payer, &g, &q, &px), &xp)).await.unwrap_or_else(|x| panic!("{x}"));
    assert_eq!(ret_u64(&sx), 1_000);
    assert_eq!(env.lamports(&normal).await - nb, ONE_SOL / 3);
    ev.push("DBC pool authority + Global as holders: skipped; the normal holder in the same page paid".into());

    // empty pot
    let pe = add_pool(&mut env, &g, &q, T22, 1, 0, 0).await;
    open_cursor(&mut env, IX_PAY4, v4_accounts(payer, &g, &q, &pe), &pe).await;
    let he = add_holder(&mut env, &pe, Pubkey::new_unique(), 10, false, ONE_SOL);
    let r = env.send(IX_PAY4, &10u64.to_le_bytes(), with(v4_accounts(payer, &g, &q, &pe), &[(he.base_acct, he.wallet)])).await;
    assert_eq!(code(&r), Some(E_EMPTY_POT), "{:?}", r.err());
    ev.push(format!("empty holder pot -> {E_EMPTY_POT}"));
    write_evidence("guards", &ev);
}

fn write_evidence(name: &str, ev: &[String]) {
    let dir = std::env::var("T22_EVIDENCE_DIR").unwrap_or_else(|_| "target/t22-evidence".into());
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(format!("{dir}/{name}.txt"), ev.join("\n") + "\n").unwrap();
    println!("[{name}]\n{}", ev.join("\n"));
}
