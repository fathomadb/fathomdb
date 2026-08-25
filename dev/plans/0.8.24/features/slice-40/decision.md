---
title: 0.8.24 Slice 40 — decision record
status: PROPOSED
target_release: 0.8.24
---

# Slice 40 decision record

## Open owner decisions

| ID | Decision required | Current ruling | Consequence |
| --- | --- | --- | --- |
| P24-09 | Windows CUDA SDK surface: Python, npm, or both; define unsupported routes. | **Open.** Recommended minimum: Python-only via a first-party PEP 503 exact-local-version route, without selecting a final identity here. | Blocks artifact identity, public documentation, loader design, and TDD implementation. |
| P24-10 | Remote Windows CUDA executor and trust/transfer boundary. | **Open.** Current self-hosted CUDA inventory is Linux-only; the local Windows VM and hosted Windows jobs are CPU-only validation, not CUDA proof. | Blocks build, GPU smoke, and any claim of supported Windows CUDA. |

## Accepted constraints

- No local Windows compile is required or authorized.
- Do not use hosted Windows CPU as CUDA proof.
- Preserve existing CPU Python artifacts and the accepted npm package
  `fathomdb-native-win32-x64-msvc`.
- Keep GPU builder credentials separate from hosted registry publication.
- Do not change canonical needs, requirements, or acceptance before P24-09 and
  P24-10 are decided.

## Decision capture template

When the owner decides, record: selected surface; exact artifact identity and
user selection; unsupported/forced behavior; executor selector and observed
facts; builder-to-publisher transfer; owner-approved registry/environment; and
the exact candidate-installed smoke command/evidence location. Then amend the
design and promote only the relevant draft contracts.
