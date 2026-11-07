//! Remaining integrated paths: DAMM, creator, pay v2, burn, releases,
//! bounded auto-LP past 22 pools, reservation consume/refund, fee transfer.

use solana_account::{Account, AccountSharedData};
use solana_instruction::{AccountMeta, Instruction};
use solana_message::Message;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_signer::Signer;
use solana_system_interface::program as system_program;
use solana_transaction::Transaction;

const BPF: Pubkey = solana_pubkey::pubkey!("BPFLoader2111111111111111111111111111111111");
const SHARED: Pubkey = solana_pubkey::pubkey!("C1nBL5ufPcQvjkmj6hAmRgoLntHonMuf5WdxVxJenJaE");
const PER: Pubkey = solana_pubkey::pubkey!("C5hUUPNfyui9sr2EXzVgiZa733W1TRbUdvJcZLMFXdtw");
const DBC: Pubkey = solana_pubkey::pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
const DBC_AUTH: Pubkey = solana_pubkey::pubkey!("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");
const T22: Pubkey = solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const DAMM: Pubkey = solana_pubkey::pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
const DAMM_AUTH: Pubkey = solana_pubkey::pubkey!("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");
const NATIVE: Pubkey = solana_pubkey::pubkey!("So11111111111111111111111111111111111111112");

const IX_INIT: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const IX_REG_CFG: [u8; 8] = [32, 247, 82, 131, 35, 183, 7, 57];
const IX_REG_POOL: [u8; 8] = [85, 229, 114, 47, 75, 145, 166, 100];
const IX_SYNC: [u8; 8] = [130, 168, 89, 25, 125, 164, 73, 104];
const IX_CLAIM: [u8; 8] = [193, 62, 163, 14, 168, 236, 179, 103];
const IX_PAY3: [u8; 8] = [26, 92, 113, 244, 121, 99, 140, 121];
const IX_PAY2: [u8; 8] = [187, 255, 119, 89, 187, 17, 96, 79];
const IX_REL1: [u8; 8] = [224, 64, 74, 119, 186, 164, 250, 47];
const IX_REL2: [u8; 8] = [1, 142, 214, 119, 205, 117, 241, 120];
const IX_BURN: [u8; 8] = [118, 75, 25, 157, 201, 143, 78, 137];
const IX_SET_BURN: [u8; 8] = [245, 243, 127, 223, 152, 205, 137, 210];
const IX_DAMM: [u8; 8] = [129, 173, 59, 184, 182, 112, 147, 238];
const IX_CREATOR: [u8; 8] = [223, 73, 251, 122, 3, 19, 20, 33];
const IX_MARK: [u8; 8] = [125, 72, 57, 129, 59, 15, 247, 251];
const IX_INIT_LP: [u8; 8] = [166, 15, 73, 164, 58, 151, 116, 19];
const IX_SET_LP: [u8; 8] = [77, 69, 121, 149, 171, 199, 117, 85];
const IX_AUTO: [u8; 8] = [105, 243, 28, 82, 221, 253, 201, 200];
const IX_INIT_RES: [u8; 8] = [11, 127, 62, 113, 165, 8, 89, 208];
const IX_IMPORT: [u8; 8] = [63, 237, 29, 130, 75, 236, 148, 88];
const IX_SEAL: [u8; 8] = [145, 255, 187, 175, 182, 82, 226, 114];
const IX_OPEN: [u8; 8] = [199, 255, 0, 21, 197, 1, 87, 215];
const IX_CONSUME: [u8; 8] = [79, 166, 172, 47, 219, 130, 169, 58];
const IX_REFUND: [u8; 8] = [42, 187, 253, 194, 9, 216, 228, 107];
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
fn fee_mint(dec: u8, auth: &Pubkey) -> Vec<u8> {
    let mut d = mint(dec, auth);
    d.resize(165, 0);
    d.push(1);
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&108u16.to_le_bytes());
    let mut fee = vec![0u8; 108];
    fee[80..88].copy_from_slice(&u64::MAX.to_le_bytes());
    fee[88..90].copy_from_slice(&100u16.to_le_bytes());
    fee[98..106].copy_from_slice(&u64::MAX.to_le_bytes());
    fee[106..108].copy_from_slice(&100u16.to_le_bytes());
    d.extend_from_slice(&fee);
    d
}
fn fee_token(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = token(mint, owner, amount);
    d.push(2);
    d.extend_from_slice(&2u16.to_le_bytes());
    d.extend_from_slice(&8u16.to_le_bytes());
    d.extend_from_slice(&0u64.to_le_bytes());
    d
}
fn amt(a: &Account) -> u64 { u64::from_le_bytes(a.data[64..72].try_into().unwrap()) }
fn pda(program: &Pubkey, seeds: &[&[u8]]) -> Pubkey { Pubkey::find_program_address(seeds, program).0 }
fn ata(owner: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    let prog = solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    Pubkey::find_program_address(&[owner.as_ref(), token_program.as_ref(), mint.as_ref()], &prog).0
}
fn dbc_pool(config: &Pubkey, creator: &Pubkey, base: &Pubkey, migrated: u8) -> Vec<u8> {
    let mut d = vec![0u8; 400];
    d[..8].copy_from_slice(&DBC_POOL_DISC);
    d[72..104].copy_from_slice(config.as_ref());
    d[104..136].copy_from_slice(creator.as_ref());
    d[136..168].copy_from_slice(base.as_ref());
    d[305] = migrated;
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
    err.find(marker).and_then(|i| err[i + marker.len()..].chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
}
fn load(pt: &mut ProgramTest, id: Pubkey, path: &str) {
    let b = std::fs::read(path).unwrap();
    pt.add_account(id, Account { lamports: rent(b.len()), data: b, owner: BPF, executable: true, rent_epoch: 0 });
}
struct Rec(String, bool, String, u64);

#[tokio::test]
async fn integrated_rest() {
    let mut pt = ProgramTest::default();
    pt.prefer_bpf(false);
    pt.set_compute_max_units(1_400_000);
    let shared_so = std::env::var("INTEGRATED_SHARED_SO").unwrap_or_else(|_| "/workspace/experiments/integrated/shared_fee/target/deploy/fused_fee.so".into());
    let per_so = std::env::var("INTEGRATED_PER_SO").unwrap_or_else(|_| "/workspace/experiments/integrated/per_pool_fee/target/deploy/fused_fee.so".into());
    let shared_id: Pubkey = std::env::var("INTEGRATED_SHARED_ID").ok().and_then(|s| s.parse().ok()).unwrap_or(SHARED);
    let per_id: Pubkey = std::env::var("INTEGRATED_PER_ID").ok().and_then(|s| s.parse().ok()).unwrap_or(PER);
    let only = std::env::var("INTEGRATED_ONLY").unwrap_or_default();
    if only != "per" { load(&mut pt, shared_id, &shared_so); }
    if only != "shared" { load(&mut pt, per_id, &per_so); }
    let mock = std::fs::read("/workspace/harness/mock_meteora/target/deploy/mock_meteora.so").unwrap();
    for id in [DBC, DAMM] {
        pt.add_account(id, Account { lamports: rent(mock.len()), data: mock.clone(), owner: BPF, executable: true, rent_epoch: 0 });
    }
    let mut ctx = pt.start_with_context().await;
    let _ = ctx.warp_to_slot(1);
    let mut rt = Rt { ctx, slot: 1 };
    rt.store(&DBC_AUTH, owned(system_program::ID, vec![]));
    rt.store(&NATIVE, owned(spl_token::ID, mint(9, &rt.payer())));
    let mut rows = Vec::new();
    if only != "per" {
        rt.send(shared_id, IX_INIT, &[], vec![AccountMeta::new(rt.payer(), true), AccountMeta::new(pda(&shared_id, &[b"global"]), false), AccountMeta::new_readonly(system_program::ID, false)]).await.unwrap();
        shared_cases(&mut rt, shared_id, &mut rows).await;
    }
    if only != "shared" {
        rt.send(per_id, IX_INIT, &[], vec![AccountMeta::new(rt.payer(), true), AccountMeta::new(pda(&per_id, &[b"global"]), false), AccountMeta::new_readonly(system_program::ID, false)]).await.unwrap();
        per_cases(&mut rt, per_id, &mut rows).await;
    }
    let mut out = String::from("{\"rows\":[\n");
    for (i, r) in rows.iter().enumerate() {
        let c = if i + 1 == rows.len() { "" } else { "," };
        out.push_str(&format!("{{\"id\":\"{}\",\"ok\":{},\"detail\":\"{}\",\"cu\":{}}}{c}\n", r.0, r.1, r.2.replace('"', "'"), r.3));
    }
    out.push_str("]}\n");
    let path = std::env::var("INTEGRATED_OUT").unwrap_or_else(|_| "/workspace/experiments/results/integrated-rest.json".into());
    std::fs::write(&path, &out).unwrap();
    eprintln!("{out}");
    assert!(rows.iter().all(|r| r.1), "a rest case failed:\n{out}");
}

async fn shared_cases(rt: &mut Rt, prog: Pubkey, rows: &mut Vec<Rec>) {
    let admin = rt.payer();
    let g = pda(&prog, &[b"global"]);
    // Register during backfill, then seal only after that pool is imported.
    let (quote, base, config, pool, prec, vault, qsrc, reserve, qs, cfg) = boot_shared(rt, prog, &g, &admin, false).await;
    let again = import_result(rt, prog, &admin, &g, &reserve, &prec, &pool).await;
    rows.push(Rec("shared-double-import".into(), again.as_ref().err().and_then(|e| custom(e)) == Some(6053), format!("{:?}", again.as_ref().err().and_then(|e| custom(e))), 0));
    assert_eq!(again.as_ref().err().and_then(|e| custom(e)), Some(6053), "{again:?}");
    let pool2 = Pubkey::new_unique();
    rt.store(&pool2, owned(DBC, dbc_pool(&config, &admin, &base, 0)));
    let prec2 = pda(&prog, &[b"pool", pool2.as_ref()]);
    reg_pool(rt, prog, &admin, &g, &cfg, &prec2, &pool2, &config, &base, &quote, &qs, Some(&reserve)).await;
    let early = rt.send(prog, IX_SEAL, &[], vec![
        AccountMeta::new_readonly(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false),
    ]).await;
    let code = early.as_ref().err().and_then(|e| custom(e));
    rows.push(Rec("shared-register-during-backfill-blocks-seal".into(), code == Some(6051), format!("{code:?}"), 0));
    assert_eq!(code, Some(6051), "{early:?}");
    import(rt, prog, &admin, &g, &reserve, &prec2, &pool2).await;
    rt.send(prog, IX_SEAL, &[], vec![
        AccountMeta::new_readonly(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false),
    ]).await.expect("seal after both imports");
    let late = import_result(rt, prog, &admin, &g, &reserve, &prec2, &pool2).await;
    rows.push(Rec("shared-import-after-seal".into(), late.as_ref().err().and_then(|e| custom(e)) == Some(6052), format!("{:?}", late.as_ref().err().and_then(|e| custom(e))), 0));

    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(&base, &g, 0)));
    let base_vault = Pubkey::new_unique();
    rt.store(&base_vault, owned(system_program::ID, vec![]));
    let mut claim_data = Vec::new();
    claim_data.extend_from_slice(&1u64.to_le_bytes());
    claim_data.extend_from_slice(&11_000u64.to_le_bytes());
    let omitted = rt.send(prog, IX_CLAIM, &claim_data, claim_metas(g, prec, config, pool, vault, qsrc, base, quote, base_dest, base_vault, None)).await;
    rows.push(Rec("shared-claim-omitted-reserve".into(), omitted.as_ref().err().and_then(|e| custom(e)) == Some(6050), format!("{:?}", omitted.as_ref().err().and_then(|e| custom(e))), 0));
    let substituted = rt.send(prog, IX_CLAIM, &claim_data, claim_metas(g, prec, config, pool, vault, qsrc, base, quote, base_dest, base_vault, Some(qs))).await;
    rows.push(Rec("shared-claim-substituted-reserve".into(), substituted.as_ref().err().and_then(|e| custom(e)) == Some(6051), format!("{:?}", substituted.as_ref().err().and_then(|e| custom(e))), 0));
    assert_eq!(pots(&rt.acct(&prec).await.unwrap()).0, 0);

    // DAMM mock claim, creator claim, release v2. Manufactured shortfall is a separate row.
    damm_and_creator(rt, prog, &g, &admin, true, rows).await;
    fee_pay(rt, prog, &g, &admin, true, rows).await;
    autolp_and_batch(rt, prog, &g, &admin, true, rows).await;
}

async fn per_cases(rt: &mut Rt, prog: Pubkey, rows: &mut Vec<Rec>) {
    let admin = rt.payer();
    let g = pda(&prog, &[b"global"]);
    // Pay v2, release v1, direct burn, preexisting short, rollback. Vault is the pool ATA.
    let (quote, base, config, pool, prec, gvault, qsrc, _, _, _) = boot_curve(rt, prog, &g, &admin, false).await;
    let pvault = ata(&prec, &quote, &spl_token::ID);
    rt.store(&pvault, owned(spl_token::ID, token(&quote, &prec, 0)));
    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(&base, &g, 0)));
    let base_vault = Pubkey::new_unique();
    rt.store(&base_vault, owned(system_program::ID, vec![]));
    let cursor = pda(&prog, &[b"paycur", pool.as_ref()]);
    let sol_vault = pda(&prog, &[b"sol_vault"]);
    let unwrap = pda(&prog, &[b"unwrap", pool.as_ref()]);
    rt.send(prog, IX_PAY2, &0u64.to_le_bytes(), pay2_accounts(admin, g, prec, pvault, cursor, sol_vault, unwrap, false, None)).await.expect("open per v2");
    let mut claim_data = Vec::new();
    claim_data.extend_from_slice(&1u64.to_le_bytes());
    claim_data.extend_from_slice(&11_000u64.to_le_bytes());
    let bogus = Pubkey::new_unique();
    let bh = rt.ctx.banks_client.get_latest_blockhash().await.unwrap();
    let payer = rt.ctx.payer.insecure_clone();
    let mut d1 = IX_CLAIM.to_vec(); d1.extend_from_slice(&claim_data);
    let ix1 = Instruction { program_id: prog, accounts: claim_metas(g, prec, config, pool, gvault, qsrc, base, quote, base_dest, base_vault, Some(pvault)), data: d1.clone() };
    let ix2 = Instruction { program_id: prog, accounts: claim_metas(g, bogus, config, pool, gvault, qsrc, base, quote, base_dest, base_vault, Some(pvault)), data: d1 };
    let tx = Transaction::new_signed_with_payer(&[ix1, ix2], Some(&payer.pubkey()), &[&payer], bh);
    let rolled = rt.ctx.banks_client.process_transaction_with_metadata(tx).await.unwrap();
    assert!(rolled.result.is_err());
    assert_eq!(amt(&rt.acct(&pvault).await.unwrap()), 0);
    rows.push(Rec("per-two-claim-rollback".into(), true, "pool vault stayed empty".into(), 0));
    rt.send(prog, IX_CLAIM, &claim_data, claim_metas(g, prec, config, pool, gvault, qsrc, base, quote, base_dest, base_vault, Some(pvault))).await.expect("per claim");
    assert_eq!(amt(&rt.acct(&gvault).await.unwrap()), 0);
    assert_eq!(amt(&rt.acct(&pvault).await.unwrap()), 11_000);
    let wallet = Pubkey::new_unique();
    let hb = Pubkey::new_unique();
    let hd = Pubkey::new_unique();
    rt.store(&hb, owned(spl_token::ID, token(&base, &wallet, 100)));
    rt.store(&hd, owned(spl_token::ID, token(&quote, &wallet, 0)));
    let mut pay_accts = pay2_accounts(admin, g, prec, pvault, cursor, sol_vault, unwrap, false, None);
    pay_accts.push(AccountMeta::new_readonly(hb, false));
    pay_accts.push(AccountMeta::new(hd, false));
    let cu = rt.send(prog, IX_PAY2, &100u64.to_le_bytes(), pay_accts).await.expect("per pay v2");
    assert_eq!(amt(&rt.acct(&hd).await.unwrap()), 9_000);
    assert_eq!(amt(&rt.acct(&pvault).await.unwrap()), 2_000);
    assert_eq!(amt(&rt.acct(&gvault).await.unwrap()), 0);
    rows.push(Rec("per-pay-v2-from-pool-vault".into(), true, "paid 9000 from pool vault, global stayed 0".into(), cu));
    let bal = amt(&rt.acct(&pvault).await.unwrap());
    let mut short = rt.acct(&pvault).await.unwrap();
    short.data[64..72].copy_from_slice(&(bal - 1).to_le_bytes());
    rt.store(&pvault, short);
    let op = Pubkey::new_unique();
    rt.store(&op, owned(spl_token::ID, token(&quote, &admin, 0)));
    let rel = rt.send(prog, IX_REL1, &[], vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new(pvault, false),
        AccountMeta::new(op, false), AccountMeta::new_readonly(spl_token::ID, false),
    ]).await;
    rows.push(Rec("per-preexisting-short-release-v1".into(), rel.is_err(), format!("labeled preexisting err={:?}", rel.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(rel.is_err());
    assert_eq!(pots(&rt.acct(&prec).await.unwrap()).1, 2_000);
    let mut restored = rt.acct(&pvault).await.unwrap();
    restored.data[64..72].copy_from_slice(&bal.to_le_bytes());
    rt.store(&pvault, restored);
    let rel_ok = rt.send(prog, IX_REL1, &[], vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new(pvault, false),
        AccountMeta::new(op, false), AccountMeta::new_readonly(spl_token::ID, false),
    ]).await.expect("per release v1");
    assert_eq!(amt(&rt.acct(&op).await.unwrap()), 2_000);
    assert_eq!(amt(&rt.acct(&gvault).await.unwrap()), 0);
    rows.push(Rec("per-release-v1-from-pool-vault".into(), true, "burn left the pool vault".into(), rel_ok));
    // A second curve funds the direct burn. set_burn_mint locks this quote.
    let (q2, b2, c2, p2, r2, gv2, src2, _, _, _) = boot_curve(rt, prog, &g, &admin, false).await;
    let pv2 = ata(&r2, &q2, &spl_token::ID);
    rt.store(&pv2, owned(spl_token::ID, token(&q2, &r2, 0)));
    let bd = Pubkey::new_unique();
    rt.store(&bd, owned(spl_token::ID, token(&b2, &g, 0)));
    let bv = Pubkey::new_unique();
    rt.store(&bv, owned(system_program::ID, vec![]));
    rt.send(prog, IX_CLAIM, &claim_data, claim_metas(g, r2, c2, p2, gv2, src2, b2, q2, bd, bv, Some(pv2))).await.expect("burn claim");
    rt.send(prog, IX_SET_BURN, &q2.to_bytes(), vec![AccountMeta::new_readonly(admin, true), AccountMeta::new(g, false)]).await.expect("per set burn");
    let mut mq = rt.acct(&q2).await.unwrap();
    mq.data[36..44].copy_from_slice(&100_000u64.to_le_bytes());
    rt.store(&q2, mq);
    let bcu = rt.send(prog, IX_BURN, &[], vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(r2, false), AccountMeta::new(q2, false),
        AccountMeta::new(pv2, false), AccountMeta::new_readonly(spl_token::ID, false),
    ]).await.expect("per burn");
    assert_eq!(pots(&rt.acct(&r2).await.unwrap()).1, 0);
    assert_eq!(amt(&rt.acct(&gv2).await.unwrap()), 0);
    assert_eq!(amt(&rt.acct(&pv2).await.unwrap()), 9_000);
    rows.push(Rec("per-direct-burn-from-pool-vault".into(), true, "burned from the pool vault".into(), bcu));

    damm_and_creator(rt, prog, &g, &admin, false, rows).await;
    fee_pay(rt, prog, &g, &admin, false, rows).await;
    autolp_and_batch(rt, prog, &g, &admin, false, rows).await;
}

async fn damm_and_creator(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool, rows: &mut Vec<Rec>) {
    let creator = Pubkey::new_unique();
    let (quote, base, config, pool, prec, vault, _, reserve, _, _) = boot_curve(rt, prog, g, admin, shared).await;
    if shared {
        rt.send(prog, IX_SEAL, &[], vec![
            AccountMeta::new_readonly(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false),
        ]).await.expect("damm seal");
    }
    let mut migrated = dbc_pool(&config, &creator, &base, 1);
    rt.store(&pool, owned(DBC, migrated.split_off(0)));
    rt.send(prog, IX_MARK, &[], vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new(prec, false), AccountMeta::new_readonly(pool, false),
    ]).await.expect("mark");
    let pvault = ata(&prec, &quote, &spl_token::ID);
    if !shared { rt.store(&pvault, owned(spl_token::ID, token(&quote, &prec, 0))); }
    let position = Pubkey::new_unique();
    let tok_a = ata(g, &base, &spl_token::ID);
    let nft = Pubkey::new_unique();
    let ta = Pubkey::new_unique();
    let tb = Pubkey::new_unique();
    let event = Pubkey::new_unique();
    rt.store(&position, owned(DAMM, vec![0; 8]));
    rt.store(&tok_a, owned(spl_token::ID, token(&base, g, 0)));
    rt.store(&nft, owned(spl_token::ID, token(&Pubkey::new_unique(), g, 1)));
    rt.store(&ta, owned(spl_token::ID, token(&base, g, 0)));
    rt.store(&tb, owned(spl_token::ID, token(&quote, g, 100_000)));
    rt.store(&event, owned(system_program::ID, vec![]));
    let owed = pda(&prog, &[b"cown", pool.as_ref()]);
    let mut accts = vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new(prec, false), AccountMeta::new(vault, false),
        AccountMeta::new_readonly(DBC_AUTH, false), AccountMeta::new_readonly(pool, false), AccountMeta::new(position, false),
        AccountMeta::new(tok_a, false), AccountMeta::new(vault, false), AccountMeta::new(ta, false), AccountMeta::new(tb, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(nft, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(event, false), AccountMeta::new_readonly(DAMM, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(pool, false), AccountMeta::new_readonly(config, false), AccountMeta::new(owed, false),
    ];
    if shared { accts.push(AccountMeta::new(reserve, false)); } else { accts.push(AccountMeta::new(pvault, false)); }
    let missing = accts[..accts.len() - 6].to_vec();
    let no_tail = rt.send(prog, IX_DAMM, &[], missing).await;
    rows.push(Rec(format!("{}-damm-omitted-tail", if shared {"shared"} else {"per"}), no_tail.is_err(), "fail closed".into(), 0));
    assert!(no_tail.is_err(), "{no_tail:?}");
    let cu = rt.send(prog, IX_DAMM, &[], accts).await.expect("damm");
    let (holder, burn, _) = pots(&rt.acct(&prec).await.unwrap());
    let owed_amt = u64::from_le_bytes(rt.acct(&owed).await.unwrap().data[40..48].try_into().unwrap());
    assert_eq!((holder, burn, owed_amt), (8_280, 1_840, 880), "8% then 20/110");
    if shared {
        assert_eq!(amt(&rt.acct(&vault).await.unwrap()), 11_000);
    } else {
        assert_eq!(amt(&rt.acct(&vault).await.unwrap()), 0);
        assert_eq!(amt(&rt.acct(&pvault).await.unwrap()), 11_000);
    }
    rows.push(Rec(format!("{}-damm-claim", if shared {"shared"} else {"per"}), true, format!("holder={holder} burn={burn} owed={owed_amt}"), cu));
    let dest = ata(&creator, &quote, &spl_token::ID);
    rt.store(&dest, owned(spl_token::ID, token(&quote, &creator, 0)));
    let pay_vault = if shared { vault } else { pvault };
    let mut creator_ix = vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec, false), AccountMeta::new(owed, false),
        AccountMeta::new_readonly(pool, false), AccountMeta::new(pay_vault, false), AccountMeta::new_readonly(quote, false),
        AccountMeta::new(dest, false), AccountMeta::new_readonly(spl_token::ID, false),
    ];
    if shared { creator_ix.push(AccountMeta::new(reserve, false)); }
    let ccu = rt.send(prog, IX_CREATOR, &[], creator_ix).await.expect("creator");
    assert_eq!(amt(&rt.acct(&dest).await.unwrap()), 880);
    assert_eq!(u64::from_le_bytes(rt.acct(&owed).await.unwrap().data[40..48].try_into().unwrap()), 0);
    rows.push(Rec(format!("{}-creator-claim", if shared {"shared"} else {"per"}), true, "paid 880 from the vault that holds the claim".into(), ccu));
    let op = Pubkey::new_unique();
    rt.store(&op, owned(spl_token::ID, token(&quote, admin, 0)));
    let mut rel = vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new(prec, false), AccountMeta::new(pay_vault, false),
        AccountMeta::new_readonly(quote, false), AccountMeta::new(op, false), AccountMeta::new_readonly(spl_token::ID, false),
    ];
    if shared { rel.push(AccountMeta::new(reserve, false)); }
    let rcu = rt.send(prog, IX_REL2, &[], rel).await.expect("release v2");
    assert_eq!(amt(&rt.acct(&op).await.unwrap()), 1_840);
    assert_eq!(pots(&rt.acct(&prec).await.unwrap()).1, 0);
    rows.push(Rec(format!("{}-release-v2", if shared {"shared"} else {"per"}), true, "burn 1840 released".into(), rcu));
}

async fn fee_pay(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool, rows: &mut Vec<Rec>) {
    let quote = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    rt.store(&quote, owned(T22, fee_mint(6, admin)));
    rt.store(&base, owned(spl_token::ID, mint(6, admin)));
    rt.store(&config, owned(DBC, dbc_cfg(&quote, g)));
    rt.store(&pool, owned(DBC, dbc_pool(&config, admin, &base, 0)));
    let cfg = pda(&prog, &[b"config", quote.as_ref()]);
    rt.send(prog, IX_REG_CFG, &quote.to_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(quote, false),
    ]).await.expect("fee cfg");
    let qs = pda(&prog, &[b"qstat", quote.as_ref()]);
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(prog, IX_SYNC, &sync, vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("fee sync");
    let reserve = pda(&prog, &[b"qres", quote.as_ref()]);
    if shared {
        rt.send(prog, IX_INIT_RES, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
        ]).await.expect("fee res");
    }
    let prec = pda(&prog, &[b"pool", pool.as_ref()]);
    reg_pool(rt, prog, admin, g, &cfg, &prec, &pool, &config, &base, &quote, &qs, if shared { Some(&reserve) } else { None }).await;
    let owner = if shared { *g } else { prec };
    let vault = ata(&owner, &quote, &T22);
    rt.store(&vault, owned(T22, fee_token(&quote, &owner, 0)));
    if shared {
        import(rt, prog, admin, g, &reserve, &prec, &pool).await;
        rt.send(prog, IX_SEAL, &[], vec![
            AccountMeta::new_readonly(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new(vault, false), AccountMeta::new_readonly(T22, false),
        ]).await.expect("fee seal");
    }
    let cursor = pda(&prog, &[b"paycur", pool.as_ref()]);
    let mut open = vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(prec, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(T22, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new(cursor, false),
    ];
    if shared { open.push(AccountMeta::new(reserve, false)); }
    rt.send(prog, IX_PAY3, &0u64.to_le_bytes(), open).await.expect("fee cursor");
    // Seed a solvent pot. The mock claim cannot speak Token-2022, so this book is a fixture.
    // The transfer that follows is the real Token-2022 program.
    let mut pr = rt.acct(&prec).await.unwrap();
    pr.data[137..145].copy_from_slice(&1u64.to_le_bytes());
    pr.data[145..153].copy_from_slice(&10_000u64.to_le_bytes());
    rt.store(&prec, pr);
    let mut v = rt.acct(&vault).await.unwrap();
    v.data[64..72].copy_from_slice(&10_000u64.to_le_bytes());
    rt.store(&vault, v);
    if shared {
        let mut r = rt.acct(&reserve).await.unwrap();
        r.data[56..64].copy_from_slice(&10_000u64.to_le_bytes());
        rt.store(&reserve, r);
    }
    let wallet = Pubkey::new_unique();
    let hb = Pubkey::new_unique();
    let hd = Pubkey::new_unique();
    rt.store(&hb, owned(spl_token::ID, token(&base, &wallet, 100)));
    rt.store(&hd, owned(T22, fee_token(&quote, &wallet, 0)));
    let mut pay = vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(prec, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(T22, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new(cursor, false),
    ];
    if shared { pay.push(AccountMeta::new(reserve, false)); }
    pay.push(AccountMeta::new_readonly(hb, false));
    pay.push(AccountMeta::new(hd, false));
    let cu = rt.send(prog, IX_PAY3, &100u64.to_le_bytes(), pay).await.expect("fee pay");
    let src = amt(&rt.acct(&vault).await.unwrap());
    let dst = amt(&rt.acct(&hd).await.unwrap());
    let pot = pots(&rt.acct(&prec).await.unwrap()).0;
    let ok = src == 0 && dst == 9_900 && pot == 0;
    rows.push(Rec(format!("{}-t22-fee-1pct-pay", if shared {"shared"} else {"per"}), ok, format!("debited=10000 received={dst} fee={}", 10_000 - dst), cu));
    assert!(ok, "source={src} dest={dst} pot={pot}");
}

async fn autolp_and_batch(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool, rows: &mut Vec<Rec>) {
    let base = Pubkey::new_unique();
    rt.store(&base, owned(spl_token::ID, mint(6, admin)));
    let a = prepare_side(rt, prog, g, admin, &base, shared, true).await;
    let b = prepare_side(rt, prog, g, admin, &base, shared, true).await;
    // 21 further pools on quote A, after it is live, so the quote group is past 22 combined.
    let mut extras = Vec::new();
    for _ in 0..21 {
        extras.push(prepare_pool_on(rt, prog, g, admin, &base, &a, shared).await);
    }
    let omitted = prepare_pool_on(rt, prog, g, admin, &base, &a, shared).await;
    claim_side(rt, prog, g, &a, &base).await;
    claim_side(rt, prog, g, &b, &base).await;
    claim_side(rt, prog, g, &omitted, &base).await;
    let auto = pda(&prog, &[b"autolp"]);
    rt.send(prog, IX_INIT_LP, &[], vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(auto, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("init lp");
    let damm_pool = Pubkey::new_unique();
    let position = Pubkey::new_unique();
    let nft_mint = Pubkey::new_unique();
    let nft = Pubkey::new_unique();
    rt.store(&damm_pool, owned(DAMM, vec![0; 32]));
    rt.store(&position, owned(DAMM, vec![0; 8]));
    rt.store(&nft_mint, owned(spl_token::ID, mint(0, admin)));
    rt.store(&nft, owned(spl_token::ID, token(&nft_mint, g, 1)));
    let mut set = Vec::new();
    for k in [&damm_pool, &position, &nft, &a.quote, &b.quote] { set.extend_from_slice(k.as_ref()); }
    rt.send(prog, IX_SET_LP, &set, vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(*admin, true), AccountMeta::new(auto, false),
    ]).await.expect("set lp");
    let ga = ata(g, &a.quote, &spl_token::ID);
    let gb = ata(g, &b.quote, &spl_token::ID);
    if rt.acct(&ga).await.is_none() { rt.store(&ga, owned(spl_token::ID, token(&a.quote, g, 0))); }
    if rt.acct(&gb).await.is_none() { rt.store(&gb, owned(spl_token::ID, token(&b.quote, g, 0))); }
    let va = Pubkey::new_unique();
    let vb = Pubkey::new_unique();
    rt.store(&va, owned(spl_token::ID, token(&a.quote, &DAMM_AUTH, 0)));
    rt.store(&vb, owned(spl_token::ID, token(&b.quote, &DAMM_AUTH, 0)));
    let event = Pubkey::new_unique();
    rt.store(&event, owned(system_program::ID, vec![]));
    let qs_a = pda(&prog, &[b"qstat", a.quote.as_ref()]);
    let qs_b = pda(&prog, &[b"qstat", b.quote.as_ref()]);
    let mut head = vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(auto, false), AccountMeta::new_readonly(*admin, true),
        AccountMeta::new(damm_pool, false), AccountMeta::new(position, false), AccountMeta::new_readonly(nft, false),
        AccountMeta::new(ga, false), AccountMeta::new(gb, false), AccountMeta::new(va, false), AccountMeta::new(vb, false),
        AccountMeta::new_readonly(a.quote, false), AccountMeta::new_readonly(b.quote, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(event, false), AccountMeta::new_readonly(DAMM, false),
        AccountMeta::new_readonly(qs_a, false), AccountMeta::new_readonly(qs_b, false),
    ];
    if shared {
        head.push(AccountMeta::new(a.reserve, false));
        head.push(AccountMeta::new(b.reserve, false));
    }
    let mut dup = head.clone();
    push_pool(&mut dup, prog, &a, shared);
    push_pool(&mut dup, prog, &a, shared);
    let dup_res = rt.send(prog, IX_AUTO, &auto_args(1, 1_000, 1_000), dup).await;
    rows.push(Rec(format!("{}-autolp-duplicate-pool", tag(shared)), dup_res.is_err(), format!("{:?}", dup_res.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(dup_res.is_err(), "{dup_res:?}");
    let mut one = head.clone();
    push_pool(&mut one, prog, &a, shared);
    push_pool(&mut one, prog, &b, shared);
    let before_omit = pots(&rt.acct(&omitted.prec).await.unwrap());
    let before_omit_vault = amt(&rt.acct(&omitted.vault).await.unwrap());
    let (n_exec, bytes_exec) = measure(&one, admin, &IX_AUTO, &auto_args(1, 1_000, 1_000), prog);
    let mut full = head.clone();
    push_pool(&mut full, prog, &a, shared);
    push_pool(&mut full, prog, &omitted, shared);
    for extra in &extras { push_pool(&mut full, prog, extra, shared); }
    push_pool(&mut full, prog, &b, shared);
    let (n_full, bytes_full) = measure(&full, admin, &IX_AUTO, &auto_args(1, 1_000, 1_000), prog);
    let cu = rt.send(prog, IX_AUTO, &auto_args(1, 1_000, 1_000), one).await.expect("autolp");
    let listed_burn = pots(&rt.acct(&a.prec).await.unwrap()).1;
    let omit_after = pots(&rt.acct(&omitted.prec).await.unwrap());
    let omit_vault = amt(&rt.acct(&omitted.vault).await.unwrap());
    // Shared quotes keep every pool in one vault, so that vault falling by the
    // listed budget is the spend. Per-pool vaults are separate: the omitted one stays put.
    let vault_ok = if shared { omit_vault + 1_000 == before_omit_vault } else { omit_vault == before_omit_vault };
    let ok = listed_burn == 1_000 && omit_after == before_omit && vault_ok && n_exec <= 64 && bytes_exec <= 1232 && (n_full > 64 || bytes_full > 1232);
    rows.push(Rec(
        format!("{}-autolp-bounded-past-22", tag(shared)),
        ok,
        format!("executed_accounts={n_exec} executed_bytes={bytes_exec} cu={cu} full_accounts={n_full} full_bytes={bytes_full} omitted_burn={}", omit_after.1),
        cu,
    ));
    assert!(ok, "exec {n_exec}/{bytes_exec} full {n_full}/{bytes_full} listed_burn={listed_burn} omit={omit_after:?}/{omit_vault} was {before_omit:?}/{before_omit_vault}");

    // Reservation across the two quote groups. Exactly once.
    let batch = pda(&prog, &[b"lpbatch", &1u64.to_le_bytes()]);
    let esc_a = ata(&batch, &a.quote, &spl_token::ID);
    let esc_b = ata(&batch, &b.quote, &spl_token::ID);
    rt.store(&esc_a, owned(spl_token::ID, token(&a.quote, &batch, 0)));
    rt.store(&esc_b, owned(spl_token::ID, token(&b.quote, &batch, 0)));
    let open = open_accounts(*admin, *g, batch, auto, &a, &b, esc_a, esc_b, shared);
    let src_a_before = amt(&rt.acct(&a.vault).await.unwrap());
    let src_b_before = amt(&rt.acct(&b.vault).await.unwrap());
    rt.send(prog, IX_OPEN, &1u64.to_le_bytes(), open).await.expect("open batch");
    assert_eq!(amt(&rt.acct(&esc_a).await.unwrap()), 500, "half of the remaining 1000 burn");
    assert_eq!(amt(&rt.acct(&esc_b).await.unwrap()), 500);
    assert_eq!(amt(&rt.acct(&a.vault).await.unwrap()), src_a_before - 500);
    assert_eq!(pots(&rt.acct(&a.prec).await.unwrap()).1, 500);
    let refund = refund_accounts(*admin, *g, batch, &a, &b, esc_a, esc_b, shared);
    rt.send(prog, IX_REFUND, &1u64.to_le_bytes(), refund.clone()).await.expect("refund");
    assert_eq!(amt(&rt.acct(&esc_a).await.unwrap()), 0);
    assert_eq!(amt(&rt.acct(&a.vault).await.unwrap()), src_a_before);
    assert_eq!(amt(&rt.acct(&b.vault).await.unwrap()), src_b_before);
    assert_eq!(pots(&rt.acct(&a.prec).await.unwrap()).1, 1_000);
    let second = rt.send(prog, IX_REFUND, &1u64.to_le_bytes(), refund).await;
    let closed = if shared { 6054 } else { 6051 };
    rows.push(Rec(format!("{}-lp-refund-once", tag(shared)), second.as_ref().err().and_then(|e| custom(e)) == Some(closed), format!("{:?}", second.as_ref().err().and_then(|e| custom(e))), 0));
    assert_eq!(second.as_ref().err().and_then(|e| custom(e)), Some(closed), "{second:?}");
    let batch2 = pda(&prog, &[b"lpbatch", &2u64.to_le_bytes()]);
    let e2a = ata(&batch2, &a.quote, &spl_token::ID);
    let e2b = ata(&batch2, &b.quote, &spl_token::ID);
    rt.store(&e2a, owned(spl_token::ID, token(&a.quote, &batch2, 0)));
    rt.store(&e2b, owned(spl_token::ID, token(&b.quote, &batch2, 0)));
    rt.send(prog, IX_OPEN, &2u64.to_le_bytes(), open_accounts(*admin, *g, batch2, auto, &a, &b, e2a, e2b, shared)).await.expect("open 2");
    let dest_a = Pubkey::new_unique();
    let dest_b = Pubkey::new_unique();
    rt.store(&dest_a, owned(spl_token::ID, token(&a.quote, &DAMM_AUTH, 0)));
    rt.store(&dest_b, owned(spl_token::ID, token(&b.quote, &DAMM_AUTH, 0)));
    let consume = vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(batch2, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new(e2a, false), AccountMeta::new(dest_a, false), AccountMeta::new(e2b, false), AccountMeta::new(dest_b, false),
    ];
    rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume.clone()).await.expect("consume");
    assert_eq!(amt(&rt.acct(&dest_a).await.unwrap()), 500);
    assert_eq!(amt(&rt.acct(&dest_b).await.unwrap()), 500);
    assert_eq!(amt(&rt.acct(&e2a).await.unwrap()), 0);
    let again = rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume).await;
    rows.push(Rec(format!("{}-lp-consume-once-two-quotes", tag(shared)), again.is_err(), "second consume rejected, both quote groups conserved".into(), 0));
    assert!(again.is_err(), "{again:?}");
    let omit_final = pots(&rt.acct(&omitted.prec).await.unwrap());
    assert_eq!(omit_final, before_omit, "omitted pool was not in the batch");
}

struct Side { quote: Pubkey, reserve: Pubkey, pool: Pubkey, prec: Pubkey, vault: Pubkey, config: Pubkey, qsrc: Pubkey }

async fn prepare_side(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, base: &Pubkey, shared: bool, claim_ready: bool) -> Side {
    let quote = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, mint(9, admin)));
    let config = Pubkey::new_unique();
    rt.store(&config, owned(DBC, dbc_cfg(&quote, g)));
    let cfg = pda(&prog, &[b"config", quote.as_ref()]);
    rt.send(prog, IX_REG_CFG, &quote.to_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(quote, false),
    ]).await.expect("side cfg");
    let qs = pda(&prog, &[b"qstat", quote.as_ref()]);
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(prog, IX_SYNC, &sync, vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("side sync");
    let reserve = pda(&prog, &[b"qres", quote.as_ref()]);
    if shared {
        rt.send(prog, IX_INIT_RES, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
        ]).await.expect("side res");
    }
    let side_pool = prepare_pool_raw(rt, prog, g, admin, base, &quote, &config, &cfg, &qs, &reserve, shared).await;
    let gvault = ata(g, &quote, &spl_token::ID);
    rt.store(&gvault, owned(spl_token::ID, token(&quote, g, 0)));
    if shared {
        import(rt, prog, admin, g, &reserve, &side_pool.prec, &side_pool.pool).await;
        rt.send(prog, IX_SEAL, &[], vec![
            AccountMeta::new_readonly(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new(gvault, false), AccountMeta::new_readonly(spl_token::ID, false),
        ]).await.expect("side seal");
    }
    let qsrc = Pubkey::new_unique();
    rt.store(&qsrc, owned(spl_token::ID, token(&quote, g, 500_000)));
    let _ = claim_ready;
    Side { quote, reserve, pool: side_pool.pool, prec: side_pool.prec, vault: if shared { gvault } else { side_pool.vault }, config, qsrc }
}

async fn prepare_pool_on(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, base: &Pubkey, side: &Side, shared: bool) -> Side {
    let cfg = pda(&prog, &[b"config", side.quote.as_ref()]);
    let qs = pda(&prog, &[b"qstat", side.quote.as_ref()]);
    let made = prepare_pool_raw(rt, prog, g, admin, base, &side.quote, &side.config, &cfg, &qs, &side.reserve, shared).await;
    Side { quote: side.quote, reserve: side.reserve, pool: made.pool, prec: made.prec, vault: made.vault, config: side.config, qsrc: side.qsrc }
}

struct Made { pool: Pubkey, prec: Pubkey, vault: Pubkey }

async fn prepare_pool_raw(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, base: &Pubkey, quote: &Pubkey, config: &Pubkey, cfg: &Pubkey, qs: &Pubkey, reserve: &Pubkey, shared: bool) -> Made {
    let pool = Pubkey::new_unique();
    rt.store(&pool, owned(DBC, dbc_pool(config, admin, base, 0)));
    let prec = pda(&prog, &[b"pool", pool.as_ref()]);
    reg_pool(rt, prog, admin, g, cfg, &prec, &pool, config, base, quote, qs, if shared { Some(reserve) } else { None }).await;
    let vault = if shared { ata(g, quote, &spl_token::ID) } else { ata(&prec, quote, &spl_token::ID) };
    if !shared { rt.store(&vault, owned(spl_token::ID, token(quote, &prec, 0))); }
    Made { pool, prec, vault }
}

async fn claim_side(rt: &mut Rt, prog: Pubkey, g: &Pubkey, side: &Side, base: &Pubkey) {
    let gvault = ata(g, &side.quote, &spl_token::ID);
    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(base, g, 0)));
    let base_vault = Pubkey::new_unique();
    rt.store(&base_vault, owned(system_program::ID, vec![]));
    let mut data = Vec::new();
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&11_000u64.to_le_bytes());
    let extra = if side.vault == gvault { None } else { Some(side.vault) };
    // Shared claim's extra is the reserve. Per-pool's extra is the pool vault.
    let tail = if extra.is_none() { Some(side.reserve) } else { extra };
    rt.send(prog, IX_CLAIM, &data, claim_metas(*g, side.prec, side.config, side.pool, gvault, side.qsrc, *base, side.quote, base_dest, base_vault, tail)).await.expect("claim side");
}

fn push_pool(metas: &mut Vec<AccountMeta>, prog: Pubkey, side: &Side, shared: bool) {
    metas.push(AccountMeta::new(side.prec, false));
    if !shared { metas.push(AccountMeta::new(side.vault, false)); }
    let owed = pda(&prog, &[b"cown", side.pool.as_ref()]);
    metas.push(AccountMeta::new_readonly(owed, false));
}

fn open_accounts(admin: Pubkey, g: Pubkey, batch: Pubkey, auto: Pubkey, a: &Side, b: &Side, ea: Pubkey, eb: Pubkey, shared: bool) -> Vec<AccountMeta> {
    let mut v = vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(batch, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(auto, false), AccountMeta::new_readonly(spl_token::ID, false),
    ];
    for (s, e) in [(a, ea), (b, eb)] {
        v.push(AccountMeta::new(s.prec, false));
        v.push(AccountMeta::new(s.vault, false));
        v.push(AccountMeta::new_readonly(s.quote, false));
        v.push(AccountMeta::new(e, false));
        if shared { v.push(AccountMeta::new(s.reserve, false)); }
    }
    v
}
fn refund_accounts(admin: Pubkey, g: Pubkey, batch: Pubkey, a: &Side, b: &Side, ea: Pubkey, eb: Pubkey, shared: bool) -> Vec<AccountMeta> {
    let mut v = vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(batch, false), AccountMeta::new_readonly(spl_token::ID, false),
    ];
    for (s, e) in [(a, ea), (b, eb)] {
        v.push(AccountMeta::new(e, false));
        v.push(AccountMeta::new(s.vault, false));
        v.push(AccountMeta::new(s.prec, false));
        if shared { v.push(AccountMeta::new(s.reserve, false)); }
    }
    v
}

fn auto_args(liq: u128, a: u64, b: u64) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&liq.to_le_bytes());
    d.extend_from_slice(&a.to_le_bytes());
    d.extend_from_slice(&b.to_le_bytes());
    d
}
fn measure(accounts: &[AccountMeta], payer: &Pubkey, disc: &[u8; 8], data: &[u8], prog: Pubkey) -> (usize, usize) {
    let mut buf = disc.to_vec();
    buf.extend_from_slice(data);
    let ix = Instruction { program_id: prog, accounts: accounts.to_vec(), data: buf };
    let msg = Message::new(&[ix], Some(payer));
    let bytes = bincode::serialize(&msg).unwrap().len();
    (msg.account_keys.len(), bytes)
}
fn tag(shared: bool) -> &'static str { if shared { "shared" } else { "per" } }

fn claim_metas(g: Pubkey, prec: Pubkey, config: Pubkey, pool: Pubkey, vault: Pubkey, qsrc: Pubkey, base: Pubkey, quote: Pubkey, base_dest: Pubkey, base_vault: Pubkey, extra: Option<Pubkey>) -> Vec<AccountMeta> {
    let mut v = vec![
        AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false), AccountMeta::new_readonly(DBC_AUTH, false),
        AccountMeta::new_readonly(config, false), AccountMeta::new(pool, false), AccountMeta::new(base_dest, false),
        AccountMeta::new(vault, false), AccountMeta::new(base_vault, false), AccountMeta::new(qsrc, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(quote, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false), AccountMeta::new_readonly(DBC, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];
    if let Some(x) = extra { v.push(AccountMeta::new(x, false)); }
    v
}
fn pay2_accounts(admin: Pubkey, g: Pubkey, prec: Pubkey, vault: Pubkey, cursor: Pubkey, sol: Pubkey, unwrap: Pubkey, shared: bool, reserve: Option<Pubkey>) -> Vec<AccountMeta> {
    let mut v = vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(prec, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(cursor, false), AccountMeta::new(sol, false), AccountMeta::new(unwrap, false), AccountMeta::new_readonly(NATIVE, false),
    ];
    if shared { v.push(AccountMeta::new(reserve.unwrap(), false)); }
    v
}

async fn boot_shared(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, _shared: bool) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    boot_curve(rt, prog, g, admin, true).await
}
async fn boot_curve(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    let quote = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, mint(9, admin)));
    rt.store(&base, owned(spl_token::ID, mint(6, admin)));
    rt.store(&config, owned(DBC, dbc_cfg(&quote, g)));
    rt.store(&pool, owned(DBC, dbc_pool(&config, admin, &base, 0)));
    let vault = ata(g, &quote, &spl_token::ID);
    rt.store(&vault, owned(spl_token::ID, token(&quote, g, 0)));
    let qsrc = Pubkey::new_unique();
    rt.store(&qsrc, owned(spl_token::ID, token(&quote, g, 100_000)));
    let cfg = pda(&prog, &[b"config", quote.as_ref()]);
    rt.send(prog, IX_REG_CFG, &quote.to_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(quote, false),
    ]).await.expect("cfg");
    let qs = pda(&prog, &[b"qstat", quote.as_ref()]);
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(prog, IX_SYNC, &sync, vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("sync");
    let reserve = pda(&prog, &[b"qres", quote.as_ref()]);
    if shared {
        rt.send(prog, IX_INIT_RES, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
        ]).await.expect("res");
    }
    let prec = pda(&prog, &[b"pool", pool.as_ref()]);
    reg_pool(rt, prog, admin, g, &cfg, &prec, &pool, &config, &base, &quote, &qs, if shared { Some(&reserve) } else { None }).await;
    if shared { import(rt, prog, admin, g, &reserve, &prec, &pool).await; }
    (quote, base, config, pool, prec, vault, qsrc, reserve, qs, cfg)
}
async fn reg_pool(rt: &mut Rt, prog: Pubkey, admin: &Pubkey, g: &Pubkey, cfg: &Pubkey, prec: &Pubkey, pool: &Pubkey, config: &Pubkey, base: &Pubkey, quote: &Pubkey, qs: &Pubkey, reserve: Option<&Pubkey>) {
    let mut rd = Vec::new();
    rd.extend_from_slice(pool.as_ref()); rd.extend_from_slice(base.as_ref()); rd.extend_from_slice(quote.as_ref());
    let mut accts = vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*cfg, false), AccountMeta::new(*prec, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(*g, false),
        AccountMeta::new_readonly(*pool, false), AccountMeta::new_readonly(*config, false),
        AccountMeta::new_readonly(*quote, false), AccountMeta::new(*qs, false),
    ];
    if let Some(r) = reserve { accts.push(AccountMeta::new(*r, false)); }
    rt.send(prog, IX_REG_POOL, &rd, accts).await.expect("reg");
}
async fn import(rt: &mut Rt, prog: Pubkey, admin: &Pubkey, g: &Pubkey, reserve: &Pubkey, prec: &Pubkey, pool: &Pubkey) {
    import_result(rt, prog, admin, g, reserve, prec, pool).await.expect("import");
}
async fn import_result(rt: &mut Rt, prog: Pubkey, admin: &Pubkey, g: &Pubkey, reserve: &Pubkey, prec: &Pubkey, pool: &Pubkey) -> Result<u64, String> {
    let marker = pda(&prog, &[b"qimp", pool.as_ref()]);
    let owed = Pubkey::new_unique();
    rt.send(prog, IX_IMPORT, &[], vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(*reserve, false),
        AccountMeta::new_readonly(*prec, false), AccountMeta::new_readonly(owed, false),
        AccountMeta::new(marker, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await
}
