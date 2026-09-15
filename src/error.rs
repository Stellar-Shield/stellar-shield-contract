use soroban_sdk::contracterror;

/// Every way these contracts refuse on purpose.
///
/// Numbered and stable, so an integrator branches on `LimitExceeded` rather
/// than string-matching a panic message that could be reworded at any time.
///
/// Note what is NOT here: there is no `NotAdmin`. An unauthorised caller is
/// stopped by `require_auth` on the stored admin, which fails at the host with
/// `Error(Auth, InvalidAction)` before this contract runs a line. A contract
/// error would mean we had let them in far enough to produce one.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The registry has not been initialised with an admin yet.
    NotInitialised = 1,
    /// Already initialised. An admin cannot be silently replaced.
    AlreadyInitialised = 2,
    /// Amounts and limits must be greater than zero.
    BadAmount = 3,
    /// This spend would take the user past their daily limit.
    LimitExceeded = 4,
    /// The running total would overflow i128.
    Overflow = 5,
    /// No passkey is registered for this user.
    NoKey = 6,
    /// A multi-sig threshold must be at least 1.
    BadThreshold = 7,
}
