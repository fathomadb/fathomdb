"""Cancelled D finalization must preserve an existing unreadable/corrupt history."""

import json
from pathlib import Path
import tempfile
import unittest

from d_runner import _finalize_cancelled


class DCancelStateTests(unittest.TestCase):
    def test_existing_invalid_or_unreadable_state_retains_bytes_and_custody(self):
        for unreadable in (False, True):
            with self.subTest(unreadable=unreadable), tempfile.TemporaryDirectory() as directory:
                allocation = Path(directory)
                root = allocation/'d'
                root.mkdir()
                state = root/'d-state.json'
                original = b'{invalid original qualified/failed history' if not unreadable else b'{"qualified":{"status":"complete"}}'
                state.write_bytes(original)
                budget = {'schema': 'slice30-gd-budget/v1', 'used_ns': 100,
                    'inflight': {'cell': 'd-c-held-n10000-r0', 'status': 'guardian_stopped',
                                 'quiescence_known': True, 'accounted_through_monotonic_ns': 200},
                    'd_guardian': None, 'm_guardian': None, 'closed_guardian': None,
                    's50_guardian': None, 'cell_used_ns': {'d-c-held-n10000-r0': 70},
                    'common_used_ns': {'D': 30}}
                ledger = allocation/'g-budget.json'
                ledger.write_text(json.dumps(budget))
                before = ledger.read_bytes()
                guardian = {'boundary': {'removed_exact_empty_boundary': True},
                            'stopped_monotonic_ns': 200, 'controller_pid': 12345,
                            'cleanup_uncertain': False}
                if unreadable:
                    state.chmod(0)
                try:
                    with self.assertRaises((OSError, ValueError),
                        msg='cancellation replaced unreadable original history and cleared custody'):
                        _finalize_cancelled(root, allocation, guardian)
                finally:
                    state.chmod(0o600)
                self.assertEqual(state.read_bytes(), original)
                self.assertEqual(ledger.read_bytes(), before)


if __name__ == '__main__':
    unittest.main()
