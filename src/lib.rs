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

pub use crate::{
    account::{Account, AccountFlags, AccountRaw},
    error::{
        ClientCreationError, ClientCreationErrorKind, CreateAccountError, CreateAccountErrorKind,
        CreateAccountsError, SendError, SendErrorKind,
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
    ) -> Result<Self, ClientCreationError> {
        unsafe {
            let packet_count = concurrent_packets
                .try_into()
                .map_err(|_| ClientCreationErrorKind::PacketsCountInvalid)?;
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
                operation: sys_safe::OperationKind::CreateAccounts as _,
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

        let mut res: Vec<sys::tb_create_accounts_result_t> = reply_receiver
            .await
            .expect("reply_sender has been closed, probably due to on_completion call panic")?
            .into_vec();

        res.retain(|raw| CreateAccountsError::try_from_raw(*raw).is_some());

        let res = {
            let length = res.len();
            let capacity = res.capacity();
            let ptr = res.as_mut_ptr();
            mem::forget(res);
            // SAFETY: just transposing original vec into vec of transparent `Copy` newtypes
            unsafe { Vec::from_raw_parts(ptr.cast::<CreateAccountsError>(), length, capacity) }
        };

        Ok(res)
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
