#![forbid(unsafe_code)]
#![deny(
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    rustdoc,
    missing_copy_implementations,
    missing_debug_implementations
)]
#![warn(
    unused,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    variant_size_differences
)]

mod error;
mod slice_ext;

pub mod channel;
pub mod ogg;
pub mod packet;

pub use self::error::{Error, Result};
