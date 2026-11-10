# Slice 10 minimal release bootstrap proposal

This is a reviewable procedure only. No ref, index, source, state or board was
changed. Parent performs serial metadata import after actual9 completion and
10 plan PASS; source implementation stays in the fresh10 owner's worktree.

## Candidate files and exact facts

Create only `dev/plans/release-state-0.8.29.json` and
`dev/plans/runs/STATUS-0.8.29.md`; add three declared generated regions to
`dev/plans/plan-0.8.29.md`: plan-immediate-next in master, status-current-state
and status-next-action in board. State owns all region content; parent invokes
existing renderer. No marker appears in undeclared files. Historical.28 stays
byte-identical. Existing9/master direct-mandate reconciliation is retained.

State fields: release0.8.29; board/plan exact paths above;
master `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md` (existing ACTIVE
program authority, also cited by historical.27/.28 states);
active_ref refs/heads/release/0.8.29; schema_version37; no completion/published
receipt, no main-integration/publication claim; public_docs_target_release0.8.28.
ladder has each actual0–9 proposal/reserved closure exactly once with full
reachable SHA and actual independent review log;10 IN_PROGRESS without closure
SHA;20..160 open/conditional per accepted master. `landed=[]` (no remote-main
claim), `next_slice=10`, remaining_ladder10,20,...160. Preserve conditional40,
90 approved80+Block1 exit,140 repair-only and150 individual selection conditions;
no numeric dependency asserts a pending artifact/fault/optional ruling is met.
Decisions.ruled carries actual batches/direct mandate/keep-query/seq367/368 and
settled compact/B2-8/omission boundaries; unruled carries only genuine actual
item-specific choices from reviewed9. Readiness status NOT_READY/publication HELD. The master plan supplies actual
requirements, immediate-next-slice, slice-ladder, reserved-gap and cross-cutting
DoD headings for the existing commission citation contract; no hook is weakened.

Actual current closure witnesses:0 at850f2c15df1ebf3262f0346e340721f9ff2422ad;
1/2 at39289560125367aea0c80fb335c6d51260376ba0;
3 at2d5041795f0cbbe49d55d81a2710f09e2043b8a9;
4/5 at52a85c38cb50af220b418a4d05bc4c6e31f23fe2;
6 atde85cbf40c85cd89dd8785453346b5eb0367f01c;
7/8 ata0274cee6722c415e90b0bdda7a5578a43d41fda.
Verify these commit objects/actual files/reviews from Git again;9 at e76031ae422e56b9aead73042dedc6190a96d0ae
now has actual dual review and parent closure. Exact state/board/master candidates
are supplied beside the eight Slice10 documents at their repository-relative
paths. Their closure witnesses/three declared views were statically checked;
no candidate was staged, activated or certified by strict preflight.

## Proposed command order, with real exits

1. In attached existing parent, verify actual Git root/llm branch, no concurrent
   writer, clean imported planning baseline and final9 PASS records/closure
   commit. Save `bootstrap_base=$(git rev-parse HEAD)` and record0–9 ancestor
   checks. Read release-current.py empty current selector before activation.
2. Verify `release/0.8.29` is absent (`git show-ref --verify` nonzero is expected
   absence, not a test PASS). If absent, `git branch release/0.8.29 "$bootstrap_base"`.
   If present, inspect owner/worktree and require actual9 SHA ancestral to that
   ref and ref ancestral to bootstrap_base. A conflicting ref stops; do not
   overwrite/reset another branch. Save its exact old SHA as `release_old` for
   the later compare-and-swap. This local ref makes no remote claim.
3. Import only reviewed JSON/board/master candidates; stage these exact files
   with `git add -- <three paths>` so tracked-only discovery sees new inputs.
   Run `scripts/check-release-state-views.sh --write`, restage declared outputs;
   run `scripts/check-release-state-views.sh`, `scripts/release-current.py` and
   `python3 scripts/preflight-release-state.py --release 0.8.29
   --state dev/plans/release-state-0.8.29.json
   --board dev/plans/runs/STATUS-0.8.29.md --plan dev/plans/plan-0.8.29.md
   --expect-closed 9 --target-head HEAD` as one command.
   Require exact.29 selector and0 exits. Verify historical.28 diff empty.
4. Run `scripts/preflight.sh --expect-closed 9 --plan dev/plans/plan-0.8.29.md`
   in this existing checkout, retaining all strict guards. Run scoped metadata/
   Markdown checks and commit through normal hooks; missing owned Markdown
   tooling remains a disclosed setup failure, not a skipped/pass gate. Parent
   decides permitted existing hook/fallback behavior; no hook bypass is proposed.
5. Record `bootstrap_commit=$(git rev-parse HEAD)` after the metadata commit.
   Verify the local release ref still equals its recorded expected old SHA, is
   not attached to any worktree, and `git merge-base --is-ancestor` from old SHA
   to bootstrap_commit succeeds. Fast-forward only with compare-and-swap:
   `git update-ref refs/heads/release/0.8.29 "$bootstrap_commit" "$release_old"`.
   Any concurrent ref movement/attached branch/nonancestor stops; no force/reset
   or branch switch. Parent remains on llm/0.8.29-chat. Confirm `git show` at
   that exact release ref includes the reviewed state, board and master bytes.
   From that committed bootstrap tip, parent runs strict dependency9 precreation
   preflight, then `git worktree add` for a distinct unused llm source branch
   under the approved project-worktrees root. Do not infer origin/main's old tree
   has these plans: require fetched/known main and product baseline ancestral to
   bootstrap tip, preserving explicit current.29 integration ancestry. Parent
   creates `llm/0.8.29-s10-preparation` (unique suffix if occupied) at the exact
   bootstrap commit. Same10 owner enters that cwd, verifies HEAD and committed
   bootstrap files, then runs strict
   `scripts/preflight.sh --worktree <absolute-path> --expect-closed 9
   --plan dev/plans/plan-0.8.29.md` as one command. Parent never edits the child
   source worktree. No stale-base exception.
6. Require new worktree HEAD==bootstrap_commit==local release ref, clean tracked
   source, and state/board/master bytes equal their approved committed blobs.
   Only then provision reviewed owned resources and begin TDD implementation.
   Closing10 later requires actual code/evidence/fresh review/integration SHA,
   state update through JSON and regenerated views. No self-referential closure
   hash, invented product PASS or ledger event is needed for this bootstrap.
   S10 acceptance closes preparation only; release_readiness stays NOT_READY and
   publication HELD until separate release gates pass.

All shell substitutions above are task variables; don't repurpose HOME or any
system option. Capture checker output and its direct status; never pipe through
tail or mask failures behind echo. No checkout switch, environment install,
external permission mutation or publication is needed to bootstrap metadata.
