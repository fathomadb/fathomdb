import unittest
from types import SimpleNamespace

from cpu_profile import finish_collector, recording_issues


class CollectorCleanupTests(unittest.TestCase):
    """Only lifecycle accounting and stopped-PID release ordering are exercised."""

    def test_terminal_wait_survives_stop_write_and_repeated_interrupts(self):
        for stop_write_fails in (True, False):
            with self.subTest(stop_write_fails=stop_write_fails):
                state = {'pumps': 0, 'terminal': False}
                collector = SimpleNamespace(poll=lambda: 0 if state['terminal'] else None)

                def request_stop():
                    if stop_write_fails:
                        raise BrokenPipeError('stop FIFO failed')

                def pump():
                    state['pumps'] += 1
                    if state['pumps'] < 3:
                        raise KeyboardInterrupt('authorization is still pending')
                    state['terminal'] = True

                issues = finish_collector(collector, request_stop, pump,
                                          lambda: state['terminal'])
                self.assertTrue(state['terminal'], 'stopped PID cannot be released before terminal')
                self.assertEqual(state['pumps'], 3)
                self.assertTrue(any('KeyboardInterrupt' in issue for issue in issues))
                if stop_write_fails:
                    self.assertTrue(any('BrokenPipeError' in issue for issue in issues))

    def test_unexpected_benchmark_exit_is_inconclusive_but_101_is_retained(self):
        for code in (0, 101):
            self.assertEqual(recording_issues({'stop_reason': None,
                                              'benchmark_exit': code, 'collector_exit': 0}), [])
        for code in (-9, 1):
            self.assertTrue(recording_issues({'stop_reason': None,
                                             'benchmark_exit': code, 'collector_exit': 0}))


if __name__ == '__main__':
    unittest.main()
