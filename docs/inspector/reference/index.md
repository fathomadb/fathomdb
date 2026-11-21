# Inspector reference and initial scope

**Inspector 0.2.0 uses local-only distribution.** The implemented
native views and launch flags below retain their actual request/read contexts.
Installed pairing is technically qualified; no human-study validation is claimed.

| Boundary | Initial scope |
| --- | --- |
| Interface | Browser GUI served by a local application/service on loopback. |
| Database | One existing local database per session, at the build's current supported schema. |
| Writes | No Inspector editing, bootstrap, migration, repair or rebuild controls; ordinary Engine lifecycle/worker effects are allowed and disclosed. |
| Availability | External writers stopped; Inspector may own/start an Engine and its workers. No immutable as-found snapshot guarantee. |
| Retrieval | Supported real lexical, vector, hybrid, direct-read, and graph operations with their actual contexts. |
| Models | Compatible already-cached query assets only; no implicit download or repair. |
| References | Database/session/class-qualified identities, revision evidence, and explicit unresolved/ambiguous outcomes. |
| Completeness | Counts, pages, samples, traversals and results disclose their scope and limits. |

All assessed correctness corrections and recommended design improvements are
carried into 0.2.0. In particular, system-data labels, incomplete provenance,
class-specific lifecycle, metric/lexical attribution, non-destructive refusals,
and actual graphical interaction checks are release work, not optional extras.

The engineering budgets remain binding. Installed technical qualification covers
the exact Ubuntu 22.04.5/glibc 2.35, Linux 7.0.0-43-generic/Landlock ABI 8 and
Chromium 156.0.8078.4 pair; the compatible binary requires at mostGLIBC 2.34.
Cached CPU needs Landlock ABI >=3; stock Ubuntu 22/kernel 5.15 is insufficient. Current development launch commands are in getting started. Disclosure defaults and the exact `--no-show-inactive`, `--show-erased`
and `--show-payloads` flags are settled in [operating boundaries](../operations/index.md#observations-and-disclosure);
Slice90 creates the final local dist bundle after80 readiness; no registry is used. No broader support should be inferred from
FathomDB's existing SDK platform matrix.

## Exact producer and model prerequisites

Selected Engine source is `6edc1155f039f87cd0a37660cb27ae4373bbce71` on0.8.29,
compiled schema 37. Inspector package version0.2.0 is independent of Engine/SDK
versions. Existing retained legacy/incomplete provenance stays labeled; matching
text never supplies an origin. Multiple vector source windows are not qualified.

Cached CPU uses `BAAI/bge-small-en-v1.5`, revision
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`, dimension 384, Engine identity
`fathomdb-bge-small-en-v1.5`. The build description lists required hashes for
`config.json`, `tokenizer.json` and `model.safetensors`. The existing source
cache is read-only to Inspector; the loader receives a sealed private view.
There is no GPU/reranker chooser or model bundle. A successful stored L2 view is
separate from availability of native cached-CPU hybrid retrieval.

`candidate.json` outside the tar identifies the clean packaging source SHA,
actual compiled build description, archive/executable hashes and qualified
support boundary. This is unsigned local provenance, not a registry signature
or proof of reproducible compilation. Inner `SHA256SUMS` covers bundle files;
outer `SHA256SUMS` covers the tar archive.
