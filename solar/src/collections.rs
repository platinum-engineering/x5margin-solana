use std::{
    io::{ErrorKind, Write},
    marker::PhantomData,
    mem::size_of,
    mem::{align_of, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr::drop_in_place,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use crate::{
    mem::memmove,
    reinterpret::{
        is_valid_for_type, reinterpret_mut_unchecked, reinterpret_slice_mut_unchecked,
        reinterpret_slice_unchecked, reinterpret_unchecked,
    },
};

/// A simple `Vec`-like type for usage as a dynamically-sized container of objects
/// inside Solana accounts.
pub struct VecViewMut<'a, T> {
    len: &'a mut u64,
    elems: &'a mut [MaybeUninit<T>],
}

pub struct VecView<'a, T> {
    len: &'a u64,
    elems: &'a [MaybeUninit<T>],
}

impl<'a, T> Deref for VecView<'a, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { from_raw_parts(self.elems.as_ptr().cast(), *self.len as usize) }
    }
}

impl<'a, T> Drop for VecViewMut<'a, T> {
    fn drop(&mut self) {
        for elem in self.as_mut() {
            unsafe { drop_in_place(elem as *mut T) };
        }
    }
}

impl<'a, T> Deref for VecViewMut<'a, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { from_raw_parts(self.elems.as_ptr().cast(), *self.len as usize) }
    }
}

impl<'a, T> DerefMut for VecViewMut<'a, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { from_raw_parts_mut(self.elems.as_mut_ptr().cast(), *self.len as usize) }
    }
}

impl<'a, T> VecView<'a, T> {
    pub fn load(data: &'a [u8]) -> Option<Self> {
        // must have valid alignment for T and VecData
        // and must be able to hold at least 1 `T`
        if !is_valid_for_type::<T>(data)
            || !is_valid_for_type::<u64>(data)
            || data.len() < size_of::<u64>() + size_of::<T>()
        {
            None
        } else {
            let (len, elems) = data.split_at(size_of::<u64>());

            let len = unsafe { reinterpret_unchecked::<u64>(len) };
            let elems = unsafe { reinterpret_slice_unchecked::<MaybeUninit<T>>(elems) };

            if elems.len() < *len as usize {
                return None;
            }

            Some(Self { len, elems })
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.elems.len()
    }
}

#[inline]
unsafe fn vec_like_push<T>(len: &mut u64, capacity: usize, elems: *mut T, elem: T) {
    assert!((*len as usize) < capacity);
    elems.add(*len as usize).write(elem);
    *len = len.checked_add(1).expect("integer overflow");
}

#[inline]
unsafe fn vec_like_pop<T>(len: &mut u64, elems: *mut T) -> Option<T> {
    if *len > 0 {
        *len = len.checked_sub(1).expect("integer underflow");
        let elem = elems.add(*len as usize).read();
        Some(elem)
    } else {
        None
    }
}

#[inline]
unsafe fn vec_like_remove<T>(len: &mut u64, elems: *mut T, idx: usize) -> Option<T> {
    if idx < *len as usize {
        let elem = elems.add(idx).read();

        let to_move = *len as usize - idx - 1;
        if to_move > 1 {
            memmove(
                elems.add(idx + 1).cast(),
                elems.add(idx).cast(),
                to_move * size_of::<T>(),
            )
        }

        *len = len.checked_sub(1).expect("integer underflow");

        Some(elem)
    } else {
        None
    }
}

#[inline]
unsafe fn vec_like_insert<T>(len: &mut u64, capacity: usize, elems: *mut T, idx: usize, elem: T) {
    assert!((*len as usize) < capacity);
    assert!(idx <= *len as usize);

    let to_move = *len as usize - idx;
    if to_move > 0 {
        memmove(
            elems.add(idx).cast(),
            elems.add(idx + 1).cast(),
            to_move * size_of::<T>(),
        );

        elems.add(idx).write(elem);

        *len = len.checked_add(1).expect("integer overflow");
    } else {
        vec_like_push(len, capacity, elems, elem)
    }
}

impl<'a, T> VecViewMut<'a, T> {
    pub fn load(data: &'a mut [u8]) -> Option<Self> {
        // must have valid alignment for T and VecData
        // and must be able to hold at least 1 `T`
        if !is_valid_for_type::<T>(data)
            || !is_valid_for_type::<u64>(data)
            || data.len() < size_of::<u64>() + size_of::<T>()
        {
            None
        } else {
            let (len, elems) = data.split_at_mut(size_of::<u64>());

            let len = unsafe { reinterpret_mut_unchecked::<u64>(len) };
            let elems = unsafe { reinterpret_slice_mut_unchecked::<MaybeUninit<T>>(elems) };

            if elems.len() < *len as usize {
                return None;
            }

            Some(Self { len, elems })
        }
    }

    #[inline]
    fn elems_mut_ptr(&mut self) -> *mut T {
        self.elems.as_mut_ptr().cast()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.elems.len()
    }

    #[inline]
    pub fn push(&mut self, elem: T) {
        let elems = self.elems_mut_ptr();
        unsafe { vec_like_push(&mut self.len, self.elems.len(), elems, elem) }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        let elems = self.elems_mut_ptr();
        unsafe { vec_like_pop(&mut self.len, elems) }
    }

    #[inline]
    pub fn remove(&mut self, idx: usize) -> Option<T> {
        let elems = self.elems_mut_ptr();
        unsafe { vec_like_remove(&mut self.len, elems, idx) }
    }

    #[inline]
    pub fn insert(&mut self, idx: usize, elem: T) {
        let elems = self.elems_mut_ptr();
        unsafe { vec_like_insert(&mut self.len, self.elems.len(), elems, idx, elem) }
    }
}

#[repr(C)]
pub struct SolCell<T, const S: usize> {
    is_initialized: bool,
    data: MaybeUninit<[u8; S]>,
    _phantom: PhantomData<T>,
}

impl<T, const S: usize> Default for SolCell<T, S> {
    fn default() -> Self {
        Self {
            is_initialized: false,
            data: MaybeUninit::zeroed(),
            _phantom: PhantomData::default(),
        }
    }
}

impl<T, const S: usize> Drop for SolCell<T, S> {
    fn drop(&mut self) {
        if self.is_initialized {
            Self::take(self);
        }
    }
}

impl<T, const S: usize> SolCell<T, S> {
    pub fn put(dest: &mut Self, object: T) {
        if dest.is_initialized {
            Self::take(dest);
        }

        let ptr = dest.data.as_mut_ptr().cast::<T>();
        assert!(ptr as usize % align_of::<T>() == 0);

        unsafe {
            ptr.write(object);
        }

        dest.is_initialized = true;
    }

    pub fn take(src: &mut Self) -> T {
        let ptr = src.data.as_mut_ptr().cast::<T>();
        assert!(src.is_initialized);
        assert!(ptr as usize % align_of::<T>() == 0);
        src.is_initialized = false;
        unsafe { ptr.read() }
    }
}

impl<T, const S: usize> Deref for SolCell<T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        assert!(size_of::<T>() <= S);
        assert!(self.is_initialized);

        unsafe { &*self.data.as_ptr().cast() }
    }
}

impl<T, const S: usize> DerefMut for SolCell<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        assert!(size_of::<T>() <= S);
        assert!(self.is_initialized);

        unsafe { &mut *self.data.as_mut_ptr().cast() }
    }
}

pub struct StaticVec<T, const N: usize> {
    elems: [MaybeUninit<T>; N],
    len: u64,
}

impl<T, const N: usize> Default for StaticVec<T, N> {
    fn default() -> Self {
        Self {
            elems: MaybeUninit::uninit_array(),
            len: 0,
        }
    }
}

impl<T, const N: usize> Deref for StaticVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { from_raw_parts(self.elems.as_ptr().cast(), self.len as usize) }
    }
}

impl<T, const N: usize> DerefMut for StaticVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { from_raw_parts_mut(self.elems.as_mut_ptr().cast(), self.len as usize) }
    }
}

impl<T, const N: usize> StaticVec<T, N> {
    #[inline]
    fn elems_mut_ptr(&mut self) -> *mut T {
        self.elems.as_mut_ptr().cast()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.elems.len()
    }

    #[inline]
    pub fn push(&mut self, elem: T) {
        let elems = self.elems_mut_ptr();
        unsafe { vec_like_push(&mut self.len, self.elems.len(), elems, elem) }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        let elems = self.elems_mut_ptr();
        unsafe { vec_like_pop(&mut self.len, elems) }
    }

    #[inline]
    pub fn remove(&mut self, idx: usize) -> Option<T> {
        let elems = self.elems_mut_ptr();
        unsafe { vec_like_remove(&mut self.len, elems, idx) }
    }

    #[inline]
    pub fn insert(&mut self, idx: usize, elem: T) {
        let elems = self.elems_mut_ptr();
        unsafe { vec_like_insert(&mut self.len, self.elems.len(), elems, idx, elem) }
    }
}

impl<const N: usize> Write for StaticVec<u8, N> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let writable = &buf[..self.capacity() - self.len()];

        if !writable.is_empty() {
            unsafe {
                self.elems_mut_ptr()
                    .add(self.len())
                    .copy_from_nonoverlapping(writable.as_ptr(), writable.len())
            }
        }

        Ok(writable.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if buf.len() <= (self.capacity() - self.len()) {
            unsafe {
                self.elems_mut_ptr()
                    .add(self.len())
                    .copy_from_nonoverlapping(buf.as_ptr(), buf.len())
            }

            Ok(())
        } else {
            Err(ErrorKind::Interrupted.into())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
