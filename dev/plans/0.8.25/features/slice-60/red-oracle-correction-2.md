# Slice 60 Python RED oracle correction

Date: 2026-09-07

An independent read-only audit confirmed that the Python RED oracle's `_edge`
helper emitted a noncanonical flat mapping with `"edge": True`. The shipped
Python write contract requires an edge envelope of the form
`{"edge": {"kind": ..., "from": ..., "to": ...}}`. Broadening
`Engine.write` to accept the erroneous helper shape would have introduced a
new compatibility shim unrelated to Slice 60.

The helper now emits the canonical nested write shape. Its seven call sites
span four test groups: depth-zero seed ordering; direction, edge-kind, and
target-kind behavior; the parameterized exact-W/W+1 work-bound test; and legacy
graph-verb compatibility. No call site, graph-expansion assertion, or public
intent changed.

The correction follows READY v5 **Test strategy** and **Direction and
edge/target-kind filters**: consumer tests insert feasible canonical edges, then
exercise the new read operation's direction and filtering behavior. It also
preserves the repository's established Python `Engine.write` interface instead
of adding a second input grammar.

## Frozen-file hash

| Path | Before SHA-256 | After SHA-256 |
| --- | --- | --- |
| `src/python/tests/test_slice60_graph_expand.py` | `9b242d351bba2d3cf55f3db382e8bd0682d678f6d3bc71ad04583d8d5b4022d5` | `fd07ff1d4032df961fc99b7e8df9af9cb5644877045e8620f6d6f4b5bc1cb934` |

This is the independently audited one-time exception for the Python oracle.
The shared fixture and the other five frozen Slice 60 test files were not
changed.
