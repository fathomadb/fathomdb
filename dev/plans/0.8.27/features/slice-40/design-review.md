---
title: FathomDB 0.8.27 Slice 40 - design review
status: PASS
reviewed_on: 2026-09-22
---

# Slice 40 design review

Independent read-only review checked the Slice 40 plan and design against the
current engine, master plan, prework testing contract, feature manifest,
surface comparator, and prior-slice allocations.

The first review required correction of three issues: authoritative records
still claimed every default build excluded every test seam; per-batch compare
steps omitted the clean committed candidate capture they require; and the hook
test scope was not reproducibly bounded. Re-review additionally required a
resolved full 40-character candidate SHA rather than literal `HEAD`, plus the
focused Slice-72 rendezvous behavior test rather than compile-only evidence.

All findings are closed. The final allocation is coherent and bounded:

- `errors.rs` owns shared open/operation error carriers and corruption types,
  while runtime configuration and `EmbedderChoice` remain for Slice 90;
- `identity.rs` owns primitive newtypes, hash/validation, and derivation while
  request DTOs and operations remain for Slice 50;
- `temporal.rs` owns the one temporal SQL/clock/normalization authority while
  domain call sites remain for their later slices; and
- `test_hooks.rs` owns cfg-preserved hook/rendezvous/pause mechanisms while
  semantic call sites remain in their domains.

Every batch remains within the advisory 300-1,200 moved-line range, preserves
root re-exports and private crate topology, and avoids absorbing later domain
operations. Verdict: **PASS**, with no open design findings.
