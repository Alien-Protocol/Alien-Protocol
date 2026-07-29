//! Canonical cross-contract interface definitions for Alien Protocol.
//!
//! # Interface versioning
//!
//! The [`INTERFACE_VERSION`] constant marks the current version of this module.
//! Increment it whenever a **breaking** change is made to any trait below
//! (renamed function, changed argument type, changed return type, removed
//! function).  Additive changes (new optional functions) should still bump the
//! version so consumers can detect them.
//!
//! **Rule:** a breaking change to any trait in this file **must** be
//! accompanied by a version bump *and* a corresponding update to every
//! consumer in the same PR.  CI enforces this via `cargo check --workspace`.
//!
//! # Interface ownership
//!
//! | Trait                   | Owned by contract   | Consumed by                    |
//! |-------------------------|---------------------|--------------------------------|
//! | [`OracleInterface`]     | `oracle-adapter`    | `collateral-vault`             |
//! | [`LendingPoolInterface`]| `lending-pool`      | `collateral-vault`             |
//! | [`VaultInterface`]      | `collateral-vault`  | `liquidation-engine`           |

use soroban_sdk::{contractclient, Address, Env};

use crate::types::PriceData;

/// Monotonically increasing version of this interface file.
///
/// Consumers should compare against this constant at compile time using a
/// `const _: () = assert!(shared::interfaces::INTERFACE_VERSION == EXPECTED);`
/// guard in their crate root.
pub const INTERFACE_VERSION: u32 = 1;

/// Interface exposed by the `oracle-adapter` contract.
///
/// The generated [`OracleAdapterClient`] is used by any contract that needs
/// to fetch asset prices.
#[contractclient(name = "OracleAdapterClient")]
pub trait OracleInterface {
    /// Returns the latest stored price for `asset`, or `None` if no price has
    /// ever been published.  Does **not** check staleness.
    fn get_price(env: Env, asset: Address) -> Option<PriceData>;

    /// Returns the latest price for `asset` and panics with
    /// `OracleError::PriceNotFound` or `OracleError::StalePrice` if the price
    /// is absent or older than the configured staleness threshold.
    fn get_price_or_fail(env: Env, asset: Address) -> PriceData;

    /// Returns `true` when the most recently stored price for `asset` is
    /// within the configured staleness threshold.
    fn is_price_fresh(env: Env, asset: Address) -> bool;
}

/// Interface exposed by the `lending-pool` contract.
///
/// The generated [`LendingPoolClient`] is used by `collateral-vault` to check
/// a user's outstanding debt before allowing withdrawals or liquidations.
#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolInterface {
    /// Returns the total outstanding debt (in the pool's accounting unit) for
    /// `user`.  Returns `0` when the user has no open borrow position.
    fn get_user_debt(env: Env, user: Address) -> i128;

    /// Returns `true` when `user`'s collateral-to-debt ratio has fallen below
    /// the liquidation threshold and the position is eligible for liquidation.
    fn is_liquidatable(env: Env, user: Address) -> bool;
}

/// Interface exposed by the `collateral-vault` contract.
///
/// The generated [`VaultClient`] is used by `liquidation-engine` to query
/// collateral values and trigger collateral seizure.
#[contractclient(name = "VaultClient")]
pub trait VaultInterface {
    /// Returns the total USD value of all collateral held by `user`, scaled
    /// by the oracle's price precision (10^7).
    fn get_collateral_value(env: Env, user: Address) -> i128;

    /// Transfers `amount` of `asset` from `user`'s position to
    /// `liquidation_engine`.  Callable only by the registered liquidation engine.
    fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        asset: Address,
        amount: i128,
    );

    /// Returns `true` when `liquidation_engine` is authorised to seize
    /// collateral from `user`.  Callable only by the registered liquidation engine.
    fn authorize_liquidation(env: Env, liquidation_engine: Address, user: Address) -> bool;
}
