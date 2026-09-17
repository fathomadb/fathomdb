# Long-file investigation

This directory examines whether very large source files reduce the efficiency
or effectiveness of LLM coding agents and applies the evidence to the
`fathomdb-engine/src/lib.rs` found in the `release/0.8.25` worktree.

## Reading order

1. [`report.md`](report.md) — conclusions and recommendations.
2. [`agent-compatible-code-organization.md`](agent-compatible-code-organization.md)
   — source-layout, shared-instruction, and harness requirements for Codex and
   Claude.
3. [`repository-observations.md`](repository-observations.md) — reproducible
   local measurements and their snapshot limits.
4. [`source-ledger.md`](source-ledger.md) — quantitative evidence, source by
   source, with limitations.
5. [`plan.md`](plan.md) — scope, safeguards, and method.

The release worktree was read only. All investigation artifacts were written
to the local `main` checkout.

## Verification

- The repository Markdown configuration reports zero errors across these six
  files.
- Plan-status, design-status, findings, and generated release-state checks
  pass.
- Offline link validation reports zero errors; 22 external links were excluded
  from that offline pass and were checked while reviewing their sources.
- The unscoped repository Markdown gate is currently blocked by 830 errors in
  an unrelated, untracked benchmark virtual environment under
  `data/performance-benchmarking/`. No files in that environment were changed.
- The repository-wide plan-anchor scanner is also blocked by existing
  `dev/plans/plan-0.8.20.md` citations that now match multiple paths elsewhere
  in the checkout. None of its reported locations is in this investigation.
