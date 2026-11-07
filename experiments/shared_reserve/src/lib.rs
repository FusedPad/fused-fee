//! Experimental shared-vault program. Not fused_fee and not a mainnet candidate.
//! One quote vault, owned by this program's global PDA. A reserve account stores
//! the running sum of holder, burn, and creator liabilities. Every change updates
//! that sum in the same instruction. Seeds start with "xb" so they do not alias
//! fused_fee's "global", "pool", "qstat", or "cown" accounts.

use exp_common::*;
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process);
solana_program::declare_id!("6URwbPipuA4MJLG7LCRRZuWnms3JZ9cRG3z9indXWz8G");

fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.is_empty() {
        return Err(err(E_BAD_ACCOUNTS));
    }
    match data[0] {
        1 => init_global(program_id, accounts),
        2 => init_reserve(program_id, accounts),
        3 => register_pool(program_id, accounts, data),
        4 => import_pool(program_id, accounts, data),
        5 => seal(program_id, accounts),
        6 => deposit(program_id, accounts, data),
        7 => pay(program_id, accounts, data, Pot::Holder),
        8 => pay(program_id, accounts, data, Pot::Burn),
        9 => pay(program_id, accounts, data, Pot::Owed),
        10 => set_paused(program_id, accounts, data),
        11 => lp_fund(program_id, accounts, data),
        12 => lp_consume(program_id, accounts, data),
        13 => lp_refund(program_id, accounts, data),
        14 => close_reserve(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

enum Pot {
    Holder,
    Burn,
    Owed,
}

fn init_global(program_id: &Pubkey, a: &[AccountInfo]) -> ProgramResult {
    let (payer, global, system) = three(a)?;
    if !payer.is_signer {
        return Err(err(E_NOT_ADMIN));
    }
    let (key, bump) = Pubkey::find_program_address(&[b"xb", b"global"], program_id);
    if global.key != &key {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let bump_seed = [bump];
    create_pda(payer, global, system, GLOBAL_LEN, &[b"xb", b"global", &bump_seed], program_id)?;
    let mut d = global.data.borrow_mut();
    d[0..32].copy_from_slice(payer.key.as_ref());
    d[32] = bump;
    d[33] = 0;
    Ok(())
}

fn init_reserve(program_id: &Pubkey, a: &[AccountInfo]) -> ProgramResult {
    let (payer, global, reserve, mint, system) = five(a)?;
    require_admin(global, payer)?;
    let (gkey, _) = Pubkey::find_program_address(&[b"xb", b"global"], program_id);
    if global.key != &gkey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let (key, bump) = Pubkey::find_program_address(&[b"xb", b"reserve", mint.key.as_ref()], program_id);
    if reserve.key != &key {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let bump_seed = [bump];
    create_pda(payer, reserve, system, RESERVE_LEN, &[b"xb", b"reserve", mint.key.as_ref(), &bump_seed], program_id)?;
    let mut d = reserve.data.borrow_mut();
    d[0] = ST_FROZEN;
    d[1] = bump;
    d[8..40].copy_from_slice(mint.key.as_ref());
    Ok(())
}

fn register_pool(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (payer, global, reserve, pool, mint, system) = six(a)?;
    require_admin(global, payer)?;
    let pool_id = pubkey_at(data, 1)?;
    check_reserve_mint(reserve, mint.key)?;
    let state = reserve.data.borrow()[0];
    if state != ST_FROZEN && state != ST_BACKFILL && state != ST_LIVE {
        return Err(err(E_BAD_STATE));
    }
    let (key, bump) = Pubkey::find_program_address(&[b"xb", b"pool", pool_id.as_ref()], program_id);
    if pool.key != &key {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let bump_seed = [bump];
    create_pda(payer, pool, system, POOL_LEN, &[b"xb", b"pool", pool_id.as_ref(), &bump_seed], program_id)?;
    {
        let mut d = pool.data.borrow_mut();
        d[0..32].copy_from_slice(mint.key.as_ref());
        d[32..64].copy_from_slice(pool_id.as_ref());
        d[64] = bump;
        d[65] = 0;
    }
    let mut r = reserve.data.borrow_mut();
    let n = add(ru64(&r, 40), 1)?;
    wu64(&mut r, 40, n);
    let g = add(ru64(&r, 104), 1)?;
    wu64(&mut r, 104, g);
    msg!("xb_register pools={}", n);
    Ok(())
}

fn import_pool(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global, reserve, pool) = four(a)?;
    require_admin(global, admin)?;
    if data.len() < 1 + 24 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let holder = ru64(data, 1);
    let burn = ru64(data, 9);
    let owed = ru64(data, 17);
    let quote = {
        let pr = pool.data.borrow();
        if pr.len() != POOL_LEN || pr[65] != 0 {
            return Err(err(E_ALREADY));
        }
        let pool_id = Pubkey::try_from(&pr[32..64]).unwrap();
        let (expect, _) = Pubkey::find_program_address(&[b"xb", b"pool", pool_id.as_ref()], program_id);
        if pool.key != &expect {
            return Err(err(E_BAD_ACCOUNTS));
        }
        Pubkey::try_from(&pr[0..32]).unwrap()
    };
    check_reserve_mint(reserve, &quote)?;
    let mut r = reserve.data.borrow_mut();
    if r[0] != ST_FROZEN && r[0] != ST_BACKFILL {
        return Err(err(E_BAD_STATE));
    }
    r[0] = ST_BACKFILL;
    add_at(&mut r, 72, 1)?;
    add_at(&mut r, 80, holder)?;
    add_at(&mut r, 88, burn)?;
    add_at(&mut r, 96, owed)?;
    drop(r);
    let mut pr = pool.data.borrow_mut();
    if ru64(&pr, 72) != 0 || ru64(&pr, 80) != 0 || ru64(&pr, 88) != 0 {
        return Err(err(E_ALREADY));
    }
    wu64(&mut pr, 72, holder);
    wu64(&mut pr, 80, burn);
    wu64(&mut pr, 88, owed);
    pr[65] = 1;
    Ok(())
}

fn seal(program_id: &Pubkey, a: &[AccountInfo]) -> ProgramResult {
    let (admin, global, reserve, vault) = four(a)?;
    require_admin(global, admin)?;
    let (gkey, _) = Pubkey::find_program_address(&[b"xb", b"global"], program_id);
    let mut r = reserve.data.borrow_mut();
    if r[0] != ST_FROZEN && r[0] != ST_BACKFILL {
        return Err(err(E_BAD_STATE));
    }
    if ru64(&r, 72) != ru64(&r, 40) {
        return Err(err(E_COUNT));
    }
    let holder = ru64(&r, 80);
    let burn = ru64(&r, 88);
    let owed = ru64(&r, 96);
    let sum = add(add(holder, burn)?, owed)?;
    let mint = Pubkey::try_from(&r[8..40]).unwrap();
    let bal = token_amount(&vault.data.borrow())?;
    if token_mint(vault)? != mint || token_owner(vault)? != gkey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    if bal < sum {
        return Err(err(E_VAULT_SHORT));
    }
    wu64(&mut r, 48, holder);
    wu64(&mut r, 56, burn);
    wu64(&mut r, 64, owed);
    wu64(&mut r, 112, sub(bal, sum)?);
    r[0] = ST_LIVE;
    msg!("xb_seal pools={} sum={} surplus={}", ru64(&r, 40), sum, bal - sum);
    Ok(())
}

fn deposit(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global, reserve, pool, vault, source, mint, token_program) = eight(a)?;
    require_admin(global, admin)?;
    require_unpaused(global)?;
    if data.len() < 11 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let amount = ru64(data, 1);
    let kind = data[9];
    let decimals = data[10];
    require_live(reserve)?;
    let quote = quote_of(pool)?;
    check_reserve_mint(reserve, &quote)?;
    if mint.key != &quote {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let (gkey, gbump) = Pubkey::find_program_address(&[b"xb", b"global"], program_id);
    if token_owner(vault)? != gkey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let before = token_amount(&vault.data.borrow())?;
    // The user signs the source. No PDA seeds.
    let empty: &[&[u8]] = &[];
    move_tokens(token_program, source, mint, vault, admin, amount, decimals, empty)?;
    let delta = sub(token_amount(&vault.data.borrow())?, before)?;
    if delta == 0 {
        return Err(err(E_MATH));
    }
    let (creator, holder, burn) = split(delta, kind)?;
    add_pool(pool, holder, burn, creator)?;
    add_reserve(reserve, holder, burn, creator)?;
    let _ = gbump;
    msg!("xb_deposit delta={} holder={} burn={} creator={}", delta, holder, burn, creator);
    Ok(())
}

fn pay(program_id: &Pubkey, a: &[AccountInfo], data: &[u8], pot: Pot) -> ProgramResult {
    let (admin, global, reserve, pool, vault, dest, mint, token_program) = eight(a)?;
    require_admin(global, admin)?;
    require_unpaused(global)?;
    if data.len() < 10 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let amount = ru64(data, 1);
    let decimals = data[9];
    require_live(reserve)?;
    let quote = quote_of(pool)?;
    check_reserve_mint(reserve, &quote)?;
    if mint.key != &quote {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let (gkey, gbump) = Pubkey::find_program_address(&[b"xb", b"global"], program_id);
    if token_owner(vault)? != gkey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let (h, b, o) = pots(pool)?;
    let (rh, rb, ro) = reserve_totals(reserve)?;
    let sum = add(add(rh, rb)?, ro)?;
    let bal = token_amount(&vault.data.borrow())?;
    if bal < sum || amount > bal {
        return Err(err(E_VAULT_SHORT));
    }
    let (field, total_off, have, total_have) = match pot {
        Pot::Holder => (72usize, 48usize, h, rh),
        Pot::Burn => (80, 56, b, rb),
        Pot::Owed => (88, 64, o, ro),
    };
    if amount > have || amount > total_have {
        return Err(err(E_VAULT_SHORT));
    }
    let before = bal;
    let bump_seed = [gbump];
    move_tokens(token_program, vault, mint, dest, global, amount, decimals, &[b"xb", b"global", &bump_seed])?;
    let debit = sub(before, token_amount(&vault.data.borrow())?)?;
    if debit != amount {
        return Err(err(E_MATH));
    }
    {
        let mut d = pool.data.borrow_mut();
        wu64(&mut d, field, sub(have, amount)?);
        add_at(&mut d, 96, 1)?;
    }
    {
        let mut r = reserve.data.borrow_mut();
        wu64(&mut r, total_off, sub(total_have, amount)?);
        add_at(&mut r, 104, 1)?;
    }
    msg!("xb_out amount={}", amount);
    Ok(())
}

fn set_paused(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global) = two(a)?;
    require_admin(global, admin)?;
    let (gkey, _) = Pubkey::find_program_address(&[b"xb", b"global"], program_id);
    if global.key != &gkey || data.len() < 2 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    global.data.borrow_mut()[33] = data[1];
    Ok(())
}

fn lp_fund(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if a.len() < 9 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let admin = &a[0];
    let global = &a[1];
    let reserve = &a[2];
    let vault = &a[3];
    let escrow = &a[4];
    let escrow_tok = &a[5];
    let mint = &a[6];
    let token_program = &a[7];
    let system = &a[8];
    require_admin(global, admin)?;
    require_unpaused(global)?;
    require_live(reserve)?;
    if data.len() < 10 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let batch = ru64(data, 1);
    let n = data[9] as usize;
    if n == 0 || n > MAX_BATCH || data.len() < 10 + n * 8 || a.len() < 9 + n {
        return Err(err(E_BATCH));
    }
    let (ekey, ebump) = Pubkey::find_program_address(&[b"xb", b"escrow", &batch.to_le_bytes()], program_id);
    if escrow.key != &ekey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let eb = [ebump];
    create_pda(admin, escrow, system, ESCROW_LEN, &[b"xb", b"escrow", &batch.to_le_bytes(), &eb], program_id)?;
    let (gkey, gbump) = Pubkey::find_program_address(&[b"xb", b"global"], program_id);
    if token_owner(vault)? != gkey || token_owner(escrow_tok)? != ekey || mint.key != &quote_of_reserve(reserve)? {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let gb = [gbump];
    let mut spent = 0u64;
    let mut pools = [Pubkey::default(); MAX_BATCH];
    let mut amounts = [0u64; MAX_BATCH];
    for i in 0..n {
        let pool = &a[9 + i];
        let amount = ru64(data, 10 + i * 8);
        let (h, b, o) = pots(pool)?;
        let quote = quote_of(pool)?;
        check_reserve_mint(reserve, &quote)?;
        if amount > lp_budget(b) {
            return Err(err(E_BUDGET));
        }
        let (rh, rb, ro) = reserve_totals(reserve)?;
        let sum = add(add(rh, rb)?, ro)?;
        if token_amount(&vault.data.borrow())? < sum {
            return Err(err(E_VAULT_SHORT));
        }
        let before = token_amount(&vault.data.borrow())?;
        move_tokens(token_program, vault, mint, escrow_tok, global, amount, mint.data.borrow()[44], &[b"xb", b"global", &gb])?;
        if sub(before, token_amount(&vault.data.borrow())?)? != amount {
            return Err(err(E_MATH));
        }
        {
            let mut d = pool.data.borrow_mut();
            wu64(&mut d, 80, sub(b, amount)?);
            let _ = (h, o);
        }
        sub_reserve_burn(reserve, amount)?;
        pools[i] = *pool.key;
        amounts[i] = amount;
        spent = add(spent, amount)?;
    }
    let mut e = escrow.data.borrow_mut();
    e[0] = 1;
    e[1] = ebump;
    e[2] = n as u8;
    wu64(&mut e, 8, batch);
    e[16..48].copy_from_slice(mint.key.as_ref());
    for i in 0..n {
        wu64(&mut e, 48 + i * 8, amounts[i]);
        e[80 + i * 32..112 + i * 32].copy_from_slice(pools[i].as_ref());
    }
    msg!("xb_lp_fund spent={}", spent);
    Ok(())
}

fn lp_consume(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global, escrow, escrow_tok, dest, mint, token_program) = seven(a)?;
    require_admin(global, admin)?;
    let batch = ru64(data, 1);
    let (ekey, ebump) = Pubkey::find_program_address(&[b"xb", b"escrow", &batch.to_le_bytes()], program_id);
    if escrow.key != &ekey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let state = escrow.data.borrow()[0];
    if state == 2 {
        msg!("xb_lp_consume replay");
        return Ok(());
    }
    if state != 1 {
        return Err(err(E_ESCROW));
    }
    let amount = token_amount(&escrow_tok.data.borrow())?;
    let decimals = mint.data.borrow()[44];
    let eb = [ebump];
    move_tokens(token_program, escrow_tok, mint, dest, escrow, amount, decimals, &[b"xb", b"escrow", &batch.to_le_bytes(), &eb])?;
    escrow.data.borrow_mut()[0] = 2;
    msg!("xb_lp_consume amount={}", amount);
    Ok(())
}

fn lp_refund(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global, reserve, vault, escrow, escrow_tok, mint, token_program) = eight(a)?;
    require_admin(global, admin)?;
    if a.len() < 8 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let batch = ru64(data, 1);
    let (ekey, ebump) = Pubkey::find_program_address(&[b"xb", b"escrow", &batch.to_le_bytes()], program_id);
    if escrow.key != &ekey || escrow.data.borrow()[0] != 1 {
        return Err(err(E_ESCROW));
    }
    let n = escrow.data.borrow()[2] as usize;
    if a.len() < 8 + n {
        return Err(err(E_BATCH));
    }
    let (gkey, _) = Pubkey::find_program_address(&[b"xb", b"global"], program_id);
    if token_owner(vault)? != gkey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let eb = [ebump];
    for i in 0..n {
        let pool = &a[8 + i];
        let recorded = Pubkey::try_from(&escrow.data.borrow()[80 + i * 32..112 + i * 32]).unwrap();
        if pool.key != &recorded {
            return Err(err(E_ESCROW));
        }
        let amount = ru64(&escrow.data.borrow(), 48 + i * 8);
        move_tokens(
            token_program,
            escrow_tok,
            mint,
            vault,
            escrow,
            amount,
            mint.data.borrow()[44],
            &[b"xb", b"escrow", &batch.to_le_bytes(), &eb],
        )?;
        let mut d = pool.data.borrow_mut();
        add_at(&mut d, 80, amount)?;
        add_reserve(reserve, 0, amount, 0)?;
    }
    escrow.data.borrow_mut()[0] = 3;
    msg!("xb_lp_refund");
    Ok(())
}

fn close_reserve(program_id: &Pubkey, a: &[AccountInfo]) -> ProgramResult {
    let (admin, global, reserve) = three(a)?;
    require_admin(global, admin)?;
    let (key, _) = Pubkey::find_program_address(&[b"xb", b"reserve", &reserve.data.borrow()[8..40]], program_id);
    if reserve.key != &key {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let r = reserve.data.borrow();
    if ru64(&r, 40) != 0 || ru64(&r, 48) != 0 || ru64(&r, 56) != 0 || ru64(&r, 64) != 0 {
        return Err(err(E_BAD_STATE));
    }
    drop(r);
    let src = reserve.lamports();
    reserve.realloc(0, false)?;
    **admin.lamports.borrow_mut() = admin.lamports().checked_add(src).ok_or(err(E_MATH))?;
    **reserve.lamports.borrow_mut() = 0;
    Ok(())
}

fn require_live(reserve: &AccountInfo) -> ProgramResult {
    if reserve.data.borrow()[0] != ST_LIVE {
        return Err(err(E_FROZEN));
    }
    Ok(())
}

fn check_reserve_mint(reserve: &AccountInfo, mint: &Pubkey) -> ProgramResult {
    let d = reserve.data.borrow();
    if d.len() < RESERVE_LEN || &d[8..40] != mint.as_ref() {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok(())
}

fn quote_of(pool: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let d = pool.data.borrow();
    if d.len() != POOL_LEN {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok(Pubkey::try_from(&d[0..32]).unwrap())
}

fn quote_of_reserve(reserve: &AccountInfo) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::try_from(&reserve.data.borrow()[8..40]).unwrap())
}

fn pots(pool: &AccountInfo) -> Result<(u64, u64, u64), ProgramError> {
    let d = pool.data.borrow();
    Ok((ru64(&d, 72), ru64(&d, 80), ru64(&d, 88)))
}

fn reserve_totals(reserve: &AccountInfo) -> Result<(u64, u64, u64), ProgramError> {
    let d = reserve.data.borrow();
    Ok((ru64(&d, 48), ru64(&d, 56), ru64(&d, 64)))
}

fn add_pool(pool: &AccountInfo, holder: u64, burn: u64, owed: u64) -> ProgramResult {
    let mut d = pool.data.borrow_mut();
    add_at(&mut d, 72, holder)?;
    add_at(&mut d, 80, burn)?;
    add_at(&mut d, 88, owed)?;
    add_at(&mut d, 96, 1)?;
    Ok(())
}

fn add_reserve(reserve: &AccountInfo, holder: u64, burn: u64, owed: u64) -> ProgramResult {
    let mut r = reserve.data.borrow_mut();
    add_at(&mut r, 48, holder)?;
    add_at(&mut r, 56, burn)?;
    add_at(&mut r, 64, owed)?;
    add_at(&mut r, 104, 1)?;
    Ok(())
}

fn sub_reserve_burn(reserve: &AccountInfo, amount: u64) -> ProgramResult {
    let mut r = reserve.data.borrow_mut();
    sub_at(&mut r, 56, amount)?;
    add_at(&mut r, 104, 1)?;
    Ok(())
}

fn token_mint(ai: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let d = ai.data.borrow();
    if d.len() < 32 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok(Pubkey::try_from(&d[0..32]).unwrap())
}

fn token_owner(ai: &AccountInfo) -> Result<Pubkey, ProgramError> {
    let d = ai.data.borrow();
    if d.len() < 64 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok(Pubkey::try_from(&d[32..64]).unwrap())
}

fn pubkey_at(data: &[u8], at: usize) -> Result<Pubkey, ProgramError> {
    let s = data.get(at..at + 32).ok_or(err(E_BAD_ACCOUNTS))?;
    Ok(Pubkey::try_from(s).unwrap())
}

fn two<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 2 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok((&a[0], &a[1]))
}
fn three<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 3 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok((&a[0], &a[1], &a[2]))
}
fn four<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 4 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok((&a[0], &a[1], &a[2], &a[3]))
}
fn five<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 5 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok((&a[0], &a[1], &a[2], &a[3], &a[4]))
}
fn six<'a, 'info>(
    a: &'a [AccountInfo<'info>],
) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 6 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok((&a[0], &a[1], &a[2], &a[3], &a[4], &a[5]))
}
fn seven<'a, 'info>(
    a: &'a [AccountInfo<'info>],
) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError>
{
    if a.len() < 7 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok((&a[0], &a[1], &a[2], &a[3], &a[4], &a[5], &a[6]))
}
fn eight<'a, 'info>(
    a: &'a [AccountInfo<'info>],
) -> Result<
    (&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>),
    ProgramError,
> {
    if a.len() < 8 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok((&a[0], &a[1], &a[2], &a[3], &a[4], &a[5], &a[6], &a[7]))
}
