#!/usr/bin/env python3
"""Compile-only G observer exposure controls; expected rejections are not runtime RED."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class GFeatureBoundaryTests(unittest.TestCase):
    def test_default_consumers_reject_and_explicit_feature_consumers_accept_both_surfaces(self):
        consumer = Path(tempfile.mkdtemp(prefix="fathomdb-slice30-g-consumer-"))
        (consumer / "src").mkdir()
        (consumer / "Cargo.toml").write_text(
            '[package]\nname="slice30-g-boundary-consumer"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[features]\nobserve=["fathomdb-engine/slice30-observer","fathomdb-sdk/slice30-observer"]\n'
            '[dependencies]\n'
            f'fathomdb-engine={{path="{ROOT}/src/rust/crates/fathomdb-engine",default-features=false}}\n'
            f'fathomdb-sdk={{path="{ROOT}/src/rust/crates/fathomdb-sdk",default-features=false}}\n'
        )
        (consumer / "src/main.rs").write_text(
            'fn main() {\n'
            '    let _ = fathomdb_engine::Engine::begin_slice30_observation;\n'
            '    let _ = fathomdb_sdk::Engine::begin_slice30_observation;\n'
            '}\n'
        )
        environment = dict(os.environ)
        target = Path(environment.get("CARGO_TARGET_DIR", str(ROOT / "target")))
        environment["CARGO_TARGET_DIR"] = str(target / "slice30-g-consumers")
        command = ["cargo", "check", "--offline", "--message-format=json", "--manifest-path", str(consumer / "Cargo.toml")]
        print(f"compile-only retained consumer: {consumer}", flush=True)
        for features, expected in [(["--features", "observe"], True), ([], False)]:
            result = subprocess.run(command + features, env=environment, text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
            log = consumer / ("feature-positive.log" if expected else "default-negative.log")
            log.write_text(result.stdout)
            if expected:
                self.assertEqual(result.returncode, 0, result.stdout)
                continue
            self.assertNotEqual(result.returncode, 0, "default G observer consumer unexpectedly compiled")
            rejected = set()
            errors = []
            for line in result.stdout.splitlines():
                if not line.startswith("{"):
                    continue
                message = json.loads(line)
                if message.get("reason") != "compiler-message" or message["message"]["level"] != "error":
                    continue
                diagnostic = message["message"]
                errors.append(diagnostic.get("code", {}).get("code"))
                for span in diagnostic["spans"]:
                    if span["is_primary"] and span["file_name"].endswith("main.rs"):
                        rejected.add(span["line_start"])
            self.assertEqual(rejected, {2, 3}, result.stdout)
            self.assertEqual(errors, ["E0599", "E0599"], result.stdout)


if __name__ == "__main__":
    unittest.main()
