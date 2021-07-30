//! Helper functions for reinterpreting byte slices as objects.
//!
//! # Safety
//!
//! Ensure that all invariants of the reinterpreted-as type are upheld *before* a reference
//! is obtained. Rust's std docs and the Rustonomicon are the best sources on which invariants
//! need to be upheld, but a short list:  
//! * The reinterpreted bytes must be a valid representation of the object. This means it must not contain
//! invalid values for any of the object's fields, for which there are invalid memory representations, such as
//! bools, references, etc.
//! * The data must be initialized.
//! * The reinterpreted object *should* be `#[repr(C)]` or `#[repr(packed)]` or a primitive type with a trivial
//! memory representation, because Rust's ABI is unstable and may change between compiler versions.
//! * The data must be properly aligned to the reinterpreted object's alignment requirements.
//!
//! The intended use case of these functions is to be used on `#[repr(C)]` or `#[repr(packed)]` structs which contain
//! no references and no other complex types with strict invariants. Using them on anything else is a *bad idea*.

use std::{
    mem::align_of,
    mem::size_of,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use solana_program::pubkey::Pubkey;

/// Checks if the given byte slice satisfies the size and alignment requirements of the target type `T`.
pub fn is_valid_for_type<T>(data: &[u8]) -> bool {
    data.len() >= size_of::<T>() && (data.as_ptr() as usize) % align_of::<T>() == 0
}

/// Reinterpret a slice of bytes as an object.
///
/// The slice can be bigger than necessary.
///
/// # Safety
/// `data` must be a valid representation of object `T`
pub unsafe fn try_reinterpret<T>(data: &[u8]) -> Option<&T> {
    if is_valid_for_type::<T>(data) {
        Some(reinterpret_unchecked(data))
    } else {
        None
    }
}

/// Reinterpret a slice of bytes as an object.
///
/// The slice can be bigger than necessary.
///
/// # Safety
/// `data` must be a valid representation of object `T`
///
/// Unlike [`try_reinterpret`], this function does not check size or alignment.
pub unsafe fn reinterpret_unchecked<T>(data: &[u8]) -> &T {
    &*(data.as_ptr() as *const T)
}

/// Reinterpret a slice of bytes as an object.
///
/// The slice can be bigger than necessary.
///
/// # Safety
/// `data` must be a valid representation of object `T`
pub unsafe fn try_reinterpret_mut<T>(data: &mut [u8]) -> Option<&mut T> {
    if is_valid_for_type::<T>(data) {
        Some(reinterpret_mut_unchecked(data))
    } else {
        None
    }
}

/// Reinterpret a slice of bytes as an object.
///
/// The slice can be bigger than necessary.
///
/// # Safety
/// `data` must be a valid representation of object `T`
///
/// Unlike [`try_reinterpret_mut`], this function does not check size or alignment.
pub unsafe fn reinterpret_mut_unchecked<T>(data: &mut [u8]) -> &mut T {
    &mut *(data.as_mut_ptr() as *mut T)
}

/// Reinterpret a slice of bytes as a slice of objects.
///
/// If the slice isn't divisible into a whole number of `T` objects, extra bytes will be ignored.
///
/// # Safety
/// `data` must be a valid representation of some number of objects `T`
pub unsafe fn try_reinterpret_slice<T>(data: &[u8]) -> Option<&[T]> {
    if is_valid_for_type::<T>(data) {
        Some(reinterpret_slice_unchecked(data))
    } else {
        None
    }
}

/// Reinterpret a slice of bytes as a slice of objects.
///
/// If the slice isn't divisible into a whole number of `T` objects, extra bytes will be ignored.
///
/// # Safety
/// `data` must be a valid representation of some number of objects `T`
///
/// Unlike [`try_reinterpret_slice`], this function does not check for alignment or size.
pub unsafe fn reinterpret_slice_unchecked<T>(data: &[u8]) -> &[T] {
    let count = data.len() / size_of::<T>();

    from_raw_parts(data.as_ptr() as *const T, count)
}

/// Reinterpret a slice of bytes as a slice of objects.
///
/// If the slice isn't divisible into a whole number of `T` objects, extra bytes will be ignored.
///
/// # Safety
/// `data` must be a valid representation of some number of objects `T`
pub unsafe fn try_reinterpret_slice_mut<T>(data: &mut [u8]) -> Option<&mut [T]> {
    if is_valid_for_type::<T>(data) {
        Some(reinterpret_slice_mut_unchecked(data))
    } else {
        None
    }
}

/// Reinterpret a slice of bytes as a slice of objects.
///
/// If the slice isn't divisible into a whole number of `T` objects, extra bytes will be ignored.
///
/// # Safety
/// `data` must be a valid representation of some number of objects `T`
///
/// Unlike [`try_reinterpret_slice_mut`], this function does not check for alignment or size.
pub unsafe fn reinterpret_slice_mut_unchecked<T>(data: &mut [u8]) -> &mut [T] {
    let count = data.len() / size_of::<T>();

    from_raw_parts_mut(data.as_mut_ptr() as *mut T, count)
}

pub fn as_bytes<T>(value: &T) -> &[u8] {
    unsafe { from_raw_parts(value as *const _ as *const u8, size_of::<T>()) }
}

pub unsafe trait ReinterpretSafe {}

macro_rules! impl_reinterpret_safe {
    ($($t:ty),*) => {
        $(
            unsafe impl ReinterpretSafe for $t {}
        )*
    };
}

impl_reinterpret_safe! {
    i8, i16, i32, i64, i128,
    u8, u16, u32, u64, u128,
    Pubkey
}

unsafe impl<T: ReinterpretSafe, const N: usize> ReinterpretSafe for [T; N] {}
