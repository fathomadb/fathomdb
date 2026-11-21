# Extractor output, provenance and vector kinds

`ingest_with_extractor` / `ingestWithExtractor` runs a caller-selected external
process using the `fathomdb.extract.v1` NDJSON protocol. FathomDB owns transport
validation and storage; your extractor owns semantic extraction. This path is
distinct from writing versioned provenance and registering source dependencies.

## Caller-owned source attribution

Each input document supplies `source_doc_id` and `body`. The stored source ID
comes from the caller's batch. For a single-document batch, attribution is
unambiguous and a model's echoed source ID is ignored. For multiple documents,
the echo must select an ID in that same caller-supplied batch; a missing or
unknown selection fails with `ExtractorError` instead of guessing provenance.

The extractor path writes legacy canonical node/edge variants. It does not
create `WriteProvenanceV1` artifact/source-revision identities, UTF-8 source
locators, or registered dependency relationships automatically. A source ID
supports erasure attribution; it is not by itself source-byte evidence or a
dependency chain. If exact revision/span evidence and dependency tracing are
required, author the versioned provenance and dependency requests through the
ordinary governed write/register or atomic actuation APIs.

The partial-span trace fix in 0.8.30 authenticates registered relationships with
valid source locators; it does not retrofit extractor output. See
[frozen evidence and trace](frozen-evidence.md#trace-registered-dependencies).

## Entity identity and deduplication

Entity identity derives from the extractor's type and name; edge identity derives
from the endpoint identities and relation. This is deterministic identity
construction, not entity resolution or a first-class namespace system.
An existing active entity is reused. Within one response, duplicate logical IDs
keep their first occurrence. Shared entities consequently retain one source ID,
not a multi-source provenance set; erasing a later document does not remove an
entity attributed to the first document. Model output does not establish semantic
truth, and the ingest call is batched rather than one whole-corpus transaction.

## A kind can be text-searchable without being dense-indexable

Dense node projection supports the fixed kinds `email`, `article`, `paper`,
`meeting`, `note`, `todo`, and `doc`. Extractor entity types such as `person` or
`entity` can be stored and used by text/graph routes but do not acquire dense
vectors merely because a vector projection is declared. Inspect
`vector_unsupported_kinds` / `vectorUnsupportedKinds` and projection readiness.
The report is computed at declaration time; a new unsupported kind written later
does not retroactively update that receipt. Custom searchable properties and
filterable attributes also do not expand the dense kind vocabulary.

Choose kinds for their stored meaning and verify the retrieval route your
application actually needs. A healthy embedder does not imply that every record
has a vector. See [embedding](../embedder.md) and
[search filtering](hybrid-search-filtering.md).
