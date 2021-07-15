use std::{
    marker::PhantomData,
    mem::size_of,
    mem::{align_of, MaybeUninit},
    ops::{Deref, DerefMut},
    ptr::drop_in_place,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use crate::{
    data::{
        is_valid_for_type, reinterpret_mut_unchecked, reinterpret_slice_mut_unchecked,
        reinterpret_slice_unchecked, reinterpret_unchecked,
    },
    mem::memmove,
};

/// A simple `Vec`-like type for usage as a dynamically-sized container of objects
/// inside Solana accounts.
pub struct SolVec<'a, T> {
    len: &'a mut u64,
    elems: &'a mut [MaybeUninit<T>],
}

pub struct SolSlice<'a, T> {
    len: &'a u64,
    elems: &'a [MaybeUninit<T>],
}

impl<'a, T> AsRef<[T]> for SolSlice<'a, T> {
    fn as_ref(&self) -> &[T] {
        unsafe { from_raw_parts(self.elems.as_ptr().cast(), *self.len as usize) }
    }
}

impl<'a, T> Drop for SolVec<'a, T> {
    fn drop(&mut self) {
        for elem in self.as_mut() {
            unsafe { drop_in_place(elem as *mut T) };
        }
    }
}

impl<'a, T> AsRef<[T]> for SolVec<'a, T> {
    fn as_ref(&self) -> &[T] {
        unsafe { from_raw_parts(self.elems.as_ptr().cast(), *self.len as usize) }
    }
}

impl<'a, T> AsMut<[T]> for SolVec<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        unsafe { from_raw_parts_mut(self.elems.as_mut_ptr().cast(), *self.len as usize) }
    }
}

impl<'a, T> SolSlice<'a, T> {
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

impl<'a, T> SolVec<'a, T> {
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
        assert!((*self.len as usize) < self.elems.len());

        unsafe { self.elems_mut_ptr().add(*self.len as usize).write(elem) };
        *self.len += 1;
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if *self.len > 0 {
            *self.len -= 1;
            let elem = unsafe { self.elems_mut_ptr().add(*self.len as usize).read() };
            Some(elem)
        } else {
            None
        }
    }

    #[inline]
    pub fn remove(&mut self, idx: usize) -> Option<T> {
        if idx < *self.len as usize {
            let elems = self.elems_mut_ptr();
            let elem = unsafe { elems.add(idx).read() };

            let to_move = *self.len as usize - idx - 1;
            if to_move > 1 {
                unsafe {
                    memmove(
                        elems.add(idx + 1).cast(),
                        elems.add(idx).cast(),
                        to_move * size_of::<T>(),
                    )
                }
            }

            Some(elem)
        } else {
            None
        }
    }

    #[inline]
    pub fn insert(&mut self, idx: usize, elem: T) {
        assert!((*self.len as usize) < self.elems.len());
        assert!(idx <= *self.len as usize);

        let to_move = *self.len as usize - idx;
        if to_move > 0 {
            let elems = self.elems_mut_ptr();

            unsafe {
                memmove(
                    elems.add(idx).cast(),
                    elems.add(idx + 1).cast(),
                    to_move * size_of::<T>(),
                )
            }

            unsafe { elems.add(idx).write(elem) }
        } else {
            self.push(elem)
        }
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
