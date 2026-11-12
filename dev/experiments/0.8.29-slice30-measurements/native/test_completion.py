"""Fixed-selector real CLI oracle for the private G/D observer."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class CompletionTests(unittest.TestCase):
    def test_existing_real_search_emits_one_engine_owned_completion(self):
        binary = Path(os.environ["CARGO_TARGET_DIR"]) / "release/examples/slice30_gd_diagnostic"
        print(json.dumps({"binary":str(binary),"sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),
                          "features":["slice30-measurement"]}),flush=True)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            corpus, queries = root / "corpus.jsonl", root / "queries.jsonl"
            corpus.write_text('"alphaonly evidence"\n"betaonly evidence"\n')
            queries.write_text('"alphaonly"\n')
            config = root / "cell.json"
            config.write_text(json.dumps({"family":"P1","n":2,"adapters":["engine"],
                "warmup":0,"samples":1,"corpus":str(corpus),"queries":str(queries)}))
            out = root / "out"
            child = subprocess.run([str(binary),str(config),str(out)],capture_output=True,text=True,timeout=30)
            self.assertEqual(child.returncode,0,child.stderr)
            samples = [json.loads(line) for line in (out / "samples.jsonl").read_text().splitlines()]
            self.assertEqual(samples[0]["ids"],["l:s30doc-0"])
            self.assertGreater(samples[0]["elapsed_ns"],0)
            observations = out / "observer.jsonl"
            rows = [json.loads(line) for line in observations.read_text().splitlines()] if observations.exists() else []
            self.assertEqual(len(rows),1)
            self.assertEqual(rows[0]["event"],"QueryComplete")
            self.assertGreater(rows[0]["engine_id"],0)
            self.assertGreater(rows[0]["call_id"],0)


if __name__ == "__main__":
    unittest.main()
