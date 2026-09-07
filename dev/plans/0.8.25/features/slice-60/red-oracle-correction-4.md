# Slice 60 canonical-byte oracle correction

I60-01 confirmed that the original wire oracle compared both encoders against
`serde_json::Value` reserialization. That normalization imposes lexicographic
map ordering and cannot prove READY v5's declaration-order canonical bytes.

The request assertion now reads the separately frozen canonical request string.
The result assertion reads the new compact declaration-order result fixture;
both retain their existing decode-and-value-equality coverage. No request,
response, error, or public-contract intent changed.

| Path | Before SHA-256 | After SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_wire.rs` | `9f0626019fbdc5bc451d42cdcf97b98fc17218e2b49886f4bc4ce3ff0b60ee4f` | `378f9ecd651d965f661365708d9fdc66353888e9bf93f7a3583ae781cd01a3f9` |
| `dev/fixtures/slice60-fix1-canonical-result-v1.json` | new | `d1d267548d5e3028327223f22dd28444072c9c611ff36275027a59083baccb28` |

The follow-up lint-only commit replaces one needless `&&str` borrow with the
same `&str` argument. It does not change the byte oracle, decoded value, or
contract intent.
