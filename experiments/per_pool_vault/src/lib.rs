//! Experimental per-pool vault program. Not fused_fee and not a mainnet candidate.
//! Each pool's quote tokens sit in a token account owned by that pool's PDA.
//! A pay or burn touches that vault only. Auto-LP moves a bounded batch into an
//! escrow and can be consumed or refunded. Seeds start with "xa".

use exp_common::*;
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process);
solana_program::declare_id!("6YMEjhBqVTMaSRWcmVkLrnHZ22FWEDJEpTeonAg8GKSy");

fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.is_empty() {
        return Err(err(E_BAD_ACCOUNTS));
    }
    match data[0] {
        1 => init_global(program_id, accounts),
        2 => register_pool(program_id, accounts, data),
        3 => deposit(program_id, accounts, data),
        4 => outbound(program_id, accounts, data, 72),
        5 => outbound(program_id, accounts, data, 80),
        6 => outbound(program_id, accounts, data, 88),
        7 => set_paused(program_id, accounts, data),
        8 => migrate_in(program_id, accounts, data),
        9 => lp_fund(program_id, accounts, data),
        10 => lp_consume(program_id, accounts, data),
        11 => lp_refund(program_id, accounts, data),
        12 => close_pool(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn init_global(program_id: &Pubkey, a: &[AccountInfo]) -> ProgramResult {
    let (payer, global, system) = three(a)?;
    if !payer.is_signer {
        return Err(err(E_NOT_ADMIN));
    }
    let (key, bump) = Pubkey::find_program_address(&[b"xa", b"global"], program_id);
    if global.key != &key {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let bump_seed = [bump];
    create_pda(payer, global, system, GLOBAL_LEN, &[b"xa", b"global", &bump_seed], program_id)?;
    let mut d = global.data.borrow_mut();
    d[0..32].copy_from_slice(payer.key.as_ref());
    d[32] = bump;
    Ok(())
}

fn register_pool(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (payer, global, pool, vault, mint, system) = six(a)?;
    require_admin(global, payer)?;
    let pool_id = need_pubkey(data, 1)?;
    let (key, bump) = Pubkey::find_program_address(&[b"xa", b"pool", pool_id.as_ref()], program_id);
    if pool.key != &key {
        return Err(err(E_BAD_ACCOUNTS));
    }
    if token_owner(vault)? != key || token_mint(vault)? != *mint.key || token_amount(&vault.data.borrow())? != 0 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let bump_seed = [bump];
    create_pda(payer, pool, system, POOL_LEN, &[b"xa", b"pool", pool_id.as_ref(), &bump_seed], program_id)?;
    let mut d = pool.data.borrow_mut();
    d[0..32].copy_from_slice(mint.key.as_ref());
    d[32..64].copy_from_slice(pool_id.as_ref());
    d[64] = bump;
    Ok(())
}

fn deposit(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global, pool, vault, source, mint, token_program) = seven(a)?;
    require_admin(global, admin)?;
    require_unpaused(global)?;
    let _ = program_id;
    if data.len() < 11 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let amount = ru64(data, 1);
    let kind = data[9];
    let decimals = data[10];
    check_vault(pool, vault, mint)?;
    let before = token_amount(&vault.data.borrow())?;
    move_tokens(token_program, source, mint, vault, admin, amount, decimals, &[])?;
    let delta = sub(token_amount(&vault.data.borrow())?, before)?;
    if delta == 0 {
        return Err(err(E_MATH));
    }
    let (creator, holder, burn) = split(delta, kind)?;
    add_pots(pool, holder, burn, creator)?;
    msg!("xa_deposit delta={}", delta);
    Ok(())
}

fn outbound(program_id: &Pubkey, a: &[AccountInfo], data: &[u8], field: usize) -> ProgramResult {
    let (admin, global, pool, vault, dest, mint, token_program) = seven(a)?;
    require_admin(global, admin)?;
    require_unpaused(global)?;
    if data.len() < 10 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let amount = ru64(data, 1);
    let decimals = data[9];
    check_vault(pool, vault, mint)?;
    let d = pool.data.borrow();
    let have = ru64(&d, field);
    let sum = add(add(ru64(&d, 72), ru64(&d, 80))?, ru64(&d, 88))?;
    let bal = token_amount(&vault.data.borrow())?;
    drop(d);
    if bal < sum || amount > have || amount > bal {
        return Err(err(E_VAULT_SHORT));
    }
    let (key, bump) = pool_pda(pool, program_id)?;
    if vault_owner_ok(vault, &key)? == false {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let before = bal;
    let bump_seed = [bump];
    let id = pool_id(pool)?;
    move_tokens(token_program, vault, mint, dest, pool, amount, decimals, &[b"xa", b"pool", id.as_ref(), &bump_seed])?;
    if sub(before, token_amount(&vault.data.borrow())?)? != amount {
        return Err(err(E_MATH));
    }
    let mut d = pool.data.borrow_mut();
    wu64(&mut d, field, sub(have, amount)?);
    add_at(&mut d, 96, 1)?;
    msg!("xa_out amount={}", amount);
    Ok(())
}

fn migrate_in(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global, pool, vault, source, mint, token_program) = seven(a)?;
    require_admin(global, admin)?;
    let _ = program_id;
    if data.len() < 26 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let holder = ru64(data, 1);
    let burn = ru64(data, 9);
    let owed = ru64(data, 17);
    check_vault(pool, vault, mint)?;
    if pool.data.borrow()[65] != 0 {
        return Err(err(E_ALREADY));
    }
    let sum = add(add(holder, burn)?, owed)?;
    if sum > 0 {
        let before = token_amount(&vault.data.borrow())?;
        move_tokens(token_program, source, mint, vault, admin, sum, data[25], &[])?;
        if sub(token_amount(&vault.data.borrow())?, before)? != sum {
            return Err(err(E_MATH));
        }
    }
    add_pots(pool, holder, burn, owed)?;
    pool.data.borrow_mut()[65] = 1;
    msg!("xa_migrate sum={}", sum);
    Ok(())
}

fn set_paused(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global) = two(a)?;
    require_admin(global, admin)?;
    let (key, _) = Pubkey::find_program_address(&[b"xa", b"global"], program_id);
    if global.key != &key || data.len() < 2 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    global.data.borrow_mut()[33] = data[1];
    Ok(())
}

fn lp_fund(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 10 || a.len() < 7 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let admin = &a[0];
    let global = &a[1];
    let escrow = &a[2];
    let escrow_tok = &a[3];
    let mint = &a[4];
    let token_program = &a[5];
    let system = &a[6];
    require_admin(global, admin)?;
    require_unpaused(global)?;
    let batch = ru64(data, 1);
    let n = data[9] as usize;
    if n == 0 || n > MAX_BATCH || data.len() < 10 + n * 8 || a.len() < 7 + n * 2 {
        return Err(err(E_BATCH));
    }
    let (ekey, ebump) = Pubkey::find_program_address(&[b"xa", b"escrow", &batch.to_le_bytes()], program_id);
    if escrow.key != &ekey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let eb = [ebump];
    create_pda(admin, escrow, system, ESCROW_LEN, &[b"xa", b"escrow", &batch.to_le_bytes(), &eb], program_id)?;
    if token_owner(escrow_tok)? != ekey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let mut amounts = [0u64; MAX_BATCH];
    let mut pools = [Pubkey::default(); MAX_BATCH];
    for i in 0..n {
        let pool = &a[7 + i * 2];
        let vault = &a[8 + i * 2];
        let amount = ru64(data, 10 + i * 8);
        check_vault(pool, vault, mint)?;
        let burn = ru64(&pool.data.borrow(), 80);
        let sum = {
            let d = pool.data.borrow();
            add(add(ru64(&d, 72), ru64(&d, 80))?, ru64(&d, 88))?
        };
        if amount > lp_budget(burn) || token_amount(&vault.data.borrow())? < sum {
            return Err(err(E_BUDGET));
        }
        let (key, bump) = pool_pda(pool, program_id)?;
        let id = pool_id(pool)?;
        if key != *pool.key {
            return Err(err(E_BAD_ACCOUNTS));
        }
        let bump_seed = [bump];
        let before = token_amount(&vault.data.borrow())?;
        move_tokens(
            token_program,
            vault,
            mint,
            escrow_tok,
            pool,
            amount,
            mint.data.borrow()[44],
            &[b"xa", b"pool", id.as_ref(), &bump_seed],
        )?;
        if sub(before, token_amount(&vault.data.borrow())?)? != amount {
            return Err(err(E_MATH));
        }
        {
            let mut d = pool.data.borrow_mut();
            wu64(&mut d, 80, sub(burn, amount)?);
        }
        amounts[i] = amount;
        pools[i] = *pool.key;
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
    msg!("xa_lp_fund n={}", n);
    Ok(())
}

fn lp_consume(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global, escrow, escrow_tok, dest, mint, token_program) = seven(a)?;
    require_admin(global, admin)?;
    let batch = ru64(data, 1);
    let (ekey, ebump) = Pubkey::find_program_address(&[b"xa", b"escrow", &batch.to_le_bytes()], program_id);
    if escrow.key != &ekey {
        return Err(err(E_BAD_ACCOUNTS));
    }
    if escrow.data.borrow()[0] == 2 {
        return Ok(());
    }
    if escrow.data.borrow()[0] != 1 {
        return Err(err(E_ESCROW));
    }
    let amount = token_amount(&escrow_tok.data.borrow())?;
    let eb = [ebump];
    move_tokens(
        token_program,
        escrow_tok,
        mint,
        dest,
        escrow,
        amount,
        mint.data.borrow()[44],
        &[b"xa", b"escrow", &batch.to_le_bytes(), &eb],
    )?;
    escrow.data.borrow_mut()[0] = 2;
    Ok(())
}

fn lp_refund(program_id: &Pubkey, a: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (admin, global, escrow, escrow_tok, mint, token_program) = six(a)?;
    require_admin(global, admin)?;
    let batch = ru64(data, 1);
    let (ekey, ebump) = Pubkey::find_program_address(&[b"xa", b"escrow", &batch.to_le_bytes()], program_id);
    if escrow.key != &ekey || escrow.data.borrow()[0] != 1 {
        return Err(err(E_ESCROW));
    }
    let n = escrow.data.borrow()[2] as usize;
    if a.len() < 6 + n * 2 {
        return Err(err(E_BATCH));
    }
    let eb = [ebump];
    for i in 0..n {
        let pool = &a[6 + i * 2];
        let vault = &a[7 + i * 2];
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
            &[b"xa", b"escrow", &batch.to_le_bytes(), &eb],
        )?;
        let mut d = pool.data.borrow_mut();
        add_at(&mut d, 80, amount)?;
    }
    escrow.data.borrow_mut()[0] = 3;
    Ok(())
}

fn close_pool(program_id: &Pubkey, a: &[AccountInfo]) -> ProgramResult {
    let (admin, global, pool, vault) = four(a)?;
    require_admin(global, admin)?;
    let (key, _) = pool_pda(pool, program_id)?;
    if pool.key != &key {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let d = pool.data.borrow();
    if ru64(&d, 72) != 0 || ru64(&d, 80) != 0 || ru64(&d, 88) != 0 || token_amount(&vault.data.borrow())? != 0 {
        return Err(err(E_BAD_STATE));
    }
    drop(d);
    let src = pool.lamports();
    pool.realloc(0, false)?;
    **admin.lamports.borrow_mut() = admin.lamports().checked_add(src).ok_or(err(E_MATH))?;
    **pool.lamports.borrow_mut() = 0;
    Ok(())
}

fn check_vault(pool: &AccountInfo, vault: &AccountInfo, mint: &AccountInfo) -> ProgramResult {
    if token_mint(vault)? != *mint.key || quote_of(pool)? != *mint.key || token_owner(vault)? != *pool.key {
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

fn pool_id(pool: &AccountInfo) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::try_from(&pool.data.borrow()[32..64]).unwrap())
}

fn pool_pda(pool: &AccountInfo, program_id: &Pubkey) -> Result<(Pubkey, u8), ProgramError> {
    let id = pool_id(pool)?;
    Ok(Pubkey::find_program_address(&[b"xa", b"pool", id.as_ref()], program_id))
}

fn vault_owner_ok(vault: &AccountInfo, key: &Pubkey) -> Result<bool, ProgramError> {
    Ok(token_owner(vault)? == *key)
}

fn add_pots(pool: &AccountInfo, holder: u64, burn: u64, owed: u64) -> ProgramResult {
    let mut d = pool.data.borrow_mut();
    add_at(&mut d, 72, holder)?;
    add_at(&mut d, 80, burn)?;
    add_at(&mut d, 88, owed)?;
    add_at(&mut d, 96, 1)?;
    Ok(())
}

fn token_mint(ai: &AccountInfo) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::try_from(&ai.data.borrow()[0..32]).unwrap())
}
fn token_owner(ai: &AccountInfo) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::try_from(&ai.data.borrow()[32..64]).unwrap())
}
fn need_pubkey(data: &[u8], at: usize) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::try_from(data.get(at..at + 32).ok_or(err(E_BAD_ACCOUNTS))?).unwrap())
}
fn two<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 2 { return Err(err(E_BAD_ACCOUNTS)); }
    Ok((&a[0], &a[1]))
}
fn three<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 3 { return Err(err(E_BAD_ACCOUNTS)); }
    Ok((&a[0], &a[1], &a[2]))
}
fn four<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 4 { return Err(err(E_BAD_ACCOUNTS)); }
    Ok((&a[0], &a[1], &a[2], &a[3]))
}
fn six<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 6 { return Err(err(E_BAD_ACCOUNTS)); }
    Ok((&a[0], &a[1], &a[2], &a[3], &a[4], &a[5]))
}
fn seven<'a, 'info>(a: &'a [AccountInfo<'info>]) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>, &'a AccountInfo<'info>), ProgramError> {
    if a.len() < 7 { return Err(err(E_BAD_ACCOUNTS)); }
    Ok((&a[0], &a[1], &a[2], &a[3], &a[4], &a[5], &a[6]))
}
