---
title: 0.8.25 Slice 10 independent design review — cycle 1
status: FAIL_RESOLVED_BY_FIX_2
reviewed_design_version: 3
---

# Slice 10 independent design review — cycle 1

The independent reviewer returned **FAIL** on version 3. This is the second and
penultimate allowed design correction.

| Severity | Finding | FIX-2 disposition |
| --- | --- | --- |
| P1 | Call paths, optional standalone scope, component/artifact/exclusion vocabularies, and migration were not closed. | Version 4 defines the missing object, nullable standalone arm, and every closed vocabulary/schema. |
| P1 | Numeric/boolean-only metrics could not classify qualitative lifecycle outcomes. | Version 4 supports closed enum measurements and manifest-fixed measurement roots. |
| P1 | Historical positive call counts lacked a reproducible source/checkpoint derivation. | Version 4 binds two commit-addressed source blobs, config, checkpoint cells, and lower-bound call semantics. |
| P1 | Blocked post-cutover runs would permanently deadlock lint. | Version 4 makes blocked a valid sidecar outcome that satisfies presence but cannot support a successful claim. |
| P2 | Sidecar identity omitted the classification content and publish could overwrite a race. | Version 4 hashes the full body and uses atomic hard-link no-replace publication. |
| P2 | Policy/manifest paths and fast/heavy/all commands were not exact. | Version 4 fixes both paths, closes their fields, and lists all selected commands. |
| P3 | Path count and frontmatter were inconsistent. | Version 4 describes three classified run groups and consistently records FIX-2. |

No implementation begins until cycle 2 returns READY with no unresolved P1/P2.
