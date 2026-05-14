use soroban_sdk::{Address, Env, Symbol};

pub fn add_trusted_drip(env: &Env, drip: &Address) {
    env.storage()
        .persistent()
        .set(&(Symbol::new(env, "drip"), drip.clone()), &true);
}

pub fn is_trusted_drip(env: &Env, drip: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&(Symbol::new(env, "drip"), drip.clone()))
        .unwrap_or(false)
}
