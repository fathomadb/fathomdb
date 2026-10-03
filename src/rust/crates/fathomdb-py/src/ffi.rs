use super::*;

// ===== String validation (AC-068a / AC-068b) =========================

/// Reject strings carrying an embedded NUL or an unpaired UTF-16
/// surrogate codepoint (`U+D800..=U+DFFF`).
///
/// Both are valid Python `str` values but invalid for SQLite text
/// columns; AC-068a/b requires the binding to reject them BEFORE the
/// writer transaction opens (no-row-written invariant).
pub fn validate_ffi_string(value: &str) -> Result<(), String> {
    if value.as_bytes().contains(&0) {
        return Err("embedded NUL byte in FFI string".to_string());
    }
    for ch in value.chars() {
        let cp = ch as u32;
        if (0xD800..=0xDFFF).contains(&cp) {
            return Err(format!("unpaired UTF-16 surrogate U+{cp:04X} in FFI string"));
        }
    }
    Ok(())
}

pub(super) fn validate_ffi_string_py(value: &str) -> PyResult<()> {
    validate_ffi_string(value).map_err(WriteValidationError::new_err)
}

/// Extract a Python string into a Rust `String` and run
/// [`validate_ffi_string_py`]. PyO3's built-in `str` extraction already
/// fails on lone surrogates (the underlying `PyUnicode_AsUTF8AndSize`
/// raises `UnicodeEncodeError`); we re-raise those as the typed
/// `WriteValidationError` so callers can dispatch on a single class.
pub(super) fn extract_validated_str(value: &Bound<'_, PyAny>) -> PyResult<String> {
    match value.extract::<String>() {
        Ok(s) => {
            validate_ffi_string_py(&s)?;
            Ok(s)
        }
        Err(_) => Err(WriteValidationError::new_err(
            "string contains characters not representable as UTF-8 (lone surrogate)",
        )),
    }
}

/// `Option` lift of [`extract_validated_str`]: `None`/`None`-valued stays
/// `None` (preserving the all-`None` byte-identical unfiltered path); a
/// present value is extracted and validated through the same FFI gate as the
/// write path. Used by `search` for the G10 `SearchFilter` string fields.
pub(super) fn extract_opt_validated_str(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<String>> {
    match value {
        Some(v) if !v.is_none() => Ok(Some(extract_validated_str(v)?)),
        _ => Ok(None),
    }
}

/// Run the engine call inside `py.detach` and `catch_unwind`;
/// translate any escaping panic to `EngineError`.
///
/// `AssertUnwindSafe` wraps the caller's closure so we do not need to
/// require `UnwindSafe` from `f`. The engine's `Arc<dyn Embedder>`
/// makes the natural `UnwindSafe` bound unsatisfiable; the engine
/// itself takes care of its own atomicity post-panic.
pub(super) fn call_engine<R: Send>(
    py: Python<'_>,
    f: impl FnOnce() -> Result<R, RustEngineError> + Send,
) -> PyResult<R> {
    logging_subscriber::reject_reentry()?;
    let wrapped = AssertUnwindSafe(f);
    let result = py.detach(|| catch_unwind(wrapped));
    match result {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(engine_error_to_py(err)),
        Err(_) => Err(PanicException::new_err("engine panic (see logs)")),
    }
}
