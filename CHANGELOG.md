# Changelog

All notable changes to Alien Protocol are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for published contract releases.

## [Unreleased]

### Added

- A changelog for tracking user-visible contract, interface, and storage changes.
- Implemented the lending pool's supply, borrow, repayment, and
  interest-accrual flows.
- Implemented the liquidation engine, including partial liquidations, a partial
  repayment solver, liquidation bonuses, and close-factor limits.
- Integrated the `shared` crate across the lending pool, liquidation engine,
  and collateral vault for common types and risk calculations.

### Changed

- Unified contract price feeds on the shared `PriceData` type.

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
