#![no_std]

mod auth;
mod guard;
mod registry;

use soroban_sdk::{contract, contractimpl, Address, Env};

// ── Guard Contract ────────────────────────────────────────────────────────────

#[contract]
pub struct GuardContract;

#[contractimpl]
impl GuardContract {
    /// Set the caller's daily spending limit.
    pub fn set_limit(env: Env, user: Address, new_limit: i128) {
        user.require_auth();
        guard::set_limit(&env, &user, new_limit);
    }

    /// Execute a guarded transfer, enforcing the daily velocity limit.
    /// Whitelisted drip addresses bypass the limit check.
    pub fn execute_transfer(env: Env, user: Address, to: Address, amount: i128) {
        user.require_auth();
        let is_drip = registry::is_trusted_drip(&env, &to);
        if !is_drip {
            guard::check_and_record(&env, &user, amount);
        }
        // Actual token transfer would be invoked here via a token client.
    }
}

// ── Registry Contract ─────────────────────────────────────────────────────────

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
    /// Whitelist a drip address (exempt from velocity limits).
    pub fn add_trusted_drip(env: Env, admin: Address, drip_address: Address) {
        admin.require_auth();
        registry::add_trusted_drip(&env, &drip_address);
    }

    pub fn is_trusted_drip(env: Env, drip_address: Address) -> bool {
        registry::is_trusted_drip(&env, &drip_address)
    }
}

// ── Auth Contract ─────────────────────────────────────────────────────────────

#[contract]
pub struct AuthContract;

#[contractimpl]
impl AuthContract {
    /// Register a passkey public key for a user (65-byte uncompressed P-256 key).
    pub fn register_key(env: Env, user: Address, pubkey: soroban_sdk::BytesN<65>) {
        user.require_auth();
        auth::register_key(&env, &user, pubkey);
    }

    /// Set the multi-sig threshold for a user.
    pub fn set_threshold(env: Env, user: Address, threshold: u32) {
        user.require_auth();
        auth::set_threshold(&env, &user, threshold);
    }

    /// Verify a SECP256R1 signature. Message is SHA-256 hashed internally.
    pub fn verify_sig(
        env: Env,
        user: Address,
        message: soroban_sdk::Bytes,
        signature: soroban_sdk::BytesN<64>,
    ) -> bool {
        auth::verify_sig(&env, &user, message, signature)
    }
}
