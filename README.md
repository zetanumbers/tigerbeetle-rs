# tigerbeetle-rs

Unofficial [tigerbeetle] bindings for Rust.

## Status

Because this tigerbeetle client library implementation is not a part of the official tigerbeetle repo, it is hard to ensure and keep some of rust safety guarantees from the outside.
For that reason I invite people to contribute to this repo or finally develop the official rust client library.

## Repo Overview

The repository hosts the following libraries:

 * [![Crates.io](https://img.shields.io/crates/v/tigerbeetle-unofficial.svg?label=tigerbeetle-unofficial)](https://crates.io/crates/tigerbeetle-unofficial)
   [![docs.rs](https://docs.rs/tigerbeetle-unofficial/badge.svg)](https://docs.rs/tigerbeetle-unofficial/) - Safe high-level async bindings. Implemented with `#![forbid(unsafe_code)]` upon `tigerbeetle-unofficial-core`.
 * [![Crates.io](https://img.shields.io/crates/v/tigerbeetle-unofficial-core.svg?label=tigerbeetle-unofficial-core)](https://crates.io/crates/tigerbeetle-unofficial-core)
   [![docs.rs](https://docs.rs/tigerbeetle-unofficial-core/badge.svg)](https://docs.rs/tigerbeetle-unofficial-core/) - Safe low-level callback-based async bindings.
 * [![Crates.io](https://img.shields.io/crates/v/tigerbeetle-unofficial-sys.svg?label=tigerbeetle-unofficial-sys)](https://crates.io/crates/tigerbeetle-unofficial-sys)
   [![docs.rs](https://docs.rs/tigerbeetle-unofficial-sys/badge.svg)](https://docs.rs/tigerbeetle-unofficial-sys/) - Unsafe native bindings.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE.APACHE](LICENSE.APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE.MIT](LICENSE.MIT) or
   https://opensource.org/licenses/MIT)

at your option.

[tigerbeetle]: https://tigerbeetle.com/
