//! Local stand-in for Meteora DBC and DAMM. Not those programs.
//! One binary is installed at both program ids. It only moves token amounts
//! and marks the position account so fused_fee can observe the CPI.

use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey, sysvar::Sysvar,
};

entrypoint!(process);

const CLAIM_TRADING_FEE: [u8; 8] = [8, 236, 89, 49, 152, 125, 177, 81];
const CLAIM_POSITION_FEE: [u8; 8] = [180, 38, 154, 17, 133, 33, 162, 211];
const ADD_LIQUIDITY: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72];
const PERMANENT_LOCK: [u8; 8] = [165, 176, 125, 6, 231, 171, 186, 213];
const TOKEN_2022: Pubkey = solana_program::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    // Claim trading fee CPI: quote_vault=4, quote_pool_vault=6, global signer=9, token program=11.
    if data[..8] == CLAIM_TRADING_FEE {
        if data.len() < 24 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let add = u64::from_le_bytes(data[16..24].try_into().unwrap());
        return spl_transfer(&accounts[11], &accounts[8], &accounts[6], &accounts[4], &accounts[9], add);
    }
    // Claim position fee CPI: token_b=4, token_b_vault=6, mint=8, global signer=10, token_b program=12.
    if data[..8] == CLAIM_POSITION_FEE {
        return spl_transfer(&accounts[12], &accounts[8], &accounts[6], &accounts[4], &accounts[10], 11_000);
    }
    // add_liquidity CPI: position=1, token_a=2, token_b=3, vault_a=4, vault_b=5, global=9, prog_a=10, prog_b=11.
    if data[..8] == ADD_LIQUIDITY {
        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let a = u64::from_le_bytes(data[24..32].try_into().unwrap());
        let b = u64::from_le_bytes(data[32..40].try_into().unwrap());
        spl_transfer(&accounts[10], &accounts[6], &accounts[2], &accounts[4], &accounts[9], a)?;
        spl_transfer(&accounts[11], &accounts[7], &accounts[3], &accounts[5], &accounts[9], b)?;
        // Position is owned by this program, so the lock flag can be written directly.
        if *accounts[1].owner != *program_id {
            return Err(ProgramError::IllegalOwner);
        }
        let mut pos = accounts[1].try_borrow_mut_data()?;
        if pos.is_empty() {
            return Err(ProgramError::InvalidAccountData);
        }
        pos[0] = 1;
        return Ok(());
    }
    if data[..8] == PERMANENT_LOCK {
        if *accounts[1].owner != *program_id {
            return Err(ProgramError::IllegalOwner);
        }
        let mut pos = accounts[1].try_borrow_mut_data()?;
        if pos.first().copied() != Some(1) {
            return Err(ProgramError::InvalidAccountData);
        }
        pos[1] = 1;
        return Ok(());
    }
    Err(ProgramError::InvalidInstructionData)
}

fn spl_transfer<'a>(
    token_program: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    dest: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
) -> ProgramResult {
    if *token_program.key == TOKEN_2022 {
        let fee = token2022_fee(mint, amount)?;
        let decimals = mint.try_borrow_data().map_err(|_| ProgramError::InvalidAccountData)?[44];
        let mut data = Vec::with_capacity(19);
        data.push(26u8);
        data.push(1u8);
        data.extend_from_slice(&amount.to_le_bytes());
        data.push(decimals);
        data.extend_from_slice(&fee.to_le_bytes());
        return solana_program::program::invoke(
            &solana_program::instruction::Instruction {
                program_id: *token_program.key,
                accounts: vec![
                    solana_program::instruction::AccountMeta::new(*source.key, false),
                    solana_program::instruction::AccountMeta::new_readonly(*mint.key, false),
                    solana_program::instruction::AccountMeta::new(*dest.key, false),
                    solana_program::instruction::AccountMeta::new_readonly(*authority.key, true),
                ],
                data,
            },
            &[source.clone(), mint.clone(), dest.clone(), authority.clone(), token_program.clone()],
        );
    }
    let mut data = Vec::with_capacity(9);
    data.push(3u8);
    data.extend_from_slice(&amount.to_le_bytes());
    solana_program::program::invoke(
        &solana_program::instruction::Instruction {
            program_id: *token_program.key,
            accounts: vec![
                solana_program::instruction::AccountMeta::new(*source.key, false),
                solana_program::instruction::AccountMeta::new(*dest.key, false),
                solana_program::instruction::AccountMeta::new_readonly(*authority.key, true),
            ],
            data,
        },
        &[source.clone(), dest.clone(), authority.clone(), token_program.clone()],
    )
}

fn token2022_fee(mint: &AccountInfo, amount: u64) -> Result<u64, ProgramError> {
    let d = mint.try_borrow_data().map_err(|_| ProgramError::InvalidAccountData)?;
    if d.len() <= 166 || amount == 0 {
        return Ok(0);
    }
    let epoch = Clock::get()?.epoch;
    let mut i = 166usize;
    while i + 4 <= d.len() {
        let tag = u16::from_le_bytes([d[i], d[i + 1]]);
        let len = u16::from_le_bytes([d[i + 2], d[i + 3]]) as usize;
        let start = i + 4;
        if start + len > d.len() {
            return Err(ProgramError::InvalidAccountData);
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
            let raw = ((amount as u128) * bps + 9_999) / 10_000;
            return Ok((raw as u64).min(max_fee));
        }
        i = start + len;
    }
    Ok(0)
}
