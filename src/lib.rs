pub mod account;
mod blob;
pub mod error;
mod packet_guard;
pub mod transfer;

use std::{
    mem,
    num::{NonZeroU32, NonZeroU8},
    panic::catch_unwind,
    ptr, slice,
    sync::Arc,
};

use blob::Blob;
use error::{
    CreateAccountsApiError, CreateAccountsError, CreateAccountsIndividualApiError,
    CreateTransfersApiError, CreateTransfersError, CreateTransfersIndividualApiError,
    NewClientError, NewClientErrorKind, SendError, SendErrorKind,
};
use packet_guard::PacketGuard;
use tigerbeetle_sys::{self as sys, generated_safe as sys_safe};
use tokio::sync::{oneshot as async_oneshot, Semaphore as AsyncSemaphore};

pub use crate::{account::Account, transfer::Transfer};

#[derive(Clone)]
pub struct Client {
    shared: Arc<ClientUnique>,
}

impl Client {
    pub fn new(
        cluster_id: u32,
        address: &str,
        concurrency_max: u32,
    ) -> Result<Self, NewClientError> {
        unsafe {
            let permits = concurrency_max
                .try_into()
                .map_err(|_| NewClientErrorKind::ConcurrencyMaxInvalid)?;
            let pool = AsyncSemaphore::new(permits);

            let mut raw = mem::zeroed();
            let status = sys::tb_client_init(
                &mut raw,
                cluster_id,
                address.as_ptr().cast(),
                address
                    .len()
                    .try_into()
                    .map_err(|_| NewClientErrorKind::AddressInvalid)?,
                concurrency_max,
                0,
                Some(on_completion),
            );
            if let Some(c) = NonZeroU32::new(status) {
                return Err(NewClientError(c));
            }
            Ok(Self {
                shared: Arc::new(ClientUnique { raw, pool }),
            })
        }
    }

    pub fn try_join(self) -> Result<(), Self> {
        Arc::try_unwrap(self.shared)
            .map(drop)
            .map_err(|shared| Client { shared })
    }

    pub async fn create_accounts(&self, accounts: Vec<Account>) -> Result<(), CreateAccountsError> {
        let data = Blob::from_vec(accounts);

        let mut res = self
            .submit(data, sys_safe::OperationKind::CreateAccounts)
            .await?
            .into_vec();
        res.retain(|raw| CreateAccountsIndividualApiError::try_from_raw(*raw).is_some());

        // SAFETY: just transposing original vec into vec of transparent `Copy` newtypes
        let vec = unsafe {
            transpose_vec::<sys::tb_create_accounts_result_t, CreateAccountsIndividualApiError>(res)
        };
        if let Some(e) = CreateAccountsApiError::from_vec(vec) {
            Err(CreateAccountsError::Api(e))
        } else {
            Ok(())
        }
    }

    pub async fn lookup_accounts(&self, ids: Vec<u128>) -> Result<Vec<Account>, SendError> {
        let data = Blob::from_vec(ids);
        Ok(self
            .submit(data, sys_safe::OperationKind::LookupAccounts)
            .await?
            .into_vec())
    }

    pub async fn create_transfers(
        &self,
        transfers: Vec<Transfer>,
    ) -> Result<(), CreateTransfersError> {
        let data = Blob::from_vec(transfers);

        let mut res = self
            .submit(data, sys_safe::OperationKind::CreateTransfers)
            .await?
            .into_vec();
        res.retain(|raw| CreateTransfersIndividualApiError::try_from_raw(*raw).is_some());

        // SAFETY: just transposing original vec into vec of transparent `Copy` newtypes

        let vec = unsafe {
            transpose_vec::<sys::tb_create_transfers_result_t, CreateTransfersIndividualApiError>(
                res,
            )
        };
        if let Some(e) = CreateTransfersApiError::from_vec(vec) {
            Err(CreateTransfersError::Api(e))
        } else {
            Ok(())
        }
    }

    pub async fn lookup_transfers(&self, ids: Vec<u128>) -> Result<Vec<Transfer>, SendError> {
        let data = Blob::from_vec(ids);
        Ok(self
            .submit(data, sys_safe::OperationKind::LookupTransfers)
            .await?
            .into_vec())
    }

    async fn submit(
        &self,
        data: Blob,
        operation: sys_safe::OperationKind,
    ) -> Result<Blob, SendError> {
        let data_ptr = data.as_ptr();
        let data_size = data
            .byte_size()
            .try_into()
            .map_err(|_| SendErrorKind::TooMuchData)?;

        let packet_guard = PacketGuard::acquire(self.clone()).await;
        let packet_ptr = packet_guard.packet();
        let (reply_sender, reply_receiver) = async_oneshot::channel();
        let user_data = Box::into_raw(Box::new(PacketUserData {
            _data: data,
            reply_sender,
            // SAFETY: owning a client handle to prevent resource pool from droping
            _packet_guard: packet_guard,
        }));

        // SAFETY: packet_ptr is valid, tb_packet_t is not covariant on anything
        unsafe {
            packet_ptr.as_ptr().write(sys::tb_packet_t {
                next: ptr::null_mut(),
                user_data: user_data.cast(),
                operation: operation as _,
                status: 0,
                data_size,
                data: data_ptr.as_ptr().cast(),
            })
        };

        unsafe { sys::tb_client_submit(self.shared.raw, packet_ptr.as_ptr()) };

        reply_receiver
            .await
            .expect("reply_sender has been closed, probably due to on_completion call panic")
    }
}

struct ClientUnique {
    raw: sys::tb_client_t,
    pool: AsyncSemaphore,
}

unsafe impl Send for ClientUnique {}
unsafe impl Sync for ClientUnique {}

impl Drop for ClientUnique {
    fn drop(&mut self) {
        unsafe {
            sys::tb_client_deinit(self.raw);
        }
    }
}

struct PacketUserData {
    reply_sender: async_oneshot::Sender<Result<Blob, SendError>>,
    // lifetime is extended by owning a client handle
    _packet_guard: PacketGuard,
    _data: Blob,
}

unsafe extern "C" fn on_completion(
    _ctx: usize,
    _raw_client: sys::tb_client_t,
    packet: *mut sys::tb_packet_t,
    payload: *const u8,
    payload_size: u32,
) {
    let _ = catch_unwind(|| {
        let user_data = Box::from_raw((*packet).user_data.cast::<PacketUserData>());
        let packet_status = (*packet).status;
        let res = match NonZeroU8::new(packet_status) {
            Some(packet_status) => Err(SendError(packet_status)),
            None => Ok(Blob::copy_from_reply(
                slice::from_raw_parts(payload, payload_size as usize),
                (*packet).operation,
            )),
        };
        let _ = user_data.reply_sender.send(res);
    });
}

unsafe fn transpose_vec<T, U>(mut v: Vec<T>) -> Vec<U> {
    let length = v.len();
    let capacity = v.capacity();
    let ptr = v.as_mut_ptr();
    mem::forget(v);
    Vec::from_raw_parts(ptr.cast::<U>(), length, capacity)
}
