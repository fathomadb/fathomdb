---
title: Inspector 0.2.0 Slice 75 execution status
status: COMPLETE
---

# Slice 75 status

COMPLETE on the owner-approved sequential release worktree/branch
`inspector/dev/0.2.0`, planning baseline
`4288d1a6347b43d1ef48e4b504808b773c90f8df`. All seven expert findings are resolved;
[execution reconciliation](plan.md#execution-reconciliation) and
[reviewed design](design.md#reviewed-implementation-decisions) record the source
constraints. Inspector-only; no Engine/SDK/MCP/API/schema/pin changes, main merge,
push, tag or publication. The original published 0.8.29 database and 41 screenshots,
and the prior Slice 70 installed package remain unchanged historical evidence.

## Remediation and source evidence

| Finding | Existing needs/AC | Implemented behavior and RED→GREEN |
| --- | --- | --- |
| 1 · attachment | UN-01/09; INS-02.4/X01/X09 | Durable accepted attachment state, separately scoped feedback, late-response guards. `bc95a0546`→`f45f0c53a`. |
| 2 · correspondence | UN-05/08/14/16; INS-05.1/5/7/8/9 | Typed native membership remains unchanged; separately labeled bounded exact-origin correspondences preserve ranks/scores and require one permitted known physical origin. `d63ffd842`→`2442b32cf`. |
| 3 · layout/actions | UN-06/07/10/17; INS-03.2/4, INS-09, INS-X05/08 | Query/origin actions precede long content; raw regions are locally contained at 1042/640 px. RED measured 3057 px page width in 1042 px viewport. `ed16d5a5f`→`db43f1916`. |
| 4 · labels/counts | UN-02/03/04/05/14; INS-01/02/03.1 | Admitted title/name previews read only complete bodies ≤4096 bytes under shared budget, ≤120 characters; distinct explicit unavailable/oversize fallback and exact identity retained. `1dfe32914`→`fab4002cd`; review fallback `c9fc734bd`→`1bbfc886b`. |
| 5 · properties | UN-05/15/16; INS-03.1/04.3/09.2 | Actual eligible declaration choices including default tokenizer, local invalid-input feedback and original-result attribution. `1576b29e1`→`9a4653614`; real backend-refusal detached-feedback `aa446715c`→`1bbfc886b`. |
| 6 · graph | UN-07/08/10; INS-03.4/09.4/6/7/X02/08 | Bounded selected-anchor priority, independently owned traversal frontier, stable local viewport/keyboard controls and scope beside diagram. `b5a597d0b`→`71e469d85`; review RED `e338f7e8e`→`5d1163eaa`. |
| 7 · evidence | UN-08/13/15/16; INS-06.1–4/08.1–4 | Compact original method/request/view/filter/limit plus separately timed later source/policy checks; exact-unit BigInt UTC alongside raw epoch, unknowns remain unknown. `9aa483ac6`/`d4eb3bcd7`→`b838e932f`. |

Direct Sol-high design review approved constraints applied in the implemented
source. Fresh Sol-high code review found two P2 graph gaps, remediated and
re-reviewed APPROVE at `5d1163eaa`. Its final canonical-AC check found the missing
explicit preview fallback; a genuine failing browser case precedes that fix.
Focused final source re-review APPROVE includes explicit fallback and current-view
refusal feedback. No material source findings remain.

Focused source evidence: Rust graph 9/9 including all 8 disclosure combinations
PASS; comparison/presentation unit 10/10 PASS; graph+UX browser 16/16 PASS before
final fallback, then UX+lexical 13/13 PASS including unchanged inherited lexical
oracle and both latest review regressions. The first broad comparison/lexical/
vector/workspace run 33 PASS/1 cancelled is preserved; the failing lexical case was
fixed, not waived. Initial installed Ubuntu 22 UX 7/7 PASS precedes the final
follow-up and is not substituted for final-candidate qualification.

## Updated installed candidate and independent verification

Final unpublished candidate:
`/mnt/data/inspector75-qualified/final/fathomdb-inspector-0.2.0-linux-x86_64/`.
Sibling tar/checksums are preserved. Binary SHA256
`7b25646ae14c7df6448a9dc4765f4302abdad2130f53a832345f157291321e52`,
build `f1e9337dacdf79f77b60dd382a197eea340aa816aa3f1cc96992ab059ddbf7d1`.
Locked/offline Rust 1.95 release, explicit `cached-cpu=true`/`fixture-control=false`, selected
Engine `6edc1155f`/schema 37/protocol 1. Separate Ubuntu 22 target retained at
`/tmp/inspector70-jammy-target`; no host objects reused. Actual offline Ubuntu 22.04/
glibc 2.35, existing kernel 7.0.0-43/Landlock ABI 8, Chromium 156 normal sandbox;
ELF maximum required GLIBC_2.34. All four package checksums pass. This qualifies the
selected userland/kernel/browser pairing, not stock Ubuntu 22/kernel 5.15 or all
platforms. The previous Slice 70 artifact and first Slice 75 staging artifact remain
separate historical builds.

Independent direct Terra used fresh copies of the preserved published 0.8.29
review store and found no product defect in these exercised scenarios:

- Attachment, detach, missing-path refusal and reattachment state/feedback PASS.
- `quartzledger` body/BM25F: 6 typed union entries, 0 typed intersection, 2 separate
  verified shared revisions (`review:robin-a` cursor 658, `review:robin-c` cursor 660),
  distinct native ranks/metrics, keyboard origin and original Back/focus PASS.
- Ten actual `title`/`fleeting` results: document widths 1027/625 at viewports
  1042×793 and 640×793, local raw scrolling and preceding origin actions PASS. Actual
  screenshots visually inspected by Terra and parent.
- Stored corpus titles/explicit fallback and 223 node revisions/222 identities/
  444 edges with distinct count labels PASS.
- Actual declared `body`/`title` choices, invalid nearby feedback, ten-result
  successful retry and cleared feedback PASS.
- Note→`project:compliance-audit` retains selected hub at partial 93 nodes/107 edges/
  108 examined links; keyboard zoom and collapse→3 nodes/2 edges PASS.
- Compact own request/method/metric and native versus later policy/origin time,
  UTC+exact epoch and expired record epoch 1700000000 with separately sourced
  later facts PASS.

The same corpus note opened actual lexical, stored 384-dimensional vector,
measured L2 and cache-only CPU hybrid views. Exact `Tromsø 🦀` and
`9007199254740993` clipboard readback, source, registered dependency and retained
revision navigation PASS. Installed D-01 comparison all 8 combinations PASS.
Successful native harness runs verify actual Alt+F4 closure, recorded descendant
reaping, private-directory cleanup, independent lock reacquisition, 0 external UI
requests and sampled service/browser caps. Unchanged scale/cache-confinement
campaigns retain their original Slice 70 scope; no wholesale new qualification or
human-study pass is inferred.

Terra scripts, screenshots 01–11 and actual logs are preserved under
`/mnt/data/inspector75-qualified/terra/`, notably `verify-with-lock.log`,
`verify-targeted.log`, `verify-final-targeted.log` and `policy.log`. Those scenarios
passed collectively across completion-aware targeted runs, not an uninterrupted
initial four-case run. The focused independent equal-content identity probe PASS 1/1: `robin-a` and
`robin-b` retain different copied IDs with identical complete 5,562-byte bodies;
`robin-c` has the same name and different content. Fresh A/B pages retain zero
shared exact canonical revisions and zero typed intersection. Evidence:
`equal-identities-rerun.log` and screenshot 12. Actual native close 180 ms,
service sampled peak 221,933,568 bytes and browser heap growth 18,251,670 bytes, with
successful lock/private-state/owned-descendant cleanup. Its first early pin was
legitimately FIFO-evicted by intervening reads; that failed setup log remains.
The corrected setup pins both fresh bounded observations immediately before
comparison without weakening its identity oracle.

## Final source gate and preserved attempts

The single strict `PATH=/tmp/inspector40-tools:/usr/local/go/bin:$PATH
./scripts/agent-verify.sh` at clean `1bbfc886b` exited 0: **206/206 suites PASS,
skipped 0/excluded 0**. Lint/typecheck and strict security PASS, 0 violations/
0 blockers/0 downgrades; AC-036/037 syscall/netns checks actually executed.
Individual ignored/conditional cases remain distinct, including inapplicable
AArch64/CUDA lint and opt-in long measurements. They are not counted as executed
positive qualification. Source was frozen through the gate; no broad repeat.
Scoped JavaScript/shell syntax and ShellCheck, Markdown/docs lint and strict
MkDocs also PASS. Raw author logs, including failures, are preserved under
`/mnt/data/inspector75-qualified/logs/` from `/tmp/inspector75-*.log`.

Preserved failures: the initial title browser assertion raced page completion;
`c00a9ea5a` added the explicit completion wait without changing expected values.
The first broad comparison/lexical/vector/workspace run 33 PASS/1 cancelled exposed
refusal feedback written into a detached form. A wording-only rerun failed again;
request/step diagnostics established the actual invalidation cause. No tests or
native semantics were weakened. The real invalid-limit RED now passes after
current-view feedback rendering; empty input is handled locally and preserves
the prior result. Historical failed attempts remain failed.

Five real human participant sessions remain UNPERFORMED in Slice 70. They do not
block Slice 75 completion, and Terra/expert checks cannot satisfy them. Updated
candidate is available for that remaining release qualification.

The first independent review launches correctly refused a disposable baseline
that omitted the original lock sentinel; `diagnostic.log` and screenshot 00 retain
the refusal. Copying the existing original sentinel into the owned baseline
resolved setup without modifying the source database or product admission.
Immediate focus/clipboard assertions failed before asynchronous completion;
completion-aware targeted reruns preserve the same intended values. These failed
attempts remain failed, not retrospective passes.

## Closure and cleanup

After the full gate's workspace feature unification, explicitly restored the
cached-CPU binary/examples with locked offline Cargo. The restored development
pair's real person investigation PASS 1/1: full biography, indexed match, head
vector/L2, actual CPU query, edge/source/history and original Back. This verifies
the restored artifact configuration; it is not another broad regression run.
The independently tested release executable is unchanged. Package checksums and
original review database hash remain verified; the original 223-revision store,
source manifest, seed and 41 screenshots are preserved.

All owned runtime processes have exited and native private resources/locks were
released. No extra branch/worktree was needed. Preserve the owner-approved
release checkout, target symlink/local exclusion, original datasets and existing
caches; no main merge/push/tag/publication. Source/build/runtime ownership returns
to the parent. Slice 75 is complete independently of Slice 70's unperformed five
human sessions. Final closure changes are documentation only and use scoped
validators, without repeating the passed full gate.
