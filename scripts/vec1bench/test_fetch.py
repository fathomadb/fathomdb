"""Acquisition must never start a multi-GB network transfer implicitly."""

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("fetch.py")
SPEC = importlib.util.spec_from_file_location("vec1bench_fetch", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
fetch = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fetch)


class AcquisitionTests(unittest.TestCase):
    def test_absent_source_refuses_without_network_flag(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaisesRegex(FileNotFoundError, "allow-network"):
                fetch.acquire_dataset("sift-128-euclidean", Path(temp), allow_network=False)

    def test_present_corrupt_source_refuses_without_download(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            raw = root / "raw"
            raw.mkdir()
            (raw / "sift-128-euclidean.hdf5").write_bytes(b"not a trusted corpus")
            with self.assertRaisesRegex(ValueError, "SHA256"):
                fetch.acquire_dataset("sift-128-euclidean", root, allow_network=False)


if __name__ == "__main__":
    unittest.main()
