# 🌊 StellarShield — Wave Program Contribution Plan

StellarShield participates in the Wave Program, where maintainers post scoped issues that contributors pick up during sprint cycles. This document describes the categories of work we post, what a well-scoped issue looks like, and how contributors can engage.

---

## How It Works

Maintainers open issues tagged with `wave` before each sprint begins. Each issue is self-contained — it has a clear goal, acceptance criteria, and an estimated effort level. Contributors claim an issue by commenting, then open a pull request against `main` before the sprint closes.

---

## Types of Work We Post

### 🐛 Bug Fixes

We post bugs that are reproducible, isolated, and don't require deep context to fix. Each bug issue includes the failing behaviour, the expected behaviour, and the relevant file or function.

**Examples of issues we'd post:**
- `check_and_record` does not reset the spend counter when `stored_day` overflows `u32` near the maximum ledger sequence
- `is_trusted_drip` returns a stale `true` after a drip address is re-used across contract redeployments
- TTL extension in `check_and_record` uses the same value for `min_ttl` and `max_ttl`, causing unnecessary ledger writes when the entry is still fresh

**What we provide:** a failing test that reproduces the bug, the expected output, and a pointer to the relevant lines in `guard.rs`, `registry.rs`, or `auth.rs`.

---

### ✨ New Features

Feature issues are scoped to a single contract function or a small cross-contract interaction. We avoid posting features that require redesigning storage layout or changing the auth model — those are maintainer-led.

**Examples of issues we'd post:**
- `remove_trusted_drip(admin, drip_address)` — add an admin-gated function to RegistryContract that removes a whitelisted address
- `get_spent(user)` — a read-only view function on GuardContract that returns the current day's cumulative spend for a user
- `get_limit(user)` — a read-only view function that returns the stored velocity limit, defaulting to `i128::MAX` if unset
- `reset_limit(user)` — lets a user remove their own limit entry from persistent storage, reverting to the unlimited default
- Multi-key support in AuthContract — allow a user to register a second P-256 key as a backup passkey

**What we provide:** the function signature, expected storage behaviour, auth requirements, and a stub test to fill in.

---

### 📖 Documentation

Docs issues target the README, inline code comments, or new standalone guides. All docs work is in plain Markdown or Rust doc comments — no toolchain beyond a text editor is required.

**Examples of issues we'd post:**
- Add `///` doc comments to every public function in `lib.rs` following the Rust doc convention
- Write a `CONTRIBUTING.md` that explains how to set up the local Soroban test environment
- Add a `DEPLOYMENT.md` walkthrough for deploying all three contracts to Testnet using the Stellar CLI
- Expand the Security Model section in `README.md` with a worked example of a replay attack and how the day-bucket prevents it
- Document the stroop denomination clearly in all function-level comments that accept `amount: i128`

**What we provide:** the target file, the section or function to document, and a style reference.

---

### 🧪 Testing

Test issues ask contributors to add coverage for edge cases, error paths, or scenarios not currently exercised by the five existing integration tests.

**Examples of issues we'd post:**
- Test that `execute_transfer` with a drip address succeeds even when the user has no limit set
- Test that `verify_sig` returns `false` for a user with no registered key, without panicking
- Test day-window rollover: advance the ledger sequence past `17_280` and assert the spend counter resets to zero
- Test that `set_threshold` stores the value correctly and that a second call overwrites the first
- Fuzz test `check_and_record` with random `amount` values near `i128::MAX` to catch overflow

**What we provide:** the scenario description, the Soroban test environment setup pattern to follow, and the assertion to verify.

---

### 🔧 Refactoring & Code Quality

These issues improve internal structure without changing external behaviour. They are a good entry point for contributors who want to understand the codebase before tackling features.

**Examples of issues we'd post:**
- Extract storage key construction into a shared `keys.rs` module so `("limit", user)` tuples are not repeated across functions
- Replace raw `Symbol::new(env, "...")` literals with typed constants to prevent key typos
- Add `#[allow(dead_code)]` audit — remove any unused imports or functions surfaced by `cargo clippy`
- Enforce `cargo fmt` and add a CI check that fails on unformatted code

**What we provide:** the specific files to touch, the pattern to follow, and confirmation that no storage key names may change (breaking change).

---

## Effort Labels

Every wave issue is tagged with one of three effort levels so contributors can self-select:

| Label | Typical scope |
|---|---|
| `effort:small` | Single function, one file, < 2 hours |
| `effort:medium` | 2–3 files, may touch tests, 2–6 hours |
| `effort:large` | Cross-contract change or new module, requires design discussion first |

---

## Acceptance Criteria

All pull requests must:

1. Pass `cargo test` with no new failures
2. Pass `cargo clippy -- -D warnings` with no new warnings
3. Include or update at least one test for any logic change
4. Not change existing storage key names (breaking change to deployed contracts)

Maintainers review within 48 hours of submission during an active sprint.
