use std::{io::Write, mem::size_of, ops::Deref};

use solana_api_types::{program::ProgramError, Pubkey};

use crate::{
    account::{onchain::Account, AccountBackend, AccountFields},
    collections::StaticVec,
    forward_account_backend,
    invoke::Invoker,
    log::Loggable,
    math::Checked,
    reinterpret::{is_valid_for_type, reinterpret_unchecked},
    util::pubkey_eq,
};

solana_api_types::declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

#[repr(packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mint {
    mint_authority_tag: u32,
    mint_authority: Pubkey,
    supply: Checked<u64>,
    decimals: u8,
    is_initialized: bool,
    freeze_authority_tag: u32,
    freeze_authority: Pubkey,
}

impl Mint {
    pub fn mint_authority(&self) -> Option<&Pubkey> {
        if self.mint_authority_tag == 1 {
            Some(&self.mint_authority)
        } else {
            None
        }
    }

    pub fn freeze_authority(&self) -> Option<&Pubkey> {
        if self.freeze_authority_tag == 1 {
            Some(&self.freeze_authority)
        } else {
            None
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    pub fn supply(&self) -> Checked<u64> {
        self.supply
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wallet {
    mint: Pubkey,
    authority: Pubkey,
    amount: Checked<u64>,
    delegate_tag: u32,
    delegate: Pubkey,
    state: u8,
    is_native_tag: u32,
    is_native: Checked<u64>,
    delegated_amount: Checked<u64>,
    close_authority_tag: u32,
    close_authority: Pubkey,
}

#[derive(IntoStaticStr, Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum AccountState {
    Uninitialized,
    Initialized,
    Frozen,
    Invalid,
}

impl Wallet {
    pub fn mint(&self) -> &Pubkey {
        &self.mint
    }

    pub fn authority(&self) -> &Pubkey {
        &self.authority
    }

    pub fn amount(&self) -> Checked<u64> {
        self.amount
    }

    pub fn delegate(&self) -> Option<&Pubkey> {
        if self.delegate_tag == 1 {
            Some(&self.delegate)
        } else {
            None
        }
    }

    pub fn state(&self) -> AccountState {
        match self.state {
            0 => AccountState::Uninitialized,
            1 => AccountState::Initialized,
            2 => AccountState::Frozen,
            _ => AccountState::Invalid,
        }
    }

    pub fn is_native(&self) -> bool {
        self.is_native_tag == 1
    }

    pub fn native_reserve(&self) -> Option<Checked<u64>> {
        if self.is_native_tag == 1 {
            Some(self.is_native)
        } else {
            None
        }
    }

    pub fn delegated_amount(&self) -> Checked<u64> {
        self.delegated_amount
    }

    pub fn close_authority(&self) -> Option<&Pubkey> {
        if self.close_authority_tag == 1 {
            Some(&self.close_authority)
        } else {
            None
        }
    }
}

#[derive(IntoStaticStr, Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum SplReadError {
    InvalidData,
    InvalidOwner,
    InvalidMint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MintAccount<B: AccountBackend> {
    account: B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalletAccount<B: AccountBackend> {
    account: B,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenProgram<B> {
    account: B,
}

impl<'a, 'b: 'a, B: AccountBackend> MintAccount<B> {
    pub fn any(account: B) -> Result<Self, SplReadError> {
        let data = account.data();

        if !pubkey_eq(account.owner(), &ID) {
            Err(SplReadError::InvalidOwner)
        } else if data.len() != size_of::<Mint>() || !is_valid_for_type::<Mint>(data) {
            Err(SplReadError::InvalidData)
        } else {
            Ok(Self { account })
        }
    }

    pub fn wallet(&self, account: B) -> Result<WalletAccount<B>, SplReadError> {
        let wallet = WalletAccount::<B>::any(account)?;

        if !pubkey_eq(wallet.mint, self.key()) {
            Err(SplReadError::InvalidMint)
        } else {
            Ok(wallet)
        }
    }
}

impl<B: AccountBackend> Deref for MintAccount<B> {
    type Target = Mint;

    fn deref(&self) -> &Self::Target {
        unsafe { reinterpret_unchecked(self.account.data()) }
    }
}

impl<B: AccountBackend> WalletAccount<B> {
    pub fn any(account: B) -> Result<Self, SplReadError> {
        let data = account.data();

        if !pubkey_eq(account.owner(), &ID) {
            Err(SplReadError::InvalidOwner)
        } else if data.len() != size_of::<Wallet>() || !is_valid_for_type::<Wallet>(data) {
            Err(SplReadError::InvalidData)
        } else {
            Ok(Self { account })
        }
    }
}

impl<B: AccountBackend> Deref for WalletAccount<B> {
    type Target = Wallet;

    fn deref(&self) -> &Self::Target {
        unsafe { reinterpret_unchecked(self.account.data()) }
    }
}

impl<B: AccountBackend> TokenProgram<B> {
    pub fn load(account: B) -> Result<Self, SplReadError> {
        if !pubkey_eq(account.key(), &ID) {
            Err(SplReadError::InvalidOwner)
        } else {
            Ok(Self { account })
        }
    }

    pub fn account(&self) -> &B {
        &self.account
    }
}

impl<T: AccountBackend> TokenProgram<T> {
    fn handle_result(
        error: Result<(), ProgramError>,
    ) -> Result<Result<(), TokenError>, ProgramError> {
        match error {
            Ok(()) => Ok(Ok(())),
            Err(ProgramError::Custom(code)) => Ok(Err(TokenError::from(code))),
            Err(error) => Err(error),
        }
    }

    #[inline(never)]
    pub fn transfer(
        &self,
        from: &mut WalletAccount<T>,
        to: &mut WalletAccount<T>,
        amount: u64,
        authority: &T,
        seeds: &[&[&[u8]]],
    ) -> Result<Result<(), TokenError>, ProgramError>
    where
        T: AccountBackend<Impl = Account>,
    {
        let mut invoker = Invoker::<4>::new();
        invoker.push(from);
        invoker.push(to);
        invoker.push_signed(authority);

        Self::handle_result(invoker.invoke_signed(
            self.backend(),
            &TokenInstruction::Transfer { amount }.pack(),
            seeds,
        ))
    }
}

forward_account_backend!(TokenProgram, account);
forward_account_backend!(WalletAccount, account);
forward_account_backend!(MintAccount, account);

#[repr(u8)]
#[derive(IntoStaticStr, Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityType {
    MintTokens,
    FreezeAccount,
    AccountOwner,
    CloseAccount,
}

#[derive(IntoStaticStr, Debug, Display, Clone, PartialEq, Eq)]
pub enum TokenInstruction {
    InitializeMint {
        decimals: u8,
        mint_authority: Pubkey,
        freeze_authority: Option<Pubkey>,
    },
    InitializeAccount,
    InitializeMultisig {
        m: u8,
    },
    Transfer {
        amount: u64,
    },
    Approve {
        amount: u64,
    },
    Revoke,
    SetAuthority {
        authority_type: AuthorityType,
        new_authority: Option<Pubkey>,
    },
    MintTo {
        amount: u64,
    },
    Burn {
        amount: u64,
    },
    CloseAccount,
    FreezeAccount,
    ThawAccount,
    TransferChecked {
        amount: u64,
        decimals: u8,
    },
    ApproveChecked {
        amount: u64,
        decimals: u8,
    },
    MintToChecked {
        amount: u64,
        decimals: u8,
    },
    BurnChecked {
        amount: u64,
        decimals: u8,
    },
    InitializeAccount2 {
        owner: Pubkey,
    },
}

#[repr(u32)]
#[derive(IntoStaticStr, Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum TokenError {
    NotRentExempt = 0,
    InsufficientFunds = 1,
    InvalidMint = 2,
    MintMismatch = 3,
    OwnerMismatch = 4,
    FixedSupply = 5,
    AlreadyInUse = 6,
    InvalidNumberOfProvidedSigners = 7,
    InvalidNumberOfRequiredSigners = 8,
    UninitializedState = 9,
    NativeNotSupported = 10,
    NonNativeHasBalance = 11,
    InvalidInstruction = 12,
    InvalidState = 13,
    Overflow = 14,
    AuthorityTypeNotSupported = 15,
    MintCannotFreeze = 16,
    AccountFrozen = 17,
    MintDecimalsMismatch = 18,

    Unknown,
}

impl TokenError {
    pub fn from(code: u32) -> Self {
        if code <= 18 {
            unsafe { std::mem::transmute(code) }
        } else {
            Self::Unknown
        }
    }
}

fn write_pubkey<W: Write>(mut writer: W, pubkey: &Pubkey) -> std::io::Result<()> {
    writer.write_all(pubkey.as_ref())
}

fn write_pubkey_option<W: Write>(mut writer: W, pubkey: &Option<Pubkey>) -> std::io::Result<()> {
    use byteorder::WriteBytesExt;
    if let Some(pubkey) = pubkey {
        write_pubkey(writer, pubkey)
    } else {
        writer.write_u8(0)
    }
}

impl TokenInstruction {
    #[inline]
    pub fn id(&self) -> u8 {
        match self {
            TokenInstruction::InitializeMint { .. } => 0,
            TokenInstruction::InitializeAccount => 1,
            TokenInstruction::InitializeMultisig { .. } => 2,
            TokenInstruction::Transfer { .. } => 3,
            TokenInstruction::Approve { .. } => 4,
            TokenInstruction::Revoke => 5,
            TokenInstruction::SetAuthority { .. } => 6,
            TokenInstruction::MintTo { .. } => 7,
            TokenInstruction::Burn { .. } => 8,
            TokenInstruction::CloseAccount => 9,
            TokenInstruction::FreezeAccount => 10,
            TokenInstruction::ThawAccount => 11,
            TokenInstruction::TransferChecked { .. } => 12,
            TokenInstruction::ApproveChecked { .. } => 13,
            TokenInstruction::MintToChecked { .. } => 14,
            TokenInstruction::BurnChecked { .. } => 15,
            TokenInstruction::InitializeAccount2 { .. } => 16,
        }
    }

    pub fn pack(&self) -> StaticVec<u8, 96> {
        let mut vec = StaticVec::<u8, 96>::default();
        self.write(&mut vec).expect("infallible");
        vec
    }

    pub fn write<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        use byteorder::WriteBytesExt;
        use byteorder::LE;

        writer.write_u8(self.id())?;

        match self {
            TokenInstruction::InitializeMint {
                decimals,
                mint_authority,
                freeze_authority,
            } => {
                writer.write_u8(*decimals)?;
                write_pubkey(&mut writer, mint_authority)?;
                write_pubkey_option(&mut writer, freeze_authority)?;
            }
            TokenInstruction::InitializeAccount => {}
            TokenInstruction::InitializeMultisig { m } => writer.write_u8(*m)?,
            TokenInstruction::Transfer { amount } => writer.write_u64::<LE>(*amount)?,
            TokenInstruction::Approve { amount } => writer.write_u64::<LE>(*amount)?,
            TokenInstruction::Revoke => {}
            TokenInstruction::SetAuthority {
                authority_type,
                new_authority,
            } => {
                writer.write_u8(*authority_type as u8)?;
                write_pubkey_option(writer, new_authority)?;
            }
            TokenInstruction::MintTo { amount } => writer.write_u64::<LE>(*amount)?,
            TokenInstruction::Burn { amount } => writer.write_u64::<LE>(*amount)?,
            TokenInstruction::CloseAccount => {}
            TokenInstruction::FreezeAccount => {}
            TokenInstruction::ThawAccount => {}
            TokenInstruction::TransferChecked { amount, decimals }
            | TokenInstruction::ApproveChecked { amount, decimals }
            | TokenInstruction::MintToChecked { amount, decimals }
            | TokenInstruction::BurnChecked { amount, decimals } => {
                writer.write_u64::<LE>(*amount)?;
                writer.write_u8(*decimals)?;
            }
            TokenInstruction::InitializeAccount2 { owner } => write_pubkey(writer, owner)?,
        }

        Ok(())
    }
}

impl Loggable for SplReadError {
    fn push_to_logger<const S: usize>(&self, logger: &mut crate::log::Logger<S>) {
        logger.push_str(self.into())
    }
}

impl Loggable for TokenError {
    fn push_to_logger<const S: usize>(&self, logger: &mut crate::log::Logger<S>) {
        logger.push_str(self.into())
    }
}
