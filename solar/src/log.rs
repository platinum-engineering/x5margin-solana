//! Minimal-overhead logging facilities for on-chain BPF programs.
//!
//! This is intended to be used as a light-weight alternative for formatting and logging that generates
//! less bytecode and uses fewer instructions than `std::fmt`.
//!
//! The logger in this module allocates buffer space for the formatted output directly on the stack,
//! without using the heap or zero-initializing the memory. Allocating memory in this way is effectively free,
//! though limited to BPF's stack size restrictions.

use std::{mem::MaybeUninit, slice::from_raw_parts};

use itoap::write_to_ptr;

use crate::mem::memcpy;

pub struct Logger<const S: usize> {
    buf: MaybeUninit<[u8; S]>,
    cursor: usize,
}

extern "C" {
    fn sol_log_(src: *const u8, len: u64);
}

impl<const S: usize> Logger<S> {
    pub fn push_str(&mut self, s: &str) {
        assert!(self.cursor + s.len() <= S);

        unsafe {
            memcpy(
                s.as_ptr(),
                self.buf.as_mut_ptr().add(self.cursor).cast(),
                s.len(),
            )
        }

        self.cursor += s.len();
    }

    pub fn push_int<I: itoap::Integer>(&mut self, i: I) {
        assert!(self.cursor + I::MAX_LEN <= S);

        self.cursor += unsafe { write_to_ptr(self.buf.as_mut_ptr().add(self.cursor).cast(), i) };
    }

    pub fn log(&self) {
        if cfg!(target_arch = "bpf") {
            unsafe {
                sol_log_(self.buf.as_ptr().cast(), self.cursor as u64);
            }
        } else {
            let buf = unsafe { from_raw_parts(self.buf.as_ptr().cast::<u8>(), self.cursor) };
            let output = String::from_utf8_lossy(buf);
            println!("Program log: {}", output);
        }
    }
}

pub trait Loggable {
    fn push_to_logger<const S: usize>(&self, logger: &mut Logger<S>);
}

impl Loggable for str {
    fn push_to_logger<const S: usize>(&self, logger: &mut Logger<S>) {
        logger.push_str(self)
    }
}

impl Loggable for &str {
    fn push_to_logger<const S: usize>(&self, logger: &mut Logger<S>) {
        logger.push_str(self)
    }
}

impl<const S: usize> Default for Logger<S> {
    fn default() -> Self {
        Logger {
            buf: MaybeUninit::uninit(),
            cursor: 0,
        }
    }
}

#[macro_export]
macro_rules! qlog {
    (@$buf_size:expr, $($item:expr),+) => {
        let mut logger = $crate::log::Logger::<$buf_size>::default();
        $(
            $crate::log::Loggable::push_to_logger(&$item, &mut logger);
        )+
        logger.log();
    };

    ($($item:expr),+) => {
        $crate::qlog!(@256, $($item),+)
    }
}

macro_rules! impl_loggable_int {
    ($($i:ty),+) => {
        $(
            impl Loggable for $i {
                fn push_to_logger<const S: usize>(&self, logger: &mut Logger<S>) {
                    logger.push_int(*self)
                }
            }
        )+
    };
}

impl_loggable_int!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, usize, isize);
