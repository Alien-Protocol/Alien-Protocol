use soroban_sdk::{Address, Env};

pub fn get_admin(env: &Env) -> Address {
    env.storage().persistent().get(&"Admin").unwrap()
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().persistent().set(&"Admin", admin);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage().persistent().get(&"Paused").unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().persistent().set(&"Paused", &paused);
}
