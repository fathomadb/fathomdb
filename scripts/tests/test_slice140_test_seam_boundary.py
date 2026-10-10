#!/usr/bin/env python3
"""Compile independent consumers of the recorded test-only engine paths."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
INVENTORY = ROOT / "dev/plans/0.8.27/features/slice-140/test-seam-inventory.json"


class EngineTestSeamBoundary(unittest.TestCase):
    def test_external_default_and_explicit_feature_consumers(self) -> None:
        paths = json.loads(INVENTORY.read_text())["removed_default_paths"]
        with tempfile.TemporaryDirectory(prefix="fathomdb-slice140-consumer-") as directory:
            consumer = Path(directory)
            (consumer / "src").mkdir()
            (consumer / "Cargo.toml").write_text(
                '[package]\nname = "slice140-seam-consumer"\nversion = "0.0.0"\nedition = "2021"\n'
                '[workspace]\n[features]\nhooks = ["fathomdb-engine/test-hooks"]\n'
                '[dependencies]\n'
                f'fathomdb-engine = {{ path = "{ROOT}/src/rust/crates/fathomdb-engine", default-features = false }}\n'
                f'fathomdb-embedder-api = {{ path = "{ROOT}/src/rust/crates/fathomdb-embedder-api" }}\n'
            )
            (consumer / "src/main.rs").write_text(
                "fn main() {\n" + "".join(f'    let _ = {row["expression"]};\n' for row in paths) + "}\n"
            )
            environment = dict(os.environ)
            environment["CARGO_TARGET_DIR"] = str(ROOT / "target/slice140-consumers")
            for release in (False, True):
                command = ["cargo", "check", "--message-format=json", "--offline", "--manifest-path", str(consumer / "Cargo.toml")]
                if release:
                    command.append("--release")
                positive = subprocess.run(command + ["--features", "hooks"], env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                self.assertEqual(positive.returncode, 0, positive.stdout)
                negative = subprocess.run(command, env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                self.assertNotEqual(negative.returncode, 0, "Default consumer unexpectedly compiled every test path")
                rejected_lines = set()
                for line in negative.stdout.splitlines():
                    if not line.startswith("{"):
                        continue
                    message = json.loads(line)
                    if message.get("reason") != "compiler-message":
                        continue
                    diagnostic = message["message"]
                    if diagnostic["level"] != "error":
                        continue
                    for span in diagnostic["spans"]:
                        if span["is_primary"] and span["file_name"].endswith("main.rs"):
                            rejected_lines.add(span["line_start"])
                for line_number, row in enumerate(paths, start=2):
                    self.assertIn(line_number, rejected_lines, f'{row["path"]} was not rejected at its own source line:\n{negative.stdout}')


if __name__ == "__main__":
    unittest.main()
