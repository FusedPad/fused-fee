//! Old/new register caller against one loaded fused_fee SBF binary.
//!
//! Account index 7 is remaining_accounts[0] (the quote mint). A 9th account, when
//! present, is remaining_accounts[1] (QuoteStat). This file does not change the
//! program. Point FUSED_FEE_SO at the binary and MATRIX_LABEL at a name.
//! 80060ab and the current tree are separate runs.

use solana_account::{Account, AccountSharedData};
use solana_instruction::{AccountMeta, Instruction};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_pubkey::Pubkey;
use solana_rent::Rent;
use solana_signer::Signer;
use solana_system_interface::program as system_program;
use solana_transaction::Transaction;
use std::path::PathBuf;

const BPF_LOADER: Pubkey = solana_pubkey::pubkey!("BPFLoader2111111111111111111111111111111111");
const DBC_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
const COMPUTE_BUDGET: Pubkey = solana_pubkey::pubkey!("ComputeBudget111111111111111111111111111111");
const MEMO_PROGRAM: Pubkey = solana_pubkey::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

const DBC_POOL_DISC: [u8; 8] = [213, 224, 5, 209, 98, 69, 119, 92];
const DBC_CONFIG_DISC: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];
const IX_INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const IX_REGISTER_CONFIG: [u8; 8] = [32, 247, 82, 131, 35, 183, 7, 57];
const IX_REGISTER_POOL: [u8; 8] = [85, 229, 114, 47, 75, 145, 166, 100];
const IX_SYNC_QSTAT: [u8; 8] = [130, 168, 89, 25, 125, 164, 73, 104];

const ERR_BAD_MINT: u32 = 6000;
const ERR_QUOTE_MISMATCH: u32 = 6002;
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

fn spl_mint_bytes(decimals: u8, authority: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 82];
    d[0..4].copy_from_slice(&1u32.to_le_bytes());
    d[4..36].copy_from_slice(authority.as_ref());
    d[44] = decimals;
    d[45] = 1;
    let _ = authority;
    d
}

fn dbc_pool_bytes(config: &Pubkey, creator: &Pubkey, base: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 400];
    d[..8].copy_from_slice(&DBC_POOL_DISC);
    d[72..104].copy_from_slice(config.as_ref());
    d[104..136].copy_from_slice(creator.as_ref());
    d[136..168].copy_from_slice(base.as_ref());
    d
}

fn dbc_config_bytes(quote: &Pubkey, fee_claimer: &Pubkey) -> Vec<u8> {
    let mut d = vec![0u8; 256];
    d[..8].copy_from_slice(&DBC_CONFIG_DISC);
    d[8..40].copy_from_slice(quote.as_ref());
    d[40..72].copy_from_slice(fee_claimer.as_ref());
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

struct Rt {
    ctx: ProgramTestContext,
    slot: u64,
}

struct Sent {
    ok: bool,
    err: Option<u32>,
    logs: Vec<String>,
    cu: u64,
}

impl Rt {
    async fn send(&mut self, ixs: &[Instruction]) -> Sent {
        self.slot += 1;
        let _ = self.ctx.warp_to_slot(self.slot);
        let bh = self.ctx.banks_client.get_latest_blockhash().await.unwrap();
        let payer = self.ctx.payer.insecure_clone();
        let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), &[&payer], bh);
        let wire = bincode::serialize(&tx).expect("tx bytes");
        match self.ctx.banks_client.process_transaction_with_metadata(tx).await {
            Ok(r) => {
                let logs = r.metadata.as_ref().map(|m| m.log_messages.clone()).unwrap_or_default();
                let cu = r.metadata.as_ref().map(|m| m.compute_units_consumed).unwrap_or(0);
                match r.result {
                    Ok(()) => Sent { ok: true, err: None, logs, cu },
                    Err(e) => {
                        let text = format!("{e:?} logs={logs:?} wire={}", wire.len());
                        Sent { ok: false, err: custom_code(&text), logs, cu }
                    }
                }
            }
            Err(e) => Sent {
                ok: false,
                err: None,
                logs: vec![format!("client {e:?}")],
                cu: 0,
            },
        }
    }

    async fn account(&mut self, key: &Pubkey) -> Option<Account> {
        self.ctx.banks_client.get_account(*key).await.unwrap()
    }

    fn store(&mut self, key: &Pubkey, account: Account) {
        self.ctx.set_account(key, &AccountSharedData::from(account));
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
    None
}

fn locate_so() -> PathBuf {
    if let Ok(p) = std::env::var("FUSED_FEE_SO") {
        let path = PathBuf::from(p);
        assert!(path.is_file(), "FUSED_FEE_SO missing: {}", path.display());
        return path;
    }
    PathBuf::from("/workspace/target/deploy/fused_fee.so")
}

fn program_test() -> ProgramTest {
    let path = locate_so();
    let elf = std::fs::read(&path).expect("read so");
    eprintln!("MATRIX_SO bytes={} sha_prefix path={}", elf.len(), path.display());
    let mut pt = ProgramTest::default();
    pt.prefer_bpf(false);
    pt.set_compute_max_units(1_400_000);
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
    pt
}

fn register_data(pool: &Pubkey, base: &Pubkey, quote: &Pubkey) -> Vec<u8> {
    let mut data = Vec::with_capacity(96);
    data.extend_from_slice(pool.as_ref());
    data.extend_from_slice(base.as_ref());
    data.extend_from_slice(quote.as_ref());
    assert_eq!(8 + data.len(), 104);
    data
}

/// Live client order. Index 7 is the quote mint and is remaining_accounts[0].
/// `qs` appends remaining_accounts[1]. `tail` appends anything after that.
fn register_ix(
    payer: Pubkey,
    global: Pubkey,
    quote: Pubkey,
    base: Pubkey,
    config: Pubkey,
    pool: Pubkey,
    index7: Pubkey,
    qs: Option<(Pubkey, bool)>,
    tail: &[AccountMeta],
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
        AccountMeta::new_readonly(index7, false),
    ];
    assert_eq!(metas.len(), 8, "live registerPoolInstruction length");
    if let Some((qs, writable)) = qs {
        metas.push(if writable {
            AccountMeta::new(qs, false)
        } else {
            AccountMeta::new_readonly(qs, false)
        });
    }
    metas.extend_from_slice(tail);
    ix(IX_REGISTER_POOL, &register_data(&pool, &base, &quote), metas)
}

fn sync_ix(admin: Pubkey, global: Pubkey, quote: Pubkey, count: u64) -> Instruction {
    let (qs, _) = pda(&[b"qstat", quote.as_ref()]);
    let mut data = Vec::new();
    data.extend_from_slice(quote.as_ref());
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

fn register_config_ix(admin: Pubkey, global: Pubkey, quote: Pubkey, config: Pubkey) -> Instruction {
    let (config_record, _) = pda(&[b"config", quote.as_ref()]);
    let data = quote.to_bytes().to_vec();
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

async fn qs_state(rt: &mut Rt, quote: &Pubkey) -> String {
    let (key, _) = pda(&[b"qstat", quote.as_ref()]);
    match rt.account(&key).await {
        None => "absent".to_string(),
        Some(a) if a.data.is_empty() => format!("empty owner={} lamports={}", a.owner, a.lamports),
        Some(a) => {
            let count = if a.data.len() >= 48 {
                u64::from_le_bytes(a.data[40..48].try_into().unwrap()).to_string()
            } else {
                format!("short-len-{}", a.data.len())
            };
            format!("count={count} len={} owner={}", a.data.len(), a.owner)
        }
    }
}

async fn pool_exists(rt: &mut Rt, pool: &Pubkey) -> bool {
    let (key, _) = pda(&[b"pool", pool.as_ref()]);
    matches!(rt.account(&key).await, Some(a) if a.owner == pid() && !a.data.is_empty())
}

struct QuoteFx {
    quote: Pubkey,
    config: Pubkey,
    base: Pubkey,
}

async fn new_quote(rt: &mut Rt, global: &Pubkey) -> QuoteFx {
    let quote = Pubkey::new_unique();
    let config = Pubkey::new_unique();
    let base = Pubkey::new_unique();
    let auth = Pubkey::new_unique();
    rt.store(&quote, owned(spl_token::ID, spl_mint_bytes(9, &auth)));
    rt.store(&base, owned(spl_token::ID, spl_mint_bytes(6, &auth)));
    rt.store(&config, owned(DBC_PROGRAM_ID, dbc_config_bytes(&quote, global)));
    let admin = rt.ctx.payer.pubkey();
    let sent = rt.send(&[register_config_ix(admin, *global, quote, config)]).await;
    assert!(sent.ok, "register_config {:?}", sent.logs);
    QuoteFx { quote, config, base }
}

fn place_pool(rt: &mut Rt, fx: &QuoteFx, pool: &Pubkey) {
    let creator = Pubkey::new_unique();
    rt.store(pool, owned(DBC_PROGRAM_ID, dbc_pool_bytes(&fx.config, &creator, &fx.base)));
}

struct Row {
    case_id: String,
    caller: String,
    counter: String,
    ok: bool,
    err: String,
    pool: bool,
    qs: String,
    cu: u64,
}

fn push(rows: &mut Vec<Row>, case: &str, caller: &str, counter: &str, sent: &Sent, pool: bool, qs: String) {
    rows.push(Row {
        case_id: case.to_string(),
        caller: caller.to_string(),
        counter: counter.to_string(),
        ok: sent.ok,
        err: sent.err.map(|n| n.to_string()).unwrap_or_else(|| if sent.ok { "none".into() } else { "unparsed".into() }),
        pool,
        qs: qs.clone(),
        cu: sent.cu,
    });
    eprintln!(
        "CASE {case} caller={caller} counter={counter} ok={} err={} pool={pool} qs={qs} cu={}",
        sent.ok,
        sent.err.map(|n| n.to_string()).unwrap_or_else(|| "-".into()),
        sent.cu
    );
}

#[tokio::test]
async fn register_caller_matrix() {
    let label = std::env::var("MATRIX_LABEL").unwrap_or_else(|_| "unlabeled".into());
    let so = locate_so();
    let elf = std::fs::read(&so).unwrap();
    let sha = sha256(&elf);
    eprintln!("MATRIX_LABEL={label} sha256={sha} bytes={}", elf.len());

    let mut rt = {
        let mut ctx = program_test().start_with_context().await;
        let _ = ctx.warp_to_slot(1);
        Rt { ctx, slot: 1 }
    };
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
    let sent = rt.send(&[init]).await;
    assert!(sent.ok, "initialize {:?}", sent.logs);

    let mut rows = Vec::new();

    // Old caller: exactly 8 accounts, quote at index 7, no QuoteStat.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, None, &[])])
            .await;
        let pool_ok = pool_exists(&mut rt, &pool).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "old8-absent", "8", "absent", &sent, pool_ok, qs);
    }

    // New caller, QuoteStat address included, account does not exist yet.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, Some((qs, true)), &[])])
            .await;
        let pool_ok = pool_exists(&mut rt, &pool).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "new9-empty", "9-writable", "empty-pda", &sent, pool_ok, qs);
    }

    // 9th account read-only. Must not create the pool.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, Some((qs, false)), &[])])
            .await;
        let pool_ok = pool_exists(&mut rt, &pool).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "new9-readonly", "9-readonly", "empty-pda", &sent, pool_ok, qs);
    }

    // Wrong pubkey at index 7. The data quote is the real one. Index 7 is what the program checks.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let wrong = Pubkey::new_unique();
        rt.store(&wrong, owned(spl_token::ID, spl_mint_bytes(9, &admin)));
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, wrong, None, &[])])
            .await;
        let pool_ok = pool_exists(&mut rt, &pool).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "index7-not-quote-8", "8-wrong-index7", "absent", &sent, pool_ok, qs);
        assert!(!pool_ok, "index 7 mismatch must roll the pool back");
    }

    // Index 7 is the quote check, but only after a 9th account is present on the
    // build that rejects a missing QuoteStat first. Both builds must reject this.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let wrong = Pubkey::new_unique();
        rt.store(&wrong, owned(spl_token::ID, spl_mint_bytes(9, &admin)));
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let _ = rt.send(&[sync_ix(admin, global, fx.quote, 0)]).await;
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, wrong, Some((qs, true)), &[])])
            .await;
        let pool_ok = pool_exists(&mut rt, &pool).await;
        let qs_s = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "index7-not-quote-9", "9-wrong-index7", "existing-0", &sent, pool_ok, qs_s);
        assert!(!pool_ok, "wrong index 7 must roll the pool back");
        assert_eq!(sent.err, Some(ERR_BAD_MINT), "index7 with qs {:?}", sent.logs);
    }

    // Existing initialized counter. sync sets the absolute value, then one register.
    {
        let fx = new_quote(&mut rt, &global).await;
        let synced = rt.send(&[sync_ix(admin, global, fx.quote, 4)]).await;
        assert!(synced.ok, "sync {:?}", synced.logs);
        assert!(qs_state(&mut rt, &fx.quote).await.contains("count=4"), "{}", qs_state(&mut rt, &fx.quote).await);
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, Some((qs, true)), &[])])
            .await;
        let pool_ok = pool_exists(&mut rt, &pool).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "new9-existing-4", "9-writable", "existing-4", &sent, pool_ok, qs);
    }

    // Stale counter: absolute sync to 0 after an 8-account pool, then a 9-account register.
    // The 8-account half is itself a matrix row. The follow-up shows increment does not recount.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool1 = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool1);
        let first = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool1, fx.quote, None, &[])])
            .await;
        let pool1_ok = pool_exists(&mut rt, &pool1).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "stale-setup-old8", "8", "absent", &first, pool1_ok, qs);
        if pool1_ok {
            let synced = rt.send(&[sync_ix(admin, global, fx.quote, 0)]).await;
            assert!(synced.ok, "stale sync {:?}", synced.logs);
            let pool2 = Pubkey::new_unique();
            place_pool(&mut rt, &fx, &pool2);
            let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
            let sent = rt
                .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool2, fx.quote, Some((qs, true)), &[])])
                .await;
            let pool2_ok = pool_exists(&mut rt, &pool2).await;
            let qs = qs_state(&mut rt, &fx.quote).await;
            push(&mut rows, "stale-then-9", "9-writable", "synced-0-but-1-pool-already", &sent, pool2_ok, qs);
            let repair = rt.send(&[sync_ix(admin, global, fx.quote, if pool2_ok { 2 } else { 1 })]).await;
            assert!(repair.ok, "repair sync {:?}", repair.logs);
            let qs = qs_state(&mut rt, &fx.quote).await;
            push(
                &mut rows,
                "sync-overwrite",
                "sync",
                "absolute",
                &repair,
                pool_exists(&mut rt, &pool1).await,
                qs,
            );
        }
    }

    // Initialized but wrong absolute count. Register only adds one. It does not recount pools.
    {
        let fx = new_quote(&mut rt, &global).await;
        let synced = rt.send(&[sync_ix(admin, global, fx.quote, 7)]).await;
        assert!(synced.ok, "stale absolute {:?}", synced.logs);
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, Some((qs, true)), &[])])
            .await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "stale-absolute-7", "9-writable", "synced-7-zero-pools", &sent, pool_exists(&mut rt, &pool).await, qs);
    }

    // Brand-new quote: sync 0, then register, as two transactions.
    {
        let fx = new_quote(&mut rt, &global).await;
        let synced = rt.send(&[sync_ix(admin, global, fx.quote, 0)]).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "sync-new-quote-0", "sync", "create-0", &synced, false, qs);
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, Some((qs, true)), &[])])
            .await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "register-after-sync-0", "9-writable", "existing-0", &sent, pool_exists(&mut rt, &pool).await, qs);
    }

    // Same two steps in one transaction.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let sent = rt
            .send(&[
                sync_ix(admin, global, fx.quote, 0),
                register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, Some((qs, true)), &[]),
            ])
            .await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "sync-and-register-one-tx", "sync+9", "empty-then-0", &sent, pool_exists(&mut rt, &pool).await, qs);
    }

    // Two registers, one transaction, same quote. Second pool is a different pool.
    {
        let fx = new_quote(&mut rt, &global).await;
        let _ = rt.send(&[sync_ix(admin, global, fx.quote, 0)]).await;
        let p1 = Pubkey::new_unique();
        let p2 = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &p1);
        place_pool(&mut rt, &fx, &p2);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let sent = rt
            .send(&[
                register_ix(admin, global, fx.quote, fx.base, fx.config, p1, fx.quote, Some((qs, true)), &[]),
                register_ix(admin, global, fx.quote, fx.base, fx.config, p2, fx.quote, Some((qs, true)), &[]),
            ])
            .await;
        let both = pool_exists(&mut rt, &p1).await && pool_exists(&mut rt, &p2).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "two-registers-one-tx", "9+9", "existing-0", &sent, both, qs);
    }

    // Same transaction, second instruction is a bad pool. Both must roll back.
    {
        let fx = new_quote(&mut rt, &global).await;
        let _ = rt.send(&[sync_ix(admin, global, fx.quote, 0)]).await;
        let before = qs_state(&mut rt, &fx.quote).await;
        let p1 = Pubkey::new_unique();
        let p2 = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &p1);
        place_pool(&mut rt, &fx, &p2);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let bad_base = Pubkey::new_unique();
        let sent = rt
            .send(&[
                register_ix(admin, global, fx.quote, fx.base, fx.config, p1, fx.quote, Some((qs, true)), &[]),
                register_ix(admin, global, fx.quote, bad_base, fx.config, p2, fx.quote, Some((qs, true)), &[]),
            ])
            .await;
        let leftover = pool_exists(&mut rt, &p1).await || pool_exists(&mut rt, &p2).await;
        let qs_after = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "rollback-second-ix", "9+9-bad", "existing-0", &sent, leftover, format!("{qs_after} before={before}"));
        assert!(!sent.ok, "bad second register must fail the tx");
        assert!(!leftover, "first pool must not survive a failed tx");
        assert_eq!(qs_after, before, "counter must not survive a failed tx");
    }

    // Failure before the counter write: wrong QuoteStat address. Pool must not remain.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let wrong = Pubkey::new_unique();
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, Some((wrong, true)), &[])])
            .await;
        let pool_ok = pool_exists(&mut rt, &pool).await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "rollback-wrong-qs", "9-wrong-pda", "absent", &sent, pool_ok, qs);
        assert!(!pool_ok, "wrong QuoteStat must not leave a pool");
    }

    // A 10th account after a valid 9th. Not a live caller. Records whether the program ignores it.
    {
        let fx = new_quote(&mut rt, &global).await;
        let _ = rt.send(&[sync_ix(admin, global, fx.quote, 1)]).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let extra = Pubkey::new_unique();
        let sent = rt
            .send(&[register_ix(
                admin,
                global,
                fx.quote,
                fx.base,
                fx.config,
                pool,
                fx.quote,
                Some((qs, true)),
                &[AccountMeta::new_readonly(extra, false)],
            )])
            .await;
        let qs = qs_state(&mut rt, &fx.quote).await;
        push(&mut rows, "tenth-ignored-or-not", "10", "existing-1", &sent, pool_exists(&mut rt, &pool).await, qs);
    }

    // Direct empty-account failure must not leave the pool. Separate from new9-empty so the
    // assertion is explicit when the program rejects uninitialized QuoteStat.
    {
        let fx = new_quote(&mut rt, &global).await;
        let pool = Pubkey::new_unique();
        place_pool(&mut rt, &fx, &pool);
        let (qs, _) = pda(&[b"qstat", fx.quote.as_ref()]);
        let sent = rt
            .send(&[register_ix(admin, global, fx.quote, fx.base, fx.config, pool, fx.quote, Some((qs, true)), &[])])
            .await;
        if !sent.ok {
            assert!(!pool_exists(&mut rt, &pool).await, "rejected register left a pool");
            let qs = qs_state(&mut rt, &fx.quote).await;
            assert_eq!(qs, "absent", "rejected register created QuoteStat: {qs}");
        }
        let _ = (ERR_QSTAT_REQUIRED, ERR_QSTAT_UNINIT, ERR_QUOTE_MISMATCH);
    }

    write_rows(&label, &sha, elf.len(), &rows);
}

fn write_rows(label: &str, sha: &str, bytes: usize, rows: &[Row]) {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"label\": \"{label}\",\n"));
    out.push_str(&format!("  \"so_sha256\": \"{sha}\",\n"));
    out.push_str(&format!("  \"so_bytes\": {bytes},\n"));
    out.push_str("  \"rows\": [\n");
    for (i, r) in rows.iter().enumerate() {
        let comma = if i + 1 == rows.len() { "" } else { "," };
        out.push_str(&format!(
            "    {{\"case\":\"{}\",\"caller\":\"{}\",\"counter\":\"{}\",\"ok\":{},\"err\":\"{}\",\"pool\":{},\"qs\":\"{}\",\"cu\":{}}}{comma}\n",
            r.case_id, r.caller, r.counter, r.ok, r.err, r.pool, r.qs, r.cu
        ));
    }
    out.push_str("  ]\n}\n");
    let dir = PathBuf::from("/workspace/experiments/results");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("register-caller-{label}.json"));
    std::fs::write(&path, &out).unwrap();
    eprintln!("WROTE {}", path.display());
}

fn sha256(bytes: &[u8]) -> String {
    let digest = solana_sha256_hasher::hash(bytes).to_bytes();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn legacy_len(ixs: &[Instruction], payer: &Pubkey) -> (usize, usize) {
    let tx = Transaction::new_with_payer(ixs, Some(payer));
    let wire = bincode::serialize(&tx).unwrap();
    (wire.len(), tx.message.account_keys.len())
}

fn price_ix() -> Instruction {
    let mut data = vec![3u8];
    data.extend_from_slice(&100_000u64.to_le_bytes());
    Instruction { program_id: COMPUTE_BUDGET, accounts: vec![], data }
}

fn limit_ix(units: u32) -> Instruction {
    let mut data = vec![2u8];
    data.extend_from_slice(&units.to_le_bytes());
    Instruction { program_id: COMPUTE_BUDGET, accounts: vec![], data }
}

fn memo_ix(signer: &Pubkey) -> Instruction {
    Instruction {
        program_id: MEMO_PROGRAM,
        accounts: vec![AccountMeta::new_readonly(*signer, true)],
        data: b"fusedpad".to_vec(),
    }
}

fn one_register(payer: &Pubkey, with_qs: bool, n_tag: u8) -> Instruction {
    // Distinct pubkeys so the size is the client's worst shape, not a shared-key shortcut.
    let quote = Pubkey::new_from_array([n_tag; 32]);
    let pool = Pubkey::new_from_array([n_tag.wrapping_add(1); 32]);
    let base = Pubkey::new_from_array([n_tag.wrapping_add(2); 32]);
    let config = Pubkey::new_from_array([n_tag.wrapping_add(3); 32]);
    let global = Pubkey::new_from_array([9u8; 32]);
    let (config_record, _) = Pubkey::find_program_address(&[b"config", quote.as_ref()], &pid());
    let (pool_record, _) = Pubkey::find_program_address(&[b"pool", pool.as_ref()], &pid());
    let mut metas = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(config_record, false),
        AccountMeta::new(pool_record, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(global, false),
        AccountMeta::new_readonly(pool, false),
        AccountMeta::new_readonly(config, false),
        AccountMeta::new_readonly(quote, false),
    ];
    if with_qs {
        let (qs, _) = Pubkey::find_program_address(&[b"qstat", quote.as_ref()], &pid());
        metas.push(AccountMeta::new(qs, false));
    }
    ix(IX_REGISTER_POOL, &register_data(&pool, &base, &quote), metas)
}

#[test]
fn register_message_sizes() {
    let payer = Pubkey::new_from_array([7u8; 32]);
    let r8 = one_register(&payer, false, 20);
    let r9 = one_register(&payer, true, 21);
    let (b8, l8) = legacy_len(&[r8.clone()], &payer);
    let (b9, l9) = legacy_len(&[r9.clone()], &payer);
    let (p8, _) = legacy_len(&[price_ix(), limit_ix(200_000), r8.clone()], &payer);
    let (p9, _) = legacy_len(&[price_ix(), limit_ix(200_000), r9.clone()], &payer);
    let memo_signer = Pubkey::new_from_array([8u8; 32]);
    // Memo adds a second signer, so the serialized tx grows by a signature as well as the memo.
    let (m8, _) = legacy_len(&[memo_ix(&memo_signer), price_ix(), limit_ix(200_000), r8], &payer);
    let (m9, _) = legacy_len(&[memo_ix(&memo_signer), price_ix(), limit_ix(200_000), r9.clone()], &payer);
    let (bundle, bundle_locks) = legacy_len(&[sync_placeholder(&payer), r9], &payer);
    eprintln!("SIZE bare8={b8} locks={l8} bare9={b9} locks={l9} delta={}", b9 - b8);
    eprintln!("SIZE price+limit 8={p8} 9={p9}");
    eprintln!("SIZE memo+price+limit 8={m8} 9={m9} (two signatures)");
    eprintln!("SIZE sync+register={bundle} locks={bundle_locks}");
    assert_eq!(b9 - b8, 33, "one new key is 32 bytes plus one account index");
    assert!(b8 < 1232 && b9 < 1232 && p9 < 1232, "a single register still fits");

    // /admin shape: one legacy tx, no compute budget, every coin its own quote.
    let mut fit8 = 0usize;
    let mut fit9 = 0usize;
    for n in 1..=12 {
        let mut ixs8 = Vec::new();
        let mut ixs9 = Vec::new();
        for i in 0..n {
            ixs8.push(one_register(&payer, false, 30 + i as u8));
            ixs9.push(one_register(&payer, true, 80 + i as u8));
        }
        let (s8, k8) = legacy_len(&ixs8, &payer);
        let (s9, k9) = legacy_len(&ixs9, &payer);
        eprintln!("ADMIN n={n} bytes8={s8} locks8={k8} bytes9={s9} locks9={k9}");
        if s8 <= 1232 {
            fit8 = n;
        }
        if s9 <= 1232 {
            fit9 = n;
        }
    }
    let mut out = format!(
        "{{\"bare8\":{b8},\"locks8\":{l8},\"bare9\":{b9},\"locks9\":{l9},\"delta\":{},\"with_budget8\":{p8},\"with_budget9\":{p9},\"with_memo_budget8\":{m8},\"with_memo_budget9\":{m9},\"sync_plus_register\":{bundle},\"sync_plus_register_locks\":{bundle_locks},\"admin_fit8\":{fit8},\"admin_fit9\":{fit9}}}\n",
        b9 - b8
    );
    let _ = out.pop();
    std::fs::create_dir_all("/workspace/experiments/results").unwrap();
    std::fs::write("/workspace/experiments/results/register-message-sizes.json", out).unwrap();
}

fn sync_placeholder(payer: &Pubkey) -> Instruction {
    let quote = Pubkey::new_from_array([21u8; 32]);
    let global = Pubkey::new_from_array([9u8; 32]);
    let (qs, _) = Pubkey::find_program_address(&[b"qstat", quote.as_ref()], &pid());
    let mut data = Vec::new();
    data.extend_from_slice(quote.as_ref());
    data.extend_from_slice(&0u64.to_le_bytes());
    ix(
        IX_SYNC_QSTAT,
        &data,
        vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(global, false),
            AccountMeta::new(qs, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    )
}
