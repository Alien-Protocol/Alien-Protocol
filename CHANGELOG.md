# Changelog

All notable changes to Alien Protocol are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
for published contract releases.

## Contract crate versioning policy

- `0.1.0` marks the first testnet ABI freeze, subject to maintainer approval:
  contract error codes, persistent `DataKey` layouts, and event topics are part
  of the compatibility commitment. Do not renumber existing error codes after
  testnet (see `docs/TODO.md`).
- Follow Semantic Versioning for subsequent releases: patch versions cover
  compatible fixes; pre-1.0 minor versions may introduce breaking changes with
  documented migration guidance. At and after 1.0.0, use a major version for
  incompatible changes and a minor version for compatible additions.
- Workspace crate versions are distinct from on-chain contract and storage
  schema version counters.

## [Unreleased]

### Tooling

- Added Caatinga configuration for building, deploying, tracking, and checking the five Soroban contracts on testnet, with pinned CLI dependencies and a deployment guide.
- Updated the pinned Rust toolchain and CI to a Caatinga-supported version.
- Deployed the five contracts to testnet and recorded their addresses and WASM hashes in `docs/deployment.md`.

### Audit corrections

- Bound initialization to a constructor-supplied owner on all four contracts and added an atomic deployment factory.
- Vault version 3 requires complete collateral configuration at listing and enforces token decimals. It indexes positions in separate ledger entries and supports resumable migration from the legacy vector.
- Unified valuation and seizure conversion with wide intermediates. Multi-asset liquidations use one pre-settlement health check. Unavailable risk data propagates to eligibility callers.
- Enforced vault borrowing pauses, pool rotation checks, withdrawal preview preconditions and oracle source and timestamp checks.
- Vault upgrade finalization verifies the installed hash and release version. See the [operating guide](docs/contracts.md) for ABI and migration details.

### Contract integration and accounting

- Distributed collected borrower interest through liquidity-provider shares. Existing supply balances are recognized at par.
- Capped underwater settlements at the collateral value available for repayment plus bonus. Residual debt stays with the borrower.
- Added paginated position reads and a vault health quote for a supplied debt.
- Required a successful debt lookup for withdrawals and seven-decimal token metadata at pool initialization.
- Preserved fractional interest in an optional per-borrower storage entry. Existing debt records keep their shape.
- Routed liquidation payment directly from the signer to the pool. Wallets should simulate the updated authorization tree.
- Added four-contract integration checks for settlement, stale prices and rollback, with a release-WASM test target.

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
