---
title: 0.8.24 Slice 40 — decision record
status: PROPOSED
target_release: 0.8.24
---

# Slice 40 decision record

## Open owner decisions

| ID | Decision required | Current ruling | Consequence |
| --- | --- | --- | --- |
| P24-09 | Windows CUDA SDK surface: Python, npm, or both; define unsupported routes. | **Open.** Recommended minimum: Python-only via a first-party PEP 503 full exact-local-version pin and selected index/mapping, as an exclusive CPU/CUDA replacement or upgrade in one environment. The final identity is not selected here. | Blocks artifact identity, public documentation, loader design, and TDD implementation. |
| P24-10 | Remote Windows CUDA builder and trust/transfer boundary. | **Open.** It may be an owner-operated external builder not registered with Actions, or an Actions builder restricted to a dedicated selected-repository/selected-workflow runner group. Current self-hosted CUDA inventory is Linux-only; the local Windows VM and hosted Windows jobs are CPU-only validation, not CUDA proof. | Blocks build, GPU smoke, and any claim of supported Windows CUDA. |

## Accepted constraints

- No local Windows compile is required or authorized.
- Do not use hosted Windows CPU as CUDA proof.
- Preserve existing CPU Python artifacts and the accepted npm package
  `fathomdb-native-win32-x64-msvc`.
- For a Python local-version route, forbid CPU/CUDA co-install, floating
  extra-index selection, and PyPI upload. Preserve the CPU PyPI artifact
  separately.
- Keep GPU builder credentials separate from hosted registry publication.
- Do not treat Actions labels as access control. An Actions builder is limited
  to a dedicated selected-repository/selected-workflow runner group, never PR,
  fork, or `pull_request_target`, and receives no secret, OIDC, or publish
  credential.
- Do not change canonical needs, requirements, or acceptance before P24-09 and
  P24-10 are decided.

## Decision capture template

When the owner decides, record: selected surface; exact artifact identity and
user selection; unsupported/forced behavior; trusted-builder form and observed
facts; builder-to-publisher transfer; owner-approved registry/environment; and
the exact candidate-installed smoke command/evidence location. The smoke must
retain selected UUID == GPU UUID with active computation PID, plus model,
driver, toolkit, candidate SHA, and artifact digest. Then amend the design and
promote only the relevant draft contracts.
