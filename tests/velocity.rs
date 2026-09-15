#![cfg(test)]
//! The daily spending cap, which is the product.

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};
use stellar_shield::{
    GuardContract, GuardContractClient, RegistryContract, RegistryContractClient,
};

const LEDGERS_PER_DAY: u32 = 17_280;

struct World<'a> {
    env: Env,
    guard: GuardContractClient<'a>,
    registry: RegistryContractClient<'a>,
    registry_id: Address,
    user: Address,
    to: Address,
}

fn world() -> World<'static> {
    let env = Env::default();
    env.mock_all_auths();
    // Raise the ceiling before anything is written. The test host archives
    // entries aggressively by default, and these tests are about the day
    // boundary, not about archival.
    env.ledger().with_mut(|li| {
        li.max_entry_ttl = LEDGERS_PER_DAY * 90;
    });
    let guard_id = env.register(GuardContract, ());
    let registry_id = env.register(RegistryContract, ());
    let guard = GuardContractClient::new(&env, &guard_id);
    let registry = RegistryContractClient::new(&env, &registry_id);
    let admin = Address::generate(&env);
    registry.init(&admin);
    let user = Address::generate(&env);
    let to = Address::generate(&env);
    World {
        env,
        guard,
        registry,
        registry_id,
        user,
        to,
    }
}

#[test]
fn a_spend_inside_the_limit_is_recorded() {
    let w = world();
    w.guard.set_limit(&w.user, &500);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &300);
    assert_eq!(w.guard.spent_today(&w.user), 300);
    assert_eq!(w.guard.limit_of(&w.user), Some(500));
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // LimitExceeded
fn the_limit_is_a_running_daily_total_not_a_per_spend_cap() {
    let w = world();
    w.guard.set_limit(&w.user, &100);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &80);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &30);
}

/// The hole, stated as a test.
///
/// check_and_record did `spent + amount` with no sign check, so a negative
/// amount SUBTRACTED from the day's running total. Spend -1000 and the budget
/// grows by 1000. The cap is the entire point of the contract, so an input
/// that quietly raises it is the worst kind of bug here.
#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BadAmount
fn a_negative_spend_cannot_buy_back_budget() {
    let w = world();
    w.guard.set_limit(&w.user, &100);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &90);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &-1000);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BadAmount
fn a_zero_spend_is_refused_rather_than_silently_recorded() {
    let w = world();
    w.guard.set_limit(&w.user, &100);
    w.guard.execute_transfer(&w.user, &w.registry_id, &w.to, &0);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BadAmount
fn a_limit_must_be_positive() {
    let w = world();
    w.guard.set_limit(&w.user, &-1);
}

#[test]
fn a_user_who_set_no_limit_is_unguarded_and_that_is_deliberate() {
    let w = world();
    assert_eq!(w.guard.limit_of(&w.user), None);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &1_000_000);
    // Nothing recorded, because nothing was being enforced.
    assert_eq!(w.guard.spent_today(&w.user), 0);
}

#[test]
fn the_budget_refills_the_next_day() {
    let w = world();
    w.guard.set_limit(&w.user, &100);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &100);
    assert_eq!(w.guard.spent_today(&w.user), 100);

    // Move to tomorrow. max_entry_ttl is raised too: the test host archives
    // entries aggressively by default, and we are testing the day boundary,
    // not archival.
    w.env
        .ledger()
        .with_mut(|li| li.sequence_number = LEDGERS_PER_DAY + 1);
    assert_eq!(
        w.guard.spent_today(&w.user),
        0,
        "yesterday's total is not today's"
    );
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &100);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")] // Overflow
fn a_running_total_that_would_overflow_is_refused_not_wrapped() {
    let w = world();
    w.guard.set_limit(&w.user, &i128::MAX);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &i128::MAX);
    w.guard.execute_transfer(&w.user, &w.registry_id, &w.to, &1);
}

/// This is the one the old suite claimed but did not test. Its version asserted
/// only that the registry flag was set, with a comment conceding that the guard
/// never consulted it.
#[test]
fn a_whitelisted_destination_really_does_bypass_the_cap() {
    let w = world();
    w.guard.set_limit(&w.user, &10);
    w.registry.add_trusted_drip(&w.to);

    // Far over the limit, straight through, because the guard now actually
    // calls the registry.
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &10_000);
    assert_eq!(
        w.guard.spent_today(&w.user),
        0,
        "an exempt spend is not counted"
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // LimitExceeded
fn withdrawing_an_exemption_puts_the_cap_back() {
    let w = world();
    w.guard.set_limit(&w.user, &10);
    w.registry.add_trusted_drip(&w.to);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &10_000);

    w.registry.remove_trusted_drip(&w.to);
    w.guard
        .execute_transfer(&w.user, &w.registry_id, &w.to, &10_000);
}
