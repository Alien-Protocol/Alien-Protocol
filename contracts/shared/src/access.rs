//! Generic, storage-agnostic access-control helpers for contract administration.

use soroban_sdk::Address;

/// Validates that an administrator address is configured and requires its authorization.
///
/// Returns the authenticated administrator address on success, or `Err(not_initialized)`
/// if no administrator has been configured.
pub fn require_admin<E>(admin: Option<Address>, not_initialized: E) -> Result<Address, E> {
    let admin = admin.ok_or(not_initialized)?;
    admin.require_auth();
    Ok(admin)
}

/// Ensures that a proposed new administrator address is not identical to the current one.
///
/// Returns `Err(already_admin)` if `current == new`.
pub fn ensure_new_admin<E>(current: &Address, new: &Address, already_admin: E) -> Result<(), E> {
    if current == new {
        return Err(already_admin);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{contract, contracterror, contractimpl, testutils::Address as _, Env};

    #[contracterror]
    #[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
    #[repr(u32)]
    pub enum MockError {
        NotInitialized = 1,
        AlreadyAdmin = 2,
    }

    #[contract]
    struct DummyContract;

    #[contractimpl]
    impl DummyContract {
        pub fn call_require_admin(admin: Option<Address>) -> Result<Address, MockError> {
            require_admin(admin, MockError::NotInitialized)
        }
    }

    #[test]
    fn test_require_admin_success() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(DummyContract, ());
        let client = DummyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        let result = client.call_require_admin(&Some(admin.clone()));
        assert_eq!(result, admin);
    }

    #[test]
    fn test_require_admin_missing_returns_error() {
        let env = Env::default();
        let contract_id = env.register(DummyContract, ());
        let client = DummyContractClient::new(&env, &contract_id);

        let result = client.try_call_require_admin(&None);
        assert_eq!(result, Err(Ok(MockError::NotInitialized)));
    }

    #[test]
    fn test_ensure_new_admin_different_address_succeeds() {
        let env = Env::default();
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        let result = ensure_new_admin(&admin1, &admin2, MockError::AlreadyAdmin);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_ensure_new_admin_same_address_fails() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let result = ensure_new_admin(&admin, &admin, MockError::AlreadyAdmin);
        assert_eq!(result, Err(MockError::AlreadyAdmin));
    }
}
