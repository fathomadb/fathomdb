# Slice 60 RED-oracle correction 8 — pointer escaping

Independent audit found that the frozen Slice 60 top-level unknown-member case
inserted the already RFC 6901-escaped JSON member name `a~1b~0c` while expecting
the same escaped pointer. That did not exercise escaping the raw slash and
tilde required by the oracle's stated intent.

This isolated correction changes only that inserted raw member name to
`a/b~c`, retaining the expected `/a~1b~0c` pointer, reason, and all other
assertions. No product code, fixture, public contract, or test intent changes.

| Path | Old SHA-256 | New SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_wire.rs` | `378f9ecd651d965f661365708d9fdc66353888e9bf93f7a3583ae781cd01a3f9` | `8b0fbeb539975c09212d5e6903a231545a1a293712efb7598e63bfdb8a05cb88` |
