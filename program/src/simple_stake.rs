use std::mem::size_of;

use chrono::{DateTime, Duration, Utc};
use solana_program::pubkey::Pubkey;
use solar::{
    account::{
        onchain::{Account, AccountRef},
        AccountFields, AccountFieldsMut,
    },
    input::ProgramInput,
    math::Checked,
    prelude::AccountBackend,
    qlog,
    reinterpret::as_bytes,
    spl::{MintAccount, TokenProgram, WalletAccount},
    time::{SolDuration, SolTimestamp},
    util::{datetime_now, is_zeroed, pubkey_eq, ResultExt},
};
use solar_macros::parse_accounts;

use crate::{
    data::{AccountType, Entity, EntityAllocator, EntityKind},
    error::Error,
    impl_entity_simple_deref,
};

#[derive(Debug)]
pub struct StakePool;
#[derive(Debug)]
pub struct StakerTicket;

#[repr(C)]
pub struct StakePoolState {
    pub administrator_authority: Pubkey,
    pub program_authority: Pubkey,
    pub stake_mint: Pubkey,
    pub stake_vault: Pubkey,
    pub program_authority_salt: u64,

    pub stake_target_amount: Checked<u64>,
    pub stake_acquired_amount: Checked<u64>,
    pub reward_amount: Checked<u64>,
    pub deposited_reward_amount: Checked<u64>,

    pub allocator: EntityAllocator,

    pub genesis: SolTimestamp,
    pub lockup_duration: SolDuration,
    pub topup_duration: SolDuration,
}

#[repr(C)]
pub struct StakerTicketState {
    pub authority: Pubkey,
    pub staked_amount: Checked<u64>,
}

impl AccountType for StakePool {
    const KIND: EntityKind = EntityKind::SimpleStakePool;

    fn is_valid_size(size: usize) -> bool {
        size == size_of::<Self>()
    }
}

impl AccountType for StakerTicket {
    const KIND: EntityKind = EntityKind::SimpleStakeTicket;

    fn is_valid_size(size: usize) -> bool {
        size == size_of::<Self>()
    }
}

impl_entity_simple_deref!(StakePool, StakePoolState);
impl_entity_simple_deref!(StakerTicket, StakerTicketState);

#[derive(Debug)]
pub struct InitializeArgsAccounts<B: AccountBackend> {
    pub administrator_authority: B,
    pub program_authority: B,
    pub stake_mint: MintAccount<B>,
    pub stake_vault: WalletAccount<B>,
}

impl InitializeArgsAccounts<AccountRef> {
    pub fn from_program_input(input: &mut ProgramInput) -> Result<Self, Error> {
        parse_accounts! {
            &administrator_authority,
            &program_authority,
            &stake_mint = MintAccount::any(this)?,
            &stake_vault = stake_mint.wallet(this)?
        }

        Ok(Self {
            administrator_authority,
            program_authority,
            stake_mint,
            stake_vault,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct InitializeArgs {
    pub program_authority_salt: u64,
    pub lockup_duration: SolDuration,
    pub topup_duration: SolDuration,
    pub target_amount: Checked<u64>,
    pub reward_amount: Checked<u64>,
}

#[derive(Debug)]
pub struct StakeArgsAccounts<B: AccountBackend> {
    pub token_program: TokenProgram<B>,

    pub pool: Entity<B, StakePool>,
    pub ticket: Entity<B, StakerTicket>,
    pub staker: B,
    pub stake_vault: WalletAccount<B>,
    pub source_authority: B,
    pub source_wallet: WalletAccount<B>,
}

#[derive(Debug)]
pub struct UnStakeArgsAccounts<B: AccountBackend> {
    pub token_program: TokenProgram<B>,

    pub pool: Entity<B, StakePool>,
    pub ticket: Entity<B, StakerTicket>,
    pub staker: B,
    pub program_authority: B,
    pub stake_vault: WalletAccount<B>,
    pub target_wallet: WalletAccount<B>,
}

#[derive(Debug)]
pub struct AddRewardArgsAccounts<B: AccountBackend> {
    pub token_program: TokenProgram<B>,
    pub pool: Entity<B, StakePool>,
    pub stake_vault: WalletAccount<B>,
    pub source_authority: B,
    pub source_wallet: WalletAccount<B>,
}

impl StakeArgsAccounts<AccountRef> {
    pub fn from_program_input(input: &mut ProgramInput) -> Result<Self, Error> {
        let program_id = input.program_id();

        parse_accounts!(
            &token_program = TokenProgram::load(this)?,
            &mut pool = <Entity<AccountRef, StakePool>>::load(program_id, this)?,
            &staker,
            &mut ticket = pool.load_or_init_ticket(&staker, this)?,
            &mut stake_vault = pool.stake_vault(this)?,
            &source_authority,
            &mut source_wallet = pool.stake_wallet(this)?
        );

        Ok(Self {
            token_program,

            pool,
            ticket,
            staker,
            stake_vault,
            source_authority,
            source_wallet,
        })
    }
}

impl UnStakeArgsAccounts<AccountRef> {
    pub fn from_program_input(input: &mut ProgramInput) -> Result<Self, Error> {
        let program_id = input.program_id();

        parse_accounts!(
            &token_program = TokenProgram::load(this)?,
            &mut pool = <Entity<AccountRef, StakePool>>::load(program_id, this)?,
            &mut ticket = pool.load_ticket(this)?,
            &mut staker,
            &program_authority,
            &mut stake_vault = pool.stake_vault(this)?,
            &mut target_wallet = pool.stake_wallet(this)?
        );

        Ok(Self {
            token_program,
            pool,
            ticket,
            staker,
            program_authority,
            stake_vault,
            target_wallet,
        })
    }
}

impl AddRewardArgsAccounts<AccountRef> {
    pub fn from_program_input(input: &mut ProgramInput) -> Result<Self, Error> {
        let program_id = input.program_id();

        parse_accounts!(
            &token_program = TokenProgram::load(this)?,
            &mut pool = <Entity<AccountRef, StakePool>>::load(program_id, this)?,
            &mut stake_vault = pool.stake_vault(this)?,
            &source_authority,
            &mut source_wallet = pool.stake_wallet(this)?,
        );

        Ok(Self {
            token_program,
            pool,
            stake_vault,
            source_authority,
            source_wallet,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub struct StakeArgs {
    pub amount: Checked<u64>,
}

impl<B> Entity<B, StakePool>
where
    B: AccountBackend,
{
    pub fn initialize(
        program_id: &Pubkey,
        account: B,
        accounts: &mut InitializeArgsAccounts<B>,
        args: InitializeArgs,
    ) -> Result<(), Error>
    where
        B::Impl: AccountFieldsMut,
    {
        let mut entity = Self::raw_initialized(program_id, account)?;

        let expected_program_authority = Pubkey::create_program_address(
            &[
                entity.account().key().as_ref(),
                accounts.administrator_authority.key().as_ref(),
                &args.program_authority_salt.to_le_bytes(),
            ],
            program_id,
        )
        .ok()
        .bpf_expect("couldn't derive program authority");

        if !pubkey_eq(
            accounts.program_authority.key(),
            &expected_program_authority,
        ) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority);
        }

        if !pubkey_eq(
            accounts.stake_vault.authority(),
            &expected_program_authority,
        ) {
            qlog!("stake vault authority does not match program authority");
            return Err(Error::InvalidAuthority);
        }

        if !pubkey_eq(accounts.stake_mint.key(), accounts.stake_vault.mint()) {
            qlog!("stake vault mint does not match provided stake mint");
            return Err(Error::InvalidParent);
        }

        let now = datetime_now();
        let topup_duration: Duration = args.topup_duration.into();
        let lockup_duration: Duration = args.lockup_duration.into();

        if topup_duration > lockup_duration {
            qlog!("topup_duration should be less than lockup_duration");
            return Err(Error::InvalidData);
        }

        entity.program_authority = *accounts.program_authority.key();
        entity.administrator_authority = *accounts.administrator_authority.key();
        entity.genesis = now.into();
        entity.topup_duration = topup_duration.into();
        entity.lockup_duration = lockup_duration.into();

        entity.stake_acquired_amount = 0.into();
        entity.stake_target_amount = args.target_amount;
        entity.reward_amount = args.reward_amount;

        entity.stake_mint = *accounts.stake_mint.key();
        entity.stake_vault = *accounts.stake_vault.key();

        let id = entity.allocator.allocate_id();
        let entity_key = *entity.account().key();
        let header = entity.header_mut();
        header.kind = EntityKind::SimpleStakePool;
        header.id = id;
        header.parent_id = id;
        header.root = entity_key;

        Ok(())
    }

    pub fn genesis(&self) -> DateTime<Utc> {
        self.genesis.into()
    }

    pub fn topup_duration(&self) -> Duration {
        self.topup_duration.into()
    }

    pub fn lockup_duration(&self) -> Duration {
        self.lockup_duration.into()
    }

    pub fn can_topup(&self, now: DateTime<Utc>) -> bool {
        now < self.genesis() + self.topup_duration()
    }

    pub fn can_withdraw(&self, now: DateTime<Utc>) -> bool {
        self.can_topup(now) || self.is_expired(now)
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now > self.genesis() + self.lockup_duration()
    }

    pub fn authority_seeds(&self) -> [&[u8]; 3] {
        [
            self.account().key().as_ref(),
            self.administrator_authority.as_ref(),
            as_bytes(&self.program_authority_salt),
        ]
    }

    pub fn stake_mint(&self, account: B) -> Result<MintAccount<B>, Error> {
        let mint = MintAccount::any(account)?;

        if !pubkey_eq(&self.stake_mint, mint.key()) {
            return Err(Error::InvalidMint);
        }

        Ok(mint)
    }

    pub fn stake_wallet(&self, account: B) -> Result<WalletAccount<B>, Error> {
        let wallet = WalletAccount::any(account)?;

        if !pubkey_eq(&self.stake_mint, wallet.mint()) {
            return Err(Error::InvalidMint);
        }

        Ok(wallet)
    }

    pub fn stake_vault(&self, account: B) -> Result<WalletAccount<B>, Error> {
        let wallet = WalletAccount::any(account)?;

        if !pubkey_eq(&self.stake_vault, wallet.key()) {
            return Err(Error::InvalidAccount);
        }

        Ok(wallet)
    }

    fn load_ticket(&self, ticket: B) -> Result<Entity<B, StakerTicket>, Error> {
        let ticket = Entity::<B, StakerTicket>::raw_any(self.account().owner(), ticket)?;
        if ticket.header().kind == EntityKind::SimpleStakeTicket {
            if !ticket.is_child(self) {
                Err(Error::InvalidParent)
            } else {
                Ok(ticket)
            }
        } else {
            Err(Error::InvalidKind)
        }
    }

    fn load_or_init_ticket(
        &mut self,
        authority: &B,
        ticket: B,
    ) -> Result<Entity<B, StakerTicket>, Error>
    where
        B::Impl: AccountFieldsMut,
    {
        let mut ticket = Entity::<B, StakerTicket>::raw_any(self.account().owner(), ticket)?;

        if ticket.header().kind == EntityKind::SimpleStakeTicket {
            if !ticket.is_child(self) {
                Err(Error::InvalidParent)
            } else {
                Ok(ticket)
            }
        } else if ticket.header().kind == EntityKind::None {
            if !is_zeroed(ticket.account().data()) {
                Err(Error::InvalidData)
            } else {
                let header = ticket.header_mut();
                header.id = self.allocator.allocate_id();
                header.parent_id = self.header().id;
                header.root = *self.account().key();
                header.kind = EntityKind::SimpleStakeTicket;

                ticket.authority = *authority.key();

                Ok(ticket)
            }
        } else {
            Err(Error::InvalidKind)
        }
    }

    pub fn add_stake(
        _program_id: &Pubkey,
        accounts: StakeArgsAccounts<B>,
        args: StakeArgs,
    ) -> Result<(), Error>
    where
        B: AccountBackend<Impl = Account>,
    {
        let StakeArgsAccounts {
            token_program,
            mut pool,
            mut ticket,
            mut stake_vault,
            source_authority,
            mut source_wallet,
            ..
        } = accounts;

        if source_wallet.amount() < args.amount {
            qlog!("not enough funds in wallet");
            return Err(Error::Validation);
        }

        let now = datetime_now();

        if !pool.can_topup(now) {
            qlog!("pool is locked and funds can no longer be added");
            return Err(Error::Validation);
        }

        let transfer_amount = args
            .amount
            .min(pool.stake_target_amount - pool.stake_acquired_amount);

        if transfer_amount == 0.into() {
            qlog!("pool is full");
            return Err(Error::Validation);
        }

        let amount_before = stake_vault.amount();
        token_program
            .transfer(
                &mut source_wallet,
                &mut stake_vault,
                transfer_amount.value(),
                &source_authority,
                &[],
            )
            .bpf_expect("call failed")
            .bpf_expect("transfer failed");
        let amount_after = stake_vault.amount();

        assert!(amount_after - amount_before == transfer_amount);

        pool.stake_acquired_amount += transfer_amount;
        ticket.staked_amount += transfer_amount;

        Ok(())
    }

    pub fn remove_stake(
        _program_id: &Pubkey,
        accounts: UnStakeArgsAccounts<B>,
        args: StakeArgs,
    ) -> Result<(), Error>
    where
        B: AccountBackend<Impl = Account>,
    {
        let UnStakeArgsAccounts {
            token_program,
            mut pool,
            mut staker,
            mut ticket,
            program_authority,
            mut stake_vault,
            mut target_wallet,
        } = accounts;

        if !pubkey_eq(&ticket.authority, staker.key()) {
            qlog!("wrong staker provided");
            return Err(Error::Validation);
        }

        if !staker.is_signer() {
            qlog!("the staker is expected to sign");
            return Err(Error::Validation);
        }

        let now = datetime_now();

        if !pool.can_withdraw(now) {
            qlog!("pool is locked and funds can no longer be removed");
            return Err(Error::Validation);
        }

        let transfer_amount = if !pool.is_expired(now) {
            args.amount.min(ticket.staked_amount)
        } else {
            ticket.staked_amount
        };

        let seeds = pool.authority_seeds();
        let amount_before = stake_vault.amount();
        token_program
            .transfer(
                &mut stake_vault,
                &mut target_wallet,
                transfer_amount.value(),
                &program_authority,
                &[&seeds],
            )
            .bpf_expect("call failed")
            .bpf_expect("transfer failed");
        let amount_after = stake_vault.amount();

        assert!(amount_before - amount_after == transfer_amount);

        pool.stake_acquired_amount -= transfer_amount;
        ticket.staked_amount -= transfer_amount;

        if pool.is_expired(now) {
            assert!(ticket.staked_amount == 0.into());
        }

        ticket.collect(&mut staker)?;

        Ok(())
    }

    pub fn add_reward(
        _program_id: &Pubkey,
        accounts: AddRewardArgsAccounts<B>,
        args: StakeArgs,
    ) -> Result<(), Error>
    where
        B: AccountBackend<Impl = Account>,
    {
        let AddRewardArgsAccounts {
            token_program,
            mut pool,
            mut stake_vault,
            source_authority,
            mut source_wallet,
        } = accounts;

        let transfer_amount = args
            .amount
            .min(pool.reward_amount - pool.deposited_reward_amount)
            .min(source_wallet.amount());

        if transfer_amount == 0.into() {
            qlog!("no reward to deposit");
            return Err(Error::Validation);
        }

        let amount_before = stake_vault.amount();
        token_program
            .transfer(
                &mut source_wallet,
                &mut stake_vault,
                transfer_amount.value(),
                &source_authority,
                &[],
            )
            .bpf_expect("call failed")
            .bpf_expect("transfer failed");
        let amount_after = stake_vault.amount();
        assert!(amount_after - amount_before == transfer_amount);

        pool.deposited_reward_amount += transfer_amount;
        assert!(pool.deposited_reward_amount <= pool.reward_amount);

        Ok(())
    }

    pub fn load(program_id: &Pubkey, account: B) -> Result<Self, Error> {
        Self::raw_initialized(program_id, account)
    }
}

impl<B: AccountBackend> Entity<B, StakerTicket> {
    pub fn collect(&mut self, beneficiary: &mut B) -> Result<bool, Error>
    where
        B: AccountFieldsMut,
    {
        if self.staked_amount == 0.into() {
            beneficiary.set_lamports(beneficiary.lamports() + self.account().lamports());
            self.account_mut().set_lamports(0);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
