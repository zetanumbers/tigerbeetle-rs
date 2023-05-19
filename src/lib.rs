use std::{mem, num::NonZeroU32, ops::RangeInclusive, sync::Mutex};

use tigerbeetle_sys as sys;

pub use sys::generated_safe::StatusErrorKind as ClientCreationErrorKind;

pub struct Client {
    raw: sys::tb_client_t,
    pool: Mutex<sys::tb_packet_list_t>,
}

impl Client {
    pub fn new(
        cluster_id: u32,
        address: &str,
        concurrent_packets: u32,
    ) -> Result<Self, ClientCreationError> {
        unsafe {
            let mut raw = mem::zeroed();
            let mut pool = mem::zeroed();
            let status = sys::tb_client_init(
                &mut raw,
                &mut pool,
                cluster_id,
                address.as_ptr().cast(),
                address
                    .len()
                    .try_into()
                    .map_err(|_| ClientCreationErrorKind::AddressInvalid)?,
                concurrent_packets,
                0,
                Some(on_completion),
            );
            if let Some(c) = NonZeroU32::new(status) {
                return Err(ClientCreationError(c));
            }
            Ok(Self {
                raw,
                pool: Mutex::new(pool),
            })
        }
    }
}

unsafe impl Send for Client {}
unsafe impl Sync for Client {}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe { sys::tb_client_deinit(self.raw) }
    }
}

#[derive(Clone, Copy)]
pub struct ClientCreationError(NonZeroU32);

impl ClientCreationError {
    const CODE_RANGE: RangeInclusive<u32> =
        sys::generated_safe::MIN_STATUS_ERROR_CODE..=sys::generated_safe::MAX_STATUS_ERROR_CODE;

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

impl std::fmt::Debug for ClientCreationError {
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

impl std::fmt::Display for ClientCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind())
    }
}

impl std::error::Error for ClientCreationError {}

impl From<ClientCreationErrorKind> for ClientCreationError {
    /// Panics on hidden `ClientCreationErrorKind::UnstableUncategorized` variant.
    fn from(value: ClientCreationErrorKind) -> Self {
        let code = value as _;
        if !Self::CODE_RANGE.contains(&code) {
            panic!("ClientCreationErrorKind::{value:?}")
        }
        ClientCreationError(NonZeroU32::new(code).unwrap())
    }
}

unsafe extern "C" fn on_completion(
    _ctx: usize,
    _raw_client: sys::tb_client_t,
    _packet: *mut sys::tb_packet_t,
    _payload: *const u8,
    _payload_size: u32,
) {
}
