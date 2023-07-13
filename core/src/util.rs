//! Helpful abstractions to generalize over various types

mod owned_slice;
mod raw_const_ptr;
pub mod send_marker;

pub use owned_slice::*;
pub use raw_const_ptr::RawConstPtr;
pub use send_marker::SendMarker;
