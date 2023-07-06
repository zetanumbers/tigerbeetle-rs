use std::{pin::Pin, rc::Rc, sync::Arc};

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
    #[inline]
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        Box::into_raw(this).cast_const()
    }

    #[inline]
    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Box::from_raw(ptr.cast_mut())
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Pin<Box<T>> {
    #[inline]
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        Box::into_raw_const_ptr(unsafe { Pin::into_inner_unchecked(this) })
    }

    #[inline]
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
    #[inline]
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        this as _
    }

    #[inline]
    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        &*ptr
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Pin<&T> {
    #[inline]
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        <&T>::into_raw_const_ptr(unsafe { Pin::into_inner_unchecked(this) })
    }

    #[inline]
    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Pin::new_unchecked(<&T>::from_raw_const_ptr(ptr))
    }
}

unsafe impl<T: ?Sized> RawConstPtr for &mut T {
    #[inline]
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        (this as *mut Self::Target).cast_const()
    }

    #[inline]
    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        &mut *ptr.cast_mut()
    }
}

unsafe impl<T: ?Sized> RawConstPtr for Pin<&mut T> {
    #[inline]
    fn into_raw_const_ptr(this: Self) -> *const Self::Target {
        <&mut T>::into_raw_const_ptr(unsafe { Pin::into_inner_unchecked(this) })
    }

    #[inline]
    unsafe fn from_raw_const_ptr(ptr: *const Self::Target) -> Self {
        Pin::new_unchecked(<&mut T>::from_raw_const_ptr(ptr))
    }
}
