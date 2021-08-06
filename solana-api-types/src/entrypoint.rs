pub const MAX_PERMITTED_DATA_INCREASE: usize = 1_024 * 10; // 0x0000_0000_0000_2800usize

/// Programs indicate success with a return value of 0
pub const SUCCESS: u64 = 0;

/// Start address of the memory region used for program heap.
pub const HEAP_START_ADDRESS: u64 = 0x300000000;
/// Length of the heap memory region used for program heap.
pub const HEAP_LENGTH: u64 = 32 * 1024;
