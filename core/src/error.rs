use std::mem;
use std::num::{NonZeroU32, NonZeroU8};

pub use sys::generated_safe::{
    self as sys_safe, CreateAccountErrorKind, CreateTransferErrorKind,
    PacketAcquireStatusErrorKind as AcquirePacketErrorKind, PacketStatusErrorKind as SendErrorKind,
    StatusErrorKind as NewClientErrorKind,
};
pub use sys::tb_create_accounts_result_t as RawCreateAccountsIndividualApiResult;
pub use sys::tb_create_transfers_result_t as RawCreateTransfersIndividualApiResult;

#[derive(Clone, Copy)]
pub struct NewClientError(pub(crate) NonZeroU32);

#[derive(Clone, Copy)]
pub struct AcquirePacketError(pub(crate) NonZeroU32);

#[derive(Clone, Copy)]
pub struct SendError(pub(crate) NonZeroU8);

#[derive(Clone, Copy)]
pub struct CreateAccountError(pub(crate) NonZeroU32);

/// Type indicating individual api error for account creation.
///
/// Safe to `transpose` from [`RawCreateAccountsIndividualApiResult`]
/// if [`Self::from_raw_result_unchecked`] would also be safe.
//
// INVARIANT: self.0.result must not be zero
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CreateAccountsIndividualApiError(RawCreateAccountsIndividualApiResult);

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

/// Type indicating individual api error for account creation.
///
/// Safe to `transpose` from [`RawCreateTransfersIndividualApiResult`]
/// if [`Self::from_raw_result_unchecked`] would also be safe.
//
// INVARIANT: self.0.result must not be zero
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct CreateTransfersIndividualApiError(RawCreateTransfersIndividualApiResult);

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

    pub fn code(self) -> NonZeroU32 {
        self.0
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

impl AcquirePacketError {
    const CODE_RANGE: std::ops::RangeInclusive<u32> = sys_safe::MIN_PACKET_ACQUIRE_STATUS_ERROR_CODE
        ..=sys_safe::MAX_PACKET_ACQUIRE_STATUS_ERROR_CODE;

    pub fn kind(self) -> AcquirePacketErrorKind {
        let code = self.0.get();
        if Self::CODE_RANGE.contains(&code) {
            // SAFETY: We checked if it's in range right above
            unsafe { std::mem::transmute(code) }
        } else {
            AcquirePacketErrorKind::UnstableUncategorized
        }
    }

    pub fn code(self) -> NonZeroU32 {
        self.0
    }
}

impl std::fmt::Debug for AcquirePacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.0.get();
        let mut d = f.debug_tuple("AcquirePacketErrorError");
        if Self::CODE_RANGE.contains(&code) {
            d.field(&self.kind());
        } else {
            d.field(&code);
        }
        d.finish()
    }
}

impl std::fmt::Display for AcquirePacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind())
    }
}

impl std::error::Error for AcquirePacketError {}

impl From<AcquirePacketErrorKind> for AcquirePacketError {
    /// Panics on hidden `AcquirePacketErrorKind::UnstableUncategorized` variant.
    fn from(value: AcquirePacketErrorKind) -> Self {
        let code = value as _;
        if !Self::CODE_RANGE.contains(&code) {
            panic!("AcquirePacketErrorKind::{value:?}")
        }
        AcquirePacketError(NonZeroU32::new(code).unwrap())
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

    pub fn code(self) -> NonZeroU8 {
        self.0
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

    pub fn code(self) -> NonZeroU32 {
        self.0
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
    /// Create error from raw result.
    ///
    /// # Errors
    ///
    /// Returns `None` if `raw.result` is zero.
    pub fn from_raw_result(raw: RawCreateAccountsIndividualApiResult) -> Option<Self> {
        (raw.result != 0).then_some(CreateAccountsIndividualApiError(raw))
    }

    /// Create error from raw result. Unchecked version of [`Self::from_raw_result`].
    ///
    /// # Safety
    ///
    /// This function is unsafe. `raw.result` must not be zero.
    pub unsafe fn from_raw_result_unchecked(raw: RawCreateAccountsIndividualApiResult) -> Self {
        CreateAccountsIndividualApiError(raw)
    }

    /// Create vec of errors from vec of raw results.
    ///
    /// Retains only elements `r` of vec `v` that satisfy `r.result != 0`.
    pub fn vec_from_raw_results(mut v: Vec<RawCreateAccountsIndividualApiResult>) -> Vec<Self> {
        v.retain(|r| r.result != 0);
        unsafe { Self::vec_from_raw_results_unchecked(v) }
    }

    /// Create vec of errors from vec of raw results. Unchecked version of
    /// [`Self::vec_from_raw_results`]
    ///
    /// # Safety
    ///
    /// This function is unsafe. Every element `r` of vec `v` must satisfy
    /// `r.result != 0`.
    pub unsafe fn vec_from_raw_results_unchecked(
        v: Vec<RawCreateAccountsIndividualApiResult>,
    ) -> Vec<Self> {
        let mut v = mem::ManuallyDrop::new(v);
        let len = v.len();
        let cap = v.capacity();
        let ptr = v.as_mut_ptr().cast::<CreateAccountsIndividualApiError>();
        // SAFETY: this is fine because `Vec::from_raw_parts` has pretty loose
        // safety requirements, and since `CreateAccountsIndividualApiError` is
        // just a transparent wrapper of `RawCreateAccountsIndividualApiResult`
        // this is safe.
        Vec::from_raw_parts(ptr, len, cap)
    }

    /// Get index of the failed account.
    pub fn index(&self) -> u32 {
        self.0.index
    }

    /// Get error stripped of context, like index.
    pub fn inner(&self) -> CreateAccountError {
        CreateAccountError(
            // SAFETY: type invariant
            unsafe { NonZeroU32::new_unchecked(self.0.result) },
        )
    }

    /// Get kind of error to match upon.
    pub fn kind(&self) -> CreateAccountErrorKind {
        self.inner().kind()
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
    /// Get a slice of individual errors. Never empty.
    pub fn as_slice(&self) -> &[CreateAccountsIndividualApiError] {
        &self.0
    }

    /// Create error from vec of raw results.
    ///
    /// # Errors
    ///
    /// Returns `None` if `v.is_empty()`.
    pub fn from_errors(v: Vec<CreateAccountsIndividualApiError>) -> Option<Self> {
        (!v.is_empty()).then_some(CreateAccountsApiError(v))
    }

    /// Create error from vec of raw results.
    ///
    /// Retains only results with errors.
    pub fn from_raw_results(v: Vec<RawCreateAccountsIndividualApiResult>) -> Option<Self> {
        Self::from_errors(CreateAccountsIndividualApiError::vec_from_raw_results(v))
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

    pub fn code(self) -> NonZeroU32 {
        self.0
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
    /// Create error from raw struct.
    ///
    /// # Errors
    ///
    /// Returns `None` if `raw.result` is zero.
    pub fn from_raw_result(raw: RawCreateTransfersIndividualApiResult) -> Option<Self> {
        (raw.result != 0).then_some(CreateTransfersIndividualApiError(raw))
    }

    /// Create error from raw struct. Unchecked version of [`Self::from_raw_result`].
    ///
    /// # Safety
    ///
    /// This function is unsafe. `raw.result` must not be zero.
    pub unsafe fn from_raw_result_unchecked(raw: RawCreateTransfersIndividualApiResult) -> Self {
        CreateTransfersIndividualApiError(raw)
    }

    /// Create vec of errors from vec of raw results.
    ///
    /// Retains only elements `r` of vec `v` that satisfy `r.result != 0`.
    pub fn vec_from_raw_results(mut v: Vec<RawCreateTransfersIndividualApiResult>) -> Vec<Self> {
        v.retain(|r| r.result != 0);
        unsafe { Self::vec_from_raw_results_unchecked(v) }
    }

    /// Create vec of errors from vec of raw results. Unchecked version of
    /// [`Self::vec_from_raw_results`]
    ///
    /// # Safety
    ///
    /// This function is unsafe. Every element `r` of vec `v` must satisfy
    /// `r.result != 0`.
    pub unsafe fn vec_from_raw_results_unchecked(
        v: Vec<RawCreateTransfersIndividualApiResult>,
    ) -> Vec<Self> {
        let mut v = mem::ManuallyDrop::new(v);
        let len = v.len();
        let cap = v.capacity();
        let ptr = v.as_mut_ptr().cast::<CreateTransfersIndividualApiError>();
        // SAFETY: this is fine because `Vec::from_raw_parts` has pretty loose
        // safety requirements, and since `CreateTransfersIndividualApiError` is
        // just a transparent wrapper of `RawCreateTransfersIndividualApiResult`
        // this is safe.
        Vec::from_raw_parts(ptr, len, cap)
    }

    /// Get index of the failed transfer.
    pub fn index(&self) -> u32 {
        self.0.index
    }

    /// Get error stripped of context, like index.
    pub fn inner(&self) -> CreateTransferError {
        CreateTransferError(
            // SAFETY: type invariant
            unsafe { NonZeroU32::new_unchecked(self.0.result) },
        )
    }

    /// Get kind of error to match upon.
    pub fn kind(&self) -> CreateTransferErrorKind {
        self.inner().kind()
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
    /// Get a slice of individual errors. Never empty.
    pub fn as_slice(&self) -> &[CreateTransfersIndividualApiError] {
        &self.0
    }

    /// Create error from vec of raw results.
    ///
    /// # Errors
    ///
    /// Returns `None` if `v.is_empty()`.
    pub fn from_errors(v: Vec<CreateTransfersIndividualApiError>) -> Option<Self> {
        (!v.is_empty()).then_some(CreateTransfersApiError(v))
    }

    /// Create error from vec of raw results.
    ///
    /// Retains only results with errors.
    pub fn from_raw_results(v: Vec<RawCreateTransfersIndividualApiResult>) -> Option<Self> {
        Self::from_errors(CreateTransfersIndividualApiError::vec_from_raw_results(v))
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
