//! Offline bank tests for the required QuoteStat on register_pool.
//! Program under test: live baseline 80060ab plus that one change.
use solana_account::{Account, AccountSharedData};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_signer::Signer;
use solana_system_interface::program as system_program;
use solana_transaction::Transaction;

const PROGRAM: Pubkey = solana_pubkey::pubkey!("AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9");
const DBC: Pubkey = solana_pubkey::pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
const BPF: Pubkey = solana_pubkey::pubkey!("BPFLoader2111111111111111111111111111111111");
const IX_INIT: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const IX_REG_CFG: [u8; 8] = [32, 247, 82, 131, 35, 183, 7, 57];
const IX_REG_POOL: [u8; 8] = [85, 229, 114, 47, 75, 145, 166, 100];
const IX_SYNC: [u8; 8] = [130, 168, 89, 25, 125, 164, 73, 104];
const IX_RELEASE: [u8; 8] = [224, 64, 74, 119, 186, 164, 250, 47];
const IX_PAY3: [u8; 8] = [26, 92, 113, 244, 121, 99, 140, 121];
const QSTAT_TAG: [u8; 8] = [196, 19, 91, 20, 68, 151, 114, 86];

struct Env {
    ctx: ProgramTestContext,
    slot: u64,
}
impl Env {
    async fn send_with(
        &mut self,
        disc: [u8; 8],
        data: &[u8],
        accounts: Vec<AccountMeta>,
        extra: &[&Keypair],
    ) -> Result<u64, String> {
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
                Ok(()) => Ok(r.metadata.map(|m| m.compute_units_consumed).unwrap_or(0)),
                Err(e) => Err(format!("{e:?}")),
            },
            Err(e) => Err(format!("{e:?}")),
        }
    }
    async fn send(&mut self, disc: [u8; 8], data: &[u8], accounts: Vec<AccountMeta>) -> Result<u64, String> {
        self.send_with(disc, data, accounts, &[]).await
    }
    fn store(&mut self, k: &Pubkey, a: Account) {
        self.ctx.set_account(k, &AccountSharedData::from(a));
    }
    async fn acct(&mut self, k: &Pubkey) -> Option<Account> {
        self.ctx.banks_client.get_account(*k).await.unwrap()
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
fn mint_data(dec: u8, auth: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    d[0..4].copy_from_slice(&1u32.to_le_bytes());
    d[4..36].copy_from_slice(auth.as_ref());
    d[44] = dec;
    d[45] = 1;
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
fn amt(a: &Account) -> u64 {
    u64::from_le_bytes(a.data[64..72].try_into().unwrap())
}
fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let prog = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    Pubkey::find_program_address(&[owner.as_ref(), spl_token::ID.as_ref(), mint.as_ref()], &prog).0
}
fn dbc_pool(config: &Pubkey, creator: &Pubkey, base: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 400];
    d[..8].copy_from_slice(&[213, 224, 5, 209, 98, 69, 119, 92]);
    d[72..104].copy_from_slice(config.as_ref());
    d[104..136].copy_from_slice(creator.as_ref());
    d[136..168].copy_from_slice(base.as_ref());
    d
}
fn dbc_cfg(quote: &Pubkey, global: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 256];
    d[..8].copy_from_slice(&[26, 108, 14, 123, 116, 230, 129, 43]);
    d[8..40].copy_from_slice(quote.as_ref());
    d[40..72].copy_from_slice(global.as_ref());
    d
}
fn custom(err: &str) -> Option<u32> {
    let marker = "Custom(";
    err.find(marker).and_then(|i| {
        err[i + marker.len()..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()
    })
}
fn count_of(a: &Account) -> u64 {
    u64::from_le_bytes(a.data[40..48].try_into().unwrap())
}

struct Quote {
    mint: Pubkey,
    base: Pubkey,
    config: Pubkey,
    cfg: Pubkey,
    qs: Pubkey,
}

async fn boot(env: &mut Env) -> Pubkey {
    let g = pda(&[b"global"]);
    env.send(IX_INIT, &[], vec![
        AccountMeta::new(env.payer(), true),
        AccountMeta::new(g, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ])
    .await
    .expect("init");
    g
}

async fn add_quote(env: &mut Env, g: &Pubkey, baseline: u64) -> Quote {
    let mint = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    env.store(&mint, owned(spl_token::ID, mint_data(9, &env.payer())));
    env.store(&base, owned(spl_token::ID, mint_data(6, &env.payer())));
    env.store(&config, owned(DBC, dbc_cfg(&mint, g)));
    let cfg = pda(&[b"config", mint.as_ref()]);
    env.send(IX_REG_CFG, mint.as_ref(), vec![
        AccountMeta::new(env.payer(), true),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(mint, false),
    ])
    .await
    .expect("config");
    let qs = pda(&[b"qstat", mint.as_ref()]);
    let mut data = mint.to_bytes().to_vec();
    data.extend_from_slice(&baseline.to_le_bytes());
    env.send(IX_SYNC, &data, vec![
        AccountMeta::new(env.payer(), true),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new(qs, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ])
    .await
    .expect("sync");
    Quote { mint, base, config, cfg, qs }
}

fn reg_data(pool: &Pubkey, base: &Pubkey, quote: &Pubkey) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(pool.as_ref());
    d.extend_from_slice(base.as_ref());
    d.extend_from_slice(quote.as_ref());
    d
}

/// Website account order: 7 named, quote mint, then optional QuoteStat.
fn website_accounts(payer: Pubkey, q: &Quote, pool: &Pubkey, prec: &Pubkey, g: &Pubkey, qstat: Option<Pubkey>) -> Vec<AccountMeta> {
    let mut a = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(q.cfg, false),
        AccountMeta::new(*prec, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new_readonly(q.config, false),
        AccountMeta::new_readonly(q.mint, false),
    ];
    if let Some(qs) = qstat {
        a.push(AccountMeta::new(qs, false));
    }
    a
}

struct Reg {
    pool: Pubkey,
    prec: Pubkey,
    result: Result<u64, String>,
}

async fn register(env: &mut Env, payer: Pubkey, extra: &[&Keypair], g: &Pubkey, q: &Quote, qstat: Option<Pubkey>) -> Reg {
    let pool = Pubkey::new_unique();
    env.store(&pool, owned(DBC, dbc_pool(&q.config, &payer, &q.base)));
    let prec = pda(&[b"pool", pool.as_ref()]);
    let result = env
        .send_with(IX_REG_POOL, &reg_data(&pool, &q.base, &q.mint), website_accounts(payer, q, &pool, &prec, g, qstat), extra)
        .await;
    Reg { pool, prec, result }
}

fn signed_len(accounts: &[AccountMeta]) -> usize {
    let mut data = IX_REG_POOL.to_vec();
    data.extend(reg_data(&accounts[5].pubkey, &Pubkey::new_unique(), &accounts[7].pubkey));
    let ix = Instruction { program_id: PROGRAM, accounts: accounts.to_vec(), data };
    let msg = Message::new(&[ix], Some(&accounts[0].pubkey));
    let n = msg.header.num_required_signatures as usize;
    1 + n * 64 + bincode::serialize(&msg).unwrap().len()
}

fn code(r: &Result<u64, String>) -> Option<u32> {
    custom(r.as_ref().err().expect("expected failure"))
}

#[tokio::test]
async fn quotestat_register_cases() {
    let so_path = std::env::var("QUOTESTAT_SO").unwrap_or_else(|_| "target/deploy/fused_fee.so".into());
    let so = std::fs::read(&so_path).unwrap_or_else(|e| panic!("read {so_path}: {e}"));
    let mut pt = ProgramTest::default();
    pt.prefer_bpf(false);
    pt.set_compute_max_units(1_400_000);
    pt.add_account(PROGRAM, Account { lamports: rent(so.len()), data: so, owner: BPF, executable: true, rent_epoch: 0 });
    let mut env = Env { ctx: pt.start_with_context().await, slot: 1 };
    let g = boot(&mut env).await;
    let mut evidence: Vec<String> = Vec::new();

    // New quote: admin baseline 0, then each successful new pool adds exactly one.
    let fresh = add_quote(&mut env, &g, 0).await;
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 0);
    let payer = env.payer();
    let a = register(&mut env, payer, &[], &g, &fresh, Some(fresh.qs)).await;
    let b = register(&mut env, payer, &[], &g, &fresh, Some(fresh.qs)).await;
    let c_fresh = count_of(&env.acct(&fresh.qs).await.unwrap());
    evidence.push(format!("new-quote baseline 0, two new pools -> {c_fresh}"));
    assert!(a.result.is_ok() && b.result.is_ok() && c_fresh == 2);
    let stored_quote = Pubkey::try_from(&env.acct(&a.prec).await.unwrap().data[72..104]).unwrap();
    assert_eq!(stored_quote, fresh.mint);

    // Same pool again: init fails, count stays, stored quote stays.
    let again = env
        .send(IX_REG_POOL, &reg_data(&a.pool, &fresh.base, &fresh.mint), website_accounts(payer, &fresh, &a.pool, &a.prec, &g, Some(fresh.qs)))
        .await;
    evidence.push(format!("repeat same pool rejected={} count={}", again.is_err(), count_of(&env.acct(&fresh.qs).await.unwrap())));
    assert!(again.is_err());
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 2);
    assert_eq!(Pubkey::try_from(&env.acct(&a.prec).await.unwrap().data[72..104]).unwrap(), fresh.mint);

    // A second quote cannot be written onto that pool, and the wrong counter does not move.
    let other = add_quote(&mut env, &g, 4).await;
    assert_eq!(count_of(&env.acct(&other.qs).await.unwrap()), 4);
    let mut wrong_quote_accounts = website_accounts(payer, &other, &a.pool, &a.prec, &g, Some(other.qs));
    // The DBC pool and its config stay the original ones. The args name the other quote.
    wrong_quote_accounts[5] = AccountMeta::new_readonly(a.pool, false);
    wrong_quote_accounts[6] = AccountMeta::new_readonly(fresh.config, false);
    let wrong_quote = env
        .send(IX_REG_POOL, &reg_data(&a.pool, &fresh.base, &other.mint), wrong_quote_accounts)
        .await;
    evidence.push(format!("cannot retarget pool quote rejected={}", wrong_quote.is_err()));
    assert!(wrong_quote.is_err());
    assert_eq!(Pubkey::try_from(&env.acct(&a.prec).await.unwrap().data[72..104]).unwrap(), fresh.mint);
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 2);
    assert_eq!(count_of(&env.acct(&other.qs).await.unwrap()), 4);

    // Passing the other quote's counter while registering a new pool for this quote does not count it.
    let stray_pool = Pubkey::new_unique();
    env.store(&stray_pool, owned(DBC, dbc_pool(&fresh.config, &env.payer(), &fresh.base)));
    let stray_prec = pda(&[b"pool", stray_pool.as_ref()]);
    let mut stray = website_accounts(payer, &fresh, &stray_pool, &stray_prec, &g, Some(other.qs));
    stray[8] = AccountMeta::new(other.qs, false);
    let stray_res = env.send(IX_REG_POOL, &reg_data(&stray_pool, &fresh.base, &fresh.mint), stray).await;
    evidence.push(format!("wrong-quote counter err={:?}", code(&stray_res)));
    assert_eq!(code(&stray_res), Some(6002));
    assert!(env.acct(&stray_prec).await.is_none());
    assert_eq!(count_of(&env.acct(&other.qs).await.unwrap()), 4);
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 2);

    // Preexisting-pool baseline is the admin absolute write, then one new pool adds one.
    let existing = add_quote(&mut env, &g, 13).await;
    let ex = register(&mut env, payer, &[], &g, &existing, Some(existing.qs)).await;
    evidence.push(format!("initialized quote baseline 13 plus one register -> {}", count_of(&env.acct(&existing.qs).await.unwrap())));
    assert!(ex.result.is_ok());
    assert_eq!(count_of(&env.acct(&existing.qs).await.unwrap()), 14);
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 2);

    // A QuoteStat that already exists (rewritten in place, not created by this register) increments once.
    let hand = add_quote(&mut env, &g, 0).await;
    let mut hand_acct = env.acct(&hand.qs).await.unwrap();
    let bump = hand_acct.data[48];
    let mut raw = vec![0u8; 49];
    raw[..8].copy_from_slice(&QSTAT_TAG);
    raw[8..40].copy_from_slice(hand.mint.as_ref());
    raw[40..48].copy_from_slice(&7u64.to_le_bytes());
    raw[48] = bump;
    hand_acct.data = raw;
    env.store(&hand.qs, hand_acct);
    let hand_reg = register(&mut env, payer, &[], &g, &hand, Some(hand.qs)).await;
    evidence.push(format!("existing-account count 7 plus one register -> {}", count_of(&env.acct(&hand.qs).await.unwrap())));
    assert!(hand_reg.result.is_ok());
    assert_eq!(count_of(&env.acct(&hand.qs).await.unwrap()), 8);

    // Legacy website call: 8 accounts, no QuoteStat. No pool, no count change.
    let legacy = register(&mut env, payer, &[], &g, &fresh, None).await;
    evidence.push(format!("legacy 8-account err={:?}", code(&legacy.result)));
    assert_eq!(code(&legacy.result), Some(6048));
    assert!(env.acct(&legacy.prec).await.is_none());
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 2);

    // Empty canonical PDA is not treated as the first pool.
    let bare_mint = Pubkey::new_unique();
    let bare_base = Pubkey::new_unique();
    let bare_config = Pubkey::new_unique();
    env.store(&bare_mint, owned(spl_token::ID, mint_data(9, &env.payer())));
    env.store(&bare_base, owned(spl_token::ID, mint_data(6, &env.payer())));
    env.store(&bare_config, owned(DBC, dbc_cfg(&bare_mint, &g)));
    let bare_cfg = pda(&[b"config", bare_mint.as_ref()]);
    env.send(IX_REG_CFG, bare_mint.as_ref(), vec![
        AccountMeta::new(env.payer(), true),
        AccountMeta::new_readonly(g, false),
        AccountMeta::new_readonly(bare_config, false),
        AccountMeta::new(bare_cfg, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(bare_mint, false),
    ])
    .await
    .expect("bare config");
    let bare = Quote { mint: bare_mint, base: bare_base, config: bare_config, cfg: bare_cfg, qs: pda(&[b"qstat", bare_mint.as_ref()]) };
    let empty = register(&mut env, payer, &[], &g, &bare, Some(bare.qs)).await;
    evidence.push(format!("empty QuoteStat err={:?}", code(&empty.result)));
    assert_eq!(code(&empty.result), Some(6049));
    assert!(env.acct(&empty.prec).await.is_none());
    assert!(env.acct(&bare.qs).await.is_none());

    // Read-only canonical account.
    let mut ro_accounts = website_accounts(payer, &fresh, &legacy.pool, &legacy.prec, &g, Some(fresh.qs));
    ro_accounts[8] = AccountMeta::new_readonly(fresh.qs, false);
    let ro = env.send(IX_REG_POOL, &reg_data(&legacy.pool, &fresh.base, &fresh.mint), ro_accounts).await;
    evidence.push(format!("readonly QuoteStat err={:?}", code(&ro)));
    assert_eq!(code(&ro), Some(6000));
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 2);
    assert!(env.acct(&legacy.prec).await.is_none());

    // Wrong owner, wrong tag, wrong mint bytes, short account.
    let saved = env.acct(&fresh.qs).await.unwrap();
    let mut bad = saved.clone();
    bad.owner = system_program::ID;
    env.store(&fresh.qs, bad);
    let wrong_owner = env
        .send(IX_REG_POOL, &reg_data(&legacy.pool, &fresh.base, &fresh.mint), website_accounts(payer, &fresh, &legacy.pool, &legacy.prec, &g, Some(fresh.qs)))
        .await;
    evidence.push(format!("wrong-owner err={:?}", code(&wrong_owner)));
    assert_eq!(code(&wrong_owner), Some(6002));
    env.store(&fresh.qs, saved.clone());

    let mut tagged = saved.clone();
    tagged.data[0] ^= 0xff;
    env.store(&fresh.qs, tagged);
    let malformed = env
        .send(IX_REG_POOL, &reg_data(&legacy.pool, &fresh.base, &fresh.mint), website_accounts(payer, &fresh, &legacy.pool, &legacy.prec, &g, Some(fresh.qs)))
        .await;
    evidence.push(format!("malformed tag err={:?}", code(&malformed)));
    assert_eq!(code(&malformed), Some(6002));
    env.store(&fresh.qs, saved.clone());

    let mut minted = saved.clone();
    minted.data[8..40].copy_from_slice(other.mint.as_ref());
    env.store(&fresh.qs, minted);
    let bad_mint = env
        .send(IX_REG_POOL, &reg_data(&legacy.pool, &fresh.base, &fresh.mint), website_accounts(payer, &fresh, &legacy.pool, &legacy.prec, &g, Some(fresh.qs)))
        .await;
    evidence.push(format!("wrong mint bytes err={:?}", code(&bad_mint)));
    assert_eq!(code(&bad_mint), Some(6002));
    env.store(&fresh.qs, saved.clone());

    let mut short = saved.clone();
    short.data.truncate(40);
    short.lamports = rent(40);
    env.store(&fresh.qs, short);
    let short_res = env
        .send(IX_REG_POOL, &reg_data(&legacy.pool, &fresh.base, &fresh.mint), website_accounts(payer, &fresh, &legacy.pool, &legacy.prec, &g, Some(fresh.qs)))
        .await;
    evidence.push(format!("short account err={:?}", code(&short_res)));
    assert_eq!(code(&short_res), Some(6002));
    env.store(&fresh.qs, saved);

    // Overflow rolls the new pool back and leaves the counter.
    let mut full = env.acct(&fresh.qs).await.unwrap();
    let saved = full.clone();
    full.data[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
    env.store(&fresh.qs, full);
    let overflow = env
        .send(IX_REG_POOL, &reg_data(&legacy.pool, &fresh.base, &fresh.mint), website_accounts(payer, &fresh, &legacy.pool, &legacy.prec, &g, Some(fresh.qs)))
        .await;
    evidence.push(format!("overflow err={:?} pool_absent={}", code(&overflow), env.acct(&legacy.prec).await.is_none()));
    assert_eq!(code(&overflow), Some(6006));
    assert!(env.acct(&legacy.prec).await.is_none());
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), u64::MAX);
    env.store(&fresh.qs, saved);
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 2);

    // A stranger cannot rewrite the baseline. A stranger who pays can still register once.
    let stranger = Keypair::new();
    env.store(&stranger.pubkey(), Account { lamports: 2_000_000_000, data: vec![], owner: system_program::ID, executable: false, rent_epoch: 0 });
    let mut sync = fresh.mint.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    let stranger_sync = env
        .send_with(IX_SYNC, &sync, vec![
            AccountMeta::new(stranger.pubkey(), true),
            AccountMeta::new_readonly(g, false),
            AccountMeta::new(fresh.qs, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ], &[&stranger])
        .await;
    evidence.push(format!("stranger sync rejected={}", stranger_sync.is_err()));
    assert!(stranger_sync.is_err());
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 2);
    let stranger_reg = register(&mut env, stranger.pubkey(), &[&stranger], &g, &fresh, Some(fresh.qs)).await;
    evidence.push(format!("stranger register ok={} count={}", stranger_reg.result.is_ok(), count_of(&env.acct(&fresh.qs).await.unwrap())));
    assert!(stranger_reg.result.is_ok());
    assert_eq!(count_of(&env.acct(&fresh.qs).await.unwrap()), 3);
    assert_eq!(count_of(&env.acct(&other.qs).await.unwrap()), 4);

    // Release and one holder payment still move the pots. Claim stays a Meteora CPI and is not re-run.
    evidence.push(release_and_pay(&mut env, &g, &fresh, &a.prec).await);

    let eight = website_accounts(payer, &fresh, &legacy.pool, &legacy.prec, &g, None);
    let nine = website_accounts(payer, &fresh, &legacy.pool, &legacy.prec, &g, Some(fresh.qs));
    let eight_len = signed_len(&eight);
    let nine_len = signed_len(&nine);
    evidence.push(format!("signed-bytes website8={eight_len} proposed9={nine_len} packet_cap=1232"));
    assert!(eight_len < 1232 && nine_len < 1232 && nine_len > eight_len);
    assert_eq!(eight.len(), 8);
    assert_eq!(nine.len(), 9);

    let out = std::env::var("QUOTESTAT_EVIDENCE").unwrap_or_else(|_| "target/quotestat-evidence.txt".into());
    if let Some(parent) = std::path::Path::new(&out).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out, evidence.join("\n") + "\n").unwrap();
    println!("{}", evidence.join("\n"));
}

async fn release_and_pay(env: &mut Env, g: &Pubkey, q: &Quote, prec: &Pubkey) -> String {
    let mut pr = env.acct(prec).await.unwrap();
    pr.data[137..145].copy_from_slice(&1u64.to_le_bytes());
    pr.data[145..153].copy_from_slice(&9_000u64.to_le_bytes());
    pr.data[153..161].copy_from_slice(&2_000u64.to_le_bytes());
    env.store(prec, pr);
    let vault = ata(g, &q.mint);
    env.store(&vault, owned(spl_token::ID, token(&q.mint, g, 11_000)));
    let operator = ata(&env.payer(), &q.mint);
    env.store(&operator, owned(spl_token::ID, token(&q.mint, &env.payer(), 0)));
    let released = env
        .send(IX_RELEASE, &[], vec![
            AccountMeta::new_readonly(*g, false),
            AccountMeta::new(*prec, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(operator, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ])
        .await;
    assert!(released.is_ok(), "{released:?}");
    let burn_left = u64::from_le_bytes(env.acct(prec).await.unwrap().data[153..161].try_into().unwrap());
    let holder_left_before = u64::from_le_bytes(env.acct(prec).await.unwrap().data[145..153].try_into().unwrap());
    let op_amt = amt(&env.acct(&operator).await.unwrap());
    assert_eq!(burn_left, 0);
    assert_eq!(op_amt, 2_000);
    assert_eq!(holder_left_before, 9_000);
    let holder = Pubkey::new_unique();
    let base_acct = Pubkey::new_unique();
    env.store(&base_acct, owned(spl_token::ID, token(&q.base, &holder, 1_000)));
    let dest = Pubkey::new_unique();
    env.store(&dest, owned(spl_token::ID, token(&q.mint, &holder, 0)));
    let pool_key = Pubkey::try_from(&env.acct(prec).await.unwrap().data[8..40]).unwrap();
    let cursor = pda(&[b"paycur", pool_key.as_ref()]);
    let pay_accounts = vec![
        AccountMeta::new(env.payer(), true),
        AccountMeta::new_readonly(*g, false),
        AccountMeta::new(*prec, false),
        AccountMeta::new(vault, false),
        AccountMeta::new_readonly(q.mint, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(q.base, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(cursor, false),
        AccountMeta::new_readonly(base_acct, false),
        AccountMeta::new(dest, false),
    ];
    // A brand-new cursor is closed for the round it is created on. That is the
    // existing rule, so the first page only creates it. The next round pays.
    let opened = env.send(IX_PAY3, &1_000u64.to_le_bytes(), pay_accounts[..9].to_vec()).await;
    assert!(opened.is_ok(), "{opened:?}");
    assert_eq!(amt(&env.acct(&dest).await.unwrap()), 0);
    let mut pr = env.acct(prec).await.unwrap();
    pr.data[137..145].copy_from_slice(&2u64.to_le_bytes());
    env.store(prec, pr);
    let paid = env.send(IX_PAY3, &1_000u64.to_le_bytes(), pay_accounts).await;
    assert!(paid.is_ok(), "{paid:?}");
    let holder_left = u64::from_le_bytes(env.acct(prec).await.unwrap().data[145..153].try_into().unwrap());
    let dest_amt = amt(&env.acct(&dest).await.unwrap());
    assert_eq!(holder_left, 0);
    assert_eq!(dest_amt, 9_000);
    format!(
        "release ok burn_left={burn_left} operator={op_amt} holder_before={holder_left_before} closed_cursor_paid=0 next_round holder_left={holder_left} dest={dest_amt}"
    )
}
