mod account;
mod blob;
mod error;
mod resource_pool;

use std::{
    mem,
    num::{NonZeroU32, NonZeroU8},
    panic::catch_unwind,
    ptr, slice,
    sync::Arc,
};

use blob::Blob;
use futures_channel::oneshot as async_oneshot;
use resource_pool::ResourcePool;
use tigerbeetle_sys::{self as sys, generated_safe as sys_safe};

pub use sys::tb_transfer_t as Transfer;
pub use sys_safe::TransferFlags;

pub use crate::{
    account::{Account, AccountFlags, AccountRaw},
    error::{
        CreateAccountError, CreateAccountErrorKind, CreateAccountsError, CreateTransferError,
        CreateTransferErrorKind, CreateTransfersError, NewClientError, NewClientErrorKind,
        SendError, SendErrorKind,
    },
};

#[derive(Clone)]
pub struct Client {
    shared: Arc<ClientUnique>,
}

impl Client {
    pub fn new(
        cluster_id: u32,
        address: &str,
        concurrent_packets: u32,
    ) -> Result<Self, NewClientError> {
        unsafe {
            let packet_count = concurrent_packets
                .try_into()
                .map_err(|_| NewClientErrorKind::PacketsCountInvalid)?;
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
                    .map_err(|_| NewClientErrorKind::AddressInvalid)?,
                concurrent_packets,
                0,
                Some(on_completion),
            );
            if let Some(c) = NonZeroU32::new(status) {
                return Err(NewClientError(c));
            }
            Ok(Self {
                shared: Arc::new(ClientUnique {
                    raw,
                    pool: ResourcePool::new(pool, packet_count),
                }),
            })
        }
    }

    pub fn try_join(self) -> Result<(), Self> {
        Arc::try_unwrap(self.shared)
            .map(drop)
            .map_err(|shared| Client { shared })
    }

    pub async fn create_accounts(
        &self,
        accounts: Vec<Account>,
    ) -> Result<Vec<CreateAccountsError>, SendError> {
        let data = Blob::from_vec(accounts);

        let mut res = self
            .submit(data, sys_safe::OperationKind::CreateAccounts)
            .await?
            .into_vec();
        res.retain(|raw| CreateAccountsError::try_from_raw(*raw).is_some());

        // SAFETY: just transposing original vec into vec of transparent `Copy` newtypes
        Ok(unsafe { transpose_vec::<sys::tb_create_accounts_result_t, CreateAccountsError>(res) })
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
    ) -> Result<Vec<CreateTransfersError>, SendError> {
        let data = Blob::from_vec(transfers);

        let mut res = self
            .submit(data, sys_safe::OperationKind::CreateTransfers)
            .await?
            .into_vec();
        res.retain(|raw| CreateTransfersError::try_from_raw(*raw).is_some());

        // SAFETY: just transposing original vec into vec of transparent `Copy` newtypes
        Ok(
            unsafe {
                transpose_vec::<sys::tb_create_transfers_result_t, CreateTransfersError>(res)
            },
        )
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

        let packet = self.shared.pool.acquire_packet().await;
        let packet_ptr = packet.packet();
        let (reply_sender, reply_receiver) = async_oneshot::channel();
        let user_data = Box::into_raw(Box::new(PacketUserData {
            _data: data,
            reply_sender,
            _client: self.clone(),
            // SAFETY: owning a client handle to prevent resource pool from droping
            _packet_guard: unsafe { mem::transmute(packet) },
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

        let mut packet_list = sys::tb_packet_list_t {
            head: packet_ptr.as_ptr(),
            tail: packet_ptr.as_ptr(),
        };

        unsafe { sys::tb_client_submit(self.shared.raw, &mut packet_list) };

        reply_receiver
            .await
            .expect("reply_sender has been closed, probably due to on_completion call panic")
    }
}

struct ClientUnique {
    raw: sys::tb_client_t,
    pool: ResourcePool,
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
    _packet_guard: resource_pool::PacketGuard<'static>,
    _data: Blob,
    _client: Client,
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
