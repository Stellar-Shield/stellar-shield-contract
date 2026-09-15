#![no_std]

//! StellarShield: an on-chain security layer for Stellar wallets.
//!
//! Three contracts that are meant to be deployed together:
//!
//!   - `GuardContract`    a per-user daily spending cap
//!   - `RegistryContract` destinations the cap does not apply to
//!   - `AuthContract`     passkey (P-256) registration and verification
//!
//! The cap is the product. Everything else exists to let a person spend
//! normally without turning it off.

mod auth;
mod error;
mod guard;
mod registry;

pub use error::Error;

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env};

// -- Guard -------------------------------------------------------------------

#[contract]
pub struct GuardContract;

#[contractimpl]
impl GuardContract {
    /// Set the caller's daily spending limit. Must be positive.
    pub fn set_limit(env: Env, user: Address, new_limit: i128) {
        user.require_auth();
        guard::set_limit(&env, &user, new_limit);
    }

    /// The caller's limit, if they have set one. `None` means unguarded.
    pub fn limit_of(env: Env, user: Address) -> Option<i128> {
        guard::limit_of(&env, &user)
    }

    /// What this user has spent against today's budget.
    pub fn spent_today(env: Env, user: Address) -> i128 {
        guard::spent_today(&env, &user)
    }

    /// Record a guarded spend, enforcing the daily limit.
    ///
    /// `registry` is the registry contract to consult for exempt destinations.
    /// It is passed rather than assumed so a deployment cannot be pointed at a
    /// registry nobody audited; the previous version read its own storage for
    /// a whitelist that only the registry contract ever wrote, so the exemption
    /// never actually applied.
    pub fn execute_transfer(env: Env, user: Address, registry: Address, to: Address, amount: i128) {
        user.require_auth();

        let exempt = RegistryContractClient::new(&env, &registry).is_trusted_drip(&to);
        if !exempt {
            guard::check_and_record(&env, &user, amount);
        }

        // NOTE: this contract records and authorises; it does not custody or
        // move tokens. Wiring a token client is tracked as an open issue, and
        // is deliberately not claimed to work here.
    }
}

// -- Registry ----------------------------------------------------------------

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
    /// Set the admin. Callable once, at deployment.
    pub fn init(env: Env, admin: Address) {
        registry::init(&env, &admin);
    }

    /// The admin of this registry.
    pub fn admin(env: Env) -> Address {
        registry::admin(&env)
    }

    /// Exempt a destination from the velocity limit. Admin only.
    ///
    /// This used to take an `admin` argument and require_auth it, which proved
    /// only that whoever was named had signed -- so anyone could name
    /// themselves. The admin is now read from storage.
    pub fn add_trusted_drip(env: Env, drip_address: Address) {
        registry::require_admin(&env);
        registry::set_trusted_drip(&env, &drip_address, true);
    }

    /// Withdraw an exemption. Admin only.
    ///
    /// A whitelist you cannot remove from is not a whitelist.
    pub fn remove_trusted_drip(env: Env, drip_address: Address) {
        registry::require_admin(&env);
        registry::set_trusted_drip(&env, &drip_address, false);
    }

    pub fn is_trusted_drip(env: Env, drip_address: Address) -> bool {
        registry::is_trusted_drip(&env, &drip_address)
    }
}

// -- Auth --------------------------------------------------------------------

#[contract]
pub struct AuthContract;

#[contractimpl]
impl AuthContract {
    /// Register a passkey public key (65-byte uncompressed P-256).
    pub fn register_key(env: Env, user: Address, pubkey: BytesN<65>) {
        user.require_auth();
        auth::register_key(&env, &user, pubkey);
    }

    /// Whether a passkey is registered for this user.
    pub fn has_key(env: Env, user: Address) -> bool {
        auth::key_of(&env, &user).is_some()
    }

    /// Set the multi-sig threshold. Must be at least 1.
    ///
    /// Stored and readable, but NOT yet enforced by any entrypoint. It is
    /// exposed so the gap is visible rather than implied to work.
    pub fn set_threshold(env: Env, user: Address, threshold: u32) {
        user.require_auth();
        auth::set_threshold(&env, &user, threshold);
    }

    pub fn threshold_of(env: Env, user: Address) -> u32 {
        auth::threshold_of(&env, &user)
    }

    /// Verify a P-256 signature. Traps on a bad signature; see `auth::verify_sig`.
    pub fn verify_sig(env: Env, user: Address, message: Bytes, signature: BytesN<64>) -> bool {
        auth::verify_sig(&env, &user, message, signature)
    }
}
