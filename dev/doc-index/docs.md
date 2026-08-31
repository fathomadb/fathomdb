# DOC-INDEX detail — `docs/`

> Long-form per-doc notes for this area of the doc tree. The thin map at
> `dev/DOC-INDEX.md` links here; **every path listed below also has its own row in
> `dev/DOC-INDEX.md`** (path + a ≤120-char purpose clause) — this file carries the
> full slice-history / decision-record prose that DOC-INDEX.md compresses away.
> Update this file (not DOC-INDEX.md) when you want to add narrative detail; update
> BOTH files when you add/rename/materially change a doc (DOC-INDEX.md row +
> this file's row, same closing commit — mirrors DOC-INDEX.md's own rule).

## `docs/` — user-facing documentation (mkdocs, `nav` in `mkdocs.yml`)

| Path | Purpose | Owning slice / AC | Last-touched |
|------|---------|-------------------|--------------|
| `docs/index.md` | Docs home for the published 0.8.23 surface | X2 (nav) | 2026-08-23 |
| `docs/getting-started/index.md` | Getting-started overview for 0.8.23 | — | 2026-08-23 |
| `docs/getting-started/quickstart.md` | Quickstart (five-operation contract) | 5/30 (new surface examples) | 2026-05-17 |
| `docs/install/python.md` | Python install for published 0.8.23 | — | 2026-08-23 |
| `docs/install/typescript.md` | TypeScript install for published 0.8.23 | — | 2026-08-23 |
| `docs/install/rust.md` | Rust install for published 0.8.23 | — | 2026-08-23 |
| `docs/reference/index.md` | API-reference overview for the published 0.8.23 surface | 0.8.24 Slice 7 | 2026-08-23 |
| `docs/reference/python-api.md` | Python API reference including published governed `read.*`, projection configuration/derived readiness, and `read.projection_status` | Slices 21/22 published by 0.8.23 | 2026-08-23 |
| `docs/reference/typescript-api.md` | TypeScript API reference including published governed `read.*`, projection configuration/derived readiness, and `read.projectionStatus` | Slices 21/22 published by 0.8.23 | 2026-08-23 |
| `docs/reference/cli.md` | CLI reference (recovery verbs CLI-only) | 34; 0.8.24 Slice 7 link correction | 2026-08-23 |
| `docs/reference/errors.md` | Error reference (taxonomy) | 0.8.24 Slice 7 link correction | 2026-08-23 |
| `docs/reference/config.md` | Config reference | 0.8.24 Slice 7 link correction | 2026-08-23 |
| `docs/concepts/index.md` | Concepts overview for the published 0.8.23 surface | 0.8.24 Slice 7 | 2026-08-23 |
| `docs/embedder.md` | Default embedder, published 0.8.23 framing | 0.8.24 Slice 7 | 2026-08-23 |
| `docs/compatibility/index.md` | Compatibility matrix for published 0.8.23 | 0.8.24 Slice 7 | 2026-08-23 |
| `docs/operations/index.md` | Operations guide for the published 0.8.23 surface | 0.8.24 Slice 7 | 2026-08-23 |
| `docs/operations/worktree-consolidation.md` | Worktree consolidation protocol overview | 0.8.24 Slice 7 link correction | 2026-08-23 |
| `docs/operations/erasure.md` | **Erasure boundary** — what `erase_source`/`purge` guarantee and, explicitly, what they do NOT (copies, SQLite free pages absent `VACUUM`, CoW/snapshotted filesystems, backups); the retention-exempt erasure-audit record; the non-PII `source_id` rule with its CORRECTED rationale (design defect D-A: v4 §3.6's "audit retains `source_id` permanently" premise was verified FALSE and is SUPERSEDED IN PART — the real basis is an unswept audit row, now made durable) + the `derive_logical_id` case-folding note (D-C); `doctor orphan-provenance` usage | 0.8.20 Slice 5d (R-20-E4/E8, design §4 item 12) | 2026-07-19 |
| `docs/guides/index.md` | Guides hub for the published 0.8.23 surface | 0.8.24 Slice 7 | 2026-08-23 |
| `docs/guides/structured-search-hits.md` | Structured `SearchHit` usage guide (id/kind/body/score/branch; Py + TS) | 5 (G1); 10 (score → RRF) | 2026-06-03 |
| `docs/guides/retrieve-by-id.md` | Retrieve-by-id guide — `read.get`/`read.get_many` point lookup by `logical_id` (active-only) + `read.collection`/`read.mutations` paginated op-store read-back (mandatory limit + after-id cursor); Py + TS | 30 (G2/G3) | 2026-06-04 |
| `docs/guides/hybrid-search-filtering.md` | Hybrid-search guide — RRF plus Python/TS `SearchFilter` metadata and declared-projection attribute filters | 0.8.22 documentation correctness | 2026-08-08 |
| `docs/positions/index.md` | Positions hub | — | 2026-05-01 |
| `docs/positions/sdk-parity.md` | Position: SDK parity (guarantee carried forward by 25) | 25 | 2026-05-01 |
| `docs/positions/recovery-surface.md` | Position: recovery surface (denylist, CLI-only) | preserved by 25/30 | 2026-05-01 |
| `docs/positions/tokenizer-policy.md` | Position: tokenizer policy | 5 (FTS5 default upgrade) | 2026-05-01 |
| `docs/positions/embedder-identity.md` | Position: embedder identity | — | 2026-05-01 |
| `docs/release-notes/0.6.0.md` | 0.6.0 historical release notes; current-link banner names 0.8.23 | 0.8.24 Slice 7 | 2026-08-23 |
| `docs/release-notes/0.6.1.md` | 0.6.1 historical release notes; current-link banner names 0.8.23 | 0.8.24 Slice 7 | 2026-08-23 |
| `docs/release-notes/0.8.0.md` | 0.8.0 historical release notes; current-link banner names 0.8.23 | 0.8.24 Slice 7 | 2026-08-23 |
| `dev/releases/0.8.0.md` | **0.8.0 internal release record** — engineering companion to the user notes: behavior-compat events, AC-075/076 gate restructure (◆ B-1), CI split, verification posture; every claim traces to a measured Slice-40 result or signed ADR | 40/GA-2 | 2026-06-08 |
