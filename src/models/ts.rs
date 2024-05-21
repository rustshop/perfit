use std::ops::{self, Add};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Encode, Decode, Serialize, Deserialize, Debug, Copy, Clone, Default)]
pub struct Ts(u64);

impl From<time::OffsetDateTime> for Ts {
    fn from(value: time::OffsetDateTime) -> Self {
        Ts(value.unix_timestamp() as u64)
    }
}

impl From<SystemTime> for Ts {
    fn from(value: SystemTime) -> Self {
        value
            .duration_since(UNIX_EPOCH)
            .map(|d| Ts(d.as_secs()))
            .unwrap_or_default()
    }
}

impl From<time::PrimitiveDateTime> for Ts {
    fn from(value: time::PrimitiveDateTime) -> Self {
        Self::from(OffsetDateTime::new_utc(value.date(), value.time()))
    }
}

impl ops::Sub for Ts {
    type Output = u64;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0.saturating_sub(rhs.0)
    }
}

impl Ts {
    pub const ZERO: Self = Ts(0);
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
    }

    pub fn to_absolute_secs(self) -> u64 {
        self.0
    }

    pub fn to_datetime(self) -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp(self.to_absolute_secs() as i64)
            .expect("can't fail")
    }

    pub fn inc(self) -> Ts {
        Self(self.0 + 1)
    }
}

pub trait DateTimeExt {
    fn round_down_to_hour(self) -> Self;
    fn round_up_to_hour(self) -> Self;
    fn round_up_exclusive_to_hour(self) -> Self;

    fn our_fmt(self) -> String;
}

impl DateTimeExt for time::OffsetDateTime {
    fn our_fmt(self) -> String {
        format!(
            "{}T{:02}:{:02}:{:02}Z",
            self.date(),
            self.hour(),
            self.minute(),
            self.second()
        )
    }
    fn round_down_to_hour(self) -> Self {
        let (h, _m, _s) = self.to_hms();

        time::OffsetDateTime::new_utc(
            self.date(),
            time::Time::from_hms(h, 0, 0).expect("Can't fail"),
        )
    }

    fn round_up_to_hour(self) -> Self {
        self.add(Duration::from_secs(60 * 60 - 1))
            .round_down_to_hour()
    }
    fn round_up_exclusive_to_hour(self) -> Self {
        self.add(Duration::from_secs(60 * 60)).round_down_to_hour()
    }
}
