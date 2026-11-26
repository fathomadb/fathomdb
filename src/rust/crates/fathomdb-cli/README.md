# fathomdb-cli

The `fathomdb` operator and diagnostics binary for **FathomDB**, a local-first embedded retrieval
engine built on SQLite.

This is a **tool, not a library API**. It exists so an operator holding a FathomDB database file can
inspect it, verify it and repair it without writing code. It is deliberately *not* a query client:
there is no `search`, no `get`, no `list`. Application access goes through the
Rust application `fathomdb-sdk` crate or the Python / Node.js SDKs. The
separate `fathomdb` crate provides the lower-level facade.

## Status: pre-1.0, beta

The 0.8.x line is under active development. Verbs, flags and JSON shapes can change between minor
releases.

## Install

```bash
cargo install fathomdb-cli --version '=0.8.31' --locked
fathomdb --version
```

That installs a binary named `fathomdb` (published release `0.8.31`). Starting
in `0.8.31`, `fathomdb-cli` is also published as a lockstep PyPI binary wheel
installable via `pip install 'fathomdb[cli]==0.8.31'` while keeping the base
`fathomdb` SDK wheel free of console scripts.

## Three roots

```text
fathomdb doctor <verb> [--json] [<db-path>]              # diagnostics / export / maintenance
fathomdb recover --accept-data-loss [flags] <db-path>    # lossy repair
fathomdb migrate --backup-dir <dir> [--json] <db-path>   # offline Schema 38 -> 39 (or chained 37 -> 38 -> 39) upgrade
```

`doctor` contains diagnostics, exports and explicit maintenance. Ordinary-open
commands can initialize a missing database or start projection workers;
`recompute-mean` mutates the stored mean and sign-bit vectors. `recover` admits
lossy repair, which is why every invocation requires an
explicit `--accept-data-loss` acknowledgement: without it the command refuses with
`E_RECOVER_REQUIRES_ACCEPT_DATA_LOSS` and exits 70. There is no way to run a lossy workflow by
accident. `migrate` performs an explicit offline Schema `38 -> 39` (and chained
`37 -> 38 -> 39`) upgrade on the `0.8.32` target line (published `0.8.31`
upgraded `37 -> 38`) for a quiescent committed Schema 37 or Schema 38 store with
no pending erasures: it writes a SHA-256-verified backup under `--backup-dir`,
executes the upgrade, and runs post-upgrade `check-integrity --full`.

Most `doctor` verbs take a database path. `gpu`, `reranker-gpu`, `platform`,
`cuda-allocator`, `cuda-libraries` and `warm-cache` are database-free;
`warm-cache` can populate the shared model cache.

### `doctor` verbs

| Verb | What it does |
| --- | --- |
| `check-integrity` | Structural integrity check; reports typed findings with locators |
| `data-plane-integrity` | Bounded dependency/projection/edge checks (including `edge_endpoint_integrity`) through an immutable, exact-schema operator boundary |
| `safe-export` | Materialise a safe export of the database |
| `verify-embedder` | Confirm the embedder identity recorded in the database matches this build |
| `trace` | Trace the resolution chain for a source reference |
| `dump-schema` | Dump the canonical schema definition |
| `dump-row-counts` | Per-table row counts |
| `dump-profile` | Stored embedder identity, dimension and registered vector kinds |
| `dump-mutations` | Page through the operational-mutation log for one collection |
| `orphan-provenance` | Per-`source_id` census; reports rows reachable by **no** erasure verb |
| `gpu`, `reranker-gpu` | Inspect embedding or cross-encoder device policy |
| `platform` | Classify the host without opening a database |
| `cuda-allocator`, `cuda-libraries` | Inspect CUDA allocation and library selection |
| `warm-cache` | Pre-fetch and verify the pinned default-embedder weights (needs the `default-embedder` feature) |
| `recompute-mean` | Re-derive and re-pin the corpus mean, re-quantizing every vector in one transaction |

Before `doctor data-plane-integrity`, stop every FathomDB process using the
store. The command requires an existing product `.lock`, refuses a non-empty
`-wal` or `-journal`, and requires the database schema to exactly match the
binary. It uses immutable read-only SQLite and never migrates, repairs, starts
workers, or rewrites product files. A persistent `-shm` is allowed and left
unchanged.

### `recover` flags

`--truncate-wal`, `--rebuild-vec0`, `--rebuild-projections`, `--complete-erasures`, `--excise-source <id>`, and
`--excise-collection <c> --excise-record-key <k>` (which must be given together).

## Machine-readable output

`--json` is available on every verb and is the normative output contract; the human-readable form is
for humans.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Clean |
| 64 | `recover` completed, but only because lossy action was explicitly accepted |
| 65 | Diagnostics found actionable non-clean state |
| 66 | An artifact-producing verb failed to materialise its export |
| 70 | Unrecoverable command failure |
| 71 | Lock held, or an equivalent precondition blocked the command |

These classes are stable and safe to branch on in scripts.

## Cargo features

| Feature | Effect |
| --- | --- |
| `default-embedder` | Required for `doctor warm-cache` to actually fetch weights. Without it the subcommand is present but cannot load the embedder. |

## License

MIT. See the `LICENSE` file shipped in this crate.

Source, issues and full documentation: <https://github.com/fathomadb/fathomdb>
