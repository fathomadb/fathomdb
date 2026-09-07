# Slice 60 RED-oracle correction 7 — Python stub typing

Independent audit found seven Pyright `reportAttributeAccessIssue` diagnostics
in the frozen FIX-2 Python Unicode oracle. The test dynamically constructs
`types.ModuleType` doubles, so direct attribute assignment is valid at runtime
but unknown to static typing. This correction replaces only those assignments
with runtime-equivalent `setattr(module, name, value)` calls.

No assertion, assigned value, fixture byte, call ordering, or runtime behavior
changes. The correction makes the normal repository Pyright gate executable
without broad diagnostic suppression or product changes.

| Path | Old SHA-256 | New SHA-256 |
| --- | --- | --- |
| `src/python/tests/test_slice60_fix2_graph_expand.py` | `e8cc8e149c8478bb66d658115abfe173926aa32f276505e6215da742355fac52` | `0163d4ac4b8f6ccb9245ebe80320cc0f23fc5e4449a123ae2108e2308e75dd20` |

The isolated control `pyright -p src/python` reports 0 errors, and the corrected
Python FIX-2 transport test passes unchanged at runtime.
