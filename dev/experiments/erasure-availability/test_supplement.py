"""Selection and carried-budget oracles for the reviewed continuation."""
import unittest
from campaign import cells
from supplement import pending_attempts, remaining_seconds


class Continuation(unittest.TestCase):
    def test_only_never_started_preserves_alternating_order(self):
        attempts = [dict(cell=i, replica=r, **cell, disposition='not_run')
                    for i, cell in enumerate(cells()) for r in range(3)]
        attempts[0].update(disposition='complete', elapsed_s=16)
        attempts[3].update(disposition='incomplete', elapsed_s=30.1)
        original = dict(attempts=attempts)
        manifest = dict(arm_order=[['A', 'B'], ['B', 'A'], ['A', 'B']])
        chosen = pending_attempts(original, manifest)
        self.assertEqual(len(chosen), 70)
        self.assertEqual([(a['cell'], a['replica']) for a in chosen[:4]],
                         [(1, 1), (0, 1), (0, 2), (1, 2)])
        self.assertTrue(all(a['disposition'] == 'not_run' for a in chosen))

    def test_original_spent_and_global_stop_are_not_reset(self):
        original = dict(attempts=[dict(cell=7, disposition='complete', elapsed_s=35),
                                  dict(cell=7, disposition='incomplete', elapsed_s=30.2),
                                  dict(cell=7, disposition='not_run')])
        self.assertAlmostEqual(remaining_seconds(original, 7, 100, 0), 24.8)
        self.assertEqual(remaining_seconds(original, 7, 10, 0), 10)
        self.assertEqual(remaining_seconds(original, 7, 10, 11), 0)


if __name__ == '__main__':
    unittest.main()
