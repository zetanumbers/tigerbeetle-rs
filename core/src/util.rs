use std::{marker::PhantomData, mem, pin::Pin, ptr::NonNull, rc::Rc, sync::Arc};

/// Trait that is used to generalize over various smart pointers.
///
/// # Safety
///
/// `into_raw_const_ptr` should pass ownership into the returned const pointer,
/// while `from_raw_const_ptr` should regain ownership from a const pointer.
pub unsafe trait RawConstPtr: std::ops::Deref {
    /// Pass ownership onto a returned const pointer. Returned pointer is
    /// non-null.
    ///
    /// You can safely immutably dereference returned pointer until
    /// you call `<Self as RawConstPtr>::from_raw_const_ptr` on it.
    /// Mutable dereferencing is allowed if `Self: DerefMut`. It is also alowed
    /// to create `Pin<&mut Self::Target>` if `Self` is `Pin<P>`
    /// where `P: DerefMut`.
    ///
    /// Dereference from other threads only if `Self::Target: Sync`.
    fn into_raw_const_ptr(this: Self) -> *const Self::Target;

    /// Regain ownership from a const pointer.
    ///
    /// # Safety
    ///
    /// You can only call this function once on a pointer returned from
    /// `<Self as RawConstPtr>::into_raw_const_ptr`. Can be called from other
    /// thread only if `Self: Send`.
    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self;
}

unsafe impl<T: ?Sized> RawConstPtr for Box<T> {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        Box::into_raw(this).cast_const()
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Box::from_raw(ptr.cast_mut())
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Pin<Box<T>> {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        Box::into_raw_const_ptr(unsafe { Pin::into_inner_unchecked(this) })
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Pin::new_unchecked(Box::from_raw_const_ptr(ptr))
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Arc<T> {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        Arc::into_raw(this)
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Arc::from_raw(ptr)
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Pin<Arc<T>> {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        Arc::into_raw_const_ptr(unsafe { Pin::into_inner_unchecked(this) })
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Pin::new_unchecked(Arc::from_raw_const_ptr(ptr))
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Rc<T> {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        Rc::into_raw(this)
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Rc::from_raw(ptr)
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Pin<Rc<T>> {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        Rc::into_raw_const_ptr(unsafe { Pin::into_inner_unchecked(this) })
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Pin::new_unchecked(Rc::from_raw_const_ptr(ptr))
    }
}

unsafe impl<T: ?Sized> RawConstPtr for &T {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        this as _
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        &*ptr
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Pin<&T> {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        <&T>::into_raw_const_ptr(unsafe { Pin::into_inner_unchecked(this) })
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Pin::new_unchecked(<&T>::from_raw_const_ptr(ptr))
    }
}

unsafe impl<T: ?Sized> RawConstPtr for &mut T {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        (this as *mut Self::Target).cast_const()
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        &mut *ptr.cast_mut()
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Pin<&mut T> {
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        <&mut T>::into_raw_const_ptr(unsafe { Pin::into_inner_unchecked(this) })
    }

    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Pin::new_unchecked(<&mut T>::from_raw_const_ptr(ptr))
    }
}

/// Generalized version of `Vec<T>`, `Arc<[T]>`, `Box<[T]>`, `Rc<[T]>` or other
/// sequential containers where `T` is an element type.
///
/// `S` is marker type for `Send` trait. If it set to [`Sendable`], then
/// [`OwnedSlice`] implements `Send`, or [`Unsendable`] for `!Send`.
/// There is [`SendOwnedSlice`] shortcut if you want sendable owned slice.
pub struct OwnedSlice<T = (), S = Unsendable>
where
    S: SendMarker,
{
    ptr: NonNull<T>,
    /// Some slice containers may allow only invariance over elements `T`
    marker: PhantomData<(fn(T), S)>,
    ctx: SliceHandleContext,
}
pub type SendOwnedSlice<T = ()> = OwnedSlice<T, Sendable>;

unsafe impl<T> Send for SendOwnedSlice<T> {}

/// Do not put constrain over `M` cause we can always send reference to a `Sync`
/// type to another thread.
unsafe impl<T, S> Sync for OwnedSlice<T, S>
where
    T: Sync,
    S: SendMarker,
{
}

#[derive(Clone, Copy)]
struct SliceHandleContext {
    len: usize,
    addend: usize,
    drop: unsafe fn(NonNull<()>, usize, usize),
}

impl<T, S> AsRef<[T]> for OwnedSlice<T, S>
where
    S: SendMarker,
{
    fn as_ref(&self) -> &[T] {
        unsafe { NonNull::slice_from_raw_parts(self.ptr, self.ctx.len).as_ref() }
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
    pub unsafe fn from_raw_parts(
        ptr: NonNull<T>,
        len: usize,
        addend: usize,
        drop: unsafe fn(NonNull<()>, usize, usize),
    ) -> Self {
        OwnedSlice {
            ptr,
            marker: PhantomData,
            ctx: SliceHandleContext { len, addend, drop },
        }
    }

    /// Get erased over `T` owned slice. Lifetimes of `self.as_ref()` references
    /// could be safely extended until owned slice is dropped.
    pub fn erase_type(self) -> OwnedSlice<(), S> {
        OwnedSlice {
            ptr: self.ptr.cast(),
            marker: PhantomData,
            ctx: self.ctx,
        }
    }
}

impl<T> From<SendOwnedSlice<T>> for OwnedSlice<T> {
    fn from(value: SendOwnedSlice<T>) -> Self {
        OwnedSlice {
            ptr: value.ptr,
            marker: PhantomData,
            ctx: value.ctx,
        }
    }
}

impl<T> From<Vec<T>> for SendOwnedSlice<T> {
    fn from(value: Vec<T>) -> Self {
        unsafe fn drop_impl<T>(ptr: NonNull<()>, length: usize, capacity: usize) {
            Vec::<T>::from_raw_parts(ptr.cast().as_ptr(), length, capacity);
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
        unsafe fn drop_impl<P, T>(ptr: NonNull<()>, len: usize, _: usize)
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
        unsafe fn drop_impl<P, T>(ptr: NonNull<()>, len: usize, _: usize)
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
    fn drop(&mut self) {
        unsafe { (self.ctx.drop)(self.ptr.cast(), self.ctx.len, self.ctx.addend) }
    }
}

pub struct Unsendable(PhantomData<*const ()>);
unsafe impl Sync for Unsendable {}

pub struct Sendable(());

pub trait SendMarker: send_marker_seal::Sealed {}
impl SendMarker for Sendable {}
impl SendMarker for Unsendable {}

mod send_marker_seal {
    pub trait Sealed {}
    impl Sealed for super::Sendable {}
    impl Sealed for super::Unsendable {}
}
