use anchor_lang::prelude::*;
use anchor_spl::associated_token::{get_associated_token_address, get_associated_token_address_with_program_id};
use anchor_spl::token::{self, Burn, CloseAccount, InitializeAccount3, Token, TokenAccount, Transfer};

// V2: mainnet ids by default; `--features devnet` builds the devnet test variant.
#[cfg(not(feature = "devnet"))]
declare_id!("AZY2uXFPUfgTkhoNaXNL3oaZMdb4r3NfHYymA3s4mMw9");
#[cfg(feature = "devnet")]
declare_id!("8U45yvkgMohJBqCrd3uepXEsuVTfFY9Ak8Lu3ZU9Rm9F"); // DEVNET_PROGRAM_ID: replaced by the runbook

/// Partner share of volume is 1.10%. Of that, 0.90 goes to holders and 0.20 burns.
const HOLDER_NUM: u64 = 90;
const BURN_NUM: u64 = 20;
const SPLIT_DENOM: u64 = 110;

const DBC_PROGRAM_ID: Pubkey = pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
const DBC_POOL_AUTHORITY: Pubkey = pubkey!("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM");
/// sha256("global:claim_trading_fee")[..8]
const CLAIM_TRADING_FEE_DISC: [u8; 8] = [8, 236, 89, 49, 152, 125, 177, 81];
const DAMM_PROGRAM_ID: Pubkey = pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
const DAMM_POOL_AUTHORITY: Pubkey = pubkey!("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");
/// sha256("global:claim_position_fee")[..8]
const CLAIM_POSITION_FEE_DISC: [u8; 8] = [180, 38, 154, 17, 133, 33, 162, 211];
/// V5 auto-LP: sha256("global:add_liquidity")[..8]
const ADD_LIQUIDITY_DISC: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72];
/// V5 auto-LP: sha256("global:permanent_lock_position")[..8]
const PERMANENT_LOCK_DISC: [u8; 8] = [165, 176, 125, 6, 231, 171, 186, 213];
/// V5: default share of the 0.20% burn pot that becomes locked $FUSED LP (50%).
const DEFAULT_AUTO_LP_BPS: u16 = 5_000;

/// FIX: DBC account layouts (Meteora DBC IDL; checked against live mainnet accounts 21 Oct 2026).
/// sha256("account:VirtualPool")[..8]
const DBC_VIRTUAL_POOL_DISC: [u8; 8] = [213, 224, 5, 209, 98, 69, 119, 92];
/// sha256("account:PoolConfig")[..8]
const DBC_POOL_CONFIG_DISC: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];
/// VirtualPool: 8 disc + 64 VolatilityTracker, then config, creator, base_mint ...
const VP_CONFIG_OFFSET: usize = 72;
const VP_BASE_MINT_OFFSET: usize = 136;
/// ... u64 x6 reserves/fees (232..280), sqrt_price u128 (280..296), activation_point u64, pool_type u8 (304), is_migrated u8 (305)
const VP_IS_MIGRATED_OFFSET: usize = 305;
/// PoolConfig: 8 disc, quote_mint, fee_claimer ...
const PC_QUOTE_MINT_OFFSET: usize = 8;
const PC_FEE_CLAIMER_OFFSET: usize = 40;

/// FIX: the crank signer that is allowed to call crank_pay_page besides the admin.
#[cfg(not(feature = "devnet"))]
const CRANK_AUTHORITY: Pubkey = pubkey!("2tUjZV4UX9V8P65RZxFBhTxHTDhYRhWo6MYsAGuTsn1h");
/// Devnet-only crank stand-in (throwaway key used only for devnet tests).
#[cfg(feature = "devnet")]
const CRANK_AUTHORITY: Pubkey = pubkey!("4k1ny3fhH6WaSR6CrobbGmaqJipe36AVfwdeY9e7tLXs");

/// V4: after graduation a pool whose DBC config gives the creator NO share of the DAMM liquidity
/// (all LP locked to Global) pays the pool's creator 8% of every DAMM fee claim, the same 8% the
/// creator earns on the curve (0.096% of volume). The creator gets it in a per-pool CreatorOwed PDA
/// and pulls it with claim_creator_share.
const CREATOR_SHARE_NUM: u64 = 8;
const CREATOR_SHARE_DENOM: u64 = 100;
/// V4: VirtualPool.creator
const VP_CREATOR_OFFSET: usize = 104;
/// V4: PoolConfig LP split bytes (Meteora DBC IDL, 1048-byte account): creator vesting info is bytes 200..216
/// (is_initialized u8 @200, vesting_percentage u8 @201); permanent-locked / unlocked LP percentages below.
const PC_CREATOR_VEST_INIT_OFFSET: usize = 200;
const PC_CREATOR_VEST_PCT_OFFSET: usize = 201;
const PC_CREATOR_PERM_LOCKED_OFFSET: usize = 241;
const PC_CREATOR_LP_OFFSET: usize = 242;
const PC_MIN_LEN: usize = 243;

/// V2: wrapped SOL mint. Pools quoted in it pay holders plain SOL (lamports).
const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
/// V2: a brand-new PayCursor starts "closed" for the current round (no wallet sorts above this).
const MAX_OWNER: Pubkey = Pubkey::new_from_array([0xff; 32]);
/// V2: classic SPL token account size (temp wSOL account used to unwrap one page).
const TOKEN_ACCOUNT_LEN: u64 = 165;

/// V3: Token-2022 program and its (Meteora-rejected) wrapped-SOL mint.
const TOKEN_2022_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
const T22_NATIVE_MINT: Pubkey = pubkey!("9pan9bMn5HatX4EJdBwg9VgCa7Uz5HL8N1m5D3NdXejP");
/// V3: highest Token-2022 transfer fee (current AND scheduled) a pair may carry: 1%.
const MAX_TRANSFER_FEE_BPS: u16 = 100;
/// V3: mint_policy flags (returned by check_quote_mint).
const F_TOKEN_2022: u32 = 1;
const F_FREEZE_AUTHORITY: u32 = 1 << 1;
const F_PERMANENT_DELEGATE: u32 = 1 << 2;
const F_PAUSABLE: u32 = 1 << 3;
const F_HOOK_AUTHORITY: u32 = 1 << 4;
const F_TRANSFER_FEE: u32 = 1 << 5;
const F_DEFAULT_ACCOUNT_STATE: u32 = 1 << 6;
const F_SCALED_OR_INTEREST: u32 = 1 << 7;
const F_CONFIDENTIAL: u32 = 1 << 8;
const F_CLOSE_AUTHORITY: u32 = 1 << 9;
/// flags that make a mint "issuer-controlled" (tier 1)
const ISSUER_FLAGS: u32 =
    F_FREEZE_AUTHORITY | F_PERMANENT_DELEGATE | F_PAUSABLE | F_HOOK_AUTHORITY | F_TRANSFER_FEE | F_DEFAULT_ACCOUNT_STATE;

#[program]
pub mod fused_fee {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let g = &mut ctx.accounts.global;
        g.admin = ctx.accounts.admin.key();
        g.bump = ctx.bumps.global;
        g.paused = false;
        g.burn_mint = Pubkey::default();
        g.burn_mint_set = false;
        Ok(())
    }

    pub fn register_config(ctx: Context<RegisterConfig>, quote_mint: Pubkey) -> Result<()> {
        require!(quote_mint != Pubkey::default(), FusedError::BadMint);
        // V3: admin only; remaining_accounts[0] = the quote mint, which must pass the pair policy;
        // the DBC config must be for this quote mint with the Global PDA as fee claimer.
        require_keys_eq!(ctx.accounts.payer.key(), ctx.accounts.global.admin, FusedError::NotAdmin);
        let mint_ai = ctx.remaining_accounts.first().ok_or(error!(FusedError::BadMint))?;
        require_keys_eq!(mint_ai.key(), quote_mint, FusedError::BadMint);
        mint_policy(mint_ai)?;
        check_dbc_config(&ctx.accounts.config, &quote_mint, &ctx.accounts.global.key())?;
        let c = &mut ctx.accounts.config_record;
        c.quote_mint = quote_mint;
        c.config = ctx.accounts.config.key();
        c.bump = ctx.bumps.config_record;
        Ok(())
    }

    pub fn update_config(ctx: Context<UpdateConfig>, new_config: Pubkey) -> Result<()> {
        require!(new_config != Pubkey::default(), FusedError::BadMint);
        ctx.accounts.config_record.config = new_config;
        Ok(())
    }

    pub fn register_pool(
        ctx: Context<RegisterPool>,
        pool: Pubkey,
        base_mint: Pubkey,
        quote_mint: Pubkey,
    ) -> Result<()> {
        require!(pool != Pubkey::default(), FusedError::BadPool);
        require!(base_mint != Pubkey::default() && quote_mint != Pubkey::default(), FusedError::BadMint);
        require!(ctx.accounts.config_record.quote_mint == quote_mint, FusedError::QuoteMismatch);
        // FIX: record the config the DBC pool was really created with, not whatever
        // ConfigRecord holds right now (they drift whenever configs are rebuilt).
        require_keys_eq!(ctx.accounts.dbc_pool.key(), pool, FusedError::BadPool);
        let (real_config, real_base_mint, _) = read_dbc_pool(&ctx.accounts.dbc_pool)?;
        require_keys_eq!(real_base_mint, base_mint, FusedError::BadMint);
        require_keys_eq!(ctx.accounts.dbc_config.key(), real_config, FusedError::ConfigMismatch);
        check_dbc_config(&ctx.accounts.dbc_config, &quote_mint, &ctx.accounts.global.key())?;
        // V3: optional remaining_accounts[0] = quote mint -> refuse unsupported pairs at registration.
        if let Some(mint_ai) = ctx.remaining_accounts.first() {
            require_keys_eq!(mint_ai.key(), quote_mint, FusedError::BadMint);
            mint_policy(mint_ai)?;
        }
        let p = &mut ctx.accounts.pool_record;
        p.pool = pool;
        p.base_mint = base_mint;
        p.quote_mint = quote_mint;
        p.config = real_config;
        p.graduated = false;
        p.round = 0;
        p.holder_pot = 0;
        p.burn_pot = 0;
        p.bump = ctx.bumps.pool_record;
        // PERFECT: track pool count per quote mint so crank_auto_lp can require the full set.
        let qs = &mut ctx.accounts.quote_stat;
        if qs.quote_mint == Pubkey::default() {
            qs.quote_mint = quote_mint;
            qs.bump = ctx.bumps.quote_stat;
        }
        require_keys_eq!(qs.quote_mint, quote_mint, FusedError::QuoteMismatch);
        qs.pool_count = qs.pool_count.checked_add(1).ok_or(FusedError::Math)?;
        Ok(())
    }

    pub fn set_burn_mint(ctx: Context<Admin>, burn_mint: Pubkey) -> Result<()> {
        let g = &mut ctx.accounts.global;
        require!(!g.burn_mint_set, FusedError::BurnMintLocked);
        require!(burn_mint != Pubkey::default(), FusedError::BadMint);
        g.burn_mint = burn_mint;
        g.burn_mint_set = true;
        Ok(())
    }

    pub fn set_paused(ctx: Context<Admin>, paused: bool) -> Result<()> {
        ctx.accounts.global.paused = paused;
        Ok(())
    }

    /// Admin sets the burn operator once. Cannot be changed later.

    /// Transfer the burn pot to the burn operator's token account.
    /// The operator then does the Jupiter swap + burn off-chain.
    pub fn release_burn_pot(ctx: Context<ReleaseBurnPot>) -> Result<()> {
        require!(!ctx.accounts.global.paused, FusedError::Paused);
        let pot = ctx.accounts.pool_record.burn_pot;
        require!(pot > 0, FusedError::EmptyPot);
        // DEEP-FIX: legacy v1 lacked VaultShort; without it a short vault can still
        // release this pool's burn and leave other pools' holder pots uncovered.
        require!(pot <= ctx.accounts.quote_vault.amount, FusedError::VaultShort);
        let bump = ctx.accounts.global.bump;
        let seeds: &[&[u8]] = &[b"global", &[bump]];
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.quote_vault.to_account_info(),
                    to: ctx.accounts.operator_token.to_account_info(),
                    authority: ctx.accounts.global.to_account_info(),
                },
                &[seeds],
            ),
            pot,
        )?;
        ctx.accounts.pool_record.burn_pot = 0;
        Ok(())
    }

    pub fn migrate_global(ctx: Context<MigrateGlobal>) -> Result<()> {
        let g = &mut ctx.accounts.global;
        Ok(())
    }

    pub fn mark_graduated(ctx: Context<MarkGraduated>) -> Result<()> {
        // FIX: was callable by anyone with no checks. Now only flips when DBC says the pool migrated.
        let (_, _, is_migrated) = read_dbc_pool(&ctx.accounts.dbc_pool)?;
        require!(is_migrated == 1, FusedError::NotMigrated);
        ctx.accounts.pool_record.graduated = true;
        Ok(())
    }

    /// Claim the DBC partner fee into the PDA quote vault, then split only the delta.
    pub fn crank_claim<'info>(
        ctx: Context<'_, '_, 'info, 'info, CrankClaim<'info>>,
        max_base: u64,
        max_quote: u64,
    ) -> Result<()> {
        require!(!ctx.accounts.global.paused, FusedError::Paused);
        require!(!ctx.accounts.pool_record.graduated, FusedError::UseDammPath);

        // V3: classic SPL or Token-2022, per mint. Same 16 accounts as v2.
        let global_key = ctx.accounts.global.key();
        let qprog = *ctx.accounts.quote_mint.owner;
        let bprog = *ctx.accounts.base_mint.owner;
        require_keys_eq!(ctx.accounts.token_quote_program.key(), qprog, FusedError::WrongTokenProgram);
        require_keys_eq!(ctx.accounts.token_base_program.key(), bprog, FusedError::WrongTokenProgram);
        require!(is_token_program(&bprog), FusedError::WrongTokenProgram);
        {
            require!(ctx.accounts.base_dest.lamports() > 0, FusedError::VaultMissing);
            let bd = read_token_account(&ctx.accounts.base_dest, &bprog).map_err(|_| error!(FusedError::BadVault))?;
            require!(bd.mint == ctx.accounts.pool_record.base_mint && bd.owner == global_key, FusedError::BadVault);
        }
        let before = check_vault(&ctx.accounts.quote_vault, &global_key, &ctx.accounts.pool_record.quote_mint, &qprog)?;
        mint_policy(&ctx.accounts.quote_mint)?;
        cpi_claim_trading_fee(&ctx, max_base, max_quote)?;
        let after = token_amount(&ctx.accounts.quote_vault)?;
        let delta = after.checked_sub(before).ok_or(FusedError::Math)?;
        require!(delta > 0, FusedError::NothingToClaim);

        let burn = delta.checked_mul(BURN_NUM).ok_or(FusedError::Math)? / SPLIT_DENOM;
        let holder = delta.checked_sub(burn).ok_or(FusedError::Math)?;

        let p = &mut ctx.accounts.pool_record;
        p.burn_pot = p.burn_pot.checked_add(burn).ok_or(FusedError::Math)?;
        p.holder_pot = p.holder_pot.checked_add(holder).ok_or(FusedError::Math)?;
        p.round = p.round.checked_add(1).ok_or(FusedError::Math)?;
        Ok(())
    }

    /// Claim the partner's locked DAMM v2 position fee into the PDA quote vault, then split only the delta.
    pub fn crank_claim_damm<'info>(
        ctx: Context<'_, '_, 'info, 'info, CrankClaimDamm<'info>>,
    ) -> Result<()> {
        require!(!ctx.accounts.global.paused, FusedError::Paused);
        require!(ctx.accounts.pool_record.graduated, FusedError::UseDammPath);
        // V4 (gate-all): every graduated pool MUST pass the 5 extra accounts (remaining_accounts) =
        // [payer (signer), system_program, dbc_pool, dbc_config, creator_owed]. The program reads the pool's DBC
        // config; if the creator owns no DAMM liquidity (permanent-locked, unlocked and vested all 0) 8% of every
        // claim is set aside for the creator, otherwise (92/8 configs) nothing is taken. A claim without the
        // extras (a v3-style claim) is refused for EVERY pool, because without the config account the program cannot
        // tell the two kinds apart and crank_claim_damm is permissionless (fail closed).
        let extra = ctx.remaining_accounts;
        require!(extra.len() >= 5, FusedError::UseSplitPath);
        let (payer, sys, dbc_pool, dbc_cfg) = (&extra[0], &extra[1], &extra[2], &extra[3]);
        require!(payer.is_signer && payer.is_writable, FusedError::BadCreator);
        require_keys_eq!(sys.key(), anchor_lang::system_program::ID, FusedError::BadCreator);
        require_keys_eq!(dbc_pool.key(), ctx.accounts.pool_record.pool, FusedError::BadPool);
        let (real_config, _, _) = read_dbc_pool(dbc_pool)?;
        require_keys_eq!(dbc_cfg.key(), real_config, FusedError::ConfigMismatch);
        require_keys_eq!(dbc_cfg.key(), ctx.accounts.pool_record.config, FusedError::ConfigMismatch);
        // An unreadable / unexpected config account makes this return an error: the claim fails closed.
        let creator_gets_share = !creator_has_lp(dbc_cfg)?;

        // V3: classic SPL or Token-2022, per mint. Same 18 accounts as v2.
        let global_key = ctx.accounts.global.key();
        let aprog = *ctx.accounts.token_a_mint.owner;
        let bprog = *ctx.accounts.token_b_mint.owner;
        require_keys_eq!(ctx.accounts.token_a_program.key(), aprog, FusedError::WrongTokenProgram);
        require_keys_eq!(ctx.accounts.token_b_program.key(), bprog, FusedError::WrongTokenProgram);
        require!(is_token_program(&aprog), FusedError::WrongTokenProgram);
        require_keys_eq!(
            ctx.accounts.token_a_account.key(),
            get_associated_token_address_with_program_id(&global_key, &ctx.accounts.pool_record.base_mint, &aprog),
            FusedError::BadVault
        );
        let before = check_vault(&ctx.accounts.quote_vault, &global_key, &ctx.accounts.pool_record.quote_mint, &bprog)?;
        mint_policy(&ctx.accounts.token_b_mint)?;
        cpi_claim_position_fee(&ctx)?;
        let after = token_amount(&ctx.accounts.quote_vault)?;
        let delta = after.checked_sub(before).ok_or(FusedError::Math)?;
        require!(delta > 0, FusedError::NothingToClaim);

        // V4: the creator's 8% comes off the top (0 unless the rule above says so); the rest splits 90/110 and 20/110 as in v3.
        let creator_amt = if creator_gets_share {
            delta.checked_mul(CREATOR_SHARE_NUM).ok_or(FusedError::Math)? / CREATOR_SHARE_DENOM
        } else {
            0
        };
        if creator_amt > 0 {
            accrue_creator_share(&extra[0], &extra[1], &extra[4], &ctx.accounts.pool_record.pool, ctx.program_id, creator_amt)?;
        }
        let rest = delta.checked_sub(creator_amt).ok_or(FusedError::Math)?;
        let burn = rest.checked_mul(BURN_NUM).ok_or(FusedError::Math)? / SPLIT_DENOM;
        let holder = rest.checked_sub(burn).ok_or(FusedError::Math)?;

        let p = &mut ctx.accounts.pool_record;
        p.burn_pot = p.burn_pot.checked_add(burn).ok_or(FusedError::Math)?;
        p.holder_pot = p.holder_pot.checked_add(holder).ok_or(FusedError::Math)?;
        p.round = p.round.checked_add(1).ok_or(FusedError::Math)?;
        Ok(())
    }


    /// V4: pay the creator share set aside by crank_claim_damm. Permissionless, but the money can only go to
    /// the quote token account of the pool's creator (read from the DBC pool account). The destination must exist.
    /// Accounts: [global, pool_record, creator_owed, dbc_pool, quote_vault, quote_mint, creator_quote, quote_token_program]
    pub fn claim_creator_share<'info>(ctx: Context<'_, '_, 'info, 'info, ClaimCreatorShare>) -> Result<()> {
        let a = ctx.remaining_accounts;
        require!(a.len() >= 8, FusedError::BadCreator);
        let global: Account<Global> = Account::try_from(&a[0])?;
        require_keys_eq!(a[0].key(), Pubkey::find_program_address(&[b"global"], ctx.program_id).0, FusedError::BadCreator);
        require!(!global.paused, FusedError::Paused);
        let pool: Account<PoolRecord> = Account::try_from(&a[1])?;
        require_keys_eq!(a[1].key(), Pubkey::find_program_address(&[b"pool", pool.pool.as_ref()], ctx.program_id).0, FusedError::BadCreator);
        require_keys_eq!(a[3].key(), pool.pool, FusedError::BadPool);
        require_keys_eq!(a[2].key(), creator_owed_key(&pool.pool, ctx.program_id), FusedError::BadCreator);
        require_keys_eq!(*a[2].owner, *ctx.program_id, FusedError::BadCreator);
        let (vault, mint, dest, tprog) = (&a[4], &a[5], &a[6], &a[7]);
        let creator = read_dbc_creator(&a[3])?;
        let (owed, paid) = read_owed(&a[2], &pool.pool)?;
        require!(owed > 0, FusedError::NoCreatorShare);
        require_keys_eq!(mint.key(), pool.quote_mint, FusedError::BadMint);
        let qprog = *mint.owner;
        require_keys_eq!(tprog.key(), qprog, FusedError::WrongTokenProgram);
        let mi = mint_policy(mint)?;
        let vault_amount = check_vault(vault, &global.key(), &pool.quote_mint, &qprog)?;
        require!(owed <= vault_amount, FusedError::VaultShort);
        require_keys_eq!(
            dest.key(),
            get_associated_token_address_with_program_id(&creator, &pool.quote_mint, &qprog),
            FusedError::BadCreator
        );
        require!(dest.lamports() > 0 && *dest.owner == qprog && dest.is_writable, FusedError::CreatorAtaUnusable);
        let q = read_token_account(dest, &qprog).map_err(|_| error!(FusedError::CreatorAtaUnusable))?;
        require!(
            q.mint == pool.quote_mint && q.owner == creator && q.state == 1 && !q.memo_required,
            FusedError::CreatorAtaUnusable
        );
        let seeds: &[&[u8]] = &[b"global", &[global.bump]];
        transfer_checked_signed(tprog, vault, mint, dest, &a[0], owed, mi.decimals, seeds)?;
        write_owed(&a[2], &pool.pool, 0, paid.checked_add(owed).ok_or(FusedError::Math)?)?;
        msg!("creator_paid={}", owed);
        Ok(())
    }

    /// V2: retired. The old per-(pool, round, wallet) checkpoint scheme is replaced by PayCursor.
    /// Kept only so a stale crank fails loudly instead of paying with the old layout.
    pub fn crank_pay_page(_ctx: Context<CrankPayRetired>, _eligible_total: u64) -> Result<()> {
        err!(FusedError::UsePayV2)
    }

    /// V2: pay one page of holders.
    /// Remaining accounts are PAIRS: [holder_base_token_account, destination, ...].
    /// destination = the holder's wallet (system-owned) for plain SOL (wSOL-quoted pools only), or
    /// the holder's quote token account (classic path, any pool).
    /// Wallets must come in strictly ascending byte order within a round (PayCursor.last_owner);
    /// anything at or below the cursor is skipped, so a wallet can be paid at most once per round.
    /// Contract kept: eligible_total = eligible base still unpaid this round (shrinks page by page).
    /// Returns (return data) the base amount actually paid in this page (u64 LE).
    pub fn crank_pay_page_v2<'info>(
        ctx: Context<'_, '_, 'info, 'info, CrankPayV2<'info>>,
        eligible_total: u64,
    ) -> Result<()> {
        require!(!ctx.accounts.global.paused, FusedError::Paused);
        let payer_key = ctx.accounts.payer.key();
        require!(
            payer_key == ctx.accounts.global.admin || payer_key == CRANK_AUTHORITY,
            FusedError::NotCrank
        );
        require!(ctx.remaining_accounts.len() % 2 == 0, FusedError::BadPage);

        let round = ctx.accounts.pool_record.round;
        let pool_key = ctx.accounts.pool_record.pool;
        let quote_mint = ctx.accounts.pool_record.quote_mint;
        let base_mint = ctx.accounts.pool_record.base_mint;
        let global_key = ctx.accounts.global.key();

        // ---- cursor: fresh cursors start CLOSED for the current round, so wallets the old
        //      checkpoint scheme already paid this round can never be paid again.
        {
            let c = &mut ctx.accounts.pay_cursor;
            if c.pool == Pubkey::default() {
                c.pool = pool_key;
                c.bump = ctx.bumps.pay_cursor;
                c.round = round;
                c.last_owner = MAX_OWNER;
                c.paid_base = 0;
                c.paid_amount = 0;
            } else if c.round < round {
                c.round = round;
                c.last_owner = Pubkey::default();
                c.paid_base = 0;
                c.paid_amount = 0;
            }
            require!(c.round == round, FusedError::BadRound);
        }
        // An empty page only creates/syncs the cursor (lets the crank open it before a pool's first claim).
        if ctx.remaining_accounts.is_empty() {
            return Ok(());
        }
        require!(round > 0, FusedError::BadRound);
        require!(eligible_total > 0, FusedError::BadTotal);
        let pot = ctx.accounts.pool_record.holder_pot;
        require!(pot > 0, FusedError::EmptyPot);

        let is_sol = quote_mint == NATIVE_MINT;
        let rent = Rent::get()?;
        let wallet_min = rent.minimum_balance(0);
        let sol_vault_key = ctx.accounts.sol_vault.key();
        let unwrap_key = ctx.accounts.unwrap_account.key();

        // ---- pass 1: validate every pair and work out shares. No money moves here.
        let mut last = ctx.accounts.pay_cursor.last_owner;
        let mut lamport_pays: Vec<(usize, u64)> = Vec::new();
        let mut token_pays: Vec<(usize, u64)> = Vec::new();
        let mut paid_base: u64 = 0;
        let mut i = 0;
        while i < ctx.remaining_accounts.len() {
            let base_ai = &ctx.remaining_accounts[i];
            let dest_ai = &ctx.remaining_accounts[i + 1];
            let dest_idx = i + 1;
            i += 2;

            let base: Account<TokenAccount> = Account::try_from(base_ai)?;
            require!(base.mint == base_mint, FusedError::BadMint);
            let wallet = base.owner;
            if wallet <= last {
                continue; // already handled this round (or out of order): never paid twice
            }
            last = wallet;
            if base.amount == 0 {
                continue;
            }
            // never pay the curve's vault authority, the DAMM authority, Global or our own PDAs
            if wallet == DBC_POOL_AUTHORITY
                || wallet == DAMM_POOL_AUTHORITY
                || wallet == global_key
                || wallet == sol_vault_key
                || wallet == unwrap_key
            {
                msg!("skip excluded holder {}", wallet);
                continue;
            }
            let share = ((pot as u128)
                .checked_mul(base.amount as u128)
                .ok_or(FusedError::Math)?
                / (eligible_total as u128)) as u64;
            if share == 0 {
                continue;
            }

            if *dest_ai.owner == anchor_lang::system_program::ID {
                // plain-SOL path
                require!(is_sol, FusedError::BadDest);
                require_keys_eq!(dest_ai.key(), wallet, FusedError::OwnerMismatch);
                require!(dest_ai.is_writable, FusedError::BadDest);
                require!(!dest_ai.executable && dest_ai.data_is_empty(), FusedError::BadDest);
                // A wallet must end rent-exempt (the runtime rejects a credit that leaves it below).
                // Covers empty (0-lamport) wallets and any legacy below-rent wallet.
                if dest_ai.lamports().saturating_add(share) < wallet_min {
                    msg!("carry forward {} lamports for wallet {} (would stay below rent-exempt)", share, wallet);
                    continue; // stays in the pot for a later round
                }
                lamport_pays.push((dest_idx, share));
            } else {
                // classic token path
                let q: Account<TokenAccount> = Account::try_from(dest_ai)?;
                require!(q.mint == quote_mint, FusedError::BadMint);
                require_keys_eq!(q.owner, wallet, FusedError::OwnerMismatch);
                require!(dest_ai.is_writable, FusedError::BadDest);
                token_pays.push((dest_idx, share));
            }
            paid_base = paid_base.checked_add(base.amount).ok_or(FusedError::Math)?;
        }

        let lamport_total = lamport_pays
            .iter()
            .try_fold(0u64, |a, x| a.checked_add(x.1))
            .ok_or(FusedError::Math)?;
        let token_total = token_pays
            .iter()
            .try_fold(0u64, |a, x| a.checked_add(x.1))
            .ok_or(FusedError::Math)?;
        let total = lamport_total.checked_add(token_total).ok_or(FusedError::Math)?;
        require!(total <= pot, FusedError::Math);
        // DEEP-FIX: mirror v3 — refuse if the shared vault cannot cover this page.
        require!(total <= ctx.accounts.quote_vault.amount, FusedError::VaultShort);

        let gbump = ctx.accounts.global.bump;
        let gseeds: &[&[u8]] = &[b"global", &[gbump]];

        // ---- pass 2a: classic token payouts, exactly as before
        for (idx, share) in token_pays.iter() {
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.quote_vault.to_account_info(),
                        to: ctx.remaining_accounts[*idx].clone(),
                        authority: ctx.accounts.global.to_account_info(),
                    },
                    &[gseeds],
                ),
                *share,
            )?;
        }

        // ---- pass 2b: unwrap ONLY this page's lamport total, then pay wallets directly
        if lamport_total > 0 {
            let sol_vault_ai = ctx.accounts.sol_vault.to_account_info();
            let unwrap_ai = ctx.accounts.unwrap_account.to_account_info();
            let payer_ai = ctx.accounts.payer.to_account_info();
            let vault_before = sol_vault_ai.lamports();
            let temp_rent = rent.minimum_balance(TOKEN_ACCOUNT_LEN as usize);
            let ubump = ctx.bumps.unwrap_account;
            let useeds: &[&[u8]] = &[b"unwrap", pool_key.as_ref(), &[ubump]];

            create_pda_account(
                &payer_ai,
                &unwrap_ai,
                &ctx.accounts.system_program.to_account_info(),
                temp_rent,
                TOKEN_ACCOUNT_LEN,
                &anchor_spl::token::ID,
                useeds,
            )?;
            token::initialize_account3(CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                InitializeAccount3 {
                    account: unwrap_ai.clone(),
                    mint: ctx.accounts.native_mint.to_account_info(),
                    authority: ctx.accounts.global.to_account_info(),
                },
            ))?;
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.quote_vault.to_account_info(),
                        to: unwrap_ai.clone(),
                        authority: ctx.accounts.global.to_account_info(),
                    },
                    &[gseeds],
                ),
                lamport_total,
            )?;
            let unwrap_lamports = unwrap_ai.lamports();
            token::close_account(CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                CloseAccount {
                    account: unwrap_ai.clone(),
                    destination: sol_vault_ai.clone(),
                    authority: ctx.accounts.global.to_account_info(),
                },
                &[gseeds],
            ))?;
            require!(
                sol_vault_ai.lamports()
                    == vault_before.checked_add(unwrap_lamports).ok_or(FusedError::Math)?,
                FusedError::Math
            );
            for (idx, share) in lamport_pays.iter() {
                let dest = &ctx.remaining_accounts[*idx];
                **sol_vault_ai.try_borrow_mut_lamports()? -= *share;
                **dest.try_borrow_mut_lamports()? += *share;
            }
            // anything left above the vault's own balance (the temp account's rent) goes back to the payer
            let back = sol_vault_ai
                .lamports()
                .checked_sub(vault_before)
                .ok_or(FusedError::Math)?;
            **sol_vault_ai.try_borrow_mut_lamports()? -= back;
            **payer_ai.try_borrow_mut_lamports()? += back; // the temp account's rent (plus any stray donation)
            require!(sol_vault_ai.lamports() == vault_before, FusedError::Math);
        }

        let p = &mut ctx.accounts.pool_record;
        p.holder_pot = p.holder_pot.checked_sub(total).ok_or(FusedError::Math)?;
        let c = &mut ctx.accounts.pay_cursor;
        c.last_owner = last;
        c.paid_base = c.paid_base.checked_add(paid_base).ok_or(FusedError::Math)?;
        c.paid_amount = c.paid_amount.checked_add(total).ok_or(FusedError::Math)?;
        msg!("paid_base={} paid_amount={} lamports={} tokens={}", paid_base, total, lamport_total, token_total);
        anchor_lang::solana_program::program::set_return_data(&paid_base.to_le_bytes());
        Ok(())
    }

    /// V3: pay one page of holders in the pool's quote token, for ANY token program
    /// (classic SPL or Token-2022) and any decimals, using transfer_checked.
    /// Same cursor (PayCursor PDA), same share maths and same contract as crank_pay_page_v2,
    /// so a wallet can never be paid twice in a round across v2 and v3.
    /// Remaining accounts are PAIRS: [holder_base_token_account, holder_quote_token_account, ...].
    /// No plain-SOL path here (wSOL pools keep using crank_pay_page_v2).
    /// Frozen or memo-required destinations are carried forward (share stays in the pot).
    /// Returns (return data) the base amount actually paid in this page (u64 LE).
    pub fn crank_pay_page_v3<'info>(
        ctx: Context<'_, '_, 'info, 'info, CrankPayV3<'info>>,
        eligible_total: u64,
    ) -> Result<()> {
        require!(!ctx.accounts.global.paused, FusedError::Paused);
        let payer_key = ctx.accounts.payer.key();
        require!(
            payer_key == ctx.accounts.global.admin || payer_key == CRANK_AUTHORITY,
            FusedError::NotCrank
        );
        require!(ctx.remaining_accounts.len() % 2 == 0, FusedError::BadPage);

        let round = ctx.accounts.pool_record.round;
        let pool_key = ctx.accounts.pool_record.pool;
        let quote_mint = ctx.accounts.pool_record.quote_mint;
        let base_mint = ctx.accounts.pool_record.base_mint;
        let global_key = ctx.accounts.global.key();
        let qprog = *ctx.accounts.quote_mint.owner;
        let bprog = *ctx.accounts.base_mint.owner;
        require_keys_eq!(ctx.accounts.quote_token_program.key(), qprog, FusedError::WrongTokenProgram);
        require!(is_token_program(&bprog), FusedError::WrongTokenProgram);
        let vault_amount = check_vault(&ctx.accounts.quote_vault, &global_key, &quote_mint, &qprog)?;
        let mi = mint_policy(&ctx.accounts.quote_mint)?;

        sync_cursor(&mut ctx.accounts.pay_cursor, pool_key, round, ctx.bumps.pay_cursor)?;
        if ctx.remaining_accounts.is_empty() {
            return Ok(());
        }
        require!(round > 0, FusedError::BadRound);
        require!(eligible_total > 0, FusedError::BadTotal);
        let pot = ctx.accounts.pool_record.holder_pot;
        require!(pot > 0, FusedError::EmptyPot);
        let sol_vault_key = Pubkey::find_program_address(&[b"sol_vault"], ctx.program_id).0;
        let unwrap_key = Pubkey::find_program_address(&[b"unwrap", pool_key.as_ref()], ctx.program_id).0;

        let mut last = ctx.accounts.pay_cursor.last_owner;
        let mut pays: Vec<(usize, u64)> = Vec::new();
        let mut paid_base: u64 = 0;
        let mut total: u64 = 0;
        let mut i = 0;
        while i < ctx.remaining_accounts.len() {
            let base_ai = &ctx.remaining_accounts[i];
            let dest_ai = &ctx.remaining_accounts[i + 1];
            let dest_idx = i + 1;
            i += 2;

            let base = read_token_account(base_ai, &bprog)?;
            require!(base.mint == base_mint, FusedError::BadMint);
            let wallet = base.owner;
            if wallet <= last {
                continue;
            }
            last = wallet;
            if base.amount == 0 {
                continue;
            }
            if wallet == DBC_POOL_AUTHORITY
                || wallet == DAMM_POOL_AUTHORITY
                || wallet == global_key
                || wallet == sol_vault_key
                || wallet == unwrap_key
            {
                msg!("skip excluded holder {}", wallet);
                continue;
            }
            let share = ((pot as u128)
                .checked_mul(base.amount as u128)
                .ok_or(FusedError::Math)?
                / (eligible_total as u128)) as u64;
            if share == 0 {
                continue;
            }
            let q = read_token_account(dest_ai, &qprog).map_err(|_| error!(FusedError::BadDest))?;
            require!(q.mint == quote_mint, FusedError::BadMint);
            require_keys_eq!(q.owner, wallet, FusedError::OwnerMismatch);
            require!(dest_ai.is_writable, FusedError::BadDest);
            if q.state != 1 || q.memo_required {
                msg!("carry forward {} for wallet {} (destination frozen or memo-required)", share, wallet);
                continue;
            }
            pays.push((dest_idx, share));
            total = total.checked_add(share).ok_or(FusedError::Math)?;
            paid_base = paid_base.checked_add(base.amount).ok_or(FusedError::Math)?;
        }
        require!(total <= pot, FusedError::Math);
        require!(total <= vault_amount, FusedError::VaultShort);

        let gbump = ctx.accounts.global.bump;
        let gseeds: &[&[u8]] = &[b"global", &[gbump]];
        for (idx, share) in pays.iter() {
            transfer_checked_signed(
                &ctx.accounts.quote_token_program,
                &ctx.accounts.quote_vault,
                &ctx.accounts.quote_mint,
                &ctx.remaining_accounts[*idx],
                &ctx.accounts.global.to_account_info(),
                *share,
                mi.decimals,
                gseeds,
            )?;
        }

        let p = &mut ctx.accounts.pool_record;
        p.holder_pot = p.holder_pot.checked_sub(total).ok_or(FusedError::Math)?;
        let c = &mut ctx.accounts.pay_cursor;
        c.last_owner = last;
        c.paid_base = c.paid_base.checked_add(paid_base).ok_or(FusedError::Math)?;
        c.paid_amount = c.paid_amount.checked_add(total).ok_or(FusedError::Math)?;
        msg!("paid_base={} paid_amount={} holders={}", paid_base, total, pays.len());
        anchor_lang::solana_program::program::set_return_data(&paid_base.to_le_bytes());
        Ok(())
    }

    /// V3: release_burn_pot for any token program (transfer_checked), with a solvency check.
    /// Signerless like release_burn_pot; the operator token account must belong to the admin.
    pub fn release_burn_pot_v2(ctx: Context<ReleaseBurnPotV2>) -> Result<()> {
        require!(!ctx.accounts.global.paused, FusedError::Paused);
        let pot = ctx.accounts.pool_record.burn_pot;
        require!(pot > 0, FusedError::EmptyPot);
        let quote_mint = ctx.accounts.pool_record.quote_mint;
        let qprog = *ctx.accounts.quote_mint.owner;
        require_keys_eq!(ctx.accounts.token_program.key(), qprog, FusedError::WrongTokenProgram);
        let mi = mint_policy(&ctx.accounts.quote_mint)?;
        let vault_amount = check_vault(&ctx.accounts.quote_vault, &ctx.accounts.global.key(), &quote_mint, &qprog)?;
        require!(pot <= vault_amount, FusedError::VaultShort);
        let op = read_token_account(&ctx.accounts.operator_token, &qprog).map_err(|_| error!(FusedError::BadDest))?;
        require!(op.mint == quote_mint, FusedError::BadMint);
        require_keys_eq!(op.owner, ctx.accounts.global.admin, FusedError::BadDest);
        let bump = ctx.accounts.global.bump;
        let seeds: &[&[u8]] = &[b"global", &[bump]];
        transfer_checked_signed(
            &ctx.accounts.token_program,
            &ctx.accounts.quote_vault,
            &ctx.accounts.quote_mint,
            &ctx.accounts.operator_token,
            &ctx.accounts.global.to_account_info(),
            pot,
            mi.decimals,
            seeds,
        )?;
        ctx.accounts.pool_record.burn_pot = 0;
        Ok(())
    }

    /// V3: view-only pair check. Accounts: [mint]. Changes nothing; simulate it for free.
    /// Return data: [tier u8 (0 supported, 1 issuer-controlled), decimals u8, token_2022 u8, flags u32 LE].
    /// Fails with the same error the claim/pay/register paths would give for an unsupported mint.
    pub fn check_quote_mint(ctx: Context<CheckQuoteMint>) -> Result<()> {
        let mi = mint_policy(&ctx.accounts.mint)?;
        let mut out = [0u8; 7];
        out[0] = mi.tier;
        out[1] = mi.decimals;
        out[2] = (mi.flags & F_TOKEN_2022 != 0) as u8;
        out[3..7].copy_from_slice(&mi.flags.to_le_bytes());
        msg!("pair_check tier={} decimals={} flags={:#x}", mi.tier, mi.decimals, mi.flags);
        anchor_lang::solana_program::program::set_return_data(&out);
        Ok(())
    }

    /// V2: close old 8-byte checkpoints [b"ckpt", pool, round_le, owner] of one pool and send their
    /// rent to CRANK_AUTHORITY. Remaining accounts = the checkpoints, in the same order as `entries`.
    /// The address is re-derived from (pool, round, owner, bump), so only real checkpoints match.
    /// Only rounds BEFORE the pool's current round can be closed.
    pub fn close_old_checkpoints<'info>(
        ctx: Context<'_, '_, 'info, 'info, CloseOldCheckpoints<'info>>,
        entries: Vec<LegacyCheckpoint>,
    ) -> Result<()> {
        let signer = ctx.accounts.signer.key();
        require!(
            signer == ctx.accounts.global.admin || signer == CRANK_AUTHORITY,
            FusedError::NotCrank
        );
        require!(
            entries.len() == ctx.remaining_accounts.len() && !entries.is_empty(),
            FusedError::BadPage
        );
        let pool_key = ctx.accounts.pool_record.pool;
        let current = ctx.accounts.pool_record.round;
        let receiver = ctx.accounts.rent_receiver.to_account_info();
        for (ai, e) in ctx.remaining_accounts.iter().zip(entries.iter()) {
            require!(e.round < current, FusedError::BadRound);
            let k = Pubkey::create_program_address(
                &[
                    b"ckpt",
                    pool_key.as_ref(),
                    &e.round.to_le_bytes(),
                    e.owner.as_ref(),
                    &[e.bump],
                ],
                ctx.program_id,
            )
            .map_err(|_| error!(FusedError::BadCheckpoint))?;
            require_keys_eq!(k, ai.key(), FusedError::BadCheckpoint);
            require_keys_eq!(*ai.owner, *ctx.program_id, FusedError::BadCheckpoint);
            require!(ai.data_len() == 8 && ai.is_writable, FusedError::BadCheckpoint);
            let l = ai.lamports();
            **ai.try_borrow_mut_lamports()? = 0;
            let new_bal = receiver.lamports().checked_add(l).ok_or(FusedError::Math)?;
            **receiver.try_borrow_mut_lamports()? = new_bal;
            ai.resize(0)?;
            ai.assign(&anchor_lang::system_program::ID);
        }
        Ok(())
    }

    /// FIX: admin repairs PoolRecord.config. The new config must be the one the DBC pool
    /// actually uses, belong to the same quote mint, and have the Global PDA as fee claimer.
    pub fn set_pool_config(ctx: Context<SetPoolConfig>) -> Result<()> {
        let (real_config, real_base_mint, _) = read_dbc_pool(&ctx.accounts.dbc_pool)?;
        require_keys_eq!(real_base_mint, ctx.accounts.pool_record.base_mint, FusedError::BadMint);
        require_keys_eq!(ctx.accounts.new_config.key(), real_config, FusedError::ConfigMismatch);
        let quote_mint = ctx.accounts.pool_record.quote_mint;
        check_dbc_config(&ctx.accounts.new_config, &quote_mint, &ctx.accounts.global.key())?;
        ctx.accounts.pool_record.config = real_config;
        Ok(())
    }

    /// FIX: admin can undo a wrong graduated flag (e.g. one set by the old open mark_graduated).
    pub fn admin_set_graduated(ctx: Context<AdminPool>, graduated: bool) -> Result<()> {
        ctx.accounts.pool_record.graduated = graduated;
        Ok(())
    }

    /// Burn the waiting slice only when the quote mint is the burn mint. Otherwise leave it.
    pub fn crank_burn_direct(ctx: Context<CrankBurnDirect>) -> Result<()> {
        require!(!ctx.accounts.global.paused, FusedError::Paused);
        require!(ctx.accounts.global.burn_mint_set, FusedError::BurnMintUnset);
        require!(
            ctx.accounts.pool_record.quote_mint == ctx.accounts.global.burn_mint,
            FusedError::NeedsSwap
        );
        let amount = ctx.accounts.pool_record.burn_pot;
        require!(amount > 0, FusedError::EmptyPot);
        // DEEP-FIX: refuse if vault cannot cover the burn (same class as release_burn_pot).
        require!(amount <= ctx.accounts.quote_vault.amount, FusedError::VaultShort);

        let bump = ctx.accounts.global.bump;
        let seeds: &[&[u8]] = &[b"global", &[bump]];
        let cpi = Burn {
            mint: ctx.accounts.burn_mint.to_account_info(),
            from: ctx.accounts.quote_vault.to_account_info(),
            authority: ctx.accounts.global.to_account_info(),
        };
        token::burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi,
                &[seeds],
            ),
            amount,
        )?;
        ctx.accounts.pool_record.burn_pot = 0;
        Ok(())
    }

    /// V5: create the AutoLpConfig PDA (once). Default share is 50% of the burn pot to locked LP.
    /// Targets (DAMM pool / position) stay unset until admin calls set_auto_lp after $FUSED graduates.
    pub fn init_auto_lp(ctx: Context<InitAutoLp>) -> Result<()> {
        let a = &mut ctx.accounts.auto_lp;
        a.damm_pool = Pubkey::default();
        a.position = Pubkey::default();
        a.position_nft_account = Pubkey::default();
        a.token_a_mint = Pubkey::default();
        a.token_b_mint = Pubkey::default();
        a.lp_share_bps = DEFAULT_AUTO_LP_BPS;
        a.bump = ctx.bumps.auto_lp;
        Ok(())
    }

    /// V5: admin sets the $FUSED DAMM v2 pool + Global-owned position that auto-LP tops up.
    /// Call this once $FUSED has graduated and the migration position is known.
    pub fn set_auto_lp(
        ctx: Context<SetAutoLp>,
        damm_pool: Pubkey,
        position: Pubkey,
        position_nft_account: Pubkey,
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
    ) -> Result<()> {
        require!(damm_pool != Pubkey::default(), FusedError::BadPool);
        require!(position != Pubkey::default(), FusedError::BadPool);
        require!(position_nft_account != Pubkey::default(), FusedError::BadPool);
        require!(
            token_a_mint != Pubkey::default() && token_b_mint != Pubkey::default(),
            FusedError::BadMint
        );
        require!(token_a_mint != token_b_mint, FusedError::BadMint);
        let a = &mut ctx.accounts.auto_lp;
        a.damm_pool = damm_pool;
        a.position = position;
        a.position_nft_account = position_nft_account;
        a.token_a_mint = token_a_mint;
        a.token_b_mint = token_b_mint;
        Ok(())
    }

    /// V5: admin sets how much of the 0.20% burn share goes to locked $FUSED LP (0..=10000 bps).
    /// 5000 = half burn / half LP. 0 = burn only (today's behaviour). 10000 = all LP, no burn.
    pub fn set_auto_lp_share(ctx: Context<SetAutoLpShare>, lp_share_bps: u16) -> Result<()> {
        require!(lp_share_bps <= 10_000, FusedError::BadAutoLpShare);
        ctx.accounts.auto_lp.lp_share_bps = lp_share_bps;
        Ok(())
    }

    /// PERFECT: admin sets/repairs QuoteStat.pool_count after upgrade (existing pools were
    /// registered before the counter existed). New registers increment via register_pool.
    pub fn sync_quote_stat(ctx: Context<SyncQuoteStat>, quote_mint: Pubkey, pool_count: u64) -> Result<()> {
        require!(quote_mint != Pubkey::default(), FusedError::BadMint);
        let qs = &mut ctx.accounts.quote_stat;
        qs.quote_mint = quote_mint;
        qs.pool_count = pool_count;
        qs.bump = ctx.bumps.quote_stat;
        msg!("quote_stat mint={} count={}", quote_mint, pool_count);
        Ok(())
    }

    /// V5: add SOL+$FUSED (already in Global's ATAs) to the configured DAMM position and permanently lock.
    /// Crank (or admin) only. Fails with AutoLpNotReady until set_auto_lp has been called
    /// (pre-graduation = crank keeps burning 100%, never calls this).
    /// Args match Meteora DAMM v2 add_liquidity; liquidity_delta is also the permanent_lock amount.
    /// Accounts (remaining): [global, auto_lp, payer(signer), damm_pool, position, position_nft_account,
    ///   token_a_account, token_b_account, token_a_vault, token_b_vault, token_a_mint, token_b_mint,
    ///   token_a_program, token_b_program, event_authority, damm_program]
    pub fn crank_auto_lp<'info>(
        ctx: Context<'_, '_, 'info, 'info, CrankAutoLp>,
        liquidity_delta: u128,
        token_a_amount_threshold: u64,
        token_b_amount_threshold: u64,
    ) -> Result<()> {
        let a = ctx.remaining_accounts;
        require!(a.len() >= 16, FusedError::BadAutoLpAccounts);
        let global: Account<Global> = Account::try_from(&a[0])?;
        require_keys_eq!(
            a[0].key(),
            Pubkey::find_program_address(&[b"global"], ctx.program_id).0,
            FusedError::BadAutoLpAccounts
        );
        require!(!global.paused, FusedError::Paused);
        require_keys_eq!(
            a[1].key(),
            Pubkey::find_program_address(&[b"autolp"], ctx.program_id).0,
            FusedError::BadAutoLpAccounts
        );
        let auto_lp: Account<AutoLpConfig> = Account::try_from(&a[1])?;
        require!(auto_lp.damm_pool != Pubkey::default(), FusedError::AutoLpNotReady);
        let payer = &a[2];
        require!(payer.is_signer, FusedError::NotCrank);
        require!(
            payer.key() == global.admin || payer.key() == CRANK_AUTHORITY,
            FusedError::NotCrank
        );

        let (
            damm_pool, position, nft, tok_a, tok_b, vault_a, vault_b, mint_a, mint_b, prog_a, prog_b, ev, damm,
        ) = (
            &a[3], &a[4], &a[5], &a[6], &a[7], &a[8], &a[9], &a[10], &a[11], &a[12], &a[13], &a[14], &a[15],
        );
        require_keys_eq!(damm_pool.key(), auto_lp.damm_pool, FusedError::BadPool);
        require_keys_eq!(position.key(), auto_lp.position, FusedError::BadPool);
        require_keys_eq!(nft.key(), auto_lp.position_nft_account, FusedError::BadPool);
        require_keys_eq!(mint_a.key(), auto_lp.token_a_mint, FusedError::BadMint);
        require_keys_eq!(mint_b.key(), auto_lp.token_b_mint, FusedError::BadMint);
        require_keys_eq!(damm.key(), DAMM_PROGRAM_ID, FusedError::BadAutoLpAccounts);
        require!(damm_pool.is_writable && position.is_writable, FusedError::BadAutoLpAccounts);
        require!(tok_a.is_writable && tok_b.is_writable, FusedError::BadAutoLpAccounts);
        require!(vault_a.is_writable && vault_b.is_writable, FusedError::BadAutoLpAccounts);

        let aprog = *mint_a.owner;
        let bprog = *mint_b.owner;
        require_keys_eq!(prog_a.key(), aprog, FusedError::WrongTokenProgram);
        require_keys_eq!(prog_b.key(), bprog, FusedError::WrongTokenProgram);
        require!(is_token_program(&aprog) && is_token_program(&bprog), FusedError::WrongTokenProgram);

        let gkey = global.key();
        // Token accounts must be Global's own ATAs — funds never leave to a caller-chosen wallet.
        require_keys_eq!(
            tok_a.key(),
            get_associated_token_address_with_program_id(&gkey, &auto_lp.token_a_mint, &aprog),
            FusedError::BadVault
        );
        require_keys_eq!(
            tok_b.key(),
            get_associated_token_address_with_program_id(&gkey, &auto_lp.token_b_mint, &bprog),
            FusedError::BadVault
        );
        // Position NFT must be owned by Global (migration partner position).
        // DAMM position NFTs are Token-2022; fall back to classic SPL just in case.
        let nft_info = match read_token_account(nft, &TOKEN_2022_ID) {
            Ok(v) => v,
            Err(_) => read_token_account(nft, &anchor_spl::token::ID)
                .map_err(|_| error!(FusedError::BadAutoLpAccounts))?,
        };
        require_keys_eq!(nft_info.owner, gkey, FusedError::BadAutoLpAccounts);
        require!(nft_info.amount == 1, FusedError::BadAutoLpAccounts);

        require!(liquidity_delta > 0, FusedError::BadAutoLpAmounts);
        require!(
            token_a_amount_threshold > 0 && token_b_amount_threshold > 0,
            FusedError::BadAutoLpAmounts
        );
        // FIX (KupiaSec + Crank design + PERFECT M3): LP funded FROM burn_pot;
        // on-chain QuoteStat forces the full PoolRecord set for each quote mint.
        //
        // remaining_accounts layout (Crank MUST match exactly):
        //   [0]  global PDA
        //   [1]  auto_lp PDA
        //   [2]  payer signer (admin | CRANK_AUTHORITY)
        //   [3]  damm_pool
        //   [4]  position
        //   [5]  position_nft_account
        //   [6]  token_a_account  (Global ATA)
        //   [7]  token_b_account  (Global ATA)
        //   [8]  token_a_vault
        //   [9]  token_b_vault
        //   [10] token_a_mint
        //   [11] token_b_mint
        //   [12] token_a_program
        //   [13] token_b_program
        //   [14] event_authority
        //   [15] damm_program
        //   [16] QuoteStat PDA seeds=["qstat", token_a_mint]
        //   [17] QuoteStat PDA seeds=["qstat", token_b_mint]
        //   [18..] writable PoolRecords for EVERY pool with quote in {token_a, token_b}
        //          (count per mint MUST equal QuoteStat.pool_count), then CreatorOwed PDAs
        //          (each AFTER its PoolRecord).
        //
        // Per PoolRecord: lp_budget = burn_pot * lp_share_bps / 10000.
        // Spend from burn is capped by sum(budgets) on-chain (C2). Surplus (unbooked) may
        // also be spent (e.g. pre-bought $FUSED) without touching burn_pot.
        // After add+lock: decrement burn_pots by from_burn; holder/owed untouched;
        // vault >= sum(holder + remaining_burn + owed) else VaultShort.
        let lp_share_bps = auto_lp.lp_share_bps;
        require!(lp_share_bps <= 10_000, FusedError::BadAutoLpShare);
        msg!("auto_lp_share_bps={}", lp_share_bps);

        let before_a = token_amount(tok_a)?;
        let before_b = token_amount(tok_b)?;

        // Need quote stats + at least room for them; pools may be zero if both counts are 0.
        require!(a.len() >= 18, FusedError::BadAutoLpAccounts);

        let qs_a_ai = &a[16];
        let qs_b_ai = &a[17];
        require_keys_eq!(*qs_a_ai.owner, *ctx.program_id, FusedError::BadAutoLpAccounts);
        require_keys_eq!(*qs_b_ai.owner, *ctx.program_id, FusedError::BadAutoLpAccounts);
        let (qs_a_key, _) = Pubkey::find_program_address(&[b"qstat", auto_lp.token_a_mint.as_ref()], ctx.program_id);
        let (qs_b_key, _) = Pubkey::find_program_address(&[b"qstat", auto_lp.token_b_mint.as_ref()], ctx.program_id);
        require_keys_eq!(qs_a_ai.key(), qs_a_key, FusedError::BadAutoLpAccounts);
        require_keys_eq!(qs_b_ai.key(), qs_b_key, FusedError::BadAutoLpAccounts);
        let qs_a: Account<QuoteStat> = Account::try_from(qs_a_ai)?;
        let qs_b: Account<QuoteStat> = Account::try_from(qs_b_ai)?;
        require_keys_eq!(qs_a.quote_mint, auto_lp.token_a_mint, FusedError::BadMint);
        require_keys_eq!(qs_b.quote_mint, auto_lp.token_b_mint, FusedError::BadMint);
        let expect_a = qs_a.pool_count;
        let expect_b = qs_b.pool_count;

        let mut pool_ais: Vec<&AccountInfo<'info>> = Vec::new();
        let mut pool_keys: Vec<Pubkey> = Vec::new();
        let mut pool_quotes: Vec<Pubkey> = Vec::new();
        let mut pool_budgets: Vec<u64> = Vec::new();
        let mut holder_a: u64 = 0;
        let mut holder_b: u64 = 0;
        let mut burn_a: u64 = 0;
        let mut burn_b: u64 = 0;
        let mut owed_a: u64 = 0;
        let mut owed_b: u64 = 0;
        let mut budget_a: u64 = 0;
        let mut budget_b: u64 = 0;
        let mut count_a: u64 = 0;
        let mut count_b: u64 = 0;
        let mut seen_pools: Vec<(Pubkey, Pubkey)> = Vec::new();

        for ai in a.iter().skip(18) {
            require_keys_eq!(*ai.owner, *ctx.program_id, FusedError::BadAutoLpAccounts);
            let is_owed = {
                let d = ai.try_borrow_data()?;
                d.len() == OWED_LEN && d[..8] == OWED_TAG
            };
            if is_owed {
                let (pool, owed) = {
                    let d = ai.try_borrow_data()?;
                    let pool = Pubkey::try_from(&d[8..40]).map_err(|_| error!(FusedError::BadCreator))?;
                    let owed = u64::from_le_bytes(d[40..48].try_into().unwrap());
                    (pool, owed)
                };
                require_keys_eq!(ai.key(), creator_owed_key(&pool, ctx.program_id), FusedError::BadAutoLpAccounts);
                let quote = seen_pools
                    .iter()
                    .find(|(p, _)| *p == pool)
                    .map(|(_, q)| *q)
                    .ok_or_else(|| error!(FusedError::BadAutoLpAccounts))?;
                if quote == auto_lp.token_a_mint {
                    owed_a = owed_a.checked_add(owed).ok_or(FusedError::Math)?;
                } else if quote == auto_lp.token_b_mint {
                    owed_b = owed_b.checked_add(owed).ok_or(FusedError::Math)?;
                } else {
                    return err!(FusedError::BadMint);
                }
            } else {
                require!(ai.is_writable, FusedError::BadAutoLpAccounts);
                let pr: Account<PoolRecord> = Account::try_from(ai)?;
                let (expected, _) = Pubkey::find_program_address(&[b"pool", pr.pool.as_ref()], ctx.program_id);
                require_keys_eq!(ai.key(), expected, FusedError::BadAutoLpAccounts);
                let budget = (pr.burn_pot as u128)
                    .checked_mul(lp_share_bps as u128)
                    .ok_or(FusedError::Math)?
                    .checked_div(10_000u128)
                    .ok_or(FusedError::Math)? as u64;
                if pr.quote_mint == auto_lp.token_a_mint {
                    holder_a = holder_a.checked_add(pr.holder_pot).ok_or(FusedError::Math)?;
                    burn_a = burn_a.checked_add(pr.burn_pot).ok_or(FusedError::Math)?;
                    budget_a = budget_a.checked_add(budget).ok_or(FusedError::Math)?;
                    count_a = count_a.checked_add(1).ok_or(FusedError::Math)?;
                } else if pr.quote_mint == auto_lp.token_b_mint {
                    holder_b = holder_b.checked_add(pr.holder_pot).ok_or(FusedError::Math)?;
                    burn_b = burn_b.checked_add(pr.burn_pot).ok_or(FusedError::Math)?;
                    budget_b = budget_b.checked_add(budget).ok_or(FusedError::Math)?;
                    count_b = count_b.checked_add(1).ok_or(FusedError::Math)?;
                } else {
                    return err!(FusedError::BadMint);
                }
                seen_pools.push((pr.pool, pr.quote_mint));
                pool_ais.push(ai);
                pool_keys.push(pr.pool);
                pool_quotes.push(pr.quote_mint);
                pool_budgets.push(budget);
            }
        }
        // PERFECT M3: crank cannot understate the reserved set.
        require!(count_a == expect_a, FusedError::BadAutoLpAccounts);
        require!(count_b == expect_b, FusedError::BadAutoLpAccounts);
        msg!("auto_lp_pools count_a={} count_b={} budget_a={} budget_b={}", count_a, count_b, budget_a, budget_b);

        // Fail-fast: thresholds must not exceed burn LP budget + unbooked surplus.
        let booked_a = holder_a
            .checked_add(burn_a)
            .ok_or(FusedError::Math)?
            .checked_add(owed_a)
            .ok_or(FusedError::Math)?;
        let booked_b = holder_b
            .checked_add(burn_b)
            .ok_or(FusedError::Math)?
            .checked_add(owed_b)
            .ok_or(FusedError::Math)?;
        let surplus_a = before_a.saturating_sub(booked_a);
        let surplus_b = before_b.saturating_sub(booked_b);
        let max_spend_a = surplus_a.checked_add(budget_a).ok_or(FusedError::Math)?;
        let max_spend_b = surplus_b.checked_add(budget_b).ok_or(FusedError::Math)?;
        require!(token_a_amount_threshold <= max_spend_a, FusedError::BadAutoLpAmounts);
        require!(token_b_amount_threshold <= max_spend_b, FusedError::BadAutoLpAmounts);

        let bump = global.bump;
        let seeds: &[&[u8]] = &[b"global", &[bump]];
        cpi_add_liquidity(
            damm_pool, position, tok_a, tok_b, vault_a, vault_b, mint_a, mint_b, nft, &a[0],
            prog_a, prog_b, ev, damm, seeds, liquidity_delta, token_a_amount_threshold,
            token_b_amount_threshold,
        )?;
        cpi_permanent_lock(damm_pool, position, nft, &a[0], ev, damm, seeds, liquidity_delta)?;

        let after_a = token_amount(tok_a)?;
        let after_b = token_amount(tok_b)?;
        require!(after_a <= before_a && after_b <= before_b, FusedError::Math);
        let spent_a = before_a.checked_sub(after_a).ok_or(FusedError::Math)?;
        let spent_b = before_b.checked_sub(after_b).ok_or(FusedError::Math)?;
        require!(spent_a <= token_a_amount_threshold, FusedError::BadAutoLpAmounts);
        require!(spent_b <= token_b_amount_threshold, FusedError::BadAutoLpAmounts);
        require!(spent_a <= max_spend_a, FusedError::VaultShort);
        require!(spent_b <= max_spend_b, FusedError::VaultShort);

        // Portion of spend that comes from burn budgets (surplus is consumed first so we
        // never decrement burn for tokens that were never booked).
        let from_burn_a = spent_a.saturating_sub(surplus_a);
        let from_burn_b = spent_b.saturating_sub(surplus_b);
        require!(from_burn_a <= budget_a, FusedError::VaultShort);
        require!(from_burn_b <= budget_b, FusedError::VaultShort);

        let n = pool_ais.len();
        // Pro-rata decrement of burn_pot by mint; holder_pot untouched.
        // Pass 1: floor(from_burn * budget_i / budget_tot). Pass 2: give leftover
        // lamports one-by-one to pools that still have budget room (handles rounding).
        let mut take_amt: Vec<u64> = vec![0u64; n];
        for (mint_is_a, from_burn, budget_tot) in [
            (true, from_burn_a, budget_a),
            (false, from_burn_b, budget_b),
        ] {
            if from_burn == 0 || budget_tot == 0 {
                continue;
            }
            let mut assigned: u64 = 0;
            let mut idxs: Vec<usize> = Vec::new();
            for i in 0..n {
                let is_a = pool_quotes[i] == auto_lp.token_a_mint;
                if is_a != mint_is_a {
                    continue;
                }
                idxs.push(i);
                let t = ((from_burn as u128)
                    .checked_mul(pool_budgets[i] as u128)
                    .ok_or(FusedError::Math)?
                    / (budget_tot as u128)) as u64;
                let t = core::cmp::min(t, pool_budgets[i]);
                take_amt[i] = t;
                assigned = assigned.checked_add(t).ok_or(FusedError::Math)?;
            }
            let mut left = from_burn.checked_sub(assigned).ok_or(FusedError::Math)?;
            for &i in idxs.iter() {
                if left == 0 {
                    break;
                }
                let room = pool_budgets[i].checked_sub(take_amt[i]).ok_or(FusedError::Math)?;
                if room == 0 {
                    continue;
                }
                let add = core::cmp::min(room, left);
                take_amt[i] = take_amt[i].checked_add(add).ok_or(FusedError::Math)?;
                left = left.checked_sub(add).ok_or(FusedError::Math)?;
            }
            require!(left == 0, FusedError::Math);
        }
        let mut taken_a: u64 = 0;
        let mut taken_b: u64 = 0;
        for i in 0..n {
            let take = take_amt[i];
            if pool_quotes[i] == auto_lp.token_a_mint {
                taken_a = taken_a.checked_add(take).ok_or(FusedError::Math)?;
            } else {
                taken_b = taken_b.checked_add(take).ok_or(FusedError::Math)?;
            }
            if take == 0 {
                continue;
            }
            let mut pr: Account<PoolRecord> = Account::try_from(pool_ais[i])?;
            require_keys_eq!(pr.pool, pool_keys[i], FusedError::BadAutoLpAccounts);
            let holder_before = pr.holder_pot;
            pr.burn_pot = pr.burn_pot.checked_sub(take).ok_or(FusedError::Math)?;
            require!(pr.holder_pot == holder_before, FusedError::Math);
            pr.exit(ctx.program_id)?;
        }
        require!(taken_a == from_burn_a, FusedError::Math);
        require!(taken_b == from_burn_b, FusedError::Math);

        // Solvency: vault must still cover holder + remaining burn + creator owed.
        let reserved_a = holder_a
            .checked_add(burn_a.checked_sub(taken_a).ok_or(FusedError::Math)?)
            .ok_or(FusedError::Math)?
            .checked_add(owed_a)
            .ok_or(FusedError::Math)?;
        let reserved_b = holder_b
            .checked_add(burn_b.checked_sub(taken_b).ok_or(FusedError::Math)?)
            .ok_or(FusedError::Math)?
            .checked_add(owed_b)
            .ok_or(FusedError::Math)?;
        require!(after_a >= reserved_a, FusedError::VaultShort);
        require!(after_b >= reserved_b, FusedError::VaultShort);

        msg!(
            "auto_lp_added={} spent_a={} spent_b={} from_burn_a={} from_burn_b={} reserved_a={} reserved_b={} lp_share_bps={}",
            liquidity_delta, spent_a, spent_b, from_burn_a, from_burn_b, reserved_a, reserved_b, lp_share_bps
        );
        Ok(())
    }
}

/// FIX: read (config, base_mint, is_migrated) from a Meteora DBC VirtualPool account.
fn read_dbc_pool(ai: &AccountInfo) -> Result<(Pubkey, Pubkey, u8)> {
    require_keys_eq!(*ai.owner, DBC_PROGRAM_ID, FusedError::NotDbcAccount);
    let d = ai.try_borrow_data()?;
    require!(d.len() > VP_IS_MIGRATED_OFFSET, FusedError::NotDbcAccount);
    require!(d[..8] == DBC_VIRTUAL_POOL_DISC, FusedError::NotDbcAccount);
    let config = Pubkey::try_from(&d[VP_CONFIG_OFFSET..VP_CONFIG_OFFSET + 32]).unwrap();
    let base_mint = Pubkey::try_from(&d[VP_BASE_MINT_OFFSET..VP_BASE_MINT_OFFSET + 32]).unwrap();
    Ok((config, base_mint, d[VP_IS_MIGRATED_OFFSET]))
}


/// V4: creator of a Meteora DBC VirtualPool.
fn read_dbc_creator(ai: &AccountInfo) -> Result<Pubkey> {
    require_keys_eq!(*ai.owner, DBC_PROGRAM_ID, FusedError::NotDbcAccount);
    let d = ai.try_borrow_data()?;
    require!(d.len() > VP_IS_MIGRATED_OFFSET, FusedError::NotDbcAccount);
    require!(d[..8] == DBC_VIRTUAL_POOL_DISC, FusedError::NotDbcAccount);
    Ok(Pubkey::try_from(&d[VP_CREATOR_OFFSET..VP_CREATOR_OFFSET + 32]).unwrap())
}

/// V4: does the DBC config give the pool creator any DAMM liquidity (locked, unlocked or vested)?
/// If so the creator earns its own share straight from DAMM and Global must not pay it again.
fn creator_has_lp(ai: &AccountInfo) -> Result<bool> {
    require_keys_eq!(*ai.owner, DBC_PROGRAM_ID, FusedError::NotDbcAccount);
    let d = ai.try_borrow_data()?;
    require!(d.len() >= PC_MIN_LEN, FusedError::NotDbcAccount);
    require!(d[..8] == DBC_POOL_CONFIG_DISC, FusedError::NotDbcAccount);
    Ok(d[PC_CREATOR_PERM_LOCKED_OFFSET] != 0
        || d[PC_CREATOR_LP_OFFSET] != 0
        || (d[PC_CREATOR_VEST_INIT_OFFSET] != 0 && d[PC_CREATOR_VEST_PCT_OFFSET] != 0))
}

/// V4: PDA holding what the pool's creator is owed. Layout: 8-byte tag, pool 32, owed u64, paid u64, bump u8.
const OWED_TAG: [u8; 8] = [83, 80, 110, 53, 75, 132, 102, 33];
const OWED_LEN: usize = 8 + 32 + 8 + 8 + 1;
fn creator_owed_key(pool: &Pubkey, program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"cown", pool.as_ref()], program_id).0
}
/// (owed, paid) of an existing CreatorOwed account that belongs to `pool`.
fn read_owed(ai: &AccountInfo, pool: &Pubkey) -> Result<(u64, u64)> {
    let d = ai.try_borrow_data()?;
    require!(d.len() == OWED_LEN && d[..8] == OWED_TAG && d[8..40] == pool.to_bytes(), FusedError::BadCreator);
    Ok((u64::from_le_bytes(d[40..48].try_into().unwrap()), u64::from_le_bytes(d[48..56].try_into().unwrap())))
}
fn write_owed(ai: &AccountInfo, pool: &Pubkey, owed: u64, paid: u64) -> Result<()> {
    let mut d = ai.try_borrow_mut_data()?;
    require!(d.len() == OWED_LEN && d[..8] == OWED_TAG && d[8..40] == pool.to_bytes(), FusedError::BadCreator);
    d[40..48].copy_from_slice(&owed.to_le_bytes());
    d[48..56].copy_from_slice(&paid.to_le_bytes());
    Ok(())
}
/// Add `amount` to the pool's CreatorOwed PDA, creating it (rent paid by `payer`) the first time.
fn accrue_creator_share<'info>(
    payer: &AccountInfo<'info>,
    sys: &AccountInfo<'info>,
    ai: &AccountInfo<'info>,
    pool: &Pubkey,
    program_id: &Pubkey,
    amount: u64,
) -> Result<()> {
    let (key, bump) = Pubkey::find_program_address(&[b"cown", pool.as_ref()], program_id);
    require_keys_eq!(ai.key(), key, FusedError::BadCreator);
    if ai.data_is_empty() {
        let lamports = Rent::get()?.minimum_balance(OWED_LEN);
        let seeds: &[&[u8]] = &[b"cown", pool.as_ref(), &[bump]];
        create_pda_account(payer, ai, sys, lamports, OWED_LEN as u64, program_id, seeds)?;
        let mut d = ai.try_borrow_mut_data()?;
        d[..8].copy_from_slice(&OWED_TAG);
        d[8..40].copy_from_slice(pool.as_ref());
        d[40..48].copy_from_slice(&amount.to_le_bytes());
        d[48..56].fill(0);
        d[56] = bump;
    } else {
        require_keys_eq!(*ai.owner, *program_id, FusedError::BadCreator);
        let (owed, paid) = read_owed(ai, pool)?;
        write_owed(ai, pool, owed.checked_add(amount).ok_or(FusedError::Math)?, paid)?;
    }
    Ok(())
}

#[derive(Accounts)]
pub struct ClaimCreatorShare {}
/// FIX: a DBC PoolConfig must be for this quote mint and pay partner fees to the Global PDA.
fn check_dbc_config(ai: &AccountInfo, quote_mint: &Pubkey, global: &Pubkey) -> Result<()> {
    require_keys_eq!(*ai.owner, DBC_PROGRAM_ID, FusedError::NotDbcAccount);
    let d = ai.try_borrow_data()?;
    require!(d.len() >= PC_FEE_CLAIMER_OFFSET + 32, FusedError::NotDbcAccount);
    require!(d[..8] == DBC_POOL_CONFIG_DISC, FusedError::NotDbcAccount);
    let q = Pubkey::try_from(&d[PC_QUOTE_MINT_OFFSET..PC_QUOTE_MINT_OFFSET + 32]).unwrap();
    let fc = Pubkey::try_from(&d[PC_FEE_CLAIMER_OFFSET..PC_FEE_CLAIMER_OFFSET + 32]).unwrap();
    require_keys_eq!(q, *quote_mint, FusedError::QuoteMismatch);
    require_keys_eq!(fc, *global, FusedError::BadFeeClaimer);
    Ok(())
}

/// V3: SPL Token / Token-2022 TransferChecked (instruction 12: amount u64 LE, decimals u8), signed by Global.
/// Hand-built so the program does not link the spl-token-2022 crate.
#[allow(clippy::too_many_arguments)]
fn transfer_checked_signed<'info>(
    token_program: &AccountInfo<'info>,
    from: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    decimals: u8,
    seeds: &[&[u8]],
) -> Result<()> {
    require!(is_token_program(token_program.key), FusedError::WrongTokenProgram);
    let mut data = Vec::with_capacity(10);
    data.push(12u8);
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);
    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::instruction::Instruction {
            program_id: *token_program.key,
            accounts: vec![
                AccountMeta::new(*from.key, false),
                AccountMeta::new_readonly(*mint.key, false),
                AccountMeta::new(*to.key, false),
                AccountMeta::new_readonly(*authority.key, true),
            ],
            data,
        },
        &[from.clone(), mint.clone(), to.clone(), authority.clone(), token_program.clone()],
        &[seeds],
    )?;
    Ok(())
}

/// V3
fn is_token_program(k: &Pubkey) -> bool {
    *k == anchor_spl::token::ID || *k == TOKEN_2022_ID
}

/// V3: result of mint_policy
pub struct MintInfo {
    pub tier: u8,
    pub flags: u32,
    pub decimals: u8,
}

fn nonzero(b: &[u8]) -> bool {
    b.iter().any(|x| *x != 0)
}

/// V3: the pair policy (PLAN §3b). Same code at register, claim, pay, release and check_quote_mint.
/// Hand-written Token-2022 TLV walker: fail-closed on any extension type it does not know.
fn mint_policy(ai: &AccountInfo) -> Result<MintInfo> {
    let prog = *ai.owner;
    require!(is_token_program(&prog), FusedError::WrongTokenProgram);
    let d = ai.try_borrow_data()?;
    require!(d.len() >= 82 && d[45] == 1, FusedError::BadMint);
    let decimals = d[44];
    require!(decimals <= 18, FusedError::UnsupportedMint);
    let mut flags = 0u32;
    if d[46..50] != [0u8; 4] {
        flags |= F_FREEZE_AUTHORITY;
    }
    if prog == TOKEN_2022_ID {
        flags |= F_TOKEN_2022;
        require!(ai.key() != T22_NATIVE_MINT, FusedError::UnsupportedMint);
        if d.len() > 82 {
            require!(d.len() >= 166 && d[165] == 1, FusedError::BadMint);
            let mut off = 166usize;
            while off + 4 <= d.len() {
                let t = u16::from_le_bytes([d[off], d[off + 1]]);
                let l = u16::from_le_bytes([d[off + 2], d[off + 3]]) as usize;
                let v0 = off + 4;
                let v1 = v0.checked_add(l).ok_or(FusedError::BadMint)?;
                require!(v1 <= d.len(), FusedError::BadMint);
                let v = &d[v0..v1];
                match t {
                    0 => break, // Uninitialized: end of the used TLV area
                    // MetadataPointer, TokenMetadata, GroupPointer, TokenGroup, GroupMemberPointer, TokenGroupMember
                    18..=23 => {}
                    3 => flags |= F_CLOSE_AUTHORITY,                  // MintCloseAuthority
                    10 | 25 => flags |= F_SCALED_OR_INTEREST,         // InterestBearingConfig, ScaledUiAmount
                    4 => flags |= F_CONFIDENTIAL,                     // ConfidentialTransferMint
                    16 => {}                                          // ConfidentialTransferFeeConfig (fee lives in TransferFeeConfig)
                    12 => flags |= F_PERMANENT_DELEGATE,              // PermanentDelegate
                    14 => {
                        // TransferHook { authority, program_id }
                        require!(l >= 64, FusedError::BadMint);
                        require!(!nonzero(&v[32..64]), FusedError::HookActive);
                        if nonzero(&v[0..32]) {
                            flags |= F_HOOK_AUTHORITY;
                        }
                    }
                    26 => {
                        // Pausable { authority, paused }
                        require!(l >= 33, FusedError::BadMint);
                        require!(v[32] == 0, FusedError::MintPaused);
                        flags |= F_PAUSABLE;
                    }
                    6 => {
                        // DefaultAccountState { state }: 2 = Frozen
                        require!(l >= 1, FusedError::BadMint);
                        require!(v[0] != 2, FusedError::DefaultFrozen);
                        flags |= F_DEFAULT_ACCOUNT_STATE;
                    }
                    1 => {
                        // TransferFeeConfig: older fee bps @88, newer (scheduled) fee bps @106
                        require!(l >= 108, FusedError::BadMint);
                        let older = u16::from_le_bytes([v[88], v[89]]);
                        let newer = u16::from_le_bytes([v[106], v[107]]);
                        require!(older <= MAX_TRANSFER_FEE_BPS && newer <= MAX_TRANSFER_FEE_BPS, FusedError::TransferFeeTooHigh);
                        if older > 0 || newer > 0 || nonzero(&v[0..32]) {
                            flags |= F_TRANSFER_FEE;
                        }
                    }
                    9 | 24 => return err!(FusedError::UnsupportedMint), // NonTransferable, ConfidentialMintBurn
                    _ => return err!(FusedError::UnknownExtension),
                }
                off = v1;
            }
        }
    } else {
        require!(d.len() == 82, FusedError::BadMint);
    }
    let tier = if flags & ISSUER_FLAGS != 0 { 1 } else { 0 };
    Ok(MintInfo { tier, flags, decimals })
}

/// V3: the parts of a classic or Token-2022 token account we use.
pub struct TokInfo {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub state: u8,
    pub memo_required: bool,
}

fn read_token_account(ai: &AccountInfo, prog: &Pubkey) -> Result<TokInfo> {
    require_keys_eq!(*ai.owner, *prog, FusedError::WrongTokenProgram);
    let d = ai.try_borrow_data()?;
    require!(d.len() >= 165, FusedError::BadTokenAccount);
    let state = d[108];
    require!(state == 1 || state == 2, FusedError::BadTokenAccount);
    let mut memo_required = false;
    if d.len() > 165 {
        require!(*prog == TOKEN_2022_ID && d[165] == 2, FusedError::BadTokenAccount);
        let mut off = 166usize;
        while off + 4 <= d.len() {
            let t = u16::from_le_bytes([d[off], d[off + 1]]);
            let l = u16::from_le_bytes([d[off + 2], d[off + 3]]) as usize;
            let v1 = (off + 4).checked_add(l).ok_or(FusedError::BadTokenAccount)?;
            require!(v1 <= d.len(), FusedError::BadTokenAccount);
            if t == 0 {
                break;
            }
            if t == 8 && l >= 1 && d[off + 4] != 0 {
                memo_required = true; // MemoTransfer { require_incoming_transfer_memos }
            }
            off = v1;
        }
    }
    Ok(TokInfo {
        mint: Pubkey::try_from(&d[0..32]).unwrap(),
        owner: Pubkey::try_from(&d[32..64]).unwrap(),
        amount: u64::from_le_bytes(d[64..72].try_into().unwrap()),
        state,
        memo_required,
    })
}

fn token_amount(ai: &AccountInfo) -> Result<u64> {
    let d = ai.try_borrow_data()?;
    require!(d.len() >= 72, FusedError::BadTokenAccount);
    Ok(u64::from_le_bytes(d[64..72].try_into().unwrap()))
}

/// V3: the Global vault for a mint is ATA(Global, mint, mint's token program). Returns its balance.
/// Classic mints: the same address as v2.
fn check_vault(vault: &AccountInfo, global: &Pubkey, mint: &Pubkey, prog: &Pubkey) -> Result<u64> {
    require_keys_eq!(
        vault.key(),
        get_associated_token_address_with_program_id(global, mint, prog),
        FusedError::BadVault
    );
    require!(vault.lamports() > 0 && *vault.owner == *prog, FusedError::VaultMissing);
    let t = read_token_account(vault, prog)?;
    require!(t.mint == *mint && t.owner == *global, FusedError::BadVault);
    Ok(t.amount)
}

/// V3: same cursor rules as crank_pay_page_v2.
fn sync_cursor(c: &mut Account<PayCursor>, pool_key: Pubkey, round: u64, bump: u8) -> Result<()> {
    if c.pool == Pubkey::default() {
        c.pool = pool_key;
        c.bump = bump;
        c.round = round;
        c.last_owner = MAX_OWNER;
        c.paid_base = 0;
        c.paid_amount = 0;
    } else if c.round < round {
        c.round = round;
        c.last_owner = Pubkey::default();
        c.paid_base = 0;
        c.paid_amount = 0;
    }
    require!(c.round == round, FusedError::BadRound);
    Ok(())
}

/// V2: create a PDA-owned account even if someone pre-funded its address (create_account would fail).
fn create_pda_account<'info>(
    payer: &AccountInfo<'info>,
    target: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
    seeds: &[&[u8]],
) -> Result<()> {
    use anchor_lang::system_program as sp;
    let have = target.lamports();
    if have == 0 {
        sp::create_account(
            CpiContext::new_with_signer(
                system_program.clone(),
                sp::CreateAccount { from: payer.clone(), to: target.clone() },
                &[seeds],
            ),
            lamports,
            space,
            owner,
        )
    } else {
        require_keys_eq!(*target.owner, sp::ID, FusedError::BadDest);
        if have < lamports {
            sp::transfer(
                CpiContext::new(
                    system_program.clone(),
                    sp::Transfer { from: payer.clone(), to: target.clone() },
                ),
                lamports - have,
            )?;
        }
        sp::allocate(
            CpiContext::new_with_signer(
                system_program.clone(),
                sp::Allocate { account_to_allocate: target.clone() },
                &[seeds],
            ),
            space,
        )?;
        sp::assign(
            CpiContext::new_with_signer(
                system_program.clone(),
                sp::Assign { account_to_assign: target.clone() },
                &[seeds],
            ),
            owner,
        )
    }
}

fn cpi_claim_trading_fee<'info>(
    ctx: &Context<CrankClaim<'info>>,
    max_base: u64,
    max_quote: u64,
) -> Result<()> {
    let bump = ctx.accounts.global.bump;
    let seeds: &[&[u8]] = &[b"global", &[bump]];

    let mut data = Vec::with_capacity(24);
    data.extend_from_slice(&CLAIM_TRADING_FEE_DISC);
    data.extend_from_slice(&max_base.to_le_bytes());
    data.extend_from_slice(&max_quote.to_le_bytes());

    let account_metas = vec![
        AccountMeta::new_readonly(ctx.accounts.pool_authority.key(), false),
        AccountMeta::new_readonly(ctx.accounts.config.key(), false),
        AccountMeta::new(ctx.accounts.pool.key(), false),
        AccountMeta::new(ctx.accounts.base_dest.key(), false),
        AccountMeta::new(ctx.accounts.quote_vault.key(), false),
        AccountMeta::new(ctx.accounts.base_vault.key(), false),
        AccountMeta::new(ctx.accounts.quote_pool_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.base_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.quote_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.global.key(), true),
        AccountMeta::new_readonly(ctx.accounts.token_base_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_quote_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
        AccountMeta::new_readonly(ctx.accounts.dbc_program.key(), false),
    ];

    let infos = vec![
        ctx.accounts.pool_authority.to_account_info(),
        ctx.accounts.config.to_account_info(),
        ctx.accounts.pool.to_account_info(),
        ctx.accounts.base_dest.to_account_info(),
        ctx.accounts.quote_vault.to_account_info(),
        ctx.accounts.base_vault.to_account_info(),
        ctx.accounts.quote_pool_vault.to_account_info(),
        ctx.accounts.base_mint.to_account_info(),
        ctx.accounts.quote_mint.to_account_info(),
        ctx.accounts.global.to_account_info(),
        ctx.accounts.token_base_program.to_account_info(),
        ctx.accounts.token_quote_program.to_account_info(),
        ctx.accounts.event_authority.to_account_info(),
        ctx.accounts.dbc_program.to_account_info(),
    ];

    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::instruction::Instruction {
            program_id: DBC_PROGRAM_ID,
            accounts: account_metas,
            data,
        },
        &infos,
        &[seeds],
    )
    .map_err(|_| error!(FusedError::ClaimFailed))?;
    Ok(())
}

fn cpi_claim_position_fee<'info>(ctx: &Context<CrankClaimDamm<'info>>) -> Result<()> {
    let bump = ctx.accounts.global.bump;
    let seeds: &[&[u8]] = &[b"global", &[bump]];

    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&CLAIM_POSITION_FEE_DISC);

    let account_metas = vec![
        AccountMeta::new_readonly(ctx.accounts.pool_authority.key(), false),
        AccountMeta::new_readonly(ctx.accounts.damm_pool.key(), false),
        AccountMeta::new(ctx.accounts.position.key(), false),
        AccountMeta::new(ctx.accounts.token_a_account.key(), false),
        AccountMeta::new(ctx.accounts.token_b_account.key(), false),
        AccountMeta::new(ctx.accounts.token_a_vault.key(), false),
        AccountMeta::new(ctx.accounts.token_b_vault.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_a_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_b_mint.key(), false),
        AccountMeta::new_readonly(ctx.accounts.position_nft_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.global.key(), true),
        AccountMeta::new_readonly(ctx.accounts.token_a_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_b_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
        AccountMeta::new_readonly(ctx.accounts.damm_program.key(), false),
    ];

    let infos = vec![
        ctx.accounts.pool_authority.to_account_info(),
        ctx.accounts.damm_pool.to_account_info(),
        ctx.accounts.position.to_account_info(),
        ctx.accounts.token_a_account.to_account_info(),
        ctx.accounts.token_b_account.to_account_info(),
        ctx.accounts.token_a_vault.to_account_info(),
        ctx.accounts.token_b_vault.to_account_info(),
        ctx.accounts.token_a_mint.to_account_info(),
        ctx.accounts.token_b_mint.to_account_info(),
        ctx.accounts.position_nft_account.to_account_info(),
        ctx.accounts.global.to_account_info(),
        ctx.accounts.token_a_program.to_account_info(),
        ctx.accounts.token_b_program.to_account_info(),
        ctx.accounts.event_authority.to_account_info(),
        ctx.accounts.damm_program.to_account_info(),
    ];

    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::instruction::Instruction {
            program_id: DAMM_PROGRAM_ID,
            accounts: account_metas,
            data,
        },
        &infos,
        &[seeds],
    )
    .map_err(|_| error!(FusedError::ClaimFailed))?;
    Ok(())
}



/// V5: DAMM v2 add_liquidity, signed by Global.
fn cpi_add_liquidity<'info>(
    pool: &AccountInfo<'info>,
    position: &AccountInfo<'info>,
    token_a_account: &AccountInfo<'info>,
    token_b_account: &AccountInfo<'info>,
    token_a_vault: &AccountInfo<'info>,
    token_b_vault: &AccountInfo<'info>,
    token_a_mint: &AccountInfo<'info>,
    token_b_mint: &AccountInfo<'info>,
    position_nft_account: &AccountInfo<'info>,
    global: &AccountInfo<'info>,
    token_a_program: &AccountInfo<'info>,
    token_b_program: &AccountInfo<'info>,
    event_authority: &AccountInfo<'info>,
    damm_program: &AccountInfo<'info>,
    seeds: &[&[u8]],
    liquidity_delta: u128,
    token_a_amount_threshold: u64,
    token_b_amount_threshold: u64,
) -> Result<()> {
    let mut data = Vec::with_capacity(8 + 16 + 8 + 8);
    data.extend_from_slice(&ADD_LIQUIDITY_DISC);
    data.extend_from_slice(&liquidity_delta.to_le_bytes());
    data.extend_from_slice(&token_a_amount_threshold.to_le_bytes());
    data.extend_from_slice(&token_b_amount_threshold.to_le_bytes());

    let account_metas = vec![
        AccountMeta::new(pool.key(), false),
        AccountMeta::new(position.key(), false),
        AccountMeta::new(token_a_account.key(), false),
        AccountMeta::new(token_b_account.key(), false),
        AccountMeta::new(token_a_vault.key(), false),
        AccountMeta::new(token_b_vault.key(), false),
        AccountMeta::new_readonly(token_a_mint.key(), false),
        AccountMeta::new_readonly(token_b_mint.key(), false),
        AccountMeta::new_readonly(position_nft_account.key(), false),
        AccountMeta::new_readonly(global.key(), true),
        AccountMeta::new_readonly(token_a_program.key(), false),
        AccountMeta::new_readonly(token_b_program.key(), false),
        AccountMeta::new_readonly(event_authority.key(), false),
        AccountMeta::new_readonly(damm_program.key(), false),
    ];
    let infos = vec![
        pool.clone(),
        position.clone(),
        token_a_account.clone(),
        token_b_account.clone(),
        token_a_vault.clone(),
        token_b_vault.clone(),
        token_a_mint.clone(),
        token_b_mint.clone(),
        position_nft_account.clone(),
        global.clone(),
        token_a_program.clone(),
        token_b_program.clone(),
        event_authority.clone(),
        damm_program.clone(),
    ];
    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::instruction::Instruction {
            program_id: DAMM_PROGRAM_ID,
            accounts: account_metas,
            data,
        },
        &infos,
        &[seeds],
    )
    .map_err(|_| error!(FusedError::AutoLpCpiFailed))?;
    Ok(())
}

/// V5: DAMM v2 permanent_lock_position, signed by Global.
fn cpi_permanent_lock<'info>(
    pool: &AccountInfo<'info>,
    position: &AccountInfo<'info>,
    position_nft_account: &AccountInfo<'info>,
    global: &AccountInfo<'info>,
    event_authority: &AccountInfo<'info>,
    damm_program: &AccountInfo<'info>,
    seeds: &[&[u8]],
    permanent_lock_liquidity: u128,
) -> Result<()> {
    let mut data = Vec::with_capacity(8 + 16);
    data.extend_from_slice(&PERMANENT_LOCK_DISC);
    data.extend_from_slice(&permanent_lock_liquidity.to_le_bytes());

    let account_metas = vec![
        AccountMeta::new(pool.key(), false),
        AccountMeta::new(position.key(), false),
        AccountMeta::new_readonly(position_nft_account.key(), false),
        AccountMeta::new_readonly(global.key(), true),
        AccountMeta::new_readonly(event_authority.key(), false),
        AccountMeta::new_readonly(damm_program.key(), false),
    ];
    let infos = vec![
        pool.clone(),
        position.clone(),
        position_nft_account.clone(),
        global.clone(),
        event_authority.clone(),
        damm_program.clone(),
    ];
    anchor_lang::solana_program::program::invoke_signed(
        &anchor_lang::solana_program::instruction::Instruction {
            program_id: DAMM_PROGRAM_ID,
            accounts: account_metas,
            data,
        },
        &infos,
        &[seeds],
    )
    .map_err(|_| error!(FusedError::AutoLpCpiFailed))?;
    Ok(())
}

#[account]
pub struct Global {
    pub admin: Pubkey,
    pub burn_mint: Pubkey,
    pub burn_mint_set: bool,
    pub paused: bool,
    pub bump: u8,
}

/// V5: one PDA, seeds [b"autolp"]. Holds the $FUSED DAMM target and the burn/LP split.
/// damm_pool == default means "not ready" (pre-graduation); crank must keep burning 100%.
#[account]
pub struct AutoLpConfig {
    pub damm_pool: Pubkey,
    pub position: Pubkey,
    pub position_nft_account: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub lp_share_bps: u16,
    pub bump: u8,
}
impl AutoLpConfig {
    pub const LEN: usize = 32 + 32 + 32 + 32 + 32 + 2 + 1;
}

/// PERFECT: one per quote mint, seeds [b"qstat", quote_mint]. pool_count is the number of
/// PoolRecords registered for that quote. crank_auto_lp requires the crank to pass exactly
/// that many PoolRecords so reserved pots cannot be understated (M3).
#[account]
pub struct QuoteStat {
    pub quote_mint: Pubkey,
    pub pool_count: u64,
    pub bump: u8,
}
impl QuoteStat {
    pub const LEN: usize = 32 + 8 + 1;
}

/// V2: one per pool, seeds [b"paycur", pool]. Tracks who was paid in the current round.
#[account]
pub struct PayCursor {
    pub pool: Pubkey,
    pub round: u64,
    pub last_owner: Pubkey,
    pub paid_base: u64,
    pub paid_amount: u64,
    pub bump: u8,
}
impl PayCursor {
    pub const LEN: usize = 32 + 8 + 32 + 8 + 8 + 1;
}

/// V2: program-owned lamport holder, seeds [b"sol_vault"]. Only ever holds its own rent between instructions.
#[account]
pub struct SolVault {}

/// V2: argument for close_old_checkpoints.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct LegacyCheckpoint {
    pub round: u64,
    pub owner: Pubkey,
    pub bump: u8,
}

#[account]
pub struct ConfigRecord {
    pub quote_mint: Pubkey,
    pub config: Pubkey,
    pub bump: u8,
}

#[account]
pub struct PoolRecord {
    pub pool: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub config: Pubkey,
    pub graduated: bool,
    pub round: u64,
    pub holder_pot: u64,
    pub burn_pot: u64,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 32 + 1 + 32 + 1 + 1,
        seeds = [b"global"],
        bump
    )]
    pub global: Account<'info, Global>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(quote_mint: Pubkey)]
pub struct RegisterConfig<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    /// CHECK: Meteora config account. Must already exist with this PDA as fee claimer.
    pub config: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 1,
        seeds = [b"config", quote_mint.as_ref()],
        bump
    )]
    pub config_record: Account<'info, ConfigRecord>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut, seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"config", config_record.quote_mint.as_ref()], bump = config_record.bump)]
    pub config_record: Account<'info, ConfigRecord>,
}

#[derive(Accounts)]
#[instruction(pool: Pubkey, base_mint: Pubkey, quote_mint: Pubkey)]
pub struct RegisterPool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [b"config", config_record.quote_mint.as_ref()], bump = config_record.bump)]
    pub config_record: Account<'info, ConfigRecord>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 32 + 1 + 8 + 8 + 8 + 1,
        seeds = [b"pool", pool.as_ref()],
        bump
    )]
    pub pool_record: Account<'info, PoolRecord>,
    pub system_program: Program<'info, System>,
    // FIX: new accounts, appended so the first four keep their positions.
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    /// CHECK: Meteora DBC VirtualPool; owner, discriminator and fields checked in read_dbc_pool
    pub dbc_pool: UncheckedAccount<'info>,
    /// CHECK: Meteora DBC PoolConfig; checked in check_dbc_config
    pub dbc_config: UncheckedAccount<'info>,
    // PERFECT: appended — existing clients that omit it fail at Anchor account count; crank/launch updated together.
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + QuoteStat::LEN,
        seeds = [b"qstat", quote_mint.as_ref()],
        bump
    )]
    pub quote_stat: Account<'info, QuoteStat>,
}

#[derive(Accounts)]
pub struct SetPoolConfig<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    /// CHECK: must be the DBC pool this record points at; contents checked in read_dbc_pool
    #[account(address = pool_record.pool)]
    pub dbc_pool: UncheckedAccount<'info>,
    /// CHECK: must equal the DBC pool's config; checked in check_dbc_config
    pub new_config: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct AdminPool<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
}

#[derive(Accounts)]
pub struct Admin<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
}

#[derive(Accounts)]
pub struct MarkGraduated<'info> {
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    /// CHECK: FIX: the DBC pool; contents checked in read_dbc_pool
    #[account(address = pool_record.pool)]
    pub dbc_pool: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CrankClaim<'info> {
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    /// CHECK: fixed Meteora pool authority
    #[account(address = DBC_POOL_AUTHORITY)]
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: config registered for this pool
    #[account(address = pool_record.config)]
    pub config: UncheckedAccount<'info>,
    /// CHECK: the DBC pool
    #[account(mut, address = pool_record.pool)]
    pub pool: UncheckedAccount<'info>,
    // PHASE0 FIX: fees must land in Global's own accounts, never a caller-chosen account.
    /// CHECK: V3: a Global-owned token account of the base mint (either token program); checked in crank_claim
    #[account(mut)]
    pub base_dest: UncheckedAccount<'info>,
    /// CHECK: V3: must be ATA(Global, quote mint, quote mint's token program); checked in check_vault
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: DBC base vault
    #[account(mut)]
    pub base_vault: UncheckedAccount<'info>,
    /// CHECK: DBC quote vault
    #[account(mut)]
    pub quote_pool_vault: UncheckedAccount<'info>,
    /// CHECK: V3: must be this pool's base mint
    #[account(address = pool_record.base_mint @ FusedError::BadMint)]
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: V3: must be this pool's quote mint; read by mint_policy
    #[account(address = pool_record.quote_mint @ FusedError::BadMint)]
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: V3: must equal the base mint's owner program
    pub token_base_program: UncheckedAccount<'info>,
    /// CHECK: V3: must equal the quote mint's owner program
    pub token_quote_program: UncheckedAccount<'info>,
    /// CHECK: DBC event authority
    pub event_authority: UncheckedAccount<'info>,
    /// CHECK: DBC program
    #[account(address = DBC_PROGRAM_ID)]
    pub dbc_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CrankClaimDamm<'info> {
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    /// CHECK: V3: must be ATA(Global, quote mint, quote mint's token program); checked in check_vault
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: DAMM pool authority
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: DAMM pool
    pub damm_pool: UncheckedAccount<'info>,
    /// CHECK: partner position
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    /// CHECK: PHASE0 FIX: fee A must go to Global's base ATA (V3: with the base mint's token program; checked in handler)
    #[account(mut)]
    pub token_a_account: UncheckedAccount<'info>,
    /// CHECK: PHASE0 FIX: fee B must go to Global's quote vault
    #[account(mut, address = quote_vault.key() @ FusedError::BadVault)]
    pub token_b_account: UncheckedAccount<'info>,
    /// CHECK: pool token A vault
    #[account(mut)]
    pub token_a_vault: UncheckedAccount<'info>,
    /// CHECK: pool token B vault
    #[account(mut)]
    pub token_b_vault: UncheckedAccount<'info>,
    /// CHECK: PHASE0 FIX: DAMM pool must be this launch's base/quote pair
    #[account(address = pool_record.base_mint @ FusedError::BadMint)]
    pub token_a_mint: UncheckedAccount<'info>,
    /// CHECK: PHASE0 FIX
    #[account(address = pool_record.quote_mint @ FusedError::BadMint)]
    pub token_b_mint: UncheckedAccount<'info>,
    /// CHECK: position NFT account owned by the Global PDA
    pub position_nft_account: UncheckedAccount<'info>,
    /// CHECK: token A program
    pub token_a_program: UncheckedAccount<'info>,
    /// CHECK: token B program
    pub token_b_program: UncheckedAccount<'info>,
    /// CHECK: DAMM event authority
    pub event_authority: UncheckedAccount<'info>,
    #[account(address = DAMM_PROGRAM_ID)]
    pub damm_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

/// V2: the retired crank_pay_page takes no accounts it uses.
#[derive(Accounts)]
pub struct CrankPayRetired<'info> {
    pub payer: Signer<'info>,
}

/// V2 pay page. The first six accounts keep the old crank_pay_page order.
#[derive(Accounts)]
pub struct CrankPayV2<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    #[account(mut, constraint = quote_vault.mint == pool_record.quote_mint,
        address = get_associated_token_address(&global.key(), &pool_record.quote_mint) @ FusedError::BadVault)]
    pub quote_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + PayCursor::LEN,
        seeds = [b"paycur", pool_record.pool.as_ref()],
        bump
    )]
    pub pay_cursor: Account<'info, PayCursor>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8,
        seeds = [b"sol_vault"],
        bump
    )]
    pub sol_vault: Account<'info, SolVault>,
    /// CHECK: temp wSOL account, created and closed inside one instruction
    #[account(mut, seeds = [b"unwrap", pool_record.pool.as_ref()], bump)]
    pub unwrap_account: UncheckedAccount<'info>,
    /// CHECK: wrapped SOL mint
    #[account(address = NATIVE_MINT)]
    pub native_mint: UncheckedAccount<'info>,
}

/// V3 pay page (any token program). Shares the PayCursor PDA with crank_pay_page_v2.
#[derive(Accounts)]
pub struct CrankPayV3<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    /// CHECK: ATA(Global, quote mint, quote token program); checked in check_vault
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: this pool's quote mint; read by mint_policy
    #[account(address = pool_record.quote_mint @ FusedError::BadMint)]
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: must equal the quote mint's owner program (checked in handler)
    pub quote_token_program: UncheckedAccount<'info>,
    /// CHECK: this pool's base mint (its owner program decides which holder accounts are valid)
    #[account(address = pool_record.base_mint @ FusedError::BadMint)]
    pub base_mint: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + PayCursor::LEN,
        seeds = [b"paycur", pool_record.pool.as_ref()],
        bump
    )]
    pub pay_cursor: Account<'info, PayCursor>,
}

/// V3
#[derive(Accounts)]
pub struct ReleaseBurnPotV2<'info> {
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    /// CHECK: ATA(Global, quote mint, quote token program); checked in check_vault
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: this pool's quote mint; read by mint_policy
    #[account(address = pool_record.quote_mint @ FusedError::BadMint)]
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: the admin's token account for the quote mint; checked in handler
    #[account(mut)]
    pub operator_token: UncheckedAccount<'info>,
    /// CHECK: must equal the quote mint's owner program (checked in handler)
    pub token_program: UncheckedAccount<'info>,
}

/// V3
#[derive(Accounts)]
pub struct CheckQuoteMint<'info> {
    /// CHECK: any account; mint_policy decides
    pub mint: UncheckedAccount<'info>,
}

/// V2
#[derive(Accounts)]
pub struct CloseOldCheckpoints<'info> {
    pub signer: Signer<'info>,
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    /// CHECK: rent goes back to the crank wallet only
    #[account(mut, address = CRANK_AUTHORITY @ FusedError::BadDest)]
    pub rent_receiver: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CrankBurnDirect<'info> {
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    #[account(mut, address = global.burn_mint)]
    pub burn_mint: Account<'info, anchor_spl::token::Mint>,
    #[account(mut, constraint = quote_vault.mint == pool_record.quote_mint,
        address = get_associated_token_address(&global.key(), &pool_record.quote_mint) @ FusedError::BadVault)]
    pub quote_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum FusedError {
    #[msg("mint is unset or default")]
    BadMint,
    #[msg("pool is unset")]
    BadPool,
    #[msg("quote mint does not match the config")]
    QuoteMismatch,
    #[msg("burn mint is already set")]
    BurnMintLocked,
    #[msg("crank is paused")]
    Paused,
    #[msg("pool has graduated, use the DAMM path")]
    UseDammPath,
    #[msg("math overflow")]
    Math,
    #[msg("nothing was claimed")]
    NothingToClaim,
    #[msg("eligible total must be greater than zero")]
    BadTotal,
    #[msg("holder page must be pairs of accounts")]
    BadPage,
    #[msg("pot is empty")]
    EmptyPot,
    #[msg("base and quote owner differ")]
    OwnerMismatch,
    #[msg("zero balance")]
    ZeroBalance,
    #[msg("burn mint is not set")]
    BurnMintUnset,
    #[msg("quote is not the burn mint, leave it for the swap")]
    NeedsSwap,
    #[msg("meteora claim failed")]
    ClaimFailed,
    #[msg("checkpoint account is not the expected pda")]
    BadCheckpoint,
    #[msg("burn operator is already set")]
    BurnOperatorLocked,
    #[msg("burn operator is not set")]
    BurnOperatorUnset,
    // FIX: new errors appended so existing error numbers do not change.
    #[msg("account is not the expected Meteora DBC account")]
    NotDbcAccount,
    #[msg("config does not match the DBC pool's config")]
    ConfigMismatch,
    #[msg("DBC config fee claimer is not the Global PDA")]
    BadFeeClaimer,
    #[msg("DBC pool has not migrated yet")]
    NotMigrated,
    #[msg("only the admin or the crank may pay holders")]
    NotCrank,
    // PHASE0: appended so existing error numbers do not change (6024).
    #[msg("token account is not the Global vault for this mint")]
    BadVault,
    // V2: appended (6025+).
    #[msg("crank_pay_page is retired, use crank_pay_page_v2")]
    UsePayV2,
    #[msg("round is not valid for this action")]
    BadRound,
    #[msg("destination account is not allowed")]
    BadDest,
    // V3: appended (6028+).
    #[msg("this token is not supported as a pair (non-transferable, confidential mint/burn, Token-2022 wSOL or >18 decimals)")]
    UnsupportedMint,
    #[msg("the token has an active transfer hook: Meteora cannot release its fees; pair suspended")]
    HookActive,
    #[msg("the token is paused by its issuer; pair suspended until unpaused")]
    MintPaused,
    #[msg("the token's new accounts start frozen; not supported")]
    DefaultFrozen,
    #[msg("the token's transfer fee is above the 1% cap")]
    TransferFeeTooHigh,
    #[msg("the token has a Token-2022 extension this program does not know; needs review")]
    UnknownExtension,
    #[msg("the Global vault holds less than this payout (tokens were moved out by the issuer?)")]
    VaultShort,
    #[msg("token program does not match the mint's owner program")]
    WrongTokenProgram,
    #[msg("the Global vault for this token does not exist yet (create the ATA first)")]
    VaultMissing,
    #[msg("only the admin may do this")]
    NotAdmin,
    #[msg("not a valid token account")]
    BadTokenAccount,
    // V4: appended (6039+).
    #[msg("crank_claim_damm needs the 5 creator-share accounts (payer, system program, dbc pool, dbc config, creator_owed)")]
    UseSplitPath,
    #[msg("creator account is not the pool creator's token account, or the pool/config does not match")]
    BadCreator,
    #[msg("no creator share is waiting")]
    NoCreatorShare,
    #[msg("the creator's token account is missing, frozen or not usable; create it and try again")]
    CreatorAtaUnusable,
    // V5 auto-LP: appended (6043+).
    #[msg("auto-LP is not configured yet (set the $FUSED DAMM pool after graduation)")]
    AutoLpNotReady,
    #[msg("auto-LP share must be 0..=10000 basis points")]
    BadAutoLpShare,
    #[msg("auto-LP accounts are missing or do not match the configured pool/position")]
    BadAutoLpAccounts,
    #[msg("auto-LP liquidity delta or thresholds must be greater than zero")]
    BadAutoLpAmounts,
    #[msg("DAMM add_liquidity or permanent_lock_position CPI failed")]
    AutoLpCpiFailed,
}

#[derive(Accounts)]
pub struct SetBurnOperator<'info> {
    #[account(mut, seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
    pub admin: Signer<'info>,
}
#[derive(Accounts)]
pub struct ReleaseBurnPot<'info> {
    #[account(seeds = [b"global"], bump = global.bump)]
    pub global: Account<'info, Global>,
    #[account(mut, seeds = [b"pool", pool_record.pool.as_ref()], bump = pool_record.bump)]
    pub pool_record: Account<'info, PoolRecord>,
    #[account(mut, constraint = quote_vault.mint == pool_record.quote_mint,
        address = get_associated_token_address(&global.key(), &pool_record.quote_mint) @ FusedError::BadVault)]
    pub quote_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = operator_token.mint == pool_record.quote_mint,
        constraint = operator_token.owner == global.admin
    )]
    pub operator_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct InitAutoLp<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
    #[account(
        init,
        payer = admin,
        space = 8 + AutoLpConfig::LEN,
        seeds = [b"autolp"],
        bump
    )]
    pub auto_lp: Account<'info, AutoLpConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetAutoLp<'info> {
    #[account(seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"autolp"], bump = auto_lp.bump)]
    pub auto_lp: Account<'info, AutoLpConfig>,
}

#[derive(Accounts)]
pub struct SetAutoLpShare<'info> {
    #[account(seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"autolp"], bump = auto_lp.bump)]
    pub auto_lp: Account<'info, AutoLpConfig>,
}

#[derive(Accounts)]
#[instruction(quote_mint: Pubkey)]
pub struct SyncQuoteStat<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"global"], bump = global.bump, has_one = admin)]
    pub global: Account<'info, Global>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + QuoteStat::LEN,
        seeds = [b"qstat", quote_mint.as_ref()],
        bump
    )]
    pub quote_stat: Account<'info, QuoteStat>,
    pub system_program: Program<'info, System>,
}

/// V5: accounts passed as remaining_accounts (keeps the binary small, same pattern as claim_creator_share).
#[derive(Accounts)]
pub struct CrankAutoLp {}

#[derive(Accounts)]
pub struct MigrateGlobal<'info> {
    #[account(mut, seeds = [b"global"], bump = global.bump, has_one = admin, realloc = 8 + 32 + 32 + 1 + 32 + 1 + 1, realloc::payer = admin, realloc::zero = false)]
    pub global: Account<'info, Global>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}
