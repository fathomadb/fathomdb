#!/usr/bin/env python3
"""Exact local fork consumption and refusal boundaries from Slice110's design."""

from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
CUDARC_REV = "343b04fa73f85ee78754d6ee2baa07b959cc9ccc"
CANDLE_REV = "2ee39305a78ed1463ff913f71dcd3d45ecbfd84f"
CUDARC_GIT = "file:///home/coreyt/projects/fathomdb-forks/cudarc"
CANDLE_GIT = "file:///home/coreyt/projects/fathomdb-forks/candle-fathomdb"
K9 = "182ceab0547348993b6dbec4b1aed610dcc8b23f"


class QualifiedForkPins(unittest.TestCase):
    """Require the accepted local graph while refusing unqualified replacements."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for relative in (
            "Cargo.toml", "Cargo.lock", "package.json", "package-lock.json",
            "scripts/pinned-override-rot.json", "scripts/pinned-override-advisories.json",
            "src/rust/crates/fathomdb-embedder/Cargo.toml",
        ):
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, target)

    def replace(self, relative, before, after):
        path = self.root / relative
        text = path.read_text()
        self.assertIn(before, text)
        path.write_text(text.replace(before, after))

    def check(self):
        return subprocess.run(
            ["python3", str(ROOT / "scripts/check-pinned-override-rot.py"), "--root", str(self.root)],
            capture_output=True, text=True, check=False,
        )

    def refused(self):
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_exact_qualified_graph_passes(self):
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("local qualification", result.stdout)
        self.assertIn("publication", result.stdout)

    def test_source_identity_is_exact_for_each_fork(self):
        original = (self.root / "Cargo.lock").read_text()
        for url, revision in ((CUDARC_GIT, CUDARC_REV), (CANDLE_GIT, CANDLE_REV)):
            for before, after in (
                (url, url + "-other"),
                (url, url.replace("file:///home/coreyt/projects/fathomdb-forks/", "https://github.com/coreyt/") + ".git"),
                (revision, "0" * 40),
            ):
                with self.subTest(source=after):
                    (self.root / "Cargo.lock").write_text(original.replace(before, after))
                    self.refused()
        (self.root / "Cargo.lock").write_text(original)

    def test_unpublished_versions_cannot_claim_registry_checksums(self):
        self.replace("Cargo.lock", f"git+{CUDARC_GIT}?rev={CUDARC_REV}#{CUDARC_REV}",
                     "registry+https://github.com/rust-lang/crates.io-index")
        self.refused()

    def test_git_package_cannot_claim_a_published_checksum(self):
        source = f'source = "git+{CUDARC_GIT}?rev={CUDARC_REV}#{CUDARC_REV}"'
        self.replace("Cargo.lock", source, source + '\nchecksum = "' + "0" * 64 + '"')
        self.refused()

    def test_candle_family_cannot_mix_qualified_and_published_members(self):
        self.replace("Cargo.lock", 'name = "candle-nn-fathomdb"\nversion = "0.12.0"',
                     'name = "candle-nn-fathomdb"\nversion = "0.11.0"')
        self.refused()

    def test_manifest_cannot_downgrade_the_selected_family(self):
        self.replace("src/rust/crates/fathomdb-embedder/Cargo.toml", '=0.12.0', '=0.11.0')
        self.refused()

    def test_kernels_cannot_follow_the_new_local_candle_source(self):
        self.replace("Cargo.lock", f"git+https://github.com/coreyt/candle-fathomdb.git?rev={K9}#{K9}",
                     f"git+{CANDLE_GIT}?rev={CANDLE_REV}#{CANDLE_REV}")
        self.refused()

    def test_single_cudarc_accepts_only_the_selected_tree_source(self):
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        cargo = bin_dir / "cargo"
        cargo.write_text('#!/bin/sh\ncase " $* " in *" -e normal "*) printf "%s\\n" "$TEST_CUDARC_LINE" ;; esac\n')
        cargo.chmod(0o755)
        expected = f"cudarc-fathomdb v0.20.0 ({CUDARC_GIT}?rev={CUDARC_REV}#{CUDARC_REV[:8]})"
        for line, code in ((expected, 0), ("cudarc-fathomdb v0.20.0", 1),
                           (expected.replace(CUDARC_REV, "0" * 40), 1),
                           (expected.replace(CUDARC_GIT, CUDARC_GIT + "-other"), 1),
                           ("cudarc v0.20.0\n" + expected, 1)):
            with self.subTest(line=line):
                result = subprocess.run(
                    ["bash", str(ROOT / "scripts/check-single-cudarc.sh"), "--root", str(self.root)],
                    env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
                         "TEST_CUDARC_LINE": line},
                    capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, code, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
