use std::time::{Duration, SystemTime};

use bytemuck::{Pod, TransparentWrapper, Zeroable};

pub use sys::generated_safe::AccountFilterFlags as Flags;
pub use sys::tb_account_filter_t as Raw;

#[repr(transparent)]
#[derive(Clone, Copy, TransparentWrapper, Pod, Zeroable)]
pub struct Filter(Raw);

impl std::fmt::Debug for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountFilter")
            .field("account_id", &self.0.account_id)
            .field("timestamp_min", &self.timestamp_min())
            .field("timestamp_max", &self.timestamp_max())
            .field("limit", &self.0.limit)
            .field("flags", &self.flags())
            .finish_non_exhaustive()
    }
}

impl Filter {
    #[track_caller]
    pub fn new(account_id: u128, limit: u32) -> Self {
        Filter(Raw::zeroed())
            .with_account_id(account_id)
            .with_limit(limit)
    }

    pub const fn from_raw(raw: Raw) -> Self {
        Filter(raw)
    }
    pub const fn into_raw(self) -> Raw {
        self.0
    }
    pub const fn as_raw(&self) -> &Raw {
        &self.0
    }
    pub fn as_raw_mut(&mut self) -> &mut Raw {
        &mut self.0
    }

    pub const fn account_id(&self) -> u128 {
        self.0.account_id
    }
    pub fn set_account_id(&mut self, account_id: u128) {
        assert_ne!(account_id, u128::MAX, "account_id must not be `2^128 - 1`");
        self.0.account_id = account_id;
    }
    pub const fn with_account_id(mut self, account_id: u128) -> Self {
        self.0.account_id = account_id;
        self
    }

    pub fn timestamp_min(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_nanos(self.0.timestamp_min)
    }
    pub fn set_timestamp_min(&mut self, timestamp_min: SystemTime) {
        let t = timestamp_min
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .and_then(|t| t.as_nanos().try_into().ok())
            .expect("failed to get nanoseconds since unix epoch from the argument");
        assert_ne!(t, u64::MAX, "timestamp_min must not be `2^64 - 1`");
        self.0.timestamp_min = t;
    }
    pub fn with_timestamp_min(mut self, timestamp_min: SystemTime) -> Self {
        self.set_timestamp_min(timestamp_min);
        self
    }

    pub fn timestamp_max(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_nanos(self.0.timestamp_max)
    }
    pub fn set_timestamp_max(&mut self, timestamp_max: SystemTime) {
        let t = timestamp_max
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .and_then(|t| t.as_nanos().try_into().ok())
            .expect("failed to get nanoseconds since unix epoch from the argument");
        assert_ne!(t, u64::MAX, "timestamp_max must not be `2^64 - 1`");
        self.0.timestamp_max = t;
    }
    pub fn with_timestamp_max(mut self, timestamp_max: SystemTime) -> Self {
        self.set_timestamp_max(timestamp_max);
        self
    }

    pub const fn limit(&self) -> u32 {
        self.0.limit
    }
    pub fn set_limit(&mut self, limit: u32) {
        assert_ne!(limit, 0, "limit must not be zero");
        self.0.limit = limit;
    }
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.set_limit(limit);
        self
    }

    pub const fn flags(&self) -> Flags {
        Flags::from_bits_retain(self.0.flags)
    }
    pub fn set_flags(&mut self, flags: Flags) {
        self.0.flags = flags.bits();
    }
    pub const fn with_flags(mut self, flags: Flags) -> Self {
        self.0.flags = flags.bits();
        self
    }
}

impl From<Raw> for Filter {
    fn from(value: Raw) -> Self {
        Filter(value)
    }
}
impl From<Filter> for Raw {
    fn from(value: Filter) -> Self {
        value.0
    }
}
