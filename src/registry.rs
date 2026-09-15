use soroban_sdk::{panic_with_error, Address, Env, Symbol};

use crate::error::Error;

fn admin_key(env: &Env) -> Symbol {
    Symbol::new(env, "admin")
}

/// Set the admin once, at deployment.
///
/// Without this the registry had no admin at all, and `add_trusted_drip` simply
/// believed whichever address the caller passed as `admin`.
pub fn init(env: &Env, admin: &Address) {
    crate::guard::bump_instance(env);
    if env.storage().instance().has(&admin_key(env)) {
        panic_with_error!(env, Error::AlreadyInitialised);
    }
    env.storage().instance().set(&admin_key(env), admin);
}

pub fn admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&admin_key(env))
        .unwrap_or_else(|| panic_with_error!(env, Error::NotInitialised))
}

/// Prove the caller is the stored admin.
///
/// The authorisation is required from the address this contract recorded, not
/// from one the caller handed us. That difference is the whole fix.
pub fn require_admin(env: &Env) {
    admin(env).require_auth();
}

pub fn set_trusted_drip(env: &Env, drip: &Address, trusted: bool) {
    crate::guard::bump_instance(env);
    let key = (Symbol::new(env, "drip"), drip.clone());
    env.storage().persistent().set(&key, &trusted);
    crate::guard::bump_persistent(env, &key);
}

pub fn is_trusted_drip(env: &Env, drip: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&(Symbol::new(env, "drip"), drip.clone()))
        .unwrap_or(false)
}
