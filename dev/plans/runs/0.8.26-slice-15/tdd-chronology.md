# Slice 15 prototype chronology

- `f1e608c2`: transient RED contract tests; compiler reports absent graph
  evidence types and methods.
- `79aca61b`: fixture validity correction removes edge bodies that required an
  unrelated embedder; assertions unchanged.
- `ad640796`: fixture validity correction makes the explicit seed satisfy the
  existing target-kind filter; assertions unchanged.
- `b6478d1f`: first test-hooks-only GREEN shortcut.
- `6235daab`: release-mode exploratory harness.
- `6ae37d1e`: exploratory shortcut evidence, explicitly classified as
  non-authoritative.
- `1a1e5041`: real bounded, batched full-provenance treatment.
- `a8679b96`: concurrent graph measurement harness.
- `54e96c1c`: writer-interference campaigns.
- `e77b117b`: correction that moves hydration and minting into the graph reader
  transaction, removing primary-connection serialization from the treatment.
- `7ec88e1f`: 50-result and mixed node/edge evidence arms.
- `50e471d3`: explicit corrupt hash/locator, context mismatch, foreign
  reference, post-erasure, and held-WAL refusal tests.
- `0adbb37f`: complete 10k-work, 100 KiB, process-isolated RSS, paired
  erase/excise, concurrency, and carrier measurement matrix.
- `2d0a346c`: complete inline-carrier encoder used for the option-B comparison.
- `52d1565f`: exact two-preflight, row-count, index, source-dedup, and hashing
  work counters.
- `d1f6d8a4`: durable raw samples, reproducible derivation, environment,
  surface manifest, analysis, and recommendation.
- `bf6a698e`: planned teardown of all transient runtime, test-hook, and test
  code while retaining the durable decision evidence.

The two RED corrections repair setup preconditions and do not weaken any
oracle. The subsequent exploratory result exposed that the shortcut was not the
approved post-selection hydration design. The commit remains reachable to make
that finding auditable; its numbers are not used as decision evidence.

The authoritative campaign followed the reader-transaction correction and
exercised the complete approved functional and measurement matrix. The final
teardown commit preserves the full experimental chain in history while leaving
no prototype public method, field, codec, binding, schema, persistence path, or
runtime behavior in the final tree.
