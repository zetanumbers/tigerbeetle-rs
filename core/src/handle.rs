use std::{ffi::c_void, num::NonZeroU32, ptr};

use crate::{error::AcquirePacketError, sys};

use super::{
    callback::{Callbacks, UserDataPtr},
    packet, Packet,
};

pub struct ClientHandle<'a, U>
where
    U: UserDataPtr,
{
    pub(crate) raw: sys::tb_client_t,
    pub(crate) on_completion: &'a dyn Callbacks<UserDataPtr = U>,
}

unsafe impl<U> Send for ClientHandle<'_, U> where U: UserDataPtr {}
unsafe impl<U> Sync for ClientHandle<'_, U> where U: UserDataPtr {}

impl<U> Copy for ClientHandle<'_, U> where U: UserDataPtr {}

impl<U> Clone for ClientHandle<'_, U>
where
    U: UserDataPtr,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, U> ClientHandle<'a, U>
where
    U: UserDataPtr,
{
    pub fn acquire(
        self,
        user_data: U,
        operation: packet::Operation,
    ) -> Result<Packet<'a, U>, AcquirePacketError> {
        unsafe fn impl_(
            raw_client: sys::tb_client_t,
            user_data: *const c_void,
            operation: u8,
        ) -> Result<*mut sys::tb_packet_t, AcquirePacketError> {
            let mut raw = ptr::null_mut();
            let status = sys::tb_client_acquire_packet(raw_client, &mut raw);
            if let Some(c) = NonZeroU32::new(status) {
                return Err(AcquirePacketError(c));
            }
            raw.write(sys::tb_packet_t {
                next: ptr::null_mut(),
                user_data: user_data.cast_mut(),
                operation,
                status: 0,
                data_size: 0,
                data: ptr::null_mut(),
            });
            Ok(raw)
        }

        let user_data = U::into_raw_const_ptr(user_data);

        let raw = unsafe { impl_(self.raw, user_data.cast(), operation.0)? };
        Ok(Packet { raw, handle: self })
    }
}
