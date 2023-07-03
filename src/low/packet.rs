use std::{marker::PhantomData, mem, num::NonZeroU8};

use crate::{error::SendError, sys, sys_safe, util::RawConstPtr};

pub use sys_safe::OperationKind;

use super::ClientHandle;

pub struct Packet<'a, U>
where
    U: RawConstPtr + Send,
    <U as std::ops::Deref>::Target: Sized,
{
    pub(super) raw_client: sys::tb_client_t,
    pub(super) raw: *mut sys::tb_packet_t,
    pub(super) _marker: PhantomData<(&'a sys::tb_client_t, U)>,
}

#[derive(Clone, Copy)]
pub struct Operation(pub(crate) u8);

unsafe impl<U> Sync for Packet<'_, U>
where
    U: RawConstPtr + Send,
    <U as std::ops::Deref>::Target: Sized + Sync,
{
}
unsafe impl<U> Send for Packet<'_, U>
where
    U: RawConstPtr + Send,
    <U as std::ops::Deref>::Target: Sized,
{
}

impl<'a, U> Packet<'a, U>
where
    U: RawConstPtr + Send,
    <U as std::ops::Deref>::Target: Sized,
{
    pub fn submit(self) {
        unsafe { sys::tb_client_submit(self.raw_client, self.raw) };
        mem::forget(self);
    }

    fn raw(&self) -> &sys::tb_packet_t {
        unsafe { &*self.raw }
    }

    fn raw_mut(&mut self) -> &mut sys::tb_packet_t {
        unsafe { &mut *self.raw }
    }

    pub fn into_user_data(self) -> U {
        let this = mem::ManuallyDrop::new(self);
        let user_data;
        unsafe {
            user_data = U::from_raw_const_ptr(this.raw().user_data.cast_const().cast());
            sys::tb_client_release_packet(this.raw_client, this.raw);
        }
        user_data
    }

    pub fn replace_user_data(&mut self, user_data: U) -> U {
        let new = U::into_raw_const_ptr(user_data).cast_mut().cast();
        let ptr = mem::replace(&mut self.raw_mut().user_data, new)
            .cast_const()
            .cast();
        unsafe { U::from_raw_const_ptr(ptr) }
    }

    pub fn user_data(&self) -> &U::Target {
        unsafe { self.raw().user_data.cast::<U::Target>().as_ref().unwrap() }
    }

    pub fn user_data_mut(&mut self) -> &mut U::Target
    where
        U: std::ops::DerefMut,
    {
        unsafe {
            self.raw_mut()
                .user_data
                .cast::<U::Target>()
                .as_mut()
                .unwrap()
        }
    }

    pub fn data(&self) -> &[u8]
    where
        U::Target: AsRef<[u8]>,
    {
        self.user_data().as_ref()
    }

    pub fn client_handle(&self) -> ClientHandle<'a, U> {
        ClientHandle {
            raw: self.raw_client,
            _marker: PhantomData,
        }
    }

    pub fn operation(&self) -> Operation {
        Operation(self.raw().operation)
    }

    pub fn set_operation(&mut self, operation: Operation) {
        self.raw_mut().operation = operation.0;
    }

    pub fn status(&self) -> Result<(), SendError> {
        if let Some(c) = NonZeroU8::new(self.raw().status) {
            Err(SendError(c))
        } else {
            Ok(())
        }
    }

    pub fn set_status(&mut self, status: Result<(), SendError>) {
        self.raw_mut().status = match status {
            Ok(()) => 0,
            Err(e) => e.0.get(),
        }
    }
}

impl<U> Drop for Packet<'_, U>
where
    U: RawConstPtr + Send,
    <U as std::ops::Deref>::Target: Sized,
{
    fn drop(&mut self) {
        unsafe {
            U::from_raw_const_ptr(self.raw().user_data.cast_const().cast());
            sys::tb_client_release_packet(self.raw_client, self.raw);
        }
    }
}

impl Operation {
    const CODE_RANGE: std::ops::RangeInclusive<u8> =
        sys_safe::MIN_OPERATION_CODE..=sys_safe::MAX_OPERATION_CODE;

    pub fn kind(self) -> OperationKind {
        if Self::CODE_RANGE.contains(&self.0) {
            // SAFETY: We checked if it's in range right above
            unsafe { std::mem::transmute(self.0) }
        } else {
            OperationKind::UnstableUncategorized
        }
    }

    pub fn code(self) -> u8 {
        self.0
    }
}

impl std::fmt::Debug for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_tuple("Operation");
        if Self::CODE_RANGE.contains(&self.0) {
            d.field(&self.kind());
        } else {
            d.field(&self.0);
        }
        d.finish()
    }
}

impl From<OperationKind> for Operation {
    /// Panics on hidden `OperationKind::UnstableUncategorized` variant.
    fn from(value: OperationKind) -> Self {
        let code = value as _;
        if !Self::CODE_RANGE.contains(&code) {
            panic!("OperationKind::{value:?}")
        }
        Operation(code)
    }
}
