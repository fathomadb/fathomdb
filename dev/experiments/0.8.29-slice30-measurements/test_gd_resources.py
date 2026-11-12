"""Literal resource-budget and real small-child controls for G/D tooling."""

import unittest

from gd_resources import aggregate_roots, reservation_ok, setup_growth


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


class ReservationTests(unittest.TestCase):
    def test_raw_and_unused_setup_reservations_cannot_be_consumed(self):
        gib = 1024**3
        self.assertFalse(reservation_ok(73*gib, 0, 0, 0))
        self.assertTrue(reservation_ok(64*gib, 0, 0, 0))
        self.assertTrue(reservation_ok(122*gib, 50*gib, 4*gib, 4*gib))
        self.assertFalse(reservation_ok(150*gib, 97*gib, 0, 0))
        self.assertFalse(reservation_ok(150*gib, 80*gib, 17*gib, 0))
        self.assertFalse(reservation_ok(200*gib, 96*gib, 16*gib, 16*gib))


if __name__ == "__main__":
    unittest.main()
