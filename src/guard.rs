use soroban_sdk::{Address, Env, Symbol};

/// Ledger sequences per day (approx 5s per ledger → 17280 ledgers/day).
const LEDGERS_PER_DAY: u32 = 17_280;

pub fn set_limit(env: &Env, user: &Address, new_limit: i128) {
    env.storage()
        .persistent()
        .set(&(Symbol::new(env, "limit"), user.clone()), &new_limit);
}

pub fn check_and_record(env: &Env, user: &Address, amount: i128) {
    let limit: i128 = env
        .storage()
        .persistent()
        .get(&(Symbol::new(env, "limit"), user.clone()))
        .unwrap_or(i128::MAX);

    let current_day = env.ledger().sequence() / LEDGERS_PER_DAY;

    let stored_day: u32 = env
        .storage()
        .temporary()
        .get(&(Symbol::new(env, "day"), user.clone()))
        .unwrap_or(0);

    let spent: i128 = if stored_day == current_day {
        env.storage()
            .temporary()
            .get(&(Symbol::new(env, "spent"), user.clone()))
            .unwrap_or(0)
    } else {
        0
    };

    let new_spent = spent + amount;
    assert!(new_spent <= limit, "velocity limit exceeded");

    // TTL: keep temporary entries alive for 2 days
    let ttl = LEDGERS_PER_DAY * 2;
    env.storage()
        .temporary()
        .set(&(Symbol::new(env, "day"), user.clone()), &current_day);
    env.storage()
        .temporary()
        .extend_ttl(&(Symbol::new(env, "day"), user.clone()), ttl, ttl);

    env.storage()
        .temporary()
        .set(&(Symbol::new(env, "spent"), user.clone()), &new_spent);
    env.storage()
        .temporary()
        .extend_ttl(&(Symbol::new(env, "spent"), user.clone()), ttl, ttl);
}
