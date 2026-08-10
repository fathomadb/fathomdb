# 0.8.23 Slice 0 plan review — 2026-08-10

## Reviewer

Independent adversarial reviewer subagent, used after the external Codex review
could not complete its environment-wide instruction-file scan.

## Initial verdict

**BLOCK** — five findings: three P1 and two P2.

1. A repository-scoped self-hosted runner is not secured by labels or workflow
   prose; require an organization runner group restricted to `release.yml`.
2. Registry-installed smoke occurs after immutable publish; it cannot block
   upload and must instead gate promotion, GitHub Release, and completion.
3. CPU fallback required a genuinely driverless proof, not only a smoke on the
   GPU host.
4. The CUDA witness needed attribution to the smoke PID because other processes
   use the two RTX 3090 devices.
5. The actual CUDA toolkit, driver, and maturin/manylinux build environment
   needed an explicit preflight witness.

## First resolution

All five findings are incorporated into the uncommitted follow-up to
`2b71b811`. Re-review is required before promotion.

## Re-review verdict

**BLOCK** — two additional P1 findings:

1. A restriction for `release.yml@refs/heads/main` excludes the normal
   tag-triggered publication, which evaluates the workflow at `refs/tags/v*`.
2. `coreyt/fathomdb` is personally owned, while organization runner groups can
   grant access only to repositories in their organization. The plan must halt
   until a repository-placement or Enterprise-runner solution is selected.

## Second resolution

The follow-up now requires a preflight-proven publication-ref policy and records
repository placement as an explicit HITL configuration gate. Re-review remains
required after this correction.
