---
title: Compact text block — GPT-6 Astra medium design review
date: 2026-10-21
status: ACTIVE
desc: Read-only GPT-6 Astra (medium) review of compact-text-block.md at 21cfa817e. Verdict PASS-WITH-FIXES; the findings are not yet applied to the design.
---

# Compact text block — GPT-6 Astra medium review

Reviewed `feat/agent-access-block1` at `21cfa817e` with GPT-6 Astra, medium. No files changed.

- **P1 — Bare references break the proposed grammar.** [compact-text-block.md:62](dev/design/agent-access/compact-text-block.md#L62), [tools.rs:354](src/rust/crates/fathomdb-mcp/src/tools.rs#L354), [mcp.md:78](dev/interfaces/mcp.md#L78).
  References can contain spaces, newlines and ` | `; the parser rejects only empty logical IDs and NUL. Bare references can corrupt hit boundaries and defeat verbatim recovery. Retain JSON quoting for references and writer-controlled metadata, with delimiter/whitespace tests.

- **P2 — Removing structured output loses additional operational fields.** [compact-text-block.md:90](dev/design/agent-access/compact-text-block.md#L90), [tools.rs:482](src/rust/crates/fathomdb-mcp/src/tools.rs#L482), [output.rs:474](src/rust/crates/fathomdb-mcp/src/output.rs#L474).
  Unchanged `orient` text omits projection status, affected kinds and the dense-disable reason. Search also loses `continuation_exact: false` in embedded mode. Inventory preserved/omitted fields per tool and retain these operational signals.

- **P2 — Budget accounting needs a final-render rule.** [compact-text-block.md:66](dev/design/agent-access/compact-text-block.md#L66), [compact-text-block.md:95](dev/design/agent-access/compact-text-block.md#L95), [tools.rs:329](src/rust/crates/fathomdb-mcp/src/tools.rs#L329).
  Text now contains its own token count, but today `finish` updates only structured JSON after rendering. Specify convergent final-text accounting or an explicit exclusion convention; test digit-width boundaries, hints and cut flags against the final byte count.

- **P2 — Fetch’s replacement contract is incomplete.** [compact-text-block.md:86](dev/design/agent-access/compact-text-block.md#L86), [tools.rs:1049](src/rust/crates/fathomdb-mcp/src/tools.rs#L1049).
  “One block per record” does not specify mixed successes/errors, cut flags or multiline framing. A verbatim body can resemble another record header. Define unambiguous boundaries and preserve one outcome per requested reference, including errors and hints.

- **P2 — The acceptance comparison conflates rendering and pagination changes.** [compact-text-block.md:133](dev/design/agent-access/compact-text-block.md#L133), [compact_proxy.py:99](dev/experiments/agent-remeasure/compact_proxy.py#L99), [REPORT.md:71](dev/experiments/agent-remeasure/REPORT.md#L71).
  The proxy reformats already-budgeted pages; native text budgeting can return more hits. Therefore native tokens per call need not reproduce the 33% reduction. Compare identical-page rendering separately from end-to-end page counts, cost and accuracy.

- **P3 — Distinguish protocol requirements from client behavior.** [compact-text-block.md:38](dev/design/agent-access/compact-text-block.md#L38), [compact-text-block.md:52](dev/design/agent-access/compact-text-block.md#L52).
  Omitting both fields is valid; declaring `outputSchema` requires conforming structured results. However, preserving today’s human-readable structured-mode text departs from the serialized-JSON text recommendation. Claude Code’s display preference is not a protocol guarantee. [MCP 2025-06-18](https://modelcontextprotocol.io/specification/2025-06-18/server/tools#structured-content)

- **P3 — The stale snippet description is in the experiment prompt.** [compact-text-block.md:103](dev/design/agent-access/compact-text-block.md#L103), [edge.rs:39](src/rust/crates/fathomdb-mcp/src/edge.rs#L39), [REPORT.md:93](dev/experiments/agent-remeasure/REPORT.md#L93).
  Server instructions already describe match-centred snippets. Correct the experiment prompt and the design’s stated target.

Recommendations for section 6:

1. **Default:** Choose **`text` after these fixes**, with explicit release notes and documented `structured` configuration. The measured agent benefit supports it, but does not establish compatibility across clients.
2. **Server-wide setting:** **Yes.** Separately configured stdio processes suffice. Avoid client-name heuristics; introduce explicit session negotiation only for a demonstrated deployment need.
3. **Dropped fields:** **Keep `epoch` and `validity_instant`; drop unavailable `state` and `verdict` placeholders.** Preserve `continuation_exact` and readiness/degradation information. Epoch identifies an open generation; it does not prove freshness.
4. **Fetch:** **Include it after completing its text contract.** Preserve bodies, cut flags, per-item errors and hints. Its gain is unmeasured; eliminating duplicated bodies may substantially increase usable body capacity.

**Verdict: PASS-WITH-FIXES.** The approach is viable; resolve framing, information preservation and accounting before implementation approval.
