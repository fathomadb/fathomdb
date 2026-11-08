import os
import signal
import tempfile
import threading
import time
import unittest
from pathlib import Path

from cpu_profile import analyze, enable_then_continue, start_stopped_benchmark


def samples(count, frames, comm='ac_011a_write_t', tid=42):
    return ''.join(
        f'{comm} 40/{tid} {i}.000000: cpu-clock:u:\n'
        + ''.join(f'  abc {frame} (/fixture/perf_gates)\n' for frame in frames)
        + '\n' for i in range(count)
    )


class AttributionTests(unittest.TestCase):
    """Hand-computed exclusive sample attribution and agreed threshold checks."""

    def test_innermost_anchor_has_one_family_and_workers_are_separate(self):
        text = samples(200, ['malloc', 'sqlite3Prepare',
                             'fathomdb_engine::physical_erasure::root'])
        text += samples(200, ['memcpy', 'fts5IndexFlush', 'sqlite3Prepare'])
        text += samples(100, ['fathomdb_engine::physical_erasure::root'])
        text += samples(500, ['other_leaf'])
        text += samples(50, ['sqlite3Prepare'], 'worker', 43)
        report = analyze(text)
        self.assertEqual(report['foreground_samples'], 1000)
        self.assertEqual(report['exclusive_families'], {
            'compilation': 200, 'fts_flush_merge': 200, 'engine_guard': 100,
            'other': 500, 'unclassified': 0,
        })
        self.assertEqual(report['worker_samples'], {'43': 50})
        self.assertEqual(set(report['candidate_families']), {'compilation', 'fts_flush_merge'})
        self.assertEqual(report['decision'], 'supports_focused_design_only')
        self.assertTrue(report['top_callchains'])

    def test_symbol_coverage_and_both_candidate_thresholds(self):
        short = analyze(samples(100, ['sqlite3Prepare']) + samples(400, ['other']))
        self.assertEqual(short['candidate_families'], [])
        exact = analyze(samples(200, ['sqlite3Prepare']) + samples(750, ['other'])
                        + samples(50, ['[unknown]']))
        self.assertEqual(exact['known_leaf_samples'], 950)
        self.assertEqual(exact['candidate_families'], ['compilation'])
        self.assertEqual(exact['decision'], 'supports_focused_design_only')
        below = analyze(samples(200, ['sqlite3Prepare']) + samples(749, ['other'])
                        + samples(51, ['[unknown]']))
        self.assertEqual(below['decision'], 'inconclusive')

    def test_unknown_boundaries_and_explicit_loss_remain_visible(self):
        text = samples(1, ['malloc', '[unknown]', 'sqlite3Prepare'])
        text += samples(1, ['malloc', 'sqlite3Prepare', '[unknown]'])
        text += samples(1, ['[unknown]', 'sqlite3Prepare'])
        report = analyze(text, diagnostics=['WARNING: failed to unwind a sample'])
        self.assertEqual(report['exclusive_families']['compilation'], 1)
        self.assertEqual(report['exclusive_families']['unclassified'], 2)
        self.assertEqual(report['unknown_leaf_samples'], 1)
        self.assertTrue(any('unwind' in issue for issue in report['issues']))
        self.assertEqual(report['decision'], 'inconclusive')


class FifoControlTests(unittest.TestCase):
    """Real user-owned FIFO ordering; only /bin/true is executed normally."""

    def test_enable_ack_precedes_cont_and_launcher_preserves_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory) / name for name in ('ctl', 'ack')]
            for path in paths:
                os.mkfifo(path, 0o600)
            ctl, ack = [os.open(path, os.O_RDWR | os.O_NONBLOCK) for path in paths]
            with open(os.devnull, 'wb') as sink:
                child, identity = start_stopped_benchmark('/bin/true', [], sink, sink,
                                                         os.environ.copy())
                order = []

                def collector():
                    deadline = time.monotonic() + 2
                    while time.monotonic() < deadline:
                        try:
                            command = os.read(ctl, 100)
                        except BlockingIOError:
                            time.sleep(.001)
                            continue
                        if command:
                            order.append(command.decode().strip())
                            order.append('ack')
                            os.write(ack, b'ack\n')
                            return

                thread = threading.Thread(target=collector)
                thread.start()

                def resume():
                    order.append('CONT')
                    os.kill(child.pid, signal.SIGCONT)

                try:
                    enable_then_continue(ctl, ack, resume)
                    self.assertEqual(child.wait(timeout=2), 0)
                    self.assertEqual(order, ['enable', 'ack', 'CONT'])
                    for key in ('uids', 'sid', 'autogroup'):
                        self.assertEqual(identity['parent'][key], identity['child'][key])
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait()
                    thread.join(timeout=2)
                    os.close(ctl)
                    os.close(ack)

    def test_non_ack_cannot_continue_the_benchmark(self):
        read_fd, write_fd = os.pipe()
        ctl_read, ctl_write = os.pipe()
        os.set_blocking(read_fd, False)
        os.write(write_fd, b'not-an-ack\n')
        continued = []
        try:
            with self.assertRaises(RuntimeError):
                enable_then_continue(ctl_write, read_fd, lambda: continued.append(True))
            self.assertEqual(os.read(ctl_read, 100), b'enable\n')
            self.assertEqual(continued, [])
        finally:
            for descriptor in (read_fd, write_fd, ctl_read, ctl_write):
                os.close(descriptor)


if __name__ == '__main__':
    unittest.main()
