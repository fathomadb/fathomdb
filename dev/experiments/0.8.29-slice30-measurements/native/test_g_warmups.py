"""Manual G1 warmup calls/order distinct from timed queries in every existing API mode."""

import json
import unittest

from hypothesis import Phase, given, settings, strategies as st

from test_g_mean import read_jsonl_records
from test_g_query_feed import MODES, QUESTIONS, QueryFixture


class WarmupTests(QueryFixture):
    def require_warmups(self, count, *, all_subtests=False):
        self.extra_config = {"warmups": count}
        observations = self.fixture(QUESTIONS)
        root = self.fixture_root
        by_mode = {}
        for mode in MODES:
            directory = root / "out" / mode
            issues = []
            warmups = read_jsonl_records(directory / "warmups.jsonl", issues)
            if issues:
                raise RuntimeError("existing raw framing failed, NOT_RED")
            proof = json.loads((directory / "api-proof.json").read_text())
            by_mode[mode] = (warmups, proof)
        print(json.dumps({"root": str(root), "requested_warmups": count,
                          "actual_warmups": {mode: len(rows[0]) for mode, rows in by_mode.items()}}), flush=True)
        for mode in MODES:
            if all_subtests:
                with self.subTest(mode=mode):
                    self.require_mode(mode, count, observations, by_mode[mode])
            else:
                self.require_mode(mode, count, observations, by_mode[mode])

    def require_mode(self, mode, count, observations, pair):
        warmups, proof = pair
        self.assertEqual(len(warmups), count, "missing actual unscheduled G1 warmup calls")
        self.require_forwarding(observations, QUESTIONS, mode)
        expected = QUESTIONS[:count]
        self.require_forwarding({mode: warmups}, expected, mode)
        self.assertEqual([row["call_index"] for row in warmups], list(range(count)))
        self.assertEqual([row["sample_index"] for row in warmups], list(range(count)))
        self.assertEqual([row["call_index"] for row in observations[mode]], list(range(count, count + 3)))
        self.assertEqual([row["sample_index"] for row in observations[mode]], [0, 1, 2])
        self.assertEqual(proof["api_calls"], count + 3)
        self.assertEqual(proof["timed_api_calls"], 3)
        self.assertEqual(proof["warmup_api_calls"], count)
        for row in warmups:
            self.assertTrue(row["ok"])
            self.assertGreater(row["elapsed_ns"], 0)
            self.assertEqual(row["adapter"], "engine" if mode.startswith("engine_") else "sdk")
            population = "hybrid" if mode.endswith("_hybrid") else "direct_text" if mode.endswith("_text") else "mean_embed"
            self.assertEqual(row["mode"], population)
            if population == "mean_embed":
                self.assertEqual((row["dimension"], row["finite"]), (384, True))
                self.assertGreater(row["norm_squared"], 0)
            elif population == "hybrid":
                self.assertEqual((row["returned_hits"], row["soft_fallback"], row["degradations"]), (10, "None", "[]"))
            else:
                self.assertIsNone(row["kind_filter"])
        issues = []
        directory = self.fixture_root / "out" / mode
        for stream in ["batches", "marker-probes"]:
            self.assertEqual(read_jsonl_records(directory / f"{stream}.jsonl", issues), [])
        self.assertEqual(issues, [])
        self.assertEqual(proof["raw_persistence"]["records"], count + 3)
        self.assertEqual(proof["raw_persistence"]["checkpoints"], 2 if count else 1)

    def test_literal_two_warmups_precede_timed_population_in_all_six_modes(self):
        self.require_warmups(2, all_subtests=True)

    @settings(max_examples=3, deadline=None, derandomize=True, phases=[Phase.generate])
    @given(count=st.integers(min_value=1, max_value=3))
    def test_bounded_positive_warmup_counts_order_and_phase_separation(self, count):
        self.require_warmups(count)

    def test_zero_warmup_default_preserves_exact_timed_population(self):
        self.require_warmups(0)


if __name__ == "__main__":
    unittest.main()
