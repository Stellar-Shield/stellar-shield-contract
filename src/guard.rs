use soroban_sdk::{panic_with_error, Address, Env, Symbol};

use crate::error::Error;

/// Ledger sequences per day (approx 5s per ledger, so 17280 ledgers/day).
const LEDGERS_PER_DAY: u32 = 17_280;

/// How long a day's running total is kept alive. Two days, so a total written
/// just before midnight is still readable for the whole of the next day.
const TTL: u32 = LEDGERS_PER_DAY * 2;

/// How long the contract instance itself is kept alive, and when to top it up.
/// A Soroban contract whose instance is archived stops being callable at all;
/// none of these contracts bumped their own instance, so a quiet deployment
/// would eventually have to be restored before anyone could use it again.
const INSTANCE_TTL: u32 = LEDGERS_PER_DAY * 30;
const INSTANCE_BUMP_AT: u32 = LEDGERS_PER_DAY * 7;

pub fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_BUMP_AT, INSTANCE_TTL);
}

/// Keep a persistent entry alive.
///
/// This matters more here than it looks. A user's limit lives in persistent
/// storage and nothing was extending it, so a limit set once and left alone
/// would eventually be archived -- and a missing limit reads as "no limit".
/// The guard would quietly stop guarding, which is the worst direction for a
/// security control to fail in.
pub fn bump_persistent<K: soroban_sdk::IntoVal<Env, soroban_sdk::Val>>(env: &Env, key: &K) {
    env.storage()
        .persistent()
        .extend_ttl(key, INSTANCE_BUMP_AT, INSTANCE_TTL);
}

pub fn set_limit(env: &Env, user: &Address, new_limit: i128) {
    bump_instance(env);
    // A negative limit would make every spend fail; a zero limit is a freeze
    // that revoke would express more honestly. Neither is what anyone means.
    if new_limit <= 0 {
        panic_with_error!(env, Error::BadAmount);
    }
    let key = (Symbol::new(env, "limit"), user.clone());
    env.storage().persistent().set(&key, &new_limit);
    bump_persistent(env, &key);
}

pub fn limit_of(env: &Env, user: &Address) -> Option<i128> {
    let key = (Symbol::new(env, "limit"), user.clone());
    let found: Option<i128> = env.storage().persistent().get(&key);
    if found.is_some() {
        // Reading is also using. A limit that is being enforced every day
        // should not expire out from under the user who set it.
        bump_persistent(env, &key);
    }
    found
}

/// What a user has already spent today, ignoring a total left over from an
/// earlier day.
pub fn spent_today(env: &Env, user: &Address) -> i128 {
    let current_day = env.ledger().sequence() / LEDGERS_PER_DAY;
    let stored_day: u32 = env
        .storage()
        .temporary()
        .get(&(Symbol::new(env, "day"), user.clone()))
        .unwrap_or(0);

    if stored_day == current_day {
        env.storage()
            .temporary()
            .get(&(Symbol::new(env, "spent"), user.clone()))
            .unwrap_or(0)
    } else {
        0
    }
}

pub fn check_and_record(env: &Env, user: &Address, amount: i128) {
    bump_instance(env);
    // A negative amount used to be accepted, and it SUBTRACTED from the running
    // total: spend -100 twice and the day's budget grows. The limit is the
    // product, so this is the check that has to come first.
    if amount <= 0 {
        panic_with_error!(env, Error::BadAmount);
    }

    // No limit set means no limit enforced. That is deliberate -- the guard is
    // opt-in -- but it is worth saying out loud rather than leaving in an
    // unwrap_or.
    let limit: i128 = match limit_of(env, user) {
        Some(l) => l,
        None => return,
    };

    let spent = spent_today(env, user);
    let new_spent = spent
        .checked_add(amount)
        .unwrap_or_else(|| panic_with_error!(env, Error::Overflow));

    if new_spent > limit {
        panic_with_error!(env, Error::LimitExceeded);
    }

    let current_day = env.ledger().sequence() / LEDGERS_PER_DAY;
    let day_key = (Symbol::new(env, "day"), user.clone());
    let spent_key = (Symbol::new(env, "spent"), user.clone());

    env.storage().temporary().set(&day_key, &current_day);
    env.storage().temporary().extend_ttl(&day_key, TTL, TTL);
    env.storage().temporary().set(&spent_key, &new_spent);
    env.storage().temporary().extend_ttl(&spent_key, TTL, TTL);
}
