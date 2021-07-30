use std::{
    alloc::Layout,
    mem::{align_of, size_of, MaybeUninit},
    slice::from_raw_parts,
};

use solana_program::{
    account_info::AccountInfo,
    entrypoint::{ProgramResult, MAX_PERMITTED_DATA_INCREASE},
    pubkey::Pubkey,
};

use crate::account::onchain::{Account, AccountRef};

pub const MAX_ACCOUNTS: usize = 32;

pub struct ProgramInput {
    program_id: &'static Pubkey,
    accounts: ProgramAccounts,
    data: &'static [u8],
}

pub struct ProgramAccounts {
    accounts: &'static mut [MaybeUninit<Account>; MAX_ACCOUNTS],
    len: usize,
    cursor: usize,
}

#[repr(C)]
struct SerializedAccount {
    dup_info: u8,
    is_signer: u8,
    is_writable: u8,
    executable: u8,
    padding: [u8; 4],
    key: Pubkey,
    owner: Pubkey,
    lamports: u64,
    data_len: u64,
}

impl ProgramInput {
    /// Deserialize inputs to a BPF program invocation.
    ///
    /// This implementation is hand-optimized to produce minimal bytecode.
    /// # Safety
    /// Must be called with a pointer to a BPF entrypoint memory region, or one that mimicks it.
    pub unsafe fn deserialize_from_bpf_entrypoint(mut input: *mut u8) -> Self {
        const U64_SIZE: usize = size_of::<u64>();

        let num_accounts = *(input as *const u64) as usize;

        if num_accounts > 32 {
            panic!("max 32 accounts supported in input");
        }

        input = input.add(U64_SIZE);

        let memory = std::alloc::alloc(Layout::new::<[MaybeUninit<Account>; 32]>());
        let accounts = &mut *memory.cast::<[MaybeUninit<Account>; 32]>();

        (0..num_accounts).for_each(|i| {
            let dup_info = *(input as *const u8);
            if dup_info == std::u8::MAX {
                let serialized = &mut *(input as *mut SerializedAccount);
                let data_len = serialized.data_len as usize;
                let data = input.add(size_of::<SerializedAccount>());

                let data_end = data.add(data_len + MAX_PERMITTED_DATA_INCREASE);
                let slack = align_of::<u128>() - data_end as usize % align_of::<u128>();
                let data_end = data_end.add(slack);

                let rent_epoch = *(data_end as *const u64);

                accounts.get_unchecked_mut(i).as_mut_ptr().write(Account {
                    key: &serialized.key,
                    is_signer: serialized.is_signer == 1,
                    is_writable: serialized.is_writable == 1,
                    lamports: &mut serialized.lamports,
                    data_len,
                    data,
                    owner: &serialized.owner,
                    is_executable: serialized.executable == 1,
                    rent_epoch,
                });

                input = data_end.add(U64_SIZE);
            } else {
                panic!("duplicate account inputs are unsupported");
            }
        });

        let data_len = *(input as *const u64) as usize;
        input = input.add(U64_SIZE);

        let data = from_raw_parts(input, data_len);
        let program_id: &Pubkey = &*(input.add(data_len) as *const Pubkey);

        let accounts = ProgramAccounts {
            accounts,
            len: num_accounts,
            cursor: 0,
        };

        ProgramInput {
            program_id,
            accounts,
            data,
        }
    }

    pub fn program_id(&self) -> &'static Pubkey {
        self.program_id
    }

    pub fn data(&self) -> &'static [u8] {
        self.data
    }

    pub fn accounts(&mut self) -> &mut ProgramAccounts {
        &mut self.accounts
    }

    #[inline(always)]
    pub fn take_accounts<const N: usize>(&mut self) -> [AccountRef; N] {
        self.accounts.take_accounts::<N>()
    }

    #[inline(always)]
    pub fn next_account(&mut self) -> AccountRef {
        self.accounts.next_account()
    }

    pub fn remaining(&self) -> usize {
        self.accounts.remaining()
    }

    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}

impl ProgramAccounts {
    #[inline]
    pub fn take_accounts<const N: usize>(&mut self) -> [AccountRef; N] {
        assert!(N > 0);

        if self.cursor + N > self.len {
            panic!("tried to take more accounts than available");
        }

        // NB(mori): we can't intialize the array with meaningful values,
        // so we have to use MaybeUninit as a workaround until we actually write the refs
        const UNINIT: MaybeUninit<AccountRef> = MaybeUninit::uninit();
        let mut array: [MaybeUninit<AccountRef>; N] = [UNINIT; N];
        (0..N).for_each(|i| {
            unsafe {
                // NB(mori): this function can only ever yield one reference to each account,
                // so mutable aliasing will not occur.
                //
                // previous deserialization will ensure that the Account is actually initialized,
                // so we can call `assume_init_mut` here.
                let account_ref =
                    (*self.accounts.as_mut_ptr().add(self.cursor + i)).assume_init_mut();
                array.get_unchecked_mut(i).as_mut_ptr().write(account_ref);
            }
        });

        self.cursor += N;

        // NB(mori): this is safe because all MaybeUninits have been populated with initialized values.
        // transmute via evil ptr casting
        unsafe { array.as_ptr().cast::<[AccountRef; N]>().read() }
    }

    #[inline]
    pub fn next_account(&mut self) -> AccountRef {
        if self.cursor >= self.len {
            panic!("tried to take more accounts than available");
        }

        let account = unsafe { (*self.accounts.as_mut_ptr().add(self.cursor)).assume_init_mut() };

        self.cursor += 1;
        account
    }

    pub fn remaining(&self) -> usize {
        self.len - self.cursor
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == self.cursor
    }
}

pub trait Entrypoint {
    fn call(input: ProgramInput) -> ProgramResult;
}

pub fn wrapped_entrypoint<T: Entrypoint>(
    program_id: &Pubkey,
    account_infos: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if account_infos.len() > MAX_ACCOUNTS {
        panic!("too many accounts");
    }

    let mut accounts_array: [MaybeUninit<Account>; MAX_ACCOUNTS] = MaybeUninit::uninit_array();
    for (i, info) in account_infos.iter().enumerate() {
        unsafe {
            let mut lamports = info.lamports.borrow_mut();
            let lamports = (&mut **lamports) as *mut u64;

            accounts_array[i].as_mut_ptr().write(Account {
                key: info.key,
                lamports,
                data_len: info.data_len(),
                data: info.data.borrow_mut().as_mut_ptr(),
                owner: info.owner,
                rent_epoch: info.rent_epoch,
                is_signer: info.is_signer,
                is_writable: info.is_writable,
                is_executable: info.executable,
            })
        }
    }

    let accounts = ProgramAccounts {
        accounts: unsafe { &mut *(&mut accounts_array as *mut _) },
        len: account_infos.len(),
        cursor: 0,
    };

    let input = ProgramInput {
        program_id: unsafe { &*(program_id as *const Pubkey) },
        accounts,
        data: unsafe { &*(data as *const [u8]) },
    };

    T::call(input)
}

// impl<'a> Account<'a> {
//     pub fn into_account_info(self) -> AccountInfo<'a> {
//         let data = self.data;
//         let lamports = self.lamports;

//         let data = Rc::new(RefCell::new(data));
//         let lamports = Rc::new(RefCell::new(lamports));

//         AccountInfo {
//             key: self.key,
//             is_signer: self.is_signer,
//             is_writable: self.is_writable,
//             lamports,
//             data,
//             owner: self.owner,
//             executable: self.executable,
//             rent_epoch: self.rent_epoch,
//         }
//     }
// }
