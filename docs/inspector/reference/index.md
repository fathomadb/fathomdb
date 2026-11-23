# Inspector reference and initial scope

**Inspector 0.3.0 targets FathomDB 0.8.31 (schema 38) with GitHub Release
archive delivery and local verification.** The implemented native views and
launch flags below retain their actual request/read contexts, including
outside-`body` node/edge `properties`, read-only reserved `scope`, historical
`as_of` graph exploration (`as_of_write_cursor` / `as_of_epoch_ms`), and
record-keyed lexical retrieval (`record_keyed_not_merged` on body and
property routes; `not_inferred` on BM25F). Installed pairing is technically
qualified; no human-study validation is claimed.

| Boundary | Initial scope |
| --- | --- |
| Interface | Browser GUI served by a local application/service on loopback. |
| Database | One existing local database per session, at the build's current supported schema (schema 38 for Inspector 0.3.0; schema 37 and older refuse cleanly). |
| Writes | No Inspector editing, bootstrap, migration, repair or rebuild controls; ordinary Engine lifecycle/worker effects are allowed and disclosed. |
| Availability | External writers stopped; Inspector may own/start an Engine and its workers. No immutable as-found snapshot guarantee. |
| Retrieval | Supported real lexical, vector, hybrid, direct-read, and current/historical `as_of` graph operations with their actual contexts. |
| Models | Compatible already-cached query assets only; no implicit download or repair. |
| References | Database/session/class-qualified identities, revision evidence, and explicit unresolved/ambiguous outcomes. |
| Completeness | Counts, pages, samples, traversals and results disclose their scope and limits. |

All assessed correctness corrections and recommended design improvements are
carried forward from 0.2.x into 0.3.0. In particular, system-data labels,
incomplete provenance, class-specific lifecycle, metric/lexical attribution,
outside-`body` `properties` / `scope` safety, historical `as_of` subordination
to `D-01` disclosure, non-destructive refusals, and actual graphical
interaction checks are release work, not optional extras.

The engineering budgets remain binding (including `"properties"` in the 64 MiB
graph/lexical memory admission scans). Installed technical qualification covers
the exact `D4 = B` Ubuntu 24.04 / glibc 2.39, Linux `7.0.0-43-generic` /
Landlock ABI 8 and Chromium `156.0.8078.4` pair with bundled SQLite (zero
GTK4/WebKitGTK or Docker); the native binary requires at most GLIBC 2.39
(Ubuntu 22.04 / glibc 2.35 is not supported). Cached CPU needs Landlock
ABI >= 3. Current development launch commands are in getting started.
Disclosure defaults and the exact `--no-show-inactive`, `--show-erased` and
`--show-payloads` flags are settled in
[operating boundaries](../operations/index.md#observations-and-disclosure).
`.github/workflows/release.yml` (`build-inspector`) stages
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` and
`fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` for GitHub Release
attachment, and `local-release.py promote` remains available for local
`dist/inspector/0.3.0/` staging. No broader support should be inferred from
FathomDB's existing SDK platform matrix.

## Exact producer and model prerequisites

Inspector `0.3.0` builds against the workspace FathomDB Engine targeting
`0.8.31` (compiled schema `38`, dynamically recorded in `candidate.json` and
`build-description.json`) and admits closed Schema `38` databases; Schema `37`
(`0.8.30`) and older or missing-layout stores refuse cleanly. Inspector package
version `0.3.0` is independent of Engine/SDK versions. Existing retained
legacy/incomplete provenance stays labeled; matching text never supplies an
origin. Multiple vector source windows are not qualified.

Cached CPU uses `BAAI/bge-small-en-v1.5`, revision
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`, dimension 384, Engine identity
`fathomdb-bge-small-en-v1.5`. The build description lists required hashes for
`config.json`, `tokenizer.json` and `model.safetensors`. The existing source
cache is read-only to Inspector; the loader receives a sealed private view.
There is no GPU/reranker chooser or model bundle. A successful stored L2 view is
separate from availability of native cached-CPU hybrid retrieval.

`candidate.json` outside the tar identifies the clean packaging source SHA,
workspace engine version, compiled schema (`38`), actual compiled build
description, archive/executable hashes and qualified `D4 = B` support boundary.
This is unsigned local provenance, not a registry signature or proof of
reproducible compilation. Inner `SHA256SUMS` covers bundle files; outer
`SHA256SUMS` (and GitHub Release `.tar.gz.sha256`) covers the tar archive.
