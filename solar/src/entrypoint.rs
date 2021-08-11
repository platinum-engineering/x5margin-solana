use std::{alloc::Layout, mem::size_of, ptr::null_mut};

use solana_api_types::entrypoint::{HEAP_LENGTH, HEAP_START_ADDRESS};

#[inline(always)]
pub fn panic_handler(_: &core::panic::PanicInfo) {
    // msg!("{}", info);
}
pub struct BpfAllocator {}

#[allow(clippy::integer_arithmetic)]
unsafe impl std::alloc::GlobalAlloc for BpfAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pos_ptr = HEAP_START_ADDRESS as *mut usize;

        let mut pos = *pos_ptr;
        if pos == 0 {
            pos = HEAP_START_ADDRESS + HEAP_LENGTH;
        }

        pos = pos.saturating_sub(layout.size());
        pos &= !(layout.align().wrapping_sub(1));

        if pos < HEAP_START_ADDRESS + size_of::<*mut u8>() {
            return null_mut();
        }

        *pos_ptr = pos;
        pos as *mut u8
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        //NB(mori): we never free, and the heap provided by the BPF VM is zero-initialized
        self.alloc(layout)
    }

    #[inline]
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}

#[macro_export]
macro_rules! entrypoint {
    ($process_instruction:path) => {
        #[no_mangle]
        pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
            let input =
                unsafe { $crate::input::BpfProgramInput::deserialize_from_bpf_entrypoint(input) };
            match $process_instruction(input) {
                Ok(()) => solana_program::entrypoint::SUCCESS,
                Err(error) => error.into(),
            }
        }

        #[global_allocator]
        static A: $crate::entrypoint::BpfAllocator = $crate::entrypoint::BpfAllocator {};

        #[no_mangle]
        fn custom_panic(info: &core::panic::PanicInfo<'_>) {
            $crate::entrypoint::panic_handler(info);
        }
    };
}
