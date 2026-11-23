# Inspector operating boundaries

The local Inspector 0.3.0 application starts and owns a FathomDB Engine
(targeting 0.8.31 / schema 38) to inspect one existing Schema 38 database
(Schema 37 / `0.8.30` and older databases refuse cleanly). These boundaries
apply to all implemented record, outside-`body` `properties` / reserved `scope`,
record-keyed lexical, vector, current and historical `as_of` graph, provenance,
and comparison views.

## Before attachment

Stop external writers, including another FathomDB application and raw SQLite
tools. The Inspector-owned Engine uses the product lock and may run workers.
The lock excludes cooperating writers; unrelated tools must also stay stopped.

The interface identifies Engine-backed attachment and its effects before open.
Normal Engine startup, background processing and shutdown can change database
files, projection readiness and lock/WAL state. This is not a byte-preserving
forensic view of the database as it existed before startup.

The database must already exist and match the build's supported schema.
Preflight refuses missing/empty files, missing/held lock, initially nonempty
WAL/journal, unsupported schema or corrupt/unavailable input without creating
or repairing a database. WAL created by the admitted Engine is normal.

## Effects, failures and model assets

Inspector provides inspection controls, not editing, actuation, projection
configuration, rebuild, migration or recovery commands. Startup is not
permission to invoke those actions to make a view work. An Engine-start failure
can follow partial ordinary lifecycle effects; report this rather than promise
that all failed attachments preserve every byte.

Models must already be compatibly cached. Initialization at Engine startup is
allowed where needed, but downloading, creating or repairing model caches is
not implicit. Missing/corrupt assets disable dependent operations; independent
browsing/text remains available where supported.

Preserve files/sidecars on refusal and investigate their owners. Do not delete
sidecars or run a lossy recovery verb as an automatic inspection fix. Initially
WAL-present, forensic immutable and already-running-application attachment
require separate future support.

## Observations and disclosure

Engine work can progress during the session. Counts, readiness and results
carry their observed context; separate requests do not imply one snapshot.
Earlier result content/revision is not silently replaced by a later value.
Detach closes the owned Engine through its supported lifecycle and expires
session references; a cancelled UI request is not proof native work stopped.

The owner adopted these defaults and exact launch flags on 2026-10-30; they
are implemented in the record-browser port and carry into 0.2.0. Each new view
and both pinned observations must apply them. Scope adoption establishes no
new runtime pass; no extra policy confirmation is needed.

| Data | Default | Launch flag |
| --- | --- | --- |
| Normally accessible data | Viewable. | None. |
| Retained history and inactive records | Viewable. | `--no-show-inactive` hides these records. |
| Still-retained content awaiting physical erasure | Content refused. | `--show-erased` permits viewing and explicit local export. |
| Telemetry | Structure and scoped counts, without payloads. | `--show-payloads` also permits payloads. |
| Already physically absent content | Unavailable. | No flag reconstructs it. |

All three flags may be combined. Every applicable gate must permit the read:
`--show-erased` does not bypass `--no-show-inactive` or unlock telemetry
payloads; `--show-payloads` does not unlock erasure-pending content. Restrictions
apply equally to `body` and `properties` detail (including withholding
`properties_bytes` and `typeof(properties)` metadata when unpermitted),
representations, current and historical `as_of` graph targets/edges/origins/
inline properties, query evidence, copying and export. Counts state the
policy-filtered scope. Flags allow disclosure, not invented retrieval
eligibility, ranks or missing data.

The policy is fixed for the launched session and shown as session state.
Normally accessible unrelated content remains usable when one payload is
refused. Engine workers may complete erasure before a read/export; absent
content remains unavailable. An old observation cannot bypass a newly applicable
restriction, and the tool cannot revoke an already copied/exported file.
Export is limited to explicitly selected, still-retained erasure-pending content
allowed by every gate, with bounded local output and honest partial/cancelled/
failed outcomes. It does not add general result dumps, saved investigations or
sharing. Session credentials and inspected content stay out of logs and URLs.
Cancellation, limits and incomplete/unknown evidence remain explicit.

The owner clarified on 2026-10-31 that retained-erasure display/export applies
**only if legitimate readable content exists**. Physical deletion remains
correct and unchanged. Autonomous physical deletion leaves no readable target.
Existing ExplicitMaintenance databases may retain a queued exact target scope;
`--show-erased` permits that content and narrow export while other gates still
apply. Inspector honors the stored policy and does not queue requests, activate
maintenance, retain content, delay erasure or reconstruct erased bytes.
Positive Engine end-to-end export is required only for a qualified supported
Engine that legitimately exposes this state; other native-view positives remain
required. This does not expand general export scope.

## Troubleshooting

| Outcome | Action |
| --- | --- |
| Missing/held lock or nonempty initial WAL/journal | Stop the producer normally and close all writers. Preserve the database and sidecars; investigate ownership. Do not create a missing lock sentinel or delete WAL to force admission. |
| Unsupported schema, layout or encoding | Use the matching qualified producer/database. No in-app migration or repair is provided. |
| Chromium not found or no graphical display | Install/use the declared existing Chromium and launch in a graphical session. Browser installation is a separate prerequisite, not an Inspector download. |
| Cached CPU unavailable | Check the exact model revision/hash/profile and Linux Landlock ABI >=3. Select model-free mode for independent inspection; do not repair or download assets as an implicit inspection action. |
| Invalid query/property/limit | Use the declared-property selector and nearby feedback; correct the input and retry explicitly. Previous results retain their own request/context. |
| Refused, expired or evicted observation | Read the stated policy/budget/scope. Refresh starts a new observation; it does not recreate the old snapshot or prove omitted data absent. |
| Busy, cancelled, partial or timed out | Wait for owned work to quiesce or detach/close normally. UI acknowledgement and completed native cleanup are distinct. Keep known partial scope; do not treat it as a complete empty result. |

Attachment state stays visible separately from transient request feedback.
Use title/name previews for discovery and exact identity for selection/copy.
Graph branch controls retain explicit traversal scope; query results show compact
summaries with expandable original request and separately timed evidence.
