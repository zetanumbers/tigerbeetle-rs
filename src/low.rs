pub mod packet;

use std::{
    ffi::c_void, marker::PhantomData, mem, num::NonZeroU32, panic::catch_unwind, ptr, slice,
};

use crate::{
    error::{AcquirePacketError, NewClientError, NewClientErrorKind},
    sys,
    util::RawConstPtr,
};

pub use packet::Packet;

type OnCompletionFn =
    unsafe extern "C" fn(usize, sys::tb_client_t, *mut sys::tb_packet_t, *const u8, u32);

pub struct Client<F, U>
where
    // `F::Target: Sync` because `F` is called from some zig thread.
    // `U: Send` because we are sending user_data into the callback as an
    // argument.
    F: RawConstPtr,
    F::Target: Sync,
    U: RawConstPtr + Send,
{
    raw: sys::tb_client_t,
    on_completion: *const F::Target,
    marker: PhantomData<(F, fn(U))>,
}

pub struct ClientHandle<'a, U>
where
    U: RawConstPtr + Send,
{
    raw: sys::tb_client_t,
    #[allow(clippy::complexity)]
    _marker: PhantomData<(&'a sys::tb_client_t, fn(U))>,
}

impl<'a, U> Copy for ClientHandle<'a, U> where U: RawConstPtr + Send {}

impl<'a, U> Clone for ClientHandle<'a, U>
where
    U: RawConstPtr + Send,
{
    fn clone(&self) -> Self {
        *self
    }
}

unsafe impl<F, U> Send for Client<F, U>
where
    F: RawConstPtr + Send,
    F::Target: Sync,
    U: RawConstPtr + Send,
{
}
unsafe impl<F, U> Sync for Client<F, U>
where
    F: RawConstPtr,
    F::Target: Sync,
    U: RawConstPtr + Send,
{
}

impl<F, U> Client<F, U>
where
    F: RawConstPtr,
    F::Target: Sync,
    U: RawConstPtr + Send,
{
    pub fn with_callback(
        cluster_id: u32,
        address: &[u8],
        concurrency_max: u32,
        on_completion: F,
    ) -> Result<Self, NewClientError>
    where
        // `(F, U): 'static` because we can `mem::forget(self)` and drop anything
        // that is being refered from `F` or `U`, thus invalidating callback or
        // user data.
        (F, U): 'static,
        F::Target: Fn(Packet<'_, U>, &[u8]) + Sized,
        <U as std::ops::Deref>::Target: Sized,
    {
        // SAFETY: F and U are 'static
        unsafe {
            Client::with_callback_unchecked(cluster_id, address, concurrency_max, on_completion)
        }
    }

    /// Highly unsafe method. Please use [`Self::with_callback`]
    /// unless you are *really sure* you are doing it right.
    ///
    /// # Safety
    ///
    /// `F` and `U` are unresticted by any lifetime. It's user's responsibility
    /// to ensure validity of `on_completion` callback or packet's `user_data`
    /// for client's use. If client is dropped, you can safely invalidate these
    /// things.
    pub unsafe fn with_callback_unchecked(
        cluster_id: u32,
        address: &[u8],
        concurrency_max: u32,
        on_completion: F,
    ) -> Result<Self, NewClientError>
    where
        F::Target: Fn(Packet<'_, U>, &[u8]) + Sized,
        <U as std::ops::Deref>::Target: Sized,
    {
        let on_completion_fn = on_completion_fn::<F::Target, U>;
        let on_completion = F::into_raw_const_ptr(on_completion);
        let on_completion_ctx = sptr::Strict::expose_addr(on_completion);

        unsafe fn raw_with_callback(
            cluster_id: u32,
            address: &[u8],
            concurrency_max: u32,
            on_completion_ctx: usize,
            on_completion_fn: OnCompletionFn,
        ) -> Result<sys::tb_client_t, NewClientError> {
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
                on_completion_ctx,
                Some(on_completion_fn),
            );
            if let Some(c) = NonZeroU32::new(status) {
                Err(NewClientError(c))
            } else {
                Ok(raw)
            }
        }

        Ok(Client {
            raw: unsafe {
                raw_with_callback(
                    cluster_id,
                    address,
                    concurrency_max,
                    on_completion_ctx,
                    on_completion_fn,
                )?
            },
            on_completion,
            marker: PhantomData,
        })
    }

    pub fn handle(&self) -> ClientHandle<'_, U> {
        ClientHandle {
            raw: self.raw,
            _marker: PhantomData,
        }
    }

    /// # Panics
    ///
    /// Panics if `AsRef<[u8]>::as_ref(&*user_data).len()` overflows `u32`.
    #[track_caller]
    pub fn acquire(
        &self,
        user_data: U,
        operation: packet::Operation,
    ) -> Result<Packet<'_, U>, AcquirePacketError>
    where
        <U as std::ops::Deref>::Target: AsRef<[u8]> + Sized,
    {
        self.handle().acquire(user_data, operation)
    }
}

/// Blocks until all pending requests finish
impl<F, U> Drop for Client<F, U>
where
    F: RawConstPtr,
    F::Target: Sync,
    U: RawConstPtr + Send,
{
    fn drop(&mut self) {
        unsafe {
            // waits for all callback calls
            sys::tb_client_deinit(self.raw);
            F::from_raw_const_ptr(self.on_completion);
        }
    }
}

unsafe impl<U> Send for ClientHandle<'_, U> where U: RawConstPtr + Send {}
unsafe impl<U> Sync for ClientHandle<'_, U> where U: RawConstPtr + Send {}

impl<'a, U> ClientHandle<'a, U>
where
    U: RawConstPtr + Send,
{
    /// # Panics
    ///
    /// Panics if `AsRef<[u8]>::as_ref(&*user_data).len()` overflows `u32`.
    #[track_caller]
    pub fn acquire(
        self,
        user_data: U,
        operation: packet::Operation,
    ) -> Result<Packet<'a, U>, AcquirePacketError>
    where
        <U as std::ops::Deref>::Target: AsRef<[u8]> + Sized,
    {
        unsafe fn impl_(
            raw_client: sys::tb_client_t,
            user_data: *const c_void,
            data: *const u8,
            data_size: u32,
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
                data_size,
                data: data.cast_mut().cast(),
            });
            Ok(raw)
        }

        let user_data = <U as RawConstPtr>::into_raw_const_ptr(user_data);
        let data = unsafe { (*user_data).as_ref() };
        let (data, data_size) = {
            let data_size = match data.len().try_into() {
                Ok(ds) => ds,
                Err(e) => {
                    drop(unsafe { <U as RawConstPtr>::from_raw_const_ptr(user_data) });
                    panic!("data is too large: {e:?}")
                }
            };
            (data.as_ptr(), data_size)
        };

        let raw = unsafe { impl_(self.raw, user_data.cast(), data, data_size, operation.0)? };
        Ok(Packet {
            raw_client: self.raw,
            raw,
            _marker: PhantomData,
        })
    }
}

unsafe extern "C" fn on_completion_fn<F, U>(
    ctx: usize,
    raw_client: sys::tb_client_t,
    packet: *mut sys::tb_packet_t,
    payload: *const u8,
    payload_size: u32,
) where
    F: Fn(Packet<'_, U>, &[u8]) + Sync,
    U: RawConstPtr + Send,
    <U as std::ops::Deref>::Target: Sized,
{
    let _ = catch_unwind(|| {
        let cb = sptr::from_exposed_addr::<F>(ctx);
        let payload = slice::from_raw_parts(
                payload,
                payload_size
                    .try_into()
                    .expect("While calling on_completion callback: unable to convert payload_size from u32 into usize")
            );
        let packet = Packet {
            raw_client,
            raw: packet,
            _marker: PhantomData,
        };
        (*cb)(packet, payload)
    });
}
