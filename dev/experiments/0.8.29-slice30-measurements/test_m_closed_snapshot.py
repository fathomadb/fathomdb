"""Real SQLite RO companions must not change the frozen committed main image."""

from pathlib import Path
import sqlite3
import time
import unittest
from unittest.mock import patch

from g_freeze import identify
from m_calibration import registered_cell
import m_runtime
import test_m_bind_inputs as fixtures


class ClosedSnapshotTests(unittest.TestCase):
    setUp = fixtures.InputBindingTests.setUp
    bind = fixtures.InputBindingTests.bind

    def retained_main(self):
        main = Path(self.options["closed_subject"]["main"])
        db = sqlite3.connect(main)
        try:
            self.assertEqual(db.execute("PRAGMA journal_mode=WAL").fetchone()[0], "wal")
        finally:
            db.close()
        self.cell = registered_cell("M-L-1-lazy")
        original = self.bind("original", selected=self.cell)
        self.original_bytes = (self.root / "original/manifest.json").read_bytes()
        self.options["closed_main_copy_from"] = str(self.root / "original")
        self.options["reuse_from"] = [{"root": str(self.root / "original"), "cell_id": self.cell["id"]}]
        db = sqlite3.connect(main.as_uri() + "?mode=ro", uri=True)
        try:
            self.assertEqual(db.execute("SELECT body FROM literal").fetchone()[0], "alpha")
        finally:
            db.close()
        self.assertEqual(Path(str(main) + "-wal").stat().st_size, 0)
        self.assertEqual(Path(str(main) + "-shm").stat().st_size, 32768)
        return main, original

    def corrected_binding(self):
        try:
            return self.bind("corrected", selected=self.cell)
        except ValueError as error:
            self.fail(f"unchanged committed main refused after real RO-created empty WAL/SHM: {error}")

    def test_real_ro_companions_allow_explicit_unchanged_main_copy_and_preserve_history(self):
        main, original = self.retained_main()
        companions = {suffix: identify(str(main) + suffix) for suffix in ("-wal", "-shm")}
        value = self.corrected_binding()
        self.assertEqual(value["closed_subject"], original["closed_subject"])
        self.assertEqual(value["package"], original["package"])
        (self.root / "copy").mkdir()
        with patch("m_runtime.worker_admission"):
            m_runtime.prepare(value, self.cell, self.root / "copy", time.monotonic_ns() + 10**10)
        copied = identify(self.root / "copy/stores/a.sqlite")
        self.assertEqual((copied["bytes"], copied["sha256"]), (original["package"][0]["bytes"], original["package"][0]["sha256"]))
        self.assertEqual(companions, {suffix: identify(str(main) + suffix) for suffix in companions})
        self.assertEqual(self.original_bytes, (self.root / "original/manifest.json").read_bytes())
        self.assertEqual(value["closed_main_copy"]["companions"], companions)

    def test_default_still_refuses_current_unlisted_companions(self):
        self.retained_main()
        del self.options["closed_main_copy_from"]
        with self.assertRaises(ValueError):
            self.bind("default-refusal", selected=self.cell)

    def test_nonempty_wal_cannot_supply_main_only_snapshot(self):
        main, _ = self.retained_main()
        db = sqlite3.connect(main)
        try:
            db.execute("INSERT INTO literal VALUES ('committed in WAL')")
            db.commit()
            self.assertGreater(Path(str(main) + "-wal").stat().st_size, 0)
            with self.assertRaises(ValueError):
                self.bind("wal-refusal", selected=self.cell)
        finally:
            db.close()

    def test_changed_main_cannot_reuse_original_main_identity(self):
        main, _ = self.retained_main()
        db = sqlite3.connect(main)
        try:
            db.execute("INSERT INTO literal VALUES ('different main')")
            db.commit()
        finally:
            db.close()
        with self.assertRaises(ValueError):
            self.bind("changed-refusal", selected=self.cell)

    def test_precopy_auxiliary_change_refuses_without_creating_store(self):
        main, original = self.retained_main()
        value = dict(original, closed_main_copy={
            "reference_manifest": identify(self.root / "original/manifest.json"),
            "companions": {suffix: identify(str(main) + suffix) for suffix in ("-wal", "-shm")}})
        shm = Path(str(main) + "-shm")
        shm.write_bytes(shm.read_bytes() + b"changed")
        attempt = self.root / "precopy"
        attempt.mkdir()
        with patch("m_runtime.worker_admission"), self.assertRaises(ValueError):
            m_runtime.prepare(value, self.cell, attempt, time.monotonic_ns() + 10**10)
        self.assertFalse((attempt / "stores").exists())

    def test_foreign_reference_is_refused(self):
        self.retained_main()
        self.options["closed_main_copy_from"] = str(self.root.parent / "unrelated")
        with self.assertRaises(ValueError):
            self.bind("foreign-refusal", selected=self.cell)


if __name__ == "__main__":
    unittest.main()
