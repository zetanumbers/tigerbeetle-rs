#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Available only with `generated-safe` feature
#[cfg(feature = "generated-safe")]
pub mod generated_safe {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
