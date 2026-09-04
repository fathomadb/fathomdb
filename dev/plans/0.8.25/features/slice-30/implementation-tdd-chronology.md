---
title: 0.8.25 Slice 30 implementation TDD chronology
status: ACTIVE
---

# Slice 30 TDD chronology

This record distinguishes observed RED tests from coverage added with or after
implementation. It does not retroactively describe post-hoc coverage as RED.

| Stage | Commit | Evidence |
| --- | --- | --- |
| RED-1 | `5e5a9307` | Initial direct dependency-closure failures. |
| RED-2 | `425ef1d0` | Restart and recovery failures. |
| RED-3 | `f28e00d0` | Governed read-barrier failures. |
| RED-4 | `91b6d2aa` | Physical writer-fence failures. |
| GREEN-1 | `21ff56dc` | Initial implementation. This commit also added test coverage; those additions are compatibility evidence, not pre-implementation RED evidence. |
| PIN-1 | `ee85922c` | Approved governed-surface allowlist and exact pin. |
| FIX-1 RED | `58a9b604` | Reviewer-driven failures for source-view eligibility, measured physical proof rollback, post-commit cursor publication, and strict TypeScript response decoding. |
| FIX-1 GREEN | `3a9aa8cf` | Measured proof, shared source eligibility, cursor publication ordering, strict response decoder, and this chronology. |
| FIX-2 RED | `d5920ef9` | More than 192 ineligible vector candidates demonstrated the pre-hydration truncation gap and missing degradation signal. |
| FIX-2 GREEN | `638109a6` | The vector arm now fails closed before bounded KNN when registered dependency eligibility cannot be represented, with the existing vector degradation signal. |
| RACE COVERAGE | `e78f8693` | Companion coverage proves worker-before-admission and admission-before-worker projection ordering, plus an installed non-null TypeScript closure read. This was added after the behavior and is not claimed as RED. |
| FIX-3 RED | `2cbb1c4a` | BM25F leaked barrier-fenced and expired-source dependents; TypeScript accepted invalid phase/cause/proof/blocker combinations. Identity/provenance rollback cases already failed closed under the legacy orphan guard. |
| FIX-3 GREEN | `1b4f59b6` | BM25F shares strict dependency eligibility across corpus and candidates; TypeScript mirrors the Rust closure-state matrix; physical proof measures exact pre-delete artifact-revision, source-link, and source-version scope. |
| FIX-4 RED | `33880fd2` | Logical purge failed when an unrelated artifact shared its source bucket; open and point read accepted a nonzero completed proof. |
| FIX-4 GREEN | `77922947` | Physical proof scope is root-aware, and persisted proof semantics are cause-aware and zero-validating at open and point read. |
| FIX-5 PROPERTY COVERAGE | `83c64aa2` | Bounded generated mutations sampled common and physical proof-count classes across soft and physical causes, requiring identical fail-closed point-read and reopen behavior. This post-implementation property coverage did not expose a new RED defect. |
| FIX-6 COVERAGE CORRECTION | `467f5ac1` | Each generated nonzero value now traverses the complete deterministic 18-cell cause/field matrix and proves the unmodified receipt is readable before mutation. No product RED defect was exposed. |
| FIX-7 RED | `1cffa95b` | The first full serial workspace gate exposed a real ordinary-transition vector/readiness regression and four superseded predecessor assertions plus one overbroad cache fixture. Updating the Slice 25 closure contract then exposed a second real defect: a same-batch registration was newer than the generation seen by its later closure proof. |
| FIX-7 CONTRACT RECONCILIATION | `39ee7720` | Historical v10 recall now uses its raw FTS surface; Slice 25 tests assert Slice 30 closure admission and operation-order generation precedence; TC-68 mutates only its cache key; TC-90 proves bounded `BEGIN IMMEDIATE` contention succeeds. The ordinary-transition and same-batch-generation tests remained RED at the prior implementation. |
| FIX-7 GREEN | `6082a4c2` | Ordinary transition again maintains vector metadata synchronously and selectively, while dependent closure keeps full projection removal. Actuation makes an earlier reserved dependency generation visible to later closure admission within the same rollback-safe savepoint. All six formerly failing targets pass serially. |
| FIX-8 COVERAGE/DOC CORRECTION | `57628948` | Full-workspace verification showed the projection-worker pause was already after successful `BEGIN IMMEDIATE`; its extra optional-attribution wait was removed. The worker-first oracle was corrected to distinguish the retained, lifecycle-filtered root vector from the purged dependent vector, and TC-90's top-level narrative now separates historical mechanism from current behavior. No product defect was exposed. |
| FIX-8 REVIEW CORRECTION | `7185d7b0` | Both race orderings now prove physical vec0 row identity rather than terminal-readiness or total-count proxies. Remaining TC-90 comments describe the ignored loops as post-fix contention instruments and preserve baseline reproduction claims as history. |
| VERIFICATION FIX-1 RED | `27ac460b` | A same-open-Engine point read accepted missing, malformed, and regressed durable closure-sequence singleton state that reopen correctly rejected. |
| VERIFICATION FIX-1 GREEN | `75617521` | Open and keyed closure reads now share the canonical singleton/max-sequence validator. Independent review and a fresh-wheel replay of the original corruption case pass. |

Slice 30 closed after verification FIX-1. The status record identifies the
final implementation and verification evidence.
