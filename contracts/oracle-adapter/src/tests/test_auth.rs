#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, IntoVal};

#[test]
fn test_set_price_requires_feeder_or_admin_auth() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000_000_000);

    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let feeder = Address::generate(&env);
    let asset = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "initialize",
            args: (admin.clone(), 300_u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &300);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "add_authorized_feeder",
            args: (feeder.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.add_authorized_feeder(&feeder);

    // 1. Unauthorized stranger fails
    let stranger = Address::generate(&env);
    let res_stranger = client.try_set_price(&stranger, &asset, &100, &1000);
    assert!(res_stranger.is_err());

    // 2. Feeder without auth fails
    let res_no_auth = client.try_set_price(&feeder, &asset, &100, &1000);
    assert!(res_no_auth.is_err());

    // 3. Feeder with MockAuth succeeds
    env.mock_auths(&[MockAuth {
        address: &feeder,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "set_price",
            args: (feeder.clone(), asset.clone(), 100_i128, 1000_u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res_ok = client.try_set_price(&feeder, &asset, &100, &1000);
    assert!(res_ok.is_ok());

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, feeder);
}
