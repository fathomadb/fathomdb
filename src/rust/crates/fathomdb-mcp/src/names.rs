//! Store-name grammar (MR-005.3). A name is checked before any filesystem
//! access, so a path-like name can never reach `open`.

/// Why a store name cannot be used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameError {
    /// The name breaks the grammar (path separators, `..`, absolute paths).
    BadName,
    /// The name is well formed but not addressable.
    Unknown,
}

/// Longest accepted store name, in bytes.
pub const MAX_STORE_NAME_LEN: usize = 64;

/// Check `name` against `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$` (and not `.` or
/// `..`, which the first-character rule already excludes).
///
/// # Errors
/// [`NameError::BadName`] when the name breaks the grammar.
pub fn validate_store_name(name: &str) -> Result<(), NameError> {
    let bytes = name.as_bytes();
    let Some(first) = bytes.first() else {
        return Err(NameError::BadName);
    };
    if bytes.len() > MAX_STORE_NAME_LEN || !first.is_ascii_alphanumeric() {
        return Err(NameError::BadName);
    }
    if name == "." || name == ".." {
        return Err(NameError::BadName);
    }
    if bytes.iter().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')) {
        Ok(())
    } else {
        Err(NameError::BadName)
    }
}
