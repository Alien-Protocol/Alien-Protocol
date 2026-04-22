#![no_std]
use soroban_sdk::Env;

pub mod utils {
    use soroban_sdk::Env;
    
    pub fn helper_function(_env: &Env) -> bool {
        // Shared utility functions
        true
    }
}

#[cfg(test)]
mod test {
    use super::utils;
    use soroban_sdk::Env;

    #[test]
    fn test_helper_function() {
        let env = Env::default();
        assert!(utils::helper_function(&env));
    }
}
