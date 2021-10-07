use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, TokenAccount, Transfer};

use az::CheckedAs;

declare_id!("BHfLU4UsBdxBZk56GjpGAXkzu8B7JdMitGa9A1VTMmva");

#[account]
pub struct Pool {
    pool_authority: Pubkey,
    administrator_authority: Pubkey,
    nonce: u8,

    genesis: i64,
    topup_duration: i64,
    lockup_duration: i64,

    stake_acquired_amount: u64,
    stake_target_amount: u64,
    reward_amount: u64,
    deposited_reward_amount: u64,

    stake_mint: Pubkey,
    stake_vault: Pubkey,
}

impl Pool {
    fn can_topup(&self, now: i64) -> bool {
        now < self.genesis + self.topup_duration
    }

    fn is_expired(&self, now: i64) -> bool {
        now > self.genesis + self.lockup_duration
    }
}

#[account]
pub struct Ticket {
    authority: Pubkey,
    staked_amount: u64,
}

// TODO: not so elegant
fn ticket_collect<'info>(
    ticket: &Account<'info, Ticket>,
    beneficiary: &AccountInfo<'info>,
) -> Result<bool> {
    if ticket.staked_amount == 0 {
        let beneficiary_starting_lamports = beneficiary.lamports();
        **beneficiary.lamports.borrow_mut() = beneficiary_starting_lamports
            .checked_add(ticket.as_ref().lamports())
            .ok_or(ErrorCode::IntegerOverlow)?;
        **ticket.as_ref().lamports.borrow_mut() = 0;

        Ok(true)
    } else {
        Ok(false)
    }
}

#[error]
pub enum ErrorCode {
    #[msg("Given nonce is invalid")]
    InvalidNonce,
    #[msg("Given authority does not match expected one")]
    InvalidAuthority,
    #[msg("Given topup duration lasts longer than lockup duration")]
    TopupLongerThanLockup,
    #[msg("Given wallet has not enough funds")]
    NotEnoughFunds,
    #[msg("Pool is locked and funds can no longer be added")]
    PoolIsLocked,
    #[msg("Pool is full")]
    PoolIsFull,
    #[msg("Pool rewards are full")]
    PoolRewardsAreFull,
    #[msg("Pool is not expired yet")]
    PoolIsNotExpired,
    #[msg("Pool is expired already")]
    PoolIsExpired,
    #[msg("Ticket collection invariant failed")]
    TicketCollectionFailure,
    #[msg("Not enough rewards to collect")]
    NotEnoughRewards,
    #[msg("Invalid amount transferred")]
    InvalidAmountTransferred,
    #[msg("Integer overflow occured")]
    IntegerOverlow,
}

#[program]
pub mod pool {
    use super::*;

    #[access_control(InitializePool::accounts(&ctx, nonce))]
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        nonce: u8,
        topup_duration: i64,
        lockup_duration: i64,
        target_amount: u64,
        reward_amount: u64,
    ) -> Result<()> {
        let now = ctx.accounts.clock.unix_timestamp;

        require!(topup_duration <= lockup_duration, TopupLongerThanLockup);

        let pool = &mut ctx.accounts.pool;

        pool.pool_authority = ctx.accounts.pool_authority.key();
        pool.administrator_authority = ctx.accounts.administrator_authority.key();
        pool.nonce = nonce;

        pool.genesis = now;
        pool.topup_duration = topup_duration;
        pool.lockup_duration = lockup_duration;

        pool.stake_acquired_amount = 0;
        pool.stake_target_amount = target_amount;
        pool.reward_amount = reward_amount;

        pool.stake_mint = ctx.accounts.stake_mint.key();
        pool.stake_vault = ctx.accounts.stake_vault.key();

        // TODO: ts bindings have functionality to filter out account types
        // so we don't have to have some special ids or something

        Ok(())
    }

    pub fn add_stake(ctx: Context<AddStake>, amount: u64) -> Result<()> {
        require!(ctx.accounts.source_wallet.amount >= amount, NotEnoughFunds);

        let now = ctx.accounts.clock.unix_timestamp;

        let pool = &mut ctx.accounts.pool;
        let ticket = &mut ctx.accounts.ticket;
        let stake_vault = &mut ctx.accounts.stake_vault;

        require!(pool.can_topup(now), PoolIsLocked);

        let transfer_amount = std::cmp::min(
            amount,
            pool.stake_target_amount - pool.stake_acquired_amount,
        );

        require!(transfer_amount > 0, PoolIsFull);

        let amount_before = stake_vault.amount;

        let cpi_accounts = Transfer {
            from: ctx.accounts.source_wallet.to_account_info(),
            to: stake_vault.to_account_info(),
            authority: ctx.accounts.source_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, transfer_amount)?;

        stake_vault.reload()?;
        let amount_after = stake_vault.amount;

        require!(
            amount_after - amount_before == transfer_amount,
            InvalidAmountTransferred
        );

        pool.stake_acquired_amount += transfer_amount;
        ticket.staked_amount += transfer_amount;

        Ok(())
    }

    pub fn remove_stake(ctx: Context<RemoveStake>, amount: u64) -> Result<()> {
        let now = ctx.accounts.clock.unix_timestamp;

        let pool = &mut ctx.accounts.pool;
        let ticket = &mut ctx.accounts.ticket;
        let stake_vault = &mut ctx.accounts.stake_vault;

        require!(pool.can_topup(now), PoolIsLocked);

        let transfer_amount = std::cmp::min(amount, ticket.staked_amount);

        let amount_before = stake_vault.amount;

        let cpi_accounts = Transfer {
            from: stake_vault.to_account_info(),
            to: ctx.accounts.target_wallet.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        // TODO: should be prettier
        let pool_key = pool.key();
        let seeds = &[
            pool_key.as_ref(),
            pool.administrator_authority.as_ref(),
            &[pool.nonce],
        ];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, transfer_amount)?;

        stake_vault.reload()?;
        let amount_after = stake_vault.amount;

        require!(
            amount_before - amount_after == transfer_amount,
            InvalidAmountTransferred
        );

        pool.stake_acquired_amount -= transfer_amount;

        ticket.staked_amount -= transfer_amount;
        ticket_collect(ticket, &ctx.accounts.staker)?;

        Ok(())
    }

    pub fn claim_reward(ctx: Context<ClaimReward>) -> Result<()> {
        let now = ctx.accounts.clock.unix_timestamp;

        let pool = &mut ctx.accounts.pool;
        let ticket = &mut ctx.accounts.ticket;
        let stake_vault = &mut ctx.accounts.stake_vault;

        require!(pool.is_expired(now), PoolIsNotExpired);

        use fixed::types::U64F64;

        let staked_amount = U64F64::from_num(ticket.staked_amount);
        let stake_acquired_amount = U64F64::from_num(pool.stake_acquired_amount);
        let reward_amount = U64F64::from_num(pool.reward_amount);

        let share = staked_amount / stake_acquired_amount;
        let reward_share = share * reward_amount;

        let transfer_amount = (staked_amount + reward_share)
            .checked_as::<u64>()
            .ok_or(ErrorCode::IntegerOverlow)?;

        let pool_key = pool.key();
        let seeds = &[
            pool_key.as_ref(),
            pool.administrator_authority.as_ref(),
            &[pool.nonce],
        ];
        let signer = &[&seeds[..]];

        let amount_before = stake_vault.amount;

        let cpi_accounts = Transfer {
            from: stake_vault.to_account_info(),
            to: ctx.accounts.target_wallet.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, transfer_amount)?;

        stake_vault.reload()?;
        let amount_after = stake_vault.amount;

        require!(
            amount_before - amount_after == transfer_amount,
            InvalidAmountTransferred
        );

        ticket.staked_amount = 0;

        let collected = ticket_collect(ticket, &ctx.accounts.staker)?;
        require!(collected, TicketCollectionFailure);

        Ok(())
    }

    pub fn add_reward(ctx: Context<AddReward>, amount: u64) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        let stake_vault = &mut ctx.accounts.stake_vault;

        let transfer_amount = amount
            .min(pool.reward_amount - pool.deposited_reward_amount)
            .min(ctx.accounts.source_wallet.amount);

        require!(transfer_amount > 0, NotEnoughRewards);

        let now = ctx.accounts.clock.unix_timestamp;

        require!(!pool.is_expired(now), PoolIsExpired);

        let amount_before = stake_vault.amount;

        let cpi_accounts = Transfer {
            from: ctx.accounts.source_wallet.to_account_info(),
            to: stake_vault.to_account_info(),
            authority: ctx.accounts.source_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.clone();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, transfer_amount)?;

        stake_vault.reload()?;
        let amount_after = stake_vault.amount;
        require!(
            amount_after - amount_before == transfer_amount,
            InvalidAmountTransferred
        );

        pool.deposited_reward_amount += transfer_amount;
        require!(
            pool.deposited_reward_amount <= pool.reward_amount,
            PoolRewardsAreFull
        );

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(signer)]
    administrator_authority: AccountInfo<'info>,
    pool_authority: AccountInfo<'info>,
    #[account(zero)]
    pool: Account<'info, Pool>,
    #[account(constraint = stake_mint.key() == stake_vault.mint)]
    stake_mint: Account<'info, Mint>,
    stake_vault: Account<'info, TokenAccount>,

    pub clock: Sysvar<'info, Clock>,
}

impl<'info> InitializePool<'info> {
    fn accounts(ctx: &Context<InitializePool<'info>>, nonce: u8) -> Result<()> {
        let expected_authority = Pubkey::create_program_address(
            &[
                ctx.accounts.pool.key().as_ref(),
                ctx.accounts.administrator_authority.key.as_ref(),
                &[nonce],
            ],
            ctx.program_id,
        )
        .map_err(|_| ErrorCode::InvalidNonce)?;

        if ctx.accounts.pool_authority.key != &expected_authority {
            return Err(ErrorCode::InvalidAuthority.into());
        }

        if ctx.accounts.stake_vault.owner != expected_authority {
            return Err(ErrorCode::InvalidAuthority.into());
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct AddStake<'info> {
    #[account(constraint = token_program.key == &token::ID)]
    token_program: AccountInfo<'info>,
    #[account(mut)]
    pool: Account<'info, Pool>,
    #[account(zero)]
    ticket: Account<'info, Ticket>,
    #[account(mut)]
    stake_vault: Account<'info, TokenAccount>,
    #[account(signer)]
    source_authority: AccountInfo<'info>,
    #[account(mut)]
    source_wallet: Account<'info, TokenAccount>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct RemoveStake<'info> {
    #[account(constraint = token_program.key == &token::ID)]
    token_program: AccountInfo<'info>,
    #[account(mut)]
    pool: Account<'info, Pool>,
    #[account(mut, signer)]
    staker: AccountInfo<'info>,
    #[account(mut, constraint = ticket.authority == *staker.key)]
    ticket: Account<'info, Ticket>,
    pool_authority: AccountInfo<'info>,
    #[account(mut)]
    stake_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    target_wallet: Account<'info, TokenAccount>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    #[account(constraint = token_program.key == &token::ID)]
    token_program: AccountInfo<'info>,
    pool: Account<'info, Pool>,
    #[account(mut, signer)]
    staker: AccountInfo<'info>,
    #[account(mut, constraint = ticket.authority == *staker.key)]
    ticket: Account<'info, Ticket>,
    pool_authority: AccountInfo<'info>,
    #[account(mut)]
    stake_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    target_wallet: Account<'info, TokenAccount>,

    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct AddReward<'info> {
    #[account(constraint = token_program.key == &token::ID)]
    token_program: AccountInfo<'info>,
    #[account(mut)]
    pool: Account<'info, Pool>,
    #[account(mut)]
    stake_vault: Account<'info, TokenAccount>,
    #[account(signer)]
    source_authority: AccountInfo<'info>,
    #[account(mut)]
    source_wallet: Account<'info, TokenAccount>,

    pub clock: Sysvar<'info, Clock>,
}
