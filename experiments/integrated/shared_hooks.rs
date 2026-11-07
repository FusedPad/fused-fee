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

fn bump_reserve_count(ai: &AccountInfo, mint: &Pubkey, program_id: &Pubkey) -> Result<()> {
    let state = qres_state(ai, mint, program_id)?;
    require!(state == 1 || state == 2, FusedError::ReserveNotLive);
    let mut d = ai.try_borrow_mut_data()?;
    let n = u64::from_le_bytes(d[40..48].try_into().unwrap()).checked_add(1).ok_or(FusedError::Math)?;
    d[40..48].copy_from_slice(&n.to_le_bytes());
    Ok(())
}
