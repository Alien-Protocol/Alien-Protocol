use soroban_sdk::testutils::{
    storage::Persistent, Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke,
};
use soroban_sdk::{contract, contractimpl, IntoVal, Symbol, TryFromVal, Val, Vec};
use soroban_sdk::{Address, BytesN, Env};

use crate::errors::FactoryError;
use crate::events::USERNAME_DEPLOYED;
use crate::{FactoryContract, FactoryContractClient};

#[contract]
struct StubContract;

#[contractimpl]
impl StubContract {}

fn setup_factory(env: &Env) -> (Address, FactoryContractClient<'_>, Address, Address) {
    env.mock_all_auths();
    let factory_id = env.register(FactoryContract, ());
    let factory = FactoryContractClient::new(env, &factory_id);
    let owner = Address::generate(env);
    let auction_contract = env.register(StubContract, ());
    let core_contract = env.register(StubContract, ());

    factory.initialize(&owner);
    factory.configure(&auction_contract, &core_contract);

    (factory_id, factory, auction_contract, core_contract)
}

fn setup_unconfigured_factory(env: &Env) -> (Address, FactoryContractClient<'_>) {
    env.mock_all_auths();
    let factory_id = env.register(FactoryContract, ());
    let factory = FactoryContractClient::new(env, &factory_id);
    (factory_id, factory)
}

fn username_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[7; 32])
}

#[test]
fn deploy_username_stores_record_and_emits_event() {
    let env = Env::default();
    let (factory_id, factory, auction_contract, core_contract) = setup_factory(&env);
    let owner = Address::generate(&env);
    let hash = username_hash(&env);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);
    factory.deploy_username(&hash, &owner);

    let events = env.events().all();

    let record = factory
        .get_username_record(&hash)
        .expect("username record should be stored after deploy");
    assert_eq!(record.username_hash, hash);
    assert_eq!(record.owner, owner);
    assert_eq!(record.registered_at, env.ledger().timestamp());
    assert_eq!(record.core_contract, core_contract);
    assert_eq!(events.len(), 1);

    let (event_contract, topics, data) = events.get(0).expect("expected exactly one event");
    assert_eq!(event_contract, factory_id);
    assert_eq!(topics.len(), 1);

    let event_name = Symbol::try_from_val(&env, &topics.get(0).expect("expected event name topic"))
        .expect("event name should deserialize");
    let (event_hash, event_owner, event_registered_at) =
        <(BytesN<32>, Address, u64)>::try_from_val(&env, &data)
            .expect("event payload should deserialize");

    assert_eq!(event_name, USERNAME_DEPLOYED);
    assert_eq!(event_hash, hash);
    assert_eq!(event_owner, owner);
    assert_eq!(event_registered_at, record.registered_at);
}

#[test]
fn duplicate_deployment_is_rejected() {
    let env = Env::default();
    let (factory_id, factory, auction_contract, _) = setup_factory(&env);
    let owner = Address::generate(&env);
    let hash = username_hash(&env);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args.clone(),
            sub_invokes: &[],
        },
    }]);
    factory.deploy_username(&hash, &owner);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);
    let result = factory.try_deploy_username(&hash, &owner);
    assert_eq!(result, Err(Ok(FactoryError::AlreadyDeployed)));
}

#[test]
fn non_registered_auction_auth_is_rejected() {
    let env = Env::default();
    let (factory_id, _, auction_contract, _) = setup_factory(&env);
    let wrong_caller = env.register(StubContract, ());
    let owner = Address::generate(&env);
    let hash = username_hash(&env);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &wrong_caller,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);
    let result = env.try_invoke_contract::<(), FactoryError>(
        &factory_id,
        &Symbol::new(&env, "deploy_username"),
        Vec::<Val>::from_array(&env, [hash.into_val(&env), owner.into_val(&env)]),
    );

    assert!(result.is_err());
    assert_ne!(wrong_caller, auction_contract);
}

#[test]
fn get_username_owner_returns_owner_after_deploy() {
    let env = Env::default();
    let (factory_id, factory, auction_contract, _) = setup_factory(&env);
    let owner = Address::generate(&env);
    let hash = username_hash(&env);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);
    factory.deploy_username(&hash, &owner);

    assert_eq!(factory.get_username_owner(&hash), Some(owner));
}

#[test]
fn get_username_owner_returns_none_for_unregistered_hash() {
    let env = Env::default();
    let (_, factory, _, _) = setup_factory(&env);
    let unknown_hash = BytesN::from_array(&env, &[0xFF; 32]);

    assert_eq!(factory.get_username_owner(&unknown_hash), None);
}

#[test]
fn test_deploy_username_success() {
    let env = Env::default();
    let (factory_id, factory, auction_contract, _core_contract) = setup_factory(&env);
    let owner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[10; 32]);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);

    factory.deploy_username(&hash, &owner);
    let record = factory
        .get_username_record(&hash)
        .expect("username record should be stored after deploy");
    assert_eq!(record.owner, owner);
}

#[test]
fn test_deploy_username_duplicate_fails() {
    let env = Env::default();
    let (factory_id, factory, auction_contract, _) = setup_factory(&env);
    let owner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[11; 32]);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args.clone(),
            sub_invokes: &[],
        },
    }]);
    factory.deploy_username(&hash, &owner);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);
    let result = factory.try_deploy_username(&hash, &owner);
    assert_eq!(result, Err(Ok(FactoryError::AlreadyDeployed)));
}

#[test]
fn test_deploy_unauthorized_fails() {
    let env = Env::default();
    let (factory_id, _, _, _) = setup_factory(&env);
    let wrong_caller = Address::generate(&env);
    let owner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[12; 32]);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &wrong_caller,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);

    let result = env.try_invoke_contract::<(), FactoryError>(
        &factory_id,
        &Symbol::new(&env, "deploy_username"),
        Vec::<Val>::from_array(
            &env,
            [hash.clone().into_val(&env), owner.clone().into_val(&env)],
        ),
    );

    assert!(result.is_err());
}

#[test]
fn test_get_owner_none_for_unknown() {
    let env = Env::default();
    let (_, factory, _, _) = setup_factory(&env);
    let unknown_hash = BytesN::from_array(&env, &[99; 32]);
    let record = factory.get_username_record(&unknown_hash);
    assert!(record.is_none());
}

#[test]
fn get_username_record_extends_ttl_on_read() {
    use crate::storage::{PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD};
    use crate::types::DataKey;

    let env = Env::default();
    let (factory_id, factory, auction_contract, _) = setup_factory(&env);
    let owner = Address::generate(&env);
    let hash = username_hash(&env);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);
    factory.deploy_username(&hash, &owner);

    env.ledger().with_mut(|l| {
        l.sequence_number += PERSISTENT_BUMP_AMOUNT - PERSISTENT_LIFETIME_THRESHOLD + 1;
    });

    let record = factory.get_username_record(&hash);
    assert!(record.is_some());

    env.as_contract(&factory_id, || {
        let ttl = env
            .storage()
            .persistent()
            .get_ttl(&DataKey::Username(hash.clone()));
        assert_eq!(ttl, PERSISTENT_BUMP_AMOUNT);
    });
}

#[test]
fn contract_getters_follow_soroban_convention() {
    let env = Env::default();
    let (_, factory) = setup_unconfigured_factory(&env);
    assert_eq!(factory.auction_contract(), None);
    assert_eq!(factory.core_contract(), None);

    let (_, factory, auction_contract, core_contract) = setup_factory(&env);
    assert_eq!(factory.auction_contract(), Some(auction_contract));
    assert_eq!(factory.core_contract(), Some(core_contract));
}

#[test]
fn test_transfer_username_success_v2() {
    let env = Env::default();
    let (factory_id, factory, auction_contract, _core_contract) = setup_factory(&env);

    // 1. Deploy
    let owner = Address::generate(&env);
    let hash = username_hash(&env);
    let deploy_args: Vec<Val> = (hash.clone(), owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "deploy_username",
            args: deploy_args,
            sub_invokes: &[],
        },
    }]);
    factory.deploy_username(&hash, &owner);

    // 2. Transfer
    let new_owner = Address::generate(&env);
    let transfer_args: Vec<Val> = (hash.clone(), new_owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "transfer_username",
            args: transfer_args,
            sub_invokes: &[],
        },
    }]);
    factory.transfer_username(&hash, &new_owner);

    // 3. Assertions
    let record = factory
        .get_username_record(&hash)
        .expect("record must exist");
    assert_eq!(record.owner, new_owner);
}

#[test]
fn test_transfer_username_unconfigured_auction_contract_fails() {
    let env = Env::default();
    let (factory_id, _) = setup_unconfigured_factory(&env);

    let hash = username_hash(&env);
    let new_owner = Address::generate(&env);

    let result = env.try_invoke_contract::<(), FactoryError>(
        &factory_id,
        &Symbol::new(&env, "transfer_username"),
        Vec::<Val>::from_array(&env, [hash.into_val(&env), new_owner.into_val(&env)]),
    );

    assert_eq!(result, Err(Ok(FactoryError::Unauthorized)));
}

#[test]
fn test_transfer_username_missing_record_fails() {
    let env = Env::default();
    let (factory_id, _factory, auction_contract, _) = setup_factory(&env);

    let hash = username_hash(&env);
    let new_owner = Address::generate(&env);
    let transfer_args: Vec<Val> = (hash.clone(), new_owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &auction_contract,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "transfer_username",
            args: transfer_args,
            sub_invokes: &[],
        },
    }]);

    let result = env.try_invoke_contract::<(), FactoryError>(
        &factory_id,
        &Symbol::new(&env, "transfer_username"),
        Vec::<Val>::from_array(&env, [hash.into_val(&env), new_owner.into_val(&env)]),
    );

    assert!(result.is_err());
}

#[test]
fn test_transfer_username_unauthorized_caller_fails() {
    let env = Env::default();
    let (factory_id, _, _, _) = setup_factory(&env);

    let wrong_caller = Address::generate(&env);
    let hash = username_hash(&env);
    let new_owner = Address::generate(&env);
    let transfer_args: Vec<Val> = (hash.clone(), new_owner.clone()).into_val(&env);

    env.mock_auths(&[MockAuth {
        address: &wrong_caller,
        invoke: &MockAuthInvoke {
            contract: &factory_id,
            fn_name: "transfer_username",
            args: transfer_args,
            sub_invokes: &[],
        },
    }]);

    let result = env.try_invoke_contract::<(), FactoryError>(
        &factory_id,
        &Symbol::new(&env, "transfer_username"),
        Vec::<Val>::from_array(&env, [hash.into_val(&env), new_owner.into_val(&env)]),
    );

    assert!(result.is_err());
}
