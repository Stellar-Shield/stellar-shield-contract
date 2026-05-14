use soroban_sdk::{Address, Bytes, BytesN, Env, Symbol};

pub fn register_key(env: &Env, user: &Address, pubkey: BytesN<65>) {
    env.storage()
        .persistent()
        .set(&(Symbol::new(env, "pubkey"), user.clone()), &pubkey);
}

pub fn set_threshold(env: &Env, user: &Address, threshold: u32) {
    env.storage()
        .persistent()
        .set(&(Symbol::new(env, "thresh"), user.clone()), &threshold);
}

/// Verify a SECP256R1 (P-256) signature.
/// `message` is the raw message bytes; it is SHA-256 hashed internally.
/// `signature` is the 64-byte compact (r || s) form.
/// `pubkey` stored must be the 65-byte uncompressed SEC1 form (0x04 || x || y).
pub fn verify_sig(env: &Env, user: &Address, message: Bytes, signature: BytesN<64>) -> bool {
    let pubkey: Option<BytesN<65>> = env
        .storage()
        .persistent()
        .get(&(Symbol::new(env, "pubkey"), user.clone()));

    let pubkey = match pubkey {
        Some(k) => k,
        None => return false,
    };

    let hash = env.crypto().sha256(&message);
    env.crypto().secp256r1_verify(&pubkey, &hash, &signature);
    true
}
