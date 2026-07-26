

#[cfg(test)]
mod test {
    use soroban_sdk::{testutils::Address as _, token, Address, Env};
    use crate::{CollateralVaultClient, CustodyReport};

    #[test]
    fn test_custody_reconciliation_detects_deficit() {
        let env = Env::default();
        env.mock_all_signatures();

        let vault_id = env.register_contract(None, crate::CollateralVault);
        let vault_client = CollateralVaultClient::new(&env, &vault_id);

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        // Register a standard mock Stellar token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();
        let token_client = token::Client::new(&env, &token_address);
        let token_admin_client = token::StellarAssetClient::new(&env, &token_address);

        // Mint tokens to user
        token_admin_client.mint(&user, &1000);

        // Perform deposit
        vault_client.deposit(&user, &token_address, &500);

        // Initially, custody should be healthy
        let report: CustodyReport = vault_client.get_custody_health(&token_address);
        assert_eq!(report.recorded_liability, 500);
        assert_eq!(report.actual_balance, 500);
        assert!(!report.has_deficit);

        // Simulate an accounting deficit by draining vault tokens manually
        // (e.g. mimicking a compromised transfer or external token loss)
        // Check that reconciliation catches it:
        env.as_contract(&vault_id, || {
            let client = token::Client::new(&env, &token_address);
            client.transfer(&admin, &200);
        });

        let report_after: CustodyReport = vault_client.get_custody_health(&token_address);
        assert_eq!(report_after.recorded_liability, 500);
        assert_eq!(report_after.actual_balance, 300);
        assert!(report_after.has_deficit);
    }
}
