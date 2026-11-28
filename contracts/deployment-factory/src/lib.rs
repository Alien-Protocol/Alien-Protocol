#![no_std]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, IntoVal, Symbol, Val, Vec};

#[contract]
pub struct DeploymentFactory;

#[contractimpl]
impl DeploymentFactory {
    /// The address is derived from the signing admin and salt. Each V1
    /// initializer takes the admin first, followed by its configuration.
    pub fn deploy(
        env: Env,
        admin: Address,
        salt: BytesN<32>,
        wasm_hash: BytesN<32>,
        config: Vec<Val>,
    ) -> Address {
        admin.require_auth();
        let contract = env
            .deployer()
            .with_address(admin.clone(), salt)
            .deploy_v2(wasm_hash, (&admin,));
        let mut args = config;
        args.push_front(admin.into_val(&env));
        env.invoke_contract::<Val>(&contract, &Symbol::new(&env, "initialize"), args);
        contract
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collateral_vault::VaultContractClient;
    use soroban_sdk::{testutils::Address as _, Bytes};
    #[test]
    fn failed_init_releases_the_address_for_atomic_retry() {
        let env = Env::default();
        env.mock_all_auths();
        let factory = DeploymentFactoryClient::new(&env, &env.register(DeploymentFactory, ()));
        let admin = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[1; 32]);
        let hash = env.deployer().upload_contract_wasm(Bytes::from_slice(
            &env,
            include_bytes!("../../../target/wasm32v1-none/release/collateral_vault.wasm"),
        ));
        assert!(factory
            .try_deploy(&admin, &salt, &hash, &Vec::new(&env))
            .is_err());
        let config = (
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        )
            .into_val(&env);
        let address = factory.deploy(&admin, &salt, &hash, &config);
        assert_eq!(
            VaultContractClient::new(&env, &address).get_admin(),
            Some(admin)
        );
    }
}
