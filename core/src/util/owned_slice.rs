use std::{marker::PhantomData, mem, ptr::NonNull};

use super::{send_marker, RawConstPtr, SendMarker};

/// Generalized version of `Vec<T>`, `Arc<[T]>`, `Box<[T]>`, `Rc<[T]>` or other
/// sequential containers where `T` is an element type. Has transformation from
/// these types.
///
/// `S` is marker type for `Send` trait. If it set to [`Sendable`], then
/// [`OwnedSlice`] implements `Send`, or [`Unsendable`] for `!Send`.
/// There is [`SendOwnedSlice`] shortcut if you want sendable owned slice.
pub struct OwnedSlice<T = Erased, S = send_marker::Unsendable>
where
    S: SendMarker,
{
    ptr: NonNull<T>,
    /// Some slice containers may allow only invariance over elements `T`
    marker: PhantomData<(fn(T), S)>,
    ctx: SliceHandleContext,
}

pub type SendOwnedSlice<T = Erased> = OwnedSlice<T, send_marker::Sendable>;

#[derive(Clone, Copy)]
struct SliceHandleContext {
    len: usize,
    addend: usize,
    drop: unsafe fn(NonNull<Erased>, usize, usize),
}

pub struct AsBytesOwnedSlice<S = send_marker::Unsendable>
where
    S: SendMarker,
{
    owner: OwnedSlice<Erased, S>,
    size_of_element: usize,
}

pub type SendAsBytesOwnedSlice = AsBytesOwnedSlice<send_marker::Sendable>;

pub struct Erased(());

unsafe impl<T> Send for SendOwnedSlice<T> {}

/// Do not put constrain over `M` cause we can always send reference to a `Sync`
/// type to another thread.
unsafe impl<T, S> Sync for OwnedSlice<T, S>
where
    T: Sync,
    S: SendMarker,
{
}

impl<T, S> AsRef<[T]> for OwnedSlice<T, S>
where
    S: SendMarker,
{
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, S> OwnedSlice<T, S>
where
    S: SendMarker,
{
    /// Create owned slice from raw parts given the safety requirements. Used
    /// to implement `From<T>` on `OwnedSlice`
    ///
    /// # Safety
    ///
    /// User must ensure that it is safe to create a immutible slice reference
    /// from `ptr` and `len`. `drop` should be safe to call with arguments
    /// `ptr.cast::<()>()`, `len` and `addend`. Use `SendOwnedSlice` or set
    /// type parameter `S = Sendable`, to indicate that original container can
    /// be sended to another thread to be dropped there.
    #[inline]
    pub unsafe fn from_raw_parts(
        ptr: NonNull<T>,
        len: usize,
        addend: usize,
        drop: unsafe fn(NonNull<Erased>, usize, usize),
    ) -> Self {
        OwnedSlice {
            ptr,
            marker: PhantomData,
            ctx: SliceHandleContext { len, addend, drop },
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { NonNull::slice_from_raw_parts(self.ptr, self.ctx.len).as_ref() }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.ctx.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ctx.len == 0
    }

    /// Get erased over `T` owned slice. Lifetimes of `self.as_ref()` references
    /// could be safely extended until owned slice is dropped.
    pub fn erase_type(self) -> OwnedSlice<Erased, S> {
        let this = mem::ManuallyDrop::new(self);
        OwnedSlice {
            ptr: this.ptr.cast(),
            marker: PhantomData,
            ctx: this.ctx,
        }
    }

    /// Get owned slice with ability to inspect it's bytes.
    pub fn into_as_bytes(self) -> AsBytesOwnedSlice<S>
    where
        T: bytemuck::NoUninit,
    {
        AsBytesOwnedSlice {
            owner: self.erase_type(),
            size_of_element: mem::size_of::<T>(),
        }
    }
}

impl<T> From<SendOwnedSlice<T>> for OwnedSlice<T> {
    fn from(value: SendOwnedSlice<T>) -> Self {
        let value = mem::ManuallyDrop::new(value);
        OwnedSlice {
            ptr: value.ptr,
            marker: PhantomData,
            ctx: value.ctx,
        }
    }
}

impl<T> From<Vec<T>> for SendOwnedSlice<T> {
    fn from(value: Vec<T>) -> Self {
        unsafe fn drop_impl<T>(ptr: NonNull<Erased>, length: usize, capacity: usize) {
            Vec::from_raw_parts(ptr.cast::<T>().as_ptr(), length, capacity);
        }
        let mut v = mem::ManuallyDrop::new(value);
        let len = v.len();
        let capacity = v.capacity();
        unsafe {
            OwnedSlice::from_raw_parts(
                NonNull::new_unchecked(v.as_mut_ptr()),
                len,
                capacity,
                drop_impl::<T>,
            )
        }
    }
}

impl<P, T> From<P> for SendOwnedSlice<T>
where
    P: RawConstPtr<Target = [T]> + Send + 'static,
{
    fn from(value: P) -> Self {
        unsafe fn drop_impl<P, T>(ptr: NonNull<Erased>, len: usize, _: usize)
        where
            P: RawConstPtr<Target = [T]> + 'static,
        {
            P::from_raw_const_ptr(
                NonNull::slice_from_raw_parts(ptr.cast::<T>(), len)
                    .as_ptr()
                    .cast_const(),
            );
        }
        let ptr = unsafe { NonNull::new_unchecked(P::into_raw_const_ptr(value).cast_mut()) };
        unsafe { OwnedSlice::from_raw_parts(ptr.cast(), ptr.len(), 0, drop_impl::<P, T>) }
    }
}

impl<P, T> From<P> for OwnedSlice<T>
where
    P: RawConstPtr<Target = [T]> + 'static,
{
    fn from(value: P) -> Self {
        unsafe fn drop_impl<P, T>(ptr: NonNull<Erased>, len: usize, _: usize)
        where
            P: RawConstPtr<Target = [T]> + 'static,
        {
            P::from_raw_const_ptr(
                NonNull::slice_from_raw_parts(ptr.cast::<T>(), len)
                    .as_ptr()
                    .cast_const(),
            );
        }
        let ptr = unsafe { NonNull::new_unchecked(P::into_raw_const_ptr(value).cast_mut()) };
        unsafe { OwnedSlice::from_raw_parts(ptr.cast(), ptr.len(), 0, drop_impl::<P, T>) }
    }
}

impl<T, S> Drop for OwnedSlice<T, S>
where
    S: SendMarker,
{
    #[inline]
    fn drop(&mut self) {
        unsafe { (self.ctx.drop)(self.ptr.cast(), self.ctx.len, self.ctx.addend) }
    }
}

impl<S> AsRef<[u8]> for AsBytesOwnedSlice<S>
where
    S: SendMarker,
{
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<S> AsBytesOwnedSlice<S>
where
    S: SendMarker,
{
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            NonNull::slice_from_raw_parts(
                self.owner.ptr.cast(),
                self.size_of_element * self.owner.ctx.len,
            )
            .as_ref()
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.owner.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.owner.is_empty()
    }
}
