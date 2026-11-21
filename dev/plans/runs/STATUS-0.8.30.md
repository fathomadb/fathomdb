# FathomDB 0.8.30 release status

**CLOSED — historical record.** Published on 2026-11-21 from immutable `v0.8.30`.
All selected slices are landed; registry, Tegra and final documentation delivery
are verified in the [Slice 80 record](../0.8.30/features/slice-80/status.md).
The [state](../release-state-0.8.30.json) owns status and next actions. The
[master plan](../0.8.30/plan.md) owns scope and execution contracts.

## Current state

**Complete.** All selected slices are landed and 0.8.30 is published. The [delivery record](../0.8.30/features/slice-80/status.md) preserves actual publication, recovered consumer scopes, Pages results and original failures.

## Next action

**None.** Release delivery and the requested Use Cases page are complete. Broad coverage-guided fuzzing and power-loss proof remain allocated to 0.8.31; the agent/harness research remains in the 0.9.0 backlog.

## Historical decisions and phase observations

- Scope recommendations remain ruled at seq-399; preparation approval is seq-400.
- Reviewed Slice10 implementation is integrated at `9c9b786585961d5393d1155eb6b44b611b63e06d`.
  [Actual outcomes](../0.8.30/features/slice-10/status.md) record implementation
  completion with qualification open. The full source gate failed; it is not green.
- Affected Markdown cohort checks recovered after the mechanical correction.
  Remaining tooling fixtures belong to20, hints timing to40, logging-fixture
  synchronization to50, source qualification to70 and renderer/platform
  qualification to75.
- All numerical and security criteria remain intact. Public Node26 support,
  additional platform/native proof and registry publication remain held.
- Reviewed Slice20 implementation is integrated at
  `5e7906b6d7a871611ccdddd74a9ed1410c32cc47` under seq-401. Its
  [status](../0.8.30/features/slice-20/status.md) records source/offline results,
  the original failed full gate and independent scoped recovery of both fixtures.
  The two inherited tooling holds are resolved. Hosted cancellation, Windows
  runtime and later qualification remain open; no release green is claimed.
- Reviewed Slice30 implementation is integrated at
  `5b0348775279883fe2ffcd52f60c23c90171166e` under seq-402/403. Its
  [status](../0.8.30/features/slice-30/status.md) preserves the failed wrapper,
  original 234-pass/four-failure test collection and successful focused recovery.
  Terra verified final production source `47bd06261867105964bf6a2aa2875eaee4c4adb3`.
- Slice40 is commissioned at seq-405 using the full template and whole plan.
  Feature slices50–80 remain uncommissioned. Inspector stays separate.
- Owner seq-404 places the Vercel research in the [0.9.0 backlog](../0.9.0/backlog.md).
  The .30 session remains focused on its correctness and robustness goals.

- Owner seq-406 adds explicit SQLite-method coverage and gap work to draft50/70.
  The [audit](../0.8.30/coverage-audit.md) distinguishes actual proof, inherited
  assertions and planned execution; the added rows are not yet verified.

- Owner seq-407 splits the remaining plans into50 runtime,55 data integrity/recovery,
  70 integrated source behavior and75 installed platform/artifact qualification.
  Completed statuses are unchanged. The added slices are not commissioned;
  current40 continues.75 now supplies the qualified candidate to80.

- Reviewed40 source is integrated at`d64e870bd96103851b104afc927d48c64067db35`.
  Its [status](../0.8.30/features/slice-40/status.md) retains the original full-gate
  exit1 and237/238 passing suites, sole observer recovery11/11 and independent
  rebuilt native verification. All review findings are remediated. Inherited
  hints qualification remains open with70; final pin/platform proof is unchanged.
- Owner seq-408 commissions the whole revised50 plan using the supplied template
  and `gpt-6.1-sol` high after40 completion.55/60/70/75/80 remain uncommissioned.

- Owner seq-409 replaces finish50-before55 with55 →60 →finish50 →70 →75 →80.
  Reviewed partial50 runtime/logging source is integrated at
  `daf12b6d9839816fab4cd7c72c81e2f10918bf0f`; its two remaining rows stay open.
  55 is commissioned with `gpt-6.1-sol` high and the full template. After55 and
  review remediation, activate60 then finish50 without a new sequence approval.
  Concrete live-fault/venue authority and80 publication remain separate.

- Reviewed55 is integrated at`3595192b0a3d4587556aa00b24768a06469af3ba`.
  [Status](../0.8.30/features/slice-55/status.md): all six selected rows, actual
  canonical verifier exit0,238 passed/3 explicit skips/0 failures; separate
  outer-monitor124 preserved. All reviews passed and owned processes ended.
- 60 is activated under the existing seq-409 mandate after55 completion.
  Then finish50;70/75 and separately authorized80 publication follow.
