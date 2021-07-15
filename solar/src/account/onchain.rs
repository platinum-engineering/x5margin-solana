use solana_program::pubkey::Pubkey;

use super::{AccountBackend, AccountBackendMut};

pub struct Account<'a> {
    pub(crate) key: &'a Pubkey,
    pub(crate) owner: &'a Pubkey,
    pub(crate) is_signer: bool,
    pub(crate) is_writable: bool,
    pub(crate) is_executable: bool,
    pub(crate) rent_epoch: u64,
    pub(crate) lamports: &'a mut u64,
    pub(crate) data: &'a mut [u8],
}

impl<'a> AccountBackend for Account<'a> {
    fn key(&self) -> &'a Pubkey {
        self.key
    }

    fn is_signer(&self) -> bool {
        self.is_signer
    }

    fn is_writable(&self) -> bool {
        self.is_writable
    }

    fn is_executable(&self) -> bool {
        self.is_executable
    }

    fn lamports(&self) -> u64 {
        *self.lamports
    }

    fn owner(&self) -> &'a Pubkey {
        self.owner
    }

    fn rent_epoch(&self) -> u64 {
        self.rent_epoch
    }

    fn data(&self) -> &[u8] {
        self.data
    }
}

impl<'a> AccountBackendMut for Account<'a> {
    fn set_lamports(&mut self, value: u64) {
        *self.lamports = value;
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self.data
    }
}

pub type AccountRef = &'static mut Account<'static>;
