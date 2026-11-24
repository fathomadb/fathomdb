# Durable Agent Memory (`dev/agent-memory/`)

Canonical, repo-tracked operational memory shared across **all** coding-agent
harnesses (Antigravity/Gemini, Claude Code, Codex, and others). Every harness
reads this directory before planning or executing work and records durable,
product-generic operational corrections here so lessons survive harness switches
and context compaction.

> **Project-boundary rule:** Keep every file in `dev/agent-memory/` strictly
> product-generic to FathomDB — never record external consumer-project specifics
> or secrets here.

## Index

- [execution-and-review-traps.md](execution-and-review-traps.md) — worktree
  isolation, single-writer discipline, adversarial review fallback, watchdog/ETA
  cadence, shared-host benchmarking, and HITL prompt clarity.
- [release-and-packaging-traps.md](release-and-packaging-traps.md) — `v*` tag
  publish semantics, CUDA route helper, shallow-fetch prevention, Trusted
  Publisher preflight, cross-OS CI gating, and owned-venv wheel rules.
- [hardware-and-host-notes.md](hardware-and-host-notes.md) — CUDA 12.6 / driver
  immutability, temporary GPU runner lifecycle, and Jetson Orin runner hygiene.
