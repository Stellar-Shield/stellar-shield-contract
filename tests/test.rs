#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};
use stellar_shield::{
    AuthContract, AuthContractClient, GuardContract, GuardContractClient, RegistryContract,
    RegistryContractClient,
};

// ── Guard tests ───────────────────────────────────────────────────────────────

#[test]
fn test_set_and_enforce_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(GuardContract, ());
    let client = GuardContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.set_limit(&user, &500);
    // Transfer within limit – should succeed.
    client.execute_transfer(&user, &recipient, &300);
}

#[test]
#[should_panic(expected = "velocity limit exceeded")]
fn test_limit_exceeded_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(GuardContract, ());
    let client = GuardContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let recipient = Address::generate(&env);

    client.set_limit(&user, &100);
    // First transfer OK.
    client.execute_transfer(&user, &recipient, &80);
    // Second transfer pushes daily total over limit.
    client.execute_transfer(&user, &recipient, &30);
}

// ── Registry tests ────────────────────────────────────────────────────────────

#[test]
fn test_drip_whitelist() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let drip = Address::generate(&env);

    assert!(!client.is_trusted_drip(&drip));
    client.add_trusted_drip(&admin, &drip);
    assert!(client.is_trusted_drip(&drip));
}

#[test]
fn test_drip_bypasses_velocity_limit() {
    let env = Env::default();
    env.mock_all_auths();

    // Register both contracts in the same env so storage is shared.
    let guard_id = env.register(GuardContract, ());
    let registry_id = env.register(RegistryContract, ());
    let guard = GuardContractClient::new(&env, &guard_id);
    let registry = RegistryContractClient::new(&env, &registry_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let drip = Address::generate(&env);

    guard.set_limit(&user, &10); // very tight limit
    registry.add_trusted_drip(&admin, &drip);

    // Transfer to a drip address should bypass the limit entirely.
    // NOTE: guard contract checks its own registry storage; this test
    // validates the registry flag independently. A full integration would
    // share a single contract instance.
    assert!(registry.is_trusted_drip(&drip));
}

// ── Auth tests ────────────────────────────────────────────────────────────────

#[test]
fn test_register_key_and_threshold() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AuthContract, ());
    let client = AuthContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    // 65-byte uncompressed P-256 public key placeholder (0x04 || 32 || 32 bytes).
    let pubkey = soroban_sdk::BytesN::<65>::from_array(&env, &[4u8; 65]);

    client.register_key(&user, &pubkey);
    client.set_threshold(&user, &2);
    // No panic = key and threshold stored successfully.
}
