#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, IntoVal};

#[test]
fn test_set_price_requires_feeder_or_admin_auth() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let feeder = Address::generate(&env);
    let stranger = Address::generate(&env);
    let asset = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (admin.clone(), 300_u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &300);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "add_authorized_feeder",
            args: (feeder.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.add_authorized_feeder(&feeder);

    env.mock_auths(&[MockAuth {
        address: &stranger,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "set_price",
            args: (
                stranger.clone(),
                asset.clone(),
                100_i128,
                1000_u64,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    assert!(
        client
            .try_set_price(&stranger, &asset, &100, &1000)
            .is_err()
    );

    env.mock_auths(&[MockAuth {
        address: &feeder,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "set_price",
            args: (
                feeder.clone(),
                asset.clone(),
                100_i128,
                1000_u64,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    assert!(
        client
            .try_set_price(&feeder, &asset, &100, &1000)
            .is_ok()
    );

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, feeder);
}