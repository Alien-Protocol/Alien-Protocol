# Collateral Vault Event Schema

This document details the event schema emitted by the Collateral Vault contract. All events are emitted with specific structures that indexers and off-chain consumers can use to track state changes.

## Versioning Policy

Currently, the events are not explicitly versioned in their payloads but are implicitly considered `v1`. Any breaking changes to the event structures in the future will either introduce a new event name (e.g., `DepositedV2`) or use an explicit `version` field. 

## Event Structs

Stellar Soroban events permit up to 4 topics (including the event struct name). The `#[topic]` macro is used to mark specific fields as topics for efficient filtering.

### 1. `Deposited`
Emitted when a user deposits collateral into the vault.
- **Topics**:
  - Event Name (`Deposited`)
  - `user` (`Address`): The account that made the deposit.
  - `asset` (`Address`): The token address deposited.
- **Data**:
  - `amount` (`i128`): The amount of the asset deposited. (Units: dependent on the asset's precision).

### 2. `Withdrawn`
Emitted when a user withdraws collateral from the vault.
- **Topics**:
  - Event Name (`Withdrawn`)
  - `user` (`Address`): The account that made the withdrawal.
  - `asset` (`Address`): The token address withdrawn.
- **Data**:
  - `amount` (`i128`): The amount of the asset withdrawn.

### 3. `CollateralSeized`
Emitted when a liquidation engine seizes a user's collateral.
- **Topics**:
  - Event Name (`CollateralSeized`)
  - `user` (`Address`): The user whose collateral was seized.
  - `asset` (`Address`): The seized token address.
- **Data**:
  - `amount` (`i128`): The amount seized.
  - `liquidation_engine` (`Address`): The address of the liquidation engine performing the seizure.

### 4. `AdminChanged`
Emitted when the contract's admin is updated.
- **Topics**:
  - Event Name (`AdminChanged`)
- **Data**:
  - `old_admin` (`Address`): The previous admin address.
  - `new_admin` (`Address`): The new admin address.

### 5. `LendingPoolUpdated`
Emitted when the connected lending pool is changed.
- **Topics**:
  - Event Name (`LendingPoolUpdated`)
- **Data**:
  - `old_pool` (`Option<Address>`): The previous lending pool address, or `None` if not previously set.
  - `new_pool` (`Address`): The new lending pool address.

### 6. `OracleUpdated`
Emitted when the price oracle is updated.
- **Topics**:
  - Event Name (`OracleUpdated`)
- **Data**:
  - `old_oracle` (`Option<Address>`): The previous oracle address, or `None` if not previously set.
  - `new_oracle` (`Address`): The new oracle address.

### 7. `LiquidationEngineUpdated`
Emitted when the liquidation engine is updated.
- **Topics**:
  - Event Name (`LiquidationEngineUpdated`)
- **Data**:
  - `old_engine` (`Option<Address>`): The previous liquidation engine.
  - `new_engine` (`Address`): The new liquidation engine.

### 8. `PoolUpdated`
Emitted when the pool is updated.
- **Topics**:
  - Event Name (`PoolUpdated`)
- **Data**:
  - `old_pool` (`Option<Address>`): The previous pool.
  - `new_pool` (`Address`): The new pool.

### 9. `AssetAdded`
Emitted when a new collateral asset is supported.
- **Topics**:
  - Event Name (`AssetAdded`)
  - `asset` (`Address`): The address of the added asset.
- **Data**:
  - None.

### 10. `AssetRemoved`
Emitted when a collateral asset is no longer supported.
- **Topics**:
  - Event Name (`AssetRemoved`)
  - `asset` (`Address`): The address of the removed asset.
- **Data**:
  - None.

### 11. `Paused`
Emitted when the vault is paused.
- **Topics**:
  - Event Name (`Paused`)
  - `by` (`Address`): The admin who paused the vault.
- **Data**:
  - None.

### 12. `Unpaused`
Emitted when the vault is unpaused.
- **Topics**:
  - Event Name (`Unpaused`)
  - `by` (`Address`): The admin who unpaused the vault.
- **Data**:
  - None.

### 13. `ContractUpgraded`
Emitted when the contract's WASM is upgraded.
- **Topics**:
  - Event Name (`ContractUpgraded`)
- **Data**:
  - `old_hash` (`soroban_sdk::BytesN<32>`): The previous WASM hash (if available).
  - `new_hash` (`soroban_sdk::BytesN<32>`): The new WASM hash.
