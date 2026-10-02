---
title: Slice 90 stage-2 independent verification
status: PASS
target_release: 0.8.27
---

# Slice 90 stage-2 independent verification

Candidate SHA: 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd
Verdict: PASS
Reviewer: gpt-5.6-terra
Evidence: Independent read-only verification at clean merged tree 9b04d0b25568c0f484a84bbada8aad69f7db7a4e reran 48 D27/checkpoint tests, validated the sealed AC-073 receipt, and audited source and artifact identity.

Terra confirmed byte-identical production engine source between the measured
candidate and merged tree, reran the 48 lightweight D27/checkpoint tests with
`PYTHONDONTWRITEBYTECODE=1`, and directly validated the committed AC-073
stress receipt with no errors. It checked the strict v1 historical entry bridge
and the v2 retry receipt, seven applicable numeric-PASS AC-081 observations,
named selector records, and installed Python/Node evidence. It independently
matched all 25 hashes in the then-current stage-2 manifest. A subsequent
explicit `2/1` merged-tree test passed and its hash was added to that manifest.

Terra did not rerun the host-heavy, installed, or GPU workloads. The full
unconfined `agent-verify` gate independently passed 130/130 registered suites,
zero skipped/excluded, and strict security with zero violations, blockers or
downgrades. Terra's narrative called the AC-081 R1 sequential time the median;
the official seven-run summary reports the median as 187.741 ms and the
concurrent median as 65.277 ms. The seven-run PASS finding is unchanged.

The AC-073 stress result is 493 ms within its same-run 513 ms bound. Its
combined selector exited 101 on retained AC-075 recall, which is not claimed
as a combined PASS. Terra preserved the valid D27 FAIL/PASS conflict and did
not mark the runtime checkpoint PASS.
