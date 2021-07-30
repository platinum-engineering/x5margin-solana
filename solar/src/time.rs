use chrono::{DateTime, Duration, TimeZone, Utc};

use borsh::{BorshDeserialize, BorshSerialize};

#[repr(C)]
#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize,
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

impl From<SolTimestamp> for DateTime<Utc> {
    fn from(v: SolTimestamp) -> Self {
        Utc.timestamp(v.ts, 0)
    }
}

impl From<DateTime<Utc>> for SolTimestamp {
    fn from(time: DateTime<Utc>) -> Self {
        Self {
            ts: time.timestamp(),
        }
    }
}

#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize,
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

impl From<Duration> for SolDuration {
    fn from(duration: Duration) -> Self {
        Self {
            value: duration.num_seconds(),
        }
    }
}

impl From<SolDuration> for Duration {
    fn from(duration: SolDuration) -> Self {
        Duration::seconds(duration.value)
    }
}
