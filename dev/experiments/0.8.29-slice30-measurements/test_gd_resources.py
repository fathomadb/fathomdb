"""Literal resource-budget and real small-child controls for G/D tooling."""

import unittest

from gd_resources import aggregate_roots, setup_growth


class AggregateTests(unittest.TestCase):
    def test_each_root_uses_its_larger_size_before_summing(self):
        roots = [{"allocated": 40, "apparent": 50},
                 {"allocated": 10, "apparent": 8}]
        self.assertEqual(aggregate_roots(roots), 60)


class SetupGrowthTests(unittest.TestCase):
    def test_raw_subtraction_precedes_each_root_maximum(self):
        roots = [{"path":"p", "allocated":40, "apparent":50},
                 {"path":"task", "allocated":30, "apparent":38}]
        raw = [{"allocated":10, "apparent":20}]
        self.assertEqual(setup_growth(roots, "task", raw, 60), 10)


if __name__ == "__main__":
    unittest.main()
