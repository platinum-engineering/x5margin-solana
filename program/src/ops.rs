use borsh::BorshDeserialize;
use num_enum::{FromPrimitive, IntoPrimitive};
use solar::util::ResultExt;

trait Method {
    const ID: MethodId;
}

#[repr(u16)]
#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    IntoPrimitive,
    FromPrimitive,
    BorshSerialize,
    BorshDeserialize,
)]

pub enum MethodId {
    CreateStakePool,

    #[num_enum(default)]
    Unknown = 0xFF_FF,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct InvokeHeader {
    pub id: MethodId,
    pub version: u16,
    padding: u32,
}

pub struct Invoke<'a> {
    pub header: InvokeHeader,
    pub data: &'a [u8],
}

impl<'a> Invoke<'a> {
    pub fn from_buf(mut data: &'a [u8]) -> Self {
        let header = InvokeHeader::deserialize(&mut data)
            .ok()
            .bpf_expect("invalid operation");

        Self { header, data }
    }

    pub fn id(&self) -> MethodId {
        self.header.id
    }

    pub fn version(&self) -> u16 {
        self.header.version
    }

    pub fn args<T: BorshDeserialize>(&self) -> Option<T> {
        T::try_from_slice(self.data).ok()
    }
}
