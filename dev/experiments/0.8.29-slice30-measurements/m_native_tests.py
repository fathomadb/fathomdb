"""Owned actual CLI/Adapter/SDK boundaries; explicitly selected native source tests."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from contextlib import contextmanager

from m_client import RpcChild
from m_lifecycle import alternating, first_calls, idle_cycle
from m_diagnostics import PrivateControl, run_case
from m_session import McpSession
from m_wire import decode_tool
from m_cell import execute
from m_controller import load_plan


class NativeCase(unittest.TestCase):
    """Create fresh real closed SDK stores; retain every native source-test attempt."""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="m-source-", dir=os.environ["TMPDIR"]))
        self.cli = Path(os.environ["FATHOMDB_SLICE30_M_CLI"]).resolve(strict=True)
        self.helper = Path(os.environ["FATHOMDB_SLICE30_M_HELPER"]).resolve(strict=True)
        self.stores = self.root / "stores"
        seeded = subprocess.run([str(self.helper), "--seed-fixture", str(self.stores)],
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=15)
        (self.root / "seed.stdout").write_bytes(seeded.stdout)
        (self.root / "seed.stderr").write_bytes(seeded.stderr)
        self.assertEqual(seeded.returncode, 0, seeded.stderr.decode())

    def config(self, result_format="text", **extra):
        """Write only existing provider-free configuration keys and selected literal stores."""
        value = {"stores": {name: {"path": str(self.stores / f"{name}.sqlite")}
                             for name in ("a", "b")}, "default_store": "a",
                 "use_default_embedder": False, "search_mode": "text",
                 "result_format": result_format, **extra}
        path = self.root / f"config-{result_format}.json"
        path.write_text(json.dumps(value))
        return path

    def initialize(self, client):
        """Use the existing protocol handshake and verify unchanged seven-tool discovery."""
        request_id = client.submit("initialize", {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "slice30-m-source-tests", "version": "0"}}, phase="control")
        self.assertIn("result", client.wait(request_id))
        client.notify("notifications/initialized", {})
        request_id = client.submit("tools/list", {}, phase="control")
        tools = client.wait(request_id)["result"]["tools"]
        self.assertEqual(sorted(tool["name"] for tool in tools),
                         ["expand", "feedback", "fetch", "list_stores", "orient", "search", "use_store"])

    def call(self, client, name, arguments):
        """Record one actual wire call and return its decoded outcome."""
        request_id = client.submit("tools/call", {"name": name, "arguments": arguments},
                                   phase="control", tool=name)
        return decode_tool(client.wait(request_id))

    @contextmanager
    def helper_session(self, result_format):
        """Serve the real Adapter and arm only its private existing Seams channel."""
        control_path = self.root / f"c-{result_format[0]}.sock"
        command = [str(self.helper), "--config", str(self.config(result_format)),
                   "--control-socket", str(control_path)]
        with McpSession(command, self.root / f"helper-{result_format}",
                        result_format=result_format) as session:
            with PrivateControl(control_path, session.root) as control:
                session.initialize()
                yield session, control
            self.assertEqual(control.failures, [])
        self.assertEqual(session.exit["code"], 0)
        self.assertTrue(session.exit["readers_joined"])
        self.assertEqual(session.failures, [])


class CompactSearchTests(NativeCase):
    """The compact-default normal CLI must not become an empty successful result."""

    def test_actual_compact_cli_retains_literal_hit_and_envelope(self):
        with RpcChild([str(self.cli), "--config", str(self.config())],
                      self.root / "cli") as client:
            self.initialize(client)
            result = self.call(client, "search", {"query": "alpha", "limit": 10})
            payload = result["payload"]
            self.assertEqual([row["ref"] for row in payload.get("results", [])], ["l:alpha"],
                             f"actual compact hit was lost; raw={result['text']!r}; retained={self.root}")
            self.assertTrue(result["ok"])
            self.assertEqual(payload["store"], "a")
            self.assertEqual(payload["epoch"], 1)
            self.assertEqual(payload["mode"], "text")
            self.assertEqual(payload["degradation"], [])
            self.assertEqual(payload["budget"]["max_tokens"], 4000)
            self.assertTrue(payload["response_id"].startswith("r1-"))
        self.assertEqual(client.exit["code"], 0)
        self.assertTrue(client.exit["readers_joined"])
        self.assertEqual(client.failures, [])


class PublicCellTests(NativeCase):
    """The registered M0 path executes its whole literal real-process contract."""

    def test_m0_binding_retains_literal_steps_in_both_formats(self):
        for cell in (row for row in load_plan() if row["family"] == "M0"):
            with self.subTest(format=cell["result_format"]):
                result = execute(cell, [str(self.cli), "--config",
                                 str(self.config(cell["result_format"]))],
                                 self.root / cell["id"], queries=["alpha"],
                                 deadline_ns=time.monotonic_ns() + 30 * 10**9)
                self.assertEqual(result["events"], ["search", "fetch", "orient", "typed_error"])
                self.assertEqual(result["literal_refs"], ["l:alpha"])
                self.assertEqual(result["fetch_body"], "alpha river crossing\né😀")
                self.assertEqual(result["typed_error"], "invalid_request/unknown_store")
                self.assertTrue(result["complete"])

    def test_committed_public_command_reports_both_real_m0_cells(self):
        child = subprocess.run([sys.executable, str(Path(__file__).with_name("m_runner.py")),
            "source-smoke", "--root", str(self.root / "public-entry"),
            "--cli", str(self.cli), "--helper", str(self.helper)], capture_output=True, timeout=30)
        self.assertEqual(child.returncode, 0, child.stderr.decode())
        report = json.loads(child.stdout)
        self.assertEqual([row["id"] for row in report["rows"]], ["M0-text", "M0-structured"])
        self.assertTrue(all(row["complete"] for row in report["rows"]))
        self.assertFalse(report["qualifies_natural"])


class PublicThroughputTests(NativeCase):
    """A tiny provider-free store exercises exact phase counts through the cell binding."""

    def test_registered_loop_binding_keeps_first_warm_timed_populations(self):
        cell = next(row for row in load_plan() if row["id"] == "M-R-1-4-warn")
        cell = dict(cell, search_mode="text", use_default_embedder=False)
        root = self.root / "source-loop"
        result = execute(cell, [str(self.cli), "--config", str(self.config())], root,
                         queries=["alpha"] * 500, deadline_ns=time.monotonic_ns() + 60 * 10**9)
        rows = [json.loads(line) for line in (root / "outcomes.jsonl").read_text().splitlines()]
        self.assertEqual([sum(row.get("phase") == phase for row in rows)
                          for phase in ("first", "warmup", "timed")], [1, 500, 1000])
        for phase, count in (("warmup", 500), ("timed", 1000)):
            selected = sorted((row for row in rows if row.get("phase") == phase),
                              key=lambda row: row["rpc_id"])
            self.assertEqual([row["query_index"] for row in selected],
                             [index % 500 for index in range(count)])
        self.assertTrue(result["complete"])


class PublicDegradedLoopTests(NativeCase):
    """An actual provider-free hybrid response cannot qualify the natural loop binding."""

    def test_unexpected_actual_degradation_keeps_public_loop_incomplete(self):
        cell = next(row for row in load_plan() if row["id"] == "M-R-1-1-warn")
        cell = dict(cell, use_default_embedder=False, first=1, warmups=1, timed=1)
        root = self.root / "degraded-public"
        result = execute(cell, [str(self.cli), "--config", str(self.config())], root,
                         queries=["alpha"], deadline_ns=time.monotonic_ns() + 30 * 10**9)
        rows = [json.loads(line) for line in (root / "outcomes.jsonl").read_text().splitlines()]
        searches = [row for row in rows if row["request"]["params"].get("name") == "search"]
        self.assertEqual(searches[0]["payload"]["degradation"], [
            {"arm": "vector", "reason": "embedder_absent"},
            {"arm": "vector", "reason": "embedder_unavailable"}])
        self.assertFalse(result["complete"], "actual degraded search was treated as qualified work")
        self.assertTrue(result["unexpected_degradation"])


class PublicControlRoutesTests(NativeCase):
    """Lifecycle and private diagnostics use the same registered cell execution path."""

    def test_actual_lifecycle_is_bound_to_registered_family(self):
        lazy = next(row for row in load_plan() if row["family"] == "M-L" and row["case"] == "lazy")
        lazy = dict(lazy, search_mode="text", use_default_embedder=False)
        result = execute(lazy, [str(self.cli), "--config", str(self.config())],
                         self.root / "lazy-public", queries=["alpha"],
                         deadline_ns=time.monotonic_ns() + 30 * 10**9)
        self.assertEqual([row["event"] for row in result.get("lifecycle", {}).get("events", [])],
                         ["before", "first", "immediate"])
        self.assertTrue(result["complete"])

    def test_actual_helper_is_bound_to_registered_family(self):
        diagnostic = next(row for row in load_plan() if row["family"] == "M-E"
                          and row["case"] == "queue_full" and row["result_format"] == "text")
        control = self.root / "public.sock"
        result = execute(diagnostic, [str(self.helper), "--config", str(self.config()),
                         "--control-socket", str(control)], self.root / "diag-public",
                         queries=["alpha"], control_path=control,
                         deadline_ns=time.monotonic_ns() + 30 * 10**9)
        self.assertEqual(result.get("diagnostic", {}).get("case"), "queue_full")
        self.assertTrue(result["complete"])


class TypedErrorTests(NativeCase):
    """Both actual wire formats retain the typed error and its complete remedy."""

    def test_actual_unknown_store_keeps_error_words_candidates_and_label(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    result = self.call(client, "search", {"store": "aa", "query": "alpha"})
                    error = result["payload"].get("error", {})
                    self.assertEqual(error.get("class"), "invalid_request",
                                     f"actual typed error was lost: {result}; retained={self.root}")
                    self.assertEqual(error["code"], "unknown_store")
                    self.assertEqual(error["candidates"], ["a", "b"])
                    self.assertEqual(error["message"], 'store "aa" is not addressable')
                    self.assertEqual(error["remedy"], "call list_stores and use one of the listed names")
                    self.assertFalse(result["ok"])
                    self.assertEqual(result.get("error"), "invalid_request/unknown_store")
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])


class FetchTests(NativeCase):
    """Actual fetch preserves UTF8 byte frames, duplicate order and item errors."""

    def test_actual_fetch_retains_exact_body_and_each_ordered_outcome(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    result = self.call(client, "fetch", {"refs": ["l:alpha", "l:missing", "l:alpha"]})
                    items = result["payload"].get("items", [])
                    self.assertEqual(len(items), 3,
                                     f"actual fetch frames were lost: {result}; retained={self.root}")
                    self.assertEqual([item["ref"] for item in items],
                                     ["l:alpha", "l:missing", "l:alpha"])
                    self.assertEqual(items[0]["record"],
                                     {"kind": "note", "body": "alpha river crossing\né😀",
                                      "body_cut": False})
                    self.assertEqual(items[2], items[0])
                    self.assertEqual(items[1]["error"]["class"], "not_found")
                    self.assertEqual(items[1]["error"]["code"], "not_found")
                    self.assertIn("remedy", items[1]["error"])
                    self.assertTrue(result["ok"])
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])


class ListStateTests(NativeCase):
    """Lifecycle observations come from actual list_stores without touching engines."""

    def test_actual_list_retains_closed_and_ready_states(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    before = self.call(client, "list_stores", {})["payload"]
                    self.assertEqual(before.get("stores", []), [
                        {"name": "a", "open": False, "readiness": "closed", "default": True},
                        {"name": "b", "open": False, "readiness": "closed", "default": False}],
                        f"actual initial listing was lost; retained={self.root}")
                    self.call(client, "search", {"store": "a", "query": "alpha"})
                    after = self.call(client, "list_stores", {})["payload"]
                    self.assertEqual(after["stores"], [
                        {"name": "a", "open": True, "readiness": "ready", "default": True},
                        {"name": "b", "open": False, "readiness": "closed", "default": False}])
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])


class OrientTests(NativeCase):
    """Actual orient readiness is retained without inventing provider availability."""

    def test_actual_orient_retains_provider_free_runtime_and_embedding_facts(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    result = self.call(client, "orient", {})
                    payload = result["payload"]
                    status = payload.get("projection_status", {})
                    self.assertEqual(status.get("runtime_embedder_available"), False,
                                     f"actual orient readiness was lost: {result}; retained={self.root}")
                    self.assertEqual(status["runtime_unavailability_reason"], "no_runtime")
                    self.assertEqual(status["vector_unsupported_kinds"], [])
                    self.assertEqual(payload["embedding_readiness"]["usable_embedder"], False)
                    self.assertEqual(payload["embedding_readiness"]["pending_count"], 0)
                    self.assertEqual(payload["store"], "a")
                    self.assertEqual(payload["epoch"], 1)
                    self.assertEqual(payload["budget"]["max_tokens"], 4000)
                self.assertEqual(client.exit["code"], 0)
                self.assertTrue(client.exit["readers_joined"])


class FirstLifecycleTests(NativeCase):
    """Actual first/immediate calls retain their lifecycle and inclusive client clocks."""

    def test_actual_lazy_first_process_records_listing_and_immediate_call(self):
        with McpSession([str(self.cli), "--config", str(self.config())],
                        self.root / "lazy") as session:
            session.initialize()
            result = first_calls(session, "alpha", mode="text")
            self.assertEqual([event["event"] for event in result.get("events", [])],
                             ["before", "first", "immediate"],
                             f"actual lazy lifecycle records absent; retained={self.root}")
            self.assertEqual(result["events"][0]["payload"]["stores"][0]["readiness"], "closed")
            for event in result["events"][1:]:
                self.assertEqual(event["payload"]["results"][0]["ref"], "l:alpha")
                self.assertEqual(event["payload"]["epoch"], 1)
            self.assertEqual(result["first_call_ns"], result["events"][1]["elapsed_ns"])
            self.assertEqual(result["spawn_to_first_ns"],
                             result["events"][1]["arrival_ns"] - session.started_ns)
            self.assertGreater(result["spawn_to_handshake_ns"], 0)
            self.assertLessEqual(session.handshake_complete_ns,
                                 result["events"][1]["send_start_ns"])
        self.assertEqual(session.exit["code"], 0)
        self.assertTrue(session.exit["readers_joined"])

    def test_actual_eager_reuses_first_call_recorder_without_exclusive_open_claim(self):
        config = self.config()
        value = json.loads(config.read_text())
        value["stores"]["a"]["eager"] = True
        config.write_text(json.dumps(value))
        with McpSession([str(self.cli), "--config", str(config)],
                        self.root / "eager") as session:
            session.initialize()
            result = first_calls(session, "alpha", mode="text")
            self.assertTrue(result["complete"])
            self.assertIn(result["events"][0]["payload"]["stores"][0]["readiness"],
                          ("opening", "ready"))
            self.assertEqual([row["payload"]["epoch"] for row in result["events"][1:]], [1, 1])
        self.assertEqual(session.exit["code"], 0)


class AlternatingLifecycleTests(NativeCase):
    """Actual A/B/A observes states, epochs and distinct literal routing fixtures."""

    def test_actual_a_b_a_retains_closed_ready_states_and_next_epoch(self):
        config = self.config()
        with McpSession([str(self.cli), "--config", str(config)],
                        self.root / "alternating") as session:
            session.initialize()
            result = alternating(session, "river", mode="text",
                                 files={name: self.stores / f"{name}.sqlite" for name in ("a", "b")})
            events = result.get("events", [])
            self.assertEqual([(row["event"], row["store"]) for row in events],
                             [("search","a"),("states","a"),("search","b"),
                              ("states","b"),("search","a"),("states","a")],
                             f"actual alternating lifecycle records absent; retained={self.root}")
            searches = events[::2]
            self.assertEqual([row["payload"]["results"][0]["ref"] for row in searches],
                             ["l:alpha", "l:beta", "l:alpha"])
            self.assertEqual([row["payload"]["epoch"] for row in searches], [1, 1, 2])
            for row in events[1::2]:
                states = {entry["name"]: entry for entry in row["payload"]["stores"]}
                other = "b" if row["store"] == "a" else "a"
                self.assertEqual(states[row["store"]]["readiness"], "ready")
                self.assertEqual(states[other]["readiness"], "closed")
            self.assertNotEqual(result["files"]["a"]["inode"], result["files"]["b"]["inode"])
        self.assertEqual(session.exit["code"], 0)
        self.assertTrue(session.exit["readers_joined"])
        with McpSession([str(self.cli), "--config", str(config)],
                        self.root / "released-lock") as later:
            later.initialize()
            reopened = later.tool("search", {"store": "a", "query": "alpha", "mode": "text"})
            self.assertEqual(reopened["payload"]["results"][0]["ref"], "l:alpha")
        self.assertEqual(later.exit["code"], 0)


class IdleLifecycleTests(NativeCase):
    """Actual default60s eviction must be observed before calling it a reopen."""

    def test_actual_default_idle_close_precedes_epoch2_reopen(self):
        with McpSession([str(self.cli), "--config", str(self.config())],
                        self.root / "idle") as session:
            session.initialize()
            result = idle_cycle(session, "alpha", mode="text",
                                deadline_ns=time.monotonic_ns() + 70 * 10**9)
            events = result.get("events", [])
            self.assertTrue(any(row["event"] == "idle_closed" for row in events),
                            f"no actual closed observation retained; retained={self.root}")
            closed = next(row for row in events if row["event"] == "idle_closed")
            self.assertEqual(closed["payload"]["stores"][0]["readiness"], "closed")
            searches = [row for row in events if row["event"] in ("first", "reopened", "immediate")]
            self.assertEqual([row["payload"]["results"][0]["ref"] for row in searches],
                             ["l:alpha", "l:alpha", "l:alpha"])
            self.assertEqual([row["payload"]["epoch"] for row in searches], [1, 2, 2])
            self.assertLess(closed["arrival_ns"], searches[1]["send_start_ns"])
            self.assertEqual(result["poll_interval_ms"], 250)
            self.assertTrue(result["complete"])
        self.assertEqual(session.exit["code"], 0)
        self.assertTrue(session.exit["readers_joined"])


class ProviderWordsTests(NativeCase):
    """Existing explicit provider absence remains distinct from unavailable epoch metadata."""

    def test_actual_none_provider_keeps_both_degradations_and_hint_cause(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with RpcChild([str(self.cli), "--config", str(self.config(result_format))],
                              self.root / result_format) as client:
                    self.initialize(client)
                    result = self.call(client, "search", {"query": "alpha", "mode": "hybrid"})
                    payload = result["payload"]
                    self.assertTrue(result["ok"])
                    self.assertEqual(payload["degradation"], [
                        {"arm": "vector", "reason": "embedder_absent"},
                        {"arm": "vector", "reason": "embedder_unavailable"}])
                    self.assertEqual(payload["completeness"], "partial")
                    self.assertEqual(payload.get("hints", [])[0].get("cause"), "embedder_absent",
                                     f"actual degraded hint cause was lost; retained={self.root}")
                    self.assertEqual(payload["hints"][0], {"action": "none", "reason": "degraded",
                                                        "arm": "vector", "cause": "embedder_absent"})
                self.assertEqual(client.exit["code"], 0)


class QueueFullTests(NativeCase):
    """Actual cap4 plus32 observed waiters refuses the next call in both formats."""

    def test_actual_queue_full_keeps_busy_fields_and_all_invocations(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with self.helper_session(result_format) as (session, control):
                    result = run_case("queue_full", session, control)
                    self.assertEqual(result.get("error"), "busy/admission/queue_full",
                                     f"actual queue_full disposition absent; retained={self.root}")
                    busy = result["payload"]["error"]["busy"]
                    self.assertEqual((busy["in_flight"], busy["queued"], busy["abandoned"]), (4, 32, 0))
                    self.assertEqual(result["diagnostic"]["held_before_release"], 4)
                    self.assertEqual(result["diagnostic"]["held_after_release"], 0)
                    self.assertEqual(sum(row["request"]["method"] == "tools/call"
                                         for row in session.records.values()), 38)
                    self.assertTrue(all(row["status"] == "completed" for row in session.records.values()))


class QueuedCancelTests(NativeCase):
    """Actual queued cancellation retains its flag and observed counter removal."""

    def test_actual_queued_cancel_keeps_disposition_and_observed_queue_removal(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with self.helper_session(result_format) as (session, control):
                    result = run_case("queued_cancel", session, control)
                    self.assertTrue(result.get("cancellation_requested"),
                                    f"actual queued cancel was not requested; retained={self.root}")
                    if "response" in result:
                        self.assertEqual(result.get("error"), "timeout/cancelled")
                    else:
                        self.assertEqual(result["status"], "unresolved")
                    facts = result["diagnostic"]
                    self.assertEqual((facts["queued_before_cancel"], facts["queued_after_cancel"]), (2, 1))
                    self.assertEqual((facts["held_before_release"], facts["held_after_release"]), (4, 0))
                    witness = session.records[facts["witness_rpc_id"]]
                    self.assertTrue(witness["ok"])
                    self.assertEqual(witness["payload"]["results"][0]["ref"], "l:alpha")


class WaitDeadlineTests(NativeCase):
    """Actual admission wait expires while the four running closures remain held."""

    def test_actual_default_wait_deadline_keeps_busy_load_and_live_holds(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with self.helper_session(result_format) as (session, control):
                    result = run_case("wait_deadline", session, control)
                    self.assertEqual(result.get("error"), "busy/admission/deadline",
                                     f"actual wait-deadline outcome absent; retained={self.root}")
                    busy = result["payload"]["error"]["busy"]
                    self.assertEqual((busy["in_flight"], busy["queued"], busy["abandoned"]), (4, 0, 0))
                    self.assertEqual((result["diagnostic"]["held_before_release"],
                                      result["diagnostic"]["held_after_release"]), (4, 0))
                    self.assertTrue(result["diagnostic"]["complete"])


class RunningCancelTests(NativeCase):
    """Actual running cancellation abandons work without freeing its admission permit."""

    def test_actual_running_cancel_remains_abandoned_until_explicit_release(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with self.helper_session(result_format) as (session, control):
                    result = run_case("running_cancel", session, control)
                    self.assertTrue(result.get("cancellation_requested"),
                                    f"actual running cancel was not requested; retained={self.root}")
                    if "response" in result:
                        self.assertEqual(result.get("error"), "timeout/cancelled")
                    else:
                        self.assertEqual(result["status"], "unresolved")
                    facts = result["diagnostic"]
                    busy = facts["probe"]["payload"]["error"]["busy"]
                    self.assertEqual((busy["in_flight"], busy["abandoned"]), (4, 1))
                    self.assertEqual((facts["held_before_release"], facts["held_after_release"]), (4, 0))
                    self.assertEqual(facts["recovery_held"], 4)
                    self.assertTrue(all(session.records[item]["ok"] for item in facts["recovery_rpc_ids"]))
                    self.assertTrue(facts["complete"])


class RunLimitTests(NativeCase):
    """Actual default30s run-limit replies abandon all four still-held closures."""

    def test_actual_run_limit_retains_timeout_held_permits_and_post_release_recovery(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with self.helper_session(result_format) as (session, control):
                    result = run_case("run_limit", session, control)
                    self.assertEqual(result.get("error"), "timeout/deadline",
                                     f"actual run-limit disposition absent; retained={self.root}")
                    facts = result["diagnostic"]
                    busy = facts["probe"]["payload"]["error"]["busy"]
                    self.assertEqual((busy["in_flight"], busy["abandoned"]), (4, 4))
                    self.assertEqual((facts["held_before_release"], facts["held_after_release"]), (4, 0))
                    self.assertEqual([session.records[item]["error"] for item in facts["original_rpc_ids"]],
                                     ["timeout/deadline"] * 4)
                    self.assertEqual(facts["recovery_held"], 4)
                    self.assertTrue(all(session.records[item]["ok"] for item in facts["recovery_rpc_ids"]))
                    self.assertTrue(facts["complete"])


class OpenDeadlineTests(NativeCase):
    """Actual opening deadline remains not_ready/opening, distinct from admission timeout."""

    def test_actual_open_deadline_keeps_phase_and_later_ready_engine(self):
        for result_format in ("text", "structured"):
            with self.subTest(result_format=result_format):
                with self.helper_session(result_format) as (session, control):
                    result = run_case("open_deadline", session, control)
                    self.assertEqual(result.get("error"), "not_ready/opening",
                                     f"actual opening deadline was lost; retained={self.root}")
                    facts = result["diagnostic"]
                    self.assertEqual(facts["opening"]["payload"]["stores"][0]["readiness"], "opening")
                    self.assertEqual((facts["held_before_release"], facts["held_after_release"]), (0, 0))
                    self.assertEqual(facts["recovered"]["payload"]["epoch"], 1)
                    self.assertEqual(facts["recovered"]["payload"]["results"][0]["ref"], "l:alpha")
                    self.assertTrue(facts["complete"])


if __name__ == "__main__":
    unittest.main()
