use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};

/// Event emitted when a username is successfully deployed.
pub const USERNAME_DEPLOYED: Symbol = symbol_short!("USR_DEP");
/// Event emitted when ownership of a username is transferred.
pub const OWNERSHIP_TRANSFERRED: Symbol = symbol_short!("OWN_TRF");
/// Event emitted when a role is granted to an address.
pub const ROLE_GRANTED: Symbol = symbol_short!("ROLE_GNT");

/// Emits an event when a new username is deployed.
#[allow(deprecated)]
/// Emits a username deployment event.
pub fn emit_username_deployed(
    env: &Env,
    username_hash: &BytesN<32>,
    owner: &Address,
    registered_at: u64,
) {
    env.events().publish(
        (USERNAME_DEPLOYED,),
        (username_hash.clone(), owner.clone(), registered_at),
    );
}

/// Emits an event when username ownership is transferred to a new owner.
#[allow(dead_code)]
#[allow(deprecated)]
/// Emits an ownership transfer event.
pub fn emit_ownership_transferred(
    env: &Env,
    username_hash: &BytesN<32>,
    old_owner: &Address,
    new_owner: &Address,
) {
    env.events().publish(
        (OWNERSHIP_TRANSFERRED,),
        (username_hash.clone(), old_owner.clone(), new_owner.clone()),
    );
}
