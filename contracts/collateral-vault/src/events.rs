use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Deposited {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
}

impl Deposited {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            soroban_sdk::vec![env, soroban_sdk::Symbol::new(env, "Deposited")],
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetRemoved {
    pub asset: Address,
}

impl AssetRemoved {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            soroban_sdk::vec![env, soroban_sdk::Symbol::new(env, "AssetRemoved")],
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
}

impl AdminChanged {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            soroban_sdk::vec![env, soroban_sdk::Symbol::new(env, "AdminChanged")],
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Paused {
    pub paused: bool,
}

impl Paused {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            soroban_sdk::vec![env, soroban_sdk::Symbol::new(env, "Paused")],
            self.clone(),
        );
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Unpaused {
    pub paused: bool,
}

impl Unpaused {
    pub fn publish(&self, env: &Env) {
        env.events().publish(
            soroban_sdk::vec![env, soroban_sdk::Symbol::new(env, "Unpaused")],
            self.clone(),
        );
    }
}



