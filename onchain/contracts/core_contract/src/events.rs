use soroban_sdk::{symbol_short, Address, Env, Symbol};

pub const EVT_PRIMARY_SET: Symbol = symbol_short!("PRI_SET");
pub const EVT_WALLET_ADD: Symbol = symbol_short!("WAL_ADD");
pub const EVT_WALLET_REM: Symbol = symbol_short!("WAL_REM");
pub const EVT_ESCROW_NEW: Symbol = symbol_short!("ESC_NEW");
pub const EVT_ESCROW_REL: Symbol = symbol_short!("ESC_REL");
pub const EVT_ESCROW_RFD: Symbol = symbol_short!("ESC_RFD");
pub const EVT_OWN_TRF: Symbol = symbol_short!("OWN_TRF");
pub const EVT_SEND: Symbol = symbol_short!("SEND");
#[allow(deprecated)]
pub fn emit_primary_set(env: &Env, owner: &Address, address: &Address) {
    env.events()
        .publish((EVT_PRIMARY_SET,), (owner.clone(), address.clone()));
}

#[allow(deprecated)]
pub fn emit_wallet_added(env: &Env, label: &Symbol) {
    env.events().publish((EVT_WALLET_ADD,), (label.clone(),));
}

#[allow(deprecated)]
pub fn emit_wallet_removed(env: &Env, label: &Symbol) {
    env.events().publish((EVT_WALLET_REM,), (label.clone(),));
}

#[allow(deprecated)]
pub fn emit_escrow_created(
    env: &Env,
    id: u32,
    asset: &Address,
    amount: i128,
    recipient: &Address,
    release_at: u64,
) {
    env.events().publish(
        (EVT_ESCROW_NEW,),
        (id, asset.clone(), amount, recipient.clone(), release_at),
    );
}

#[allow(deprecated)]
pub fn emit_escrow_released(env: &Env, id: u32, recipient: &Address, amount: i128) {
    env.events()
        .publish((EVT_ESCROW_REL,), (id, recipient.clone(), amount));
}

#[allow(deprecated)]
pub fn emit_escrow_refunded(env: &Env, id: u32, owner: &Address, amount: i128) {
    env.events()
        .publish((EVT_ESCROW_RFD,), (id, owner.clone(), amount));
}

#[allow(deprecated)]
pub fn emit_ownership_transferred(env: &Env, old_owner: &Address, new_owner: &Address) {
    env.events()
        .publish((EVT_OWN_TRF,), (old_owner.clone(), new_owner.clone()));
}
#[allow(deprecated)]
pub fn emit_send(env: &Env, asset: &Address, amount: i128, recipient: &Address) {
    env.events()
        .publish((EVT_SEND,), (asset.clone(), amount, recipient.clone()));
}
