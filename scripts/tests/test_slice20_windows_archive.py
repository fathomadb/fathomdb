"""Deliver exact build inputs without unrelated historical result symlinks."""
import argparse
import hashlib
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("a6_archive_operator", ROOT / "dev/experiments/0.8.29-slice20-windows/a6_operator.py")
op = importlib.util.module_from_spec(spec)
spec.loader.exec_module(op)
HISTORY = "dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot"


class ArchiveTests(unittest.TestCase):
    def fixture(self, root, link):
        source = root / "source"
        source.mkdir()
        def git(*args):
            return subprocess.check_output(["git", "-C", str(source), *args], text=True).strip()
        git("init", "-q")
        git("config", "core.symlinks", "false")
        (source / "Cargo.toml").write_text("[workspace]\nmembers=[]\n")
        (source / "src").mkdir()
        (source / "src/lib.rs").write_text("pub fn actual_code() {}\n")
        path = source / link
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("../shared/result")
        git("add", ".")
        blob = git("hash-object", str(path))
        git("update-index", "--cacheinfo", "120000," + blob + "," + link)
        git("-c", "user.name=Archive Fixture", "-c", "user.email=fixture@example.test", "commit", "-qm", "fixture")
        evidence = root / "evidence"
        evidence.mkdir()
        class Session:
            args = argparse.Namespace(source=source, candidate_sha=git("rev-parse", "HEAD"))
            cfg = {}
            run_id = "1" * 32
            def upload(self, path, relative):
                self.uploaded = path
        session = Session()
        session.root = evidence
        return source, session

    def test_historical_symlinks_are_omitted_and_build_bytes_are_exact(self):
        with tempfile.TemporaryDirectory() as directory:
            source, session = self.fixture(Path(directory), HISTORY + "/baseline/old-link")
            op.source_archive(session)
            self.assertEqual(set(session.cfg["sourceFiles"]), {"Cargo.toml", "src/lib.rs"})
            with zipfile.ZipFile(session.uploaded) as archive:
                for name, expected in session.cfg["sourceFiles"].items():
                    self.assertEqual(archive.read(name), (source / name).read_bytes())
                    self.assertEqual(hashlib.sha256(archive.read(name)).hexdigest(), expected)

    def test_other_symlinks_still_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            _, session = self.fixture(Path(directory), "src/unsafe-link")
            with self.assertRaisesRegex(ValueError, "unsafe source archive"):
                op.source_archive(session)


if __name__ == "__main__":
    unittest.main()
