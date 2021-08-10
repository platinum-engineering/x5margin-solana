use std::{
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    panic::Location,
};

use az::CheckedCast;

use fixed::{
    traits::{Fixed, ToFixed},
    types::{I64F64, U64F64},
};
use num_traits::{
    Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedNeg, CheckedRem, CheckedShl, CheckedShr,
    CheckedSub, One, Zero,
};

pub trait CheckedNum:
    Bounded
    + Zero
    + One
    + CheckedAdd
    + CheckedMul
    + CheckedDiv
    + CheckedNeg
    + CheckedRem
    + CheckedShl
    + CheckedShr
    + CheckedSub
    + Copy
    + Clone
{
}

impl<
        T: Bounded
            + Zero
            + One
            + CheckedAdd
            + CheckedMul
            + CheckedDiv
            + CheckedNeg
            + CheckedRem
            + CheckedShl
            + CheckedShr
            + CheckedSub
            + Copy
            + Clone,
    > CheckedNum for T
{
}

pub trait ToChecked: CheckedNum {
    fn to_checked(self) -> Checked<Self>;
}

impl<T: CheckedNum> ToChecked for T {
    fn to_checked(self) -> Checked<Self> {
        Checked { inner: self }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    minicbor::Encode,
    minicbor::Decode,
    parity_scale_codec::Encode,
    parity_scale_codec::Decode,
)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Checked<T: CheckedNum> {
    #[n(0)]
    inner: T,
}

impl<T: CheckedNum> Checked<T> {
    pub fn value(self) -> T {
        self.inner
    }
}

impl<T: CheckedNum> From<T> for Checked<T> {
    fn from(v: T) -> Self {
        Self { inner: v }
    }
}

impl<T, U> CheckedCast<Checked<U>> for Checked<T>
where
    T: Fixed + CheckedNum,
    U: CheckedNum,
    T: CheckedCast<U>,
{
    fn checked_cast(self) -> Option<Checked<U>> {
        self.value().checked_cast().map(Checked::from)
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
        handle_overflow($e, std::panic::Location::caller())
    };
}

macro_rules! impl_op {
    ($trait:ident, $assign_trait:ident, $method:ident, $assign_method:ident, $impl:expr) => {
        impl<T: CheckedNum> $trait for Checked<T> {
            type Output = Self;

            #[track_caller]
            #[inline]
            #[allow(clippy::all)]
            fn $method(self, rhs: Self) -> Self::Output {
                overflow_guard!(($impl)(self.inner, rhs.inner)).into()
            }
        }

        impl<T: CheckedNum> $trait<T> for Checked<T> {
            type Output = Self;

            #[track_caller]
            #[inline]
            #[allow(clippy::all)]
            fn $method(self, rhs: T) -> Self::Output {
                overflow_guard!(($impl)(self.inner, rhs)).into()
            }
        }

        impl<T: CheckedNum> $assign_trait for Checked<T> {
            #[track_caller]
            #[inline]
            #[allow(clippy::all)]
            fn $assign_method(&mut self, rhs: Self) {
                *self = overflow_guard!(($impl)(self.inner, rhs.inner)).into()
            }
        }

        impl<T: CheckedNum> $assign_trait<T> for Checked<T> {
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

impl<T: CheckedNum> Neg for Checked<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        overflow_guard!(self.inner.checked_neg()).into()
    }
}

impl<T: ToFixed + CheckedNum> ToFixed for Checked<T> {
    fn to_fixed<F: fixed::traits::Fixed>(self) -> F {
        self.inner.to_fixed()
    }

    fn checked_to_fixed<F: fixed::traits::Fixed>(self) -> Option<F> {
        self.inner.checked_to_fixed()
    }

    fn saturating_to_fixed<F: fixed::traits::Fixed>(self) -> F {
        self.inner.saturating_to_fixed()
    }

    fn wrapping_to_fixed<F: fixed::traits::Fixed>(self) -> F {
        self.inner.wrapping_to_fixed()
    }

    fn overflowing_to_fixed<F: fixed::traits::Fixed>(self) -> (F, bool) {
        self.inner.overflowing_to_fixed()
    }
}

pub trait ToF64 {
    fn to_u64f64(self) -> Checked<U64F64>;
    fn to_i64f64(self) -> Checked<I64F64>;
}

impl<T: ToFixed> ToF64 for T {
    #[inline]
    #[track_caller]
    fn to_u64f64(self) -> Checked<U64F64> {
        overflow_guard!(self.checked_to_fixed::<U64F64>()).into()
    }

    #[inline]
    #[track_caller]
    fn to_i64f64(self) -> Checked<I64F64> {
        overflow_guard!(self.checked_to_fixed::<I64F64>()).into()
    }
}
