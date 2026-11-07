/// Experimental per-pool vault hooks. The global PDA remains the Meteora fee claimer.
/// A claim still lands in the global quote vault, then this instruction moves that delta
/// into the pool PDA's quote token account. Later pays sign with the pool PDA.

fn pool_vault_ok(vault: &AccountInfo, pool_pda: &Pubkey, mint: &Pubkey, prog: &Pubkey) -> Result<()> {
    let info = read_token_account(vault, prog).map_err(|_| error!(FusedError::PoolVault))?;
    require!(info.mint == *mint, FusedError::PoolVault);
    require_keys_eq!(info.owner, *pool_pda, FusedError::PoolVault);
    require!(vault.is_writable, FusedError::PoolVault);
    Ok(())
}

fn move_claim_to_pool<'info>(
    token_program: &AccountInfo<'info>,
    global_vault: &AccountInfo<'info>,
    pool_vault: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    global: &AccountInfo<'info>,
    global_bump: u8,
    amount: u64,
    decimals: u8,
) -> Result<u64> {
    let before = token_amount(pool_vault)?;
    let bump_seed = [global_bump];
    let seeds: &[&[u8]] = &[b"global", &bump_seed];
    transfer_checked_signed(token_program, global_vault, mint, pool_vault, global, amount, decimals, seeds)?;
    let after = token_amount(pool_vault)?;
    after.checked_sub(before).ok_or(error!(FusedError::Math))
}
