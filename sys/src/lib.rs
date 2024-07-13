#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)] // u128

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Available only with `generated-safe` feature
#[cfg(feature = "generated-safe")]
#[allow(clippy::unnecessary_cast, clippy::assign_op_pattern)]
#[doc(hidden)]
pub mod generated_safe {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
