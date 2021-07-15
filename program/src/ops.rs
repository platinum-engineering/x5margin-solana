use num_enum::{FromPrimitive, IntoPrimitive};

// pub mod create_farm;
// pub mod poll;
// pub mod stake;

#[repr(u16)]
#[derive(Eq, PartialEq, IntoPrimitive, FromPrimitive)]
pub enum OperationKind {
    /*
        Service ops.
    */
    Poll = 0x00_00,
    CreateFarm = 0x00_01,

    /*
        Business operations ops.
    */
    Stake = 0x01_00,
    Unstake = 0x01_01,
    ClaimRewards = 0x01_02,

    #[num_enum(default)]
    Unknown = 0xFF_FF,
}

#[repr(C)]
pub struct OperationHeader {
    pub kind: OperationKind,
    pub version: u16,
    padding: u32,
}

#[repr(C)]
struct OperationHeaderRaw {
    kind: u16,
    version: u16,
    padding: u32,
}

pub struct Operation<'a> {
    pub header: &'a OperationHeader,
    pub data: &'a [u8],
}

impl<'a> Operation<'a> {
    pub fn from_buf(data: &'a [u8]) -> Self {
        unsafe {
            assert!(data.len() >= 4, "operation data must be at least 4 bytes");

            let raw_header_ptr = data.as_ptr().cast::<OperationHeaderRaw>();
            let raw_header = &*raw_header_ptr;

            assert!(
                OperationKind::from_primitive(raw_header.kind) != OperationKind::Unknown,
                "must provide valid operation id"
            );

            let header = &*raw_header_ptr.cast::<OperationHeader>();
            let data = &data[4..];

            Operation { header, data }
        }
    }
}
