use std::{io::Write, mem::size_of, ops::Deref};

#[cfg(feature = "onchain")]
use solana_api_types::program::ProgramError;
use solana_api_types::{system::create_account, sysvar, AccountMeta, Instruction, Pubkey};

use crate::{
    account::{pubkey::PubkeyAccount, AccountBackend, AccountFields},
    authority::Authority,
    collections::StaticVec,
    error::SolarError,
    forward_account_backend,
    log::Loggable,
    math::Checked,
    reinterpret::{is_valid_for_type, reinterpret_unchecked},
    util::{minimum_balance, pubkey_eq},
};

pub const ID: &Pubkey = &solar_macros::parse_pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MintAccount<B: AccountBackend> {
    account: B,
}

impl<B> serde::Serialize for MintAccount<B>
where
    B: AccountBackend + serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.account.serialize(serializer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalletAccount<B: AccountBackend> {
    account: B,
}

impl<B> serde::Serialize for WalletAccount<B>
where
    B: AccountBackend + serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.account.serialize(serializer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenProgram<B> {
    account: B,
}

impl<'a, 'b: 'a, B: AccountBackend> MintAccount<B> {
    pub fn any(account: B) -> Result<Self, SolarError> {
        let data = account.data();

        if !pubkey_eq(account.owner(), &*ID) {
            Err(SolarError::InvalidOwner)
        } else if data.len() != size_of::<Mint>() || !is_valid_for_type::<Mint>(data) {
            Err(SolarError::InvalidData)
        } else {
            Ok(Self { account })
        }
    }

    pub fn wallet(&self, account: B) -> Result<WalletAccount<B>, SolarError> {
        let wallet = WalletAccount::<B>::any(account)?;

        if !pubkey_eq(wallet.mint, self.key()) {
            Err(SolarError::InvalidMint)
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
    pub fn any(account: B) -> Result<Self, SolarError> {
        let data = account.data();

        if !pubkey_eq(account.owner(), &*ID) {
            Err(SolarError::InvalidOwner)
        } else if data.len() != size_of::<Wallet>() || !is_valid_for_type::<Wallet>(data) {
            Err(SolarError::InvalidData)
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
    pub fn load(account: B) -> Result<Self, SolarError> {
        if !pubkey_eq(account.key(), &*ID) {
            Err(SolarError::InvalidOwner)
        } else {
            Ok(Self { account })
        }
    }

    pub fn account(&self) -> &B {
        &self.account
    }
}

#[cfg(feature = "onchain")]
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
        authority: &Authority<T>,
        seeds: &[&[&[u8]]],
    ) -> Result<Result<(), TokenError>, ProgramError>
    where
        T: AccountBackend<Impl = crate::account::onchain::Account>,
    {
        let mut invoker = crate::invoke::Invoker::<4>::new();
        invoker.push(from);
        invoker.push(to);
        invoker.push_signed(authority.account());

        Self::handle_result(invoker.invoke_signed(
            self.backend(),
            &TokenInstruction::Transfer { amount }.pack_static_vec(),
            seeds,
        ))
    }
}

impl From<Pubkey> for TokenProgram<PubkeyAccount> {
    fn from(pubkey: Pubkey) -> Self {
        Self {
            account: pubkey.into(),
        }
    }
}

impl From<Pubkey> for WalletAccount<PubkeyAccount> {
    fn from(pubkey: Pubkey) -> Self {
        Self {
            account: pubkey.into(),
        }
    }
}

impl From<Pubkey> for MintAccount<PubkeyAccount> {
    fn from(pubkey: Pubkey) -> Self {
        Self {
            account: pubkey.into(),
        }
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

    pub fn pack_static_vec(&self) -> StaticVec<u8, 96> {
        let mut vec = StaticVec::<u8, 96>::default();
        self.write(&mut vec).expect("infallible");
        vec
    }

    pub fn pack_vec(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(96);
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

impl Loggable for TokenError {
    fn push_to_logger<const S: usize>(&self, logger: &mut crate::log::Logger<S>) {
        logger.push_str(self.into())
    }
}

pub fn create_mint(
    payer: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    decimals: u8,
) -> [Instruction; 2] {
    [
        create_account(
            payer,
            mint,
            minimum_balance(size_of::<Mint>() as u64),
            size_of::<Mint>() as u64,
            ID,
        ),
        initialize_mint(mint, authority, decimals),
    ]
}

pub fn create_wallet(
    payer: &Pubkey,
    wallet: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
) -> [Instruction; 2] {
    [
        create_account(
            payer,
            wallet,
            minimum_balance(size_of::<Wallet>() as u64),
            size_of::<Wallet>() as u64,
            ID,
        ),
        initialize_wallet(wallet, mint, authority),
    ]
}

pub fn mint_to(mint: &Pubkey, wallet: &Pubkey, authority: &Pubkey, amount: u64) -> Instruction {
    Instruction {
        program_id: *ID,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(*wallet, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data: TokenInstruction::MintTo { amount }.pack_vec(),
    }
}

pub fn initialize_mint(mint: &Pubkey, authority: &Pubkey, decimals: u8) -> Instruction {
    Instruction {
        program_id: *ID,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(*sysvar::rent::ID, false),
        ],
        data: TokenInstruction::InitializeMint {
            decimals,
            mint_authority: *authority,
            freeze_authority: None,
        }
        .pack_vec(),
    }
}

pub fn initialize_wallet(wallet: &Pubkey, mint: &Pubkey, authority: &Pubkey) -> Instruction {
    Instruction {
        program_id: *ID,
        accounts: vec![
            AccountMeta::new(*wallet, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*authority, false),
            AccountMeta::new_readonly(*sysvar::rent::ID, false),
        ],
        data: TokenInstruction::InitializeAccount.pack_vec(),
    }
}
