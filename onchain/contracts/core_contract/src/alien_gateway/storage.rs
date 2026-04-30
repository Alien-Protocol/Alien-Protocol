use soroban_sdk::{Address, BytesN, Env, Symbol};

pub fn stellar_address_key(env: &Env, _addr: &Address) -> Symbol {
    Symbol::new(env, "StellarAddress")
}

pub fn smt_root_key(env: &Env) -> Symbol {
    Symbol::new(env, "SmtRoot")
}

pub fn owner_key(env: &Env) -> Symbol {
    Symbol::new(env, "Owner")
}

pub fn username_key(env: &Env) -> Symbol {
    Symbol::new(env, "Username")
}

pub fn created_at_key(env: &Env, _username_hash: &BytesN<32>) -> Symbol {
    Symbol::new(env, "CreatedAt")
}
