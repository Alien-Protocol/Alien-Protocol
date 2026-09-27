# Changelog

All notable changes to Alien Protocol are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Documented the contract and storage-schema migration history.
- Added this changelog as the canonical place for release and migration notes.

## [0.2.0] - Contract version 2

### Changed

- Collateral Vault contract version advanced to `CURRENT_CONTRACT_VERSION = 2`.
- Storage schema advanced to `CURRENT_STORAGE_SCHEMA_VERSION = 2`.
- The `1 → 2` storage migration is intentionally a compatibility-preserving
  no-op: existing storage keys and values remain valid, while the persisted
  schema marker is advanced to `2`.
- Re-running the migration, migrating backwards, or requesting a future schema
  version is rejected by the upgrade module.

The version constants are defined in
`contracts/collateral-vault/src/upgrade.rs`. Migration behavior is covered by
the upgrade tests in `contracts/collateral-vault/src/tests/test_upgrade.rs`.

## [0.1.0] - Initial contract version

### Added

- Initial collateral-vault contract and storage schema.
- Version markers for contract and storage compatibility checks.

[Unreleased]: https://github.com/Alien-Protocol/Alien-Protocol/compare/main...HEAD
[0.2.0]: https://github.com/Alien-Protocol/Alien-Protocol/releases/tag/v0.2.0
[0.1.0]: https://github.com/Alien-Protocol/Alien-Protocol/releases/tag/v0.1.0
