use std::mem::size_of;

#[cfg(feature = "onchain")]
use az::CheckedAs;
use solana_api_types::Pubkey;
#[cfg(feature = "onchain")]
use solar::{
    account::onchain::Account,
    input::AccountSource,
    math::ToF64,
    qlog,
    util::{is_zeroed, timestamp_now, ResultExt},
};
use solar::{
    account::{AccountFields, AccountFieldsMut},
    math::Checked,
    prelude::AccountBackend,
    reinterpret::as_bytes,
    spl::{MintAccount, TokenProgram, WalletAccount},
    util::pubkey_eq,
};
#[cfg(feature = "onchain")]
use solar_macros::parse_accounts;

use crate::{
    data::{AccountType, Entity, EntityAllocator, EntityKind, HEADER_RESERVED},
    error::Error,
    impl_entity_simple_deref, TokenAmount,
};

pub type StakePoolEntity<B> = Entity<B, StakePool>;
pub type StakerTicketEntity<B> = Entity<B, StakerTicket>;

#[derive(Debug, PartialEq, Eq, Clone, parity_scale_codec::Encode, parity_scale_codec::Decode)]
pub enum Method {
    CreatePool(InitializeArgs),
    Stake { amount: TokenAmount },
    Unstake { amount: TokenAmount },
    ClaimReward,
    AddReward { amount: TokenAmount },
}

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

    pub stake_target_amount: TokenAmount,
    pub stake_acquired_amount: TokenAmount,
    pub reward_amount: TokenAmount,
    pub deposited_reward_amount: TokenAmount,

    pub allocator: EntityAllocator,

    pub genesis: Checked<i64>,
    pub lockup_duration: Checked<i64>,
    pub topup_duration: Checked<i64>,
}

#[repr(C)]
pub struct StakerTicketState {
    pub authority: Pubkey,
    pub staked_amount: TokenAmount,
}

impl AccountType for StakePool {
    const KIND: EntityKind = EntityKind::SimpleStakePool;

    fn is_valid_size(size: usize) -> bool {
        size == size_of::<StakePoolState>()
    }

    fn default_size() -> usize {
        size_of::<StakePoolState>() + HEADER_RESERVED
    }
}

impl AccountType for StakerTicket {
    const KIND: EntityKind = EntityKind::SimpleStakeTicket;

    fn is_valid_size(size: usize) -> bool {
        size == size_of::<StakePoolState>()
    }

    fn default_size() -> usize {
        size_of::<StakePoolState>() + HEADER_RESERVED
    }
}

impl_entity_simple_deref!(StakePool, StakePoolState);
impl_entity_simple_deref!(StakerTicket, StakerTicketState);

#[derive(Debug)]
pub struct InitializeArgsAccounts<B: AccountBackend> {
    pub administrator_authority: B,
    pub program_authority: B,
    pub pool: B,
    pub stake_mint: MintAccount<B>,
    pub stake_vault: WalletAccount<B>,
}

#[cfg(feature = "onchain")]
impl<B: AccountBackend> InitializeArgsAccounts<B> {
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &administrator_authority,
            &program_authority,
            &mut pool,
            &stake_mint = MintAccount::any(this)?,
            &stake_vault = stake_mint.wallet(this)?
        }

        Ok(Self {
            administrator_authority,
            program_authority,
            pool,
            stake_mint,
            stake_vault,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, parity_scale_codec::Encode, parity_scale_codec::Decode)]
pub struct InitializeArgs {
    pub program_authority_salt: u64,
    pub lockup_duration: Checked<i64>,
    pub topup_duration: Checked<i64>,
    pub target_amount: TokenAmount,
    pub reward_amount: TokenAmount,
}

#[derive(Debug)]
pub struct StakeArgsAccounts<B: AccountBackend> {
    pub token_program: TokenProgram<B>,

    pub pool: Entity<B, StakePool>,
    pub staker: B,
    pub ticket: Entity<B, StakerTicket>,
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

impl<B: AccountBackend> StakeArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error>
    where
        B::Impl: AccountFieldsMut,
    {
        let program_id = *input.program_id();

        parse_accounts!(
            &token_program = TokenProgram::load(this)?,
            &mut pool = <Entity<B, StakePool>>::load(&program_id, this)?,
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

impl<B: AccountBackend> UnStakeArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline(always)]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        let program_id = *input.program_id();

        parse_accounts!(
            &token_program = TokenProgram::load(this)?,
            &mut pool = <Entity<B, StakePool>>::load(&program_id, this)?,
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

impl<B: AccountBackend> AddRewardArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        let program_id = *input.program_id();

        parse_accounts!(
            &token_program = TokenProgram::load(this)?,
            &mut pool = <Entity<B, StakePool>>::load(&program_id, this)?,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeArgs {
    pub amount: TokenAmount,
}

impl<B> Entity<B, StakePool>
where
    B: AccountBackend,
{
    #[cfg(feature = "onchain")]
    #[inline(never)]
    pub fn initialize<T>(input: &mut T, args: InitializeArgs) -> Result<(), Error>
    where
        B::Impl: AccountFieldsMut,
        T: AccountSource<B>,
    {
        let InitializeArgsAccounts {
            administrator_authority,
            program_authority,
            pool,
            stake_mint,
            stake_vault,
        } = InitializeArgsAccounts::from_program_input(input)?;

        let mut entity = Self::raw_any(input.program_id(), pool)?;

        let expected_program_authority = Pubkey::create_program_address(
            &[
                entity.account().key().as_ref(),
                administrator_authority.key().as_ref(),
                &args.program_authority_salt.to_le_bytes(),
            ],
            input.program_id(),
        )
        .bpf_expect("couldn't derive program authority");

        if !pubkey_eq(program_authority.key(), &expected_program_authority) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority);
        }

        if !pubkey_eq(stake_vault.authority(), &expected_program_authority) {
            qlog!("stake vault authority does not match program authority");
            return Err(Error::InvalidAuthority);
        }

        if !pubkey_eq(stake_mint.key(), stake_vault.mint()) {
            qlog!("stake vault mint does not match provided stake mint");
            return Err(Error::InvalidParent);
        }

        let now = timestamp_now();

        if args.topup_duration > args.lockup_duration {
            qlog!("topup_duration should be less than lockup_duration");
            return Err(Error::InvalidData);
        }

        entity.program_authority = *program_authority.key();
        entity.administrator_authority = *administrator_authority.key();
        entity.genesis = now;
        entity.topup_duration = args.topup_duration;
        entity.lockup_duration = args.lockup_duration;

        entity.stake_acquired_amount = 0.into();
        entity.stake_target_amount = args.target_amount;
        entity.reward_amount = args.reward_amount;

        entity.stake_mint = *stake_mint.key();
        entity.stake_vault = *stake_vault.key();

        let id = entity.allocator.allocate_id();
        let entity_key = *entity.account().key();
        let header = entity.header_mut();
        header.kind = EntityKind::SimpleStakePool;
        header.id = id;
        header.parent_id = id;
        header.root = entity_key;

        Ok(())
    }

    #[inline]
    pub fn genesis(&self) -> Checked<i64> {
        self.genesis
    }

    #[inline]
    pub fn topup_duration(&self) -> Checked<i64> {
        self.topup_duration
    }

    #[inline]
    pub fn lockup_duration(&self) -> Checked<i64> {
        self.lockup_duration
    }

    #[inline]
    pub fn can_topup(&self, now: Checked<i64>) -> bool {
        now < self.genesis() + self.topup_duration()
    }

    #[inline]
    pub fn can_withdraw(&self, now: Checked<i64>) -> bool {
        self.can_topup(now) || self.is_expired(now)
    }

    #[inline]
    pub fn is_expired(&self, now: Checked<i64>) -> bool {
        now > self.genesis() + self.lockup_duration()
    }

    #[inline]
    pub fn authority_seeds(&self) -> [&[u8]; 3] {
        [
            self.account().key().as_ref(),
            self.administrator_authority.as_ref(),
            as_bytes(&self.program_authority_salt),
        ]
    }

    #[inline]
    pub fn stake_mint(&self, account: B) -> Result<MintAccount<B>, Error> {
        let mint = MintAccount::any(account)?;

        if !pubkey_eq(&self.stake_mint, mint.key()) {
            return Err(Error::InvalidMint);
        }

        Ok(mint)
    }

    #[inline]
    pub fn stake_wallet(&self, account: B) -> Result<WalletAccount<B>, Error> {
        let wallet = WalletAccount::any(account)?;

        if !pubkey_eq(&self.stake_mint, wallet.mint()) {
            return Err(Error::InvalidMint);
        }

        Ok(wallet)
    }

    #[inline]
    pub fn stake_vault(&self, account: B) -> Result<WalletAccount<B>, Error> {
        let wallet = WalletAccount::any(account)?;

        if !pubkey_eq(&self.stake_vault, wallet.key()) {
            return Err(Error::InvalidAccount);
        }

        Ok(wallet)
    }

    #[inline]
    pub fn load_ticket(&self, ticket: B) -> Result<Entity<B, StakerTicket>, Error> {
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

    #[cfg(feature = "onchain")]
    #[inline]
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

    #[cfg(feature = "onchain")]
    #[inline(never)]
    pub fn add_stake<T>(input: &mut T, amount: TokenAmount) -> Result<(), Error>
    where
        B: AccountBackend<Impl = Account>,
        T: AccountSource<B>,
    {
        let StakeArgsAccounts {
            token_program,
            mut pool,
            mut ticket,
            mut stake_vault,
            source_authority,
            mut source_wallet,
            ..
        } = StakeArgsAccounts::from_program_input(input)?;

        if source_wallet.amount() < amount {
            qlog!("not enough funds in wallet");
            return Err(Error::Validation);
        }

        let now = timestamp_now();

        if !pool.can_topup(now) {
            qlog!("pool is locked and funds can no longer be added");
            return Err(Error::Validation);
        }

        let transfer_amount = amount.min(pool.stake_target_amount - pool.stake_acquired_amount);

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

    #[cfg(feature = "onchain")]
    #[inline(never)]
    pub fn remove_stake<T>(input: &mut T, amount: TokenAmount) -> Result<(), Error>
    where
        B: AccountBackend<Impl = Account>,
        T: AccountSource<B>,
    {
        let UnStakeArgsAccounts {
            token_program,
            mut pool,
            mut staker,
            mut ticket,
            program_authority,
            mut stake_vault,
            mut target_wallet,
        } = UnStakeArgsAccounts::from_program_input(input)?;

        if !pubkey_eq(&ticket.authority, staker.key()) {
            qlog!("wrong staker provided");
            return Err(Error::Validation);
        }

        if !staker.is_signer() {
            qlog!("the staker is expected to sign");
            return Err(Error::Validation);
        }

        let now = timestamp_now();

        if !pool.can_topup(now) {
            qlog!("pool is locked and funds can no longer be removed");
            return Err(Error::Validation);
        }

        let transfer_amount = amount.min(ticket.staked_amount);

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
        ticket.collect(&mut staker)?;

        Ok(())
    }

    #[cfg(feature = "onchain")]
    #[inline(never)]
    pub fn claim_reward<T>(input: &mut T) -> Result<(), Error>
    where
        B: AccountBackend<Impl = Account>,
        T: AccountSource<B>,
    {
        let UnStakeArgsAccounts {
            token_program,
            pool,
            mut staker,
            mut ticket,
            program_authority,
            mut stake_vault,
            mut target_wallet,
        } = UnStakeArgsAccounts::from_program_input(input)?;

        if !pubkey_eq(&ticket.authority, staker.key()) {
            qlog!("wrong staker provided");
            return Err(Error::Validation);
        }

        if !staker.is_signer() {
            qlog!("the staker is expected to sign");
            return Err(Error::Validation);
        }

        let now = timestamp_now();

        if !pool.is_expired(now) {
            qlog!("cannot claim pool reward yet");
            return Err(Error::Validation);
        }

        let staked_amount = ticket.staked_amount.to_u64f64();
        let stake_acquired_amount = pool.stake_acquired_amount.to_u64f64();
        let reward_amount = pool.reward_amount.to_u64f64();

        let share = staked_amount / stake_acquired_amount;
        let reward_share = share * reward_amount;

        let transfer_amount = (staked_amount + reward_share)
            .checked_as::<TokenAmount>()
            .bpf_unwrap();

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

        ticket.staked_amount = 0.into();
        assert!(ticket.collect(&mut staker)?);

        Ok(())
    }

    #[cfg(feature = "onchain")]
    #[inline(never)]
    pub fn add_reward<T>(input: &mut T, amount: TokenAmount) -> Result<(), Error>
    where
        B: AccountBackend<Impl = Account>,
        T: AccountSource<B>,
    {
        let AddRewardArgsAccounts {
            token_program,
            mut pool,
            mut stake_vault,
            source_authority,
            mut source_wallet,
        } = AddRewardArgsAccounts::from_program_input(input)?;

        let transfer_amount = amount
            .min(pool.reward_amount - pool.deposited_reward_amount)
            .min(source_wallet.amount());

        if transfer_amount == 0.into() {
            qlog!("no reward to deposit");
            return Err(Error::Validation);
        }

        let now = timestamp_now();

        if pool.is_expired(now) {
            qlog!("reward cannot be added to a pool that has expired");
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
