use std::{
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    panic::Location,
};

use borsh::{BorshDeserialize, BorshSerialize};

use num_traits::PrimInt;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, BorshSerialize, BorshDeserialize)]
#[repr(transparent)]
pub struct Checked<T: PrimInt> {
    inner: T,
}

impl<T: PrimInt> Checked<T> {
    pub fn value(self) -> T {
        self.inner
    }
}

impl<T: PrimInt> From<T> for Checked<T> {
    fn from(v: T) -> Self {
        Self { inner: v }
    }
}

// force noinline to prevent code spam
#[inline(never)]
fn handle_overflow<T>(v: Option<T>, location: &Location) -> T {
    match v {
        Some(v) => v,
        None => {
            crate::qlog!(
                location.file(),
                ":",
                location.line(),
                ":",
                location.column(),
                ": arithmetic overflow"
            );
            panic!("arithmetic overflow");
        }
    }
}

macro_rules! overflow_guard {
    ($e:expr) => {
        handle_overflow($e, std::panic::Location::caller());
    };
}

macro_rules! impl_op {
    ($trait:ident, $assign_trait:ident, $method:ident, $assign_method:ident, $impl:expr) => {
        impl<T: PrimInt> $trait for Checked<T> {
            type Output = Self;

            #[track_caller]
            #[inline]
            #[allow(clippy::all)]
            fn $method(self, rhs: Self) -> Self::Output {
                overflow_guard!(($impl)(self.inner, rhs.inner)).into()
            }
        }

        impl<T: PrimInt> $trait<T> for Checked<T> {
            type Output = Self;

            #[track_caller]
            #[inline]
            #[allow(clippy::all)]
            fn $method(self, rhs: T) -> Self::Output {
                overflow_guard!(($impl)(self.inner, rhs)).into()
            }
        }

        impl<T: PrimInt> $assign_trait for Checked<T> {
            #[track_caller]
            #[inline]
            #[allow(clippy::all)]
            fn $assign_method(&mut self, rhs: Self) {
                *self = overflow_guard!(($impl)(self.inner, rhs.inner)).into()
            }
        }

        impl<T: PrimInt> $assign_trait<T> for Checked<T> {
            #[track_caller]
            #[inline]
            #[allow(clippy::all)]
            fn $assign_method(&mut self, rhs: T) {
                *self = overflow_guard!(($impl)(self.inner, rhs)).into()
            }
        }
    };
}

impl_op!(Sub, SubAssign, sub, sub_assign, |a: T, b| a.checked_sub(&b));
impl_op!(Add, AddAssign, add, add_assign, |a: T, b| a.checked_add(&b));
impl_op!(Mul, MulAssign, mul, mul_assign, |a: T, b| a.checked_mul(&b));
impl_op!(Div, DivAssign, div, div_assign, |a: T, b| a.checked_div(&b));

pub struct FixedPoint128 {
    int: u64,
    frac: u64,
}

impl Mul for FixedPoint128 {
    type Output = FixedPoint128;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut result = 0;

        result += ((self.int as u128) * (rhs.int as u128)) << 64;
        result += (self.int as u128) * (rhs.frac as u128);
        result += (self.frac as u128) * (rhs.int as u128);
        result += ((self.frac as u128) * (rhs.frac as u128)) >> 64;

        let int = (result >> 64) as u64;
        let frac = (result << 64 >> 64) as u64;

        Self { int, frac }
    }
}
