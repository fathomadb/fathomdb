use super::*;

/// Run a blocking engine call inside `tokio::task::spawn_blocking` so
/// the libuv event loop stays free, wrapping it in `catch_unwind` so
/// panics surface as the typed [`panic_error`] rather than aborting the
/// host process. `AssertUnwindSafe` lets us thread the closure through
/// without requiring `UnwindSafe` from the engine's `Arc<dyn Embedder>`
/// substrate.
pub(crate) async fn call_engine<R, F>(f: F) -> Result<R>
where
    R: Send + 'static,
    F: FnOnce() -> std::result::Result<R, RustEngineError> + Send + 'static,
{
    let join_result = tokio::task::spawn_blocking(move || catch_unwind(AssertUnwindSafe(f))).await;
    match join_result {
        Ok(Ok(Ok(value))) => Ok(value),
        Ok(Ok(Err(err))) => Err(engine_error_to_napi(err)),
        Ok(Err(_panic)) => Err(panic_error()),
        Err(join_err) => Err(typed_error(
            CODE_PANIC,
            format!("spawn_blocking join error: {join_err}"),
            JsonValue::Null,
        )),
    }
}

/// Sync sibling of [`call_engine`]: wrap a non-blocking accessor in
/// `catch_unwind` so a panic on the JS thread surfaces as [`panic_error`]
/// instead of unwinding into napi-rs's default `GenericFailure` path.
pub(crate) fn call_engine_sync<R, F>(f: F) -> Result<R>
where
    F: FnOnce() -> Result<R>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(_panic) => Err(panic_error()),
    }
}
