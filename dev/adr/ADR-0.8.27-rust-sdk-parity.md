---
title: ADR-0.8.27-rust-sdk-parity
date: 2026-10-06
target_release: 0.8.27
desc: Add a dedicated fathomdb-sdk Rust crate as the third governed SDK with the Python/TypeScript operation surface
blast_radius: new fathomdb-sdk crate; governed-operation parity map and checker; Rust/Python/TypeScript interface docs; SDK parity position; release crate lists
status: accepted
---

# ADR-0.8.27 — Dedicated Rust SDK parity

Accepted by repository-owner direction on 2026-10-06 ("a dedicated Rust SDK
with the same surface as the Python and TypeScript SDKs"), conditioned on the
reviewed Slice 132 design.

## Context

`ADR-0.8.0-supersede-five-verb-surface-cap.md` Q5 (BIND-RUST) bound the
`fathomdb` Rust facade into the governed-surface contract through its own
37-type positive allowlist. That facade is a different consumer contract:
it re-exports `fathomdb_engine::Engine` wholesale, so every inherent core
method is reachable. That includes the Rust-only overloads, custom-provider
open, and debug-build test seams. The operation-parity checker
(`scripts/check-sdk-surface-parity.py`) covers only Python and TypeScript.
`docs/positions/sdk-parity.md` therefore declares Rust parity out of scope.

Rust application users want the same application surface as the other two
SDKs: the same namespaces, the same operations, and the same option defaults
and error categories.

## Decision

1. **New crate.** Add a dedicated crate, `fathomdb-sdk` (`fathomdb_sdk`).
   - **Wrapping.** It wraps a private core engine and exports a closed list.
   - **Surface.** The root `Engine` and the `read`, `graph`, and `admin`
     namespaces carry the 44 live canonical operations. The root also has
     standalone `rerank` and `embed_batch_cls`.
   - **Shared types.** The shared DTO names are re-exported.
   - **Errors.** An `Error` with `ErrorKind` mirrors the shared 41-class error
     taxonomy plus the base.
2. **Third binding in the parity map.** `src/conformance/governed-operation-parity.json`
   gains a `rust` endpoint per operation. The checker validates the observed
   SDK source surface against it, failing closed on missing, extra, or renamed
   members. The HITL-signed allowlist and its pin are unchanged, so
   `embed_batch_cls` stays outside the signed map, as it is for Python and
   TypeScript.
3. **Allowed translations.** The only allowed differences are the
   translation rules in `dev/interfaces/rust-sdk.md`:
   - `snake_case` names;
   - synchronous calls returning `Result`;
   - option structs implementing `Default` in place of keyword or optional
     arguments;
   - typed core request/result structs;
   - `ErrorKind` in place of exception classes;
   - the TypeScript resolution where Python and TypeScript disagree;
   - the binding NUL guard applied at the SDK boundary.
4. **Excluded from the SDK:** custom embedder injection, the
   operator/recovery seam, raw SQL, test hooks, and the Rust-only search
   overloads.
5. **BIND-RUST is amended, not superseded.** The `fathomdb` crate keeps its
   allowlist, its operator feature, and its role as the lower-level engine
   facade. Its README directs application users to `fathomdb-sdk`. Whether
   `fathomdb`, `fathomdb-engine`, and the embedder plugin crates keep their
   external positioning remains the open
   `slice-132-external-provider-disposition` decision.

## Consequences

- A new Rust SDK operation must be added in all three SDKs and in the map, or
  the checker fails.
- The crate is publishable and joins the Axis W version and crates.io publish
  lists. Publication remains separately authorized.
- `docs/positions/sdk-parity.md` now claims parity for Rust through
  `fathomdb-sdk`, not through `fathomdb`.
