//! Shared helpers for the two experimental vault programs.
//! Not a deployed program and not a change to fused_fee.

use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

pub const SHARED_ID: Pubkey = solana_program::pubkey!("6URwbPipuA4MJLG7LCRRZuWnms3JZ9cRG3z9indXWz8G");
pub const PER_POOL_ID: Pubkey = solana_program::pubkey!("6YMEjhBqVTMaSRWcmVkLrnHZ22FWEDJEpTeonAg8GKSy");

pub const TOKEN_ID: Pubkey = solana_program::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022_ID: Pubkey = solana_program::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const E_BAD_STATE: u32 = 100;
pub const E_ALREADY: u32 = 101;
pub const E_COUNT: u32 = 102;
pub const E_VAULT_SHORT: u32 = 103;
pub const E_MATH: u32 = 104;
pub const E_FROZEN: u32 = 105;
pub const E_NOT_ADMIN: u32 = 106;
pub const E_BAD_ACCOUNTS: u32 = 107;
pub const E_BATCH: u32 = 108;
pub const E_ESCROW: u32 = 109;
pub const E_PAUSED: u32 = 110;
pub const E_BUDGET: u32 = 111;

pub const LP_SHARE_BPS: u64 = 5_000;
pub const MAX_BATCH: usize = 4;

pub const GLOBAL_LEN: usize = 40;
pub const RESERVE_LEN: usize = 128;
pub const POOL_LEN: usize = 112;
pub const ESCROW_LEN: usize = 208;

pub const ST_FROZEN: u8 = 1;
pub const ST_BACKFILL: u8 = 2;
pub const ST_LIVE: u8 = 3;

pub const KIND_CURVE: u8 = 0;
pub const KIND_GRAD_NO_CREATOR: u8 = 1;
pub const KIND_GRAD_CREATOR: u8 = 2;

pub fn err(code: u32) -> ProgramError {
    ProgramError::Custom(code)
}

pub fn ru64(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}

pub fn wu64(d: &mut [u8], o: usize, v: u64) {
    d[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

pub fn add_at(d: &mut [u8], o: usize, v: u64) -> Result<(), ProgramError> {
    let next = add(ru64(d, o), v)?;
    wu64(d, o, next);
    Ok(())
}

pub fn sub_at(d: &mut [u8], o: usize, v: u64) -> Result<(), ProgramError> {
    let next = sub(ru64(d, o), v)?;
    wu64(d, o, next);
    Ok(())
}

pub fn add(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_add(b).ok_or(err(E_MATH))
}

pub fn sub(a: u64, b: u64) -> Result<u64, ProgramError> {
    a.checked_sub(b).ok_or(err(E_MATH))
}

/// Curve: creator 0, burn = delta * 20 / 110, holder = the rest.
/// Graduated with creator: creator = delta * 8 / 100, then the same 20/110 on the remainder.
/// Graduated without creator: same as curve. These are the existing ratios, not a new fee.
pub fn split(delta: u64, kind: u8) -> Result<(u64, u64, u64), ProgramError> {
    let (creator, rest) = match kind {
        KIND_CURVE | KIND_GRAD_NO_CREATOR => (0, delta),
        KIND_GRAD_CREATOR => {
            let creator = delta.checked_mul(8).ok_or(err(E_MATH))? / 100;
            (creator, delta - creator)
        }
        _ => return Err(err(E_BAD_ACCOUNTS)),
    };
    let burn = rest.checked_mul(20).ok_or(err(E_MATH))? / 110;
    let holder = rest - burn;
    Ok((creator, holder, burn))
}

pub fn lp_budget(burn: u64) -> u64 {
    burn * LP_SHARE_BPS / 10_000
}

pub fn token_amount(data: &[u8]) -> Result<u64, ProgramError> {
    if data.len() < 72 {
        return Err(err(E_BAD_ACCOUNTS));
    }
    Ok(ru64(data, 64))
}

/// Ceiling fee from a Token-2022 mint. Classic mints and mints without the extension return 0.
/// Both fee epochs are read. The newer slot is used when the clock epoch has reached it.
pub fn transfer_fee(mint: &[u8], amount: u64) -> Result<u64, ProgramError> {
    if amount == 0 || mint.len() <= 165 {
        return Ok(0);
    }
    let epoch = Clock::get().map(|c| c.epoch).unwrap_or(0);
    let mut i = 166usize;
    while i + 4 <= mint.len() {
        let tag = u16::from_le_bytes(mint[i..i + 2].try_into().unwrap());
        let len = u16::from_le_bytes(mint[i + 2..i + 4].try_into().unwrap()) as usize;
        let start = i + 4;
        let end = start.checked_add(len).ok_or(err(E_BAD_ACCOUNTS))?;
        if end > mint.len() {
            return Err(err(E_BAD_ACCOUNTS));
        }
        if tag == 0 {
            break;
        }
        if tag == 1 && len >= 108 {
            let newer_epoch = ru64(mint, start + 90);
            let base = if epoch >= newer_epoch { start + 90 } else { start + 72 };
            let max_fee = ru64(mint, base + 8);
            let bps = u16::from_le_bytes(mint[base + 16..base + 18].try_into().unwrap()) as u128;
            if bps == 0 {
                return Ok(0);
            }
            let raw = (amount as u128)
                .checked_mul(bps)
                .ok_or(err(E_MATH))?
                .checked_add(9_999)
                .ok_or(err(E_MATH))?
                / 10_000;
            let raw = u64::try_from(raw).map_err(|_| err(E_MATH))?;
            return Ok(raw.min(max_fee));
        }
        i = end;
    }
    Ok(0)
}

pub fn move_tokens<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    dest: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
    decimals: u8,
    seeds: &[&[u8]],
) -> ProgramResult {
    if amount == 0 {
        return Err(err(E_MATH));
    }
    if mint.data.borrow().get(44).copied() != Some(decimals) {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let fee = if *token_program.key == TOKEN_2022_ID {
        transfer_fee(&mint.data.borrow(), amount)?
    } else if *token_program.key == TOKEN_ID {
        0
    } else {
        return Err(err(E_BAD_ACCOUNTS));
    };
    let (data, metas_fee) = if *token_program.key == TOKEN_2022_ID {
        let mut data = Vec::with_capacity(1 + 1 + 8 + 1 + 8);
        data.push(26);
        data.push(1);
        data.extend_from_slice(&amount.to_le_bytes());
        data.push(decimals);
        data.extend_from_slice(&fee.to_le_bytes());
        (data, true)
    } else {
        let mut data = Vec::with_capacity(9);
        data.push(3);
        data.extend_from_slice(&amount.to_le_bytes());
        (data, false)
    };
    let ix = if metas_fee {
        solana_program::instruction::Instruction {
            program_id: *token_program.key,
            accounts: vec![
                solana_program::instruction::AccountMeta::new(*source.key, false),
                solana_program::instruction::AccountMeta::new_readonly(*mint.key, false),
                solana_program::instruction::AccountMeta::new(*dest.key, false),
                solana_program::instruction::AccountMeta::new_readonly(*authority.key, true),
            ],
            data,
        }
    } else {
        solana_program::instruction::Instruction {
            program_id: *token_program.key,
            accounts: vec![
                solana_program::instruction::AccountMeta::new(*source.key, false),
                solana_program::instruction::AccountMeta::new(*dest.key, false),
                solana_program::instruction::AccountMeta::new_readonly(*authority.key, true),
            ],
            data,
        }
    };
    let accounts: Vec<AccountInfo> = if metas_fee {
        vec![source.clone(), mint.clone(), dest.clone(), authority.clone(), token_program.clone()]
    } else {
        vec![source.clone(), dest.clone(), authority.clone(), token_program.clone()]
    };
    if seeds.is_empty() {
        solana_program::program::invoke(&ix, &accounts)
    } else {
        invoke_signed(&ix, &accounts, &[seeds])
    }
}

pub fn create_pda<'a>(
    payer: &AccountInfo<'a>,
    pda: &AccountInfo<'a>,
    system: &AccountInfo<'a>,
    space: usize,
    seeds: &[&[u8]],
    program_id: &Pubkey,
) -> ProgramResult {
    if pda.lamports() > 0 || !pda.data_is_empty() {
        return Err(err(E_ALREADY));
    }
    let lamports = solana_program::rent::Rent::get()?.minimum_balance(space);
    let ix = solana_program::system_instruction::create_account(payer.key, pda.key, lamports, space as u64, program_id);
    invoke_signed(&ix, &[payer.clone(), pda.clone(), system.clone()], &[seeds])
}

pub fn read_admin(global: &AccountInfo) -> Result<(Pubkey, u8, bool), ProgramError> {
    let d = global.data.borrow();
    if d.len() < GLOBAL_LEN {
        return Err(err(E_BAD_ACCOUNTS));
    }
    let admin = Pubkey::try_from(&d[0..32]).unwrap();
    Ok((admin, d[32], d[33] != 0))
}

pub fn require_admin(global: &AccountInfo, signer: &AccountInfo) -> ProgramResult {
    if !signer.is_signer {
        return Err(err(E_NOT_ADMIN));
    }
    let (admin, _, paused) = read_admin(global)?;
    if signer.key != &admin {
        return Err(err(E_NOT_ADMIN));
    }
    let _ = paused;
    Ok(())
}

pub fn require_unpaused(global: &AccountInfo) -> ProgramResult {
    let (_, _, paused) = read_admin(global)?;
    if paused {
        return Err(err(E_PAUSED));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_match_existing_economics() {
        assert_eq!(split(11_000, KIND_CURVE).unwrap(), (0, 9_000, 2_000));
        assert_eq!(split(11_000, KIND_GRAD_CREATOR).unwrap(), (880, 8_280, 1_840));
        assert_eq!(split(11_000, KIND_GRAD_NO_CREATOR).unwrap(), (0, 9_000, 2_000));
        assert_eq!(lp_budget(10_000), 5_000);
        assert_eq!(lp_budget(10_001), 5_000);
    }
}
