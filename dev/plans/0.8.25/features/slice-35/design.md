---
title: 0.8.25 Slice 35 — eligibility and optional frozen-read design
status: READY
design_version: 6
review_fix: 4
review: design-reconciliation-review.md
target_release: 0.8.25
depends_on: 30
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
adr: dev/adr/ADR-0.8.25-eligibility-and-frozen-reads.md
---

# Slice 35 design

## Authority, outcome, and limits

This design implements S35-R1 through S35-R6, the retained core of
R25/AC25-35, Memex needs 7/8 and the contract part of 21, N25-02/N25-03, and
A25-01/A25-05/A25-06. It succeeds Slices 15–30 without changing their identity,
provenance, dependency, actuation, or lifecycle authorities.

Slice 35 makes the existing closed `SearchFilter` eligibility contract uniform
and pre-truncation across body FTS, edge FTS, property FTS, vector KNN, and the
search graph arm. It adds an opt-in, stateless, Engine-minted frozen context for
the search family. Ordinary reads retain their current hot path and cost.

It does **not** add another filter DSL, owner/scope aliases without stored native
fields, membership, existence, arbitrary JSON search predicates, retained
snapshots, leases, historical replay, or access-control policy. Slice 60 adds
constrained standalone graph traversal; Slice 50 consumes the same frozen-token
validator for evidence resolution. Persisted leases remain allocated to 0.8.27.

## Requirements and acceptance criteria

| Requirement | Acceptance |
|---|---|
| S35-R1 One closed eligibility vocabulary | S35-AC1 preserves `SearchFilter` fields and declared exact attributes; representable `FilterTerm::Json` returns `InvalidFilter` on search, while range, membership, existence, owner, and scope terms are absent from the ranked-search schema. |
| S35-R2 Eligibility before bounded selection | S35-AC2 places the sole eligible result below each prior body-FTS, edge-FTS, property-FTS, vector, graph-seed, and graph-frontier cap and still returns it, or returns a declared whole-arm soft fallback before that arm runs. |
| S35-R3 One all-arm meaning | S35-AC3 proves the operation/arm matrix below and that no accepted term is Rust-post-filtered after a branch cap. |
| S35-R4 Optional frozen context | S35-AC4 mints and consumes a content-free authenticated token across restart; exact state succeeds, while tamper, another database, append, lifecycle, dependency, projection, and rebuild drift fail typed. |
| S35-R5 Current safety fences win | S35-AC5 races erasure/lifecycle mutation with consume: one SQLite snapshot linearizes the operation; a change committed before snapshot validation causes drift and no bytes are returned. |
| S35-R6 Additive parity and bounded cost | S35-AC6 proves Rust/Python/TypeScript wire parity, unknown-version/field behavior, unchanged legacy results/SQL, token <= 1 KiB, and the selected CPU/Windows/CUDA routes. |

## Eligibility contract

### Public vocabulary

Slice 35 keeps the shipped, non-exhaustive `SearchFilter` as the one search
eligibility value:

```text
SearchFilter {
  source_type?: string,
  kind?: string,
  created_after?: i64,       // created_at >= bound, Unix seconds
  status?: string,           // projected application status, not lifecycle state
  attributes: [(projection_name, canonical_text)]
}
```

The conjunction is implicit AND. Values use SQLite binary text equality; there
is no case folding or Unicode normalization. Attribute pairs are sorted by
`(name, value)` for canonical encoding. Repeated same-value pairs are redundant;
repeated differing values retain the existing implicit-AND, unsatisfiable
meaning. Frozen-context construction preserves those legacy semantics rather
than rejecting an ordinary filter accepted today. Empty strings are valid
values and remain distinct from an absent declared attribute. `created_after`
is an inclusive integer lower bound. Node `kind` means canonical node kind;
edge `kind` means relation kind. `source_type` derives from node kind and equals
`edge_fact` for edge rows. `status` is the existing vec0/application metadata
field and is distinct from canonical lifecycle `state`.

Declared attributes are exact TEXT projections only. The projection registry
must contain the name with role `filterable`, and the physical
`canonical_attributes(write_cursor, attr_name, attr_value)` and dynamic vec0
metadata column are its authorities. No range capability is inferred from a
TEXT value. Memex owner or scope constraints are supported only when the caller
declares and populates exact `filterable` projections named by its own schema;
FathomDB does not assign their meaning.

`Filter` remains the common closed syntax for `read.list`; its arbitrary
`FilterTerm::Json` predicates remain valid only on that unranked canonical
surface. Converting a JSON term to search stays a typed rejection. The accepted
0.8.11 filter ADR is amended, not replaced: Slice 35 changes execution placement
for the existing search subset, not its vocabulary.

### Backend support and lowering

| Term | Body/edge/property FTS | Vector phase 1 | Search graph seed/frontier | Unranked `read.list` |
|---|---|---|---|---|
| `source_type` | SQL predicate on the candidate's own row before rank/candidate limit | native vec0 metadata predicate before KNN | target-node predicate before seed/frontier/result caps | constant-fold from requested kind |
| `kind` | node/edge real-column predicate on the candidate's own row before rank/candidate limit | native vec0 metadata predicate before KNN | target-node real-column predicate before caps | canonical real-column predicate |
| `created_after` | indexed join to the candidate's own vec0 metadata before rank/candidate limit | native vec0 metadata predicate before KNN | indexed target-node metadata join before caps | existing JSON predicate |
| `status` | indexed join to the candidate's own vec0 metadata before rank/candidate limit | native vec0 metadata predicate before KNN | indexed target-node metadata join before caps | existing JSON predicate |
| declared exact attribute | `EXISTS` on the candidate's indexed EAV row before rank/candidate limit | native dynamic vec0 metadata predicate before KNN | `EXISTS` on the target node's indexed EAV row before caps | unsupported and unchanged; `read.list` has no attribute term |
| lifecycle/validity/dependency | canonical/edge predicates before rank/candidate limit | if any indexed vector row cannot be proven eligible natively, decline the entire vector arm with the existing `SoftFallback(Vector)` before KNN | predicates before seed/frontier/result caps | existing `ReadView` path |
| arbitrary JSON/range/set/existence | typed whole-request refusal | typed whole-request refusal | typed whole-request refusal | unchanged existing grammar where supported |

The compiler emits parameterized SQL only. Attribute names are validated on the
reader transaction's projection-registry snapshot and encoded with the existing
safe vec0-column mapping. The body-, edge-, and property-FTS statements place all
accepted eligibility predicates in `WHERE` before `ORDER BY`, FTS rank streaming,
or `LIMIT`. No accepted term calls `text_hit_passes_filter` after a branch cap.

Vector metadata remains pre-KNN. Canonical lifecycle, validity, and dependency
state is not duplicated into vec0 in this release. Before KNN, a bounded
`NOT EXISTS` eligibility guard determines whether the vector partition contains
any row that could violate the requested `ReadView`/dependency fence. If it
does, FathomDB declines the whole vector arm and reports `SoftFallback(Vector)`;
it never retrieves a truncated vector set and filters it afterward.

The search graph arm emits nodes, so `SearchFilter` constrains target nodes—not
transport edges. An edge-FTS seed is admitted before the ten-seed limit only
when at least one endpoint resolves to an eligible target node; an entity seed
must itself be eligible before that limit. Root resolution applies eligibility
before queue admission, and every target node is checked before frontier,
visited, or result-cap accounting. Transport edges remain constrained by their
own canonical validity, lifecycle, and dependency fences; a node `kind` filter
is never incorrectly compared with an edge relation kind. An ineligible target
is neither traversed through nor counted. A `source_type=edge_fact` filter makes
the node-emitting graph arm empty while the direct edge-FTS arm may still emit
eligible edge hits.
Standalone `graph_neighbors` keeps its current `ReadView` contract until Slice
60, which adds the same envelope with direction/kind constraints. Search-expand
consumption under a frozen context must run search plus expansion on one reader
transaction; the legacy unfrozen method remains unchanged.

## Optional frozen-read contract

### Public API and wire

```text
ReadContextV1 {
  schema_version: 1,
  view: ReadView,
  eligibility: SearchFilter
}

FrozenReadContextV1 {
  schema_version: 1,
  effective_valid_at: i64,
  context: ReadContextV1,
  token: string
}

Engine::freeze_read_context(ReadContextV1)
  -> Result<FrozenReadContextV1, EngineError>

Engine::search_frozen(
  query: &str,
  context: &FrozenReadContextV1,
  rerank_depth: usize,
  use_graph_arm: bool,
  alpha: f64,
  pool_n: usize,
  explain: bool,
  limit: usize
) -> Result<SearchResult, EngineError>

Engine::search_expand_frozen(
  query: &str, context: &FrozenReadContextV1, depth: u32, limit: usize
) -> Result<SearchExpandResult, EngineError>
```

Python uses `ReadContextV1`, `FrozenReadContextV1`,
`freeze_read_context`, `search_frozen`, and `search_expand_frozen` with
snake-case fields. TypeScript uses the same discriminants and camel-case
fields/methods. Rust integers remain `i64`/`u64`; nonnegative integers in JSON
or dynamic-language diagnostic details use canonical unsigned decimal strings.
On mint, the returned `context.view.valid_as_of` is always the resolved
`effective_valid_at`; callers echo the complete frozen object on consume. New
request objects reject unknown fields lexicographically after
`schemaVersion`; response readers ignore additive object fields but reject an
unknown schema version. Existing `search*`, `SearchFilter`, `Filter`, and
`ReadView` signatures and serialization remain unchanged.

The two frozen methods are the complete Slice-35 consumer set. Slice 50 and
Slice 60 may add evidence/standalone-graph consumers without changing the token.
No operation/profile selector or projection list is caller-controlled.

### Token payload and persistence

Schema step 31 adds content-free singleton state:

```text
database_id: 32 lowercase hex characters encoding 16 random bytes
read_context_key: 64 lowercase hex characters encoding 32 random bytes
read_visibility_generation: SQLite INTEGER in 0..=i64::MAX, exposed as u64
```

The generation lives in
`_fathomdb_read_visibility_state(singleton, generation)`, not the TEXT open-state
map. Three named AFTER triggers (insert/update/delete) are created on each of
these real authoritative tables:

```text
canonical_nodes, canonical_edges,
_fathomdb_artifact_revisions, _fathomdb_source_versions,
_fathomdb_source_links, _fathomdb_source_dependencies,
_fathomdb_dependency_closures,
_fathomdb_projection_registry, canonical_attributes,
_fathomdb_projection_state, _fathomdb_projection_terminal,
_fathomdb_vector_kinds, _fathomdb_vector_rows
```

Each trigger performs one atomic checked increment and raises
`read visibility generation exhausted` instead of exceeding `i64::MAX` or
wrapping. SQLite cannot attach triggers to FTS5 or vec0 virtual tables. Their
write sites must therefore mutate, in the same transaction, an owning real row:
canonical node/edge for synchronous FTS publication, projection terminal/state
for asynchronous vector publication, and projection state/terminal for rebuild.
Tests enumerate every production virtual-table mutation site and prove this
coupling. Opening compares the exact named trigger set and normalized SQL digest
with the step-31 manifest; a missing or altered trigger is storage corruption.
The Engine caches the last observed generation and rejects an in-process
regression on mint/consume point reads.

Slice 40 and later migrations that add serving-authority tables must add those
tables to this trigger manifest in the same migration. A database created by
file copy retains identity and token validity only until its bound state
diverges; a separately created database fails `database_mismatch`.

The opaque token is
`fdbfr1.<lower-hex-canonical-payload>.<lower-hex-hmac-sha256>`. HMAC-SHA256 is
implemented over the ASCII domain `fathomdb.frozen-read.v1\0` plus the canonical
payload using the stored 32-byte key. The payload is a length-prefixed binary
encoding of:

```text
schema version; database id; effective validity instant;
canonical write boundary; read visibility generation; dependency generation;
projection cursor; projection registry digest; projection state/terminal digest;
SHA-256 digest of the canonical echoed ReadContextV1
```

The token contains no query, filter value, owner/scope identifier, or stored
content. The complete token must be at most 1 KiB. The separately echoed context
is caller-supplied data; mint accepts at most 64 exact attribute terms and a
64-KiB canonical context encoding. Consume canonicalizes the echoed context,
compares its digest with the authenticated payload, and rejects any change.
Because projection bindings are digests, there is no 64-generation overflow
case in Slice 35. Slice 40 may map a binding to generation identity only when it
proves a one-to-one match; otherwise the old token drifts typed.

The canonical codec is `fathomdb.read-context.v1`: `u32` and `u64` use
fixed-width big-endian bytes; `i64` uses fixed-width big-endian two's-complement
bytes; booleans are one byte (`00`/`01`); optionals are one presence byte
followed by the value; UTF-8 strings are an unsigned big-endian `u32` byte
length followed by exact bytes; sequences are an unsigned big-endian `u32`
count followed by elements. The context order is schema version, four
`ReadView` fields, four `SearchFilter` optionals, then attributes sorted by
exact UTF-8 name and value. The context digest domain is
`fathomdb.read-context.v1\0`.

Projection-registry rows hash in `name ASC` order and encode, in order, exact
database values for `name`, `roles`, `fts_tokenizer`, `vector_embedder`,
`vector_declared`, and `source`, using the same scalar/optional codec; domain
`fathomdb.projection-registry-binding.v1\0`. Projection-state rows hash in
`kind ASC` order with `kind`, `last_enqueued_cursor`, and `updated_at`; terminal
rows follow in `write_cursor ASC` order with `write_cursor` and `state`; domain
`fathomdb.projection-serving-binding.v1\0`. SQL NULL is option tag `00`, never
an empty string. The normative fixture
`tests/fixtures/slice35_frozen_context_v1.json` pins payload bytes and all three
digests across Rust, Python, and TypeScript.

The local key prevents accidental or remote-caller forgery; it is not a claim
against an actor able to read or rewrite the database file. MAC comparison is
constant-time. Malformed hex, length, noncanonical payload, unsupported version,
or MAC mismatch returns `FrozenReadError` without exposing which MAC byte failed.

### Mint, consume, and race semantics

Mint validates/canonicalizes eligibility, begins one deferred reader
transaction, pins its SQLite snapshot with a canonical-table read, resolves
`valid_as_of=None` once, reads every boundary/digest from that snapshot, then
signs after the transaction commits. It never holds a SQLite transaction open.

Consume follows this order:

1. parse/version/bounds check and authenticate the token;
2. compare its database identity;
3. begin and pin one reader transaction;
4. validate every stored binding, including current erasure/lifecycle state, on
   that snapshot;
5. execute every requested arm and expansion inside that same transaction;
6. return results from that linearized snapshot.

Any write committed before step 3 is visible to validation and causes drift.
A write committed after step 3 linearizes after this read. FathomDB does not
claim that an already-running local read can be retroactively revoked, matching
SQLite snapshot isolation. Search and expansion share the transaction on the
new frozen path; legacy `search_expand` behavior is unchanged.

Exact failure precedence is: malformed/unsupported token; authentication;
database mismatch; echoed eligibility/read-view validation; bound-state drift
or unavailable state; ordinary query validation/backend errors. Failures are:

- `FrozenReadError { reason, field_path }` with closed reasons
  `unsupported_schema_version`, `token_malformed`, `token_too_large`,
  `token_authentication_failed`, `database_mismatch`, `context_invalid`,
  `state_unavailable`, and `state_drifted`;
- existing `InvalidFilter { reason }` for an unsupported eligibility request.

Bindings map frozen failures to `FDB_FROZEN_READ` / `FrozenReadError`; field
paths use RFC 6901 camel-case wire names. Errors never include the token, key,
query, source bytes, or eligibility values.

## Migration, compatibility, and lifecycle

Step 31 is additive shape/state only and performs no canonical or projection
backfill. Its trigger set is migration-pinned and includes every table that can
alter read visibility. Opening validates database ID/key/generation shape.
Existing databases receive random identity/key once during migration. New and
legacy ordinary calls do not mint, parse, hash, allocate, or validate a context.

Current lifecycle, erasure, dependency closure, and projection repair remain
authoritative. A frozen token never resurrects a superseded, inactive, erased,
or now-ineligible artifact. Since this release retains no historical versions
of mutable serving state, any relevant drift refuses the whole operation rather
than approximating the old result.

## TDD and verification matrix

RED tests are committed before implementation and use real SQLite databases:

| Test target | Required proof |
|---|---|
| `slice35_eligibility_contract` | repeated attribute conjunctions; 64-attribute/64-KiB bounds; exact empty/absent semantics; representable JSON rejection; prohibited range/set/existence forms absent from the schema; injection strings; property-based canonical ordering/digest stability. |
| `slice35_eligibility_pretruncation` | eligible row at cap+1 for body FTS, edge FTS, property FTS, vector KNN, graph seed, and graph frontier; query-plan pins require registry/EAV/real-column predicates before ranking/limit; whole-vector fallback for nonnative lifecycle state. |
| `slice35_frozen_read` | mint/consume, restart, token size/content absence, echoed-context tamper, token tamper, noncanonical encoding, database mismatch, append, supersession, delete/reactivate, validity boundary, dependency generation, projection configuration/rebuild, and copied-database divergence. |
| `slice35_frozen_read_races` | rendezvous before snapshot pin and after validation for lifecycle, erasure, dependency closure, and projection changes; frozen expansion remains one snapshot. |
| binding parity suites | Rust/Python/TypeScript golden token metadata, methods, typed errors, unknown fields/version, and legacy surface compatibility. |

The focused Rust gate is:

```bash
cargo test -p fathomdb-engine --features test-hooks \
  --test slice35_eligibility_contract \
  --test slice35_eligibility_pretruncation \
  --test slice35_frozen_read --test slice35_frozen_read_races \
  --test slice35_filter_grammar --test slice40_filter_unification \
  --test pr_g10_filtered_knn --test slice15e_prekn_filterable
```

Source binding gates are `cargo test -p fathomdb-py`, `cargo test -p
fathomdb-napi`, and `npm test --workspace fathomdb --
slice35-frozen-read`. They verify wrapper/wire behavior but are not installed-
artifact evidence. The later isolated package smoke invokes a standalone
Slice-35 assertion script with `env -u PYTHONPATH -u VIRTUAL_ENV
<fresh-venv>/bin/python` and the offline-installed npm tarballs; no checkout
package path may shadow either artifact.
Repository gates run `./scripts/agent-verify.sh --tier=fast`, `--tier=heavy`,
then `--tier=all`. The applicable combined feature gate is `cargo test -p
fathomdb-engine --features operator,test-hooks,default-embedder,default-reranker`;
a monolithic Cargo `--all-features` invocation is invalid because CUDA and
Metal features are mutually exclusive. The isolated local-package proof builds
the wheel with `maturin build --release --out <wheel-dir> --features
pyo3/extension-module,default-embedder` from `src/python`, runs `npm run build
--workspace fathomdb`, and then runs `scripts/release/smoke/
smoke-local-native-artifacts.sh <wheel-dir> src/ts src/ts/npm/linux-x64-gnu
linux-x64-gnu`. A Slice-35 assertion is added to that installed-wheel and
offline npm pack/install smoke before GREEN. The Windows route is the release
workflow's unchanged Rust/Python/Node matrix and is recorded unavailable rather
than simulated when no Windows executor exists. The CUDA route is
`CUDA_VISIBLE_DEVICES=0 FATHOMDB_EMBED_DEVICE=cuda:0 cargo test -p
fathomdb-engine --features embed-cuda,test-hooks --test
slice35_eligibility_pretruncation` on an RTX 3090, never the display K620.

Legacy `filter=None` vector SQL remains byte-identical. Legacy empty-filter
result bytes and ordering remain identical. Rather than promise zero internal
allocations, the performance acceptance compares the Slice-30 parent and the
Slice-35 candidate with five fresh-database repetitions each, using the 10k
SCALE-02 input pack, 100 warm-ups, and 1,000 steady queries per repetition. The
runner/config are `experiments.scale_02_slice35` and
`experiments/configs/scale-02/slice35-legacy-path.v1.json`; the command is
`PYTHONPATH=. python -m experiments.scale_02_slice35 run <config>
<external-root>`. The baseline source commit is
`b2bfb1f318f58041144acb2356a6a4c9624068b9`; the input/measurement precedent is
`experiments/runs/scale-02-a0-10000-20260822T2239Z-e993dd61/record.json`; and
query-order/bootstrap seeds are `0x5CA1E025350001` and `0x5CA1E02535B007`.
The new parent and candidate receipts are written under the external artifact
root and referenced from `status.md`. The 95% repetition-bootstrap **upper
bound** for both p50 and p95 relative regression must be at most 3%. Empty
frozen-disabled calls execute the exact legacy dispatch. Frozen mint and
validation latency are reported separately and are advisory in this release.
