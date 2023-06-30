use std::num::{NonZeroU32, NonZeroU8};

use crate::{sys, sys_safe};

pub use sys_safe::{
    CreateAccountErrorKind, CreateTransferErrorKind, PacketStatusErrorKind as SendErrorKind,
    StatusErrorKind as NewClientErrorKind,
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
pub struct CreateAccountsIndividualApiError(sys::tb_create_accounts_result_t);

// INVARIANT: self.0 must not be empty
#[derive(Debug)]
pub struct CreateAccountsApiError(Vec<CreateAccountsIndividualApiError>);

#[non_exhaustive]
#[derive(Debug)]
pub enum CreateAccountsError {
    Send(SendError),
    Api(CreateAccountsApiError),
}

#[derive(Clone, Copy)]
pub struct CreateTransferError(pub(crate) NonZeroU32);

// INVARIANT: self.0.result must not be zero
//
// Should be safe to transpose from `tb_create_transfers_result_t`
// if `CreateTransfersError::try_from_raw` wouldn't return `None`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CreateTransfersIndividualApiError(sys::tb_create_transfers_result_t);

// INVARIANT: self.0 must not be empty
#[derive(Debug)]
pub struct CreateTransfersApiError(Vec<CreateTransfersIndividualApiError>);

#[non_exhaustive]
#[derive(Debug)]
pub enum CreateTransfersError {
    Send(SendError),
    Api(CreateTransfersApiError),
}

impl NewClientError {
    const CODE_RANGE: std::ops::RangeInclusive<u32> =
        sys_safe::MIN_STATUS_ERROR_CODE..=sys_safe::MAX_STATUS_ERROR_CODE;

    pub fn kind(self) -> NewClientErrorKind {
        let code = self.0.get();
        if Self::CODE_RANGE.contains(&code) {
            // SAFETY: We checked if it's in range right above
            unsafe { std::mem::transmute(code) }
        } else {
            NewClientErrorKind::UnstableUncategorized
        }
    }
}

impl std::fmt::Debug for NewClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.0.get();
        let mut d = f.debug_tuple("NewClientErrorError");
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

impl From<NewClientErrorKind> for NewClientError {
    /// Panics on hidden `NewClientErrorKind::UnstableUncategorized` variant.
    fn from(value: NewClientErrorKind) -> Self {
        let code = value as _;
        if !Self::CODE_RANGE.contains(&code) {
            panic!("NewClientErrorKind::{value:?}")
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

impl CreateAccountsIndividualApiError {
    /// Fails (returns `None`) when `raw.result` is zero.
    pub(crate) fn try_from_raw(raw: sys::tb_create_accounts_result_t) -> Option<Self> {
        (raw.result != 0).then_some(CreateAccountsIndividualApiError(raw))
    }

    pub fn index(&self) -> u32 {
        self.0.index
    }

    pub fn inner(&self) -> CreateAccountError {
        CreateAccountError(
            // SAFETY: type invariant
            unsafe { NonZeroU32::new_unchecked(self.0.result) },
        )
    }
}

impl std::fmt::Debug for CreateAccountsIndividualApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateAccountsError")
            .field("index", &self.index())
            .field("inner", &self.inner())
            .finish()
    }
}

impl std::fmt::Display for CreateAccountsIndividualApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{}` error occured at account with index {}",
            self.inner(),
            self.index()
        )
    }
}

impl std::error::Error for CreateAccountsIndividualApiError {}

impl CreateAccountsApiError {
    pub fn as_slice(&self) -> &[CreateAccountsIndividualApiError] {
        &self.0
    }

    pub fn from_vec(v: Vec<CreateAccountsIndividualApiError>) -> Option<Self> {
        (!v.is_empty()).then_some(CreateAccountsApiError(v))
    }
}

impl AsRef<[CreateAccountsIndividualApiError]> for CreateAccountsApiError {
    fn as_ref(&self) -> &[CreateAccountsIndividualApiError] {
        &self.0
    }
}

impl std::fmt::Display for CreateAccountsApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} api errors occured at accounts' creation",
            self.0.len()
        )
    }
}

impl std::error::Error for CreateAccountsApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.first().map(|e| e as _)
    }
}

impl From<CreateAccountsIndividualApiError> for CreateAccountsApiError {
    fn from(value: CreateAccountsIndividualApiError) -> Self {
        CreateAccountsApiError(vec![value])
    }
}

impl std::error::Error for CreateAccountsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            CreateAccountsError::Send(e) => e as _,
            CreateAccountsError::Api(e) => e as _,
        })
    }
}

impl std::fmt::Display for CreateAccountsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateAccountsError::Send(_) => {
                "error occured while sending packets for accounts' creation"
            }
            CreateAccountsError::Api(_) => "api errors occured at accounts' creation",
        }
        .fmt(f)
    }
}

impl From<SendError> for CreateAccountsError {
    fn from(value: SendError) -> Self {
        CreateAccountsError::Send(value)
    }
}

impl From<CreateAccountsApiError> for CreateAccountsError {
    fn from(value: CreateAccountsApiError) -> Self {
        CreateAccountsError::Api(value)
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

impl CreateTransfersIndividualApiError {
    /// Fails (returns `None`) when `raw.result` is zero.
    pub(crate) fn try_from_raw(raw: sys::tb_create_transfers_result_t) -> Option<Self> {
        (raw.result != 0).then_some(CreateTransfersIndividualApiError(raw))
    }

    pub fn index(&self) -> u32 {
        self.0.index
    }

    pub fn inner(&self) -> CreateTransferError {
        CreateTransferError(
            // SAFETY: type invariant
            unsafe { NonZeroU32::new_unchecked(self.0.result) },
        )
    }
}

impl std::fmt::Debug for CreateTransfersIndividualApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateTransfersError")
            .field("index", &self.index())
            .field("inner", &self.inner())
            .finish()
    }
}

impl std::fmt::Display for CreateTransfersIndividualApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{}` error occured at account with index {}",
            self.inner(),
            self.index()
        )
    }
}

impl std::error::Error for CreateTransfersIndividualApiError {}

impl CreateTransfersApiError {
    pub fn as_slice(&self) -> &[CreateTransfersIndividualApiError] {
        &self.0
    }

    pub(crate) fn from_vec(v: Vec<CreateTransfersIndividualApiError>) -> Option<Self> {
        (!v.is_empty()).then_some(CreateTransfersApiError(v))
    }
}

impl AsRef<[CreateTransfersIndividualApiError]> for CreateTransfersApiError {
    fn as_ref(&self) -> &[CreateTransfersIndividualApiError] {
        &self.0
    }
}

impl std::fmt::Display for CreateTransfersApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} api errors occured at transfers' creation",
            self.0.len()
        )
    }
}

impl std::error::Error for CreateTransfersApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.first().map(|e| e as _)
    }
}

impl From<CreateTransfersIndividualApiError> for CreateTransfersApiError {
    fn from(value: CreateTransfersIndividualApiError) -> Self {
        CreateTransfersApiError(vec![value])
    }
}

impl std::error::Error for CreateTransfersError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            CreateTransfersError::Send(e) => e as _,
            CreateTransfersError::Api(e) => e as _,
        })
    }
}

impl std::fmt::Display for CreateTransfersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateTransfersError::Send(_) => {
                "error occured while sending packets for transfers' creation"
            }
            CreateTransfersError::Api(_) => "api errors occured at transfers' creation",
        }
        .fmt(f)
    }
}

impl From<SendError> for CreateTransfersError {
    fn from(value: SendError) -> Self {
        CreateTransfersError::Send(value)
    }
}

impl From<CreateTransfersApiError> for CreateTransfersError {
    fn from(value: CreateTransfersApiError) -> Self {
        CreateTransfersError::Api(value)
    }
}
