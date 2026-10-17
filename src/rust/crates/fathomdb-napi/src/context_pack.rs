use super::*;

// ===== context pack (0.8.28 Slice 41) =====================================
//
// Package-level pure operations over the JSON wire of
// `fathomdb_engine::context_pack`; `src/ts/src/context-pack.ts` owns the
// TypeScript types and builds the JSON. `JSON.stringify` escapes a lone
// surrogate, so it reaches the core as an escape the core refuses. Every
// refusal is `FDB_INVALID_ARGUMENT`; a panic is `FDB_PANIC`.

fn context_pack_call(
    call: impl FnOnce() -> std::result::Result<String, fathomdb_engine::ContextPackError>,
) -> Result<String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(call))
        .map_err(|_| panic_error())?
        .map_err(|error| typed_error(CODE_INVALID_ARGUMENT, error.message(), JsonValue::Null))
}

/// JSON wire of `packContext`.
#[napi(js_name = "packContextJson")]
pub fn pack_context_json(request: String) -> Result<String> {
    context_pack_call(|| fathomdb_engine::pack_context_json(&request))
}

/// JSON wire of `encodeRoundBody`.
#[napi(js_name = "encodeRoundBodyJson")]
pub fn encode_round_body_json(parts: String) -> Result<String> {
    context_pack_call(|| fathomdb_engine::encode_round_body_json(&parts))
}

/// JSON wire of `decodeRoundBody`.
#[napi(js_name = "decodeRoundBodyJson")]
pub fn decode_round_body_json(body: String) -> Result<String> {
    context_pack_call(|| fathomdb_engine::decode_round_body_json(&body))
}
