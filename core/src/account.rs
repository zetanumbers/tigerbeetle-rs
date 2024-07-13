use std::time::{Duration, SystemTime};

use bytemuck::{Pod, TransparentWrapper, Zeroable};

mod balance;
mod filter;

pub use balance::{Balance, Raw as RawBalance};
pub use filter::{Filter, Flags as FilterFlags, Raw as RawFilter};
pub use sys::generated_safe::AccountFlags as Flags;
pub use sys::tb_account_t as Raw;

#[repr(transparent)]
#[derive(Clone, Copy, TransparentWrapper, Pod, Zeroable)]
pub struct Account(Raw);

impl Account {
    #[track_caller]
    pub fn new(id: u128, ledger: u32, code: u16) -> Self {
        Account(Raw::zeroed())
            .with_id(id)
            .with_ledger(ledger)
            .with_code(code)
    }

    pub const fn from_raw(raw: Raw) -> Self {
        Account(raw)
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

    pub const fn id(&self) -> u128 {
        self.0.id
    }
    #[track_caller]
    pub fn set_id(&mut self, id: u128) {
        assert_ne!(id, 0, "account id must not be zero");
        assert_ne!(
            id,
            u128::MAX,
            "account id must not be `2^128 - 1` (the highest 128-bit unsigned integer)"
        );
        self.0.id = id;
    }
    #[track_caller]
    pub fn with_id(mut self, id: u128) -> Self {
        self.set_id(id);
        self
    }

    pub const fn user_data_128(&self) -> u128 {
        self.0.user_data_128
    }
    pub fn set_user_data_128(&mut self, user_data_128: u128) {
        self.0.user_data_128 = user_data_128;
    }
    pub const fn with_user_data_128(mut self, user_data_128: u128) -> Self {
        self.0.user_data_128 = user_data_128;
        self
    }

    pub const fn user_data_64(&self) -> u64 {
        self.0.user_data_64
    }
    pub fn set_user_data_64(&mut self, user_data_64: u64) {
        self.0.user_data_64 = user_data_64;
    }
    pub const fn with_user_data_64(mut self, user_data_64: u64) -> Self {
        self.0.user_data_64 = user_data_64;
        self
    }

    pub const fn user_data_32(&self) -> u32 {
        self.0.user_data_32
    }
    pub fn set_user_data_32(&mut self, user_data_32: u32) {
        self.0.user_data_32 = user_data_32;
    }
    pub const fn with_user_data_32(mut self, user_data_32: u32) -> Self {
        self.0.user_data_32 = user_data_32;
        self
    }

    pub const fn ledger(&self) -> u32 {
        self.0.ledger
    }
    #[track_caller]
    pub fn set_ledger(&mut self, ledger: u32) {
        assert_ne!(ledger, 0, "account ledger must not be zero");
        self.0.ledger = ledger;
    }
    #[track_caller]
    pub fn with_ledger(mut self, ledger: u32) -> Self {
        self.set_ledger(ledger);
        self
    }

    pub const fn code(&self) -> u16 {
        self.0.code
    }
    #[track_caller]
    pub fn set_code(&mut self, code: u16) {
        assert_ne!(code, 0, "account code must not be zero");
        self.0.code = code;
    }
    #[track_caller]
    pub fn with_code(mut self, code: u16) -> Self {
        self.set_code(code);
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

    pub const fn debits_pending(&self) -> u128 {
        self.0.debits_pending
    }
    pub const fn debits_posted(&self) -> u128 {
        self.0.debits_posted
    }
    pub const fn credits_pending(&self) -> u128 {
        self.0.credits_pending
    }
    pub const fn credits_posted(&self) -> u128 {
        self.0.credits_posted
    }

    pub fn timestamp(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_nanos(self.0.timestamp)
    }
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account")
            .field("id", &self.id())
            .field("debits_pending", &self.debits_pending())
            .field("debits_posted", &self.debits_posted())
            .field("credits_pending", &self.credits_pending())
            .field("credits_posted", &self.credits_posted())
            .field("user_data_128", &self.user_data_128())
            .field("user_data_64", &self.user_data_64())
            .field("user_data_32", &self.user_data_32())
            .field("ledger", &self.ledger())
            .field("code", &self.code())
            .field("flags", &self.flags())
            .field("timestamp", &self.timestamp())
            .finish_non_exhaustive()
    }
}

impl From<Raw> for Account {
    fn from(value: Raw) -> Self {
        Account(value)
    }
}
impl From<Account> for Raw {
    fn from(value: Account) -> Self {
        value.0
    }
}
