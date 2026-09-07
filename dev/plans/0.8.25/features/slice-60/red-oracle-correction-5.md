# Slice 60 Python Unicode oracle correction

The independent FIX-2 audit found that the first Python assertion invoked the
stdlib `json.dumps` directly without `ensure_ascii=False`. That assertion could
only observe the stdlib default and could not exercise the binding transport
implemented by `graph.expand`.

The correction removes only that direct-stdlib assertion. The test retains its
executed `graph.expand` route, captured native request, byte-for-byte fixture
comparison, no-escape assertion, and native Unicode-result observation. No
contract intent changes.

| Path | Before SHA-256 | After SHA-256 |
| --- | --- | --- |
| `src/python/tests/test_slice60_fix2_graph_expand.py` | `4cd31bd65b15a8be7f28161084c107035c980304ac0f01fb4e48a2c114ec3378` | `e8cc8e149c8478bb66d658115abfe173926aa32f276505e6215da742355fac52` |
