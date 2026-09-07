---
title: Slice 60 FIX-5 RED oracle correction 12
---

# Slice 60 FIX-5 RED oracle correction 12

The independent Cycle 5 audit separates two fail-closed trace outcomes. A
corrupted derived relation must retain an authenticated canonical root while
returning no relation; a corrupted canonical root self-link makes the root
unavailable. The prior single zero-edge assertion incorrectly treated those
distinct contracts as interchangeable. The corrected fixture records
`root_only` for the derived-node and derived-link owner corruptions, and
`trace_unavailable` for the canonical self-link owner corruption.

Cycle 5 requires a live current-RSS or executable retention witness, but does
not derive a portable byte ceiling. The previous 4 MiB comparison is therefore
removed. The subprocess samples remain live diagnostics and the replacement
RED requires deterministic retained edge-batch, frontier, visited, and
candidate counters. Their bounds come from the READY structural limits and the
two exact-work arms must have identical counter tuples despite 100,000
unrelated rows.

Sources: `implementation-review-cycle5.md` findings 1 and 3; Slice 60 design
deterministic traversal and exact-work limits.

| Path | Old SHA-256 | New SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_fix5_provenance.rs` | `6cd34728a1bc245e16169856efcea7f992f7ce2b314634bdab2098330a24e661` | `feae276b2ce9d33bc7f448f65282d68317560c332d2771d91cf0b01969ea33dd` |
| `src/rust/crates/fathomdb-engine/tests/slice60_fix5_rss.rs` | `89a82ccd1bb721ae285840975305294389c59c0217a95a0272d9909771f7cd20` | `8f8dd892fbfd2ea65733236ad0e7a8f7a319386cffecbfdd1dacdb4d714dad71` |
| `dev/fixtures/slice60-fix5-provenance-v1.json` | `4e7b8f5f7f52bc84fd1061f606bbb5d9bb8ed956abd126959e91998d0b6c8973` | `cd4d63a6dcf344d69777ab89a479672184ce5ec2c21ffacbc879fee7171e7e91` |
| `dev/fixtures/slice60-fix5-rss-v1.json` | `1532993ae4a5186b6415b6592c51bf3990dc5e1995a9402aca54749d956e19be` | `bf3cb931a4fe5f5c830d9dba79e4e6322b04672a38690977694fb3671345792a` |

The intended RED was run before implementing the counters: provenance passes
against the already-restored fail-closed product checks, while the RSS oracle
does not compile because the four retention-counter fields do not yet exist.
