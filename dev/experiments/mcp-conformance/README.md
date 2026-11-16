---
title: MCP conformance probes
status: PROPOSED
desc: Reusable real-SDK fixture, official-suite and Inspector runner, direct reference-SDK session and raw stdio characterization probes.
---

# MCP conformance probes

These qualify the currently declared tools and record upstream behavior; they do
not turn every characterization into a compliance pass. Current results and
applicability are in the [0.8.29 report](../../design/mcp-adapter/conformance-0.8.29.md).
Run against an immutable production subject, with an owned external result/tool
directory and no concurrent opener of its fixture database.

1. Select Node 25.9.0 on PATH and install owned npm 11.12.1, then install exact
   `@modelcontextprotocol/conformance@0.1.16`,
   `@modelcontextprotocol/inspector@2.11.0`, `supergateway@4.2.0` and
   `@modelcontextprotocol/sdk@1.32.1` under `$RUN/tools`. Retain the lock/log.
2. Compile `fixture.rs` with the same compiler and exact production SDK closure
   as the MCP subject. It accepts one new database path and refuses overwrite.
   The observed run reused the existing provider-free SDK rlib read-only:

   ```sh
   rustc --edition=2021 dev/experiments/mcp-conformance/fixture.rs \
     --extern "fathomdb_sdk=$SDK_RLIB" -L "dependency=$SDK_DEPS" \
     -o "$RUN/fixture"
   "$RUN/fixture" "$RUN/demo.sqlite"
   ```

3. Run the pinned official all-scenarios suite through loopback-only bridges,
   followed by direct Inspector calls. It writes text/structured configs, all
   raw suite results and each Inspector stdout/stderr/exit. It binds ports 18765
   and 18766 and cleans only its own bridge process groups. Existing suite
   failures are retained without an expected-failures baseline.

   ```sh
   python3 dev/experiments/mcp-conformance/run.py "$BIN" "$RUN"
   ```

4. Run the direct reference-SDK session and raw lifecycle characterizations:

   ```sh
   node dev/experiments/mcp-conformance/session-check.mjs \
     "$RUN/tools" "$BIN" "$RUN"
   python3 dev/experiments/mcp-conformance/raw-probe.py "$BIN" "$RUN"
   ```

The reference SDK validates structured outputs against discovery on actual
calls. Compact checks inspect actual decoded text and final UTF-8 budget counts.
The manual fixture expectations cover known selected IDs, flag counts and the
real edge. Raw probes retain each sent/received line and stderr; assertions on
known upstream discrepancies only confirm characterization, not compliance.
The suite's reference-only tools are not added to the product to make it green.
