# Slice 80.n pre-dispatch seal

The implementation source is `525919ec`; the campaign records
its final clean documentation HEAD in each raw identity marker. Product inputs
remain the protected `95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274`.

- Retained executable: `target/release/deps/perf_gates-e6869802f14984e9`.
- Fresh task-owned copy: `/tmp/fathomdb-slice80n-artifact/perf_gates`.
- Both digests: `ff4b78f36898f8109eef2dcb64f91a99741f9619433d77de0cfef0e59d51895d`.
- Test-list check: `ac_013_vector_retrieval_latency` appeared exactly once.
- Prebuilt runner SHA-256: `7b486ed2dfc5990085cf13af4d4382d411eb5ee8b5af58b7ac1721eded372124`.
- Shared competitor scanner SHA-256: `b213ef845247ae230ba846fb7ef9f4df0eac458fb117b6ee0c758def2de9b825`.
- Validator SHA-256: `88381db66229f2a20783a178b01dc21f69ce982fd7fa14b2acbca499077ffc40`.

The first smoke is retained at `raw/ac072-slice80n/smoke.log`. Its real
database run and collector records completed, but the original parser did not
recognize Rust's progress-line prefix on `AC013_NUMBERS`; it therefore has an
`INCOMPLETE` tooling verdict and is not acceptance evidence. Commit `525919ec`
adds the narrow parser repair and its RED/GREEN proof. The owner-approved
harness-fix smoke repeat remains available; no AC-072 acceptance cell has run.
- Three-cell dispatcher SHA-256: `9a33d09b437fb1cd0bacf132bc43963b21b95e42cd8566fb532c048579b2197e`.

The runner invokes the copied test executable directly with the sealed
selector and environment. It does not invoke Cargo, rustc, or `run-ac013.sh`
between start/end qualification snapshots.
