"""Pin the Slice 115 model digest false positive to its exact evidence paths."""

import hashlib
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / "scripts/security/gitleaks-current.toml"
DIGEST = "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66"
PATHS = (
    "dev/plans/0.8.27/features/slice-115/protocol.json",
    "dev/plans/0.8.27/features/slice-115/evidence/final/attempt.json",
    "dev/plans/0.8.27/features/slice-115/evidence/final/model-assets.json",
    "dev/plans/0.8.27/features/slice-115/evidence/final/protocol.json",
    "dev/plans/0.8.27/features/slice-115/evidence/invalid/attempt.json",
    "dev/plans/0.8.27/features/slice-115/evidence/superseded-attempt5/attempt.json",
    "dev/plans/0.8.27/features/slice-115/evidence/superseded-attempt5/model-assets.json",
    "dev/plans/0.8.27/features/slice-115/evidence/superseded-attempt5/protocol.json",
    "dev/plans/0.8.27/features/slice-115/evidence/superseded-attempt6/attempt.json",
    "dev/plans/0.8.27/features/slice-115/evidence/superseded-attempt6/model-assets.json",
    "dev/plans/0.8.27/features/slice-115/evidence/superseded-attempt6/protocol.json",
)


class Slice115DigestExceptionTests(unittest.TestCase):
    def test_exception_matches_only_pinned_digest_at_receipt_paths(self):
        rules = tomllib.loads(POLICY.read_text())["rules"][0]["allowlists"]
        exception = next(rule for rule in rules if rule["description"].startswith("Slice 115 model"))
        self.assertEqual(exception["condition"], "AND")
        self.assertEqual(exception["regexTarget"], "match")
        path = re.compile(exception["paths"][0])
        match = re.compile(exception["regexes"][0])
        for file in PATHS:
            self.assertIsNotNone(path.fullmatch(file), file)
        self.assertIsNone(path.fullmatch("dev/plans/0.8.27/features/slice-115/evidence/final/raw.json"))
        self.assertIsNotNone(match.fullmatch(f'tokenizer.json": "{DIGEST}"'))
        self.assertIsNone(match.fullmatch('tokenizer.json": "' + "0" * 64 + '"'))

    def test_scanner_keeps_other_high_entropy_digest_blocked(self):
        path = PATHS[0]
        other = hashlib.sha256(b"slice115-other-model").hexdigest()
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / path
            file.parent.mkdir(parents=True)
            argv = ["gitleaks", "dir", "--config", str(POLICY), "--redact=100",
                    "--no-banner", "--no-color", "--exit-code", "1", "."]
            for digest, wanted in ((DIGEST, 0), (other, 1)):
                file.write_text('{"tokenizer.json": "' + digest + '"}\n')
                result = subprocess.run(argv, cwd=directory, capture_output=True, text=True)
                self.assertEqual(result.returncode, wanted, result.stderr)


if __name__ == "__main__":
    unittest.main()
