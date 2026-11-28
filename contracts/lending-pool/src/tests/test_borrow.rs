#![cfg(test)]

use super::super::*;
use crate::errors::PoolError;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env, Symbol, Vec};

#[contract]
struct MockBorrowVault;

#[contractimpl]
impl MockBorrowVault {
    pub fn set_collateral(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        value: i128,
        max_ltv_bps: u32,
        liquidation_threshold_bps: u32,
    ) {
        let mut assets: Vec<shared::CollateralAsset> = env
            .storage()
            .persistent()
            .get(&user)
            .unwrap_or(Vec::new(&env));
        assets.push_back(shared::CollateralAsset {
            asset: asset.clone(),
            amount,
        });
        env.storage().persistent().set(&user, &assets);
        env.storage().persistent().set(&(user, 0u32), &value);
        env.storage().persistent().set(
            &asset,
            &shared::AssetConfig {
                token_decimals: 7,
                oracle_price_decimals: 7,
                max_ltv_bps,
                liquidation_threshold_bps,
            },
        );
    }

    pub fn can_originate(_env: Env, _pool: Address) -> bool {
        true
    }
    pub fn get_position(env: Env, user: Address) -> shared::Position {
        let collateral = env
            .storage()
            .persistent()
            .get(&user)
            .unwrap_or(Vec::new(&env));
        shared::Position { user, collateral }
    }

    pub fn get_asset_config(env: Env, asset: Address) -> shared::AssetConfig {
        env.storage().persistent().get(&asset).unwrap()
    }

    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        env.storage().persistent().get(&(user, 0u32)).unwrap_or(0)
    }

    pub fn get_health_factor(_env: Env, _user: Address) -> i128 {
        10_000
    }
}

struct Fixture {
    env: Env,
    pool: PoolContractClient<'static>,
    vault: MockBorrowVaultClient<'static>,
    user: Address,
    asset: Address,
    token: token::Client<'static>,
    token_admin: token::StellarAssetClient<'static>,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|ledger| ledger.timestamp = 1_000);
        let admin = Address::generate(&env);
        let pool_id = env.register(PoolContract, (&admin,));
        let pool = PoolContractClient::new(&env, &pool_id);
        let vault_id = env.register(MockBorrowVault, ());
        let vault = MockBorrowVaultClient::new(&env, &vault_id);

        let user = Address::generate(&env);
        let oracle = Address::generate(&env);
        let token_admin_id = Address::generate(&env);
        let asset = env
            .register_stellar_asset_contract_v2(token_admin_id)
            .address();
        let token = token::Client::new(&env, &asset);
        let token_admin = token::StellarAssetClient::new(&env, &asset);
        pool.initialize(&admin, &vault_id, &oracle, &asset, &500);
        Self {
            env,
            pool,
            vault,
            user,
            asset,
            token,
            token_admin,
        }
    }

    fn collateral(&self) {
        let collateral_asset = Address::generate(&self.env);
        self.vault.set_collateral(
            &self.user,
            &collateral_asset,
            &100_000,
            &100_000,
            &7_000,
            &8_000,
        );
    }

    fn liquidity(&self, amount: i128) {
        let supplier = Address::generate(&self.env);
        self.token_admin.mint(&supplier, &amount);
        self.pool.supply(&supplier, &amount);
    }
}

#[test]
fn test_borrow_success_transfers_and_records_principal() {
    let f = Fixture::new();
    f.collateral();
    f.liquidity(100_000);
    f.pool.borrow(&f.user, &f.asset, &70_000);
    assert_eq!(f.token.balance(&f.user), 70_000);
    assert_eq!(f.token.balance(&f.pool.address), 30_000);
    let debt = f.pool.get_debt(&f.user);
    assert_eq!(debt.principal, 70_000);
    assert_eq!(debt.accrued_interest, 0);
    assert_eq!(debt.interest_rate_bps, 500);
    assert_eq!(debt.last_accrual_at, 1_000);
    assert_eq!(
        f.env
            .as_contract(&f.pool.address, || storage::get_total_borrowed(&f.env)),
        70_000
    );
}

#[test]
fn test_borrow_zero_or_negative_fails() {
    let f = Fixture::new();
    assert_eq!(
        f.pool.try_borrow(&f.user, &f.asset, &0),
        Err(Ok(PoolError::InvalidAmount))
    );
    assert_eq!(
        f.pool.try_borrow(&f.user, &f.asset, &-1),
        Err(Ok(PoolError::InvalidAmount))
    );
}

#[test]
fn test_borrow_wrong_asset_fails() {
    let f = Fixture::new();
    assert_eq!(
        f.pool.try_borrow(&f.user, &Address::generate(&f.env), &1),
        Err(Ok(PoolError::UnsupportedAsset))
    );
}

#[test]
fn test_borrow_when_paused_fails() {
    let f = Fixture::new();
    f.collateral();
    f.liquidity(100_000);
    f.pool
        .pause_operation(&PauseFlag::Borrow, &Symbol::new(&f.env, "test"));
    assert_eq!(
        f.pool.try_borrow(&f.user, &f.asset, &1),
        Err(Ok(PoolError::BorrowPaused))
    );
}

#[test]
fn test_borrow_exceeds_ltv_limit_fails() {
    let f = Fixture::new();
    f.collateral();
    f.liquidity(100_000);
    assert_eq!(
        f.pool.try_borrow(&f.user, &f.asset, &70_001),
        Err(Ok(PoolError::ExceedsBorrowLimit))
    );
    assert_eq!(
        f.env
            .as_contract(&f.pool.address, || storage::get_total_borrowed(&f.env)),
        0
    );
    assert_eq!(f.token.balance(&f.user), 0);
}

#[test]
fn test_borrow_exceeds_available_liquidity_fails() {
    let f = Fixture::new();
    f.collateral();
    f.liquidity(10_000);
    assert_eq!(
        f.pool.try_borrow(&f.user, &f.asset, &10_001),
        Err(Ok(PoolError::InsufficientLiquidity))
    );
}

#[test]
fn test_borrow_without_collateral_fails() {
    let f = Fixture::new();
    f.liquidity(100_000);
    assert_eq!(
        f.pool.try_borrow(&f.user, &f.asset, &1),
        Err(Ok(PoolError::NoCollateral))
    );
}

#[test]
fn test_calculate_limit_uses_max_ltv_not_liquidation_threshold() {
    let f = Fixture::new();
    f.collateral();
    assert_eq!(f.pool.calculate_limit(&f.user), 70_000);
}

#[test]
fn test_calculate_limit_uses_lowest_max_ltv_across_collateral() {
    let f = Fixture::new();
    f.collateral();
    let second_asset = Address::generate(&f.env);
    f.vault
        .set_collateral(&f.user, &second_asset, &1, &100_000, &6_000, &8_000);
    assert_eq!(f.pool.calculate_limit(&f.user), 60_000);
}
