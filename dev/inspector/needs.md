---
title: Inspector user needs
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
desc: Research and user outcomes for FathomDB-native inspection
status: ACTIVE
---

# Inspector user needs

Current delivery scope is [owner-adopted 0.2.0](plans/0.2.0/scope.md), amended by
[INS-020-LOCAL-RELEASE](plans/0.2.0/release-decisions.md). Functional INS/V01/X,
T-01–T-05 and High investigation goals remain required. The five-person study
was removed by owner decision; no human sessions occurred or count as passed.
Slices20–60/70 technical qualification and75 UX remediation are complete;
80 completed readiness and[90 released locally](plans/0.2.0/slice-90/status.md)
to ignored `dist/inspector/0.2.0/`. Historical
source/design evidence retains its actual version and execution scope.

## Purpose and status

Humans report difficulty forming a mental model of FathomDB: what data is
stored, what structures hold it, and how data stored once can be retrieved in
different ways. The central need is to build an accurate mental model of the
database, grounded in its actual contents.

This document preserves the user-needs research and subsequent requirements
from the design discussion. The feedback above is owner-provided evidence.
The broader needs inventory is informed by research into existing tools;
the owner has placed the assessed Need and Should actions in Inspector v0.2.0
scope. Usability targets and unresolved detailed contracts remain proposals.
Inspector versions are separate from FathomDB database release numbering.
The current record-browser port targets `dev/0.8.29`; the original preparation
material records its earlier `dev/0.8.30` source context.

The Inspector is a tool for inspecting a database, not a tutor or explanatory
agent. Users build their mental model through documentation and operation of
the tool. Its responsibility is to present structures, contents, identities,
relationships, and retrieval observations faithfully and consistently. It has
no requirement to teach concepts, generate explanatory narratives, or lead a
tutorial. Ordinary labels, values, state indicators, and documentation links
remain appropriate.

One record may have multiple stored representations. Those representations
must remain visibly associated with their originating record. The Inspector
operates on FathomDB structures rather than treating SQLite tables as the
product model. Tabular browsing alone does not satisfy the need: text search,
vectors, and graphs require interaction and presentation appropriate to their
structure. A derived
canonical record, by contrast, is a distinct record with its own identity.

## Research basis

Product documentation establishes existing capabilities, not validated demand
from FathomDB users. User needs inferred from those capabilities require
validation with representative users.

| Tool | Observed capability and primary source | Inferred user needs |
| --- | --- | --- |
| DB Browser for SQLite | Spreadsheet-style browsing, sorting, searching, SQL results, and export: [official overview](https://sqlitebrowser.org/index.html). | Discover contents, inspect records, find data, communicate findings. |
| DBeaver | [Structure diagrams](https://dbeaver.com/docs/dbeaver/ER-Diagrams/), [data editor](https://dbeaver.com/docs/dbeaver/Data-Editor/), and [bounded result navigation](https://dbeaver.com/docs/dbeaver/Navigation/). | Understand structures, follow relationships, navigate responsively. |
| Datasette | [Faceted exploration](https://docs.datasette.io/en/latest/facets.html) and [CSV export with result-scope distinctions](https://docs.datasette.io/en/stable/csv_export.html). | Find subsets and understand what displayed or exported results cover. |
| MongoDB Compass | [Schema analysis](https://www.mongodb.com/docs/compass/schema/), [query history](https://www.mongodb.com/docs/compass/query/recent/), and [query explanations](https://www.mongodb.com/docs/compass/query-plan/). | Understand observed shapes, recognize anomalies, resume work, explain behavior. |
| Neo4j Bloom | Focused graph exploration, record detail, and export: [visual overview](https://neo4j.com/docs/bloom-user-guide/current/bloom-visual-tour/bloom-overview/). | Follow actual record relationships while retaining context. |
| Qdrant Web UI | Collection discovery, point inspection, API console, and interactive learning: [official overview](https://qdrant.tech/documentation/web-ui/). | Discover vector-oriented contents and learn through concrete examples. |

Schema relationships and actual record-to-record relationships serve different
inspection tasks. A storage-table browser alone would not expose all the
relationships between provenance, lifecycle, projections, and retrieval.

Safety also requires enforcement rather than hidden controls. Compass notes
that its read-only UI option does not remove underlying write privileges:
[Compass editions](https://www.mongodb.com/docs/compass/editions/).

## Full user-needs inventory

The initial research classified UN-01 through UN-10 as the essential inspection
foundation and UN-11 through UN-18 as important investigation needs. The focused
requirements below reprioritize the work around the owner's mental-model
feedback. The inventory is retained in full so that lower-priority needs are
not silently lost.

| ID | User need and desired outcome |
| --- | --- |
| UN-01 | Identify the database I am inspecting: know its location, compatibility, availability and current attachment state so I investigate the intended store. |
| UN-02 | Understand what the database contains: discover collections, record kinds, fields, relationships, and their meanings without knowing internal storage names. |
| UN-03 | Get a useful overview before opening individual records: understand volume and distribution and whether the contents broadly match expectations. |
| UN-04 | Browse contents without writing code: reach representative records through understandable navigation and recognizable stored labels, while retaining exact identity. |
| UN-05 | Find a particular record or subset: locate data using identity, text, field values, source, or other relevant constraints. |
| UN-06 | Inspect a record completely and faithfully: read full content and nested values, distinguish missing/null/empty values, and copy exact identities and values. |
| UN-07 | Follow relationships without losing context: move between records, edges, related records, and evidence while retaining my place. |
| UN-08 | Understand what displayed results represent: know filters, ordering, scope, sampling, pagination, truncation, and whether counts are exact. |
| UN-09 | Explore safely: inspect without issuing edits; understand when Engine startup/background work may change stored state. |
| UN-10 | Investigate large databases responsively: reach useful information without loading everything and understand incomplete or interrupted work. |
| UN-11 | Recognize unexpected data: discover missing fields, inconsistent shapes, surprising values, duplicates, or broken relationships. |
| UN-12 | Understand how a record came to exist: trace its source, evidence, dependencies, and derivation to assess trust. |
| UN-13 | Understand a record's lifecycle: distinguish current, superseded, and otherwise ineligible information and understand visibility or exclusion. |
| UN-14 | Distinguish stored facts from derived representations: understand authoritative content versus generated text, vector, filter, and ranking representations. |
| UN-15 | Determine whether data is ready for retrieval: distinguish stored, pending, failed, and available to a relevant retrieval path. |
| UN-16 | Explain a retrieval result or missing result: relate the query, constraints, retrieval paths, ranking evidence, and exclusions to an observed outcome. |
| UN-17 | Resume and reproduce an investigation: return to a record, filter, query, or exploration context without reconstructing every step. |
| UN-18 | Communicate a finding outside the tool: extract relevant data, context, and explanation so another person or tool can inspect and reproduce it. |

UN-12 through UN-16 are especially FathomDB-specific hypotheses inferred from
its responsibilities. Query-plan explanations are an analogy, but alone do
not explain hybrid retrieval relevance or lifecycle eligibility.

## Related specifications

See [requirements](requirements.md), [acceptance criteria](acceptance.md),
the [v0.2.0 plan](plans/0.2.0/plan.md), the [legacy v0.1 plan](plans/plan-0.1.md),
and [roadmap](ROADMAP.md).

## Slice 15 coverage baseline (2026-10-30)

UN-01–UN-08 and UN-12–UN-16 have mandatory acceptance in INS-01–INS-09;
UN-09/10 apply through INS-X01–X11. The narrower coverage and deferrals in
[requirements](requirements.md#critical-and-high-level-requirements) remain
binding: incidental navigation/copying does not claim persistent workspaces,
general export or anomaly detection. The owner additionally authorized narrow
export of still-retained erasure-pending content under `--show-erased`; this
adds bounded incidental UN-18 coverage without general sharing tooling. Critical and High both ship. The
[verification plan](plans/0.1/verification-plan.md) assigns every criterion,
including High tracing/outcome cases, to an implementation owner.
