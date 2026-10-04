//! napi-rs binding from the TypeScript SDK to `fathomdb-engine`.
//!
//! FFI safety contract (mirrors the PyO3 binding in `fathomdb-py`):
//!
//! 1. Every method that may block inside the engine runs its blocking
//!    body inside `tokio::task::spawn_blocking`, so the libuv main
//!    thread is never tied up. napi-rs's `#[napi] async fn` wraps
//!    return values as JS `Promise<T>`.
//! 2. Engine entry points return typed errors via [`engine_error_to_napi`] /
//!    [`engine_open_error_to_napi`] — single-switch mapping with no
//!    catch-all arm; the binding fails to compile when the Rust variant
//!    set drifts from the TS leaf-class set (AC-060a).
//! 3. Every string crossing the FFI is checked by [`validate_ffi_string`]
//!    for embedded NUL or unpaired UTF-16 surrogates BEFORE the writer
//!    transaction opens (AC-068a / AC-068b).
//! 4. Engine panics are caught via `catch_unwind` inside the spawn-blocking
//!    body and rethrown as a distinct `FathomDbPanicError` (code
//!    `FDB_PANIC`); the host process is not aborted (AC-067).
//
// why: napi-rs catches panics by default but throws a generic JS
// `Error` with `Status::GenericFailure` and a Rust-formatted message,
// which is hard to assert on. Explicit `catch_unwind` + a stable
// `FDB_PANIC` code lets the TS-side `rethrowTyped` map panics to a
// distinct `FathomDbPanicError` class that is intentionally NOT a
// `FathomDbError` subclass: panic is a contract bug, not a typed
// engine outcome.

mod shared;
use shared::*;

mod errors;
mod subscriber;
pub use errors::*;
mod execution;
use execution::*;
mod embedding;
pub use embedding::*;
mod types;
pub use types::*;
mod projection;
pub use projection::*;
mod engine;
pub use engine::*;
mod admin;
pub use admin::*;
mod read_search;
pub use read_search::*;
mod graph_evidence;
pub use graph_evidence::*;
mod write;
pub use write::*;
#[cfg(any(test, feature = "test-hooks"))]
mod test_support;
#[cfg(any(test, feature = "test-hooks"))]
pub use test_support::*;
