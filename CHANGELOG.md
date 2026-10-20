# Changelog

All notable changes to Alien Protocol are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for published contract releases.

## [Unreleased]

### Added

- A changelog for tracking user-visible contract, interface, and storage changes.
- `lending-pool` public interface implemented: `supply`, `borrow`,
  `repay`/`repay_for`, and per-position interest accrual via `accrue_interest`,
  together with liquidity accounting (total/available supply and utilization).
- `liquidation-engine` public interface implemented: `liquidate`, the partial
  repayment solver that targets a healthy post-liquidation health factor, the
  liquidation bonus calculation, and the close-factor cap on repaid debt.

### Changed

- Wired the `shared` crate into `collateral-vault`, `lending-pool`, and
  `liquidation-engine`, replacing per-contract copies of constants, types, and
  math helpers with their `shared` equivalents.
- Unified the oracle price type on `shared::PriceData` so the vault, pool, and
  liquidation engine decode `get_price`/`get_price_or_fail` consistently.

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
