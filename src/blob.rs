use std::{alloc, mem, ptr};

use crate::{sys, sys_safe};

/// You may reference blob contents through poiner from `Blob::as_ptr` while
/// blob is valid.
pub struct Blob {
    ptr: ptr::NonNull<u8>,
    item_count: usize,
    item_capacity: usize,
    item_layout: alloc::Layout,
}

impl Blob {
    #[track_caller]
    pub fn copy_from_reply(bytes: &[u8], operation: u8) -> Blob {
        let bytes_len = bytes.len();
        let item_layout = reply_layout(operation);
        let item_count = bytes_len / item_layout.size();
        assert_eq!(
            bytes_len % item_layout.size(),
            0,
            "reply size is not multiple of the result type size"
        );
        let ptr = unsafe {
            alloc::alloc(
                alloc::Layout::from_size_align(bytes_len, item_layout.align())
                    .expect("could not create array layout"),
            )
        };
        let Some(ptr) = ptr::NonNull::new(ptr) else {
            panic!("Out of memory")
        };
        unsafe {
            ptr.as_ptr()
                .copy_from_nonoverlapping(bytes.as_ptr(), bytes_len)
        };
        Blob {
            ptr,
            item_count,
            item_capacity: item_count,
            item_layout,
        }
    }

    pub fn as_ptr(&self) -> ptr::NonNull<u8> {
        self.ptr
    }
    pub fn byte_size(&self) -> usize {
        self.item_count * self.item_layout.size()
    }

    pub fn from_vec<T>(mut v: Vec<T>) -> Self
    where
        T: bytemuck::Pod,
    {
        let ptr = v.as_mut_ptr().cast();
        let item_count = v.len();
        let item_capacity = v.capacity();
        let item_layout = alloc::Layout::new::<T>();

        let ptr = ptr::NonNull::new(ptr).unwrap_or_else(|| {
            assert_eq!(item_capacity, 0);
            ptr::NonNull::dangling()
        });
        mem::forget(v);
        Blob {
            ptr,
            item_count,
            item_capacity,
            item_layout,
        }
    }

    #[track_caller]
    pub fn into_vec<T>(self) -> Vec<T>
    where
        T: bytemuck::Pod,
    {
        assert_eq!(alloc::Layout::new::<T>(), self.item_layout);
        let out = mem::ManuallyDrop::new(if self.item_capacity == 0 {
            Vec::new()
        } else {
            unsafe {
                Vec::from_raw_parts(
                    self.ptr.as_ptr().cast(),
                    self.item_count,
                    self.item_capacity,
                )
            }
        });
        mem::forget(self);
        mem::ManuallyDrop::into_inner(out)
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        if self.item_capacity == 0 {
            return;
        }
        let Ok(layout) = alloc::Layout::from_size_align(
            self.item_layout.size() * self.item_capacity,
            self.item_layout.align()
        ) else { return };
        unsafe { alloc::dealloc(self.ptr.as_ptr(), layout) }
    }
}

#[track_caller]
fn reply_layout(op: u8) -> alloc::Layout {
    use sys_safe::{OperationKind, MAX_OPERATION_CODE, MIN_OPERATION_CODE};
    if !(MIN_OPERATION_CODE..=MAX_OPERATION_CODE).contains(&op) {
        panic!("unknown packet operation code: {op}")
    }
    // SAFETY: validity is checked above
    let kind = unsafe { mem::transmute(op) };
    match kind {
        OperationKind::CreateAccounts => alloc::Layout::new::<sys::tb_create_accounts_result_t>(),
        OperationKind::CreateTransfers => alloc::Layout::new::<sys::tb_create_transfers_result_t>(),
        OperationKind::LookupAccounts => alloc::Layout::new::<sys::tb_account_t>(),
        OperationKind::LookupTransfers => alloc::Layout::new::<sys::tb_transfer_t>(),
        _ => unreachable!(),
    }
}
