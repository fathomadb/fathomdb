"""Actual tiny SQLite copy isolation, never the selected100k measurement subject."""

from contextlib import closing
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import time
import unittest

from g_driver import execute_stage
from g_freeze import identify
from g_span_coverage import stored_coverage


def tiny_source(root):
    store = root / "released.sqlite"
    with closing(sqlite3.connect(store)) as connection:
        connection.executescript("""
            CREATE TABLE docs(id INTEGER PRIMARY KEY, body TEXT);
            INSERT INTO docs VALUES(1,'original');
            CREATE TABLE _fathomdb_vector_spans(write_cursor INTEGER,window_number INTEGER,
                byte_start INTEGER,byte_end INTEGER,body_length INTEGER);
            INSERT INTO _fathomdb_vector_spans VALUES(1,0,0,8,8);
            PRAGMA user_version=37;
        """)
        connection.commit()
    return {"files": {"": identify(store)}, "coverage": stored_coverage(store),
            "qualified_origin": "tiny-closed-file-control,not-Default-qualification"}


class GClosedCopyTests(unittest.TestCase):
    def test_two_fresh_copies_never_mutate_baseline_or_each_other(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = tiny_source(root)
            control = [sys.executable, "-c", "\n".join([
                "import json,pathlib,sqlite3,sys",
                "c=json.loads(pathlib.Path(sys.argv[1]).read_text());o=pathlib.Path(sys.argv[2]);o.mkdir()",
                "db=sqlite3.connect(c['store']);before=db.execute('SELECT body FROM docs').fetchone()[0]",
                "db.execute(\"UPDATE docs SET body='mutated'\");db.commit();db.close()",
                "(o/'result.json').write_text(json.dumps({'complete':True,'runner_only':True,'initial':before,'store':c['store']}))",
            ])]
            manifest = {"command": control, "corpus": "unused-by-control", "queries": "unused-by-control",
                        "subjects": {"g1-n100000": source["files"][""]["path"]}, "closed_subject": source}
            cell = {"kind": "write", "stage": "G-W", "preparation": "closed-copy-v1",
                    "fresh_seed": False, "fresh_copy": True, "seed_subject": "g1-n100000"}
            first = execute_stage(cell, manifest, root / "first", time.monotonic_ns() + 5 * 10**9)
            with closing(sqlite3.connect(source["files"][""]["path"])) as connection:
                self.assertEqual(connection.execute('SELECT body FROM docs').fetchone()[0], 'original',
                                 "closed-copy driver mutated the released source instead of a new copy")
            second = execute_stage(cell, manifest, root / "second", time.monotonic_ns() + 5 * 10**9)
            self.assertTrue(first["complete"] and second["complete"])
            self.assertEqual(first["steps"][0]["result"]["initial"], "original")
            self.assertEqual(second["steps"][0]["result"]["initial"], "original")
            self.assertNotEqual(first["steps"][0]["result"]["store"], second["steps"][0]["result"]["store"])
            for value in (first, second):
                preparation = value["preparation"]
                self.assertTrue(preparation["validated"])
                self.assertGreater(preparation["copy_ns"], 0)
                self.assertGreater(preparation["validation_ns"], 0)
                self.assertEqual(preparation["copied_bytes"], source["files"][""]["bytes"])
