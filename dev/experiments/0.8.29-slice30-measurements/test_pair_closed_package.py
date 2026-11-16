"""Real tiny SQLite WAL custody controls; no native or scientific qualification."""

from contextlib import closing
import json
from pathlib import Path
import shutil
import sqlite3
import tempfile
import time
import unittest

from g_closed_copy import copy_subject
from g_freeze import identify
from g_pair_freeze import donor, package
from g_span_coverage import stored_coverage


def closed_store(root, *, committed_wal=False):
    writer = root / 'writer.sqlite'
    with closing(sqlite3.connect(writer)) as db:
        db.execute('PRAGMA journal_mode=WAL')
        db.executescript('''
            CREATE TABLE docs(body TEXT);
            CREATE TABLE canonical_nodes(kind TEXT, superseded_at INTEGER);
            CREATE TABLE _fathomdb_vector_rows(write_cursor INTEGER);
            CREATE TABLE _fathomdb_vector_spans(write_cursor INTEGER, window_number INTEGER,
                byte_start INTEGER, byte_end INTEGER, body_length INTEGER);
            PRAGMA user_version=37;
        ''')
        db.execute('PRAGMA wal_checkpoint(TRUNCATE)')
        db.execute("INSERT INTO docs VALUES('wal body')")
        db.execute("INSERT INTO canonical_nodes VALUES('doc',NULL)")
        db.execute('INSERT INTO _fathomdb_vector_rows VALUES(1)')
        db.execute('INSERT INTO _fathomdb_vector_spans VALUES(1,0,0,8,8)')
        db.commit()
        if committed_wal:
            store = root / 'closed.sqlite'
            shutil.copyfile(writer, store)
            shutil.copyfile(Path(str(writer) + '-wal'), Path(str(store) + '-wal'))
    if not committed_wal:
        store = writer
        assert not Path(str(store) + '-wal').exists()
    return store


def donor_root(root, store):
    origin = root / 'origin'
    attempt = origin / 'results/g1-n1/attempt-0001'
    attempt.mkdir(parents=True)
    facts = {'format': 37, 'natural_records': 1, 'vector_rows': 1, 'span_rows': 1,
             'physical_vector_rows': 1, 'pending_count': 0, 'generation_failed': 0}
    result = {'complete': True, 'qualification_issues': [], 'steps': [
        {'family': 'G-seed', 'closed_store': str(store), 'result': {'after_reopen': facts}}]}
    (attempt / 'result.json').write_text(json.dumps(result))
    (origin / 'manifest.json').write_text(json.dumps({'root': str(origin), 'source_sha': 'tiny-data-origin'}))
    (origin / 'results/g-outcomes.json').write_text(json.dumps([
        {'id': 'g1-n1', 'status': 'complete', 'exit': 0, 'cleanup_uncertain': False,
         'live_owned_pids': [], 'attempt': str(attempt)}]))
    return origin


class PairClosedPackageTests(unittest.TestCase):
    def test_package_captures_read_created_wal_and_preserves_committed_wal_bytes(self):
        for committed_wal in (False, True):
            with self.subTest(committed_wal=committed_wal), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                store = closed_store(root, committed_wal=committed_wal)
                before = store.read_bytes()
                wal = Path(str(store) + '-wal')
                wal_before = wal.read_bytes() if wal.exists() else None
                subject = package(store)
                self.assertIn('-wal', subject['files'], 'metadata inspection created an unaccounted WAL')
                self.assertEqual(subject['files']['-wal'], identify(wal))
                if wal_before is not None:
                    self.assertGreater(len(wal_before), 0)
                    self.assertEqual(wal.read_bytes(), wal_before)
                copied, proof = copy_subject(subject, root / 'copy.sqlite', time.monotonic_ns() + 5*10**9)
                with closing(sqlite3.connect(Path(copied).as_uri() + '?mode=ro', uri=True)) as db:
                    self.assertEqual(db.execute('SELECT body FROM docs').fetchall(), [('wal body',)])
                self.assertTrue(proof['validated'])
                self.assertEqual(store.read_bytes(), before)
                if wal_before is not None:
                    self.assertEqual(wal.read_bytes(), wal_before)

    def test_selected_donor_refreshes_only_omitted_wal_and_retains_old_package(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            store = closed_store(root)
            origin = donor_root(root, store)
            old = {'files': {'': identify(store)}, 'coverage': stored_coverage(store)}
            before = json.loads(json.dumps(old))
            actual = donor(origin, 1, previous=old)
            self.assertEqual(old, before)
            self.assertEqual(actual['files'][''], old['files'][''])
            self.assertEqual(actual['coverage'], old['coverage'])
            self.assertEqual(actual['prior_packages'], [old])
            self.assertEqual(actual['files']['-wal'], identify(Path(str(store) + '-wal')))
            copied, proof = copy_subject(actual, root / 'copy.sqlite', time.monotonic_ns() + 5*10**9)
            self.assertTrue(proof['validated'])
            self.assertTrue(Path(copied).exists())
