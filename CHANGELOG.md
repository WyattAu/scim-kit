# Changelog

All notable changes are documented here. Versions follow
[semver](https://semver.org).

## [1.0.0] - 2026-10-29

### Declared stable

First 1.0. `scim-kit` provisions production identities; this release commits
the RFC 7644 protocol types, filtering, pagination, patching, and the optional
axum router as API-stable.

### Guarantees from here on

- Breaking changes to public items require a minor bump plus a `#[deprecated]`
  window of at least one minor release.
- MSRV bumps are minor releases, not patch releases.
- `#![deny(missing_docs)]` and `#![forbid(unsafe_code)]` are enforced.
- Wire shapes for the core schemas are contractual: any 1.x client interops
  with any 1.x server.

### Explicitly not guaranteed

- New SCIM resource type helpers may arrive in minors; the core
  User/Group/Enterprise types and filter semantics will not change.

