/// Experimental shared-reserve hooks. Appended into the shared-fee copy only.
const QRES_TAG: [u8; 8] = [169, 14, 71, 201, 233, 234, 76, 183];
const QRES_LEN: usize = 82;

fn qres_key(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"qres", mint.as_ref()], program_id)
}

fn qres_state(ai: &AccountInfo, mint: &Pubkey, program_id: &Pubkey) -> Result<u8> {
    let (key, _) = qres_key(mint, program_id);
    require_keys_eq!(ai.key(), key, FusedError::ReserveMismatch);
    require!(ai.is_writable, FusedError::ReserveMismatch);
    let d = ai.try_borrow_data()?;
    require!(d.len() == QRES_LEN && d[..8] == QRES_TAG, FusedError::ReserveMismatch);
    require!(&d[8..40] == mint.as_ref(), FusedError::ReserveMismatch);
    Ok(d[80])
}

fn qres_totals(ai: &AccountInfo) -> Result<(u64, u64, u64)> {
    let d = ai.try_borrow_data()?;
    Ok((
        u64::from_le_bytes(d[56..64].try_into().unwrap()),
        u64::from_le_bytes(d[64..72].try_into().unwrap()),
        u64::from_le_bytes(d[72..80].try_into().unwrap()),
    ))
}

fn qres_add(ai: &AccountInfo, holder: u64, burn: u64, owed: u64) -> Result<()> {
    let mut d = ai.try_borrow_mut_data()?;
    let h = u64::from_le_bytes(d[56..64].try_into().unwrap()).checked_add(holder).ok_or(FusedError::Math)?;
    let b = u64::from_le_bytes(d[64..72].try_into().unwrap()).checked_add(burn).ok_or(FusedError::Math)?;
    let o = u64::from_le_bytes(d[72..80].try_into().unwrap()).checked_add(owed).ok_or(FusedError::Math)?;
    d[56..64].copy_from_slice(&h.to_le_bytes());
    d[64..72].copy_from_slice(&b.to_le_bytes());
    d[72..80].copy_from_slice(&o.to_le_bytes());
    Ok(())
}

fn qres_sub(ai: &AccountInfo, holder: u64, burn: u64, owed: u64) -> Result<()> {
    let mut d = ai.try_borrow_mut_data()?;
    let h = u64::from_le_bytes(d[56..64].try_into().unwrap()).checked_sub(holder).ok_or(FusedError::Math)?;
    let b = u64::from_le_bytes(d[64..72].try_into().unwrap()).checked_sub(burn).ok_or(FusedError::Math)?;
    let o = u64::from_le_bytes(d[72..80].try_into().unwrap()).checked_sub(owed).ok_or(FusedError::Math)?;
    d[56..64].copy_from_slice(&h.to_le_bytes());
    d[64..72].copy_from_slice(&b.to_le_bytes());
    d[72..80].copy_from_slice(&o.to_le_bytes());
    Ok(())
}

fn credit_live_reserve(ai: &AccountInfo, mint: &Pubkey, holder: u64, burn: u64, owed: u64, program_id: &Pubkey) -> Result<()> {
    let state = qres_state(ai, mint, program_id)?;
    require!(state == 2, FusedError::ReserveNotLive);
    qres_add(ai, holder, burn, owed)
}

fn debit_live_reserve(ai: &AccountInfo, mint: &Pubkey, holder: u64, burn: u64, owed: u64, program_id: &Pubkey) -> Result<()> {
    if holder == 0 && burn == 0 && owed == 0 {
        return Ok(());
    }
    let state = qres_state(ai, mint, program_id)?;
    require!(state == 2, FusedError::ReserveNotLive);
    qres_sub(ai, holder, burn, owed)
}

fn require_reserve_covers(ai: &AccountInfo, mint: &Pubkey, vault_amount: u64, program_id: &Pubkey) -> Result<()> {
    let state = qres_state(ai, mint, program_id)?;
    require!(state == 2, FusedError::ReserveNotLive);
    let (h, b, o) = qres_totals(ai)?;
    let sum = h.checked_add(b).ok_or(FusedError::Math)?.checked_add(o).ok_or(FusedError::Math)?;
    require!(vault_amount >= sum, FusedError::VaultShort);
    Ok(())
}

fn mint_fee(mint: &AccountInfo, amount: u64) -> Result<u64> {
    let d = mint.try_borrow_data()?;
    if d.len() <= 165 || amount == 0 {
        return Ok(0);
    }
    let epoch = Clock::get()?.epoch;
    let mut i = 166usize;
    while i + 4 <= d.len() {
        let tag = u16::from_le_bytes([d[i], d[i + 1]]);
        let len = u16::from_le_bytes([d[i + 2], d[i + 3]]) as usize;
        let start = i + 4;
        if start + len > d.len() {
            return err!(FusedError::BadMint);
        }
        if tag == 0 {
            break;
        }
        if tag == 1 && len >= 108 {
            let newer = u64::from_le_bytes(d[start + 90..start + 98].try_into().unwrap());
            let base = if epoch >= newer { start + 90 } else { start + 72 };
            let max_fee = u64::from_le_bytes(d[base + 8..base + 16].try_into().unwrap());
            let bps = u16::from_le_bytes([d[base + 16], d[base + 17]]) as u128;
            if bps == 0 {
                return Ok(0);
            }
            let raw = (amount as u128).saturating_mul(bps).saturating_add(9_999) / 10_000;
            let raw = u64::try_from(raw).unwrap_or(u64::MAX);
            return Ok(core::cmp::min(raw, max_fee));
        }
        i = start + len;
    }
    Ok(0)
}

fn bump_reserve_count(ai: &AccountInfo, mint: &Pubkey, program_id: &Pubkey) -> Result<()> {
    let state = qres_state(ai, mint, program_id)?;
    require!(state == 1 || state == 2, FusedError::ReserveNotLive);
    let mut d = ai.try_borrow_mut_data()?;
    let n = u64::from_le_bytes(d[40..48].try_into().unwrap()).checked_add(1).ok_or(FusedError::Math)?;
    d[40..48].copy_from_slice(&n.to_le_bytes());
    Ok(())
}

/// LP reservation. Header 20 bytes, then up to four 136-byte entries:
/// pool, mint, source vault, amount, configured DAMM vault.
const LB_TAG: [u8; 8] = [76, 66, 65, 84, 67, 72, 48, 49];
const LB_ENTRY: usize = 136;
const LB_LEN: usize = 20 + LB_ENTRY * 4;

fn lb_key(id: u64, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"lpbatch", &id.to_le_bytes()], program_id)
}

fn lb_load<'a>(ai: &'a AccountInfo<'a>, id: u64, program_id: &Pubkey) -> Result<(u8, u8, u8)> {
    let (key, _) = lb_key(id, program_id);
    require_keys_eq!(ai.key(), key, FusedError::BadAutoLpAccounts);
    require_keys_eq!(*ai.owner, *program_id, FusedError::BadAutoLpAccounts);
    let d = ai.try_borrow_data()?;
    require!(d.len() == LB_LEN && d[..8] == LB_TAG, FusedError::BadAutoLpAccounts);
    require!(u64::from_le_bytes(d[8..16].try_into().unwrap()) == id, FusedError::BadAutoLpAccounts);
    Ok((d[16], d[17], d[18]))
}

fn lb_entry(ai: &AccountInfo, i: usize) -> Result<(Pubkey, Pubkey, Pubkey, u64, Pubkey)> {
    let d = ai.try_borrow_data()?;
    let o = 20 + i * LB_ENTRY;
    Ok((
        Pubkey::try_from(&d[o..o + 32]).unwrap(),
        Pubkey::try_from(&d[o + 32..o + 64]).unwrap(),
        Pubkey::try_from(&d[o + 64..o + 96]).unwrap(),
        u64::from_le_bytes(d[o + 96..o + 104].try_into().unwrap()),
        Pubkey::try_from(&d[o + 104..o + 136]).unwrap(),
    ))
}

fn bind_configured_vault(auto_pool: &Pubkey, damm_pool: &AccountInfo, dest: &AccountInfo, mint: &Pubkey, tprog: &Pubkey) -> Result<()> {
    require_keys_eq!(damm_pool.key(), *auto_pool, FusedError::BadPool);
    require_keys_eq!(*damm_pool.owner, DAMM_PROGRAM_ID, FusedError::BadPool);
    let info = read_token_account(dest, tprog).map_err(|_| error!(FusedError::BadDest))?;
    require_keys_eq!(info.mint, *mint, FusedError::BadMint);
    require_keys_eq!(info.owner, DAMM_POOL_AUTHORITY, FusedError::BadDest);
    require!(dest.is_writable, FusedError::BadDest);
    let data = damm_pool.try_borrow_data()?;
    let needle = dest.key.to_bytes();
    let mut found = false;
    let mut i = 0usize;
    while i + 32 <= data.len() {
        if data[i..i + 32] == needle {
            found = true;
            break;
        }
        i += 1;
    }
    require!(found, FusedError::BadAutoLpAccounts);
    Ok(())
}

fn lp_batch_open<'info>(a: &'info [AccountInfo<'info>], program_id: &Pubkey, batch_id: u64) -> Result<()> {
    require!(a.len() >= 7, FusedError::BadAutoLpAccounts);
    let payer = &a[0];
    require!(payer.is_signer && payer.is_writable, FusedError::NotCrank);
    let global: Account<Global> = Account::try_from(&a[1])?;
    require_keys_eq!(a[1].key(), Pubkey::find_program_address(&[b"global"], program_id).0, FusedError::BadAutoLpAccounts);
    require!(!global.paused, FusedError::Paused);
    require!(payer.key() == global.admin || payer.key() == CRANK_AUTHORITY, FusedError::NotCrank);
    let auto_lp: Account<AutoLpConfig> = Account::try_from(&a[4])?;
    require_keys_eq!(a[4].key(), Pubkey::find_program_address(&[b"autolp"], program_id).0, FusedError::BadAutoLpAccounts);
    require!(auto_lp.damm_pool != Pubkey::default(), FusedError::AutoLpNotReady);
    require!(auto_lp.lp_share_bps <= 10_000, FusedError::BadAutoLpShare);
    let (bkey, bbump) = lb_key(batch_id, program_id);
    require_keys_eq!(a[2].key(), bkey, FusedError::BadAutoLpAccounts);
    require_keys_eq!(a[3].key(), anchor_lang::system_program::ID, FusedError::BadAutoLpAccounts);
    let tprog = &a[5];
    require!(is_token_program(tprog.key), FusedError::WrongTokenProgram);
    let damm_pool = &a[6];
    require_keys_eq!(damm_pool.key(), auto_lp.damm_pool, FusedError::BadPool);
    require_keys_eq!(*damm_pool.owner, DAMM_PROGRAM_ID, FusedError::BadPool);
    let tail = &a[7..];
    require!(!tail.is_empty() && tail.len() % 6 == 0, FusedError::BadAutoLpAccounts);
    let n = tail.len() / 6;
    require!(n >= 1 && n <= 4, FusedError::BadAutoLpAccounts);
    let idb = batch_id.to_le_bytes();
    let bump_seed = [bbump];
    let seeds: &[&[u8]] = &[b"lpbatch", &idb, &bump_seed];
    create_pda_account(payer, &a[2], &a[3], Rent::get()?.minimum_balance(LB_LEN), LB_LEN as u64, program_id, seeds)?;
    let mut seen: Vec<Pubkey> = Vec::new();
    for i in 0..n {
        let pool_ai = &tail[i * 6];
        let vault = &tail[i * 6 + 1];
        let mint = &tail[i * 6 + 2];
        let escrow = &tail[i * 6 + 3];
        let reserve = &tail[i * 6 + 4];
        let dest = &tail[i * 6 + 5];
        let pr: Account<PoolRecord> = Account::try_from(pool_ai)?;
        let expect = Pubkey::find_program_address(&[b"pool", pr.pool.as_ref()], program_id).0;
        require_keys_eq!(pool_ai.key(), expect, FusedError::BadAutoLpAccounts);
        require!(pool_ai.is_writable && vault.is_writable && escrow.is_writable && reserve.is_writable, FusedError::BadAutoLpAccounts);
        require!(!seen.iter().any(|p| *p == pr.pool), FusedError::BadAutoLpAccounts);
        seen.push(pr.pool);
        require_keys_eq!(mint.key(), pr.quote_mint, FusedError::BadMint);
        require_keys_eq!(*mint.owner, *tprog.key, FusedError::WrongTokenProgram);
        let budget = (pr.burn_pot as u128)
            .checked_mul(auto_lp.lp_share_bps as u128).ok_or(FusedError::Math)?
            / 10_000u128;
        let budget = u64::try_from(budget).map_err(|_| error!(FusedError::Math))?;
        require!(budget > 0 && budget <= pr.burn_pot, FusedError::BadAutoLpAmounts);
        require_keys_eq!(vault.key(), get_associated_token_address_with_program_id(&a[1].key(), &pr.quote_mint, tprog.key), FusedError::BadVault);
        require_keys_eq!(escrow.key(), get_associated_token_address_with_program_id(&bkey, &pr.quote_mint, tprog.key), FusedError::BadVault);
        bind_configured_vault(&auto_lp.damm_pool, damm_pool, dest, &pr.quote_mint, tprog.key)?;
        require_reserve_covers(reserve, &pr.quote_mint, token_amount(vault)?, program_id)?;
        let dec = mint.try_borrow_data()?[44];
        let gb = [global.bump];
        let gseeds: &[&[u8]] = &[b"global", &gb];
        transfer_checked_signed(tprog, vault, mint, escrow, &a[1], budget, dec, gseeds)?;
        debit_live_reserve(reserve, &pr.quote_mint, 0, budget, 0, program_id)?;
        let mut pr2: Account<PoolRecord> = Account::try_from(pool_ai)?;
        pr2.burn_pot = pr2.burn_pot.checked_sub(budget).ok_or(FusedError::Math)?;
        let pool = pr2.pool;
        let quote = pr2.quote_mint;
        pr2.exit(program_id)?;
        let mut d = a[2].try_borrow_mut_data()?;
        if i == 0 {
            d[..8].copy_from_slice(&LB_TAG);
            d[8..16].copy_from_slice(&batch_id.to_le_bytes());
            d[16] = 1;
            d[17] = n as u8;
            d[18] = bbump;
        }
        let o = 20 + i * LB_ENTRY;
        d[o..o + 32].copy_from_slice(pool.as_ref());
        d[o + 32..o + 64].copy_from_slice(quote.as_ref());
        d[o + 64..o + 96].copy_from_slice(vault.key.as_ref());
        d[o + 96..o + 104].copy_from_slice(&budget.to_le_bytes());
        d[o + 104..o + 136].copy_from_slice(dest.key.as_ref());
    }
    msg!("lp_batch_open id={} pools={}", batch_id, n);
    Ok(())
}

fn lp_batch_consume<'info>(a: &'info [AccountInfo<'info>], program_id: &Pubkey, batch_id: u64) -> Result<()> {
    require!(a.len() >= 6, FusedError::BadAutoLpAccounts);
    let payer = &a[0];
    require!(payer.is_signer, FusedError::NotCrank);
    let global: Account<Global> = Account::try_from(&a[1])?;
    require!(!global.paused, FusedError::Paused);
    require!(payer.key() == global.admin || payer.key() == CRANK_AUTHORITY, FusedError::NotCrank);
    let (state, n, bump) = lb_load(&a[2], batch_id, program_id)?;
    require!(state == 1, FusedError::LpBatchClosed);
    let tprog = &a[3];
    require!(is_token_program(tprog.key), FusedError::WrongTokenProgram);
    let auto_lp: Account<AutoLpConfig> = Account::try_from(&a[4])?;
    require_keys_eq!(a[4].key(), Pubkey::find_program_address(&[b"autolp"], program_id).0, FusedError::BadAutoLpAccounts);
    let damm_pool = &a[5];
    let n = n as usize;
    require!(a.len() == 6 + n * 2, FusedError::BadAutoLpAccounts);
    let (bkey, _) = lb_key(batch_id, program_id);
    let idb = batch_id.to_le_bytes();
    let bump_seed = [bump];
    let seeds: &[&[u8]] = &[b"lpbatch", &idb, &bump_seed];
    for i in 0..n {
        let (_pool, mint, _source, amount, configured) = lb_entry(&a[2], i)?;
        let escrow = &a[6 + i * 2];
        let dest = &a[6 + i * 2 + 1];
        require_keys_eq!(dest.key(), configured, FusedError::BadDest);
        require_keys_eq!(escrow.key(), get_associated_token_address_with_program_id(&bkey, &mint, tprog.key), FusedError::BadVault);
        bind_configured_vault(&auto_lp.damm_pool, damm_pool, dest, &mint, tprog.key)?;
        let mut data = Vec::with_capacity(9);
        data.push(3u8);
        data.extend_from_slice(&amount.to_le_bytes());
        anchor_lang::solana_program::program::invoke_signed(
            &anchor_lang::solana_program::instruction::Instruction {
                program_id: *tprog.key,
                accounts: vec![
                    AccountMeta::new(*escrow.key, false),
                    AccountMeta::new(*dest.key, false),
                    AccountMeta::new_readonly(bkey, true),
                ],
                data,
            },
            &[escrow.clone(), dest.clone(), a[2].clone(), tprog.clone()],
            &[seeds],
        )?;
    }
    a[2].try_borrow_mut_data()?[16] = 2;
    msg!("lp_batch_consumed id={}", batch_id);
    Ok(())
}

fn lp_batch_refund<'info>(a: &'info [AccountInfo<'info>], program_id: &Pubkey, batch_id: u64) -> Result<()> {
    require!(a.len() >= 4, FusedError::BadAutoLpAccounts);
    require!(a[0].is_signer, FusedError::NotCrank);
    let (state, n, bump) = lb_load(&a[2], batch_id, program_id)?;
    require!(state == 1, FusedError::LpBatchClosed);
    let tprog = &a[3];
    require!(is_token_program(tprog.key), FusedError::WrongTokenProgram);
    let n = n as usize;
    require!(a.len() == 4 + n * 4, FusedError::BadAutoLpAccounts);
    let (bkey, _) = lb_key(batch_id, program_id);
    let idb = batch_id.to_le_bytes();
    let bump_seed = [bump];
    let seeds: &[&[u8]] = &[b"lpbatch", &idb, &bump_seed];
    for i in 0..n {
        let (pool, mint, source, amount, _dest) = lb_entry(&a[2], i)?;
        let escrow = &a[4 + i * 4];
        let source_ai = &a[4 + i * 4 + 1];
        let pool_ai = &a[4 + i * 4 + 2];
        let reserve = &a[4 + i * 4 + 3];
        require_keys_eq!(escrow.key(), get_associated_token_address_with_program_id(&bkey, &mint, tprog.key), FusedError::BadVault);
        require_keys_eq!(source_ai.key(), source, FusedError::BadVault);
        require_keys_eq!(pool_ai.key(), Pubkey::find_program_address(&[b"pool", pool.as_ref()], program_id).0, FusedError::BadAutoLpAccounts);
        let mut data = Vec::with_capacity(9);
        data.push(3u8);
        data.extend_from_slice(&amount.to_le_bytes());
        anchor_lang::solana_program::program::invoke_signed(
            &anchor_lang::solana_program::instruction::Instruction {
                program_id: *tprog.key,
                accounts: vec![
                    AccountMeta::new(*escrow.key, false),
                    AccountMeta::new(*source_ai.key, false),
                    AccountMeta::new_readonly(bkey, true),
                ],
                data,
            },
            &[escrow.clone(), source_ai.clone(), a[2].clone(), tprog.clone()],
            &[seeds],
        )?;
        credit_live_reserve(reserve, &mint, 0, amount, 0, program_id)?;
        let mut pr: Account<PoolRecord> = Account::try_from(pool_ai)?;
        require_keys_eq!(pr.pool, pool, FusedError::BadAutoLpAccounts);
        require_keys_eq!(pr.quote_mint, mint, FusedError::BadMint);
        pr.burn_pot = pr.burn_pot.checked_add(amount).ok_or(FusedError::Math)?;
        pr.exit(program_id)?;
    }
    a[2].try_borrow_mut_data()?[16] = 3;
    msg!("lp_batch_refunded id={}", batch_id);
    Ok(())
}
