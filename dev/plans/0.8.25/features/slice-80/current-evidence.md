# Slice 80 — current evidence and Slice 85 handoff

Slice 80 is complete at `e54ad00c`, with state/handoff commit `846c6ed3`.
This index identifies the accepted evidence; older top-level manifest and
measurement entries remain historical, not additional open acceptance gates.

| Obligation | Accepted evidence | Slice 85 disposition |
| --- | --- | --- |
| AC-081a/b | [Measurements](../../../runs/0.8.25-slice-80/measurements.json), `slice80m.ac081`: seven qualified passes; medians 172.883034/52.527637 ms | Reuse while relevant runtime/query/build inputs remain applicable. |
| AC-081c | [Evidence review](../../../runs/0.8.25-slice-80/verification-review.md): real-database independent-reader proof | Reuse or consume its execution in the normal regression round; no separate duplicate campaign. |
| AC-072 | [Revised qualification audit](../../../runs/0.8.25-slice-80/80n-revised-qualification-audit.md): C1–C5 pass, C1–C3 meet the required three consecutive passes | Reuse with the approved host-swap-as-diagnostic policy; preserve original verdicts. |
| Protected 71B writes | [Slice 79 result](../../../runs/0.8.25-slice-79/result.md): all six candidate cells pass | Reuse unless write/projection/runtime/build changes invalidate them. No historical baseline reruns. |
| Runtime configuration | [Slice 79 design](../slice-79/design.md) and [verification](../../../runs/0.8.25-slice-79/verification-review.md) | Reuse core behavioral proof; verify applicability to final installed artifacts and SDKs. |

The read-performance executable is SHA-256
`ff4b78f36898f8109eef2dcb64f91a99741f9619433d77de0cfef0e59d51895d`;
its protected input hash is
`95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274`.
The AC-081 campaign source is `ad422346`; AC-072 continuation source is
`38987930`. Full identities and raw hashes are in the linked records.
Source identity and executable build identity are distinct; documentation-only
commits do not require rebuilding or repeating accepted performance cells.

AC-020 remains retired. AC-072 remains 10k/384d/1,000 measured warm queries,
p50 <=80 ms and p99 <=300 ms. Host-wide swap is retained diagnostic context,
not a standalone invalidator; other accepted controls remain in force.
Do not import the old zero-swap rule from historical Slice 71/75 protocols.

Slice 85 owns reconciliation of remaining Slice 75 cells and applicable
Slice 72/73 platform/model receipts, one final broad round, CI and installed
package verification. Those are not unfinished Slice 80 work. Use the
[Slice 85 execution matrix](../slice-85/execution-matrix.md); release readiness
remains pending that work, and publication is not authorized.
