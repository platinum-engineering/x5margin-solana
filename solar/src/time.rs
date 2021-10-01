#[repr(C)]
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    parity_scale_codec::Encode,
    parity_scale_codec::Decode,
)]
pub struct SolTimestamp {
    ts: i64,
}

impl From<i64> for SolTimestamp {
    fn from(ts: i64) -> Self {
        Self { ts }
    }
}

impl From<SolTimestamp> for i64 {
    fn from(v: SolTimestamp) -> Self {
        v.ts
    }
}

#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    parity_scale_codec::Encode,
    parity_scale_codec::Decode,
)]
pub struct SolDuration {
    value: i64,
}

impl From<i64> for SolDuration {
    fn from(value: i64) -> Self {
        Self { value }
    }
}

impl From<SolDuration> for i64 {
    fn from(v: SolDuration) -> Self {
        v.value
    }
}
