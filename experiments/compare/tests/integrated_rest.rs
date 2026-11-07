//! Remaining integrated paths: DAMM, creator, pay v2, burn, releases,
//! bounded auto-LP past 22 pools, reservation consume/refund, fee transfer.

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
const IX_CLOSE: [u8; 8] = [55, 254, 149, 79, 122, 41, 15, 20];
const IX_CLOSE_BATCH: [u8; 8] = [77, 158, 87, 217, 88, 244, 144, 86];
const IX_REVERSE: [u8; 8] = [135, 74, 192, 108, 126, 50, 79, 186];
const IX_IMPORT_OLD: [u8; 8] = [168, 188, 249, 36, 4, 67, 3, 37];
const IX_MIGRATE: [u8; 8] = [7, 158, 186, 54, 226, 94, 254, 47];
const IX_PAUSE: [u8; 8] = [91, 60, 125, 192, 176, 225, 166, 218];
const MEMO: Pubkey = solana_pubkey::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
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
    async fn send_with(&mut self, program: Pubkey, disc: [u8; 8], data: &[u8], accounts: Vec<AccountMeta>, extra: &[&Keypair]) -> Result<u64, String> {
        self.slot += 1;
        let _ = self.ctx.warp_to_slot(self.slot);
        let bh = self.ctx.banks_client.get_latest_blockhash().await.unwrap();
        let payer = self.ctx.payer.insecure_clone();
        let mut buf = disc.to_vec();
        buf.extend_from_slice(data);
        let ix = Instruction { program_id: program, accounts, data: buf };
        let mut signers: Vec<&Keypair> = vec![&payer];
        signers.extend(extra);
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &signers, bh);
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
    let count_before = u64::from_le_bytes(rt.acct(&qs).await.unwrap().data[40..48].try_into().unwrap());
    let rh_before = u64::from_le_bytes(rt.acct(&reserve).await.unwrap().data[56..64].try_into().unwrap());
    let pool3 = Pubkey::new_unique();
    rt.store(&pool3, owned(DBC, dbc_pool(&config, &admin, &base, 0)));
    let prec3 = pda(&prog, &[b"pool", pool3.as_ref()]);
    reg_pool(rt, prog, &admin, &g, &cfg, &prec3, &pool3, &config, &base, &quote, &qs, Some(&reserve)).await;
    let count_after = u64::from_le_bytes(rt.acct(&qs).await.unwrap().data[40..48].try_into().unwrap());
    let marker3 = pda(&prog, &[b"qimp", pool3.as_ref()]);
    let base_dest3 = Pubkey::new_unique();
    rt.store(&base_dest3, owned(spl_token::ID, token(&base, &g, 0)));
    let base_vault3 = Pubkey::new_unique();
    rt.store(&base_vault3, owned(system_program::ID, vec![]));
    let mut claim3_data = Vec::new();
    claim3_data.extend_from_slice(&1u64.to_le_bytes());
    claim3_data.extend_from_slice(&11_000u64.to_le_bytes());
    rt.send(prog, IX_CLAIM, &claim3_data, claim_metas(g, prec3, config, pool3, vault, qsrc, base, quote, base_dest3, base_vault3, Some(reserve))).await.expect("claim new pool after seal");
    let rh_after = u64::from_le_bytes(rt.acct(&reserve).await.unwrap().data[56..64].try_into().unwrap());
    let new_pots = pots(&rt.acct(&prec3).await.unwrap());
    let sealed_ok = count_after == count_before + 1 && rt.acct(&marker3).await.is_none() && new_pots.0 > 0 && rh_after == rh_before + new_pots.0 && rt.acct(&reserve).await.unwrap().data[80] == 2;
    rows.push(Rec("shared-new-pool-after-seal-no-resync-no-import".into(), sealed_ok, format!("qstat {count_before}->{count_after} holder={} reserve_holder={rh_before}->{rh_after}", new_pots.0), 0));
    assert!(sealed_ok, "qstat {count_before}->{count_after} pots={new_pots:?} reserve {rh_before}->{rh_after}");

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
    fee_mint_claim(rt, prog, &g, &admin, true, rows).await;
    two_pages(rt, prog, &g, &admin, true, rows).await;
    wsol_unwrap(rt, prog, &g, &admin, true, rows).await;
    old_layout(rt, prog, &g, &admin, true, rows).await;
    caller_matrix(rt, prog, &g, &admin, true, rows).await;
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
    fee_mint_claim(rt, prog, &g, &admin, false, rows).await;
    two_pages(rt, prog, &g, &admin, false, rows).await;
    wsol_unwrap(rt, prog, &g, &admin, false, rows).await;
    old_layout(rt, prog, &g, &admin, false, rows).await;
    caller_matrix(rt, prog, &g, &admin, false, rows).await;
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
    rt.store(&damm_pool, owned(DAMM, vec![0; 64]));
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
    let mut pool_bytes = vec![0u8; 64];
    pool_bytes[..32].copy_from_slice(va.as_ref());
    pool_bytes[32..64].copy_from_slice(vb.as_ref());
    rt.store(&damm_pool, owned(DAMM, pool_bytes));
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
    let exec_m = measure(rt, &one, admin, &IX_AUTO, &auto_args(1, 1_000, 1_000), prog).await;
    let mut full = head.clone();
    push_pool(&mut full, prog, &a, shared);
    push_pool(&mut full, prog, &omitted, shared);
    for extra in &extras { push_pool(&mut full, prog, extra, shared); }
    push_pool(&mut full, prog, &b, shared);
    let full_m = measure(rt, &full, admin, &IX_AUTO, &auto_args(1, 1_000, 1_000), prog).await;
    let cu = rt.send(prog, IX_AUTO, &auto_args(1, 1_000, 1_000), one).await.expect("autolp");
    let listed_burn = pots(&rt.acct(&a.prec).await.unwrap()).1;
    let omit_after = pots(&rt.acct(&omitted.prec).await.unwrap());
    let omit_vault = amt(&rt.acct(&omitted.vault).await.unwrap());
    // Shared quotes keep every pool in one vault, so that vault falling by the
    // listed budget is the spend. Per-pool vaults are separate: the omitted one stays put.
    let vault_ok = if shared { omit_vault + 1_000 == before_omit_vault } else { omit_vault == before_omit_vault };
    let ok = listed_burn == 1_000 && omit_after == before_omit && vault_ok && exec_m.locks <= 64 && exec_m.signed_len <= 1232 && (full_m.locks > 64 || full_m.signed_len > 1232);
    rows.push(Rec(
        format!("{}-autolp-mock-bounded-past-22", tag(shared)),
        ok,
        format!("locks={} sigs={} message_bytes={} signed_bytes={} cu={cu} unsent_full_locks={} unsent_full_signed_bytes={} omitted_burn={} omitted_holder={}", exec_m.locks, exec_m.sigs, exec_m.message_len, exec_m.signed_len, full_m.locks, full_m.signed_len, omit_after.1, omit_after.0),
        cu,
    ));
    assert!(ok, "exec locks={} signed={} full locks={} signed={} listed_burn={listed_burn} omit={omit_after:?}/{omit_vault} was {before_omit:?}/{before_omit_vault}", exec_m.locks, exec_m.signed_len, full_m.locks, full_m.signed_len);

    // Reservation across the two quote groups. Exactly once.
    let batch = pda(&prog, &[b"lpbatch", &1u64.to_le_bytes()]);
    let esc_a = ata(&batch, &a.quote, &spl_token::ID);
    let esc_b = ata(&batch, &b.quote, &spl_token::ID);
    rt.store(&esc_a, owned(spl_token::ID, token(&a.quote, &batch, 0)));
    rt.store(&esc_b, owned(spl_token::ID, token(&b.quote, &batch, 0)));
    let open = open_accounts(*admin, *g, batch, auto, damm_pool, &a, &b, esc_a, esc_b, va, vb, shared, prog);
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
    rt.send(prog, IX_OPEN, &2u64.to_le_bytes(), open_accounts(*admin, *g, batch2, auto, damm_pool, &a, &b, e2a, e2b, va, vb, shared, prog)).await.expect("open 2");
    let replay = rt.send(prog, IX_OPEN, &2u64.to_le_bytes(), open_accounts(*admin, *g, batch2, auto, damm_pool, &a, &b, e2a, e2b, va, vb, shared, prog)).await;
    rows.push(Rec(format!("{}-reservation-replay-open", tag(shared)), replay.is_err(), format!("{:?}", replay.as_ref().err().and_then(|e| custom(e))), 0));
    let snap_esc = amt(&rt.acct(&e2a).await.unwrap());
    let snap_burn = pots(&rt.acct(&a.prec).await.unwrap()).1;
    let snap_holder = pots(&rt.acct(&a.prec).await.unwrap()).0;
    let snap_va = amt(&rt.acct(&va).await.unwrap());
    let snap_vb = amt(&rt.acct(&vb).await.unwrap());
    let wallet_dest = Pubkey::new_unique();
    rt.store(&wallet_dest, owned(spl_token::ID, token(&a.quote, admin, 0)));
    let unbound = Pubkey::new_unique();
    rt.store(&unbound, owned(spl_token::ID, token(&a.quote, &DAMM_AUTH, 0)));
    let wrong_pool = Pubkey::new_unique();
    let mut wrong_bytes = vec![0u8; 64];
    wrong_bytes[..32].copy_from_slice(unbound.as_ref());
    rt.store(&wrong_pool, owned(DAMM, wrong_bytes));
    let wrong_escrow = Pubkey::new_unique();
    rt.store(&wrong_escrow, owned(spl_token::ID, token(&a.quote, &batch2, snap_esc)));
    fn consume_to(admin: Pubkey, g: Pubkey, batch: Pubkey, auto: Pubkey, pool: Pubkey, ea: Pubkey, da: Pubkey, eb: Pubkey, db: Pubkey) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(batch, false), AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(auto, false), AccountMeta::new_readonly(pool, false),
            AccountMeta::new(ea, false), AccountMeta::new(da, false), AccountMeta::new(eb, false), AccountMeta::new(db, false),
        ]
    }
    let reject_user = rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume_to(*admin, *g, batch2, auto, damm_pool, e2a, wallet_dest, e2b, vb)).await;
    let reject_same_auth = rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume_to(*admin, *g, batch2, auto, damm_pool, e2a, unbound, e2b, vb)).await;
    let reject_quote = rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume_to(*admin, *g, batch2, auto, damm_pool, e2a, vb, e2b, va)).await;
    let reject_pool = rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume_to(*admin, *g, batch2, auto, wrong_pool, e2a, unbound, e2b, vb)).await;
    let reject_escrow = rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume_to(*admin, *g, batch2, auto, damm_pool, wrong_escrow, va, e2b, vb)).await;
    let unchanged = amt(&rt.acct(&e2a).await.unwrap()) == snap_esc
        && pots(&rt.acct(&a.prec).await.unwrap()).1 == snap_burn
        && pots(&rt.acct(&a.prec).await.unwrap()).0 == snap_holder
        && amt(&rt.acct(&unbound).await.unwrap()) == 0
        && amt(&rt.acct(&wallet_dest).await.unwrap()) == 0
        && amt(&rt.acct(&va).await.unwrap()) == snap_va
        && amt(&rt.acct(&vb).await.unwrap()) == snap_vb;
    rows.push(Rec(
        format!("{}-reservation-binding-rejections", tag(shared)),
        reject_user.is_err() && reject_same_auth.is_err() && reject_quote.is_err() && reject_pool.is_err() && reject_escrow.is_err() && unchanged,
        format!("user={:?} same_authority={:?} wrong_quote={:?} wrong_pool={:?} wrong_escrow={:?} unchanged={unchanged}",
            reject_user.as_ref().err().and_then(|e| custom(e)),
            reject_same_auth.as_ref().err().and_then(|e| custom(e)),
            reject_quote.as_ref().err().and_then(|e| custom(e)),
            reject_pool.as_ref().err().and_then(|e| custom(e)),
            reject_escrow.as_ref().err().and_then(|e| custom(e))),
        0,
    ));
    assert!(unchanged, "a rejected consume moved funds");
    let consume = consume_to(*admin, *g, batch2, auto, damm_pool, e2a, va, e2b, vb);
    rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume.clone()).await.expect("configured consume");
    assert_eq!(amt(&rt.acct(&va).await.unwrap()), snap_va + 500);
    assert_eq!(amt(&rt.acct(&vb).await.unwrap()), snap_vb + 500);
    assert_eq!(amt(&rt.acct(&e2a).await.unwrap()), 0);
    rows.push(Rec(format!("{}-reservation-configured-dest", tag(shared)), true, "configured vaults received 500 each. Escrow consume is not add_liquidity and is not a lock".into(), 0));
    let again = rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume).await;
    rows.push(Rec(format!("{}-reservation-consume-once-not-lp", tag(shared)), again.is_err() && amt(&rt.acct(&va).await.unwrap()) == snap_va + 500, "second consume rejected. Balances stayed at the first consume. This is not add_liquidity and is not a lock".into(), 0));
    assert!(again.is_err(), "{again:?}");
    let holder_a = pots(&rt.acct(&a.prec).await.unwrap()).0;
    let holder_b = pots(&rt.acct(&b.prec).await.unwrap()).0;
    rows.push(Rec(format!("{}-reservation-holders-unchanged", tag(shared)), holder_a == 9_000 && holder_b == 9_000, format!("holder_a={holder_a} holder_b={holder_b}"), 0));
    assert_eq!((holder_a, holder_b), (9_000, 9_000));
    let omit_final = pots(&rt.acct(&omitted.prec).await.unwrap());
    assert_eq!(omit_final, before_omit, "omitted pool was not in the batch");

    let batch_len = rt.acct(&batch2).await.unwrap().data.len();
    let escrow_len = rt.acct(&e2a).await.unwrap().data.len();
    let escrow_owner = rt.acct(&e2a).await.unwrap().owner;
    rows.push(Rec(format!("{}-escrow-size", tag(shared)), batch_len == 564 && escrow_len == 165 && escrow_owner == spl_token::ID, format!("batch_bytes={batch_len} escrow_bytes={escrow_len} token_program={escrow_owner}"), 0));
    let batch11 = pda(&prog, &[b"lpbatch", &11u64.to_le_bytes()]);
    let e11a = ata(&batch11, &a.quote, &spl_token::ID);
    let e11b = ata(&batch11, &b.quote, &spl_token::ID);
    rt.store(&e11a, owned(spl_token::ID, token(&a.quote, &batch11, 0)));
    rt.store(&e11b, owned(spl_token::ID, token(&b.quote, &batch11, 0)));
    rt.send(prog, IX_OPEN, &11u64.to_le_bytes(), open_accounts(*admin, *g, batch11, auto, damm_pool, &a, &b, e11a, e11b, va, vb, shared, prog)).await.expect("open funded batch");
    let funded = amt(&rt.acct(&e11a).await.unwrap());
    let close_funded = rt.send(prog, IX_CLOSE, &11u64.to_le_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(batch11, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(e11a, false),
    ]).await;
    rows.push(Rec(format!("{}-close-rejects-funded-escrow", tag(shared)), close_funded.is_err() && amt(&rt.acct(&e11a).await.unwrap()) == funded, format!("err={:?} still={funded}", close_funded.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(close_funded.is_err() && amt(&rt.acct(&e11a).await.unwrap()) == funded, "{close_funded:?}");
    rt.send(prog, IX_REFUND, &11u64.to_le_bytes(), refund_accounts(*admin, *g, batch11, &a, &b, e11a, e11b, shared)).await.expect("refund funded batch");
    let stranger = Keypair::new();
    rt.store(&stranger.pubkey(), owned(system_program::ID, vec![]));
    let stranger_close = rt.send_with(prog, IX_CLOSE, &2u64.to_le_bytes(), vec![
        AccountMeta::new(stranger.pubkey(), true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(batch2, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(e2a, false),
    ], &[&stranger]).await;
    let closed = rt.send(prog, IX_CLOSE, &2u64.to_le_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(batch2, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(e2a, false),
    ]).await;
    let again_close = rt.send(prog, IX_CLOSE, &2u64.to_le_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(batch2, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(e2a, false),
    ]).await;
    let consume_after = rt.send(prog, IX_CONSUME, &2u64.to_le_bytes(), consume_to(*admin, *g, batch2, auto, damm_pool, e2a, va, e2b, vb)).await;
    let batch_still = rt.acct(&batch2).await.unwrap();
    let ok = stranger_close.is_err() && closed.is_ok() && again_close.is_err() && consume_after.is_err()
        && batch_still.data.len() == 564 && batch_still.owner == prog && rt.acct(&e2a).await.is_none();
    rows.push(Rec(format!("{}-escrow-close-keeps-batch", tag(shared)), ok, format!("stranger={:?} close={:?} replay={:?} consume_after={:?}", stranger_close.as_ref().err().and_then(|e| custom(e)), closed.as_ref().err(), again_close.as_ref().err().and_then(|e| custom(e)), consume_after.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(ok, "escrow close did not keep the batch tombstone");
    let mark = pda(&prog, &[b"lpnonce"]);
    let funded_close = rt.send(prog, IX_CLOSE_BATCH, &11u64.to_le_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(batch11, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(mark, false),
        AccountMeta::new(e11a, false), AccountMeta::new(e11b, false),
    ]).await;
    // Batch 11 was refunded, so its escrows are empty and its state is terminal. The attempt above
    // used the still-funded window only if refund had not happened. It was refunded, so close it for real below
    // only when the funded rejection did not apply. A separate open is used for the funded rejection.
    let _ = funded_close;
    let batch12 = pda(&prog, &[b"lpbatch", &12u64.to_le_bytes()]);
    let e12a = ata(&batch12, &a.quote, &spl_token::ID);
    let e12b = ata(&batch12, &b.quote, &spl_token::ID);
    rt.store(&e12a, owned(spl_token::ID, token(&a.quote, &batch12, 0)));
    rt.store(&e12b, owned(spl_token::ID, token(&b.quote, &batch12, 0)));
    rt.send(prog, IX_OPEN, &12u64.to_le_bytes(), open_accounts(*admin, *g, batch12, auto, damm_pool, &a, &b, e12a, e12b, va, vb, shared, prog)).await.expect("open 12");
    let reject_funded = rt.send(prog, IX_CLOSE_BATCH, &12u64.to_le_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(batch12, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(mark, false),
        AccountMeta::new(e12a, false), AccountMeta::new(e12b, false),
    ]).await;
    let still_funded = amt(&rt.acct(&e12a).await.unwrap());
    rows.push(Rec(format!("{}-close-batch-rejects-funded", tag(shared)), reject_funded.is_err() && still_funded > 0 && rt.acct(&batch12).await.unwrap().data.len() == 564, format!("err={:?} escrow={still_funded}", reject_funded.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(reject_funded.is_err() && still_funded > 0, "{reject_funded:?}");
    rt.send(prog, IX_REFUND, &12u64.to_le_bytes(), refund_accounts(*admin, *g, batch12, &a, &b, e12a, e12b, shared)).await.expect("refund 12");
    rt.send(prog, IX_CLOSE_BATCH, &12u64.to_le_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(batch12, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(mark, false),
        AccountMeta::new(e12a, false), AccountMeta::new(e12b, false),
    ]).await.expect("close batch 12");
    let watermark = rt.acct(&mark).await.unwrap();
    let high = u64::from_le_bytes(watermark.data[8..16].try_into().unwrap());
    let replay_closed = rt.send(prog, IX_OPEN, &12u64.to_le_bytes(), open_accounts(*admin, *g, batch12, auto, damm_pool, &a, &b, e12a, e12b, va, vb, shared, prog)).await;
    let batch_gone = rt.acct(&batch12).await.map(|a| a.data.len()).unwrap_or(0) == 0;
    let ok_life = watermark.data.len() == 17 && high >= 12 && replay_closed.is_err() && batch_gone;
    rows.push(Rec(format!("{}-batch-watermark-reusable", tag(shared)), ok_life, format!("watermark_bytes={} high={high} replay={:?} batch_bytes_left={}", watermark.data.len(), replay_closed.as_ref().err().and_then(|e| custom(e)), rt.acct(&batch12).await.map(|a| a.data.len()).unwrap_or(0)), 0));
    assert!(ok_life, "{replay_closed:?} high={high}");
    let batch13 = pda(&prog, &[b"lpbatch", &13u64.to_le_bytes()]);
    let batch15 = pda(&prog, &[b"lpbatch", &15u64.to_le_bytes()]);
    let e13a = ata(&batch13, &a.quote, &spl_token::ID);
    let e13b = ata(&batch13, &b.quote, &spl_token::ID);
    let e15a = ata(&batch15, &a.quote, &spl_token::ID);
    let e15b = ata(&batch15, &b.quote, &spl_token::ID);
    for (k, q, owner) in [(&e13a, &a.quote, &batch13), (&e13b, &b.quote, &batch13), (&e15a, &a.quote, &batch15), (&e15b, &b.quote, &batch15)] {
        rt.store(k, owned(spl_token::ID, token(q, owner, 0)));
    }
    rt.send(prog, IX_OPEN, &13u64.to_le_bytes(), open_accounts(*admin, *g, batch13, auto, damm_pool, &a, &b, e13a, e13b, va, vb, shared, prog)).await.expect("open 13");
    rt.send(prog, IX_OPEN, &15u64.to_le_bytes(), open_accounts(*admin, *g, batch15, auto, damm_pool, &a, &b, e15a, e15b, va, vb, shared, prog)).await.expect("open 15");
    let vault_before = amt(&rt.acct(&a.vault).await.unwrap());
    let escrow_before = amt(&rt.acct(&e13a).await.unwrap());
    let stranger_refund = rt.send_with(prog, IX_REFUND, &13u64.to_le_bytes(), refund_accounts(stranger.pubkey(), *g, batch13, &a, &b, e13a, e13b, shared), &[&stranger]).await;
    let vault_after = amt(&rt.acct(&a.vault).await.unwrap());
    let refund_ok = stranger_refund.is_ok() && vault_after == vault_before + escrow_before && amt(&rt.acct(&e13a).await.unwrap()) == 0 && rt.acct(&stranger.pubkey()).await.unwrap().data.is_empty();
    rows.push(Rec(format!("{}-refund-any-signer-recorded-source-only", tag(shared)), refund_ok, format!("refund={:?} vault_gained={}", stranger_refund.as_ref().err(), vault_after - vault_before), 0));
    assert!(refund_ok, "{stranger_refund:?} vault {vault_before}->{vault_after}");
    rt.send(prog, IX_REFUND, &15u64.to_le_bytes(), refund_accounts(*admin, *g, batch15, &a, &b, e15a, e15b, shared)).await.expect("refund 15");
    rt.send(prog, IX_CLOSE_BATCH, &15u64.to_le_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(batch15, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(mark, false),
        AccountMeta::new(e15a, false), AccountMeta::new(e15b, false),
    ]).await.expect("close 15");
    let reopen14 = rt.send(prog, IX_OPEN, &14u64.to_le_bytes(), open_accounts(*admin, *g, pda(&prog, &[b"lpbatch", &14u64.to_le_bytes()]), auto, damm_pool, &a, &b, e13a, e13b, va, vb, shared, prog)).await;
    let stranger_batch = rt.send_with(prog, IX_CLOSE_BATCH, &13u64.to_le_bytes(), vec![
        AccountMeta::new(stranger.pubkey(), true), AccountMeta::new_readonly(*g, false), AccountMeta::new(batch13, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(mark, false),
        AccountMeta::new(e13a, false), AccountMeta::new(e13b, false),
    ], &[&stranger]).await;
    let batch13_still = rt.acct(&batch13).await.unwrap().data.len();
    rt.send(prog, IX_CLOSE_BATCH, &13u64.to_le_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(batch13, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(mark, false),
        AccountMeta::new(e13a, false), AccountMeta::new(e13b, false),
    ]).await.expect("close lower id after higher");
    let high2 = u64::from_le_bytes(rt.acct(&mark).await.unwrap().data[8..16].try_into().unwrap());
    let replay13 = rt.send(prog, IX_OPEN, &13u64.to_le_bytes(), open_accounts(*admin, *g, batch13, auto, damm_pool, &a, &b, e13a, e13b, va, vb, shared, prog)).await;
    let order_ok = reopen14.is_err() && stranger_batch.is_err() && batch13_still == 564 && high2 >= 15 && replay13.is_err() && rt.acct(&batch13).await.map(|x| x.data.len()).unwrap_or(0) == 0 && rt.acct(&mark).await.unwrap().data.len() == 17;
    rows.push(Rec(format!("{}-watermark-out-of-order", tag(shared)), order_ok, format!("gap14={:?} stranger_close={:?} high={high2} replay13={:?}", reopen14.as_ref().err().and_then(|e| custom(e)), stranger_batch.as_ref().err().and_then(|e| custom(e)), replay13.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(order_ok, "{reopen14:?} {stranger_batch:?} high={high2}");
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

fn open_accounts(admin: Pubkey, g: Pubkey, batch: Pubkey, auto: Pubkey, damm_pool: Pubkey, a: &Side, b: &Side, ea: Pubkey, eb: Pubkey, da: Pubkey, db: Pubkey, shared: bool, prog: Pubkey) -> Vec<AccountMeta> {
    let mut v = vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(g, false), AccountMeta::new(batch, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(auto, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(damm_pool, false), AccountMeta::new(pda(&prog, &[b"lpnonce"]), false),
    ];
    for (s, e, d) in [(a, ea, da), (b, eb, db)] {
        v.push(AccountMeta::new(s.prec, false));
        v.push(AccountMeta::new(s.vault, false));
        v.push(AccountMeta::new_readonly(s.quote, false));
        v.push(AccountMeta::new(e, false));
        if shared { v.push(AccountMeta::new(s.reserve, false)); }
        v.push(AccountMeta::new(d, false));
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
struct Wire { locks: usize, sigs: u8, message_len: usize, signed_len: usize }

async fn measure(rt: &mut Rt, accounts: &[AccountMeta], payer: &Pubkey, disc: &[u8; 8], data: &[u8], prog: Pubkey) -> Wire {
    let mut buf = disc.to_vec();
    buf.extend_from_slice(data);
    let ix = Instruction { program_id: prog, accounts: accounts.to_vec(), data: buf };
    let msg = Message::new(&[ix.clone()], Some(payer));
    let message_len = bincode::serialize(&msg).unwrap().len();
    let kp = rt.ctx.payer.insecure_clone();
    let hash = rt.ctx.banks_client.get_latest_blockhash().await.unwrap();
    let tx = solana_transaction::Transaction::new_signed_with_payer(&[ix], Some(&kp.pubkey()), &[&kp], hash);
    let signed_len = bincode::serialize(&tx).unwrap().len();
    Wire { locks: tx.message.account_keys.len(), sigs: tx.message.header.num_required_signatures, message_len, signed_len }
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
    let owed = pda(&prog, &[b"cown", pool.as_ref()]);
    rt.send(prog, IX_IMPORT, &[], vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(*reserve, false),
        AccountMeta::new_readonly(*prec, false), AccountMeta::new(owed, false),
        AccountMeta::new(marker, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await
}

async fn fee_mint_claim(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool, rows: &mut Vec<Rec>) {
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
    ]).await.expect("feeclaim cfg");
    let qs = pda(&prog, &[b"qstat", quote.as_ref()]);
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(prog, IX_SYNC, &sync, vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("feeclaim sync");
    let reserve = pda(&prog, &[b"qres", quote.as_ref()]);
    if shared {
        rt.send(prog, IX_INIT_RES, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
        ]).await.expect("feeclaim res");
    }
    let prec = pda(&prog, &[b"pool", pool.as_ref()]);
    reg_pool(rt, prog, admin, g, &cfg, &prec, &pool, &config, &base, &quote, &qs, if shared { Some(&reserve) } else { None }).await;
    let gvault = ata(g, &quote, &T22);
    rt.store(&gvault, owned(T22, fee_token(&quote, g, 0)));
    let pvault = ata(&prec, &quote, &T22);
    if !shared { rt.store(&pvault, owned(T22, fee_token(&quote, &prec, 0))); }
    if shared {
        import(rt, prog, admin, g, &reserve, &prec, &pool).await;
        rt.send(prog, IX_SEAL, &[], vec![
            AccountMeta::new_readonly(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new(gvault, false), AccountMeta::new_readonly(T22, false),
        ]).await.expect("feeclaim seal");
    }
    let qsrc = Pubkey::new_unique();
    rt.store(&qsrc, owned(T22, fee_token(&quote, g, 100_000)));
    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(&base, g, 0)));
    let base_vault = Pubkey::new_unique();
    rt.store(&base_vault, owned(system_program::ID, vec![]));
    let mut data = Vec::new();
    data.extend_from_slice(&1u64.to_le_bytes());
    data.extend_from_slice(&10_000u64.to_le_bytes());
    let mut accts = vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new(prec, false), AccountMeta::new_readonly(DBC_AUTH, false),
        AccountMeta::new_readonly(config, false), AccountMeta::new(pool, false), AccountMeta::new(base_dest, false),
        AccountMeta::new(gvault, false), AccountMeta::new(base_vault, false), AccountMeta::new(qsrc, false),
        AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(quote, false),
        AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(T22, false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false), AccountMeta::new_readonly(DBC, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ];
    if shared { accts.push(AccountMeta::new(reserve, false)); } else { accts.push(AccountMeta::new(pvault, false)); }
    let cu = rt.send(prog, IX_CLAIM, &data, accts).await.expect("fee mint claim");
    let global_left = amt(&rt.acct(&gvault).await.unwrap());
    let landed = if shared { global_left } else { amt(&rt.acct(&pvault).await.unwrap()) };
    let (holder, burn, _) = pots(&rt.acct(&prec).await.unwrap());
    let expect_burn = landed * 20 / 110;
    let expect_holder = landed - expect_burn;
    let ok = holder == expect_holder && burn == expect_burn && (shared || global_left == 0) && landed > 0 && landed < 10_000;
    rows.push(Rec(format!("{}-fee-mint-claim", tag(shared)), ok, format!("landed={landed} global_left={global_left} holder={holder} burn={burn}"), cu));
    assert!(ok, "landed={landed} global={global_left} pots={holder}/{burn}");
}

async fn two_pages(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool, rows: &mut Vec<Rec>) {
    let (quote, base, config, pool, prec, gvault, qsrc, reserve, _, _) = boot_curve(rt, prog, g, admin, shared).await;
    if shared {
        rt.send(prog, IX_SEAL, &[], vec![
            AccountMeta::new_readonly(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new(gvault, false), AccountMeta::new_readonly(spl_token::ID, false),
        ]).await.expect("page seal");
    }
    let pvault = ata(&prec, &quote, &spl_token::ID);
    if !shared { rt.store(&pvault, owned(spl_token::ID, token(&quote, &prec, 0))); }
    let vault = if shared { gvault } else { pvault };
    let cursor = pda(&prog, &[b"paycur", pool.as_ref()]);
    let sol_vault = pda(&prog, &[b"sol_vault"]);
    let unwrap = pda(&prog, &[b"unwrap", pool.as_ref()]);
    let mut open = pay2_accounts(*admin, *g, prec, vault, cursor, sol_vault, unwrap, shared, Some(reserve));
    rt.send(prog, IX_PAY2, &0u64.to_le_bytes(), open.split_off(0)).await.expect("open cursor round 0");
    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(&base, g, 0)));
    let base_vault = Pubkey::new_unique();
    rt.store(&base_vault, owned(system_program::ID, vec![]));
    let mut claim_data = Vec::new();
    claim_data.extend_from_slice(&1u64.to_le_bytes());
    claim_data.extend_from_slice(&11_000u64.to_le_bytes());
    let tail = if shared { Some(reserve) } else { Some(pvault) };
    rt.send(prog, IX_CLAIM, &claim_data, claim_metas(*g, prec, config, pool, gvault, qsrc, base, quote, base_dest, base_vault, tail)).await.expect("page claim");
    let mut w1 = Pubkey::new_unique();
    let mut w2 = Pubkey::new_unique();
    if w2 < w1 { std::mem::swap(&mut w1, &mut w2); }
    let b1 = Pubkey::new_unique();
    let b2 = Pubkey::new_unique();
    let d1 = Pubkey::new_unique();
    let d2 = Pubkey::new_unique();
    rt.store(&b1, owned(spl_token::ID, token(&base, &w1, 50)));
    rt.store(&b2, owned(spl_token::ID, token(&base, &w2, 50)));
    rt.store(&d1, owned(spl_token::ID, token(&quote, &w1, 0)));
    rt.store(&d2, owned(spl_token::ID, token(&quote, &w2, 0)));
    let mut page1 = pay2_accounts(*admin, *g, prec, vault, cursor, sol_vault, unwrap, shared, Some(reserve));
    page1.push(AccountMeta::new_readonly(b1, false));
    page1.push(AccountMeta::new(d1, false));
    rt.send(prog, IX_PAY2, &100u64.to_le_bytes(), page1).await.expect("page 1");
    assert_eq!(amt(&rt.acct(&d1).await.unwrap()), 4_500);
    let mut replay = pay2_accounts(*admin, *g, prec, vault, cursor, sol_vault, unwrap, shared, Some(reserve));
    replay.push(AccountMeta::new_readonly(b1, false));
    replay.push(AccountMeta::new(d1, false));
    rt.send(prog, IX_PAY2, &50u64.to_le_bytes(), replay).await.expect("replay lower wallet while pot remains");
    assert_eq!(amt(&rt.acct(&d1).await.unwrap()), 4_500, "cursor must not pay the same wallet twice in the round");
    let mut page2 = pay2_accounts(*admin, *g, prec, vault, cursor, sol_vault, unwrap, shared, Some(reserve));
    page2.push(AccountMeta::new_readonly(b2, false));
    page2.push(AccountMeta::new(d2, false));
    rt.send(prog, IX_PAY2, &50u64.to_le_bytes(), page2).await.expect("page 2");
    assert_eq!(amt(&rt.acct(&d2).await.unwrap()), 4_500);
    let ok = amt(&rt.acct(&d1).await.unwrap()) == 4_500 && amt(&rt.acct(&d2).await.unwrap()) == 4_500 && pots(&rt.acct(&prec).await.unwrap()).0 == 0;
    rows.push(Rec(format!("{}-two-pay-pages-same-round", tag(shared)), ok, format!("first={} second={}", amt(&rt.acct(&d1).await.unwrap()), amt(&rt.acct(&d2).await.unwrap())), 0));
    assert!(ok, "cursor paid the first wallet twice or left a pot");
}

async fn wsol_unwrap(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool, rows: &mut Vec<Rec>) {
    let quote = NATIVE;
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, mint(9, admin)));
    rt.store(&base, owned(spl_token::ID, mint(6, admin)));
    rt.store(&config, owned(DBC, dbc_cfg(&quote, g)));
    rt.store(&pool, owned(DBC, dbc_pool(&config, admin, &base, 0)));
    let cfg = pda(&prog, &[b"config", quote.as_ref()]);
    rt.send(prog, IX_REG_CFG, &quote.to_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(quote, false),
    ]).await.expect("wsol cfg");
    let qs = pda(&prog, &[b"qstat", quote.as_ref()]);
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(prog, IX_SYNC, &sync, vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("wsol sync");
    let reserve = pda(&prog, &[b"qres", quote.as_ref()]);
    if shared {
        rt.send(prog, IX_INIT_RES, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
        ]).await.expect("wsol res");
    }
    let prec = pda(&prog, &[b"pool", pool.as_ref()]);
    reg_pool(rt, prog, admin, g, &cfg, &prec, &pool, &config, &base, &quote, &qs, if shared { Some(&reserve) } else { None }).await;
    let gvault = ata(g, &quote, &spl_token::ID);
    let mut gv = owned(spl_token::ID, token(&quote, g, 0));
    gv.lamports = rent(165);
    rt.store(&gvault, gv);
    let pvault = ata(&prec, &quote, &spl_token::ID);
    if !shared {
        let mut pv = owned(spl_token::ID, token(&quote, &prec, 0));
        pv.lamports = rent(165);
        rt.store(&pvault, pv);
    }
    if shared {
        import(rt, prog, admin, g, &reserve, &prec, &pool).await;
        rt.send(prog, IX_SEAL, &[], vec![
            AccountMeta::new_readonly(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new(gvault, false), AccountMeta::new_readonly(spl_token::ID, false),
        ]).await.expect("wsol seal");
    }
    let qsrc = Pubkey::new_unique();
    let mut src = owned(spl_token::ID, token(&quote, g, 100_000));
    src.lamports = rent(165) + 100_000;
    rt.store(&qsrc, src);
    let cursor = pda(&prog, &[b"paycur", pool.as_ref()]);
    let sol_vault = pda(&prog, &[b"sol_vault"]);
    let unwrap = pda(&prog, &[b"unwrap", pool.as_ref()]);
    let vault = if shared { gvault } else { pvault };
    rt.send(prog, IX_PAY2, &0u64.to_le_bytes(), pay2_accounts(*admin, *g, prec, vault, cursor, sol_vault, unwrap, shared, Some(reserve))).await.expect("wsol cursor");
    let base_dest = Pubkey::new_unique();
    rt.store(&base_dest, owned(spl_token::ID, token(&base, g, 0)));
    let base_vault = Pubkey::new_unique();
    rt.store(&base_vault, owned(system_program::ID, vec![]));
    let mut claim_data = Vec::new();
    claim_data.extend_from_slice(&1u64.to_le_bytes());
    claim_data.extend_from_slice(&11_000u64.to_le_bytes());
    let tail = if shared { Some(reserve) } else { Some(pvault) };
    rt.send(prog, IX_CLAIM, &claim_data, claim_metas(*g, prec, config, pool, gvault, qsrc, base, quote, base_dest, base_vault, tail)).await.expect("wsol claim");
    let wallet = Pubkey::new_unique();
    rt.store(&wallet, owned(system_program::ID, vec![]));
    let hb = Pubkey::new_unique();
    rt.store(&hb, owned(spl_token::ID, token(&base, &wallet, 100)));
    let before = rt.acct(&wallet).await.unwrap().lamports;
    let mut pay = pay2_accounts(*admin, *g, prec, vault, cursor, sol_vault, unwrap, shared, Some(reserve));
    pay.push(AccountMeta::new_readonly(hb, false));
    pay.push(AccountMeta::new(wallet, false));
    let cu = rt.send(prog, IX_PAY2, &100u64.to_le_bytes(), pay).await.expect("wsol unwrap pay");
    let after = rt.acct(&wallet).await.unwrap().lamports;
    let ok = after == before + 9_000 && pots(&rt.acct(&prec).await.unwrap()).0 == 0;
    rows.push(Rec(format!("{}-wsol-unwrap-pay-v2", tag(shared)), ok, format!("lamports_gained={}", after - before), cu));
    assert!(ok, "gained {}", after - before);
}

fn old_pool(prog: &Pubkey, pool: &Pubkey, base: &Pubkey, quote: &Pubkey, config: &Pubkey, holder: u64, burn: u64) -> (Pubkey, Account) {
    let (prec, bump) = Pubkey::find_program_address(&[b"pool", pool.as_ref()], prog);
    let mut d = vec![0u8; 162];
    d[..8].copy_from_slice(&[117, 217, 136, 145, 181, 104, 107, 17]);
    d[8..40].copy_from_slice(pool.as_ref());
    d[40..72].copy_from_slice(base.as_ref());
    d[72..104].copy_from_slice(quote.as_ref());
    d[104..136].copy_from_slice(config.as_ref());
    d[137..145].copy_from_slice(&1u64.to_le_bytes());
    d[145..153].copy_from_slice(&holder.to_le_bytes());
    d[153..161].copy_from_slice(&burn.to_le_bytes());
    d[161] = bump;
    (prec, Account { lamports: rent(162), data: d, owner: *prog, executable: false, rent_epoch: 0 })
}

fn old_owed(prog: &Pubkey, pool: &Pubkey, owed: u64) -> (Pubkey, Account) {
    let (key, bump) = Pubkey::find_program_address(&[b"cown", pool.as_ref()], prog);
    let mut d = vec![0u8; 57];
    d[..8].copy_from_slice(&[83, 80, 110, 53, 75, 132, 102, 33]);
    d[8..40].copy_from_slice(pool.as_ref());
    d[40..48].copy_from_slice(&owed.to_le_bytes());
    d[56] = bump;
    (key, Account { lamports: rent(57), data: d, owner: *prog, executable: false, rent_epoch: 0 })
}

async fn old_layout(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool, rows: &mut Vec<Rec>) {
    let base = Pubkey::new_unique();
    rt.store(&base, owned(spl_token::ID, mint(6, admin)));
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    if shared {
        let quote = Pubkey::new_unique();
        rt.store(&quote, owned(T22, fee_mint(6, admin)));
        rt.store(&config, owned(DBC, dbc_cfg(&quote, g)));
        rt.store(&pool, owned(DBC, dbc_pool(&config, admin, &base, 0)));
        let (prec, acct) = old_pool(&prog, &pool, &base, &quote, &config, 8_000, 1_000);
        rt.store(&prec, acct);
        let (owed_key, owed_acct) = old_owed(&prog, &pool, 1_000);
        rt.store(&owed_key, owed_acct);
        let gvault = ata(g, &quote, &T22);
        rt.store(&gvault, owned(T22, fee_token(&quote, g, 10_000)));
        let reserve = pda(&prog, &[b"qres", quote.as_ref()]);
        rt.send(prog, IX_INIT_RES, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
        ]).await.expect("old init");
        let marker = pda(&prog, &[b"qimp", pool.as_ref()]);
        rt.send(prog, IX_IMPORT_OLD, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(prec, false), AccountMeta::new(owed_key, false),
            AccountMeta::new(marker, false), AccountMeta::new_readonly(system_program::ID, false),
        ]).await.expect("import existing");
        rt.send(prog, IX_SEAL, &[], vec![
            AccountMeta::new_readonly(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new(gvault, false), AccountMeta::new_readonly(T22, false),
        ]).await.expect("seal existing");
        let res = rt.acct(&reserve).await.unwrap();
        let rh = u64::from_le_bytes(res.data[56..64].try_into().unwrap());
        let rb = u64::from_le_bytes(res.data[64..72].try_into().unwrap());
        let ro = u64::from_le_bytes(res.data[72..80].try_into().unwrap());
        let pots_now = pots(&rt.acct(&prec).await.unwrap());
        let owed_now = u64::from_le_bytes(rt.acct(&owed_key).await.unwrap().data[40..48].try_into().unwrap());
        let ok = rh == 8_000 && rb == 1_000 && ro == 1_000 && pots_now.0 == 8_000 && pots_now.1 == 1_000 && owed_now == 1_000 && amt(&rt.acct(&gvault).await.unwrap()) == 10_000;
        rows.push(Rec("shared-old-layout-fee-mint-import".into(), ok, format!("reserve={rh}/{rb}/{ro} vault={}", amt(&rt.acct(&gvault).await.unwrap())), 0));
        assert!(ok, "shared import changed liabilities");
        rt.send(prog, IX_PAUSE, &[1], vec![AccountMeta::new_readonly(*admin, true), AccountMeta::new(*g, false)]).await.expect("pause");
        let qsrc = Pubkey::new_unique();
        rt.store(&qsrc, owned(T22, fee_token(&quote, g, 50_000)));
        let base_dest = Pubkey::new_unique();
        rt.store(&base_dest, owned(spl_token::ID, token(&base, g, 0)));
        let base_vault = Pubkey::new_unique();
        rt.store(&base_vault, owned(system_program::ID, vec![]));
        let mut claim_data = Vec::new();
        claim_data.extend_from_slice(&1u64.to_le_bytes());
        claim_data.extend_from_slice(&10_000u64.to_le_bytes());
        let paused = rt.send(prog, IX_CLAIM, &claim_data, vec![
            AccountMeta::new_readonly(*g, false), AccountMeta::new(prec, false), AccountMeta::new_readonly(DBC_AUTH, false),
            AccountMeta::new_readonly(config, false), AccountMeta::new(pool, false), AccountMeta::new(base_dest, false),
            AccountMeta::new(gvault, false), AccountMeta::new(base_vault, false), AccountMeta::new(qsrc, false),
            AccountMeta::new_readonly(base, false), AccountMeta::new_readonly(quote, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new_readonly(T22, false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false), AccountMeta::new_readonly(DBC, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(reserve, false),
        ]).await;
        let code = paused.as_ref().err().and_then(|e| custom(e));
        let still = amt(&rt.acct(&gvault).await.unwrap()) == 10_000 && pots(&rt.acct(&prec).await.unwrap()).0 == 8_000;
        rows.push(Rec("shared-paused-claim-keeps-import".into(), code == Some(6004) && still, format!("err={code:?}"), 0));
        assert!(code == Some(6004) && still, "{paused:?}");
        rt.send(prog, IX_PAUSE, &[0], vec![AccountMeta::new_readonly(*admin, true), AccountMeta::new(*g, false)]).await.expect("unpause");
    } else {
        let quote = Pubkey::new_unique();
        rt.store(&quote, owned(T22, fee_mint(6, admin)));
        rt.store(&config, owned(DBC, dbc_cfg(&quote, g)));
        rt.store(&pool, owned(DBC, dbc_pool(&config, admin, &base, 0)));
        let (prec, acct) = old_pool(&prog, &pool, &base, &quote, &config, 8_000, 1_000);
        rt.store(&prec, acct);
        let (owed_key, owed_acct) = old_owed(&prog, &pool, 1_000);
        rt.store(&owed_key, owed_acct);
        let gvault = ata(g, &quote, &T22);
        rt.store(&gvault, owned(T22, fee_token(&quote, g, 10_000)));
        let pvault = ata(&prec, &quote, &T22);
        rt.store(&pvault, owned(T22, fee_token(&quote, &prec, 0)));
        let marker = pda(&prog, &[b"pcust", pool.as_ref()]);
        let moved = rt.send(prog, IX_MIGRATE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec, false),
            AccountMeta::new(gvault, false), AccountMeta::new(pvault, false), AccountMeta::new_readonly(quote, false),
            AccountMeta::new_readonly(T22, false), AccountMeta::new(marker, false), AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new(owed_key, false),
        ]).await;
        let code = moved.as_ref().err().and_then(|e| custom(e));
        let pots_now = pots(&rt.acct(&prec).await.unwrap());
        let owed_now = u64::from_le_bytes(rt.acct(&owed_key).await.unwrap().data[40..48].try_into().unwrap());
        let ok = code == Some(6053) && pots_now.0 == 8_000 && pots_now.1 == 1_000 && owed_now == 1_000
            && amt(&rt.acct(&gvault).await.unwrap()) == 10_000 && amt(&rt.acct(&pvault).await.unwrap()) == 0;
        rows.push(Rec("per-fee-mint-migration-refuses-haircut".into(), ok, format!("err={code:?} vault={} pool_vault={} owed={owed_now}", amt(&rt.acct(&gvault).await.unwrap()), amt(&rt.acct(&pvault).await.unwrap())), 0));
        assert!(ok, "{moved:?} pots={pots_now:?}");
        let quote2 = Pubkey::new_unique();
        let config2 = Pubkey::new_unique();
        let pool2 = Pubkey::new_unique();
        rt.store(&quote2, owned(spl_token::ID, mint(9, admin)));
        rt.store(&config2, owned(DBC, dbc_cfg(&quote2, g)));
        rt.store(&pool2, owned(DBC, dbc_pool(&config2, admin, &base, 0)));
        let (prec2, acct2) = old_pool(&prog, &pool2, &base, &quote2, &config2, 9_000, 2_000);
        rt.store(&prec2, acct2);
        let gv2 = ata(g, &quote2, &spl_token::ID);
        rt.store(&gv2, owned(spl_token::ID, token(&quote2, g, 11_000)));
        let pv2 = ata(&prec2, &quote2, &spl_token::ID);
        rt.store(&pv2, owned(spl_token::ID, token(&quote2, &prec2, 0)));
        let marker2 = pda(&prog, &[b"pcust", pool2.as_ref()]);
        let cown2 = pda(&prog, &[b"cown", pool2.as_ref()]);
        rt.send(prog, IX_MIGRATE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec2, false),
            AccountMeta::new(gv2, false), AccountMeta::new(pv2, false), AccountMeta::new_readonly(quote2, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(marker2, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new(cown2, false),
        ]).await.expect("classic migrate");
        let again_move = rt.send(prog, IX_MIGRATE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec2, false),
            AccountMeta::new(gv2, false), AccountMeta::new(pv2, false), AccountMeta::new_readonly(quote2, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(marker2, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new(cown2, false),
        ]).await;
        let p2 = pots(&rt.acct(&prec2).await.unwrap());
        let ok2 = p2.0 == 9_000 && p2.1 == 2_000 && amt(&rt.acct(&pv2).await.unwrap()) == 11_000 && amt(&rt.acct(&gv2).await.unwrap()) == 0
            && again_move.as_ref().err().and_then(|e| custom(e)) == Some(6055);
        rows.push(Rec("per-classic-migration-preserves-pots".into(), ok2, format!("pool_vault={} pots={}/{} second={:?}", amt(&rt.acct(&pv2).await.unwrap()), p2.0, p2.1, again_move.as_ref().err().and_then(|e| custom(e))), 0));
        assert!(ok2, "classic migrate changed pots {again_move:?}");
        let stranger = Keypair::new();
        rt.store(&stranger.pubkey(), owned(system_program::ID, vec![]));
        let stranger_rev = rt.send_with(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(stranger.pubkey(), true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec2, false),
            AccountMeta::new(gv2, false), AccountMeta::new(pv2, false), AccountMeta::new_readonly(quote2, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(marker2, false), AccountMeta::new(cown2, false),
        ], &[&stranger]).await;
        rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec2, false),
            AccountMeta::new(gv2, false), AccountMeta::new(pv2, false), AccountMeta::new_readonly(quote2, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(marker2, false), AccountMeta::new(cown2, false),
        ]).await.expect("classic reverse");
        let replay_rev = rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec2, false),
            AccountMeta::new(gv2, false), AccountMeta::new(pv2, false), AccountMeta::new_readonly(quote2, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(marker2, false), AccountMeta::new(cown2, false),
        ]).await;
        let p3 = pots(&rt.acct(&prec2).await.unwrap());
        let global_back = amt(&rt.acct(&gv2).await.unwrap());
        let pool_left = amt(&rt.acct(&pv2).await.unwrap());
        let ok3 = stranger_rev.is_err() && replay_rev.as_ref().err().and_then(|e| custom(e)) == Some(6056)
            && p3.0 == 9_000 && p3.1 == 2_000 && global_back == 11_000 && pool_left == 0 && global_back >= 11_000;
        rows.push(Rec("per-classic-reversal-restores-global".into(), ok3, format!("global={global_back} pool={pool_left} pots={}/{} stranger={:?} replay={:?}", p3.0, p3.1, stranger_rev.as_ref().err().and_then(|e| custom(e)), replay_rev.as_ref().err().and_then(|e| custom(e))), 0));
        assert!(ok3, "{stranger_rev:?} {replay_rev:?} global={global_back}");
        let quote3 = Pubkey::new_unique();
        let config3 = Pubkey::new_unique();
        let pool3 = Pubkey::new_unique();
        rt.store(&quote3, owned(T22, fee_mint(6, admin)));
        rt.store(&config3, owned(DBC, dbc_cfg(&quote3, g)));
        rt.store(&pool3, owned(DBC, dbc_pool(&config3, admin, &base, 0)));
        let (prec3, acct3) = old_pool(&prog, &pool3, &base, &quote3, &config3, 8_000, 1_000);
        rt.store(&prec3, acct3);
        let (owed3, owed3_acct) = old_owed(&prog, &pool3, 1_000);
        rt.store(&owed3, owed3_acct);
        let gv3 = ata(g, &quote3, &T22);
        rt.store(&gv3, owned(T22, fee_token(&quote3, g, 0)));
        let pv3 = ata(&prec3, &quote3, &T22);
        rt.store(&pv3, owned(T22, fee_token(&quote3, &prec3, 10_000)));
        let (m3, mbump) = Pubkey::find_program_address(&[b"pcust", pool3.as_ref()], &prog);
        let mut md = vec![0u8; 42];
        md[..8].copy_from_slice(b"pcustody");
        md[8..40].copy_from_slice(pool3.as_ref());
        md[40] = 1;
        md[41] = mbump;
        rt.store(&m3, Account { lamports: rent(42), data: md, owner: prog, executable: false, rent_epoch: 0 });
        let fee_rev = rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec3, false),
            AccountMeta::new(gv3, false), AccountMeta::new(pv3, false), AccountMeta::new_readonly(quote3, false),
            AccountMeta::new_readonly(T22, false), AccountMeta::new(m3, false), AccountMeta::new(owed3, false),
        ]).await;
        let code = fee_rev.as_ref().err().and_then(|e| custom(e));
        let owed_left = u64::from_le_bytes(rt.acct(&owed3).await.unwrap().data[40..48].try_into().unwrap());
        let pots3 = pots(&rt.acct(&prec3).await.unwrap());
        let still = marker_state(&rt.acct(&m3).await.unwrap()) == 1 && amt(&rt.acct(&pv3).await.unwrap()) == 10_000 && amt(&rt.acct(&gv3).await.unwrap()) == 0
            && pots3.0 == 8_000 && pots3.1 == 1_000 && owed_left == 1_000;
        rows.push(Rec("per-fee-mint-reversal-not-reversible".into(), code == Some(6053) && still, format!("err={code:?} pool_vault={} owed={owed_left}", amt(&rt.acct(&pv3).await.unwrap())), 0));
        assert!(code == Some(6053) && still, "{fee_rev:?}");
        let quote4 = Pubkey::new_unique();
        let config4 = Pubkey::new_unique();
        let pool4 = Pubkey::new_unique();
        rt.store(&quote4, owned(spl_token::ID, mint(9, admin)));
        rt.store(&config4, owned(DBC, dbc_cfg(&quote4, g)));
        rt.store(&pool4, owned(DBC, dbc_pool(&config4, admin, &base, 0)));
        let (prec4, acct4) = old_pool(&prog, &pool4, &base, &quote4, &config4, 8_000, 1_000);
        rt.store(&prec4, acct4);
        let (owed4, owed4_acct) = old_owed(&prog, &pool4, 1_000);
        rt.store(&owed4, owed4_acct);
        let gv4 = ata(g, &quote4, &spl_token::ID);
        rt.store(&gv4, owned(spl_token::ID, token(&quote4, g, 12_000)));
        let pv4 = ata(&prec4, &quote4, &spl_token::ID);
        rt.store(&pv4, owned(spl_token::ID, token(&quote4, &prec4, 0)));
        let other = Pubkey::new_unique();
        rt.store(&other, owned(spl_token::ID, token(&quote4, &other, 7_000)));
        let m4 = pda(&prog, &[b"pcust", pool4.as_ref()]);
        rt.send(prog, IX_MIGRATE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec4, false),
            AccountMeta::new(gv4, false), AccountMeta::new(pv4, false), AccountMeta::new_readonly(quote4, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(m4, false), AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new(owed4, false),
        ]).await.expect("migrate with owed");
        rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec4, false),
            AccountMeta::new(gv4, false), AccountMeta::new(pv4, false), AccountMeta::new_readonly(quote4, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(m4, false), AccountMeta::new(owed4, false),
        ]).await.expect("reverse with owed");
        let owed4_now = u64::from_le_bytes(rt.acct(&owed4).await.unwrap().data[40..48].try_into().unwrap());
        let p4 = pots(&rt.acct(&prec4).await.unwrap());
        let other_left = amt(&rt.acct(&other).await.unwrap());
        let with_owed = owed4_now == 1_000 && p4.0 == 8_000 && p4.1 == 1_000 && amt(&rt.acct(&gv4).await.unwrap()) == 12_000 && amt(&rt.acct(&pv4).await.unwrap()) == 0 && other_left == 7_000;
        rows.push(Rec("per-classic-reverse-keeps-owed-and-other-vault".into(), with_owed, format!("global={} pool_left={} other={other_left} owed={owed4_now}", amt(&rt.acct(&gv4).await.unwrap()), amt(&rt.acct(&pv4).await.unwrap())), 0));
        assert!(with_owed, "owed reverse moved the wrong tokens");
        let quote5 = Pubkey::new_unique();
        let config5 = Pubkey::new_unique();
        let pool5 = Pubkey::new_unique();
        rt.store(&quote5, owned(spl_token::ID, mint(9, admin)));
        rt.store(&config5, owned(DBC, dbc_cfg(&quote5, g)));
        rt.store(&pool5, owned(DBC, dbc_pool(&config5, admin, &base, 0)));
        let (prec5, acct5) = old_pool(&prog, &pool5, &base, &quote5, &config5, 8_000, 1_000);
        rt.store(&prec5, acct5);
        let (owed5, owed5_acct) = old_owed(&prog, &pool5, 1_000);
        rt.store(&owed5, owed5_acct);
        let gv5 = ata(g, &quote5, &spl_token::ID);
        rt.store(&gv5, owned(spl_token::ID, token(&quote5, g, 10_000)));
        let pv5 = ata(&prec5, &quote5, &spl_token::ID);
        rt.store(&pv5, owned(spl_token::ID, token(&quote5, &prec5, 0)));
        let m5 = pda(&prog, &[b"pcust", pool5.as_ref()]);
        rt.send(prog, IX_MIGRATE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec5, false),
            AccountMeta::new(gv5, false), AccountMeta::new(pv5, false), AccountMeta::new_readonly(quote5, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(m5, false), AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new(owed5, false),
        ]).await.expect("migrate owed for omission");
        let omit = rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec5, false),
            AccountMeta::new(gv5, false), AccountMeta::new(pv5, false), AccountMeta::new_readonly(quote5, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(m5, false),
        ]).await;
        let readonly = rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec5, false),
            AccountMeta::new(gv5, false), AccountMeta::new(pv5, false), AccountMeta::new_readonly(quote5, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(m5, false), AccountMeta::new_readonly(owed5, false),
        ]).await;
        let wrong = rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec5, false),
            AccountMeta::new(gv5, false), AccountMeta::new(pv5, false), AccountMeta::new_readonly(quote5, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(m5, false), AccountMeta::new(owed4, false),
        ]).await;
        let owed_left = u64::from_le_bytes(rt.acct(&owed5).await.unwrap().data[40..48].try_into().unwrap());
        let held = omit.as_ref().err().and_then(|e| custom(e)) == Some(6040)
            && readonly.as_ref().err().and_then(|e| custom(e)) == Some(6040)
            && wrong.as_ref().err().and_then(|e| custom(e)) == Some(6040)
            && marker_state(&rt.acct(&m5).await.unwrap()) == 1
            && amt(&rt.acct(&pv5).await.unwrap()) == 10_000
            && amt(&rt.acct(&gv5).await.unwrap()) == 0
            && owed_left == 1_000;
        rows.push(Rec("per-reverse-rejects-bad-cown".into(), held, format!("omit={:?} readonly={:?} wrong_pool={:?}", omit.as_ref().err().and_then(|e| custom(e)), readonly.as_ref().err().and_then(|e| custom(e)), wrong.as_ref().err().and_then(|e| custom(e))), 0));
        assert!(held, "{omit:?} {readonly:?} {wrong:?}");
        rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec5, false),
            AccountMeta::new(gv5, false), AccountMeta::new(pv5, false), AccountMeta::new_readonly(quote5, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(m5, false), AccountMeta::new(owed5, false),
        ]).await.expect("reverse with canonical cown");
        let replay = rt.send(prog, IX_REVERSE, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(prec5, false),
            AccountMeta::new(gv5, false), AccountMeta::new(pv5, false), AccountMeta::new_readonly(quote5, false),
            AccountMeta::new_readonly(spl_token::ID, false), AccountMeta::new(m5, false), AccountMeta::new(owed5, false),
        ]).await;
        let restored = marker_state(&rt.acct(&m5).await.unwrap()) == 2
            && amt(&rt.acct(&gv5).await.unwrap()) == 10_000
            && amt(&rt.acct(&pv5).await.unwrap()) == 0
            && u64::from_le_bytes(rt.acct(&owed5).await.unwrap().data[40..48].try_into().unwrap()) == 1_000
            && replay.as_ref().err().and_then(|e| custom(e)) == Some(6056);
        rows.push(Rec("per-reverse-canonical-cown-restores-creator-funds".into(), restored, format!("replay={:?}", replay.as_ref().err().and_then(|e| custom(e))), 0));
        assert!(restored, "{replay:?}");
    }
}

fn marker_state(a: &Account) -> u8 { a.data[40] }

async fn caller_matrix(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, shared: bool, rows: &mut Vec<Rec>) {
    let quote = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let pool = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, mint(9, admin)));
    rt.store(&base, owned(spl_token::ID, mint(6, admin)));
    rt.store(&config, owned(DBC, dbc_cfg(&quote, g)));
    rt.store(&pool, owned(DBC, dbc_pool(&config, admin, &base, 0)));
    let cfg = pda(&prog, &[b"config", quote.as_ref()]);
    rt.send(prog, IX_REG_CFG, &quote.to_bytes(), vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(config, false),
        AccountMeta::new(cfg, false), AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(quote, false),
    ]).await.expect("matrix cfg");
    let mut data = Vec::new();
    data.extend_from_slice(pool.as_ref());
    data.extend_from_slice(base.as_ref());
    data.extend_from_slice(quote.as_ref());
    let prec = pda(&prog, &[b"pool", pool.as_ref()]);
    let named = vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(cfg, false), AccountMeta::new(prec, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(*g, false),
        AccountMeta::new_readonly(pool, false), AccountMeta::new_readonly(config, false),
    ];
    let mut eight = named.clone();
    eight.push(AccountMeta::new_readonly(quote, false));
    let c8 = rt.send(prog, IX_REG_POOL, &data, eight).await.err().and_then(|e| custom(&e));
    let qs = pda(&prog, &[b"qstat", quote.as_ref()]);
    let mut empty_qs = named.clone();
    empty_qs.push(AccountMeta::new_readonly(quote, false));
    empty_qs.push(AccountMeta::new(qs, false));
    let c_empty = rt.send(prog, IX_REG_POOL, &data, empty_qs).await.err().and_then(|e| custom(&e));
    let mut sync = quote.to_bytes().to_vec();
    sync.extend_from_slice(&0u64.to_le_bytes());
    rt.send(prog, IX_SYNC, &sync, vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(qs, false), AccountMeta::new_readonly(system_program::ID, false),
    ]).await.expect("matrix sync");
    let reserve = pda(&prog, &[b"qres", quote.as_ref()]);
    let mut no_res = named.clone();
    no_res.push(AccountMeta::new_readonly(quote, false));
    no_res.push(AccountMeta::new(qs, false));
    let c_no_res = if shared { rt.send(prog, IX_REG_POOL, &data, no_res).await.err().and_then(|e| custom(&e)) } else { None };
    if shared {
        rt.send(prog, IX_INIT_RES, &[], vec![
            AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(quote, false), AccountMeta::new_readonly(system_program::ID, false),
        ]).await.expect("matrix res");
    }
    let mut good = named.clone();
    good.push(AccountMeta::new_readonly(quote, false));
    good.push(AccountMeta::new(qs, false));
    if shared { good.push(AccountMeta::new(reserve, false)); }
    let reg = rt.send(prog, IX_REG_POOL, &data, good).await;
    let ok = c8 == Some(6048) && c_empty == Some(6049) && reg.is_ok() && (!shared || c_no_res == Some(6050));
    rows.push(Rec(format!("{}-caller-qstat-qres", tag(shared)), ok, format!("eight={c8:?} empty_qstat={c_empty:?} no_reserve={c_no_res:?} with_bootstrap={:?}", reg.is_ok()), 0));
    assert!(ok, "eight={c8:?} empty={c_empty:?} nores={c_no_res:?} reg={reg:?}");
    if shared {
        let wire = register_wires(&prog, admin, &quote, &base, &pool, &config);
        rows.push(Rec("register-signed-sizes".into(), wire.eight < 1232 && wire.with_qstat < 1232, format!("eight={} qstat={} qstat_qres={} memo_qstat={} sdk_create=blocked", wire.eight, wire.with_qstat, wire.with_both, wire.memo_qstat), 0));
    }
    retained_three(rt, prog, g, admin, &pool, &prec, &quote, if shared { "shared" } else { "per_pool" }, rows).await;
}

const ABI_TEXT: &str = include_str!("../../abi/INSTRUCTION-CONTRACT.json");
const CRANK_KEY: Pubkey = solana_pubkey::pubkey!("2tUjZV4UX9V8P65RZxFBhTxHTDhYRhWo6MYsAGuTsn1h");

fn abi_disc(design: &str, name: &str) -> [u8; 8] {
    let id = format!("\"id\": \"{design}.{name}\"");
    let at = ABI_TEXT.find(&id).unwrap_or_else(|| panic!("ABI missing {id}"));
    let window = &ABI_TEXT[at..at + 700];
    let d = window.find("\"disc\": [").unwrap_or_else(|| panic!("disc {id}"));
    let body = &window[d + 9..];
    let end = body.find(']').unwrap();
    let nums: Vec<u8> = body[..end].split(',').map(|s| s.trim().parse().unwrap()).collect();
    nums.try_into().unwrap()
}

fn abi_has_accounts(design: &str, name: &str, accounts: &[&str]) {
    let id = format!("\"id\": \"{design}.{name}\"");
    let at = ABI_TEXT.find(&id).unwrap_or_else(|| panic!("ABI missing {id}"));
    let window = &ABI_TEXT[at..at + 1800];
    let mut from = 0;
    for a in accounts {
        let key = format!("\"name\": \"{a}\"");
        let j = window[from..].find(&key).unwrap_or_else(|| panic!("{id} missing {a}"));
        from += j + key.len();
    }
}

async fn retained_three(rt: &mut Rt, prog: Pubkey, g: &Pubkey, admin: &Pubkey, pool: &Pubkey, prec: &Pubkey, quote: &Pubkey, design: &str, rows: &mut Vec<Rec>) {
    for (n, accts) in [
        ("mark_graduated", &["global", "pool_record", "dbc_pool"][..]),
        ("check_quote_mint", &["mint"][..]),
        ("close_old_checkpoints", &["signer", "global", "pool_record", "rent_receiver"][..]),
    ] {
        abi_has_accounts(design, n, accts);
        let _ = abi_disc(design, n);
    }
    let mark = abi_disc(design, "mark_graduated");
    let early = rt.send(prog, mark, &[], vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new(*prec, false), AccountMeta::new_readonly(*pool, false),
    ]).await;
    let mut dbc = rt.acct(pool).await.unwrap();
    dbc.data[305] = 1;
    rt.store(pool, dbc);
    rt.send(prog, mark, &[], vec![
        AccountMeta::new_readonly(*g, false), AccountMeta::new(*prec, false), AccountMeta::new_readonly(*pool, false),
    ]).await.expect("mark graduated");
    let graduated = rt.acct(prec).await.unwrap().data[136] == 1 && early.as_ref().err().and_then(|e| custom(e)) == Some(6022);
    rows.push(Rec(format!("{design}-mark-graduated"), graduated, format!("before={:?}", early.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(graduated, "{early:?}");
    let check = abi_disc(design, "check_quote_mint");
    let checked = rt.send(prog, check, &[], vec![AccountMeta::new_readonly(*quote, false)]).await;
    rows.push(Rec(format!("{design}-check-quote-mint"), checked.is_ok(), format!("{:?}", checked.as_ref().err()), 0));
    assert!(checked.is_ok(), "{checked:?}");
    let mut pr = rt.acct(prec).await.unwrap();
    pr.data[137..145].copy_from_slice(&2u64.to_le_bytes());
    rt.store(prec, pr);
    let owner = Pubkey::new_unique();
    let round = 1u64;
    let (ckpt, bump) = Pubkey::find_program_address(&[b"ckpt", pool.as_ref(), &round.to_le_bytes(), owner.as_ref()], &prog);
    rt.store(&ckpt, Account { lamports: rent(8), data: vec![1u8; 8], owner: prog, executable: false, rent_epoch: 0 });
    rt.store(&CRANK_KEY, Account { lamports: 1_000_000, data: vec![], owner: system_program::ID, executable: false, rent_epoch: 0 });
    let mut args = Vec::new();
    args.extend_from_slice(&1u32.to_le_bytes());
    args.extend_from_slice(&round.to_le_bytes());
    args.extend_from_slice(owner.as_ref());
    args.push(bump);
    let close = abi_disc(design, "close_old_checkpoints");
    let wrong_dest = rt.send(prog, close, &args, vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(*prec, false),
        AccountMeta::new(*admin, false), AccountMeta::new(ckpt, false),
    ]).await;
    let closed = rt.send(prog, close, &args, vec![
        AccountMeta::new(*admin, true), AccountMeta::new_readonly(*g, false), AccountMeta::new_readonly(*prec, false),
        AccountMeta::new(CRANK_KEY, false), AccountMeta::new(ckpt, false),
    ]).await;
    let crank_after = rt.acct(&CRANK_KEY).await.unwrap().lamports;
    let gone = rt.acct(&ckpt).await.map(|a| a.lamports == 0 && a.data.is_empty()).unwrap_or(true);
    let ok = wrong_dest.as_ref().err().and_then(|e| custom(e)) == Some(6027) && closed.is_ok() && crank_after > 1_000_000 && gone;
    rows.push(Rec(format!("{design}-close-old-checkpoints"), ok, format!("wrong_dest={:?} crank_lamports={crank_after}", wrong_dest.as_ref().err().and_then(|e| custom(e))), 0));
    assert!(ok, "{wrong_dest:?} {closed:?} crank={crank_after}");
}

struct Wires { eight: usize, with_qstat: usize, with_both: usize, memo_qstat: usize }

fn register_wires(prog: &Pubkey, payer: &Pubkey, quote: &Pubkey, base: &Pubkey, pool: &Pubkey, config: &Pubkey) -> Wires {
    let cfg = pda(prog, &[b"config", quote.as_ref()]);
    let prec = pda(prog, &[b"pool", pool.as_ref()]);
    let qs = pda(prog, &[b"qstat", quote.as_ref()]);
    let qres = pda(prog, &[b"qres", quote.as_ref()]);
    let mut data = IX_REG_POOL.to_vec();
    data.extend_from_slice(pool.as_ref());
    data.extend_from_slice(base.as_ref());
    data.extend_from_slice(quote.as_ref());
    let base_keys = vec![
        AccountMeta::new(*payer, true), AccountMeta::new_readonly(cfg, false), AccountMeta::new(prec, false),
        AccountMeta::new_readonly(system_program::ID, false), AccountMeta::new_readonly(pda(prog, &[b"global"]), false),
        AccountMeta::new_readonly(*pool, false), AccountMeta::new_readonly(*config, false), AccountMeta::new_readonly(*quote, false),
    ];
    let mut with_qstat = base_keys.clone();
    with_qstat.push(AccountMeta::new(qs, false));
    let mut with_both = with_qstat.clone();
    with_both.push(AccountMeta::new(qres, false));
    let eight = signed_len(prog, &base_keys, &data, &[]);
    let qstat_len = signed_len(prog, &with_qstat, &data, &[]);
    let both_len = signed_len(prog, &with_both, &data, &[]);
    let memo = Instruction { program_id: MEMO, accounts: vec![AccountMeta::new_readonly(Pubkey::new_unique(), true)], data: b"fusedpad".to_vec() };
    let memo_len = signed_len(prog, &with_qstat, &data, &[memo]);
    Wires { eight, with_qstat: qstat_len, with_both: both_len, memo_qstat: memo_len }
}

fn signed_len(prog: &Pubkey, keys: &[AccountMeta], data: &[u8], extra: &[Instruction]) -> usize {
    let ix = Instruction { program_id: *prog, accounts: keys.to_vec(), data: data.to_vec() };
    let mut ixs = vec![ix];
    ixs.extend(extra.iter().cloned());
    let msg = Message::new(&ixs, Some(&keys[0].pubkey));
    let kp = Keypair::new();
    let mut tx = Transaction::new_unsigned(msg);
    tx.message.account_keys[0] = kp.pubkey();
    // Size depends on key bytes only by width, not value. One or two signatures.
    let n = tx.message.header.num_required_signatures as usize;
    let msg_len = bincode::serialize(&tx.message).unwrap().len();
    1 + n * 64 + msg_len
}
