use solana_program::pubkey::Pubkey;

use crate::mem::memcmp;

#[macro_export]
macro_rules! bytecode_marker {
    ($name:ident) => {{
        #[inline(never)]
        fn $name() {
            solana_program::msg!(stringify!($name));
        }
        $name();
    }};
}

pub trait ToPubkey {
    fn to_pubkey(&self) -> Pubkey;
}

pub trait AsPubkey {
    fn as_pubkey(&self) -> &Pubkey;
}

impl ToPubkey for Pubkey {
    #[inline(always)]
    fn to_pubkey(&self) -> Pubkey {
        *self
    }
}

impl ToPubkey for &Pubkey {
    #[inline(always)]
    fn to_pubkey(&self) -> Pubkey {
        **self
    }
}

impl AsPubkey for Pubkey {
    #[inline(always)]
    fn as_pubkey(&self) -> &Pubkey {
        self
    }
}

impl AsPubkey for &Pubkey {
    #[inline(always)]
    fn as_pubkey(&self) -> &Pubkey {
        self
    }
}

#[inline(always)]
pub fn pubkey_eq<A: AsPubkey, B: AsPubkey>(a: A, b: B) -> bool {
    unsafe {
        memcmp(
            a.as_pubkey().as_ref().as_ptr(),
            b.as_pubkey().as_ref().as_ptr(),
            32,
        ) == 0
    }
}
