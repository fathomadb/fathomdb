# Slice 135 Python frozen-read error-field repair — 2026-10-07 UTC

The installed-wheel capability exercise found that
`ReadContextV1(schema_version=2)` raised `FrozenReadError` with the expected
message but left `.reason` and `.field_path` empty. Four [red regression
cases](red.log) reproduced this at the native constructor and public
`freeze_read_context` boundary for schema versions 0 and 2. The one-branch
binding repair uses the existing typed-error mapper, preserving the exception
class and message while setting both documented fields.

The [before/after manifest](manifest.json) binds source commits, wheel/native
SHA-256 hashes, exact commands and checks. The rebuilt wheel returned
`unsupported_schema_version` and `/schemaVersion` in both fields. All 36
selected installed frozen-read tests passed. The refreshed
[44-operation result](after-capabilities.json) records **40 executed, zero
failed, one committed-closure gap and three unavailable provider/model cases**.
The exact wheel, raw outputs, test-source hashes, Clippy output and
[SHA256SUMS](SHA256SUMS) are retained here.

The repair changes the Python binding constructor; the engine, query and
schema source trees used by the E01–E12 timing comparison are unchanged.
The remaining operation gaps, full contract conditions, other platforms and
broader Phase 1 latency qualification are still open. The full repository
verification gate was deferred to the next consolidated source gate; the
scoped Clippy, format, Ruff and installed tests passed.
