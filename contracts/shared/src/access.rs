//! Shared admin-access control helpers.
///
/// These helpers are deliberately storage-agnostic: they do not touch
/// contract storage or error enums directly. Each contract keeps its own
/// `DataKey` enum, its own error type, and its own storage scope (persistent
/// vs instance), and simply passes the relevant values in.
///
/// This avoids the blockers around per-contract `DataKey`/error types and
/// differing storage scopes while still eliminating the copy-pasted
/// admin-authorization prologue.

use soracle_sdk::Address;

/// Require that a contract is initialized and that the admin has
/// authorized the current invocation.
///
/// `admin` is the admin address fetched from the contract's own storage
/// (e.g. `storage::get_admin(&env)`). `_not_initialized` is the contract's
/// own error to return when the admin has not been set yet.
///
/// Returns the authenticated admin address on success.
pub fn require_admin<E>(admin: Option<Address>, not_initialized: E) -> Result<Address, E> {
    let admin = admin.ok_or(not_initialized)?;
    admin.require_auth();
    Ok(admin)
}

/// Ensure that a proposed new admin differs from the current admin.
///
/// This guards the single-step `set_admin` flow against a self-collision
/// where the admin attempts to re-assign the same address. `already` is
/// the contract's own error to return in that case.
pub fn ensure_new_admin<E>(
    current: &Address,
    new: &Address,
    already: E,
) -> Result<(), E> {
    if current == new {
        return Err(already);
    }
    Ok(())
}

/// Read a pause bit from a bitmask.
///
/// Returns `true` when the bit corresponding to `flag` is set in `bitmask`.
pub fn is_paused(bitmask: u32, flag: u32) -> bool {
    bitmask & flag != 0
}

/// Set or clear a pause bit in a bitmask.
///
/// When `paused` is `true`, the bit corresponding to `flag` is set; otherwise
/// it is cleared. Other bits are left untouched.
pub fn set_pause_bit(bitmask: u32, flag: u32, paused: bool) -> u32 {
    if paused {
        bitmask | flag
    } else {
        bitmask & !flag
    }
}
