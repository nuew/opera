//! This crate binds the reference implementation of [RFC 8251] (as modified from [RFC 6716]) to Rust.
//!
//! [RFC 6716]: https://tools.ietf.org/html/rfc6716
//! [RFC 8251]: https://tools.ietf.org/html/rfc8251

#![allow(bad_style)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
