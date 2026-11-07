//! Executed local comparison of the two experimental vault programs.
//! SPL Token is the real program-test binary. There is no Meteora program here.
//! Deposits are direct token transfers, not DBC or DAMM claims.

use exp_common::*;
use solana_account::{Account, AccountSharedData};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_signer::Signer;
use solana_system_interface::program as system_program;
use solana_transaction::Transaction;

const BPF_LOADER: Pubkey = solana_pubkey::pubkey!("BPFLoader2111111111111111111111111111111111");

struct Rt {
    ctx: ProgramTestContext,
    slot: u64,
}
struct Sent {
    ok: bool,
    err: Option<u32>,
    cu: u64,
    logs: Vec<String>,
}

impl Rt {
    async fn send(&mut self, ixs: &[Instruction]) -> Sent {
        self.send_as(ixs, None).await
    }
    async fn send_as(&mut self, ixs: &[Instruction], payer: Option<&Keypair>) -> Sent {
        self.slot += 1;
        let _ = self.ctx.warp_to_slot(self.slot);
        let bh = self.ctx.banks_client.get_latest_blockhash().await.unwrap();
        let default_payer = self.ctx.payer.insecure_clone();
        let (payer_key, signer): (Pubkey, &Keypair) = match payer {
            Some(k) => (k.pubkey(), k),
            None => (default_payer.pubkey(), &default_payer),
        };
        let tx = Transaction::new_signed_with_payer(ixs, Some(&payer_key), &[signer], bh);
        match self.ctx.banks_client.process_transaction_with_metadata(tx).await {
            Ok(r) => {
                let logs = r.metadata.as_ref().map(|m| m.log_messages.clone()).unwrap_or_default();
                let cu = r.metadata.as_ref().map(|m| m.compute_units_consumed).unwrap_or(0);
                match r.result {
                    Ok(()) => Sent { ok: true, err: None, cu, logs },
                    Err(e) => {
                        let text = format!("{e:?}");
                        Sent { ok: false, err: custom(&text), cu, logs }
                    }
                }
            }
            Err(e) => Sent { ok: false, err: None, cu: 0, logs: vec![format!("{e:?}")] },
        }
    }
    async fn acct(&mut self, k: &Pubkey) -> Option<Account> {
        self.ctx.banks_client.get_account(*k).await.unwrap()
    }
    fn store(&mut self, k: &Pubkey, a: Account) {
        self.ctx.set_account(k, &AccountSharedData::from(a));
    }
    fn payer(&self) -> Pubkey {
        self.ctx.payer.pubkey()
    }
}

fn custom(err: &str) -> Option<u32> {
    for marker in ["Custom(", "custom program error: "] {
        if let Some(i) = err.find(marker) {
            let digits: String = err[i + marker.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse() {
                return Some(n);
            }
        }
    }
    None
}

fn rent(space: usize) -> u64 {
    Rent::default().minimum_balance(space).max(1)
}
fn owned(owner: Pubkey, data: Vec<u8>) -> Account {
    Account { lamports: rent(data.len()), data, owner, executable: false, rent_epoch: 0 }
}
fn mint_bytes(dec: u8) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    d[0..4].copy_from_slice(&1u32.to_le_bytes());
    d[44] = dec;
    d[45] = 1;
    d
}
fn token_bytes(mint: &Pubkey, owner: &Pubkey, amount: u64) -> Vec<u8> {
    let mut d = vec![0u8; 165];
    d[0..32].copy_from_slice(mint.as_ref());
    d[32..64].copy_from_slice(owner.as_ref());
    d[64..72].copy_from_slice(&amount.to_le_bytes());
    d[108] = 1;
    d
}
fn amount_of(a: &Account) -> u64 {
    u64::from_le_bytes(a.data[64..72].try_into().unwrap())
}
fn ix(program: Pubkey, data: Vec<u8>, accounts: Vec<AccountMeta>) -> Instruction {
    Instruction { program_id: program, accounts, data }
}
fn find_so(name: &str) -> std::path::PathBuf {
    let roots = ["/workspace/experiments/target/deploy", "/workspace/experiments/shared_reserve/target/deploy", "/workspace/experiments/per_pool_vault/target/deploy", "/tmp/oldbuild/target/deploy"];
    for r in roots {
        let p = std::path::PathBuf::from(r).join(format!("{name}.so"));
        if p.is_file() {
            return p;
        }
    }
    panic!("missing {name}.so");
}

fn load(pt: &mut ProgramTest, name: &str, id: Pubkey) -> (String, usize) {
    let path = find_so(name);
    let elf = std::fs::read(&path).unwrap();
    let sha = solana_sha256_hasher::hash(&elf).to_bytes();
    let sha = sha.iter().map(|b| format!("{b:02x}")).collect::<String>();
    eprintln!("SO {name} bytes={} sha256={sha}", elf.len());
    pt.add_account(id, Account { lamports: rent(elf.len()), data: elf.clone(), owner: BPF_LOADER, executable: true, rent_epoch: 0 });
    (sha, elf.len())
}

#[tokio::test]
async fn both_designs() {
    let mut pt = ProgramTest::default();
    pt.prefer_bpf(false);
    pt.set_compute_max_units(1_400_000);
    let (sha_b, bytes_b) = load(&mut pt, "shared_reserve", SHARED_ID);
    let (sha_a, bytes_a) = load(&mut pt, "per_pool_vault", PER_POOL_ID);
    let mut ctx = pt.start_with_context().await;
    let _ = ctx.warp_to_slot(1);
    let mut rt = Rt { ctx, slot: 1 };
    let mut rows = Vec::new();

    shared_suite(&mut rt, &mut rows).await;
    per_suite(&mut rt, &mut rows).await;

    let mut out = format!("{{\"shared_sha256\":\"{sha_b}\",\"shared_bytes\":{bytes_b},\"per_sha256\":\"{sha_a}\",\"per_bytes\":{bytes_a},\"rows\":[\n");
    for (i, r) in rows.iter().enumerate() {
        let c = if i + 1 == rows.len() { "" } else { "," };
        out.push_str(&format!(
            "{{\"id\":\"{}\",\"ok\":{},\"err\":\"{}\",\"detail\":\"{}\",\"cu\":{}}}{c}\n",
            r.0, r.1, r.2, r.3, r.4
        ));
    }
    out.push_str("]}\n");
    std::fs::create_dir_all("/workspace/experiments/results").unwrap();
    std::fs::write("/workspace/experiments/results/vault-compare.json", &out).unwrap();
    eprintln!("WROTE vault-compare.json rows={}", rows.len());
}

struct Rec(String, bool, String, String, u64);
fn push(rows: &mut Vec<Rec>, id: &str, s: &Sent, detail: String) {
    eprintln!("CASE {id} ok={} err={:?} cu={} {detail}", s.ok, s.err, s.cu);
    rows.push(Rec(id.into(), s.ok, s.err.map(|n| n.to_string()).unwrap_or_else(|| if s.ok { "none" } else { "unparsed" }.into()), detail, s.cu));
}
fn must(s: &Sent, what: &str) {
    assert!(s.ok, "{what} {:?}", s.logs);
}

async fn shared_suite(rt: &mut Rt, rows: &mut Vec<Rec>) {
    let admin = rt.payer();
    let global = pda_b(&[b"xb", b"global"]);
    must(&rt.send(&[ix(SHARED_ID, vec![1], vec![AccountMeta::new(admin, true), AccountMeta::new(global, false), AccountMeta::new_readonly(system_program::ID, false)])]).await, "b init");

    // Small quote: economics, rollback, race, pause, LP, fault injection.
    let mint = Pubkey::new_unique();
    rt.store(&mint, owned(spl_token::ID, mint_bytes(9)));
    let reserve = pda_b(&[b"xb", b"reserve", mint.as_ref()]);
    must(&rt.send(&[ix(SHARED_ID, vec![2], vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(global, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(system_program::ID, false),
    ])]).await, "b reserve");
    let vault = Pubkey::new_unique();
    rt.store(&vault, owned(spl_token::ID, token_bytes(&mint, &global, 0)));
    let (p1, id1) = make_pool_b(rt, &mint);
    let (p2, id2) = make_pool_b(rt, &mint);
    must(&reg_b(rt, global, reserve, p1, id1, mint).await, "reg1");
    must(&reg_b(rt, global, reserve, p2, id2, mint).await, "reg2");
    let early = deposit_b(rt, global, reserve, p1, vault, mint, 100, KIND_CURVE).await;
    push(rows, "b-deposit-while-frozen", &early, qs(rt, &reserve).await);
    assert!(!early.ok && early.err == Some(E_FROZEN), "{:?}", early.logs);

    // Third pool during backfill, before its import, seal must fail.
    must(&import_b(rt, global, reserve, p1, 0, 2_000, 0).await, "imp1");
    must(&import_b(rt, global, reserve, p2, 0, 0, 0).await, "imp2");
    let (p3, id3) = make_pool_b(rt, &mint);
    must(&reg_b(rt, global, reserve, p3, id3, mint).await, "reg3 during backfill");
    rt.store(&vault, owned(spl_token::ID, token_bytes(&mint, &global, 2_000)));
    let short_seal = seal_b(rt, global, reserve, vault).await;
    push(rows, "b-seal-missing-import", &short_seal, format!("err={:?}", short_seal.err));
    assert_eq!(short_seal.err, Some(E_COUNT));
    must(&import_b(rt, global, reserve, p3, 0, 0, 0).await, "imp3 zero");
    // Vault is short of nothing now: sum is 2000 burn. Exact.
    must(&seal_b(rt, global, reserve, vault).await, "seal");
    let src = Pubkey::new_unique();
    rt.store(&src, owned(spl_token::ID, token_bytes(&mint, &admin, 50_000)));
    let dep = deposit_b(rt, global, reserve, p1, vault, mint, 11_000, KIND_CURVE).await;
    must(&dep, "curve");
    let pots = pool_pots(rt, &p1).await;
    let tot = reserve_totals(rt, &reserve).await;
    push(rows, "b-curve-11000", &dep, format!("pots={pots:?} reserve={tot:?} vault={}", tok(rt, &vault).await));
    assert_eq!(pots, (9_000, 4_000, 0));
    let dep2 = deposit_b(rt, global, reserve, p2, vault, mint, 11_000, KIND_GRAD_CREATOR).await;
    must(&dep2, "grad");
    let pots2 = pool_pots(rt, &p2).await;
    push(rows, "b-grad-11000", &dep2, format!("pots={pots2:?} reserve={:?} vault={}", reserve_totals(rt, &reserve).await, tok(rt, &vault).await));
    assert_eq!(pots2, (8_280, 1_840, 880));

    let dest = Pubkey::new_unique();
    rt.store(&dest, owned(spl_token::ID, token_bytes(&mint, &admin, 0)));
    let before = tok(rt, &vault).await;
    let pay = out_b(rt, global, reserve, p1, vault, dest, mint, 100, 7).await;
    must(&pay, "pay");
    push(rows, "b-pay-100", &pay, format!("vault {}->{} dest={}", before, tok(rt, &vault).await, tok(rt, &dest).await));
    assert_eq!(tok(rt, &dest).await, 100);
    assert_eq!(pool_pots(rt, &p1).await.0, 8_900);

    // Same-transaction two outs. They share the reserve.
    let d2 = Pubkey::new_unique();
    rt.store(&d2, owned(spl_token::ID, token_bytes(&mint, &admin, 0)));
    let mut first = out_ix(global, reserve, p1, vault, d2, mint, 50, 7);
    let mut second = out_ix(global, reserve, p2, vault, d2, mint, 40, 9);
    first.accounts[0] = AccountMeta::new(admin, true);
    second.accounts[0] = AccountMeta::new(admin, true);
    let both = rt.send(&[first, second]).await;
    must(&both, "two out");
    push(rows, "b-two-outs-one-tx", &both, format!("dest={} reserve={:?}", tok(rt, &d2).await, reserve_totals(rt, &reserve).await));
    assert_eq!(tok(rt, &d2).await, 90);

    let stranger = Keypair::new();
    let fund = solana_system_interface::instruction::transfer(&admin, &stranger.pubkey(), 1_000_000);
    must(&rt.send(&[fund]).await, "fund stranger");
    let mut stranger_ix = out_ix(global, reserve, p1, vault, dest, mint, 1, 7);
    stranger_ix.accounts[0] = AccountMeta::new(stranger.pubkey(), true);
    let denied = rt.send_as(&[stranger_ix], Some(&stranger)).await;
    push(rows, "b-stranger", &denied, format!("{:?}", denied.err));
    assert_eq!(denied.err, Some(E_NOT_ADMIN));

    must(&rt.send(&[ix(SHARED_ID, vec![10, 1], vec![AccountMeta::new(admin, true), AccountMeta::new(global, false)])]).await, "pause");
    let paused = out_b(rt, global, reserve, p1, vault, dest, mint, 1, 7).await;
    push(rows, "b-paused", &paused, format!("{:?}", paused.err));
    assert_eq!(paused.err, Some(E_PAUSED));
    must(&rt.send(&[ix(SHARED_ID, vec![10, 0], vec![AccountMeta::new(admin, true), AccountMeta::new(global, false)])]).await, "unpause");

    // Manufactured 1-unit shortfall. Not produced by the instructions above.
    let bal = tok(rt, &vault).await;
    rt.store(&vault, owned(spl_token::ID, token_bytes(&mint, &global, bal - 1)));
    let burn_before = pool_pots(rt, &p2).await.1;
    let injected = out_b(rt, global, reserve, p2, vault, dest, mint, 1, 8).await;
    push(rows, "b-injected-short", &injected, format!("err={:?} burn_still={burn_before}", injected.err));
    assert!(!injected.ok);
    assert_eq!(pool_pots(rt, &p2).await.1, burn_before);
    rt.store(&vault, owned(spl_token::ID, token_bytes(&mint, &global, bal)));

    // LP batch of 2 using burn on p1 (imported 2000 plus curve 2000 = 4000, budget 2000).
    let escrow = pda_b(&[b"xb", b"escrow", &7u64.to_le_bytes()]);
    let etok = Pubkey::new_unique();
    rt.store(&etok, owned(spl_token::ID, token_bytes(&mint, &escrow, 0)));
    let burn_now = pool_pots(rt, &p1).await.1;
    let budget = burn_now / 2;
    let fund_lp = lp_fund_b(rt, global, reserve, vault, escrow, etok, mint, &[(p1, budget)]).await;
    must(&fund_lp, "lp fund");
    push(rows, "b-lp-fund", &fund_lp, format!("burn {}->{} escrow={}", burn_now, pool_pots(rt, &p1).await.1, tok(rt, &etok).await));
    assert_eq!(pool_pots(rt, &p1).await.1, burn_now - budget);
    assert_eq!(tok(rt, &etok).await, budget);
    let damm = Pubkey::new_unique();
    rt.store(&damm, owned(spl_token::ID, token_bytes(&mint, &admin, 0)));
    let consume = lp_consume_b(rt, global, escrow, etok, damm, mint, 7).await;
    must(&consume, "consume");
    let again = lp_consume_b(rt, global, escrow, etok, damm, mint, 7).await;
    must(&again, "consume replay");
    push(rows, "b-lp-consume-replay", &again, format!("damm={}", tok(rt, &damm).await));
    assert_eq!(tok(rt, &damm).await, budget);

    // Refund path on a second batch.
    let escrow2 = pda_b(&[b"xb", b"escrow", &8u64.to_le_bytes()]);
    let etok2 = Pubkey::new_unique();
    rt.store(&etok2, owned(spl_token::ID, token_bytes(&mint, &escrow2, 0)));
    let bnow = pool_pots(rt, &p2).await.1;
    let take = bnow / 2;
    let vault_before = tok(rt, &vault).await;
    must(&lp_fund_b(rt, global, reserve, vault, escrow2, etok2, mint, &[(p2, take)]).await, "fund2");
    must(&lp_refund_b(rt, global, reserve, vault, escrow2, etok2, mint, &[p2]).await, "refund");
    push(rows, "b-lp-refund", &Sent { ok: true, err: None, cu: 0, logs: vec![] }, format!("burn {} vault {}->{}", pool_pots(rt, &p2).await.1, vault_before, tok(rt, &vault).await));
    assert_eq!(pool_pots(rt, &p2).await.1, bnow);
    assert_eq!(tok(rt, &vault).await, vault_before);
    assert_eq!(tok(rt, &etok2).await, 0);

    // Over-budget rejected, state unchanged.
    let escrow9 = pda_b(&[b"xb", b"escrow", &9u64.to_le_bytes()]);
    let etok9 = Pubkey::new_unique();
    rt.store(&etok9, owned(spl_token::ID, token_bytes(&mint, &escrow9, 0)));
    let too = lp_fund_b(rt, global, reserve, vault, escrow9, etok9, mint, &[(p2, bnow)]).await;
    push(rows, "b-lp-over-budget", &too, format!("{:?}", too.err));
    assert_eq!(too.err, Some(E_BUDGET));
    assert_eq!(pool_pots(rt, &p2).await.1, bnow);

    // 8-account historical shape cannot register.
    let legacy = rt.send(&[ix(SHARED_ID, vec![3], vec![
        AccountMeta::new(admin, true),
        AccountMeta::new_readonly(Pubkey::new_unique(), false),
        AccountMeta::new(Pubkey::new_unique(), false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false),
        AccountMeta::new_readonly(mint, false),
    ])]).await;
    push(rows, "b-historical-8", &legacy, format!("{:?}", legacy.err));
    assert!(!legacy.ok);

    scale_shared(rt, rows).await;
    message_sizes(rows);
}

async fn scale_shared(rt: &mut Rt, rows: &mut Vec<Rec>) {
    let admin = rt.payer();
    let global = pda_b(&[b"xb", b"global"]);
    let mut sizes = vec![13u64, 4, 4, 3];
    sizes.extend(std::iter::repeat(2).take(4));
    sizes.extend(std::iter::repeat(1).take(11));
    assert_eq!(sizes.len(), 19);
    assert_eq!(sizes.iter().sum::<u64>(), 43);
    let mut pools_made = 0u64;
    for (gi, n) in sizes.into_iter().enumerate() {
        let mint = Pubkey::new_unique();
        rt.store(&mint, owned(spl_token::ID, mint_bytes(9)));
        let reserve = pda_b(&[b"xb", b"reserve", mint.as_ref()]);
        must(&rt.send(&[ix(SHARED_ID, vec![2], vec![
            AccountMeta::new(admin, true), AccountMeta::new_readonly(global, false), AccountMeta::new(reserve, false),
            AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(system_program::ID, false),
        ])]).await, "scale reserve");
        let mut ids = Vec::new();
        for _ in 0..n {
            let (rec, id) = make_pool_b(rt, &mint);
            must(&reg_b(rt, global, reserve, rec, id, mint).await, "scale reg");
            ids.push(rec);
        }
        let zero_group = n == 1 && gi == 18;
        let sum = if zero_group { 0 } else { n * 1_000 };
        let extra = if gi == 1 { 7 } else { 0 };
        let vault = Pubkey::new_unique();
        rt.store(&vault, owned(spl_token::ID, token_bytes(&mint, &global, sum + extra)));
        for (i, rec) in ids.iter().enumerate() {
            let (holder, owed) = if zero_group {
                (0, 0)
            } else if gi == 0 && i == 0 {
                (750, 250)
            } else {
                (1_000, 0)
            };
            must(&import_b(rt, global, reserve, *rec, holder, 0, owed).await, "scale import");
        }
        let sealed = seal_b(rt, global, reserve, vault).await;
        must(&sealed, "scale seal");
        if gi == 0 {
            push(rows, "b-migrate-first-group", &sealed, format!("pools={n} cu={}", sealed.cu));
        }
        pools_made += n;
    }
    push(rows, "b-migrate-43", &Sent { ok: true, err: None, cu: 0, logs: vec![] }, format!("pools={pools_made} groups=19"));
    assert_eq!(pools_made, 43);

    // 55 pools, one quote. Executed register/import/seal/pay. Account count of pay stays flat.
    let mint = Pubkey::new_unique();
    rt.store(&mint, owned(spl_token::ID, mint_bytes(9)));
    let reserve = pda_b(&[b"xb", b"reserve", mint.as_ref()]);
    must(&rt.send(&[ix(SHARED_ID, vec![2], vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(global, false), AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(system_program::ID, false),
    ])]).await, "55 reserve");
    let mut first = None;
    for i in 0..55 {
        let (rec, id) = make_pool_b(rt, &mint);
        must(&reg_b(rt, global, reserve, rec, id, mint).await, "55 reg");
        must(&import_b(rt, global, reserve, rec, 1, 0, 0).await, "55 imp");
        if i == 0 { first = Some(rec); }
    }
    let vault = Pubkey::new_unique();
    rt.store(&vault, owned(spl_token::ID, token_bytes(&mint, &global, 55)));
    must(&seal_b(rt, global, reserve, vault).await, "55 seal");
    let dest = Pubkey::new_unique();
    rt.store(&dest, owned(spl_token::ID, token_bytes(&mint, &admin, 0)));
    let mut pay_ix = out_ix(global, reserve, first.unwrap(), vault, dest, mint, 1, 7);
    pay_ix.accounts[0] = AccountMeta::new(admin, true);
    let locks = unique_locks(&pay_ix);
    let pay = rt.send(&[pay_ix]).await;
    must(&pay, "55 pay");
    push(rows, "b-scale-55-pay", &pay, format!("locks={locks} cu={}", pay.cu));
    assert!(locks < 64, "pay locks {locks}");
}

fn message_sizes(rows: &mut Vec<Rec>) {
    let payer = Pubkey::new_from_array([4u8; 32]);
    let one = dummy_pay(false);
    let (b1, k1) = wire(&[one], &payer);
    // Synthetic full list, not sent.
    let mut big = dummy_pay(false);
    for i in 0..55u8 {
        big.accounts.push(AccountMeta::new(Pubkey::new_from_array([i.wrapping_add(20); 32]), false));
    }
    let (bb, kb) = wire(&[big], &payer);
    eprintln!("SYNTHETIC shared pay bytes={b1} locks={k1}; fake-55-tail bytes={bb} locks={kb}");
    rows.push(Rec("synthetic-message".into(), true, "none".into(), format!("pay_bytes={b1} pay_locks={k1} fake55_bytes={bb} fake55_locks={kb}"), 0));
    assert!(b1 < 1232);
}

fn dummy_pay(with_tail: bool) -> Instruction {
    let _ = with_tail;
    ix(SHARED_ID, vec![7, 1, 0, 0, 0, 0, 0, 0, 0, 9], vec![
        AccountMeta::new(Pubkey::new_from_array([1; 32]), true),
        AccountMeta::new_readonly(Pubkey::new_from_array([2; 32]), false),
        AccountMeta::new(Pubkey::new_from_array([3; 32]), false),
        AccountMeta::new(Pubkey::new_from_array([4; 32]), false),
        AccountMeta::new(Pubkey::new_from_array([5; 32]), false),
        AccountMeta::new(Pubkey::new_from_array([6; 32]), false),
        AccountMeta::new_readonly(Pubkey::new_from_array([7; 32]), false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ])
}
fn wire(ixs: &[Instruction], payer: &Pubkey) -> (usize, usize) {
    let tx = Transaction::new_with_payer(ixs, Some(payer));
    (bincode::serialize(&tx).unwrap().len(), tx.message.account_keys.len())
}
fn unique_locks(ix: &Instruction) -> usize {
    wire(std::slice::from_ref(ix), &Pubkey::new_from_array([9; 32])).1
}

async fn per_suite(rt: &mut Rt, rows: &mut Vec<Rec>) {
    let admin = rt.payer();
    let global = pda_a(&[b"xa", b"global"]);
    must(&rt.send(&[ix(PER_POOL_ID, vec![1], vec![AccountMeta::new(admin, true), AccountMeta::new(global, false), AccountMeta::new_readonly(system_program::ID, false)])]).await, "a init");
    let mint = Pubkey::new_unique();
    rt.store(&mint, owned(spl_token::ID, mint_bytes(9)));
    let (p1, id1, v1) = make_pool_a(rt, &mint);
    let (p2, id2, v2) = make_pool_a(rt, &mint);
    must(&reg_a(rt, global, p1, v1, id1, mint).await, "a reg1");
    must(&reg_a(rt, global, p2, v2, id2, mint).await, "a reg2");
    let src = Pubkey::new_unique();
    rt.store(&src, owned(spl_token::ID, token_bytes(&mint, &admin, 100_000)));
    let dep = deposit_a(rt, global, p1, v1, src, mint, 11_000, KIND_CURVE).await;
    must(&dep, "a curve");
    push(rows, "a-curve-11000", &dep, format!("p1={:?} p2vault={} src={}", pool_pots_a(rt, &p1).await, tok(rt, &v2).await, tok(rt, &src).await));
    assert_eq!(pool_pots_a(rt, &p1).await, (9_000, 2_000, 0));
    assert_eq!(tok(rt, &v2).await, 0);
    assert_eq!(tok(rt, &v1).await, 11_000);
    let dep2 = deposit_a(rt, global, p2, v2, src, mint, 11_000, KIND_GRAD_CREATOR).await;
    must(&dep2, "a grad");
    assert_eq!(pool_pots_a(rt, &p2).await, (8_280, 1_840, 880));
    let dest = Pubkey::new_unique();
    rt.store(&dest, owned(spl_token::ID, token_bytes(&mint, &admin, 0)));
    let v2_before = tok(rt, &v2).await;
    must(&out_a(rt, global, p1, v1, dest, mint, 100, 4).await, "a pay");
    assert_eq!(tok(rt, &v2).await, v2_before);
    assert_eq!(tok(rt, &dest).await, 100);
    push(rows, "a-pay-isolates", &Sent { ok: true, err: None, cu: 0, logs: vec![] }, format!("v2_unchanged={v2_before}"));

    let legacy = rt.send(&[ix(PER_POOL_ID, vec![2], vec![
        AccountMeta::new(admin, true),
        AccountMeta::new_readonly(Pubkey::new_unique(), false),
        AccountMeta::new(Pubkey::new_unique(), false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false),
        AccountMeta::new_readonly(Pubkey::new_unique(), false),
        AccountMeta::new_readonly(mint, false),
    ])]).await;
    push(rows, "a-historical-8", &legacy, format!("{:?}", legacy.err));
    assert!(!legacy.ok);

    // Real migrate of 4 pools from one source, then a 4-wide LP fund, failed 4th, refund.
    let mut batch = Vec::new();
    let mig_src = Pubkey::new_unique();
    rt.store(&mig_src, owned(spl_token::ID, token_bytes(&mint, &admin, 40_000)));
    for _ in 0..4 {
        let (p, id, v) = make_pool_a(rt, &mint);
        must(&reg_a(rt, global, p, v, id, mint).await, "mig reg");
        must(&migrate_a(rt, global, p, v, mig_src, mint, 0, 10_000, 0).await, "mig");
        batch.push((p, v));
    }
    assert_eq!(tok(rt, &mig_src).await, 0);
    let escrow = pda_a(&[b"xa", b"escrow", &3u64.to_le_bytes()]);
    let etok = Pubkey::new_unique();
    rt.store(&etok, owned(spl_token::ID, token_bytes(&mint, &escrow, 0)));
    let pairs: Vec<(Pubkey, Pubkey, u64)> = batch.iter().map(|(p, v)| (*p, *v, 5_000)).collect();
    let funded = lp_fund_a(rt, global, escrow, etok, mint, &pairs).await;
    must(&funded, "a lp");
    push(rows, "a-lp-fund-4", &funded, format!("escrow={} cu={}", tok(rt, &etok).await, funded.cu));
    assert_eq!(tok(rt, &etok).await, 20_000);
    let damm = Pubkey::new_unique();
    rt.store(&damm, owned(spl_token::ID, token_bytes(&mint, &admin, 0)));
    must(&lp_consume_a(rt, global, escrow, etok, damm, mint, 3).await, "a consume");
    let replay = lp_consume_a(rt, global, escrow, etok, damm, mint, 3).await;
    must(&replay, "a replay");
    assert_eq!(tok(rt, &damm).await, 20_000);
    push(rows, "a-lp-replay", &replay, format!("damm={}", tok(rt, &damm).await));

    // Refund a second batch.
    let mut batch2 = Vec::new();
    rt.store(&mig_src, owned(spl_token::ID, token_bytes(&mint, &admin, 8_000)));
    for _ in 0..2 {
        let (p, id, v) = make_pool_a(rt, &mint);
        must(&reg_a(rt, global, p, v, id, mint).await, "r reg");
        must(&migrate_a(rt, global, p, v, mig_src, mint, 0, 4_000, 0).await, "r mig");
        batch2.push((p, v));
    }
    let escrow2 = pda_a(&[b"xa", b"escrow", &4u64.to_le_bytes()]);
    let etok2 = Pubkey::new_unique();
    rt.store(&etok2, owned(spl_token::ID, token_bytes(&mint, &escrow2, 0)));
    let pairs2: Vec<(Pubkey, Pubkey, u64)> = batch2.iter().map(|(p, v)| (*p, *v, 2_000)).collect();
    must(&lp_fund_a(rt, global, escrow2, etok2, mint, &pairs2).await, "fund refund");
    must(&lp_refund_a(rt, global, escrow2, etok2, mint, &batch2).await, "refund");
    assert_eq!(pool_pots_a(rt, &batch2[0].0).await.1, 4_000);
    assert_eq!(tok(rt, &batch2[0].1).await, 4_000);
    push(rows, "a-lp-refund", &Sent { ok: true, err: None, cu: 0, logs: vec![] }, "restored".into());

    // 5th pool rejected.
    let too = lp_fund_a(rt, global, pda_a(&[b"xa", b"escrow", &5u64.to_le_bytes()]), etok, mint, &[
        (batch2[0].0, batch2[0].1, 1),
        (batch2[0].0, batch2[0].1, 1),
        (batch2[0].0, batch2[0].1, 1),
        (batch2[0].0, batch2[0].1, 1),
        (batch2[0].0, batch2[0].1, 1),
    ]).await;
    push(rows, "a-lp-five", &too, format!("{:?}", too.err));
    assert_eq!(too.err, Some(E_BATCH));

    // Injected short on pool 1 does not block pool 2.
    let bal = tok(rt, &v1).await;
    rt.store(&v1, owned(spl_token::ID, token_bytes(&mint, &p1, 0)));
    let bad = out_a(rt, global, p1, v1, dest, mint, 1, 4).await;
    let ok2 = out_a(rt, global, p2, v2, dest, mint, 1, 4).await;
    push(rows, "a-injected-short-isolates", &ok2, format!("a_err={:?} b_ok={} ", bad.err, ok2.ok));
    assert!(!bad.ok && ok2.ok);
    rt.store(&v1, owned(spl_token::ID, token_bytes(&mint, &p1, bal)));

    // 13 pools migrated for real, pay account count flat.
    let src13 = Pubkey::new_unique();
    rt.store(&src13, owned(spl_token::ID, token_bytes(&mint, &admin, 13 * 100)));
    let mut first = None;
    for i in 0..13 {
        let (p, id, v) = make_pool_a(rt, &mint);
        must(&reg_a(rt, global, p, v, id, mint).await, "13 reg");
        must(&migrate_a(rt, global, p, v, src13, mint, 100, 0, 0).await, "13 mig");
        if i == 0 { first = Some((p, v)); }
    }
    assert_eq!(tok(rt, &src13).await, 0);
    let (p, v) = first.unwrap();
    let mut pay_ix = out_ix_a(global, p, v, dest, mint, 1, 4);
    pay_ix.accounts[0] = AccountMeta::new(admin, true);
    let locks = unique_locks(&pay_ix);
    let pay = rt.send(&[pay_ix]).await;
    must(&pay, "13 pay");
    push(rows, "a-scale-13-pay", &pay, format!("locks={locks} cu={}", pay.cu));
    assert!(locks < 20, "{locks}");

    let mint43 = Pubkey::new_unique();
    rt.store(&mint43, owned(spl_token::ID, mint_bytes(9)));
    let src43 = Pubkey::new_unique();
    rt.store(&src43, owned(spl_token::ID, token_bytes(&mint43, &admin, 43 * 100)));
    let mut groups = vec![13u64, 4, 4, 3];
    groups.extend(std::iter::repeat(2).take(4));
    groups.extend(std::iter::repeat(1).take(11));
    let mut made = 0u64;
    for n in groups {
        for _ in 0..n {
            let (p, id, v) = make_pool_a(rt, &mint43);
            must(&reg_a(rt, global, p, v, id, mint43).await, "a43 reg");
            must(&migrate_a(rt, global, p, v, src43, mint43, 100, 0, 0).await, "a43 mig");
            made += 1;
        }
    }
    assert_eq!(made, 43);
    assert_eq!(tok(rt, &src43).await, 0);
    push(rows, "a-migrate-43", &Sent { ok: true, err: None, cu: 0, logs: vec![] }, "43 real transfers, source emptied".into());

    // Empty pool close returns rent. A funded pool does not close.
    let (pe, ide, ve) = make_pool_a(rt, &mint);
    must(&reg_a(rt, global, pe, ve, ide, mint).await, "empty reg");
    let blocked = rt.send(&[ix(PER_POOL_ID, vec![12], vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(global, false), AccountMeta::new(p, false), AccountMeta::new(v, false),
    ])]).await;
    push(rows, "a-close-nonempty", &blocked, format!("{:?}", blocked.err));
    assert!(!blocked.ok);
    let closed = rt.send(&[ix(PER_POOL_ID, vec![12], vec![
        AccountMeta::new(admin, true), AccountMeta::new_readonly(global, false), AccountMeta::new(pe, false), AccountMeta::new(ve, false),
    ])]).await;
    must(&closed, "close empty");
    let gone = rt.acct(&pe).await;
    push(rows, "a-close-empty", &closed, format!("remains={gone:?}"));
}

fn pda_b(seeds: &[&[u8]]) -> Pubkey { Pubkey::find_program_address(seeds, &SHARED_ID).0 }
fn pda_a(seeds: &[&[u8]]) -> Pubkey { Pubkey::find_program_address(seeds, &PER_POOL_ID).0 }

fn make_pool_b(rt: &mut Rt, mint: &Pubkey) -> (Pubkey, Pubkey) {
    let id = Pubkey::new_unique();
    let rec = pda_b(&[b"xb", b"pool", id.as_ref()]);
    let _ = (rt, mint);
    (rec, id)
}
fn make_pool_a(rt: &mut Rt, mint: &Pubkey) -> (Pubkey, Pubkey, Pubkey) {
    let id = Pubkey::new_unique();
    let rec = pda_a(&[b"xa", b"pool", id.as_ref()]);
    let vault = Pubkey::new_unique();
    rt.store(&vault, owned(spl_token::ID, token_bytes(mint, &rec, 0)));
    (rec, id, vault)
}

async fn reg_b(rt: &mut Rt, global: Pubkey, reserve: Pubkey, pool: Pubkey, id: Pubkey, mint: Pubkey) -> Sent {
    let mut data = vec![3];
    data.extend_from_slice(id.as_ref());
    rt.send(&[ix(SHARED_ID, data, vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(reserve, false),
        AccountMeta::new(pool, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(system_program::ID, false),
    ])]).await
}
async fn import_b(rt: &mut Rt, global: Pubkey, reserve: Pubkey, pool: Pubkey, h: u64, b: u64, o: u64) -> Sent {
    let mut data = vec![4];
    data.extend_from_slice(&h.to_le_bytes());
    data.extend_from_slice(&b.to_le_bytes());
    data.extend_from_slice(&o.to_le_bytes());
    rt.send(&[ix(SHARED_ID, data, vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(reserve, false), AccountMeta::new(pool, false),
    ])]).await
}
async fn seal_b(rt: &mut Rt, global: Pubkey, reserve: Pubkey, vault: Pubkey) -> Sent {
    rt.send(&[ix(SHARED_ID, vec![5], vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(reserve, false), AccountMeta::new(vault, false),
    ])]).await
}
fn deposit_ix(global: Pubkey, reserve: Pubkey, pool: Pubkey, vault: Pubkey, mint: Pubkey, src: Pubkey, amount: u64, kind: u8) -> Instruction {
    let mut data = vec![6];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(kind);
    data.push(9);
    ix(SHARED_ID, data, vec![
        AccountMeta::new(Pubkey::new_from_array([0; 32]), true), // replaced by caller
        AccountMeta::new_readonly(global, false), AccountMeta::new(reserve, false), AccountMeta::new(pool, false),
        AccountMeta::new(vault, false), AccountMeta::new(src, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
    ])
}
async fn deposit_b(rt: &mut Rt, global: Pubkey, reserve: Pubkey, pool: Pubkey, vault: Pubkey, mint: Pubkey, amount: u64, kind: u8) -> Sent {
    let src = Pubkey::new_unique();
    // Use a stable source stored by the caller when possible. This helper creates one if amount path needs it.
    // The suite stores `src` separately for the first deposits. Here we require the vault's paired source to be the payer-owned account
    // passed via a fresh account funded from nowhere: create it with balance.
    rt.store(&src, owned(spl_token::ID, token_bytes(&mint, &rt.payer(), amount)));
    let mut ixn = deposit_ix(global, reserve, pool, vault, mint, src, amount, kind);
    ixn.accounts[0] = AccountMeta::new(rt.payer(), true);
    rt.send(&[ixn]).await
}
fn out_ix(global: Pubkey, reserve: Pubkey, pool: Pubkey, vault: Pubkey, dest: Pubkey, mint: Pubkey, amount: u64, tag: u8) -> Instruction {
    let mut data = vec![tag];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(9);
    ix(SHARED_ID, data, vec![
        AccountMeta::new(Pubkey::default(), true),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new(reserve, false),
        AccountMeta::new(pool, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(dest, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(spl_token::ID, false),
    ])
}
async fn out_b(rt: &mut Rt, global: Pubkey, reserve: Pubkey, pool: Pubkey, vault: Pubkey, dest: Pubkey, mint: Pubkey, amount: u64, tag: u8) -> Sent {
    let mut ixn = out_ix(global, reserve, pool, vault, dest, mint, amount, tag);
    ixn.accounts[0] = AccountMeta::new(rt.payer(), true);
    rt.send(&[ixn]).await
}
fn lp_fund_b_ix(global: Pubkey, reserve: Pubkey, vault: Pubkey, escrow: Pubkey, etok: Pubkey, mint: Pubkey, pools: &[(Pubkey, u64)], batch: u64) -> Instruction {
    let mut data = vec![11];
    data.extend_from_slice(&batch.to_le_bytes());
    data.push(pools.len() as u8);
    for (_, amt) in pools {
        data.extend_from_slice(&amt.to_le_bytes());
    }
    let mut metas = vec![
        AccountMeta::new(Pubkey::default(), true),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new(reserve, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(escrow, false),
        AccountMeta::new(etok, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    for (p, _) in pools {
        metas.push(AccountMeta::new(*p, false));
    }
    ix(SHARED_ID, data, metas)
}
async fn lp_fund_b(rt: &mut Rt, global: Pubkey, reserve: Pubkey, vault: Pubkey, escrow: Pubkey, etok: Pubkey, mint: Pubkey, pools: &[(Pubkey, u64)]) -> Sent {
    let batch = u64::from_le_bytes(escrow.to_bytes()[0..8].try_into().unwrap());
    // batch id is encoded in the PDA seeds by the caller. Recover it from the known calls: we pass escrow already derived.
    // The function lp_fund derives PDA from the batch in the data, so the data batch must match the escrow account.
    // Callers pass escrow = pda(batch). We can't invert the PDA. Pass batch via a side channel: encode it as the first 8 bytes? No.
    // Use the amount list only. Callers in this file use batches 7, 8, 9 and the escrow PDA of those. Detect by comparing.
    let batch = [7u64, 8, 9, 11].into_iter().find(|b| pda_b(&[b"xb", b"escrow", &b.to_le_bytes()]) == escrow).unwrap_or(batch);
    let mut ixn = lp_fund_b_ix(global, reserve, vault, escrow, etok, mint, pools, batch);
    ixn.accounts[0] = AccountMeta::new(rt.payer(), true);
    rt.send(&[ixn]).await
}
async fn lp_consume_b(rt: &mut Rt, global: Pubkey, escrow: Pubkey, etok: Pubkey, dest: Pubkey, mint: Pubkey, batch: u64) -> Sent {
    let mut data = vec![12];
    data.extend_from_slice(&batch.to_le_bytes());
    rt.send(&[ix(SHARED_ID, data, vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(escrow, false),
        AccountMeta::new(etok, false), AccountMeta::new(dest, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
    ])]).await
}
async fn lp_refund_b(rt: &mut Rt, global: Pubkey, reserve: Pubkey, vault: Pubkey, escrow: Pubkey, etok: Pubkey, mint: Pubkey, pools: &[Pubkey]) -> Sent {
    let batch = [8u64, 7, 9].into_iter().find(|b| pda_b(&[b"xb", b"escrow", &b.to_le_bytes()]) == escrow).unwrap();
    let mut data = vec![13];
    data.extend_from_slice(&batch.to_le_bytes());
    let mut metas = vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(reserve, false),
        AccountMeta::new(vault, false), AccountMeta::new(escrow, false), AccountMeta::new(etok, false),
        AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
    ];
    for p in pools { metas.push(AccountMeta::new(*p, false)); }
    rt.send(&[ix(SHARED_ID, data, metas)]).await
}

async fn reg_a(rt: &mut Rt, global: Pubkey, pool: Pubkey, vault: Pubkey, id: Pubkey, mint: Pubkey) -> Sent {
    let mut data = vec![2];
    data.extend_from_slice(id.as_ref());
    rt.send(&[ix(PER_POOL_ID, data, vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(pool, false),
        AccountMeta::new(vault, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(system_program::ID, false),
    ])]).await
}
async fn deposit_a(rt: &mut Rt, global: Pubkey, pool: Pubkey, vault: Pubkey, src: Pubkey, mint: Pubkey, amount: u64, kind: u8) -> Sent {
    let mut data = vec![3];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(kind);
    data.push(9);
    rt.send(&[ix(PER_POOL_ID, data, vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(pool, false),
        AccountMeta::new(vault, false), AccountMeta::new(src, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
    ])]).await
}
fn out_ix_a(global: Pubkey, pool: Pubkey, vault: Pubkey, dest: Pubkey, mint: Pubkey, amount: u64, tag: u8) -> Instruction {
    let mut data = vec![tag];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(9);
    ix(PER_POOL_ID, data, vec![
        AccountMeta::new(Pubkey::default(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(pool, false),
        AccountMeta::new(vault, false), AccountMeta::new(dest, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
    ])
}
async fn out_a(rt: &mut Rt, global: Pubkey, pool: Pubkey, vault: Pubkey, dest: Pubkey, mint: Pubkey, amount: u64, tag: u8) -> Sent {
    let mut ixn = out_ix_a(global, pool, vault, dest, mint, amount, tag);
    ixn.accounts[0] = AccountMeta::new(rt.payer(), true);
    rt.send(&[ixn]).await
}
async fn migrate_a(rt: &mut Rt, global: Pubkey, pool: Pubkey, vault: Pubkey, src: Pubkey, mint: Pubkey, h: u64, b: u64, o: u64) -> Sent {
    let mut data = vec![8];
    data.extend_from_slice(&h.to_le_bytes());
    data.extend_from_slice(&b.to_le_bytes());
    data.extend_from_slice(&o.to_le_bytes());
    data.push(9);
    rt.send(&[ix(PER_POOL_ID, data, vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(pool, false),
        AccountMeta::new(vault, false), AccountMeta::new(src, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
    ])]).await
}
async fn lp_fund_a(rt: &mut Rt, global: Pubkey, escrow: Pubkey, etok: Pubkey, mint: Pubkey, pools: &[(Pubkey, Pubkey, u64)]) -> Sent {
    let batch = [3u64, 4, 5, 6].into_iter().find(|b| pda_a(&[b"xa", b"escrow", &b.to_le_bytes()]) == escrow).unwrap();
    let mut data = vec![9];
    data.extend_from_slice(&batch.to_le_bytes());
    data.push(pools.len() as u8);
    for (_, _, amt) in pools { data.extend_from_slice(&amt.to_le_bytes()); }
    let mut metas = vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(escrow, false),
        AccountMeta::new(etok, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    for (p, v, _) in pools {
        metas.push(AccountMeta::new(*p, false));
        metas.push(AccountMeta::new(*v, false));
    }
    rt.send(&[ix(PER_POOL_ID, data, metas)]).await
}
async fn lp_consume_a(rt: &mut Rt, global: Pubkey, escrow: Pubkey, etok: Pubkey, dest: Pubkey, mint: Pubkey, batch: u64) -> Sent {
    let mut data = vec![10];
    data.extend_from_slice(&batch.to_le_bytes());
    rt.send(&[ix(PER_POOL_ID, data, vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(escrow, false),
        AccountMeta::new(etok, false), AccountMeta::new(dest, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
    ])]).await
}
async fn lp_refund_a(rt: &mut Rt, global: Pubkey, escrow: Pubkey, etok: Pubkey, mint: Pubkey, pools: &[(Pubkey, Pubkey)]) -> Sent {
    let batch = [4u64, 3, 5].into_iter().find(|b| pda_a(&[b"xa", b"escrow", &b.to_le_bytes()]) == escrow).unwrap();
    let mut data = vec![11];
    data.extend_from_slice(&batch.to_le_bytes());
    let mut metas = vec![
        AccountMeta::new(rt.payer(), true), AccountMeta::new_readonly(global, false), AccountMeta::new(escrow, false),
        AccountMeta::new(etok, false), AccountMeta::new_readonly(mint, false), AccountMeta::new_readonly(spl_token::ID, false),
    ];
    for (p, v) in pools {
        metas.push(AccountMeta::new(*p, false));
        metas.push(AccountMeta::new(*v, false));
    }
    rt.send(&[ix(PER_POOL_ID, data, metas)]).await
}

async fn tok(rt: &mut Rt, k: &Pubkey) -> u64 { amount_of(&rt.acct(k).await.unwrap()) }
async fn pool_pots(rt: &mut Rt, pool: &Pubkey) -> (u64, u64, u64) {
    let a = rt.acct(pool).await.unwrap();
    (u64_at(&a.data, 72), u64_at(&a.data, 80), u64_at(&a.data, 88))
}
async fn pool_pots_a(rt: &mut Rt, pool: &Pubkey) -> (u64, u64, u64) { pool_pots(rt, pool).await }
async fn reserve_totals(rt: &mut Rt, reserve: &Pubkey) -> (u64, u64, u64, u64) {
    let a = rt.acct(reserve).await.unwrap();
    (u64_at(&a.data, 48), u64_at(&a.data, 56), u64_at(&a.data, 64), a.data[0] as u64)
}
async fn qs(rt: &mut Rt, reserve: &Pubkey) -> String {
    format!("state={}", rt.acct(reserve).await.unwrap().data[0])
}
fn u64_at(d: &[u8], o: usize) -> u64 { u64::from_le_bytes(d[o..o+8].try_into().unwrap()) }
