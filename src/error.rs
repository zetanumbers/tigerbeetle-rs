use std::num::{NonZeroU32, NonZeroU8};

use crate::{sys, sys_safe};

pub use sys_safe::{
    CreateAccountErrorKind, CreateTransferErrorKind, PacketStatusErrorKind as SendErrorKind,
    StatusErrorKind as ClientCreationErrorKind,
};

#[derive(Clone, Copy)]
pub struct NewClientError(pub(crate) NonZeroU32);

#[derive(Clone, Copy)]
pub struct SendError(pub(crate) NonZeroU8);

#[derive(Clone, Copy)]
pub struct CreateAccountError(pub(crate) NonZeroU32);

// INVARIANT: self.0.result must not be zero
//
// Should be safe to transpose from `tb_create_accounts_result_t`
// if `CreateAccountsError::try_from_raw` wouldn't return `None`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CreateAccountsError(sys::tb_create_accounts_result_t);

#[derive(Clone, Copy)]
pub struct CreateTransferError(pub(crate) NonZeroU32);

// INVARIANT: self.0.result must not be zero
//
// Should be safe to transpose from `tb_create_transfers_result_t`
// if `CreateTransfersError::try_from_raw` wouldn't return `None`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CreateTransfersError(sys::tb_create_transfers_result_t);

impl NewClientError {
    const CODE_RANGE: std::ops::RangeInclusive<u32> =
        sys_safe::MIN_STATUS_ERROR_CODE..=sys_safe::MAX_STATUS_ERROR_CODE;

    pub fn kind(self) -> ClientCreationErrorKind {
        let code = self.0.get();
        if Self::CODE_RANGE.contains(&code) {
            // SAFETY: We checked if it's in range right above
            unsafe { std::mem::transmute(code) }
        } else {
            ClientCreationErrorKind::UnstableUncategorized
        }
    }
}

impl std::fmt::Debug for NewClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.0.get();
        let mut d = f.debug_tuple("ClientCreationError");
        if Self::CODE_RANGE.contains(&code) {
            d.field(&self.kind());
        } else {
            d.field(&code);
        }
        d.finish()
    }
}

impl std::fmt::Display for NewClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind())
    }
}

impl std::error::Error for NewClientError {}

impl From<ClientCreationErrorKind> for NewClientError {
    /// Panics on hidden `ClientCreationErrorKind::UnstableUncategorized` variant.
    fn from(value: ClientCreationErrorKind) -> Self {
        let code = value as _;
        if !Self::CODE_RANGE.contains(&code) {
            panic!("ClientCreationErrorKind::{value:?}")
        }
        NewClientError(NonZeroU32::new(code).unwrap())
    }
}

impl SendError {
    const CODE_RANGE: std::ops::RangeInclusive<u8> =
        sys_safe::MIN_PACKET_STATUS_ERROR_CODE..=sys_safe::MAX_PACKET_STATUS_ERROR_CODE;

    pub fn kind(self) -> SendErrorKind {
        let code = self.0.get();
        if Self::CODE_RANGE.contains(&code) {
            // SAFETY: We checked if it's in range right above
            unsafe { std::mem::transmute(code) }
        } else {
            SendErrorKind::UnstableUncategorized
        }
    }
}

impl std::fmt::Debug for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.0.get();
        let mut d = f.debug_tuple("SendError");
        if Self::CODE_RANGE.contains(&code) {
            d.field(&self.kind());
        } else {
            d.field(&code);
        }
        d.finish()
    }
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind())
    }
}

impl std::error::Error for SendError {}

impl From<SendErrorKind> for SendError {
    /// Panics on hidden `SendErrorKind::UnstableUncategorized` variant.
    fn from(value: SendErrorKind) -> Self {
        let code = value as _;
        if !Self::CODE_RANGE.contains(&code) {
            panic!("SendErrorKind::{value:?}")
        }
        SendError(NonZeroU8::new(code).unwrap())
    }
}

impl CreateAccountError {
    const CODE_RANGE: std::ops::RangeInclusive<u32> =
        sys_safe::MIN_CREATE_ACCOUNT_ERROR_CODE..=sys_safe::MAX_CREATE_ACCOUNT_ERROR_CODE;

    pub fn kind(self) -> CreateAccountErrorKind {
        let code = self.0.get();
        if Self::CODE_RANGE.contains(&code) {
            // SAFETY: We checked if it's in range right above
            unsafe { std::mem::transmute(code) }
        } else {
            CreateAccountErrorKind::UnstableUncategorized
        }
    }
}

impl std::fmt::Debug for CreateAccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.0.get();
        let mut d = f.debug_tuple("CreateAccountError");
        if Self::CODE_RANGE.contains(&code) {
            d.field(&self.kind());
        } else {
            d.field(&code);
        }
        d.finish()
    }
}

impl std::fmt::Display for CreateAccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind())
    }
}

impl std::error::Error for CreateAccountError {}

impl From<CreateAccountErrorKind> for CreateAccountError {
    /// Panics on hidden `CreateAccountErrorKind::UnstableUncategorized` variant.
    fn from(value: CreateAccountErrorKind) -> Self {
        let code = value as _;
        if !Self::CODE_RANGE.contains(&code) {
            panic!("CreateAccountErrorKind::{value:?}")
        }
        CreateAccountError(NonZeroU32::new(code).unwrap())
    }
}

impl CreateAccountsError {
    /// Fails (returns `None`) when `raw.result` is zero.
    pub(crate) fn try_from_raw(raw: sys::tb_create_accounts_result_t) -> Option<Self> {
        (raw.result != 0).then_some(CreateAccountsError(raw))
    }

    pub fn index(&self) -> u32 {
        self.0.index
    }

    pub fn error(&self) -> CreateAccountError {
        CreateAccountError(
            // SAFETY: type invariant
            unsafe { NonZeroU32::new_unchecked(self.0.result) },
        )
    }
}

impl std::fmt::Debug for CreateAccountsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateAccountsError")
            .field("index", &self.index())
            .field("error", &self.error())
            .finish()
    }
}

impl CreateTransferError {
    const CODE_RANGE: std::ops::RangeInclusive<u32> =
        sys_safe::MIN_CREATE_TRANSFER_ERROR_CODE..=sys_safe::MAX_CREATE_TRANSFER_ERROR_CODE;

    pub fn kind(self) -> CreateTransferErrorKind {
        let code = self.0.get();
        if Self::CODE_RANGE.contains(&code) {
            // SAFETY: We checked if it's in range right above
            unsafe { std::mem::transmute(code) }
        } else {
            CreateTransferErrorKind::UnstableUncategorized
        }
    }
}

impl std::fmt::Debug for CreateTransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.0.get();
        let mut d = f.debug_tuple("CreateTransferError");
        if Self::CODE_RANGE.contains(&code) {
            d.field(&self.kind());
        } else {
            d.field(&code);
        }
        d.finish()
    }
}

impl std::fmt::Display for CreateTransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind())
    }
}

impl std::error::Error for CreateTransferError {}

impl From<CreateTransferErrorKind> for CreateTransferError {
    /// Panics on hidden `CreateTransferErrorKind::UnstableUncategorized` variant.
    fn from(value: CreateTransferErrorKind) -> Self {
        let code = value as _;
        if !Self::CODE_RANGE.contains(&code) {
            panic!("CreateTransferErrorKind::{value:?}")
        }
        CreateTransferError(NonZeroU32::new(code).unwrap())
    }
}

impl CreateTransfersError {
    /// Fails (returns `None`) when `raw.result` is zero.
    pub(crate) fn try_from_raw(raw: sys::tb_create_transfers_result_t) -> Option<Self> {
        (raw.result != 0).then_some(CreateTransfersError(raw))
    }

    pub fn index(&self) -> u32 {
        self.0.index
    }

    pub fn error(&self) -> CreateTransferError {
        CreateTransferError(
            // SAFETY: type invariant
            unsafe { NonZeroU32::new_unchecked(self.0.result) },
        )
    }
}

impl std::fmt::Debug for CreateTransfersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateTransfersError")
            .field("index", &self.index())
            .field("error", &self.error())
            .finish()
    }
}
