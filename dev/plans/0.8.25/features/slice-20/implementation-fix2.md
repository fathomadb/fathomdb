---
title: 0.8.25 Slice 20 implementation FIX-2 response
status: FIX_IMPLEMENTED_AWAITING_REVIEW
review_cycle: 2
reviewed_commit: a37ba549
---

# Slice 20 implementation FIX-2 response

Preserved FIX-2 RED commit:
`8980b0a7edc7a6c816e122e66a31882287894194`.

## TDD chronology correction

FIX-1 RED commit `a89ca66ce5f42c5b0999c2908257d059d6144f13`
used `bad source` as its source-ID corruption oracle. That string is valid
under the public `SourceId` grammar. The FIX-1 GREEN commit changed the test to
`_bad-source`, so that test was both an invalid RED oracle and edited during
GREEN. The source-version case using `bad version` was outside the caller-ID
grammar and remained genuine RED evidence.

Published history is not rewritten. FIX-2 adds an explicitly labelled post-hoc
`_bad-source` regression, which already passes before its production change,
alongside genuinely failing Rust, Python, and TypeScript tests for a canonical
artifact/source revision consistently corrupted to `_bad-rev`. The FIX-2 test
files are frozen after the RED commit; GREEN changes production code only.

## Review finding under test

Design v7 requires every persisted artifact/source revision identity in a
relevant dependency chain to satisfy its public grammar independently of
reciprocal equality. A consistently corrupt canonical owner, source-version
mapping, canonical self-link, and derived source-link must fail `Storage` on a
relevant derived lookup and exact replay without introducing a whole-registry
scan.

## Implemented correction

Relevant-chain validation now independently applies the public constructors to
the persisted derived artifact revision, canonical artifact/source revision,
derived-link source revision, source-version mapping revision, and canonical
self-link revision. Malformed persisted values return `EngineError::Storage`
before equality checks in both read/replay and the Slice 25 prospective-source
loader. The checks remain within the already selected relevant chain and add no
registry scan.

The committed FIX-2 Rust, Python, and TypeScript test files were not edited
during GREEN.
