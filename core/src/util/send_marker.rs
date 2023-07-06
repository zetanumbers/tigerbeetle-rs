use std::marker::PhantomData;

pub struct Unsendable(PhantomData<*const ()>);

unsafe impl Sync for Unsendable {}

pub struct Sendable(());

pub trait SendMarker: send_marker_seal::Sealed {}

impl SendMarker for Sendable {}

impl SendMarker for Unsendable {}

pub(crate) mod send_marker_seal {
    pub trait Sealed {}
    impl Sealed for super::Sendable {}
    impl Sealed for super::Unsendable {}
}
