#![allow(unused)]

use std::{marker::PhantomData, mem::MaybeUninit, ptr::null};

use solana_api_types::{
    program::{ProgramError, ProgramResult},
    Pubkey,
};

use crate::{
    account::{
        onchain::{Account, AsAccount},
        AccountFields,
    },
    collections::StaticVec,
    prelude::AccountBackend,
    qlog,
};

#[repr(C)]
struct Instruction {
    program_id: *const Pubkey,
    meta_addr: *const Meta,
    meta_len: usize,
    data_addr: *const u8,
    data_len: usize,
}

#[doc(hidden)]
#[repr(C)]
pub struct Meta {
    pubkey: *const Pubkey,
    is_writable: bool,
    is_signer: bool,
}

#[repr(C)]
struct SignerSeed {
    addr: *const u8,
    len: u64,
}

#[repr(C)]
struct SignerSeeds {
    addr: *const SignerSeed,
    len: u64,
}

#[cfg(target_arch = "bpf")]
extern "C" {
    fn sol_invoke_signed_c(
        instruction_addr: *const Instruction,
        account_infos_addr: *const Account,
        account_infos_len: u64,
        signers_seeds_addr: *const SignerSeeds,
        signers_seeds_len: u64,
    ) -> u64;
}

#[cfg(not(target_arch = "bpf"))]
unsafe fn sol_invoke_signed_c(
    instruction_addr: *const Instruction,
    account_infos_addr: *const Account,
    account_infos_len: u64,
    signers_seeds_addr: *const SignerSeeds,
    signers_seeds_len: u64,
) -> u64 {
    use std::{
        cell::RefCell,
        rc::Rc,
        slice::{from_raw_parts, from_raw_parts_mut},
    };

    use solana_program::{account_info::AccountInfo, instruction::AccountMeta};

    #[cfg(not(feature = "runtime-test"))]
    {
        unimplemented!()
    }

    #[cfg(feature = "runtime-test")]
    {
        let accounts = from_raw_parts(account_infos_addr, account_infos_len as usize);
        let signers = from_raw_parts(signers_seeds_addr, signers_seeds_len as usize);

        let mut infos = vec![];
        for account in accounts {
            let lamports = Rc::new(RefCell::new(&mut *account.lamports));
            let data = from_raw_parts_mut(account.data, account.data_len);
            let data = Rc::new(RefCell::new(data));

            let info = AccountInfo {
                key: &*account.key.cast(),
                is_signer: account.is_signer,
                is_writable: account.is_writable,
                lamports,
                data,
                owner: &*account.owner.cast(),
                executable: account.is_executable,
                rent_epoch: account.rent_epoch,
            };

            infos.push(info)
        }

        let instruction = &*instruction_addr;

        let metas = from_raw_parts(instruction.meta_addr, instruction.meta_len);
        let data = from_raw_parts(instruction.data_addr, instruction.data_len);
        let instruction = solana_program::instruction::Instruction {
            program_id: *instruction.program_id.cast(),
            accounts: metas
                .iter()
                .map(|m| {
                    if m.is_writable {
                        AccountMeta::new(*m.pubkey.cast(), m.is_signer)
                    } else {
                        AccountMeta::new_readonly(*m.pubkey.cast(), m.is_signer)
                    }
                })
                .collect(),
            data: data.into(),
        };

        let signers = from_raw_parts(signers_seeds_addr, signers_seeds_len as usize)
            .iter()
            .map(|seeds| {
                from_raw_parts(seeds.addr, seeds.len as usize)
                    .iter()
                    .map(|seed| from_raw_parts(seed.addr, seed.len as usize))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut signers_slice = vec![];
        for seeds in &signers {
            signers_slice.push(seeds.as_slice());
        }

        if let Err(err) =
            solana_program::program::invoke_signed(&instruction, &infos, &signers_slice)
        {
            err.into()
        } else {
            0
        }
    }
}

pub struct Invoker<'a, const N: usize> {
    accounts: StaticVec<Account, N>,
    metas: StaticVec<Meta, N>,
    _phantom: PhantomData<&'a mut ()>,
}

pub trait ToInvokeMeta<'a> {
    #[doc(hidden)]
    fn __to_meta(&self, sign: bool) -> Meta;

    #[doc(hidden)]
    fn __as_account(&self) -> &Account;
}

impl<'a, T: AsAccount> ToInvokeMeta<'a> for &'a T {
    fn __to_meta(&self, sign: bool) -> Meta {
        Meta {
            pubkey: self.as_account_ref().key,
            is_writable: false,
            is_signer: sign,
        }
    }

    fn __as_account(&self) -> &Account {
        self.as_account_ref()
    }
}

impl<'a, T: AsAccount> ToInvokeMeta<'a> for &'a mut T {
    fn __to_meta(&self, sign: bool) -> Meta {
        Meta {
            pubkey: self.as_account_ref().key,
            is_writable: true,
            is_signer: sign,
        }
    }

    fn __as_account(&self) -> &Account {
        self.as_account_ref()
    }
}

impl<'a, const N: usize> Default for Invoker<'a, N> {
    fn default() -> Self {
        Self {
            accounts: StaticVec::default(),
            metas: StaticVec::default(),
            _phantom: Default::default(),
        }
    }
}

impl<'a, const N: usize> Invoker<'a, N> {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    fn push_inner(&mut self, account: &Account, meta: Meta) {
        self.accounts.push(unsafe { account.copy() });
        self.metas.push(meta);
    }

    #[inline]
    pub fn push<T: ToInvokeMeta<'a>>(&mut self, account: T) {
        self.push_inner(account.__as_account(), account.__to_meta(false))
    }

    #[inline]
    pub fn push_signed<T: ToInvokeMeta<'a>>(&mut self, account: T) {
        self.push_inner(account.__as_account(), account.__to_meta(true))
    }

    pub fn invoke<T: std::borrow::Borrow<Account>>(
        &mut self,
        program: T,
        data: &[u8],
    ) -> ProgramResult {
        self.invoke_signed(program, data, &[])
    }

    pub fn invoke_signed<T: std::borrow::Borrow<Account>>(
        &mut self,
        program: T,
        data: &[u8],
        signer_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let program: &Account = program.borrow();

        let instruction = Instruction {
            program_id: program.as_account_ref().key(),
            meta_addr: self.metas.as_ptr(),
            meta_len: self.metas.len(),
            data_addr: data.as_ptr(),
            data_len: data.len(),
        };

        self.accounts
            .push(unsafe { program.as_account_ref().copy() });

        let mut seeds = StaticVec::<SignerSeeds, 4>::default();
        let mut seed_parts = StaticVec::<SignerSeed, 32>::default();

        for seed in signer_seeds {
            let head = unsafe { seed_parts.as_ptr().add(seed_parts.len()) };

            for seed_part in *seed {
                seed_parts.push(SignerSeed {
                    addr: seed_part.as_ptr(),
                    len: seed_part.len() as u64,
                })
            }

            seeds.push(SignerSeeds {
                addr: head,
                len: seed.len() as u64,
            })
        }

        let result = unsafe {
            sol_invoke_signed_c(
                &instruction,
                self.accounts.as_ptr(),
                self.accounts.len() as u64,
                seeds.as_ptr(),
                seeds.len() as u64,
            )
        };

        self.accounts.pop();

        if result != 0 {
            Err(ProgramError::from(result))
        } else {
            Ok(())
        }
    }

    pub fn die(&mut self) {}
}
