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
| `docs/index.md` | Docs home for the 0.8.27 three-SDK surface | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/getting-started/index.md` | Getting-started overview for the 0.8.27 SDKs | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/getting-started/quickstart.md` | 0.8.27 Python write/read quickstart and SDK navigation | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/install/python.md` | Python 0.8.27 native and digest-qualified Tegra install routes | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/install/typescript.md` | TypeScript 0.8.27 native and verified Tegra Pages install routes | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/install/rust.md` | Rust SDK, lower-level facade and exact-version CLI installation | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/reference/rust-sdk.md` | Rust application SDK shape, options, errors and shared governed operations | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/generated/sdk-operation-map.md` | Source-generated index of 44 Python, TypeScript and Rust governed operations | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/generated/contributor-map.md` | Source-generated Rust, Python, TypeScript and typed developer entry-point map | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/contributing/index.md` | Contributor setup, TDD, scoped/full verification and release-state workflow | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/release-notes/0.8.27.md` | 0.8.27 changes, three SDKs, Jetson delivery and release qualification limits | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/reference/index.md` | API-reference overview for the 0.8.27 surface | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/reference/python-api.md` | Python 0.8.27 API reference, including embedder and search-view options | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/reference/typescript-api.md` | TypeScript 0.8.27 API reference and default-embedder open options | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/reference/rust-api.md` | Lower-level Rust facade reference for 0.8.27 | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/reference/cli.md` | Operator CLI reference, including integrity and CUDA diagnostics | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/reference/errors.md` | Error reference (taxonomy) | 0.8.24 Slice 7 link correction | 2026-08-23 |
| `docs/reference/config.md` | Config reference | 0.8.24 Slice 7 link correction | 2026-08-23 |
| `docs/concepts/index.md` | Concepts overview for the current retrieval and graph surface | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/embedder.md` | Default embedder, opt-in configuration and runtime policy | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/compatibility/index.md` | 0.8.27 native platform, schema-34 and Tegra verification boundaries | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/operations/index.md` | Operations guide for the current release surface | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/operations/worktree-consolidation.md` | Worktree consolidation protocol overview | 0.8.24 Slice 7 link correction | 2026-08-23 |
| `docs/operations/erasure.md` | **Erasure boundary** — what `erase_source`/`purge` guarantee and, explicitly, what they do NOT (copies, SQLite free pages absent `VACUUM`, CoW/snapshotted filesystems, backups); the retention-exempt erasure-audit record; the non-PII `source_id` rule with its CORRECTED rationale (design defect D-A: v4 §3.6's "audit retains `source_id` permanently" premise was verified FALSE and is SUPERSEDED IN PART — the real basis is an unswept audit row, now made durable) + the `derive_logical_id` case-folding note (D-C); `doctor orphan-provenance` usage | 0.8.20 Slice 5d (R-20-E4/E8, design §4 item 12) | 2026-07-19 |
| `docs/guides/index.md` | Guides hub for the current retrieval surface | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/guides/structured-search-hits.md` | Structured `SearchHit` usage guide (id/kind/body/score/branch; Py + TS) | 5 (G1); 10 (score → RRF) | 2026-06-03 |
| `docs/guides/retrieve-by-id.md` | Retrieve-by-id guide — `read.get`/`read.get_many` point lookup by `logical_id` (active-only) + `read.collection`/`read.mutations` paginated op-store read-back (mandatory limit + after-id cursor); Py + TS | 30 (G2/G3) | 2026-06-04 |
| `docs/guides/hybrid-search-filtering.md` | Hybrid-search guide — RRF plus Python/TS `SearchFilter` metadata and declared-projection attribute filters | 0.8.22 documentation correctness | 2026-08-08 |
| `docs/positions/index.md` | Positions hub | — | 2026-05-01 |
| `docs/positions/sdk-parity.md` | Three-SDK operation, option and language-specific error parity | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/positions/recovery-surface.md` | Position: recovery surface (denylist, CLI-only) | preserved by 25/30 | 2026-05-01 |
| `docs/positions/tokenizer-policy.md` | Position: tokenizer policy | 5 (FTS5 default upgrade) | 2026-05-01 |
| `docs/positions/embedder-identity.md` | Position: embedder identity | — | 2026-05-01 |
| `docs/release-notes/0.6.0.md` | 0.6.0 historical release notes; current-link banner names 0.8.27 | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/release-notes/0.6.1.md` | 0.6.1 historical release notes; current-link banner names 0.8.27 | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/release-notes/0.8.0.md` | 0.8.0 historical release notes; current-link banner names 0.8.27 | 0.8.27 Slice 140 | 2026-10-10 |
| `docs/release-notes/0.8.26.md` | Historical 0.8.26 highlights, schema boundary, published artifacts and install routes | 0.8.27 Slice 10 | 2026-09-21 |
| `dev/releases/0.8.0.md` | **0.8.0 internal release record** — engineering companion to the user notes: behavior-compat events, AC-075/076 gate restructure (◆ B-1), CI split, verification posture; every claim traces to a measured Slice-40 result or signed ADR | 40/GA-2 | 2026-06-08 |
