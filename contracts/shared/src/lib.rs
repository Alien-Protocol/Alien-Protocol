#![no_std]

pub mod bitmask;
pub mod constant;
pub mod errors;
pub mod types;

pub use bitmask::*;
pub use constant::*;
pub use errors::*;
pub use types::*;

#[cfg(test)]
mod tests;

pub mod valuation;

pub fn bind_initializer(env: &soroban_sdk::Env, admin: &soroban_sdk::Address) {
    env.storage()
        .instance()
        .set(&soroban_sdk::symbol_short!("InitOwner"), admin);
}

pub fn take_initializer(env: &soroban_sdk::Env, admin: &soroban_sdk::Address) -> bool {
    let state = env.storage().instance();
    let key = soroban_sdk::symbol_short!("InitOwner");
    if state.get::<_, soroban_sdk::Address>(&key).as_ref() != Some(admin) {
        return false;
    }
    state.remove(&key);
    true
}
