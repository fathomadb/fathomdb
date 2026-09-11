# Slice 80.n pre-dispatch seal

The implementation source is `aa2bb4f5`; the campaign records
its final clean documentation HEAD in each raw identity marker. Product inputs
remain the protected `95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274`.

- Retained executable: `target/release/deps/perf_gates-e6869802f14984e9`.
- Fresh task-owned copy: `/tmp/fathomdb-slice80n-artifact/perf_gates`.
- Both digests: `ff4b78f36898f8109eef2dcb64f91a99741f9619433d77de0cfef0e59d51895d`.
- Test-list check: `ac_013_vector_retrieval_latency` appeared exactly once.
- Prebuilt runner SHA-256: `7b486ed2dfc5990085cf13af4d4382d411eb5ee8b5af58b7ac1721eded372124`.
- Shared competitor scanner SHA-256: `b213ef845247ae230ba846fb7ef9f4df0eac458fb117b6ee0c758def2de9b825`.
- Validator SHA-256: `31a84b60d9ed656723358cb653afa0c1d7648e86e2b4b2220d473eaec9b17a2b`.

The first smoke is retained at `raw/ac072-slice80n/smoke.log`. Its real
database run and collector records completed, but the original parser did not
recognize Rust's progress-line prefix on `AC013_NUMBERS`; it therefore has an
`INCOMPLETE` tooling verdict and is not acceptance evidence. Commits `525919ec`
and `62b2c6cc` add the narrow parser repairs and RED/GREEN proof. The repaired
validator now accepts the retained receipt, which is qualified and numerically
passing as a smoke only. The owner-approved harness-fix smoke repeat remains
available; no AC-072 acceptance cell has run.
- Three-cell dispatcher SHA-256: `f25401004b937e03ef04e12e54e9c0feae4d69456e4f39249e8c4a3589c60af6`.

The runner invokes the copied test executable directly with the sealed
selector and environment. It does not invoke Cargo, rustc, or `run-ac013.sh`
between start/end qualification snapshots.
