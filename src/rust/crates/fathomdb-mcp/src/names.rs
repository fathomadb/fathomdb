//! Store-name grammar (MR-005.3).

/// Why a store name cannot be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameError {
    /// The name breaks the grammar (path separators, `..`, absolute paths).
    BadName,
    /// The name is well formed but not addressable.
    Unknown,
}

/// Check `name` against `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$`.
pub fn validate_store_name(_name: &str) -> Result<(), NameError> {
    todo!()
}
