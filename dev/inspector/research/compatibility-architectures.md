---
title: Inspector compatibility architecture research
date: 2026-10-29
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Source-grounded examples of adapters, capability policy, configuration, plugins and composition
---

# Inspector compatibility architecture research

> Current-policy note (2026-10-29): the owner subsequently permitted Engine
> startup when required. Earlier immutable/no-startup/preservation assumptions
> below are historical. Current [requirements](../requirements.md#engine-startup-allowance-2026-10-29)
> distinguish Inspector read operations from qualified Engine lifecycle effects.

## Finding and research boundary

Established tools combine executable adapters, ordinary runtime conditions,
capability records, and versioned artifacts. Configuration selects behavior
and supplies parameters; plugins package behavior; dependency injection
composes collaborators. None removes the need to encode semantic differences.
The useful architectural question is where that knowledge lives and how it
is qualified, rather than whether all `if` statements can be eliminated.

At the owner's request, a read-only research subagent explored official web
documentation and implementation sources on 2026-10-29. This report records
that exploration, with parent synthesis and independent-review clarifications,
against Inspector planning baseline
`9085c3655`. No example was executed. Tags are reproducible source samples,
not assertions about the latest release. The CDP implementation links use
moving `main` and are weaker evidence for later reproduction.

The following are observations of other products; the Inspector implications
are inferences. The independent [Astra assessment](../reviews/compatibility-architecture-review.md)
contains recommendations and qualifications. Neither document amends
requirements, approves a detailed design, or claims implemented compatibility.

## Examples at a glance

| Example / source sample | Real compatibility mechanism | Configuration, plugins and composition | Main limitation for Inspector |
| --- | --- | --- | --- |
| DBeaver 25.2.0 | JDBC transport plus database-specific providers; version branches and catalog probes populate support flags. | XML registers executable provider classes; driver settings select libraries/properties; dialect objects can be supplied. | Generic JDBC compatibility does not guarantee database-specific metadata correctness. |
| pgAdmin 4, REL-9_9 | Detected server version selects SQL through a version-aware template loader. | Packaged SQL templates, a hardcoded version-directory map, and Python controller behavior. | Older-template fallback is a maintenance assumption; ordinary connection setup has effects. |
| SQLAlchemy 2.0.44 | A selected dialect/driver encapsulates SQL/reflection behavior and derives flags during initialization. | Registry and external entry points select code; Engine construction receives pool and dialect. | A dialect seam does not make its normal initialization safe for immutable inspection. |
| Datasette 0.65.1 | SQLite runtime checks and separate attached-file introspection. | Owner supplied to Database; Pluggy entry points/hooks; metadata settings shape presentation. | Connection hooks/native extensions and inventory work need separate safety and resource review. |
| VS Code LSP client 9.0.1 / LSP 3.17 | Initialize capabilities govern provider registration and optional operations. | Host-extension API version constraint is separate; features receive client/middleware collaborators. | Negotiation requires a running server; it does not establish storage-file admission. |
| Chrome DevTools / CDP, moving main | Generated protocol commands; target-type capability masks gate registered model construction. | Internal model registry and constructor composition. | Local capability masks are not arbitrary remote protocol-version negotiation. |

### DBeaver — specialized code behind driver/provider declarations

Driver settings identify JDBC classes, libraries and connection properties.
The PostgreSQL extension declares concrete server-provider classes for
PostgreSQL, Redshift and CockroachDB. The XML registers implementations; it
does not declaratively implement their metadata semantics.
[Driver documentation](https://dbeaver.com/docs/dbeaver/Database-drivers/),
[tagged extension declarations](https://github.com/dbeaver/dbeaver/blob/25.2.0/plugins/org.jkiss.dbeaver.ext.postgresql/plugin.xml#L20).

`PostgreDataSource` combines `SELECT version()`, version thresholds such as
`isServerVersionAtLeast(12, 0)`, and catalog probes for `pg_enum` and
`pg_class.reltype`. Probe failures can become a false support flag; that is a
warning against conflating unsupported with inaccessible or failed. The
constructor accepts a supplied `SQLDialect`, demonstrating internal
composition alongside plugin discovery.
[Version/probe path](https://github.com/dbeaver/dbeaver/blob/25.2.0/plugins/org.jkiss.dbeaver.ext.postgresql/src/org/jkiss/dbeaver/ext/postgresql/model/PostgreDataSource.java#L412),
[constructor](https://github.com/dbeaver/dbeaver/blob/25.2.0/plugins/org.jkiss.dbeaver.ext.postgresql/src/org/jkiss/dbeaver/ext/postgresql/model/PostgreDataSource.java#L99).

DBeaver documents incorrect metadata from noncompliant JDBC drivers. A common
interface reduces integration work but does not establish correct results.
[JDBC tracing guidance](https://dbeaver.com/docs/dbeaver/JDBC-Tracing/).

### pgAdmin — versioned SQL artifacts with explicit fallback

`DatabaseView` includes the connection manager's version in its template path.
`VersionedTemplateLoader` walks a descending hardcoded map from `18_plus` to
older directories and `default`, skipping versions above the server and
falling back when a template is absent. The map, selection algorithm and SQL
together encode compatibility; moving a branch into a template directory does
not remove its maintenance obligation.
[Controller path](https://github.com/pgadmin-org/pgadmin4/blob/REL-9_9/web/pgadmin/browser/server_groups/servers/databases/__init__.py#L237),
[loader/map](https://github.com/pgadmin-org/pgadmin4/blob/REL-9_9/web/pgadmin/utils/versioned_template_loader.py#L11).

This is a relevant browser/Python-service product shape, documented in a newer
architecture overview than the sampled source.
[Official architecture overview, 9.17](https://www.pgadmin.org/docs/pgadmin4/9.17/code_overview.html).

In the sampled psycopg connection `_initialize`, session `SET`/`set_config`
operations precede version inspection; configured post-connection SQL runs
later. Session settings do not themselves prove persisted database writes.
They establish that ordinary initialization is broader than passive metadata
observation. Inspector must audit its own admission path rather than copy this
lifecycle.
[Connection initialization](https://github.com/pgadmin-org/pgadmin4/blob/REL-9_9/web/pgadmin/utils/driver/psycopg3/connection.py#L447).

### SQLAlchemy — dialect policy, registries, and ordinary DI

The documented `dialect+driver` URL selects implementations. A dialect registry
combines built-in imports with `PluginLoader("sqlalchemy.dialects", ...)`;
built-ins also contain explicit special cases such as MariaDB. Registry
selection is not a substitute for code implementing reflection and SQL.
[URL contract](https://docs.sqlalchemy.org/en/20/core/engines.html#database-urls),
[tagged registry](https://github.com/sqlalchemy/sqlalchemy/blob/rel_2_0_44/lib/sqlalchemy/dialects/__init__.py#L22).

`PGDialect` reads/parses the server version and derives capabilities such as
identity-column and JSONB-subscripting support during initialization. These
are version-based policy inferences, not successful feature-execution probes.
[Capability initialization](https://github.com/sqlalchemy/sqlalchemy/blob/rel_2_0_44/lib/sqlalchemy/dialects/postgresql/base.py#L3009),
[version reader](https://github.com/sqlalchemy/sqlalchemy/blob/rel_2_0_44/lib/sqlalchemy/dialects/postgresql/base.py#L3254).

`create_engine()` builds the dialect and passes it with a pool into Engine.
This is dependency injection through normal composition, separate from
external plugin discovery. The initialization contract allows configuration
from server properties on a connection with full capabilities; it is not an
immutable-inspection contract.
[Composition](https://github.com/sqlalchemy/sqlalchemy/blob/rel_2_0_44/lib/sqlalchemy/engine/create.py#L659),
[Dialect initialization contract](https://docs.sqlalchemy.org/en/20/core/internals.html#sqlalchemy.engine.Dialect.initialize).

### Datasette — runtime-library facts versus attached-file facts

SQLite utility code reads `sqlite_version()` on a separate in-memory connection
and derives support for introspection APIs. Attached-file inspection separately
reads `sqlite_master` for FTS declarations and chooses `table_xinfo` versus
`table_info` based on runtime support. An existing FTS structure and a library
able to query it are different facts: a direct analogue for Inspector's
storage-versus-query-backend distinction.
[Runtime capability functions](https://github.com/simonw/datasette/blob/0.65.1/datasette/utils/sqlite.py#L20),
[FTS/column discovery](https://github.com/simonw/datasette/blob/0.65.1/datasette/utils/__init__.py#L558).

`Database` receives its owning Datasette instance. `connect()` distinguishes
mutable-file read-only access (`mode=ro`) from immutable access (`immutable=1`);
`table_counts()` applies per-table time limits and permits cached counts.
This demonstrates connection composition and per-table bounded count work.
It does not establish an aggregate inventory deadline or bounded table
enumeration; Inspector needs its own total work/time/response bounds.
[Database implementation](https://github.com/simonw/datasette/blob/0.65.1/datasette/database.py).
SQLite's `immutable` option disables locking/change detection and requires the
file actually to remain unchanged; it does not freeze a live file.
[SQLite URI contract](https://www.sqlite.org/uri.html#uriimmutable).

Pluggy loads entry-point plugins and built-ins. `_prepare_connection` can load
configured native SQLite extensions and invoke `prepare_connection` hooks.
That lifecycle introduces arbitrary executable behavior and must not be
adopted as Inspector's read-only admission blueprint.
[Plugin discovery](https://github.com/simonw/datasette/blob/0.65.1/datasette/plugins.py#L20),
[connection preparation](https://github.com/simonw/datasette/blob/0.65.1/datasette/app.py#L555),
[hook contract](https://docs.datasette.io/en/0.65.1/plugin_hooks.html#prepare-connection-conn-database-datasette).

### VS Code/LSP — capabilities and extension compatibility are separate

LSP groups features into specification versions but exchanges client/server
capabilities during initialization. In the sampled client,
`CompletionItemFeature.initialize()` registers a provider only when the server
advertises `completionProvider`; `resolveProvider` controls optional resolution.
Features receive client and middleware collaborators.
[LSP 3.17 specification](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/lsp/3.17/specification.md),
[client implementation](https://github.com/microsoft/vscode-languageserver-node/blob/release/client/9.0.1/client/src/common/completion.ts#L90).

The installed extension separately declares an `engines.vscode` host-version
constraint and a client-library dependency range. The official guide also
records activation-declaration differences for older VS Code hosts. A plugin
system creates another API/version boundary; capability negotiation does not
replace it.
[Host compatibility](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#visual-studio-code-compatibility),
[extension implementation guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide).

### DevTools/CDP — generated contracts plus local capability policy

CDP publishes protocol descriptions and browser/protocol metadata; its
documentation warns that tip-of-tree has no guaranteed backward compatibility.
A protocol version label alone is not an arbitrary frontend/backend support
promise.
[Official CDP documentation](https://chromedevtools.github.io/devtools-protocol/index.html).

Frontend code registers generated commands. `Target` uses a target-type
`switch` to assign capability masks and constructs registered models only
when their required mask is satisfied. `SDKModel.register()` stores classes
and lifecycle metadata: an internal registry, not evidence of a user-loadable
compatibility plugin interface.
[Protocol registration](https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/core/protocol_client/InspectorBackend.ts#L67),
[type/mask and construction](https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/core/sdk/Target.ts#L38),
[model registry](https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/core/sdk/SDKModel.ts#L52).

Model construction can activate instrumentation: `DOMModel` calls `DOM.enable`
unless its target is suspended. Discovery and activation must be separate
review concerns, even in an inspector.
[DOM model constructor](https://github.com/ChromeDevTools/devtools-frontend/blob/main/front_end/core/sdk/DOMModel.ts#L1557).

## Popularity evidence and architectural quality

DBeaver's homepage claims over ten million unique users: vendor self-report,
with no independently inspected measurement method. pgAdmin describes itself
as the most popular PostgreSQL administration platform: positioning, not a
comparative measurement. The 2025 Stack Overflow survey identifies VS Code
as respondents' most-used IDE, with the survey's sampling limitations; this
does not measure usage of this particular LSP path.
[DBeaver claim](https://dbeaver.com/), [pgAdmin positioning](https://www.pgadmin.org/),
[primary developer survey](https://survey.stackoverflow.co/2025/technology#2-development-environments-and-tools).

These provide widely used product examples with explicitly qualified evidence.
No adoption rank is established for SQLAlchemy, Datasette, or CDP. Stars,
downloads and plugin counts would be proxies, not verified active users.

For the requested clean-architecture examples, the strongest focused patterns
are SQLAlchemy's dialect boundary, LSP's capability exchange, and Datasette's
separate runtime/file observations. This is an architectural judgment based on
visible boundaries, not a claim that those products are pure or universally
better. Evaluate:

- Whether compatibility knowledge is localized behind a contract.
- Whether observations, policy-inferred support, and successful execution differ.
- Whether admission can avoid activating features or arbitrary setup hooks.
- Whether adapters can be independently checked against real fixtures.
- Whether extensions have an explicit API/version/lifecycle policy.
- Whether unknown, unsupported, inaccessible, and temporarily unavailable differ.
- Whether discovery is bounded and fallback preserves semantics.

The inspected paths demonstrate ordinary runtime conditions and registries;
this was not a codebase-wide audit for absence of `ifdef`/conditional builds.
Compile-time features remain appropriate for platform or optional dependency
selection. They do not, by themselves, identify an attached file's format or
prove a request succeeded.

## Provisional Inspector implications

These are research recommendations for detailed-design reconciliation. The
[existing comparison](../design/compatibility-0.8.28-0.8.29.md) is the local
source basis: the examined .28/.29 snapshots have identical schema-34 storage
and important read mechanisms, while transient query evidence differs. Source
similarity does not qualify runtime compatibility or identify the producer of
an arbitrary schema-34 file.

1. Keep a small compiled adapter boundary for admitted storage semantics and
   query/result normalization. Start with one storage implementation where
   qualification supports it. The initial policy remains the selected build's
   current compiled schema and qualified structural contract; declaring schema
   34 is insufficient by itself. Do not create parallel engines per producer
   version or a generic multi-database framework.
2. Inject reader/query collaborators through ordinary constructors and explicit
   ownership. User-loadable plugins add API, lifecycle and distribution work;
   no current Inspector need establishes that cost as worthwhile.
3. Keep build support, attachment observations, session prerequisites, and
   request outcomes separate. Prerequisites may be unchecked or change as
   explicit requests initialize a runtime. A vector table does not prove an
   available model; producer .29 does not prove snippet support in the serving
   query library. Missing degradation metadata is absent evidence, not proof
   that every retrieval arm ran successfully.
4. Localize necessary version/layout conditions inside adapters. Expose typed
   capability/evidence information to views rather than producer-version
   branches in document, lexical, vector and graph components. Configuration
   may choose supported implementations or budgets; it must not declare an
   unrecognized layout safe or manufacture evidence.
5. Make minimum file, lock, sidecar and supported-layout checks part of bounded
   admission under the applicable quiescence/file safeguards. Broader structure
   discovery follows accepted admission, with aggregate work/time/response
   bounds. Admission is not a full integrity scan, exhaustive field inference,
   model load or feature trial. Do not call ordinary `Engine::open`, run
   arbitrary plugin initialization, load untrusted extensions, download models,
   build projections, or perform
   embeddings/searches just to populate a support label. Record execution
   outcomes when explicit operations actually run.
6. Qualify real files made by pinned producer implementations, and separately
   qualify the selected query/backend build. Preserve typed identity,
   lifecycle/eligibility, provenance, canonical/operational contents, actual
   lexical routes, vector metrics and directed graph behavior. Check database,
   sidecar and cache preservation on success, failure, cancellation and refusal.
7. Unknown capability or failed execution must not be reported as unsupported
   format. Conversely, capabilities must not demote mandatory Inspector views
   into optional omissions. Missing required safe access is a concrete blocker,
   not permission for Engine/MCP/SDK/schema work or weaker acceptance.
8. Package one UI/service pair. Check their protocol revision or equivalent
   build pairing and launch/session identity before database attachment
   requests. Reject incompatible clients, including stale browser tabs; never
   use capabilities to compensate for incompatible payload meanings. Discard
   stale session references/continuations after detach or reattachment. General
   cross-version protocol negotiation is unnecessary for this distribution.

### Concrete vector example for detailed design

This example separates facts; it is not a proposed wire schema or a giant
mutually exclusive status enum. Use the smallest typed contracts needed by the
slice and operation.

| Axis | Example fact | What it does not establish |
| --- | --- | --- |
| Build support | This pinned backend implements a qualified vector operation with a named metric. | Model readiness or a successful request. |
| Attached storage | A declaration/representation exists; individual origin materialization may be known or unknown. | Complete materialization or usable runtime. |
| Session prerequisites | Compatible cached assets are unchecked, usable, missing, or failed initialization. | That a vector request ran. Attachment must not load models just to show ready. |
| Request outcome/evidence | A specific operation ran, skipped, failed, was cancelled or exhausted its budget, with actual route/population/metric evidence. | Health or completeness of unobserved operations/arms. |

An expected missing-runtime fixture can validly report unavailability. Failure
to satisfy a required positive vector fixture remains a release blocker.
Capability checks govern operations within an admitted storage/session and a
compatible service protocol; they cannot establish those preconditions.

No source changes, plugin system, older-schema support or new support promise
is introduced by this research. The [Astra review](../reviews/compatibility-architecture-review.md)
assesses how far these implications should guide v0.1.

## Follow-up evolution assessment

The [0.9/1.0 stress test](compatibility-evolution-stress-test.md) challenges these
recommendations against structural and semantic change, and distinguishes
reusable boundaries from forward compatibility of a released binary.
