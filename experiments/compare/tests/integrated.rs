//! Local runs of the two fused_fee integrations.
//! SPL Token is real. Meteora is the local stand-in, not a permanent lock.

use solana_account::{Account, AccountSharedData};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_signer::Signer;
use solana_system_interface::program as system_program;
use solana_transaction::Transaction;

const BPF: Pubkey = solana_pubkey::pubkey!("BPFLoader2111111111111111111111111111111111");
const SHARED_DEFAULT: Pubkey = solana_pubkey::pubkey!("C1nBL5ufPcQvjkmj6hAmRgoLntHonMuf5WdxVxJenJaE");
const PER_DEFAULT: Pubkey = solana_pubkey::pubkey!("C5hUUPNfyui9sr2EXzVgiZa733W1TRbUdvJcZLMFXdtw");
const DBC: Pubkey = solana_pubkey::pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
const DBC_AUTH: Pubkey = solana_pubkey::pubkey!("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");
const T22: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

const IX_INIT: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const IX_REG_CFG: [u8; 8] = [32, 247, 82, 131, 35, 183, 7, 57];
const IX_REG_POOL: [u8; 8] = [85, 229, 114, 47, 75, 145, 166, 100];
const IX_SYNC: [u8; 8] = [130, 168, 89, 25, 125, 164, 73, 104];
const IX_CLAIM: [u8; 8] = [193, 62, 163, 14, 168, 236, 179, 103];
const IX_PAY3: [u8; 8] = [26, 92, 113, 244, 121, 99, 140, 121];
const IX_REL2: [u8; 8] = [1, 142, 214, 119, 205, 117, 241, 120];
const IX_CHECK: [u8; 8] = [18, 102, 204, 80, 128, 116, 58, 205];
const IX_PAY2: [u8; 8] = [187, 255, 119, 89, 187, 17, 96, 79];
const IX_REL1: [u8; 8] = [224, 64, 74, 119, 186, 164, 250, 47];
const IX_BURN: [u8; 8] = [118, 75, 25, 157, 201, 143, 78, 137];
const IX_SET_BURN: [u8; 8] = [245, 243, 127, 223, 152, 205, 137, 210];
const IX_DAMM: [u8; 8] = [129, 173, 59, 184, 182, 112, 147, 238];
const IX_CREATOR: [u8; 8] = [223, 73, 251, 122, 3, 19, 20, 33];
const IX_MARK: [u8; 8] = [125, 72, 57, 129, 59, 15, 247, 251];
const IX_INIT_LP: [u8; 8] = [166, 15, 73, 164, 58, 151, 116, 19];
const IX_SET_LP: [u8; 8] = [77, 69, 121, 149, 171, 199, 117, 85];
const IX_AUTO: [u8; 8] = [105, 243, 28, 82, 221, 253, 201, 200];
const IX_OPEN: [u8; 8] = [199, 255, 0, 21, 197, 1, 87, 215];
const IX_CONSUME: [u8; 8] = [79, 166, 172, 47, 219, 130, 169, 58];
const IX_REFUND: [u8; 8] = [42, 187, 253, 194, 9, 216, 228, 107];
const DAMM_AUTH: Pubkey = solana_pubkey::pubkey!("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");
const DAMM: Pubkey = solana_pubkey::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");

fn sid() -> Pubkey { std::env::var("INTEGRATED_SHARED_ID").ok().and_then(|s| s.parse().ok()).unwrap_or(SHARED_DEFAULT) }
fn pid() -> Pubkey { std::env::var("INTEGRATED_PER_ID").ok().and_then(|s| s.parse().ok()).unwrap_or(PER_DEFAULT) }
const NATIVE: Pubkey = solana_pubkey::pubkey!("So11111111111111111111111111111111111111112");
const IX_INIT_RES: [u8; 8] = [11, 127, 62, 113, 165, 8, 89, 208];
const IX_IMPORT: [u8; 8] = [63, 237, 29, 130, 75, 236, 148, 88];
const IX_SEAL: [u8; 8] = [145, 255, 187, 175, 182, 82, 226, 114];
const DBC_POOL_DISC: [u8; 8] = [213, 224, 5, 209, 98, 69, 119, 92];
const DBC_CFG_DISC: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];

struct Rt { ctx: ProgramTestContext, slot: u64 }
impl Rt {
    async fn send(&mut self, program: Pubkey, disc: [u8; 8], data: &[u8], accounts: Vec<AccountMeta>) -> Result<u64, String> {
        self.slot += 1;
        let _ = self.ctx.warp_to_slot(self.slot);
        let bh = self.ctx.banks_client.get_latest_blockhash().await.unwrap();
        let payer = self.ctx.payer.insecure_clone();
        let mut buf = disc.to_vec();
        buf.extend_from_slice(data);
        let ix = Instruction { program_id: program, accounts, data: buf };
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[&payer], bh);
        match self.ctx.banks_client.process_transaction_with_metadata(tx).await {
            Ok(r) => match r.result {
                Ok(()) => Ok(r.metadata.map(|m| m.compute_units_consumed).unwrap_or(0)),
                Err(e) => Err(format!("{e:?} logs={:?}", r.metadata.map(|m| m.log_messages).unwrap_or_default())),
            },
            Err(e) => Err(format!("{e:?}")),
        }
    }
    fn store(&mut self, k: &Pubkey, a: Account) { self.ctx.set_account(k, &AccountSharedData::from(a)); }
    async fn acct(&mut self, k: &Pubkey) -> Option<Account> { self.ctx.banks_client.get_account(*k).await.unwrap() }
    fn payer(&self) -> Pubkey { self.ctx.payer.pubkey() }
}

fn rent(n: usize) -> u64 { Rent::default().minimum_balance(n).max(1) }
fn owned(owner: Pubkey, data: Vec<u8>) -> Account { Account { lamports: rent(data.len()), data, owner, executable: false, rent_epoch: 0 } }
fn mint(dec: u8, auth: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    d[0..4].copy_from_slice(&1u32.to_le_bytes());
    d[4..36].copy_from_slice(auth.as_ref());
    d[44] = dec; d[45] = 1; d
}
fn token(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = vec![0u8; 165];
    d[0..32].copy_from_slice(mint.as_ref());
    d[32..64].copy_from_slice(owner.as_ref());
    d[64..72].copy_from_slice(&amount.to_le_bytes());
    d[108] = 1; d
}
fn amt(a: &Account) -> u64 { u64::from_le_bytes(a.data[64..72].try_into().unwrap()) }
fn pda(program: &Pubkey, seeds: &[&[u8]]) -> Pubkey { Pubkey::find_program_address(seeds, program).0 }
fn ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    let prog = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    Pubkey::find_program_address(&[owner.as_ref(), token_program.as_ref(), mint.as_ref()], &prog).0
}
fn dbc_pool(config: &Pubkey, creator: &Pubkey, base: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 400];
    d[..8].copy_from_slice(&DBC_POOL_DISC);
    d[72..104].copy_from_slice(config.as_ref());
    d[104..136].copy_from_slice(creator.as_ref());
    d[136..168].copy_from_slice(base.as_ref());
    d
}
fn dbc_cfg(quote: &Pubkey, global: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 256];
    d[..8].copy_from_slice(&DBC_CFG_DISC);
    d[8..40].copy_from_slice(quote.as_ref());
    d[40..72].copy_from_slice(global.as_ref());
    d
}
fn pots(a: &Account) -> (u64, u64, u64) {
    (
        u64::from_le_bytes(a.data[145..153].try_into().unwrap()),
        u64::from_le_bytes(a.data[153..161].try_into().unwrap()),
        u64::from_le_bytes(a.data[137..145].try_into().unwrap()),
    )
}
fn custom(err: &str) -> Option<u32> {
    let marker = "Custom(";
    err.find(marker).and_then(|i| err[i+marker.len()..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
}

fn load(pt: &mut ProgramTest, id: Pubkey, path: &str) -> (String, usize) {
    let b = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let sha = solana_sha256_hasher::hash(&b).to_bytes().iter().map(|x| format!("{x:02x}")).collect::<String>();
    eprintln!("LOAD {path} bytes={} sha={sha}", b.len());
    pt.add_account(id, Account { lamports: rent(b.len()), data: b.clone(), owner: BPF, executable: true, rent_epoch: 0 });
    (sha, b.len())
}

#[tokio::test]
async fn integrated_both() {
    let mut pt = ProgramTest::default();
    pt.prefer_bpf(false);
    pt.set_compute_max_units(1_400_000);
    let only = std::env::var("INTEGRATED_ONLY").unwrap_or_default();
    let shared_so = std::env::var("INTEGRATED_SHARED_SO").unwrap_or_else(|_| "/workspace/experiments/integrated/shared_fee/target/deploy/fused_fee.so".into());
    let per_so = std::env::var("INTEGRATED_PER_SO").unwrap_or_else(|_| "/workspace/experiments/integrated/per_pool_fee/target/deploy/fused_fee.so".into());
    let (sha_s, n_s) = if only != "per" { load(&mut pt, sid(), &shared_so) } else { (String::new(), 0) };
    let (sha_p, n_p) = if only != "shared" { load(&mut pt, pid(), &per_so) } else { (String::new(), 0) };
    let mock = ["/workspace/harness/mock_meteora/target/deploy/mock_meteora.so", "/workspace/target/deploy/mock_meteora.so"]
        .into_iter().find(|p| std::path::Path::new(p).is_file()).expect("mock_meteora.so");
    let mock_bytes = std::fs::read(mock).unwrap();
    for id in [DBC, solana_pubkey::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG")] {
        pt.add_account(id, Account { lamports: rent(mock_bytes.len()), data: mock_bytes.clone(), owner: BPF, executable: true, rent_epoch: 0 });
    }
    let mut ctx = pt.start_with_context().await;
    let _ = ctx.warp_to_slot(1);
    let mut rt = Rt { ctx, slot: 1 };
    let mut rows = Vec::new();

    if only != "per" {
        shared_flow(&mut rt, &mut rows).await;
        extra_paths(&mut rt, &mut rows).await;
    }
    if only != "shared" {
        per_flow(&mut rt, &mut rows).await;
    }
    t22_policy(&mut rt, &mut rows).await;

    let mut out = format!("{{\"shared_sha\":\"{sha_s}\",\"shared_bytes\":{n_s},\"per_sha\":\"{sha_p}\",\"per_bytes\":{n_p},\"rows\":[\n");
    for (i, r) in rows.iter().enumerate() {
        let c = if i + 1 == rows.len() { "" } else { "," };
        out.push_str(&format!("{{\"id\":\"{}\",\"ok\":{},\"detail\":\"{}\",\"cu\":{}}}{c}\n", r.0, r.1, r.2, r.3));
    }
    out.push_str("]}\n");
    let out_path = std::env::var("INTEGRATED_OUT").unwrap_or_else(|_| "/workspace/experiments/results/integrated-fee.json".into());
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("{out}");
}

struct Rec(String, bool, String, u64);

async fn shared_flow(rt: &mut Rt, rows: &mut Vec<Rec>) {
    let admin = rt.payer();
    let g = pda(&sid(), &[b"global"]);
    rt.send(sid(), IX_INIT, &[], vec![AccountMeta::new(admin, true), AccountMeta::new(g, false), AccountMeta::new_readonly(system_program::ID, false)]).await.unwrap();
    let quote = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    let auth = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, mint(9, &auth)));
    rt.store(&base, owned(spl_token::ID, mint(6, &auth)));
    rt.store(&config, owned(DBC, dbc_cfg(&quote, &g)));
    rt.store(&pool, owned(DBC, dbc_pool(&config, &auth, &base)));
    let vault = ata(&g, &quote, &spl_token::ID);
    rt.store(&vault, owned(spl_token::ID, token(&quote, &g, 0)));
    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(&base, &g, 0)));
    let qsrc = Pubkey::new_unique();
    rt.store(&qsrc, owned(spl_token::ID, token(&quote, &g, 100_000)));
    rt.store(&DBC_AUTH, owned(system_program::ID, vec![]));
    let cfg_rec = pda(&sid(), &[b"config", quote.as_ref()]);
    let mut cfg_data = quote.to_bytes().to_vec();
    rt.send(sid(), IX_REG_CFG, &cfg_data, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg_rec, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(quote, false),
    ]).await.expect("cfg");
    let qs = pda(&sid(), &[b"qstat", quote.as_ref()]);
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(sid(), IX_SYNC, &sync, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("sync");
    let reserve = pda(&sid(), &[b"qres", quote.as_ref()]);
    rt.send(sid(), IX_INIT_RES, &[], vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("init reserve");
    let prec = pda(&sid(), &[b"pool", pool.as_ref()]);
    let mut rd = Vec::new();
    rd.extend_from_slice(pool.as_ref()); rd.extend_from_slice(base.as_ref()); rd.extend_from_slice(quote.as_ref());
    let reg = rt.send(sid(), IX_REG_POOL, &rd, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(cfg_rec, false), AccountMeta::new(prec, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(g, false),
        AccountMeta::new_readonly(pool, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(qs, false), AccountMeta::new(reserve, false),
    ]).await;
    assert!(reg.is_ok(), "register {reg:?}");
    let owed = Pubkey::new_unique();
    let marker = pda(&sid(), &[b"qimp", pool.as_ref()]);
    rt.send(sid(), IX_IMPORT, &[], vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(prec, false), AccountMeta::new_readonly(owed, false),
        AccountMeta::new(marker, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("import");
    let seal_cu = rt.send(sid(), IX_SEAL, &[], vec![
        AccountMeta::new_readonly(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false),
    ]).await.expect("seal");
    rows.push(Rec("shared-seal".into(), true, "empty quote sealed".into(), seal_cu));
    let cursor = pda(&sid(), &[b"paycur", pool.as_ref()]);
    rt.send(sid(), IX_PAY3, &0u64.to_le_bytes(), vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new(cursor, false),
        AccountMeta::new(reserve, false),
    ]).await.expect("open cursor");

    let mut claim_data = Vec::new();
    claim_data.extend_from_slice(&1u64.to_le_bytes());
    claim_data.extend_from_slice(&11_000u64.to_le_bytes());
    let claim_cu = rt.send(sid(), IX_CLAIM, &claim_data, vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new_readonly(DBC_AUTH, false),
        AccountMeta::new_readonly(config, false), AccountMeta::new(pool, false), AccountMeta::new(base_dest, false),
        AccountMeta::new(vault, false), AccountMeta::new(Pubkey::new_unique(), false), AccountMeta::new(qsrc, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(quote, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false), AccountMeta::new_readonly(DBC, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(reserve, false),
    ]).await.expect("claim");
    let pr = rt.acct(&prec).await.unwrap();
    let (holder, burn, round) = pots(&pr);
    let vault_amt = amt(&rt.acct(&vault).await.unwrap());
    let res = rt.acct(&reserve).await.unwrap();
    let rh = u64::from_le_bytes(res.data[56..64].try_into().unwrap());
    let rb = u64::from_le_bytes(res.data[64..72].try_into().unwrap());
    assert_eq!((holder, burn, round), (9_000, 2_000, 1));
    assert_eq!((rh, rb, vault_amt), (9_000, 2_000, 11_000));
    rows.push(Rec("shared-claim".into(), true, format!("holder={holder} burn={burn} vault={vault_amt}"), claim_cu));

    // Pay v3 one holder. Reserve is the last named account.
    let wallet = Pubkey::new_unique();
    let base_acct = Pubkey::new_unique();
    let dest = Pubkey::new_unique();
    rt.store(&base_acct, owned(spl_token::ID, token(&base, &wallet, 100)));
    rt.store(&dest, owned(spl_token::ID, token(&quote, &wallet, 0)));
    let mut pay_data = 100u64.to_le_bytes().to_vec();
    let pay_cu = rt.send(sid(), IX_PAY3, &pay_data, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new(cursor, false),
        AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(base_acct, false), AccountMeta::new(dest, false),
    ]).await.expect("pay v3");
    let dest_amt = amt(&rt.acct(&dest).await.unwrap());
    let holder_after = pots(&rt.acct(&prec).await.unwrap()).0;
    assert_eq!(dest_amt, 9_000);
    assert_eq!(holder_after, 0);
    let rh = u64::from_le_bytes(rt.acct(&reserve).await.unwrap().data[56..64].try_into().unwrap());
    assert_eq!(rh, 0);
    rows.push(Rec("shared-pay-v3".into(), true, format!("paid={dest_amt} reserve_holder={rh}"), pay_cu));

    // Release burn. One pool, vault still covers it.
    let op = Pubkey::new_unique();
    rt.store(&op, owned(spl_token::ID, token(&quote, &admin, 0)));
    let rel = rt.send(sid(), IX_REL2, &[], vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new(vault, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(op, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new(reserve, false),
    ]).await.expect("release");
    assert_eq!(amt(&rt.acct(&op).await.unwrap()), 2_000);
    assert_eq!(pots(&rt.acct(&prec).await.unwrap()).1, 0);
    rows.push(Rec("shared-release".into(), true, "burn 2000 to admin".into(), rel));

    let pool2 = Pubkey::new_unique();
    rt.store(&pool2, owned(DBC, dbc_pool(&config, &auth, &base)));
    let prec2 = pda(&sid(), &[b"pool", pool2.as_ref()]);
    let mut rd2 = Vec::new();
    rd2.extend_from_slice(pool2.as_ref()); rd2.extend_from_slice(base.as_ref()); rd2.extend_from_slice(quote.as_ref());
    let bad = rt.send(sid(), IX_REG_POOL, &rd2, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(cfg_rec, false), AccountMeta::new(prec2, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(g, false),
        AccountMeta::new_readonly(pool2, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(qs, false),
    ]).await;
    let code = bad.as_ref().err().and_then(|e| custom(e));
    rows.push(Rec("shared-register-without-reserve".into(), code == Some(6050), format!("err={code:?}"), 0));
    assert_eq!(code, Some(6050), "{bad:?}");
}

async fn per_flow(rt: &mut Rt, rows: &mut Vec<Rec>) {
    let admin = rt.payer();
    let g = pda(&pid(), &[b"global"]);
    rt.send(pid(), IX_INIT, &[], vec![AccountMeta::new(admin, true), AccountMeta::new(g, false), AccountMeta::new_readonly(system_program::ID, false)]).await.unwrap();
    let quote = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    let auth = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, mint(9, &auth)));
    rt.store(&base, owned(spl_token::ID, mint(6, &auth)));
    rt.store(&config, owned(DBC, dbc_cfg(&quote, &g)));
    rt.store(&pool, owned(DBC, dbc_pool(&config, &auth, &base)));
    let vault = ata(&g, &quote, &spl_token::ID);
    rt.store(&vault, owned(spl_token::ID, token(&quote, &g, 0)));
    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(&base, &g, 0)));
    let qsrc = Pubkey::new_unique();
    rt.store(&qsrc, owned(spl_token::ID, token(&quote, &g, 100_000)));
    rt.store(&DBC_AUTH, owned(system_program::ID, vec![]));
    let cfg_rec = pda(&pid(), &[b"config", quote.as_ref()]);
    rt.send(pid(), IX_REG_CFG, &quote.to_bytes(), vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg_rec, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(quote, false),
    ]).await.expect("pcfg");
    let qs = pda(&pid(), &[b"qstat", quote.as_ref()]);
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(pid(), IX_SYNC, &sync, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("psync");
    let prec = pda(&pid(), &[b"pool", pool.as_ref()]);
    let mut rd = Vec::new();
    rd.extend_from_slice(pool.as_ref()); rd.extend_from_slice(base.as_ref()); rd.extend_from_slice(quote.as_ref());
    rt.send(pid(), IX_REG_POOL, &rd, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(cfg_rec, false), AccountMeta::new(prec, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(g, false),
        AccountMeta::new_readonly(pool, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(qs, false),
    ]).await.expect("preg");
    let pvault = ata(&prec, &quote, &spl_token::ID);
    rt.store(&pvault, owned(spl_token::ID, token(&quote, &prec, 0)));
    let cursor = pda(&pid(), &[b"paycur", pool.as_ref()]);
    rt.send(pid(), IX_PAY3, &0u64.to_le_bytes(), vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false),
        AccountMeta::new(pvault, false), AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new(cursor, false),
    ]).await.expect("open per cursor");
    let mut claim_data = Vec::new();
    claim_data.extend_from_slice(&1u64.to_le_bytes());
    claim_data.extend_from_slice(&11_000u64.to_le_bytes());
    let cu = rt.send(pid(), IX_CLAIM, &claim_data, vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new_readonly(DBC_AUTH, false),
        AccountMeta::new_readonly(config, false), AccountMeta::new(pool, false), AccountMeta::new(base_dest, false),
        AccountMeta::new(vault, false), AccountMeta::new(Pubkey::new_unique(), false), AccountMeta::new(qsrc, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(quote, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false), AccountMeta::new_readonly(DBC, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(pvault, false),
    ]).await.expect("pclaim");
    let global_left = amt(&rt.acct(&vault).await.unwrap());
    let pool_got = amt(&rt.acct(&pvault).await.unwrap());
    let (holder, burn, _) = pots(&rt.acct(&prec).await.unwrap());
    assert_eq!(global_left, 0);
    assert_eq!(pool_got, 11_000);
    assert_eq!((holder, burn), (9_000, 2_000));
    rows.push(Rec("per-claim-moves-to-pool-vault".into(), true, format!("global={global_left} pool={pool_got}"), cu));

    let wallet = Pubkey::new_unique();
    let base_acct = Pubkey::new_unique();
    let dest = Pubkey::new_unique();
    rt.store(&base_acct, owned(spl_token::ID, token(&base, &wallet, 100)));
    rt.store(&dest, owned(spl_token::ID, token(&quote, &wallet, 0)));
    let pay = rt.send(pid(), IX_PAY3, &100u64.to_le_bytes(), vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false),
        AccountMeta::new(pvault, false), AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new(cursor, false),
        AccountMeta::new_readonly(base_acct, false), AccountMeta::new(dest, false),
    ]).await.expect("ppay");
    assert_eq!(amt(&rt.acct(&dest).await.unwrap()), 9_000);
    assert_eq!(amt(&rt.acct(&pvault).await.unwrap()), 2_000);
    assert_eq!(amt(&rt.acct(&vault).await.unwrap()), 0, "global vault stays empty");
    rows.push(Rec("per-pay-v3".into(), true, "paid from pool vault, global untouched".into(), pay));
    let pool8 = Pubkey::new_unique();
    rt.store(&pool8, owned(DBC, dbc_pool(&config, &auth, &base)));
    let prec8 = pda(&pid(), &[b"pool", pool8.as_ref()]);
    let mut rd8 = Vec::new();
    rd8.extend_from_slice(pool8.as_ref()); rd8.extend_from_slice(base.as_ref()); rd8.extend_from_slice(quote.as_ref());
    let old = rt.send(pid(), IX_REG_POOL, &rd8, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(cfg_rec, false), AccountMeta::new(prec8, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(g, false),
        AccountMeta::new_readonly(pool8, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new_readonly(quote, false),
    ]).await;
    let old_code = old.as_ref().err().and_then(|e| custom(e));
    rows.push(Rec("per-old-8-account-register".into(), old_code == Some(6048), format!("{old_code:?}"), 0));
    assert_eq!(old_code, Some(6048), "{old:?}");
}

async fn t22_policy(rt: &mut Rt, rows: &mut Vec<Rec>) {
    let only = std::env::var("INTEGRATED_ONLY").unwrap_or_default();
    let checker = if only == "per" { pid() } else { sid() };
    let good = Pubkey::new_unique();
    let mut bytes = mint(6, &rt.payer());
    bytes.resize(166, 0);
    bytes[165] = 1;
    rt.store(&good, owned(T22, bytes));
    let ok = rt.send(checker, IX_CHECK, &[], vec![AccountMeta::new_readonly(good, false)]).await;
    rows.push(Rec("t22-plain-check".into(), ok.is_ok(), format!("{ok:?}"), *ok.as_ref().unwrap_or(&0)));
    assert!(ok.is_ok(), "{ok:?}");

    let mut fee = vec![0u8; 108];
    fee[88] = 101;
    fee[89] = 0;
    let bad = Pubkey::new_unique();
    let mut b = mint(6, &rt.payer());
    b.resize(166, 0);
    b[165] = 1;
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&108u16.to_le_bytes());
    b.extend_from_slice(&fee);
    rt.store(&bad, owned(T22, b));
    let denied = rt.send(checker, IX_CHECK, &[], vec![AccountMeta::new_readonly(bad, false)]).await;
    rows.push(Rec("t22-fee-over-1pct".into(), denied.is_err(), format!("{:?}", denied.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(denied.is_err());

    let mut hook = vec![0u8; 64];
    hook[32] = 1;
    let hk = Pubkey::new_unique();
    let mut h = mint(6, &rt.payer());
    h.resize(166, 0);
    h[165] = 1;
    h.extend_from_slice(&14u16.to_le_bytes());
    h.extend_from_slice(&64u16.to_le_bytes());
    h.extend_from_slice(&hook);
    rt.store(&hk, owned(T22, h));
    let hook_prog = if only == "shared" { sid() } else { pid() };
    let hook_denied = rt.send(hook_prog, IX_CHECK, &[], vec![AccountMeta::new_readonly(hk, false)]).await;
    rows.push(Rec("t22-active-hook".into(), hook_denied.is_err(), "active hook rejected".into(), 0));
    assert!(hook_denied.is_err());
}

async fn extra_paths(rt: &mut Rt, rows: &mut Vec<Rec>) {
    // Claim before the reserve is sealed must fail, and must not leave a pool pot.
    let admin = rt.payer();
    let g = pda(&sid(), &[b"global"]);
    let quote = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    let auth = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, mint(9, &auth)));
    rt.store(&base, owned(spl_token::ID, mint(6, &auth)));
    rt.store(&config, owned(DBC, dbc_cfg(&quote, &g)));
    rt.store(&pool, owned(DBC, dbc_pool(&config, &auth, &base)));
    let vault = ata(&g, &quote, &spl_token::ID);
    rt.store(&vault, owned(spl_token::ID, token(&quote, &g, 0)));
    let qsrc = Pubkey::new_unique();
    rt.store(&qsrc, owned(spl_token::ID, token(&quote, &g, 50_000)));
    let cfg_rec = pda(&sid(), &[b"config", quote.as_ref()]);
    rt.send(sid(), IX_REG_CFG, &quote.to_bytes(), vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg_rec, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(quote, false),
    ]).await.expect("xcfg");
    let qs = pda(&sid(), &[b"qstat", quote.as_ref()]);
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(sid(), IX_SYNC, &sync, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("xsync");
    let reserve = pda(&sid(), &[b"qres", quote.as_ref()]);
    rt.send(sid(), IX_INIT_RES, &[], vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("xres");
    let prec = pda(&sid(), &[b"pool", pool.as_ref()]);
    let mut rd = Vec::new();
    rd.extend_from_slice(pool.as_ref()); rd.extend_from_slice(base.as_ref()); rd.extend_from_slice(quote.as_ref());
    rt.send(sid(), IX_REG_POOL, &rd, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(cfg_rec, false), AccountMeta::new(prec, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(g, false),
        AccountMeta::new_readonly(pool, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(qs, false), AccountMeta::new(reserve, false),
    ]).await.expect("xreg");
    let mut claim_data = Vec::new();
    claim_data.extend_from_slice(&1u64.to_le_bytes());
    claim_data.extend_from_slice(&11_000u64.to_le_bytes());
    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(&base, &g, 0)));
    let base_vault = Pubkey::new_unique();
    rt.store(&base_vault, owned(system_program::ID, vec![]));
    let early = rt.send(sid(), IX_CLAIM, &claim_data, claim_accounts(g, prec, config, pool, vault, qsrc, base, quote, reserve, base_dest, base_vault)).await;
    let code = early.as_ref().err().and_then(|e| custom(e));
    rows.push(Rec("shared-claim-before-seal".into(), code == Some(6052), format!("{code:?}"), 0));
    assert_eq!(code, Some(6052), "{early:?}");
    assert!(rt.acct(&prec).await.unwrap().data[145] == 0 || pots(&rt.acct(&prec).await.unwrap()).0 == 0);

    // Two claims in one transaction. The second names a missing pool record, so both roll back.
    let owed = Pubkey::new_unique();
    let marker = pda(&sid(), &[b"qimp", pool.as_ref()]);
    rt.send(sid(), IX_IMPORT, &[], vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(prec, false), AccountMeta::new_readonly(owed, false),
        AccountMeta::new(marker, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("ximp");
    rt.send(sid(), IX_SEAL, &[], vec![
        AccountMeta::new_readonly(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false),
    ]).await.expect("xseal");
    let bogus = Pubkey::new_unique();
    let bad_claim = claim_accounts(g, bogus, config, pool, vault, qsrc, base, quote, reserve, base_dest, base_vault);
    let bh = rt.ctx.banks_client.get_latest_blockhash().await.unwrap();
    let payer = rt.ctx.payer.insecure_clone();
    let mut d1 = IX_CLAIM.to_vec(); d1.extend_from_slice(&claim_data);
    let d2 = d1.clone();
    let ix1 = Instruction { program_id: sid(), accounts: claim_accounts(g, prec, config, pool, vault, qsrc, base, quote, reserve, base_dest, base_vault), data: d1 };
    let ix2 = Instruction { program_id: sid(), accounts: bad_claim, data: d2 };
    let tx = Transaction::new_signed_with_payer(&[ix1, ix2], Some(&payer.pubkey()), &[&payer], bh);
    let rolled = rt.ctx.banks_client.process_transaction_with_metadata(tx).await.unwrap();
    assert!(rolled.result.is_err(), "second claim must roll the first back");
    assert_eq!(pots(&rt.acct(&prec).await.unwrap()).0, 0);
    assert_eq!(amt(&rt.acct(&vault).await.unwrap()), 0);
    rows.push(Rec("shared-two-claim-rollback".into(), true, "neither pot nor vault moved".into(), 0));

    // One claim, then pay v2, release v1, direct burn.
    rt.send(sid(), IX_CLAIM, &claim_data, claim_accounts(g, prec, config, pool, vault, qsrc, base, quote, reserve, base_dest, base_vault)).await.expect("xclaim");
    let cursor = pda(&sid(), &[b"paycur", pool.as_ref()]);
    let sol_vault = pda(&sid(), &[b"sol_vault"]);
    let unwrap = pda(&sid(), &[b"unwrap", pool.as_ref()]);
    rt.send(sid(), IX_PAY2, &0u64.to_le_bytes(), vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(cursor, false), AccountMeta::new(sol_vault, false), AccountMeta::new(unwrap, false),
        AccountMeta::new_readonly(NATIVE, false), AccountMeta::new(reserve, false),
    ]).await.expect("open v2");
    // Cursor was opened after the claim, so it is closed for this round. Open it by
    // warping the round: a second claim makes round 2 and the cursor round is 1, so the next pay resets.
    rt.send(sid(), IX_CLAIM, &claim_data, claim_accounts(g, prec, config, pool, vault, qsrc, base, quote, reserve, base_dest, base_vault)).await.expect("xclaim2");
    let wallet = Pubkey::new_unique();
    let hb = Pubkey::new_unique();
    let hd = Pubkey::new_unique();
    rt.store(&hb, owned(spl_token::ID, token(&base, &wallet, 100)));
    rt.store(&hd, owned(spl_token::ID, token(&quote, &wallet, 0)));
    let pay2 = rt.send(sid(), IX_PAY2, &100u64.to_le_bytes(), vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(cursor, false), AccountMeta::new(sol_vault, false), AccountMeta::new(unwrap, false),
        AccountMeta::new_readonly(NATIVE, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(hb, false), AccountMeta::new(hd, false),
    ]).await.expect("pay v2");
    let paid = amt(&rt.acct(&hd).await.unwrap());
    rows.push(Rec("shared-pay-v2".into(), paid == 18_000, format!("paid={paid}"), pay2));
    assert_eq!(paid, 18_000, "two claims of 9000");

    // Preexisting shortfall: drop 1 raw unit. Not created by a program instruction.
    let bal = amt(&rt.acct(&vault).await.unwrap());
    let burn_before = pots(&rt.acct(&prec).await.unwrap()).1;
    let mut short = rt.acct(&vault).await.unwrap();
    short.data[64..72].copy_from_slice(&(bal - 1).to_le_bytes());
    rt.store(&vault, short);
    let op = Pubkey::new_unique();
    rt.store(&op, owned(spl_token::ID, token(&quote, &admin, 0)));
    let rel = rt.send(sid(), IX_REL1, &[], vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new(vault, false),
        AccountMeta::new(op, false), AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(reserve, false),
    ]).await;
    let rel_code = rel.as_ref().err().and_then(|e| custom(e));
    rows.push(Rec("shared-preexisting-short-release-v1".into(), rel.is_err(), format!("err={rel_code:?} burn_still={burn_before}"), 0));
    assert!(rel.is_err());
    assert_eq!(pots(&rt.acct(&prec).await.unwrap()).1, burn_before);
    let mut restored = rt.acct(&vault).await.unwrap();
    restored.data[64..72].copy_from_slice(&bal.to_le_bytes());
    rt.store(&vault, restored);

    rt.send(sid(), IX_SET_BURN, &quote.to_bytes(), vec![
        AccountMeta::new_readonly(admin, true), AccountMeta::new(g, false),
    ]).await.expect("set burn");
    let mut mint_acct = rt.acct(&quote).await.unwrap();
    mint_acct.data[36..44].copy_from_slice(&100_000u64.to_le_bytes());
    rt.store(&quote, mint_acct);
    let burn_cu = rt.send(sid(), IX_BURN, &[], vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new(quote, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(reserve, false),
    ]).await.expect("direct burn");
    assert_eq!(pots(&rt.acct(&prec).await.unwrap()).1, 0);
    rows.push(Rec("shared-direct-burn".into(), true, "burn pot cleared".into(), burn_cu));

    // Historical website register: 7 named accounts plus the quote mint. No QuoteStat, no reserve.
    let pool8 = Pubkey::new_unique();
    rt.store(&pool8, owned(DBC, dbc_pool(&config, &auth, &base)));
    let prec8 = pda(&sid(), &[b"pool", pool8.as_ref()]);
    let mut rd8 = Vec::new();
    rd8.extend_from_slice(pool8.as_ref()); rd8.extend_from_slice(base.as_ref()); rd8.extend_from_slice(quote.as_ref());
    let old = rt.send(sid(), IX_REG_POOL, &rd8, vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(cfg_rec, false), AccountMeta::new(prec8, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(g, false),
        AccountMeta::new_readonly(pool8, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new_readonly(quote, false),
    ]).await;
    let old_code = old.as_ref().err().and_then(|e| custom(e));
    rows.push(Rec("shared-old-8-account-register".into(), old_code == Some(6048), format!("{old_code:?}"), 0));
    assert_eq!(old_code, Some(6048), "{old:?}");
}

fn claim_accounts(g: Pubkey, prec: Pubkey, config: Pubkey, pool: Pubkey, vault: Pubkey, qsrc: Pubkey, base: Pubkey, quote: Pubkey, reserve: Pubkey, base_dest: Pubkey, base_vault: Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new_readonly(DBC_AUTH, false),
        AccountMeta::new_readonly(config, false), AccountMeta::new(pool, false), AccountMeta::new(base_dest, false),
        AccountMeta::new(vault, false), AccountMeta::new(base_vault, false), AccountMeta::new(qsrc, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(quote, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false), AccountMeta::new_readonly(DBC, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(reserve, false),
    ]
}
