"""Actual host observation costs remain separate from native route latency."""

from pathlib import Path
import sys
import tempfile
import time
import unittest

from g_driver import run_program_step


class GClosedObservationCostTests(unittest.TestCase):
    def test_callback_wall_cpu_and_serialized_bytes_are_reported(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            command = [sys.executable, '-c', "\n".join([
                'import json,pathlib,sys,time',
                'o=pathlib.Path(sys.argv[2]);o.mkdir();time.sleep(.25)',
                "(o/'result.json').write_text(json.dumps(dict(complete=True,runner_only=True)))",
            ])]
            calls = []
            def observe(pid):
                time.sleep(.02)
                calls.append(pid)
                return {"pid": pid, "protocol_only": True}
            step = run_program_step(command, {"family": "G-W", "stage": "G-W", "kind": "write",
                "calibration": True}, root / 'result', time.monotonic_ns() + 5 * 10**9, observe)
            self.assertTrue(step['complete'])
            self.assertGreaterEqual(step.get('observation_wall_ns', 0), len(calls) * 20_000_000,
                                    'real blocking observation cost was not recorded')
            self.assertGreater(step['observation_parent_cpu_ns'], 0)
            self.assertGreater(step['observation_raw_bytes'], 0)
            self.assertEqual(step['observation_count'], len(calls))
            self.assertGreater(step['host_wall_ns'], step['observation_wall_ns'])
            self.assertEqual(step['observation_wait_ms'], 50)
