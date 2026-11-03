"""Selection and carried-budget oracles for the reviewed continuation."""
import unittest
from campaign import cells
from supplement import pending_attempts, remaining_seconds, recovery_inputs


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

    def test_recovery_excludes_attempted_and_carries_prior_active_and_cell_time(self):
        attempts = [dict(cell=i, replica=r, **cell, disposition='not_run')
                    for i, cell in enumerate(cells()) for r in range(3)]
        original = dict(attempts=attempts, elapsed_s=2700.15)
        previous = dict(attempts=[dict(attempts[0], disposition='incomplete', elapsed_s=11.975),
                                  dict(attempts[3], disposition='complete', elapsed_s=16)], elapsed_s=908.8956)
        combined, budget = recovery_inputs(original, previous)
        self.assertAlmostEqual(budget, 591.1044)
        self.assertAlmostEqual(combined['elapsed_s'], 3609.0456)
        chosen = pending_attempts(combined, dict(arm_order=[['A', 'B'], ['B', 'A'], ['A', 'B']]))
        self.assertEqual(len(chosen), 70)
        self.assertNotIn((0, 0), [(a['cell'], a['replica']) for a in chosen])
        self.assertNotIn((1, 0), [(a['cell'], a['replica']) for a in chosen])
        self.assertAlmostEqual(remaining_seconds(combined, 0, 1000, 0), 30)
        self.assertEqual(original['attempts'][0]['disposition'], 'not_run', 'original evidence is immutable')


if __name__ == '__main__':
    unittest.main()
