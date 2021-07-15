use std::{
    alloc::Layout,
    mem::{align_of, size_of, MaybeUninit},
    slice::{from_raw_parts, from_raw_parts_mut},
};

use solana_program::{entrypoint::MAX_PERMITTED_DATA_INCREASE, pubkey::Pubkey};

use crate::account::onchain::{Account, AccountRef};

pub const MAX_ACCOUNTS: usize = 32;

pub struct ProgramInput {
    program_id: &'static Pubkey,
    accounts: ProgramAccounts,
    data: &'static [u8],
}

pub struct ProgramAccounts {
    accounts: &'static mut [MaybeUninit<Account<'static>>; MAX_ACCOUNTS],
    size: usize,
    cursor: usize,
}

#[repr(C)]
struct SerializedAccount {
    dup_info: u8,
    is_signer: bool,
    is_writable: bool,
    executable: bool,
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
                let data_ptr = input.add(size_of::<SerializedAccount>());
                let data = from_raw_parts_mut(data_ptr, data_len);

                let data_end = data_ptr.add(data_len + MAX_PERMITTED_DATA_INCREASE);
                let slack = align_of::<u128>() - data_end as usize % align_of::<u128>();
                let data_end = data_end.add(slack);

                let rent_epoch = *(data_end as *const u64);

                accounts.get_unchecked_mut(i).as_mut_ptr().write(Account {
                    key: &serialized.key,
                    is_signer: serialized.is_signer,
                    is_writable: serialized.is_writable,
                    lamports: &mut serialized.lamports,
                    data,
                    owner: &serialized.owner,
                    is_executable: serialized.executable,
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
            size: num_accounts,
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
}

impl ProgramAccounts {
    #[inline]
    pub fn take_accounts<const N: usize>(&mut self) -> [AccountRef; N] {
        assert!(N > 0);

        if self.cursor + N > self.size {
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
