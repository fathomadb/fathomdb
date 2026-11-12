"""Literal resource-budget and real small-child controls for G/D tooling."""

import unittest

from gd_resources import aggregate_roots


class AggregateTests(unittest.TestCase):
    def test_each_root_uses_its_larger_size_before_summing(self):
        roots = [{"allocated": 40, "apparent": 50},
                 {"allocated": 10, "apparent": 8}]
        self.assertEqual(aggregate_roots(roots), 60)


if __name__ == "__main__":
    unittest.main()
