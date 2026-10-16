use super::*;

// ===== context pack (0.8.28 Slice 41) =====================================
//
// Package-level pure operations over the JSON wire of
// `fathomdb_engine::context_pack`. The Python wrapper (`fathomdb/_sdk_context_pack.py`)
// owns the dataclasses and builds the JSON with `ensure_ascii=True`, so a lone
// surrogate arrives as a `\udXXX` escape the core refuses as an invalid
// argument. Every refusal is `InvalidArgumentError` naming the field; the core
// is `catch_unwind`-wrapped like `rerank`.

fn context_pack_call(
    call: impl FnOnce() -> Result<String, fathomdb_engine::ContextPackError>,
) -> PyResult<String> {
    catch_unwind(AssertUnwindSafe(call))
        .map_err(|_| PanicException::new_err("context pack panic (see logs)"))?
        .map_err(|error| InvalidArgumentError::new_err(error.message().to_string()))
}

#[pyfunction]
pub(super) fn _pack_context_json(request: &str) -> PyResult<String> {
    context_pack_call(|| fathomdb_engine::pack_context_json(request))
}

#[pyfunction]
pub(super) fn _encode_round_body_json(parts: &str) -> PyResult<String> {
    context_pack_call(|| fathomdb_engine::encode_round_body_json(parts))
}

#[pyfunction]
pub(super) fn _decode_round_body_json(body: &str) -> PyResult<String> {
    context_pack_call(|| fathomdb_engine::decode_round_body_json(body))
}
