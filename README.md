# opi

[![Build Status](https://travis-ci.com/nuew/opi.svg?branch=master)][travis]
[![Docs.rs](https://docs.rs/opi/badge.svg)][docs]
[![Crates.io](https://img.shields.io/crates/v/opi.svg)][cargo]
[![License](https://img.shields.io/github/license/nuew/opi.svg)][license]

A pure-rust Opus decoding library, intending compliance with [RFC 6716]
\(as modified by [RFC 8251]), and [RFC 7845].

Partial support for [RFC 8486], (as it modifies [RFC 7845]) is intended, but
full support for Ambisonics is not currently a goal.

# Licensing

Opi is licensed under the ISC [license]. Some portions of Opi are derived from
or directly inspired by the reference implementation included in [RFC 6716]
\(as modified by [RFC 8251]), which is licensed under the [Simplified BSD
License].

[travis]: https://travis-ci.com/nuew/opi
[docs]: https://docs.rs/opi/
[cargo]: https://crates.io/crates/opi/
[license]: https://github.com/nuew/opi/blob/master/LICENSE
[Simplified BSD License]: https://github.com/nuew/opi/blob/master/doc/COPYING.opus-rfc6716
[RFC 6716]: https://tools.ietf.org/html/rfc6716
[RFC 7845]: https://tools.ietf.org/html/rfc7845
[RFC 8251]: https://tools.ietf.org/html/rfc8251
[RFC 8486]: https://tools.ietf.org/html/rfc8486
