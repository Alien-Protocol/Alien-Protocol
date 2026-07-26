use crate::errors::VaultError;
use crate::events;
use crate::storage;
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum Role {
    EmergencyPauser,
    AssetManager,
    RiskManager,
    UpgradeManager,
}

pub fn propose_admin(env: &Env, new_admin: Address) -> Result<(), VaultError> {
    let current_admin = storage::get_admin(env).ok_or(VaultError::InvalidInputs)?;
    current_admin.require_auth();

    if current_admin == new_admin {
        return Err(VaultError::AlreadyAdmin);
    }

    storage::set_pending_admin(env, &new_admin);
    events::AdminProposed {
        pending_admin: new_admin,
    }
    .publish(env);
    Ok(())
}

pub fn accept_admin(env: &Env) -> Result<(), VaultError> {
    let pending_admin = storage::get_pending_admin(env).ok_or(VaultError::NoPendingAdmin)?;
    pending_admin.require_auth();

    let old_admin = storage::get_admin(env).ok_or(VaultError::InvalidInputs)?;
    storage::set_admin(env, &pending_admin);
    storage::clear_pending_admin(env);

    events::AdminAccepted {
        new_admin: pending_admin,
        old_admin,
    }
    .publish(env);
    Ok(())
}

pub fn cancel_admin_transfer(env: &Env) -> Result<(), VaultError> {
    let current_admin = storage::get_admin(env).ok_or(VaultError::InvalidInputs)?;
    current_admin.require_auth();

    if !storage::has_pending_admin(env) {
        return Err(VaultError::NoPendingAdmin);
    }

    let cancelled_by = current_admin;
    storage::clear_pending_admin(env);
    events::AdminTransferCancelled { cancelled_by }.publish(env);
    Ok(())
}

pub fn require_role(env: &Env, role: &Role) {
    let admin = storage::get_admin(env).expect("not initialized");
    admin.require_auth();
    if !storage::has_role(env, role, &admin) {
        let err = match role {
            Role::EmergencyPauser => VaultError::NotEmergencyPauser,
            Role::AssetManager => VaultError::NotAssetManager,
            Role::RiskManager => VaultError::NotRiskManager,
            Role::UpgradeManager => VaultError::NotUpgradeManager,
        };
        soroban_sdk::panic_with_error!(env, err);
    }
}

pub fn require_role_or_admin(env: &Env, caller: &Address, role: &Role) {
    caller.require_auth();
    let admin = storage::get_admin(env);
    let is_admin = admin.as_ref() == Some(caller);
    if is_admin {
        return;
    }
    if !storage::has_role(env, role, caller) {
        let err = match role {
            Role::EmergencyPauser => VaultError::NotEmergencyPauser,
            Role::AssetManager => VaultError::NotAssetManager,
            Role::RiskManager => VaultError::NotRiskManager,
            Role::UpgradeManager => VaultError::NotUpgradeManager,
        };
        soroban_sdk::panic_with_error!(env, err);
    }
}

pub fn grant_role(env: &Env, role: Role, address: Address) -> Result<(), VaultError> {
    let admin = storage::get_admin(env).ok_or(VaultError::Unauthorized)?;
    admin.require_auth();
    storage::set_role(env, &role, &address);
    events::RoleGranted {
        role: role.clone(),
        address,
    }
    .publish(env);
    Ok(())
}

pub fn revoke_role(env: &Env, role: Role, address: Address) -> Result<(), VaultError> {
    let admin = storage::get_admin(env).ok_or(VaultError::Unauthorized)?;
    admin.require_auth();
    storage::remove_role(env, &role, &address);
    events::RoleRevoked {
        role: role.clone(),
        address,
    }
    .publish(env);
    Ok(())
}

pub fn get_pending_admin(env: &Env) -> Option<Address> {
    storage::get_pending_admin(env)
}
