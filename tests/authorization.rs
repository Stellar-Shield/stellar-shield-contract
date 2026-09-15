#![cfg(test)]
//! Who is allowed to do what.
//!
//! These are the tests the original suite could not have: every one of its
//! cases called env.mock_all_auths(), which turns require_auth into a no-op.
//! With authorisation mocked away, a contract with no authorisation at all
//! passes exactly the same tests as a correct one.

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};
use stellar_shield::{RegistryContract, RegistryContractClient};

fn registry(env: &Env) -> (Address, RegistryContractClient<'static>) {
    let id = env.register(RegistryContract, ());
    let client = RegistryContractClient::new(env, &id);
    (id, client)
}

/// The vulnerability, as it was.
///
/// add_trusted_drip used to take an `admin` argument and require_auth it. That
/// proves only that whoever was NAMED signed the call, and the caller chooses
/// the name. Nothing compared it against a stored admin, because there was no
/// stored admin. So any user could exempt any destination from the velocity
/// limit -- which is the entire product.
#[test]
#[should_panic(expected = "InvalidAction")] // the host refuses before we run
fn a_stranger_cannot_whitelist_an_address() {
    let env = Env::default();
    let (id, client) = registry(&env);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let attacker_payout = Address::generate(&env);

    env.mock_all_auths();
    client.init(&admin);

    // The attacker signs for themselves, which is all they can do.
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &id,
            fn_name: "add_trusted_drip",
            args: (attacker_payout.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.add_trusted_drip(&attacker_payout);
}

#[test]
fn the_admin_can_whitelist_and_withdraw_again() {
    let env = Env::default();
    let (_, client) = registry(&env);
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let drip = Address::generate(&env);

    client.init(&admin);
    assert_eq!(client.admin(), admin);

    assert!(!client.is_trusted_drip(&drip));
    client.add_trusted_drip(&drip);
    assert!(client.is_trusted_drip(&drip));

    // A whitelist you cannot remove from is not a whitelist.
    client.remove_trusted_drip(&drip);
    assert!(!client.is_trusted_drip(&drip));
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")] // NotInitialised
fn a_registry_with_no_admin_refuses_rather_than_allows() {
    let env = Env::default();
    let (_, client) = registry(&env);
    env.mock_all_auths();

    // Fail closed. An uninitialised registry that accepted writes would be the
    // same hole with an extra step.
    client.add_trusted_drip(&Address::generate(&env));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")] // AlreadyInitialised
fn the_admin_cannot_be_quietly_replaced() {
    let env = Env::default();
    let (_, client) = registry(&env);
    env.mock_all_auths();

    client.init(&Address::generate(&env));
    client.init(&Address::generate(&env));
}
