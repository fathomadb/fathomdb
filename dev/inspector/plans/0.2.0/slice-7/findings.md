---
title: Inspector 0.2.0 build and delivery evidence assessment
date: 2026-11-19
status: REVIEW
---

# Build, verifier, CI and installed delivery

Reviewed source `aef5b2eac19dbcf5850713081ccd427896909eea`, Slice 7 plan/design/
filled prompt, 0/1/4/5/6 reports, actual scripts/workflow conditions/package
builder and retained release/security evidence. Keep strict preflight and
proportional informational CI; adjust old planning SHA and no separate current
Engine-ladder assumption. No builds, campaigns, scans, installs, hosted workflow
runs, registry probes, credentials or release actions performed here.

| Item / symptom and evidence class | First actionable cause / known delay | Minimal proposal, owner and observable validation |
| --- | --- | --- |
| S7-A = S0-ENV: port full verifier stops twice before lint | Recorded exact failure: checkout-owned .venv/bin/python missing; agent-verify.sh already fails early and tests pip. Current release checkout also has no venv. Elapsed release delay unknown. | Preserve early stop; conditional 10 approved isolated native-wheel/own-venv preparation. Do not duplicate preflight, borrow venv or fall through to system Python. Full source gate only at source phase as required. |
| S7-B: generic worktree/dependency preflight selects Engine state | `preflight.sh --worktree/--expect-closed` loads live Engine selector and expects its ladder. Inspector has separate owner-approved baseline. Current bootstrap health and linked --landing checks passed with explicit selected ancestry. No actual false refusal asserted. | Keep health/landing plus exact Inspector ancestry/occupancy checks. No Engine state edit/closure fabrication or rebase. Shared “build on MAIN” reminder conflicts with own-checkout policy; separately owned harness/doc clarification optional, not an Inspector blocker. |
| S7-C: Inspector-only paths do not select language-heavy CI | Source-established: rust filter lists src/rust/**, Cargo.toml, Cargo.lock, omits src/inspector/**; heavy verify requires rust/python/TS/harness (unless manual release/CI-workflow). verify-fast still selects non-Markdown source. Existing proportional-routing fixtures have Rust paths, no Inspector sample. No hosted skip log obtained. | Individually approve narrow CI-path coverage in existing routing fixtures for Inspector Rust/assets/package versus Markdown/Engine/manual cases; map to needed jobs without broad cross-platform expansion. Shared CI owner; small/medium effort. Future executable change requires real failing fixture then GREEN, actionlint and focused routing check plus required final source gate. |
| S7-D = S0-ABI: built GNU artifact does not meet 22.04 floor | package.sh uses local x86_64 GNU release/default locked/offline build; package README records GLIBC2.39/Ubuntu24.04.5. Target triple alone cannot establish ABI floor. New compatible executor not selected; actual delay unknown. | Inspector owner selects 22.04-compatible builder/extracted artifact check; 70 installed default all-view/offline/native close. No claim existing 24.04 binary qualifies. |
| S7-VERSION: package name still hardcodes 0.1.0 | Manifest is 0.1.0 unpublished, package.sh archive and package test expect that version. Correct for baseline, not a current defect. | Keep now; 70 atomic Inspector-only 0.2.0 version/help/archive/test/docs update and installed pair check. Do not bump workspace/Engine version or publish. |
| S7-E: historical Gitleaks mismatch | Retained redesign document records expected=100 observed=121 unknown=21; not today's scan. Actual ci.yml current-tree job continue-on-error; release history advisory independent and no publisher needs it. Scripts use redacted safe reports. Latest hosted count/result unavailable. | Keep separation. Refer concrete future current/history/tool-failure evidence to security owner; no ignore widening, token output, scanner campaign or claim advisory job blocks publication. No Inspector remedy established. |
| S7-F: registry first publication versus propagation | release.md records initial SDK creation by API token, missing trusted-publisher recovery, later OIDC identity-only run 42750034084 with build/publication skipped. Exact-version wait-for-crate-version.sh already polls bounded 300s/default 5s rather than fixed sleep. Current registry/auth state unqueried. | Reuse existing owning release path; first-crate bootstrap, auth, visibility and installed smoke are separate failures. Inspector publish=false/local archive has no demonstrated registry prerequisite. External owner, postpone new work unless concrete dependency emerges. v-tag is real publish; no action taken. |
| S7-G = S5-METHOD: scoped pass mistaken for required native/install completion | Port scopes and four ignored native/scale/allocation cases remain explicit. AC-036 strict strace needs ptrace; tool presence does not prove sandbox capability. Resources synthetic pause and native launch SIGTERM are limited tests. | Retain strict failure semantics and required positive/native installed work under 20–70. Report executor failure distinctly; no quarantine, fake unavailable pass or full campaign for prose. |

Current root Markdown gate additionally requires local node_modules binary and
fresh checkout lacks it; 0 inventories this. Existing gate documents a primary
node_modules symlink convention, but creating a setup link is outside prework's
no-environment-change bound. Use existing tool read-only for scoped documents
and record normal gate failure; no waiver or install is inferred.

## Architecture and acceptance consequence

Build → source verifier → diff-routed CI → local archive → installed use are
separate evidence layers. Engine multi-registry publication is a separate
external branch. Normal hooks/security remain enabled; current Gitleaks warning
posture does not authorize ignoring the staged hook. No required-status
aggregator, merge queue, soak period, broad CI matrix or new telemetry/scanner
framework is justified. S7-C only repairs coverage in existing controls if chosen.

No new product need/AC needed. INS-X01/03/07/09/10 and F-05/06 already require
exact default build pairing, real positive behavior, resource bounds and installed
release. S0-ENV/ABI and S5-METHOD carry remedy ownership once; S4 capability
blockers cannot be repaired by CI labels. 9 decides individual proposals;
conditional 10 applies only selected in-scope setup/wording/routing, features
20–60 complete local behavior, 70 final installed qualification. External shared
code remedies require explicit scope authorization rather than commission drift.
Independent draft review/Terra and Markdown limits are in [the shared record](../slice-9/prework-verification.md). No runtime green.
