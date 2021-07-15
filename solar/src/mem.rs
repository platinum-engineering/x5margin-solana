#![allow(clippy::missing_safety_doc)]

extern "C" {
    fn sol_memcpy_(dst: *mut u8, src: *const u8, n: u64);
    fn sol_memmove_(dst: *mut u8, src: *const u8, n: u64);
    fn sol_memset_(s: *mut u8, c: u8, n: u64);
    fn sol_memcmp_(a: *const u8, b: *const u8, count: u64, result: *mut i32);
}

pub unsafe fn memcpy(src: *const u8, dst: *mut u8, n: usize) {
    if cfg!(target_arch = "bpf") && cfg!(feature = "sol-mem-intrinsics") {
        sol_memcpy_(dst, src, n as u64);
    } else {
        std::ptr::copy_nonoverlapping(src, dst, n);
    }
}

pub unsafe fn memmove(src: *mut u8, dst: *mut u8, n: usize) {
    if cfg!(target_arch = "bpf") && cfg!(feature = "sol-mem-intrinsics") {
        sol_memmove_(dst, src, n as u64);
    } else {
        std::ptr::copy(src, dst, n);
    }
}

pub unsafe fn memset(dst: *mut u8, value: u8, n: usize) {
    if cfg!(target_arch = "bpf") && cfg!(feature = "sol-mem-intrinsics") {
        sol_memset_(dst, value, n as u64);
    } else {
        std::ptr::write_bytes(dst, value, n);
    }
}

pub unsafe fn memcmp(a: *const u8, b: *const u8, count: usize) -> i32 {
    if cfg!(target_arch = "bpf") && cfg!(feature = "sol-mem-intrinsics") {
        let mut result = 0;
        sol_memcmp_(a, b, count as u64, &mut result as *mut i32);
        result
    } else {
        let mut i = 0;
        while i < count {
            let d: i32 = (*a.add(i) as i32) - (*b.add(i) as i32);
            if d != 0 {
                return d;
            }
            i += 1;
        }
        0
    }
}
