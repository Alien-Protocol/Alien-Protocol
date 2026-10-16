# Changelog

All notable changes to Alien Protocol are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for published contract releases.

## Versioning Policy

Contract crate versions signal the state of the public ABI (error codes,
the `DataKey` storage layout, and event topics) rather than the internal
implementation details. The policy is:

- Crates remain at `x.0.0` while the ABI is still experimental and may
change without notice. The `x.`.0` z-release is bumped for any breaking
ABI change.
- The minor version is bumped to `1.0.0` once the ABI is declared frozen.
After that point, breaking ABI changes require a major version bump and a
documented migration path.
- Additive, backward-compatible ABI changes bump the minor version.
- The workspace version in `Cargo.toml` is the single source of truth for
contract crate versions.

The ABI freeze for the first testnet is taking place at `0.1.0`. This is
the deliberate signal that the error codes, `DataKey` layout, and event
topics are stable for the testnet lineup, and it is done together with the
error-code-stability commitment in TODO §1.6. The crates will remain at
`0.1.0` until the ABI is declared fully frozen for mainnet, at which point
they will move to `1.0.0`.

## [Unreleased]

### Added

- A changelog for tracking user-visible contract, interface, and storage changes.

### Changed

- Bumped the workspace contract crate version from `0.0.0` to `0.1.0` to
mark the ABI freeze for the first testnet.
- Recorded the version-bump policy in this changelog.

## [2.0.0]

### Added

- Versioned collateral-vault upgrades with administrator authorization and
  `ContractUpgraded` events.
- Ordered storage migrations with replay, downgrade, and skipped-version
  protection and `StorageMigrated` events.

### Changed

- Advanced both `CURRENT_CONTRACT_VERSION` and
  `CURRENT_STORAGE_SCHEMA_VERSION` to `2` in
  `contracts/collateral-vault/src/upgrade.rs`.
- Defined the storage schema migration from version 1 to version 2 as an
  explicit no-op. Existing state is preserved because version 2 introduces
  migration bookkeeping and invariants without transforming stored values.

[Unreleased]: https://github.com/Alien-Protocol/Alien-Protocol/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/Alien-Protocol/Alien-Protocol/releases/tag/v2.0.0
