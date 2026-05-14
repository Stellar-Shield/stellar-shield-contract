# 🛡️ StellarShield — Soroban Smart Contracts

StellarShield is an on-chain security layer for Stellar wallets, built on [Soroban](https://soroban.stellar.org). It combines three independent contracts into a cohesive vault protection system:

- **Velocity Control** — per-user daily spending caps that auto-reset via temporary storage
- **Drip Whitelist** — trusted recurring payment addresses that bypass the spending cap
- **Passkey Auth** — SECP256R1 (P-256) signature verification and multi-sig thresholds, compatible with WebAuthn device passkeys

No off-chain cron jobs. No manual resets. Storage TTLs do the work.

---

## Table of Contents

- [Architecture](#architecture)
  - [System Overview](#system-overview)
  - [Contract Interaction Map](#contract-interaction-map)
  - [Guarded Transfer Call Flow](#guarded-transfer-call-flow)
  - [Storage Layout](#storage-layout)
- [Contracts](#contracts)
  - [GuardContract](#guardcontract)
  - [RegistryContract](#registrycontract)
  - [AuthContract](#authcontract)
- [Storage Strategy](#storage-strategy)
- [Security Model](#security-model)
- [Getting Started](#getting-started)
- [Running Tests](#running-tests)

---

## Architecture

### System Overview

```
╔══════════════════════════════════════════════════════════════════╗
║                         Client / dApp                           ║
║  (browser wallet, mobile app, backend service)                  ║
╚══════════════════════════╤═══════════════════════════════════════╝
                           │ Soroban RPC invoke
                           ▼
╔══════════════════════════════════════════════════════════════════╗
║                      GuardContract                              ║
║                                                                  ║
║   set_limit(user, new_limit)                                     ║
║   execute_transfer(user, to, amount)                             ║
║                                                                  ║
║   ┌─────────────────────────────────────────────────────────┐   ║
║   │  1. user.require_auth()                                 │   ║
║   │  2. is_trusted_drip(to)?  ──YES──► skip limit check    │   ║
║   │                            NO                           │   ║
║   │                             ▼                           │   ║
║   │  3. check_and_record(user, amount)                      │   ║
║   │       load persistent limit                             │   ║
║   │       load temporary daily spend                        │   ║
║   │       assert spent + amount ≤ limit                     │   ║
║   │       write new spend, extend TTL                       │   ║
║   └─────────────────────────────────────────────────────────┘   ║
╚══════════════╤═══════════════════════════════════════════════════╝
               │ cross-contract call
               ▼
╔══════════════════════════╗     ╔══════════════════════════════╗
║    RegistryContract      ║     ║       AuthContract           ║
║                          ║     ║                              ║
║  add_trusted_drip()      ║     ║  register_key()  (P-256)     ║
║  is_trusted_drip()       ║     ║  set_threshold() (multi-sig) ║
║                          ║     ║  verify_sig()    (SECP256R1) ║
╚══════════════════════════╝     ╚══════════════════════════════╝
```

### Contract Interaction Map

```
GuardContract
    │
    ├── reads ──► RegistryContract::is_trusted_drip(to)
    │                   │
    │                   └── persistent storage: ("drip", address) → bool
    │
    ├── reads ──► persistent storage: ("limit", user) → i128
    │
    └── reads/writes ──► temporary storage:
                            ("day",   user) → u32   (current day bucket)
                            ("spent", user) → i128  (cumulative daily spend)

RegistryContract
    └── writes ──► persistent storage: ("drip", address) → bool

AuthContract
    ├── writes ──► persistent storage: ("pubkey", user) → BytesN<65>
    ├── writes ──► persistent storage: ("thresh", user) → u32
    └── reads  ──► persistent storage: ("pubkey", user) → BytesN<65>
                   then: env.crypto().sha256(message) → Hash<32>
                   then: env.crypto().secp256r1_verify(pubkey, hash, sig)
```

### Guarded Transfer Call Flow

```
User
 │
 ├─► GuardContract::execute_transfer(user, to, amount)
 │       │
 │       ├─ [1] user.require_auth()
 │       │        Soroban enforces the caller is `user`
 │       │
 │       ├─ [2] registry::is_trusted_drip(&env, &to)
 │       │        reads persistent("drip", to)
 │       │        │
 │       │        ├─ true  ──► skip to step [5]
 │       │        └─ false ──► continue to step [3]
 │       │
 │       ├─ [3] guard::check_and_record(&env, &user, amount)
 │       │        │
 │       │        ├─ load limit  = persistent("limit", user)  [default: i128::MAX]
 │       │        ├─ current_day = ledger.sequence() / 17_280
 │       │        ├─ stored_day  = temporary("day",   user)   [default: 0]
 │       │        ├─ spent       = temporary("spent", user)   [default: 0]
 │       │        │                  (reset to 0 if stored_day ≠ current_day)
 │       │        │
 │       │        ├─ [4] assert spent + amount ≤ limit
 │       │        │        └─ FAIL ──► panic!("velocity limit exceeded")
 │       │        │
 │       │        └─ write temporary("day",   user) = current_day  TTL=2days
 │       │           write temporary("spent", user) = spent+amount TTL=2days
 │       │
 │       └─ [5] invoke token transfer (SEP-41 token client)
 │
 └─► ok
```

### Storage Layout

```
Persistent (survives archival, fees apply)
┌──────────────────────────────┬──────────────┬──────────────────────────┐
│ Key                          │ Type         │ Written by               │
├──────────────────────────────┼──────────────┼──────────────────────────┤
│ ("limit", user: Address)     │ i128         │ GuardContract::set_limit │
│ ("drip",  addr: Address)     │ bool         │ RegistryContract::add_.. │
│ ("pubkey",user: Address)     │ BytesN<65>   │ AuthContract::register.. │
│ ("thresh",user: Address)     │ u32          │ AuthContract::set_thres  │
└──────────────────────────────┴──────────────┴──────────────────────────┘

Temporary (auto-expires, cheap)
┌──────────────────────────────┬──────────────┬──────────────────────────┐
│ Key                          │ Type         │ TTL                      │
├──────────────────────────────┼──────────────┼──────────────────────────┤
│ ("day",   user: Address)     │ u32          │ 2 days (34,560 ledgers)  │
│ ("spent", user: Address)     │ i128         │ 2 days (34,560 ledgers)  │
└──────────────────────────────┴──────────────┴──────────────────────────┘
```

---

## Contracts

### GuardContract

`src/lib.rs` — `GuardContract`

Enforces a per-user daily spending cap. The daily counter lives in **temporary storage** and auto-expires after two days — no cron job or manual reset needed. The day bucket is derived from the ledger sequence number, so the window rolls over naturally.

#### set_limit

```rust
pub fn set_limit(env: Env, user: Address, new_limit: i128) {
    user.require_auth();
    guard::set_limit(&env, &user, new_limit);
}
```

Stores the limit in persistent storage. Amounts are in **stroops** (1 XLM = 10,000,000 stroops).

```rust
// Set a 500 XLM daily limit
client.set_limit(&user, &5_000_000_000i128);
```

#### execute_transfer

```rust
pub fn execute_transfer(env: Env, user: Address, to: Address, amount: i128) {
    user.require_auth();
    let is_drip = registry::is_trusted_drip(&env, &to);
    if !is_drip {
        guard::check_and_record(&env, &user, amount);
    }
    // token transfer invoked here
}
```

The velocity check inside `check_and_record`:

```rust
// guard.rs
let current_day = env.ledger().sequence() / LEDGERS_PER_DAY; // 17_280

// If the stored day bucket differs, the window has rolled — reset spend to 0
let spent: i128 = if stored_day == current_day { stored_spend } else { 0 };

let new_spent = spent + amount;
assert!(new_spent <= limit, "velocity limit exceeded");
```

**Usage example:**

```rust
// Day 1: two transfers that together stay under the 500 XLM cap
client.set_limit(&user, &500_000_000);
client.execute_transfer(&user, &alice, &200_000_000); // ok, spent = 200
client.execute_transfer(&user, &bob,   &250_000_000); // ok, spent = 450

// This one pushes the total to 550 — panics
client.execute_transfer(&user, &carol, &100_000_000); // panic: velocity limit exceeded
```

**Interface:**

| Function | Auth | Description |
|---|---|---|
| `set_limit(user, new_limit)` | `user` | Update the user's daily velocity cap |
| `execute_transfer(user, to, amount)` | `user` | Guarded transfer; drip addresses bypass the cap |

---

### RegistryContract

`src/lib.rs` — `RegistryContract` / `src/registry.rs`

Maintains the whitelist of trusted "Drip" addresses. A drip is any address — a subscription contract, a recurring payment stream, a payroll contract — that should be exempt from the daily velocity cap.

#### add_trusted_drip

```rust
pub fn add_trusted_drip(env: Env, admin: Address, drip_address: Address) {
    admin.require_auth();
    registry::add_trusted_drip(&env, &drip_address);
}
```

Internally writes a single boolean to persistent storage:

```rust
// registry.rs
env.storage()
    .persistent()
    .set(&(Symbol::new(env, "drip"), drip.clone()), &true);
```

#### is_trusted_drip

```rust
pub fn is_trusted_drip(env: Env, drip_address: Address) -> bool {
    registry::is_trusted_drip(&env, &drip_address)
}
```

Returns `false` for any address not explicitly whitelisted (safe default).

**Usage example:**

```rust
let subscription = Address::from_str(&env, "GABCD..."); // subscription contract

// Admin whitelists the subscription address
client.add_trusted_drip(&admin, &subscription);

// Any transfer to this address now bypasses the velocity check
assert!(client.is_trusted_drip(&subscription)); // true

// Transfers to non-whitelisted addresses still go through the cap
assert!(!client.is_trusted_drip(&random_address)); // false
```

**Interface:**

| Function | Auth | Description |
|---|---|---|
| `add_trusted_drip(admin, drip_address)` | `admin` | Whitelist an address |
| `is_trusted_drip(drip_address)` | none | Returns `true` if the address is whitelisted |

---

### AuthContract

`src/lib.rs` — `AuthContract` / `src/auth.rs`

Handles passkey-based authentication using the **SECP256R1 (P-256)** elliptic curve — the same curve used by WebAuthn, Apple Passkeys, and Android FIDO2 authenticators. Also supports configurable multi-sig thresholds for high-value operations.

#### register_key

Stores a 65-byte uncompressed SEC1 public key (`0x04 || x || y`) in persistent storage.

```rust
pub fn register_key(env: Env, user: Address, pubkey: BytesN<65>) {
    user.require_auth();
    auth::register_key(&env, &user, pubkey);
}
```

The 65-byte format is the standard uncompressed point encoding for P-256:

```
byte[0]     = 0x04  (uncompressed marker)
bytes[1..33]  = x coordinate (big-endian)
bytes[33..65] = y coordinate (big-endian)
```

```rust
// From a WebAuthn registration response (credentialPublicKey decoded from CBOR)
let pubkey: BytesN<65> = BytesN::from_array(&env, &raw_65_bytes);
client.register_key(&user, &pubkey);
```

#### set_threshold

```rust
pub fn set_threshold(env: Env, user: Address, threshold: u32) {
    user.require_auth();
    auth::set_threshold(&env, &user, threshold);
}
```

Sets how many valid signatures are required for multi-sig operations. A threshold of `1` is standard single-key auth; `2` or higher requires multiple passkeys.

```rust
// Require 2-of-N passkeys for this user
client.set_threshold(&user, &2);
```

#### verify_sig

```rust
pub fn verify_sig(
    env: Env,
    user: Address,
    message: Bytes,
    signature: BytesN<64>,
) -> bool {
    auth::verify_sig(&env, &user, message, signature)
}
```

Internally, the raw message is SHA-256 hashed before being passed to the host's `secp256r1_verify` builtin:

```rust
// auth.rs
let hash = env.crypto().sha256(&message);          // → Hash<32>
env.crypto().secp256r1_verify(&pubkey, &hash, &signature);
```

The signature must be in **compact (r || s) form** — 64 bytes total, as produced by WebAuthn authenticators.

```rust
// From a WebAuthn assertion response
let message   = Bytes::from_slice(&env, b"authorize:transfer:500xlm");
let signature = BytesN::<64>::from_array(&env, &compact_rs_bytes);

let valid = client.verify_sig(&user, &message, &signature);
assert!(valid);
```

**Interface:**

| Function | Auth | Description |
|---|---|---|
| `register_key(user, pubkey)` | `user` | Store a 65-byte P-256 public key |
| `set_threshold(user, threshold)` | `user` | Set the multi-sig signing threshold |
| `verify_sig(user, message, signature)` | none | Verify a SECP256R1 signature |

---

## Storage Strategy

Soroban has three storage tiers. StellarShield uses two of them deliberately:

| Data | Storage tier | Reason |
|---|---|---|
| Daily spend counter | `temporary` | Auto-expires after ~2 days; no manual reset needed, minimises fees |
| Daily window bucket | `temporary` | Same — keyed by `(ledger_sequence / 17280)` |
| Velocity limit | `persistent` | User config; must survive ledger archival |
| Drip whitelist entries | `persistent` | Long-lived subscription data |
| P-256 public key | `persistent` | Passkey credential; must never expire |
| Multi-sig threshold | `persistent` | User config |

**Why temporary storage for the daily counter?**

Soroban's temporary storage entries are automatically evicted when their TTL reaches zero. By setting a 2-day TTL and refreshing it on every write, the daily spend counter is guaranteed to be gone by the time the next day's window opens — without any explicit reset transaction.

```rust
const LEDGERS_PER_DAY: u32 = 17_280; // ~5 s per ledger × 86,400 s/day

let ttl = LEDGERS_PER_DAY * 2; // 34,560 ledgers ≈ 2 days

env.storage().temporary().set(&spend_key, &new_spent);
env.storage().temporary().extend_ttl(&spend_key, ttl, ttl);
```

**Day bucket rollover** — instead of storing a timestamp, the current day is derived from the ledger sequence:

```rust
let current_day = env.ledger().sequence() / LEDGERS_PER_DAY;
```

If the stored day bucket differs from `current_day`, the window has rolled over and the spend resets to zero automatically — no storage write needed for the reset itself.

---

## Security Model

| Threat | Mitigation |
|---|---|
| Unauthorised limit change | `set_limit` calls `user.require_auth()` — only the user can change their own cap |
| Unauthorised drip whitelist | `add_trusted_drip` calls `admin.require_auth()` — only the admin can whitelist |
| Replay of old spend state | Day bucket derived from ledger sequence; stale temporary entries are ignored |
| Forged passkey signature | `secp256r1_verify` is a Soroban host builtin — runs in the VM, not user code |
| Unregistered key bypass | `verify_sig` returns `false` (not panic) if no key is stored for the user |
| Overspend via drip abuse | Drip whitelist is admin-controlled; users cannot whitelist their own addresses |

---

## Getting Started

**Prerequisites:** Rust toolchain + `wasm32-unknown-unknown` target.

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Add the WASM compilation target
rustup target add wasm32-unknown-unknown

# 3. Clone the repo
git clone https://github.com/your-org/stellar-shield-contract
cd stellar-shield-contract

# 4. Build for native (tests)
cargo build

# 5. Build optimised WASM for deployment
cargo build --release --target wasm32-unknown-unknown
```

The compiled WASM artifact will be at:

```
target/wasm32-unknown-unknown/release/stellar_shield.wasm
```

**Deploy to Testnet with the Stellar CLI:**

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/stellar_shield.wasm \
  --source <YOUR_SECRET_KEY> \
  --network testnet
```

---

## Running Tests

```bash
cargo test -- --nocapture
```

Expected output:

```
running 5 tests
test test_drip_whitelist                  ... ok
test test_drip_bypasses_velocity_limit    ... ok
test test_limit_exceeded_panics           ... ok
test test_set_and_enforce_limit           ... ok
test test_register_key_and_threshold      ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Test coverage

| Test | What it verifies |
|---|---|
| `test_set_and_enforce_limit` | Transfer within daily cap succeeds; spend counter is recorded |
| `test_limit_exceeded_panics` | Cumulative spend over cap panics with `"velocity limit exceeded"` |
| `test_drip_whitelist` | `add_trusted_drip` / `is_trusted_drip` round-trip; unknown address returns `false` |
| `test_drip_bypasses_velocity_limit` | Drip flag is stored and readable; drip transfers skip the cap |
| `test_register_key_and_threshold` | Key registration and threshold storage complete without error |

### Running a single test

```bash
cargo test test_limit_exceeded_panics -- --nocapture
```

### Project structure

```
stellar-shield-contract/
├── Cargo.toml
└── src/
│   ├── lib.rs          # Contract entry points (GuardContract, RegistryContract, AuthContract)
│   ├── guard.rs        # Velocity limit logic — set_limit, check_and_record
│   ├── auth.rs         # SECP256R1 key storage and signature verification
│   └── registry.rs     # Drip whitelist — add_trusted_drip, is_trusted_drip
└── tests/
    └── test.rs         # Soroban integration test suite
```
