use serde::{Deserialize, Serialize};

use crate::{Epoch, Slot, UnixTimestamp};

/// Clock represents network time.  Members of Clock start from 0 upon
///  network boot.  The best way to map Clock to wallclock time is to use
///  current Slot, as Epochs vary in duration (they start short and grow
///  as the network progresses).
///
#[repr(C)]
#[derive(Serialize, Clone, Deserialize, Debug, Default, PartialEq)]
pub struct Clock {
    /// the current network/bank Slot
    pub slot: Slot,
    /// the timestamp of the first Slot in this Epoch
    pub epoch_start_timestamp: UnixTimestamp,
    /// the bank Epoch
    pub epoch: Epoch,
    /// the future Epoch for which the leader schedule has
    ///  most recently been calculated
    pub leader_schedule_epoch: Epoch,
    /// originally computed from genesis creation time and network time
    /// in slots (drifty); corrected using validator timestamp oracle as of
    /// timestamp_correction and timestamp_bounding features
    pub unix_timestamp: UnixTimestamp,
}

use crate::{impl_sysvar_get, program::ProgramError, sysvar::Sysvar};

crate::declare_sysvar_id!("SysvarC1ock11111111111111111111111111111111", Clock);

impl Sysvar for Clock {
    impl_sysvar_get!(sol_get_clock_sysvar);
}
