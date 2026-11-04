import unittest
from slice84b_campaign import cells, screen


class Protocol(unittest.TestCase):
    def test_screen_is_retained_repetition_zero(self):
        grid = cells()
        self.assertEqual(len(grid), 42)
        self.assertEqual(len([c for c in grid if c['repetition'] == 0 and c['phase'] == 'core']), 12)
        self.assertEqual(len({c['id'] for c in grid}), 42)
        self.assertTrue(all(c['write_hz'] == (20 if c['family'] == 'property' else 100) for c in grid))

    def test_missing_controls_never_qualify(self):
        self.assertFalse(screen([], 'node')['qualified'])

    def test_control_miss_rejects_otherwise_improved_pair(self):
        rows = []
        for arm in ('baseline', 'candidate'):
            for mode in ('erase', 'control'):
                streams = {s: dict(scheduled=1500, completed=1500, failed=0, unfinished=0, unknown=0,
                                  accounting_errors=0, engine_refused=0, generator_missed=0) for s in ('read', 'write')}
                rows.append(dict(cell=dict(family='node', arm=arm, mode=mode, repetition=0, phase='core'),
                                 complete=True, result=dict(native_error=None, native_ns=100 if arm == 'baseline' else 50), streams=streams))
        self.assertTrue(screen(rows, 'node')['qualified'])
        rows[-1]['streams']['write']['generator_missed'] = 1
        self.assertFalse(screen(rows, 'node')['qualified'])


class FinalQualification(unittest.TestCase):
    def test_idle_retry_and_accounting_error_cannot_qualify(self):
        from slice84b_campaign import valid_cell
        row = dict(cell=dict(mode='idle'), complete=True, result=dict(native_error='ErasureIncomplete'),
                   streams={s: dict(failed=0, unfinished=0, unknown=0, accounting_errors=0) for s in ('read', 'write')})
        self.assertFalse(valid_cell(row))
        row['result']['native_error'] = None
        self.assertTrue(valid_cell(row))
        row['streams']['read']['accounting_errors'] = 1
        self.assertFalse(valid_cell(row))
