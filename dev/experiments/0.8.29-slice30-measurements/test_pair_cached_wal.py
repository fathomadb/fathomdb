"""Cached-donor correction uses real SQLite; the GPU boundary always refuses."""

from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from g_freeze import identify
from g_pair_freeze import donor, prepare_inputs
from g_span_coverage import stored_coverage
from test_pair_closed_package import closed_store, donor_root


class PairCachedWalTests(unittest.TestCase):
    def test_cached_missing_nonempty_wal_is_refused_even_when_coverage_matches(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            store = closed_store(root, committed_wal=True)
            origin = donor_root(root, store)
            old = {'files': {'': identify(store)}, 'coverage': stored_coverage(store)}
            wal = Path(str(store)+'-wal')
            before = wal.read_bytes()
            self.assertGreater(len(before), 0)
            with self.assertRaisesRegex(ValueError, 'empty|WAL'):
                donor(origin, 1, previous=old)
            self.assertEqual(wal.read_bytes(), before)

    def test_selected_preparation_requalifies_cached_missing_wal_before_gpu_boundary(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            store = closed_store(root)
            origin = donor_root(root, store)
            old = {'files': {'': identify(store)}, 'coverage': stored_coverage(store)}
            manifest = {'root': str(root), 'templates': {}, 'donors': {'G1': old},
                        'g_donors': {'1': str(origin)}, '_selection_record': {'guardian_owner': {'pid': 7}},
                        'requested_gpu': {'uuid': 'unavailable-in-control', 'ordinal': 0}}
            captured = []
            def actual_donor(*args, **kwargs):
                value = donor(*args, **kwargs)
                captured.append(value)
                return value
            with patch('g_pair_freeze.donor', side_effect=actual_donor), \
                    patch('g_gpu.inspect_gpu', side_effect=ValueError('GPU boundary deliberately unavailable')):
                with self.assertRaisesRegex(ValueError, 'GPU boundary deliberately unavailable'):
                    prepare_inputs(manifest, [{'family': 'G-R', 'n': 1}], time.monotonic_ns()+5*10**9)
            self.assertEqual(len(captured), 1, 'selected preparation kept the stale cached package')
            self.assertEqual(captured[0]['files']['-wal'], identify(Path(str(store)+'-wal')))
            self.assertEqual(captured[0]['prior_packages'], [old])
            self.assertFalse((root / 'results/prepared.json').exists())
