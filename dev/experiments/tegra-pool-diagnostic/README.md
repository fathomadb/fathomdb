# Tegra private-pool diagnostic

Slice 95 CI22 authorizes one preregistered local diagnostic after code review and
independent verification. This is an investigation, not a v1 witness or release
qualification. Earlier v1 failures remain failures; the 64 MiB v1 floor stays fixed.

`run.sh` runs the unchanged maintained installed Node row once. Prepare a fresh
consumer containing the exact candidate main/Tegra packages and bind their native,
package and source hashes externally. Then, from this repository:

```sh
bash dev/experiments/tegra-pool-diagnostic/run.sh "$CONSUMER" "$NEW_EVIDENCE_DIR"
```

Use the pinned Node 25.9.0 environment. The runner preserves the child PID, exact
command, row source hash, stdout/stderr, row JSON and original exit status. A v1
failure can therefore leave a useful diagnostic while the runner still exits 1.
Do not retry it to obtain a pass.

The controlled window is the single awaited `Engine.open`. The driver launches no
concurrent SDK/GPU operation or additional worker during that window. Native N-API
and runtime threads still exist; this is not an OS-single-thread guarantee. The
unchanged row can call report/embed/close afterwards, outside the witness window.
No general runtime exclusivity guarantee is claimed.

The private `FATHOMDB_GPU_POOL_DIAGNOSTIC=1` switch is effective only inside an
already requested CUDA allocation witness. Other values leave it off. It emits one
`fathomdb-tegra-pool-diagnostic` stderr line with the distinct
`fathomdb.tegra-gpu-pool-diagnostic/v1` JSON schema, including original v1 outcomes
and partial failure observations. It adds no OpenReport or SDK field. Opt-out
performs no extra probe, control or diagnostic output.

The session holds the actual witnessed stream, context and private pool once. Its
identity includes PID, ordinal, UUID, checked driver context ID and raw handles.
Original global v1 readings are retained separately. USED, RESERVED and diagnostic
global reads follow sequentially; they are not an atomic snapshot. Query and
identity failures invalidate only the diagnostic. A non-private allocator is
explicitly unsupported and receives no diagnostic control.

Only after original samples, checked model load, real forward and the computed v1
verdict does the still-live model's pool receive one 64 MiB control on that same
stream. It is touched nonzero, completed and measured; only that control is dropped,
then completion and exact USED-baseline restoration are checked. The original GiB
raw allocation control, identity checks, floor, forward and v1 verdict stay intact.
No trimming, allocator policy change, sleep, row reordering, retry or clamping occurs.

Any missing identity, query/completion error or control rise/release failure stops
the counter proposal. Only a verified same-pool model USED increase of at least 64 MiB
plus the live/released control and external exclusivity attestation can support
reviewing a successor design. RESERVED/high-water/global values cannot substitute
for USED; this tool does not adopt a successor method or make an acceptance claim.
