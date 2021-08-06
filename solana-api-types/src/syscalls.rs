use crate::program::UNSUPPORTED_SYSVAR;

pub fn sol_log(message: &str) {
    println!("{}", message);
}

pub fn sol_get_clock_sysvar(_var_addr: *mut u8) -> u64 {
    UNSUPPORTED_SYSVAR
}
