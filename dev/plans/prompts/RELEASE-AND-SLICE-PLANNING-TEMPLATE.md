# Release Scaffolding, Planning & Slice Execution Template

> **Portable across projects, harnesses, and model families.**
> Core phases below are written using abstract roles and reasoning tiers. Before starting, read **Section 0 (Harness & Model Adaptation Guide)** for your active pairing and ignore the other pairings.

## Release Parameters (Fill In Per Release)

- **Project:** `<project-name>`
- **Development branch:** `dev/<x.y.z>` (cut from `<base-branch>`)
- **Development worktree:** `<dev-worktree-path>`
- **Durable plan directory:** `<plan-dir>` (e.g., `dev/plans/<x.y.z>/`)
- **Active harness + model pairing:** `<Antigravity+Gemini | Claude Code+Anthropic | OpenAI Codex+OpenAI>`

---

## Global Invariants (Read First)

1. **Never pollute `main`.** Build all release work on `dev/<x.y.z>` in a dedicated git worktree.
2. **No implementation during prework (Slices 0–9).** Slices 0–8 are read-only audits and durable proposals only (except creating `dev/<x.y.z>`, its worktree, and the draft plan scaffold in Slice 0). Do not edit product code, upgrade dependencies, or delete files until approved in Slice 9 and executed in Slice 10+.
3. **Write durably; never rely on chat memory.** Every slice records its outputs to disk under `<plan-dir>` (e.g., `<plan-dir>/prework/slice-<N>.md`, `<plan-dir>/slice-<N>/plan.md`, `<plan-dir>/slice-<N>/status.md`).
4. **One writer per checkout.** Read-only research/review subagents may share the checkout and run concurrently. Concurrent file-mutating subagents must each get their own git worktree or be serialized.
5. **Direct delegation, minimal ceremony.** Spawn specialist subagents directly using your harness's native primitives—do **not** route through legacy Steward/Orchestrator wrappers or MCP orchestration tools.
6. **Proportional verification.** When fixing a specific review finding or local bug, run targeted tests covering the blast radius; reserve full-workspace regression suites for comprehensive changes or final slice/release gates.
7. **Verify from ground truth.** Confirm completion from real test output, exit codes, and `git status`/`git diff`—never from narration alone.
8. **Clean up at release end.** Once the release is finalized and merged/published, remove temporary/release worktrees (`git worktree remove` + `git worktree prune`) and delete merged temporary/dev branches.

---

## GPU Availability & Backoff Protocol

If you require GPU resources and find them occupied or lacking sufficient VRAM:

1. **Do not exit or fail the task.** Enter a polling loop.
2. Execute a shell sleep command before re-checking availability. Use the following backoff schedule:
   - Attempt 1: sleep `120 + $((RANDOM % 30))` seconds (2 mins)
   - Attempt 2: sleep `240 + $((RANDOM % 45))` seconds (4 mins)
   - Attempt 3: sleep `480 + $((RANDOM % 60))` seconds (8 mins)
   - Attempt 4+: sleep `900 + $((RANDOM % 60))` seconds (15 mins max cap)
3. **Re-verify before claiming:** Immediately after waking up and confirming free VRAM, launch your GPU process right away so another agent does not claim the memory window.
4. **Timeout:** Only abort and report failure to the user if the GPU remains unavailable after 6 hours (24 attempts).

---

## Section 0 · Harness & Model Adaptation Guide

Resolve every abstract tier and primitive in Sections 1–2 using **only** your active row below.

### Abstract Tiers & Roles

| Abstract Symbol | Purpose |
| --- | --- |
| `TIER_DEEP` | Critical plan review, design review, adversarial code review, high-stakes verification |
| `TIER_ANALYSIS_PLAN` | Architecture review, product doc CRUD, testing adequacy, triage synthesis, complex draft slice planning |
| `TIER_STANDARD` | Slice implementation, dependency/pin sweeps, independent verification |
| `TIER_FAST` | Bounded read-only sweeps (cruft inventory, stale docs, CI/transcript forensics) |
| `ROLE_AUDITOR` | Read-only research/audit subagent (no repo edits outside its own durable output if delegated) |
| `ROLE_REVIEWER` | Read-only adversarial critic returning `APPROVE` or `REQUEST_CHANGES` with P1/P2 findings |
| `ROLE_PLANNER` | Drafts complete executable slice plans (planning only; no implementation) |
| `ROLE_IMPLEMENTER` | Executes TDD implementation (RED → GREEN → Refactor) inside the slice workspace |
| `ROLE_VERIFIER` | Independently runs tests/builds/UI checks against real artifacts (no mocks) |

### Pairing 1 · Antigravity + Gemini (`neon`, `hydrogen`, `carbon`)

- **Model tier mapping:**
  - `TIER_DEEP` & `TIER_ANALYSIS_PLAN`: `Model: "pro"` (top Gemini reasoning tier, e.g., `carbon` / `neon`)
  - `TIER_STANDARD`: `Model: "inherit"` (or `neon` / `hydrogen`)
  - `TIER_FAST`: `Model: "flash"` (or `flash_lite` for trivial sweeps, e.g., `hydrogen`)
- **Subagents & parallel fan-out:**
  - Use `invoke_subagent` / `define_subagent` directly (`TypeName: "research"` for `ROLE_AUDITOR`/`ROLE_REVIEWER`, `TypeName: "self"` for `ROLE_IMPLEMENTER`/`ROLE_PLANNER`/`ROLE_VERIFIER`, `TypeName: "browser"` for live UI verification).
  - Use `run_workflow` (or parallel `invoke_subagent` calls) to fan out read-only Slices 1–7 and Phase C draft plans concurrently.
- **Worktree isolation:** Pass `Workspace: "share"` for concurrent writers; keep `Workspace: "inherit"` for read-only reviewers/verifiers.
- **HITL gates:** Use `ask_question` for structured multi-choice/multi-select owner rulings and user-facing **Artifacts** (`RequestFeedback: true`) for triage tables and plan sign-off.
- **GPU backoff adaptation:** Follow the exact backoff durations and 24-attempt / 6-hour cap above, using Antigravity's native `schedule` one-shot timer rather than blocking a shell tool call with `sleep`.
- **Web research & transcripts:** Use `search_web`; inspect prior transcripts at `<appDataDir>/brain/<conversation-id>/.system_generated/logs/transcript.jsonl` (read `transcript_full.jsonl` only for `truncated_fields`).
- **Gemini execution discipline:** Keep invariants front-loaded and output schemas rigid (exact tables/verdict tokens). Enforce explicit timeouts on process-spawning/UI/trace commands and keep verification scoped to blast radius so high-reasoning tiers don't over-run full regressions on minor fixes.

### Pairing 2 · Claude Code + Anthropic (`opus`, `sonnet`, `fable`)

- **Model tier mapping:**
  - `TIER_DEEP`: `fable` (critical plan review, design review, adversarial code review, high-stakes review gates)
  - `TIER_ANALYSIS_PLAN` & `ROLE_IMPLEMENTER` (`TIER_STANDARD` implementation): `opus` (architecture review, product documentation CRUD, testing adequacy, triage synthesis, complex draft slice planning, and **slice implementation**)
  - `TIER_FAST` & non-implementation `TIER_STANDARD` sweeps/verification: `sonnet` (dependency/pin sweeps, bounded read-only sweeps, independent verification)
- **Subagents & parallel fan-out:**
  - Use native `Agent` delegation (enumerate `.claude/agents/` first; never guess/invent agent type names).
  - Fan out independent read-only Slices 1–7 and Phase C drafts in a single parallel tool-call turn.
- **Worktree isolation:** Use `isolation: "worktree"` (or explicit `git worktree add`) for file-mutating subagents; keep reviewers read-only.
- **HITL gates:** Use `AskUserQuestion` (or a concise scored markdown table + explicit decision prompt) and stop for owner confirmation at Slice 9 gates.
- **Web research & transcripts:** Use `WebSearch` / `WebFetch`; inspect session transcripts and persistent memory under `~/.claude/projects/<repo-path-slug>/`.
- **Anthropic execution discipline:** Keep the main thread as coordinator and delegate implementation/reviews cleanly. Enforce strict `FIX-n` caps and require read-only reviewers (`fable`) to emit findings rather than silently editing files.

### Pairing 3 · OpenAI Codex + OpenAI (`astra`, `sol`, `terra`)

- **Model tier mapping:**
  - `TIER_DEEP` & `TIER_ANALYSIS_PLAN`: `astra` / `sol` (architecture review, critical plan & design review, adversarial code review, complex slice plan drafting)
  - `TIER_STANDARD`: `sol` / `terra` (slice implementation, dependency/pin sweeps)
  - `TIER_FAST` & `ROLE_VERIFIER`: `terra` (dedicated independent test/verification execution and fast forensics sweeps)
- **Subagents & parallel fan-out:**
  - Spawn `terra`, `sol`, and `astra` subagents directly through Codex's native subagent mechanism—never via MCP orchestration wrappers or Steward/Orchestrator layers.
  - Run read-only Slices 1–7 and Phase C draft plans concurrently where the harness supports parallel subagents.
- **Worktree isolation:** Create explicit `git worktree add` per concurrent writer (or serialize mutations on the dev worktree) and remove temporary worktrees after merging back.
- **HITL gates:** Present concise numbered options and scored triage tables directly in chat/terminal; pause for explicit owner rulings before advancing past Slice 9 gates.
- **Web research & transcripts:** Use native web search when enabled; inspect Codex session/rollout logs alongside git history and CI logs.
- **OpenAI execution discipline:** Keep prompts concise and action-oriented, verify every status claim directly from command exit codes and `git diff`, and bound review loops to the stated `FIX-n` ceiling.

---

## Section 1 · Release Scaffolding & Planning Playbook

### Phase A · Prework (Slices 0–9)

#### Slice 0 · Environment, Infrastructure & Release Scaffold
1. Create branch `dev/<x.y.z>` from `<base-branch>` and its dedicated worktree at `<dev-worktree-path>`; switch to it.
2. Ensure an overall draft release plan exists under `<plan-dir>` (create one if absent).
3. Audit and durably document **environment and infrastructure decisions needed** (take no environment-mutating action yet):
   - **Toolchains & runtimes:** `.venv` / Python version, `npm` / Node / TypeScript, Rust / compiler toolchains.
   - **Platforms & clients:** OS/architecture targets (Linux/Windows/macOS, x86_64/arm64) and required browsers/runtimes (Chrome, Firefox, etc.).
   - **Compute & CI infrastructure:** existence, access, and location of external hosts, VMs, GPUs, or GitHub/remote runners.
4. **Stop gate:** Write `<plan-dir>/prework/slice-0.md`; take no action beyond branch/worktree/plan scaffold.

#### Slices 1–7 · Independent Read-Only Audits (Fan Out in Parallel)
Spawn `ROLE_AUDITOR` subagents for Slices 1–7. Each writes `<plan-dir>/prework/slice-<N>.md` and **takes no mutating action**:

- **Slice 1 · Library & Security Sweep** (`TIER_STANDARD`):
  - Enumerate libraries needing upgrades, Dependabot/advisory alerts, and lockfile drift.
  - Audit every pinned version or fork source: record *why* it is pinned (compatibility, upstream bug, transitive constraint) before proposing any change.
  - Draft an upgrade execution plan.
- **Slice 2 · Repo-Wide Cruft Review** (`TIER_FAST`):
  - Audit program docs, internal engineering docs (`dev/`: requirements, acceptance criteria, architecture, design, code/test notes, intermediate docs), developer docs, and public docs.
  - Enumerate candidates and propose an action for each: `keep` | `deprecate-in-place` | `archive` | `delete`.
- **Slice 3 · Product Documentation CRUD Drafts** (`TIER_ANALYSIS_PLAN`):
  - Review User Needs, Requirements, and Acceptance Criteria.
  - Draft Create/Rename/Update/Delete (CRUD) changes as **drafts** (do not overwrite canonical docs yet) and allocate each draft to a target slice.
- **Slice 4 · Architecture Review & Code Alignment** (`TIER_ANALYSIS_PLAN`):
  - Synthesize Slices 0–3 outputs; draft any needed architecture CRUD changes and allocate them to slices.
  - Perform a high-level code-to-architecture alignment review; propose whether code or architecture should change where drift exists.
- **Slice 5 · Testing & Verification Adequacy** (`TIER_ANALYSIS_PLAN`):
  - Trace coverage: (a) do requirements have acceptance criteria, and (b) do acceptance criteria have tests?
  - Assess broader adequacy against overall product goals and confirm whether critical paths are identified and tested.
- **Slice 6 · Stale Documentation Forensics** (`TIER_FAST`):
  - Review repo docs and available evidence (including session transcripts) for stale content across: (1) end-user docs, (2) developer/consumer docs (libraries, APIs, SDKs), and (3) contributor docs.
- **Slice 7 · CI/CD & Release-Friction Forensics** (`TIER_FAST`):
  - Review transcripts, CI logs, and recent release history for build failures, preflight/bootstrap failures, verification-gate misuse or excessive runtime, packaging/gitleaks/registry failures, and other post-build release delays.
  - Propose pragmatic fixes that remove recurring friction without adding bureaucracy.
- **Slice 8 · Reserved** (optional release-specific prework slot; mark unused if not needed).

#### Slice 9 · Triage, HITL Decision Gate & Slice 10 Plan
1. Collect all proposals from Slices 0–8 into `<plan-dir>/prework/slice-9-triage.md` using this schema:

   | ID | Source Slice | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation (`include` / `postpone` / `reject`) |
   | --- | --- | --- | --- | --- | --- | --- |

2. **HITL Gate 1:** Present the scored triage clearly and concisely using your harness's native HITL mechanism. **STOP and collect owner decisions.**
3. Record all owner decisions durably.
4. Draft the **Slice 10 implementation plan** (`<plan-dir>/slice-10/plan.md`) covering every approved prework item (`TIER_ANALYSIS_PLAN`).
5. **Plan Review Loop (up to 4 `FIX-n` cycles):**
   - Run `ROLE_REVIEWER` (`TIER_DEEP`) against the Slice 10 plan.
   - Address findings and re-submit until `APPROVE` (max 4 cycles; escalate to HITL if blocked).
6. **HITL Gate 2:** Present the reviewed changes interactively, capture owner approval, and update the durable plan.

---

### Phase B · Feature Slice Ladder (Starts at Slice 20)

1. Organize all remaining release scope into feature slices numbered by multiples of 10, **starting at Slice 20** (`20`, `30`, `40`, …; Slice `10` is reserved for Slice 9 prework execution).
2. **Critical Release-Plan Review (max 1 `FIX-n` cycle):**
   - Spawn `ROLE_REVIEWER` (`TIER_DEEP`) to critically review the release plan for gaps, sequencing risks, overbuilding, and simplification opportunities.
   - Check all edits thoroughly before returning for re-review (only **one** `FIX-n` cycle is budgeted).
3. After `APPROVE`, commit all uncommitted planning work on `dev/<x.y.z>`.

---

### Phase C · Draft Per-Slice Plans (Planning Only — No Implementation)

For each feature slice (`20`, `30`, …), spawn `ROLE_PLANNER` (`TIER_ANALYSIS_PLAN`) to author a durable draft plan at `<plan-dir>/slice-<N>/plan.md` (fan out independent slices concurrently). **Write the plans only; do not perform slice work.**

Each draft plan must be an executable plan (not a high-level note) and include a **Slice Prep** section covering:
1. **Goals & non-goals:** Clear slice outcomes and boundaries.
2. **Draft Needs, Requirements & Acceptance Criteria:** Proposed additions/changes written durably local to the slice.
3. **Architecture & Draft Design:** Read current architecture, draft the design, identify challenging technical areas, perform web research on difficult aspects, verify architectural fit, and revise the draft.
4. **Prerequisites discovered:** Any prerequisite work surfaced during planning, flagged explicitly for the next reviewer.
5. **TDD Implementation & Verification Plan:** Concrete RED → GREEN test plan, blast-radius check, GPU protocol (if GPU-dependent), and proportional verification commands.

---

### Phase D · Release Closure & Worktree/Branch Cleanup

Once all slices are complete, final verification passes, and the release is merged/tagged/published per project release rules:
1. Verify all work on `dev/<x.y.z>` is cleanly merged and pushed to its target destination (no unmerged commits or dirty files left behind).
2. Remove the release development worktree and any remaining temporary slice worktrees (`git worktree remove <path>` followed by `git worktree prune`).
3. Delete merged local temporary/slice/dev branches (`git branch -d dev/<x.y.z>` and any merged helper branches; never force-delete unmerged work without owner confirmation).
4. Record final release closure status in the durable release ledger/board.

---

## Section 2 · Per-Slice Execution Template (Slices 20+)

Copy and instantiate the block below when dispatching a subagent (or starting a session) for any Slice `N` (`N >= 20`):

```markdown
# Execute Slice <N> — `<project-name>` (`dev/<x.y.z>`)

## Context & Guardrails
- **Branch / Worktree:** `dev/<x.y.z>` in `<dev-worktree-path>`.
- **Workspace hygiene:** A new branch or worktree is normally unnecessary for a sequential slice. If you create a temporary branch or worktree, merge it cleanly back into `dev/<x.y.z>` and remove the temporary worktree and branch before finishing.
- **Stay pragmatic & minimize ceremony:** Solve the work outlined in Slice `<N>`, check impacts across the blast radius, and avoid overbuilding.
- **Proportional verification:** When resolving a specific finding, run targeted tests for the blast radius rather than full regressions unless the change is comprehensive enough to warrant the overhead.
- **GPU contention:** If GPU resources are required and unavailable, follow the **GPU Availability & Backoff Protocol** (do not fail immediately; poll with jittered exponential backoff up to 6 hours / 24 attempts).
- **Direct delegation:** Use your harness's native subagents directly per Section 0 (no Steward/Orchestrator wrappers or MCP orchestration tools).

## Execution Phases (Follow in Order)

### 1. Reconcile & Update the Slice Plan
1. Read `<plan-dir>/slice-<N>/plan.md`, any slice-local change plans, assigned features/functions, allocated prework drafts, and relevant repo changes landed since the draft was written.
2. Enumerate deltas since the draft plan was authored.
3. Update, approve, reject, or adjust the draft plan so it is **complete, accurate, and not overbuilt** (`TIER_ANALYSIS_PLAN`), ensuring coverage of:
   - Needs, Requirements, and Acceptance Criteria updates.
   - Design updates.
   - TDD implementation (RED → GREEN) and proportional verification scope.

### 2. Slice Plan Review Gate (Max 1 `FIX-n` Cycle)
- Spawn `ROLE_REVIEWER` (`TIER_DEEP` per Section 0) to critically review the updated Slice `<N>` plan for completeness, architectural fit, and overbuilding.
- Double-check all edits before returning for re-review (only **one** `FIX-n` cycle is allowed).
- Once `APPROVE`d, commit the updated plan before starting implementation.

### 3. Requirements & Design (with Design Review)
- Finalize slice Needs, Requirements, and Acceptance Criteria.
- Write/update the slice design document.
- Spawn `ROLE_REVIEWER` (`TIER_DEEP` per Section 0) to review all design additions and changes; resolve findings before coding.

### 4. TDD Implementation & Code Review
- Implement according to the approved plan (`ROLE_IMPLEMENTER` per Section 0) using **TDD (RED → GREEN → Refactor)**—failing test first; fix code rather than weakening tests.
- Spawn `ROLE_REVIEWER` (`TIER_DEEP` per Section 0) for an adversarial code review of the full slice diff. Resolve all material findings with targeted verification.

### 5. Independent Verification
- Spawn `ROLE_VERIFIER` (`TIER_STANDARD` / `terra` / `browser` as applicable per Section 0) to independently run the slice's verification suite against real artifacts/databases (no mocks) and report raw results.

### 6. Status Record, Cleanup & Commit
1. Write `<plan-dir>/slice-<N>/status.md` recording landed changes, review verdicts, and verified test results.
2. Merge and delete any temporary worktree/branch if one was used.
3. Confirm clean `git status` and commit all slice work on `dev/<x.y.z>`.
```
