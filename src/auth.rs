use soroban_sdk::{panic_with_error, Address, Bytes, BytesN, Env, Symbol};

use crate::error::Error;

pub fn register_key(env: &Env, user: &Address, pubkey: BytesN<65>) {
    crate::guard::bump_instance(env);
    let key = (Symbol::new(env, "pubkey"), user.clone());
    env.storage().persistent().set(&key, &pubkey);
    crate::guard::bump_persistent(env, &key);
}

pub fn key_of(env: &Env, user: &Address) -> Option<BytesN<65>> {
    env.storage()
        .persistent()
        .get(&(Symbol::new(env, "pubkey"), user.clone()))
}

pub fn set_threshold(env: &Env, user: &Address, threshold: u32) {
    if threshold < 1 {
        panic_with_error!(env, Error::BadThreshold);
    }
    env.storage()
        .persistent()
        .set(&(Symbol::new(env, "thresh"), user.clone()), &threshold);
}

pub fn threshold_of(env: &Env, user: &Address) -> u32 {
    env.storage()
        .persistent()
        .get(&(Symbol::new(env, "thresh"), user.clone()))
        .unwrap_or(1)
}

/// Verify a SECP256R1 (P-256) signature over `message`.
///
/// Read the failure modes before branching on the result. The host's
/// `secp256r1_verify` returns nothing and TRAPS when a signature does not
/// verify -- there is no false to return. So:
///
///   - no key registered for this user  -> traps with NoKey
///   - key registered, signature bad    -> traps inside the host
///   - key registered, signature good   -> returns true
///
/// The `bool` therefore only ever carries `true`. It is kept because callers
/// read better with it, but a caller must understand that a forged signature
/// reverts their transaction rather than returning false to be handled. The
/// previous version returned `true` unconditionally after ignoring the host
/// call's outcome, which read as a check and was not one.
pub fn verify_sig(env: &Env, user: &Address, message: Bytes, signature: BytesN<64>) -> bool {
    let pubkey = match key_of(env, user) {
        Some(k) => k,
        None => panic_with_error!(env, Error::NoKey),
    };

    let hash = env.crypto().sha256(&message);
    env.crypto().secp256r1_verify(&pubkey, &hash, &signature);
    true
}
