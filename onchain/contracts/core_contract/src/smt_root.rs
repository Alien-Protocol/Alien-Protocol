use soroban_sdk::{BytesN, Env};

use crate::events::EVENT_ROOT_UPD;
use crate::storage::DataKey;

pub struct SmtRoot;

impl SmtRoot {
    #[allow(dead_code)]
    pub fn update_root(env: &Env, new_root: BytesN<32>) {
        let old_root: Option<BytesN<32>> = env.storage().instance().get(&DataKey::SmtRoot);

        env.storage().instance().set(&DataKey::SmtRoot, &new_root);

        #[allow(deprecated)]
        env.events().publish((EVENT_ROOT_UPD,), (old_root, new_root));
    }

    pub fn get_root(env: Env) -> Option<BytesN<32>> {
        env.storage().instance().get(&DataKey::SmtRoot)
    }
}
