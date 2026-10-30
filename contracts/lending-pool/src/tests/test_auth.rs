#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{contract, contractimpl, token, Address, Env, IntoVal};

#[contract]
pub struct MockVaultContract;

#[contractimpl]
impl MockVaultContract {
    pub fn get_position(env: Env, user: Address) -> shared::Position {
        let mut collateral = soroban_sdk::Vec::new(&env);
        collateral.push_back(shared::CollateralAsset {
            asset: Address::generate(&env),
            amount: 100_000_000,
        });
        shared::Position { user, collateral }
    }

    pub fn get_asset_config(_env: Env, _asset: Address) -> shared::AssetConfig {
        shared::AssetConfig {
            token_decimals: 7,
            oracle_price_decimals: 7,
            max_ltv_bps: 8_000,
            liquidation_threshold_bps: 8_500,
        }
    }

    pub fn get_collateral_value(_env: Env, _user: Address) -> i128 {
        100_000_000
    }

    pub fn get_health_factor(_env: Env, _user: Address) -> i128 {
        15_000
    }
}

fn setup_auth_pool_env() -> (
    Env,
    PoolContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();

    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let vault_id = env.register(MockVaultContract, ());

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin.clone());
    let borrow_asset = sac.address();
    let token_client = token::Client::new(&env, &borrow_asset);
    let token_admin_client = token::StellarAssetClient::new(&env, &borrow_asset);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "initialize",
            args: (
                admin.clone(),
                vault_id.clone(),
                oracle.clone(),
                borrow_asset.clone(),
                1000_u32,
            )
                .into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &vault_id, &oracle, &borrow_asset, &1000);

    (
        env,
        client,
        admin,
        user,
        borrow_asset,
        token_admin,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_borrow_requires_user_auth() {
    let (env, client, _admin, user, borrow_asset, token_admin, _token_client, token_admin_client) =
        setup_auth_pool_env();

    // Supplier supplies liquidity
    let supplier = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &token_admin,
        invoke: &MockAuthInvoke {
            contract: &borrow_asset,
            fn_name: "mint",
            args: (supplier.clone(), 20_000_000_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    token_admin_client.mint(&supplier, &20_000_000);

    env.mock_auths(&[MockAuth {
        address: &supplier,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "supply",
            args: (supplier.clone(), 20_000_000_i128).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &borrow_asset,
                fn_name: "transfer",
                args: (supplier.clone(), client.address.clone(), 20_000_000_i128).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    client.supply(&supplier, &20_000_000);

    // 1. Without auth, try_borrow fails
    let res_no_auth = client.try_borrow(&user, &borrow_asset, &15_000_000);
    assert!(res_no_auth.is_err());

    // 2. With wrong signer auth, try_borrow fails
    let other = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &other,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "borrow",
            args: (user.clone(), borrow_asset.clone(), 15_000_000_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let res_wrong_auth = client.try_borrow(&user, &borrow_asset, &15_000_000);
    assert!(res_wrong_auth.is_err());

    // 3. With user auth, borrow succeeds
    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "borrow",
            args: (user.clone(), borrow_asset.clone(), 15_000_000_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.borrow(&user, &borrow_asset, &15_000_000);

    // 4. Verify auths tree recorded user invocation
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, user);
}

#[test]
fn test_repay_for_requires_payer_not_user() {
    let (env, client, _admin, user, borrow_asset, token_admin, _token_client, token_admin_client) =
        setup_auth_pool_env();

    let payer = Address::generate(&env);

    // Fund supplier and supply liquidity
    let supplier = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &token_admin,
        invoke: &MockAuthInvoke {
            contract: &borrow_asset,
            fn_name: "mint",
            args: (supplier.clone(), 30_000_000_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    token_admin_client.mint(&supplier, &30_000_000);

    env.mock_auths(&[MockAuth {
        address: &supplier,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "supply",
            args: (supplier.clone(), 30_000_000_i128).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &borrow_asset,
                fn_name: "transfer",
                args: (supplier.clone(), client.address.clone(), 30_000_000_i128).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    client.supply(&supplier, &30_000_000);

    // User borrows 15_000_000 (> MIN_REMAINING_DEBT)
    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "borrow",
            args: (user.clone(), borrow_asset.clone(), 15_000_000_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.borrow(&user, &borrow_asset, &15_000_000);

    // Mint tokens to payer so payer can repay_for
    env.mock_auths(&[MockAuth {
        address: &token_admin,
        invoke: &MockAuthInvoke {
            contract: &borrow_asset,
            fn_name: "mint",
            args: (payer.clone(), 15_000_000_i128).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    token_admin_client.mint(&payer, &15_000_000);

    // Repay full debt (15_000_000) so remaining debt is 0
    let repay_amount = 15_000_000_i128;

    // 1. Attempt repay_for without auth should fail
    let res_no_auth = client.try_repay_for(&payer, &user, &borrow_asset, &repay_amount);
    assert!(res_no_auth.is_err());

    // 2. Attempt repay_for with ONLY user auth (no payer auth) should fail because payer.require_auth() is enforced
    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "repay_for",
            args: (
                payer.clone(),
                user.clone(),
                borrow_asset.clone(),
                repay_amount,
            )
                .into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &borrow_asset,
                fn_name: "transfer",
                args: (payer.clone(), client.address.clone(), repay_amount).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    let res_user_auth_only = client.try_repay_for(&payer, &user, &borrow_asset, &repay_amount);
    assert!(res_user_auth_only.is_err());

    // 3. repay_for succeeds with ONLY payer auth (user auth is NOT required)
    env.mock_auths(&[MockAuth {
        address: &payer,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "repay_for",
            args: (
                payer.clone(),
                user.clone(),
                borrow_asset.clone(),
                repay_amount,
            )
                .into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &borrow_asset,
                fn_name: "transfer",
                args: (payer.clone(), client.address.clone(), repay_amount).into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);
    client.repay_for(&payer, &user, &borrow_asset, &repay_amount);

    // 4. Verify auths tree: payer authorized the call with transfer sub-invocation; user did NOT authorize
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, payer);
    assert_eq!(auths[0].1.sub_invocations.len(), 1);
}
