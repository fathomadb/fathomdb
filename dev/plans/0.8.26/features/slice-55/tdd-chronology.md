# Slice 55 TDD chronology

## Initial RED/GREEN

- `fa8255ae` committed the validator mutations, Python/TypeScript real-surface
  oracles, and standalone TypeScript rerank tests before production code. The
  validator tests failed because the checker did not exist; TypeScript
  typecheck failed because `rerank` was not exported.
- `0671bf5b` added the companion map and pure validator, exact runtime
  introspection, the Rust N-API and TypeScript rerank peer, and maintained
  design/interface guidance. The validator reported 69 signed tokens / 44 live
  canonical operations, Python passed 19/19 focused surface tests, and the full
  TypeScript suite passed 466/466.

## Code-review RED/GREEN

- Independent review found one acceptance gap and three correctness issues: no
  nonempty positive-depth feature-off proof, missing canonical IDs in mismatch
  diagnostics, an overbroad all-SDK design claim, and incomplete interface
  summaries presented as exhaustive.
- `ac87f10f` added the feature-off proof and a diagnostic assertion. The
  mutation suite RED failed because the missing-operation message omitted
  canonical ID `read.get_many`.
- `498289f1` made the diagnostic GREEN and narrowed/qualified the governing
  prose. The focused rerank/parity suite passed 4/4, validator mutations passed
  10/10, and independent rereview returned PASS with no P0–P3 findings.

## GPT-6 Astra review RED/GREEN

- Post-completion GPT-6 Astra medium review found two real P2 defects: malformed
  runtime query/body types reached N-API, and feature-enabled empty input
  resolved reranker device policy. Its initial discovery P1 was withdrawn after
  repository export/naming authority was applied.
- `a057d9e4` committed both RED reproductions before production changes.
- `a5859cd9` added local pre-native rerank string guards and moved the shared
  empty-pool return ahead of device-policy resolution. Focused downstream tests
  and the 117/117 canonical gate passed; Astra rereview returned PASS.
