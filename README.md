# jmap-tools

[![crates.io](https://img.shields.io/crates/v/jmap-tools)](https://crates.io/crates/jmap-tools)
[![build](https://github.com/stalwartlabs/jmap-tools/actions/workflows/rust.yml/badge.svg)](https://github.com/stalwartlabs/jmap-tools/actions/workflows/rust.yml)
[![docs.rs](https://img.shields.io/docsrs/jmap-tools)](https://docs.rs/jmap-tools)
[![crates.io](https://img.shields.io/crates/l/jmap-tools)](http://www.apache.org/licenses/LICENSE-2.0)

_jmap-tools_ is a Rust library for working with [JMAP](https://jmap.io/) objects. It provides a straightforward way to parse, inspect, and modify JMAP data structures.

In addition to parsing, the library includes built-in support for [JSON Pointer (RFC 6901)](https://datatracker.ietf.org/doc/html/rfc6901), which lets you:

- **Query** deeply nested elements within arbitrary Rust objects using JSON Pointer paths.
- **Patch** object elements in place, replacing or updating values without rewriting the entire structure.

This makes it easy to build servers, clients, or tools that need to manipulate JMAP objects reliably and efficiently.

## Usage

```rust
...
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Funding

Part of the development of this library was funded through the [NGI Zero Core](https://nlnet.nl/NGI0/), a fund established by [NLnet](https://nlnet.nl/) with financial support from the European Commission's programme, under the aegis of DG Communications Networks, Content and Technology under grant agreement No 101092990.

If you find this library useful you can help by [becoming a sponsor](https://opencollective.com/stalwart). Thank you!

## Copyright

Copyright (C) 2020, Stalwart Labs LLC
