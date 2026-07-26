//! Read-only, paginated view functions.
//!
//! No on-chain write touches these paths.  Every call is bounded by `limit`
//! (capped at [`MAX_PAGE_LIMIT`]) so the ledger footprint, return-value size,
//! and CPU budget are all constant with respect to total protocol users.
//!
//! # Pagination pattern
//! Pass `cursor = 0` for the first page.  The returned `next_cursor` is the
//! offset to supply on the next call.  When `next_cursor == NO_NEXT_CURSOR`
//! (`u32::MAX`) the caller has reached the end of the collection.
//!
//! Off-chain indexers should subscribe to the contract events emitted by
//! `deposit`, `withdraw`, and `seize_collateral` rather than polling
//! `get_positions_page` to reconstruct the full set of active positions.

use crate::errors::VaultError;
use crate::storage;
use crate::types::{AssetsPage, Position, PositionsPage, MAX_PAGE_LIMIT, NO_NEXT_CURSOR};
use soroban_sdk::{Address, Env, Vec};

/// Panic if `limit` is outside the permitted range `1..=MAX_PAGE_LIMIT`.
fn validated_limit(env: &Env, limit: u32) {
    if limit == 0 || limit > MAX_PAGE_LIMIT {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidInputs);
    }
}

/// Return a bounded page of active user positions starting at `cursor`.
///
/// * `cursor` – slot offset into the position index (pass `0` for the first page).
/// * `limit`  – number of positions to return; must be in `1..=MAX_PAGE_LIMIT`.
///
/// The returned [`PositionsPage`] contains:
/// - `positions` – up to `limit` populated `Position` values.
/// - `next_cursor` – next offset if more items follow, `NO_NEXT_CURSOR` if exhausted.
pub fn get_positions_page(env: &Env, cursor: u32, limit: u32) -> PositionsPage {
    validated_limit(env, limit);

    let total = storage::position_count(env);
    let mut positions: Vec<Position> = Vec::new(env);
    let mut i = cursor;

    while i < total && (i - cursor) < limit {
        if let Some(user) = storage::get_position_at(env, i) {
            if let Some(pos) = storage::get_position(env, &user) {
                positions.push_back(pos);
            }
        }
        i += 1;
    }

    let next_cursor = if i < total { i } else { NO_NEXT_CURSOR };

    PositionsPage {
        positions,
        next_cursor,
    }
}

/// Return a bounded page of supported asset addresses starting at `cursor`.
///
/// * `cursor` – slot offset into the supported-asset index.
/// * `limit`  – must be in `1..=MAX_PAGE_LIMIT`.
pub fn get_supported_assets_page(env: &Env, cursor: u32, limit: u32) -> AssetsPage {
    validated_limit(env, limit);

    let total = storage::supported_asset_count(env);
    let mut assets: Vec<Address> = Vec::new(env);
    let mut i = cursor;

    while i < total && (i - cursor) < limit {
        if let Some(asset) = storage::get_supported_asset_at(env, i) {
            assets.push_back(asset);
        }
        i += 1;
    }

    let next_cursor = if i < total { i } else { NO_NEXT_CURSOR };

    AssetsPage {
        assets,
        next_cursor,
    }
}

/// Return a bounded page of asset addresses held by `user` starting at `cursor`.
///
/// * `cursor` – slot offset into the user's asset index.
/// * `limit`  – must be in `1..=MAX_PAGE_LIMIT`.
pub fn get_user_assets_page(env: &Env, user: &Address, cursor: u32, limit: u32) -> AssetsPage {
    validated_limit(env, limit);

    let total = storage::user_asset_count(env, user);
    let mut assets: Vec<Address> = Vec::new(env);
    let mut i = cursor;

    while i < total && (i - cursor) < limit {
        if let Some(asset) = storage::get_user_asset_at(env, user, i) {
            assets.push_back(asset);
        }
        i += 1;
    }

    let next_cursor = if i < total { i } else { NO_NEXT_CURSOR };

    AssetsPage {
        assets,
        next_cursor,
    }
}
