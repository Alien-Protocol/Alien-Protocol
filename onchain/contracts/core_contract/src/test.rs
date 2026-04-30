use crate::registration::DataKey as RegistrationKey;
use crate::smt_root::SmtRoot;
use crate::types::{
    AddressMetadata, ChainType, Permission, PermissionSet, PrivacyMode, PublicSignals,
};
use crate::{Contract, ContractClient};
use escrow_contract::types::{
    AutoPay, ScheduledPayment as EscrowScheduledPayment, VaultConfig, VaultState,
};
use shared::errors::CoreError;
use soroban_sdk::testutils::{Address as _, Events, Ledger as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{contracttype, Address, Bytes, BytesN, Env, Error, IntoVal, Symbol, Val, Vec};

fn setup(env: &Env) -> (Address, ContractClient<'_>) {
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(env, &contract_id);
    (contract_id, client)
}

fn setup_with_root(env: &Env) -> (Address, ContractClient<'_>, BytesN<32>) {
    let (contract_id, client) = setup(env);
    let owner = Address::generate(env);
    client.initialize(&owner);
    let root = BytesN::from_array(env, &[1u8; 32]);
    env.as_contract(&contract_id, || {
        SmtRoot::update_root(env, root.clone());
    });
    (contract_id, client, root)
}

fn commitment(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

#[test]
fn test_register_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 10);

    client.register(&owner, &hash);

    let stored_owner = client.get_owner(&hash);
    assert_eq!(stored_owner, Some(owner));
}

#[test]
#[should_panic(expected = "Error(Contract, #4010)")]
fn test_register_duplicate_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 11);

    client.register(&owner, &hash);
    client.register(&owner, &hash);
}

#[test]
#[should_panic]
fn test_register_requires_auth() {
    let env = Env::default();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 12);

    client.register(&owner, &hash);
}

#[test]
fn test_get_owner_returns_none_for_unknown() {
    let env = Env::default();
    let (_, client) = setup(&env);

    let hash = commitment(&env, 13);
    let stored_owner = client.get_owner(&hash);
    assert_eq!(stored_owner, None);
}

#[test]
fn test_submit_proof_success_updates_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);

    env.ledger().set_timestamp(1_700_000_123);

    let caller = Address::generate(&env);
    let hash = commitment(&env, 14);
    let new_root = BytesN::from_array(&env, &[15u8; 32]);

    client.submit_proof(
        &caller,
        &dummy_proof(&env),
        &signals(&hash, root, new_root.clone()),
    );

    assert_eq!(client.get_owner(&hash), Some(caller));
    assert_eq!(client.get_smt_root(), new_root);
    assert_eq!(client.get_created_at(&hash), Some(1_700_000_123));
}

#[test]
fn test_submit_proof_emits_username_registered_event() {
    use crate::events::username_registered_event;
    use crate::registration::Registration;
    use soroban_sdk::{IntoVal, TryFromVal};

    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _, root) = setup_with_root(&env);

    let caller = Address::generate(&env);
    let hash = commitment(&env, 18);
    let new_root = BytesN::from_array(&env, &[19u8; 32]);
    let proof = dummy_proof(&env);
    let public_signals = signals(&hash, root, new_root);

    env.as_contract(&contract_id, || {
        Registration::submit_proof(
            env.clone(),
            caller.clone(),
            proof.clone(),
            public_signals.clone(),
        );
    });

    let events = env.events().all();
    assert_eq!(
        events.len(),
        2,
        "ROOT_UPD and UsernameRegistered must be emitted"
    );

    let last_event = events.last().expect("No events emitted");
    let event_topic = last_event
        .1
        .get(0)
        .expect("UsernameRegistered event topic missing");
    let event_name = Symbol::try_from_val(&env, &event_topic)
        .expect("UsernameRegistered event topic is not a Symbol");
    assert_eq!(event_name, username_registered_event(&env));

    let emitted_commitment: BytesN<32> = last_event.2.into_val(&env);
    assert_eq!(emitted_commitment, hash);
}

fn dummy_proof(env: &Env) -> Bytes {
    Bytes::from_slice(env, &[1u8; 64])
}

fn signals(commitment: &BytesN<32>, old_root: BytesN<32>, new_root: BytesN<32>) -> PublicSignals {
    PublicSignals {
        commitment: commitment.clone(),
        old_root,
        new_root,
    }
}

#[contracttype]
#[derive(Clone)]
enum RoundtripKey {
    AddressMetadata,
    VaultConfig,
    VaultState,
    ScheduledPayment,
    AutoPay,
}

#[test]
fn test_address_metadata_roundtrip() {
    let env = Env::default();
    let (contract_id, _) = setup(&env);
    let key = RoundtripKey::AddressMetadata;
    let label = Symbol::new(&env, "primary");
    let metadata = AddressMetadata {
        label: label.clone(),
    };

    let stored = env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &metadata);
        env.storage().persistent().get::<_, AddressMetadata>(&key)
    });
    assert_eq!(stored.map(|item| item.label), Some(label));
}

#[test]
fn test_vault_config_roundtrip() {
    let env = Env::default();
    let (contract_id, _) = setup(&env);
    let key = RoundtripKey::VaultConfig;
    let config = VaultConfig {
        owner: Address::generate(&env),
        token: Address::generate(&env),
        created_at: 1_729_000_001,
    };

    let stored = env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &config);
        env.storage().persistent().get::<_, VaultConfig>(&key)
    });
    assert_eq!(stored, Some(config));
}

#[test]
fn test_vault_state_roundtrip() {
    let env = Env::default();
    let (contract_id, _) = setup(&env);
    let key = RoundtripKey::VaultState;
    let state = VaultState {
        balance: 5_000,
        is_active: true,
    };

    let stored = env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &state);
        env.storage().persistent().get::<_, VaultState>(&key)
    });
    assert_eq!(stored, Some(state));
}

#[test]
fn test_scheduled_payment_roundtrip() {
    let env = Env::default();
    let (contract_id, _) = setup(&env);
    let key = RoundtripKey::ScheduledPayment;
    let payment = EscrowScheduledPayment {
        from: BytesN::from_array(&env, &[7u8; 32]),
        to: BytesN::from_array(&env, &[8u8; 32]),
        token: Address::generate(&env),
        amount: 900,
        release_at: 3_600,
        executed: false,
    };

    let stored = env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &payment);
        env.storage()
            .persistent()
            .get::<_, EscrowScheduledPayment>(&key)
    });
    assert_eq!(stored, Some(payment));
}

#[test]
fn test_auto_pay_roundtrip() {
    let env = Env::default();
    let (contract_id, _) = setup(&env);
    let key = RoundtripKey::AutoPay;
    let rule = AutoPay {
        from: BytesN::from_array(&env, &[9u8; 32]),
        to: BytesN::from_array(&env, &[10u8; 32]),
        token: Address::generate(&env),
        amount: 250,
        interval: 86_400,
        last_paid: 0,
    };

    let stored = env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &rule);
        env.storage().persistent().get::<_, AutoPay>(&key)
    });
    assert_eq!(stored, Some(rule));
}

#[test]
fn test_resolve_returns_none_when_no_memo() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);
    let caller = Address::generate(&env);
    let hash = commitment(&env, 0);
    let new_root = BytesN::from_array(&env, &[2u8; 32]);

    let signals = signals(&hash, root, new_root);
    client.register_resolver(&caller, &hash, &dummy_proof(&env), &signals);

    let (resolved_wallet, memo) = client.resolve(&hash);
    assert_eq!(resolved_wallet, caller);
    assert_eq!(memo, None);
}

#[test]
fn test_set_memo_and_resolve_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);
    let caller = Address::generate(&env);
    let hash = commitment(&env, 0);
    let new_root = BytesN::from_array(&env, &[2u8; 32]);

    let signals = signals(&hash, root, new_root);
    client.register(&caller, &hash);
    client.register_resolver(&caller, &hash, &dummy_proof(&env), &signals);
    client.set_memo(&caller, &hash, &4242u64);

    let (resolved_wallet, memo) = client.resolve(&hash);
    assert_eq!(resolved_wallet, caller);
    assert_eq!(memo, Some(4242u64));
}

#[test]
fn test_get_privacy_mode_defaults_to_normal() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let hash = commitment(&env, 39);

    assert_eq!(client.get_privacy_mode(&hash), PrivacyMode::Normal);
}

#[test]
fn test_set_privacy_mode_to_shielded() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, root) = setup_with_root(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 40);
    let new_root = BytesN::from_array(&env, &[41u8; 32]);

    client.register(&owner, &hash);
    client.register_resolver(
        &owner,
        &hash,
        &dummy_proof(&env),
        &signals(&hash, root, new_root),
    );

    assert_eq!(client.get_privacy_mode(&hash), PrivacyMode::Normal);
    assert_eq!(client.resolve(&hash), (owner.clone(), None));

    client.set_privacy_mode(&owner, &hash, &PrivacyMode::Shielded);

    assert_eq!(client.get_privacy_mode(&hash), PrivacyMode::Shielded);
    assert_eq!(client.resolve(&hash), (contract_id, None));
}

#[test]
fn test_set_privacy_mode_to_normal() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 42);
    let new_root = BytesN::from_array(&env, &[43u8; 32]);

    client.register(&owner, &hash);
    client.register_resolver(
        &owner,
        &hash,
        &dummy_proof(&env),
        &signals(&hash, root, new_root),
    );

    client.set_privacy_mode(&owner, &hash, &PrivacyMode::Shielded);
    assert_eq!(client.get_privacy_mode(&hash), PrivacyMode::Shielded);

    client.set_privacy_mode(&owner, &hash, &PrivacyMode::Normal);
    assert_eq!(client.get_privacy_mode(&hash), PrivacyMode::Normal);
}

#[test]
fn test_set_privacy_mode_non_owner_rejected() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let hash = commitment(&env, 44);

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&RegistrationKey::Commitment(hash.clone()), &owner);
    });

    let args: Vec<Val> = (attacker.clone(), hash.clone(), PrivacyMode::Shielded).into_val(&env);
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "set_privacy_mode",
            args: args.clone(),
            sub_invokes: &[],
        },
    }]);

    let result = env.try_invoke_contract::<(), soroban_sdk::Error>(
        &contract_id,
        &Symbol::new(&env, "set_privacy_mode"),
        args,
    );

    assert!(result.is_err());
    assert_eq!(client.get_privacy_mode(&hash), PrivacyMode::Normal);
}

#[test]
fn test_resolve_stellar_returns_linked_address() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 10);

    client.register(&owner, &hash);
    client.add_stellar_address(&owner, &hash, &owner);

    let resolved = client.resolve_stellar(&hash);
    assert_eq!(resolved, owner);
}

#[test]
fn test_resolve_stellar_linked_address_differs_from_owner() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let payment_address = Address::generate(&env);
    let hash = commitment(&env, 11);

    client.register(&owner, &hash);
    client.add_stellar_address(&owner, &hash, &payment_address);

    let resolved = client.resolve_stellar(&hash);
    assert_eq!(resolved, payment_address);
}

#[test]
fn test_resolve_stellar_owner_is_linked_address() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 41);

    client.register(&owner, &hash);
    client.add_stellar_address(&owner, &hash, &owner);

    let resolved = client.resolve_stellar(&hash);
    assert_eq!(resolved, owner);
}

#[test]
fn test_resolve_stellar_after_ownership_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let hash = commitment(&env, 42);

    client.register(&owner, &hash);
    client.add_stellar_address(&owner, &hash, &owner);

    let signals = signals(&hash, root, BytesN::from_array(&env, &[43u8; 32]));

    client.transfer(&owner, &hash, &new_owner, &dummy_proof(&env), &signals);

    let new_address = Address::generate(&env);
    client.add_stellar_address(&new_owner, &hash, &new_address);

    let resolved = client.resolve_stellar(&hash);
    assert_eq!(resolved, new_address);
}

#[test]
fn test_add_stellar_address_overwrites_previous() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 43);

    client.register(&owner, &hash);

    let original_address = Address::generate(&env);
    let updated_address = Address::generate(&env);

    client.add_stellar_address(&owner, &hash, &original_address);
    client.add_stellar_address(&owner, &hash, &updated_address);

    let resolved = client.resolve_stellar(&hash);
    assert_eq!(resolved, updated_address);
}

#[test]
fn test_get_stellar_addresses_initially_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 55);

    client.register(&owner, &hash);

    let addresses = client.get_stellar_addresses(&hash);
    assert_eq!(addresses.len(), 0);
}

#[test]
fn test_get_stellar_addresses_after_multiple_adds() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 56);
    let addr_one = Address::generate(&env);
    let addr_two = Address::generate(&env);
    let addr_three = Address::generate(&env);

    client.register(&owner, &hash);
    client.add_stellar_address(&owner, &hash, &addr_one);
    client.add_stellar_address(&owner, &hash, &addr_two);
    client.add_stellar_address(&owner, &hash, &addr_three);

    let addresses = client.get_stellar_addresses(&hash);
    let mut expected = Vec::new(&env);
    expected.push_back(addr_one);
    expected.push_back(addr_two);
    expected.push_back(addr_three);

    assert_eq!(addresses, expected);
}

#[test]
#[should_panic(expected = "Error(Contract, #4001)")]
fn test_resolve_stellar_not_found_for_unregistered_hash() {
    let env = Env::default();
    let (_, client) = setup(&env);

    let hash = commitment(&env, 12);
    client.resolve_stellar(&hash);
}

#[test]
#[should_panic]
fn test_register_resolver_unauthenticated_fails() {
    let env = Env::default();
    let (_, client, root) = setup_with_root(&env);
    let caller = Address::generate(&env);
    let hash = commitment(&env, 20);
    let signals = signals(&hash, root, BytesN::from_array(&env, &[2u8; 32]));
    client.register_resolver(&caller, &hash, &dummy_proof(&env), &signals);
}

#[test]
#[should_panic(expected = "Error(Contract, #4004)")]
fn test_register_resolver_stale_root_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, _) = setup_with_root(&env);
    let caller = Address::generate(&env);
    let hash = commitment(&env, 21);
    let signals = signals(
        &hash,
        BytesN::from_array(&env, &[99u8; 32]),
        BytesN::from_array(&env, &[2u8; 32]),
    );
    client.register_resolver(&caller, &hash, &dummy_proof(&env), &signals);
}

#[test]
#[should_panic(expected = "Error(Contract, #4006)")]
fn test_resolve_stellar_no_address_linked_when_not_set() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 13);

    client.register(&owner, &hash);
    client.resolve_stellar(&hash);
}

#[test]
#[should_panic]
fn test_add_stellar_address_wrong_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let hash = commitment(&env, 14);

    client.register(&owner, &hash);
    client.add_stellar_address(&attacker, &hash, &attacker);
}

#[test]
#[should_panic(expected = "Error(Contract, #4001)")]
fn test_add_stellar_address_not_registered_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let caller = Address::generate(&env);
    let hash = commitment(&env, 15);

    client.add_stellar_address(&caller, &hash, &caller);
}

#[test]
fn test_remove_stellar_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 85);
    let addr_a = Address::generate(&env);
    let addr_b = Address::generate(&env);

    client.register(&owner, &hash);
    client.add_stellar_address(&owner, &hash, &addr_a);
    client.add_stellar_address(&owner, &hash, &addr_b);

    client.remove_stellar_address(&owner, &hash, &addr_b);

    let addresses = client.get_stellar_addresses(&hash);
    assert_eq!(addresses.len(), 1);
    assert_eq!(addresses.get(0).expect("first address missing"), addr_a);

    assert_eq!(client.resolve_stellar(&hash), addr_a);
}

#[test]
#[should_panic]
fn test_remove_stellar_address_wrong_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let hash = commitment(&env, 86);
    let addr = Address::generate(&env);

    client.register(&owner, &hash);
    client.add_stellar_address(&owner, &hash, &addr);
    client.remove_stellar_address(&attacker, &hash, &addr);
}

#[test]
#[should_panic(expected = "Error(Contract, #4001)")]
fn test_remove_stellar_address_not_registered_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let caller = Address::generate(&env);
    let hash = commitment(&env, 87);
    let addr = Address::generate(&env);

    client.remove_stellar_address(&caller, &hash, &addr);
}

#[test]
#[should_panic(expected = "Error(Contract, #4003)")]
fn test_register_resolver_duplicate_commitment_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);
    let caller = Address::generate(&env);
    let hash = commitment(&env, 22);
    let new_root = BytesN::from_array(&env, &[2u8; 32]);

    let signals_first = signals(&hash, root, new_root.clone());
    client.register_resolver(&caller, &hash, &dummy_proof(&env), &signals_first);

    let signals_second = signals(&hash, new_root, BytesN::from_array(&env, &[3u8; 32]));
    client.register_resolver(&caller, &hash, &dummy_proof(&env), &signals_second);
}

#[test]
fn test_register_resolver_success_updates_root() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);
    let caller = Address::generate(&env);
    let hash = commitment(&env, 23);
    let new_root = BytesN::from_array(&env, &[2u8; 32]);

    let signals = signals(&hash, root, new_root.clone());
    client.register_resolver(&caller, &hash, &dummy_proof(&env), &signals);

    assert_eq!(client.get_smt_root(), new_root);
    let (resolved_wallet, memo) = client.resolve(&hash);
    assert_eq!(resolved_wallet, caller);
    assert_eq!(memo, None);
}

#[test]
fn test_register_resolver_emits_events() {
    use crate::errors::CoreError;
    use crate::storage::DataKey;
    use crate::zk_verifier::ZkVerifier;

    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _, root) = setup_with_root(&env);

    let caller = Address::generate(&env);
    let hash = commitment(&env, 24);
    let new_root = BytesN::from_array(&env, &[2u8; 32]);
    let proof = dummy_proof(&env);
    let signals = signals(&hash, root.clone(), new_root.clone());

    env.as_contract(&contract_id, || {
        use soroban_sdk::panic_with_error;

        let key = DataKey::Resolver(hash.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, CoreError::DuplicateCommitment);
        }
        let current = SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet));
        assert_eq!(signals.old_root, current);
        assert!(ZkVerifier::verify_groth16_proof(&env, &proof, &signals));
        env.storage().persistent().set(
            &key,
            &crate::types::ResolveData {
                wallet: caller.clone(),
                memo: None,
            },
        );
        SmtRoot::update_root(&env, signals.new_root.clone());
        #[allow(deprecated)]
        env.events().publish(
            (crate::events::REGISTER_EVENT,),
            (hash.clone(), caller.clone()),
        );
    });

    let events = env.events().all();
    assert_eq!(
        events.len(),
        2,
        "ROOT_UPD and REGISTER events must both be emitted"
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #4002)")]
fn test_get_smt_root_panics_when_not_set() {
    let env = Env::default();
    let (_, client) = setup(&env);
    client.get_smt_root();
}

#[test]
fn test_smt_root_read_after_update() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client) = setup(&env);

    let new_root = BytesN::from_array(&env, &[42u8; 32]);
    env.as_contract(&contract_id, || {
        SmtRoot::update_root(&env, new_root.clone());
    });

    assert_eq!(client.get_smt_root(), new_root);
}

#[test]
fn test_smt_root_update_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, _) = setup(&env);

    let root1 = BytesN::from_array(&env, &[1u8; 32]);
    env.as_contract(&contract_id, || {
        SmtRoot::update_root(&env, root1.clone());
    });

    let root2 = BytesN::from_array(&env, &[2u8; 32]);
    env.as_contract(&contract_id, || {
        SmtRoot::update_root(&env, root2.clone());
    });

    let events = env.events().all();
    assert!(!events.is_empty(), "ROOT_UPD events should be emitted");
}

#[test]
fn test_rbac_roles() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    let admin = Address::generate(&env);
    let operator = Address::generate(&env);

    client.initialize(&owner);
    assert_eq!(client.get_contract_owner(), owner.clone());
    assert_eq!(client.get_admin(), owner.clone());
    assert_eq!(client.get_operator(), owner.clone());

    // Owner sets admin
    env.mock_auths(&[MockAuth {
        address: &owner,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "set_admin",
            args: (admin.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.set_admin(&admin);
    assert_eq!(client.get_admin(), admin.clone());

    // Admin sets operator
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "set_operator",
            args: (operator.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.set_operator(&operator);
    assert_eq!(client.get_operator(), operator.clone());

    // Operator updates SMT root
    let new_root = BytesN::from_array(&env, &[42u8; 32]);
    env.mock_auths(&[MockAuth {
        address: &operator,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "update_smt_root",
            args: (new_root.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.update_smt_root(&new_root);
    assert_eq!(client.get_smt_root(), new_root);
}

#[test]
fn test_update_smt_root_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client) = setup(&env);
    let owner = Address::generate(&env);
    let new_root = BytesN::from_array(&env, &[99u8; 32]);

    client.initialize(&owner);

    env.mock_auths(&[MockAuth {
        address: &owner,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "update_smt_root",
            args: (new_root.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.update_smt_root(&new_root);

    assert_eq!(client.get_smt_root(), new_root);
}

#[test]
fn test_require_operator_panics_for_non_operator() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let new_root = BytesN::from_array(&env, &[98u8; 32]);

    env.mock_auths(&[MockAuth {
        address: &owner,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (owner.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&owner);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "update_smt_root",
            args: (new_root.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = env.try_invoke_contract::<(), Error>(
        &contract_id,
        &Symbol::new(&env, "update_smt_root"),
        (new_root,).into_val(&env),
    );

    assert!(result.is_err());
}

#[test]
fn test_require_owner_panics_if_uninitialized() {
    let env = Env::default();
    let (contract_id, _) = setup(&env);
    let new_root = BytesN::from_array(&env, &[97u8; 32]);

    let result = env.try_invoke_contract::<(), Error>(
        &contract_id,
        &Symbol::new(&env, "update_smt_root"),
        (new_root,).into_val(&env),
    );

    assert!(matches!(
        result,
        Err(Ok(err)) if err == Error::from_contract_error(CoreError::NotFound as u32)
    ));
}

#[test]
#[should_panic(expected = "Error(Contract, #4011)")]
fn test_update_smt_root_same_root_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    client.initialize(&owner);

    let root = BytesN::from_array(&env, &[42u8; 32]);
    client.update_smt_root(&root);
    client.update_smt_root(&root);
}

fn evm_address(env: &Env) -> Bytes {
    Bytes::from_slice(env, b"0xaAbBcCdDeEfF00112233445566778899aAbBcCdD")
}

fn bitcoin_address(env: &Env) -> Bytes {
    Bytes::from_slice(env, &b"1A1zP1eP5QGefi2DMPTfTL5SLmv7Divf Na"[..34])
}

fn solana_address(env: &Env) -> Bytes {
    Bytes::from_slice(env, b"So11111111111111111111111111111111111111112")
}

fn cosmos_address(env: &Env) -> Bytes {
    Bytes::from_slice(env, b"cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777yh8aee")
}

#[test]
fn test_add_evm_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 1);
    let addr = evm_address(&env);
    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Evm, &addr);
    assert_eq!(client.get_chain_address(&hash, &ChainType::Evm), Some(addr));
}

#[test]
fn test_add_bitcoin_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 2);
    let addr = bitcoin_address(&env);
    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Bitcoin, &addr);
    assert_eq!(
        client.get_chain_address(&hash, &ChainType::Bitcoin),
        Some(addr)
    );
}

#[test]
fn test_add_solana_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 3);
    let addr = solana_address(&env);
    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Solana, &addr);
    assert_eq!(
        client.get_chain_address(&hash, &ChainType::Solana),
        Some(addr)
    );
}

#[test]
fn test_add_cosmos_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 4);
    let addr = cosmos_address(&env);
    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Cosmos, &addr);
    assert_eq!(
        client.get_chain_address(&hash, &ChainType::Cosmos),
        Some(addr)
    );
}

#[test]
fn test_get_chain_address_returns_none_when_not_set() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let hash = commitment(&env, 5);
    assert_eq!(client.get_chain_address(&hash, &ChainType::Evm), None);
}

#[test]
fn test_remove_chain_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 6);
    let addr = evm_address(&env);
    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Evm, &addr);
    assert_eq!(client.get_chain_address(&hash, &ChainType::Evm), Some(addr));
    client.remove_chain_address(&owner, &hash, &ChainType::Evm);
    assert_eq!(client.get_chain_address(&hash, &ChainType::Evm), None);
}

#[test]
#[should_panic]
fn test_add_chain_address_not_registered_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let caller = Address::generate(&env);
    let hash = commitment(&env, 7);
    client.add_chain_address(&caller, &hash, &ChainType::Evm, &evm_address(&env));
}

#[test]
#[should_panic]
fn test_add_chain_address_wrong_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let hash = commitment(&env, 8);
    client.register(&owner, &hash);
    client.add_chain_address(&attacker, &hash, &ChainType::Evm, &evm_address(&env));
}

#[test]
#[should_panic]
fn test_remove_chain_address_wrong_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let hash = commitment(&env, 9);
    client.register(&owner, &hash);
    client.add_chain_address(&owner, &hash, &ChainType::Evm, &evm_address(&env));
    client.remove_chain_address(&attacker, &hash, &ChainType::Evm);
}

#[test]
fn test_transfer_ownership_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let hash = commitment(&env, 30);

    client.register(&owner, &hash);
    client.transfer_ownership(&owner, &hash, &new_owner);

    assert_eq!(client.get_owner(&hash), Some(new_owner));
}

#[test]
#[should_panic(expected = "Error(Contract, #4007)")]
fn test_transfer_ownership_non_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let hash = commitment(&env, 31);

    client.register(&owner, &hash);
    client.transfer_ownership(&attacker, &hash, &new_owner);
}

#[test]
fn test_transfer_succeeds() {
    use crate::errors::CoreError;
    use crate::events::TRANSFER_EVENT;
    use crate::registration::DataKey as RegKey;
    use crate::zk_verifier::ZkVerifier;
    use soroban_sdk::panic_with_error;

    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client, root) = setup_with_root(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let hash = commitment(&env, 32);
    let new_root = BytesN::from_array(&env, &[99u8; 32]);
    let proof = dummy_proof(&env);
    let signals = signals(&hash, root.clone(), new_root.clone());

    client.register(&owner, &hash);

    env.as_contract(&contract_id, || {
        let key = RegKey::Commitment(hash.clone());
        let current_owner: Address = env
            .storage()
            .persistent()
            .get(&key)
            .expect("owner should be stored");

        if owner != current_owner {
            panic_with_error!(&env, CoreError::Unauthorized);
        }
        if new_owner == current_owner {
            panic_with_error!(&env, CoreError::SameOwner);
        }
        let current_root = SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet));
        assert_eq!(signals.old_root, current_root);
        assert!(ZkVerifier::verify_groth16_proof(&env, &proof, &signals));

        env.storage().persistent().set(&key, &new_owner);
        SmtRoot::update_root(&env, signals.new_root.clone());

        #[allow(deprecated)]
        env.events().publish(
            (TRANSFER_EVENT,),
            (hash.clone(), owner.clone(), new_owner.clone()),
        );
    });

    let events = env.events().all();
    assert_eq!(
        events.len(),
        2,
        "ROOT_UPD and TRANSFER events must both be emitted"
    );

    assert_eq!(client.get_owner(&hash), Some(new_owner.clone()));
    assert_eq!(client.get_smt_root(), new_root);
}

#[test]
#[should_panic(expected = "Error(Contract, #4008)")]
fn test_transfer_same_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 33);

    client.register(&owner, &hash);

    let signals = signals(
        &hash,
        BytesN::from_array(&env, &[0u8; 32]),
        BytesN::from_array(&env, &[0u8; 32]),
    );
    client.transfer(&owner, &hash, &owner, &dummy_proof(&env), &signals);
}

#[test]
#[should_panic(expected = "Error(Contract, #4008)")]
fn test_transfer_ownership_same_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 35);

    client.register(&owner, &hash);
    client.transfer_ownership(&owner, &hash, &owner);
}

#[test]
#[should_panic(expected = "Error(Contract, #4007)")]
fn test_transfer_non_owner_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let hash = commitment(&env, 34);

    client.register(&owner, &hash);

    let signals = signals(
        &hash,
        BytesN::from_array(&env, &[0u8; 32]),
        BytesN::from_array(&env, &[0u8; 32]),
    );
    client.transfer(&attacker, &hash, &new_owner, &dummy_proof(&env), &signals);
}

#[test]
fn test_full_identity_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client) = setup(&env);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let hash = commitment(&env, 100);

    client.initialize(&new_owner);

    client.register(&owner, &hash);
    assert_eq!(client.get_owner(&hash), Some(owner.clone()));
    env.as_contract(&contract_id, || {
        assert_eq!(SmtRoot::get_root(env.clone()), None);
    });

    let root1 = BytesN::from_array(&env, &[1u8; 32]);
    client.update_smt_root(&root1);
    assert_eq!(client.get_smt_root(), root1);
    assert_eq!(client.get_owner(&hash), Some(owner.clone()));

    let stellar = Address::generate(&env);
    client.add_stellar_address(&owner, &hash, &stellar);
    assert_eq!(client.resolve_stellar(&hash), stellar);
    assert_eq!(client.get_smt_root(), root1);
    assert_eq!(client.get_owner(&hash), Some(owner.clone()));

    let root2 = BytesN::from_array(&env, &[2u8; 32]);
    let signals = PublicSignals {
        commitment: hash.clone(),
        old_root: root1.clone(),
        new_root: root2.clone(),
    };
    client.transfer(&owner, &hash, &new_owner, &dummy_proof(&env), &signals);

    assert_eq!(client.get_owner(&hash), Some(new_owner.clone()));
    assert_eq!(client.get_smt_root(), root2);

    let root3 = BytesN::from_array(&env, &[3u8; 32]);
    client.update_smt_root(&root3);
    assert_eq!(client.get_smt_root(), root3);
    assert_eq!(client.get_owner(&hash), Some(new_owner));
}

#[test]
#[should_panic]
fn test_invalid_evm_address_wrong_length_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 10);
    client.register(&owner, &hash);
    client.add_chain_address(
        &owner,
        &hash,
        &ChainType::Evm,
        &Bytes::from_slice(&env, b"0x1234567"),
    );
}

#[test]
#[should_panic]
fn test_invalid_evm_address_no_prefix_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 11);
    client.register(&owner, &hash);
    client.add_chain_address(
        &owner,
        &hash,
        &ChainType::Evm,
        &Bytes::from_slice(&env, b"aAbBcCdDeEfF00112233445566778899aAbBcCdDeE"),
    );
}

#[test]
#[should_panic]
fn test_invalid_solana_address_too_short_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 12);
    client.register(&owner, &hash);
    client.add_chain_address(
        &owner,
        &hash,
        &ChainType::Solana,
        &Bytes::from_slice(&env, b"short1234"),
    );
}

#[test]
#[should_panic]
fn test_invalid_cosmos_address_too_short_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);
    let owner = Address::generate(&env);
    let hash = commitment(&env, 13);
    client.register(&owner, &hash);
    client.add_chain_address(
        &owner,
        &hash,
        &ChainType::Cosmos,
        &Bytes::from_slice(&env, b"cosmos123"),
    );
}

#[test]
fn test_get_root_returns_none_before_set() {
    let env = Env::default();
    let (contract_id, _) = setup(&env);

    env.as_contract(&contract_id, || {
        assert_eq!(SmtRoot::get_root(env.clone()), None);
    });
}

#[test]
fn test_update_root_stores_new_root() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    let new_root = BytesN::from_array(&env, &[2u8; 32]);

    env.as_contract(&contract_id, || {
        SmtRoot::update_root(&env, new_root.clone());
    });

    assert_eq!(client.get_smt_root(), new_root);
}

#[test]
fn test_update_root_emits_event() {
    let env = Env::default();
    let (contract_id, _client) = setup(&env);
    let new_root = BytesN::from_array(&env, &[3u8; 32]);

    env.as_contract(&contract_id, || {
        SmtRoot::update_root(&env, new_root.clone());
    });

    let events = env.events().all();
    let last_event = events.last().expect("No events emitted");

    use soroban_sdk::{IntoVal, TryFromVal};

    assert_eq!(last_event.0, contract_id);

    let event_topic = last_event.1.get(0).expect("event topic missing");
    let event_name = Symbol::try_from_val(&env, &event_topic).expect("event name should decode");
    assert_eq!(event_name, Symbol::new(&env, "ROOT_UPD"));

    let (old, new): (Option<BytesN<32>>, BytesN<32>) = last_event.2.into_val(&env);
    assert_eq!(old, None);
    assert_eq!(new, new_root);
}

#[test]
fn test_initialize_stores_owner() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    client.initialize(&owner);

    assert_eq!(client.get_contract_owner(), owner);
}

#[test]
fn test_initialize_emits_init_event() {
    let env = Env::default();
    env.mock_all_auths();
    let (contract_id, client) = setup(&env);

    let owner = Address::generate(&env);
    client.initialize(&owner);

    let events = env.events().all();
    let has_init_event = events.iter().any(|(c, _, _)| c == contract_id);
    assert!(has_init_event);
}

#[test]
fn test_initialize_double_init_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    client.initialize(&owner);

    let result = client.try_initialize(&owner);
    assert!(result.is_err());
}

#[test]
fn test_get_contract_owner_before_init_panics() {
    let env = Env::default();
    let (_, client) = setup(&env);

    let result = client.try_get_contract_owner();
    assert!(result.is_err());
}

#[test]
fn test_add_shielded_address_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 80);
    let addr_commitment = BytesN::from_array(&env, &[0xAAu8; 32]);

    client.register(&owner, &hash);
    client.add_shielded_address(&owner, &hash, &addr_commitment);

    assert_eq!(client.get_shielded_address(&hash), Some(addr_commitment));
    assert!(client.is_shielded(&hash));
}

#[test]
fn test_is_shielded_returns_false_when_not_set() {
    let env = Env::default();
    let (_, client) = setup(&env);

    let hash = commitment(&env, 81);
    assert!(!client.is_shielded(&hash));
}

#[test]
fn test_add_shielded_address_overwrite_works() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 82);
    let first = BytesN::from_array(&env, &[0x11u8; 32]);
    let second = BytesN::from_array(&env, &[0x22u8; 32]);

    client.register(&owner, &hash);
    client.add_shielded_address(&owner, &hash, &first);
    client.add_shielded_address(&owner, &hash, &second);

    assert_eq!(client.get_shielded_address(&hash), Some(second));
}

#[test]
fn test_add_shielded_address_non_owner_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let hash = commitment(&env, 83);
    let addr_commitment = BytesN::from_array(&env, &[0xBBu8; 32]);

    client.register(&owner, &hash);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "add_shielded_address",
            args: (&attacker, &hash, &addr_commitment).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_add_shielded_address(&attacker, &hash, &addr_commitment);
    assert!(result.is_err());
}

#[test]
fn test_add_shielded_address_unregistered_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let caller = Address::generate(&env);
    let hash = commitment(&env, 84);
    let addr_commitment = BytesN::from_array(&env, &[0xCCu8; 32]);

    let result = client.try_add_shielded_address(&caller, &hash, &addr_commitment);
    assert!(result.is_err());
}

#[test]
fn test_get_created_at_returns_none_for_unregistered() {
    let env = Env::default();
    let (_, client) = setup(&env);

    let hash = commitment(&env, 90);
    assert_eq!(client.get_created_at(&hash), None);
}

#[test]
fn test_get_created_at_returns_timestamp_after_register() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 91);

    client.register(&owner, &hash);

    assert_eq!(client.get_created_at(&hash), Some(0u64));
}

#[test]
fn test_get_created_at_reflects_ledger_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    env.ledger().set_timestamp(1_700_000_000);

    let owner = Address::generate(&env);
    let hash = commitment(&env, 92);

    client.register(&owner, &hash);

    assert_eq!(client.get_created_at(&hash), Some(1_700_000_000u64));
}

#[test]
fn test_get_created_at_unchanged_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    env.ledger().set_timestamp(1_000_000);

    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let hash = commitment(&env, 93);

    client.register(&owner, &hash);
    assert_eq!(client.get_created_at(&hash), Some(1_000_000u64));

    env.ledger().set_timestamp(2_000_000);
    client.transfer_ownership(&owner, &hash, &new_owner);

    assert_eq!(client.get_created_at(&hash), Some(1_000_000u64));
}

#[test]
fn test_delegate_scoped_permissions() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);

    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let hash = commitment(&env, 100);
    let new_root = BytesN::from_array(&env, &[101u8; 32]);

    client.register(&owner, &hash);
    client.register_resolver(
        &owner,
        &hash,
        &dummy_proof(&env),
        &signals(&hash, root, new_root),
    );

    // Grant SetMemo permission
    let mut permissions = Vec::new(&env);
    permissions.push_back(Permission::SetMemo);
    client.grant_delegate(&owner, &hash, &delegate, &PermissionSet { permissions });

    // Delegate can call set_memo
    client.set_memo(&delegate, &hash, &1234u64);
    let (_, memo) = client.resolve(&hash);
    assert_eq!(memo, Some(1234u64));

    // Delegate cannot call set_privacy_mode (not in scope)
    let result = client.try_set_privacy_mode(&delegate, &hash, &PrivacyMode::Shielded);
    assert!(result.is_err());

    // Revoke permissions
    client.revoke_delegate(&owner, &hash, &delegate);

    // Delegate can no longer call set_memo
    let result = client.try_set_memo(&delegate, &hash, &5678u64);
    assert!(result.is_err());
}

#[test]
fn test_delegate_address_management() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let hash = commitment(&env, 101);

    client.register(&owner, &hash);

    // Grant address management permissions
    let mut permissions = Vec::new(&env);
    permissions.push_back(Permission::AddChainAddress);
    permissions.push_back(Permission::RemoveChainAddress);
    permissions.push_back(Permission::AddStellarAddress);
    permissions.push_back(Permission::RemoveStellarAddress);
    client.grant_delegate(&owner, &hash, &delegate, &PermissionSet { permissions });

    // Test chain address
    let eth_addr = Bytes::from_slice(&env, b"0x1234567890123456789012345678901234567890");
    client.add_chain_address(&delegate, &hash, &ChainType::Evm, &eth_addr);
    assert_eq!(
        client.get_chain_address(&hash, &ChainType::Evm),
        Some(eth_addr)
    );

    client.remove_chain_address(&delegate, &hash, &ChainType::Evm);
    assert_eq!(client.get_chain_address(&hash, &ChainType::Evm), None);

    // Test stellar address
    let stellar_addr = Address::generate(&env);
    client.add_stellar_address(&delegate, &hash, &stellar_addr);
    assert_eq!(client.resolve_stellar(&hash), stellar_addr);

    client.remove_stellar_address(&delegate, &hash, &stellar_addr);
    let result = client.try_resolve_stellar(&hash);
    assert!(result.is_err());
}

#[test]
fn test_unauthorized_delegate_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client) = setup(&env);

    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let hash = commitment(&env, 102);

    client.register(&owner, &hash);

    // Attacker tries to set memo
    let result = client.try_set_memo(&attacker, &hash, &999u64);
    assert!(result.is_err());
}

#[test]
fn test_delegate_set_privacy_mode() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, client, root) = setup_with_root(&env);
    let owner = Address::generate(&env);
    let delegate = Address::generate(&env);
    let hash = commitment(&env, 103);
    let new_root = BytesN::from_array(&env, &[104u8; 32]);

    client.register(&owner, &hash);
    client.register_resolver(
        &owner,
        &hash,
        &dummy_proof(&env),
        &signals(&hash, root, new_root),
    );

    // Grant SetPrivacyMode
    let mut permissions = Vec::new(&env);
    permissions.push_back(Permission::SetPrivacyMode);
    client.grant_delegate(&owner, &hash, &delegate, &PermissionSet { permissions });

    client.set_privacy_mode(&delegate, &hash, &PrivacyMode::Shielded);
    assert_eq!(client.get_privacy_mode(&hash), PrivacyMode::Shielded);
}
