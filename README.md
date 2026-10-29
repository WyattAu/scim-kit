# scim-kit
[![Estate integration](https://github.com/WyattAu/estate-integration/actions/workflows/ci.yml/badge.svg)](https://github.com/WyattAu/estate-integration/actions/workflows/ci.yml)
[![docs.rs](https://docs.rs/scim-kit/badge.svg)](https://docs.rs/scim-kit)
[![crates.io](https://img.shields.io/crates/v/scim-kit.svg)](https://crates.io/crates/scim-kit)

SCIM 2.0 provisioning toolkit for Rust: protocol types, filtering, pagination,
and patching (RFC 7644).

Part of the WyattAu kit estate: exercised by the
[estate-integration](https://github.com/WyattAu/estate-integration) composing
suite against the estate's gate matrix
([engineering-standards](https://github.com/WyattAu/engineering-standards)).

## Install

```toml
[dependencies]
scim-kit = "0.1"
```

## Stability

`scim-kit` is 1.0. Breaking changes require a minor bump and a `#[deprecated]`
window of at least one minor release; MSRV bumps are minor releases, never
patch releases. `missing_docs` is denied and `unsafe` is forbidden. Wire shapes
for the core schemas are contractual: any 1.x client interops with any 1.x
server. New resource type helpers may arrive in minors.

## License

MIT OR Apache-2.0.
