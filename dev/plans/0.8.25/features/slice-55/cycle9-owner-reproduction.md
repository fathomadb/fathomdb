---
title: 0.8.25 Slice 55 cycle-9 owner reproduction
status: FAIL
---

# Slice 55 cycle-9 owner reproduction

This read-only reproduction validates implementation review cycle 9 without
starting another FIX cycle. The exact reviewed product remains
`87f6c707c761e0693c739241e1b97fd173d15df8`; the repository HEAD containing
the review record is `5f9ca5fc9d14691a03c9f8323c99f4b57f6acdc3`.

## Exact installed-artifact reproduction

The source-wrapper import reached the documented stale worktree extension and
failed because that extension lacks `ProjectionGenerationError`. It is not
candidate evidence.

The retained exact FIX-8 wheel environment loaded its installed package from
`site-packages`. Decoding the checked-in canonical trace fixture printed:

```text
registeredDependencyGeneration=1 dependencyGenerationBoundary=0
```

The decoder accepted two additional impossible states:

```text
zero  ACCEPTED 0 1
ahead ACCEPTED 2 1
```

The cases set the edge's registered generation and boundary to `(0, 1)` and
`(2, 1)` respectively. A valid traced relation requires:

```text
1 <= registeredDependencyGeneration <= dependencyGenerationBoundary
```

The exact command used the installed FIX-8 virtual environment with
`PYTHONPATH` removed:

```text
env -u PYTHONPATH \
  /tmp/fathomdb-s55-fix8-wheel.G2wwHT/venv/bin/python \
  -c '<load trace-v1.json, mutate the two generation fields, and call
      fathomdb.engine._decode_dependency_trace_response>'
```

This independently confirms the cycle-9 P1. It does not change the review
verdict or authorize implementation.

## Bounded correction blueprint

If the owner authorizes another FIX cycle, it is limited to:

1. An isolated, design-authorized fixture correction changing the canonical
   trace boundary from 0 to 1 while preserving edge generation 1.
2. Genuine REDs in Rust, Python, and TypeScript for edge generation zero and
   edge generation greater than the decoded boundary. The exact error path is
   `/dependencyEdges/<index>/registeredDependencyGeneration`.
3. Rust encoder validation before canonical bytes are emitted, plus Rust,
   Python, and TypeScript decoder validation after the boundary is decoded.
   Every edge must satisfy the same `1..=boundary` invariant enforced by live
   trace collection.
4. Replacement of the invalid root-workspace TypeScript command in the plan
   with the established package-directory route:

   ```text
   cd src/ts
   npm run build:debug
   node --test --test-name-pattern slice55 dist/tests/*.test.js
   ```

5. Focused wire and cross-SDK RED/GREEN evidence, exact wheel and N-API
   verification, chronology, and a new independent implementation review.

No integrity-scanner, trace-query, ranking, schema, public-shape, packaging,
tagging, publication, or release-state change belongs in this correction.
