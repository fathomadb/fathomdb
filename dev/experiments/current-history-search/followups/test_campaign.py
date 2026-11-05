"""Controller tests use deterministic clocks/counters, never mocked databases."""
import unittest
import campaign


class Controls(unittest.TestCase):
    def test_fixed_grid_and_exact_order(self):
        cells = campaign.grid()
        self.assertEqual(len(cells), 252)
        self.assertEqual(len(set(cells)), 252)
        self.assertEqual(cells[:3], [(0, 128, 1, "current", f) for f in ["node", "edge", "property"]])
        self.assertEqual(cells[-1], (16, 4096, 8, "both", "property"))

    def test_stress_order_preserves_matched_controls_and_combined_budget(self):
        cells = campaign.stress_grid()
        self.assertEqual(len(cells), 18)
        expected = [(h, 4096, callers, view, family)
                    for h, callers, view in [(16, 8, "both"), (16, 1, "both"),
                                             (0, 1, "both"), (4, 1, "both"),
                                             (16, 1, "corrected"), (16, 1, "prior")]
                    for family in ["node", "edge", "property"]]
        self.assertEqual(cells, expected)
        self.assertEqual(campaign.stop_reason(1, 560 + 6640, 0), "campaign_time")

    def test_budgets_and_nonreplacement_samples(self):
        self.assertEqual(campaign.stop_reason(600, 600, 0), "cell_time")
        self.assertEqual(campaign.stop_reason(1, 7200, 0), "campaign_time")
        self.assertEqual(campaign.stop_reason(1, 1, 8 * 1024**3), "rss")
        failed = campaign.summary([{"status": "success", "ms": 1.}] * 999 + [{"status": "error", "ms": 2.}], None)
        self.assertFalse(failed["qualified"])
        self.assertIsNone(failed["p99_ms"])
        self.assertEqual(failed["success"], 999)
        self.assertEqual(failed["error"], 1)
        self.assertEqual(failed["unfinished"], 0)
        partial = campaign.summary([{"status": "success", "ms": 1.}] * 7, "rss")
        self.assertEqual(partial["unfinished"], 993)
        self.assertFalse(partial["qualified"])
        full = campaign.summary([{"status": "success", "ms": float(i)} for i in range(1000)], None)
        self.assertTrue(full["qualified"])
        self.assertEqual(full["p99_ms"], 989.)


if __name__ == "__main__":
    unittest.main()
