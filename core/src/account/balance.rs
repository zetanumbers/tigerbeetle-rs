use std::time::{Duration, SystemTime};

use bytemuck::{Pod, TransparentWrapper, Zeroable};

pub use sys::tb_account_balance_t as Raw;

#[repr(transparent)]
#[derive(Clone, Copy, TransparentWrapper, Pod, Zeroable)]
pub struct Balance(Raw);

impl Balance {
    pub const fn from_raw(raw: Raw) -> Self {
        Balance(raw)
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

    pub const fn debits_pending(&self) -> u128 {
        self.0.debits_pending
    }
    pub fn set_debits_pending(&mut self, debits_pending: u128) {
        self.0.debits_pending = debits_pending;
    }
    pub const fn with_debits_pending(mut self, debits_pending: u128) -> Self {
        self.0.debits_pending = debits_pending;
        self
    }

    pub const fn debits_posted(&self) -> u128 {
        self.0.debits_posted
    }
    pub fn set_debits_posted(&mut self, debits_posted: u128) {
        self.0.debits_posted = debits_posted;
    }
    pub const fn with_debits_posted(mut self, debits_posted: u128) -> Self {
        self.0.debits_posted = debits_posted;
        self
    }

    pub const fn credits_pending(&self) -> u128 {
        self.0.credits_pending
    }
    pub fn set_credits_pending(&mut self, credits_pending: u128) {
        self.0.credits_pending = credits_pending;
    }
    pub const fn with_credits_pending(mut self, credits_pending: u128) -> Self {
        self.0.credits_pending = credits_pending;
        self
    }

    pub const fn credits_posted(&self) -> u128 {
        self.0.credits_posted
    }
    pub fn set_credits_posted(&mut self, credits_posted: u128) {
        self.0.credits_posted = credits_posted;
    }
    pub const fn with_credits_posted(mut self, credits_posted: u128) -> Self {
        self.0.credits_posted = credits_posted;
        self
    }

    pub fn timestamp(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_nanos(self.0.timestamp)
    }
}

impl std::fmt::Debug for Balance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountBalance")
            .field("debits_pending", &self.0.debits_pending)
            .field("debits_posted", &self.0.debits_posted)
            .field("credits_pending", &self.0.credits_pending)
            .field("credits_pending", &self.0.credits_pending)
            .field("timestamp", &self.0.timestamp)
            .finish_non_exhaustive()
    }
}

impl From<Raw> for Balance {
    fn from(value: Raw) -> Self {
        Balance(value)
    }
}
impl From<Balance> for Raw {
    fn from(value: Balance) -> Self {
        value.0
    }
}
