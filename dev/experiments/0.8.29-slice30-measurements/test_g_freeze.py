"""Actual local byte controls for G preregistration; no model/native execution."""

from pathlib import Path
import tempfile
import unittest

from g_freeze import freeze


class GFreezePinsTests(unittest.TestCase):
    def test_unregistered_actual_corpus_bytes_are_refused_before_freeze_publication(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            options = {}
            for key in ("corpus", "queries", "license", "ordinary", "diagnostic", "build_receipt"):
                path = root / key
                path.write_text('"controlled unregistered bytes"\n')
                options[key] = str(path)
            cache = root / "cache"
            cache.mkdir()
            for name in ("config.json", "tokenizer.json", "model.safetensors"):
                (cache / name).write_bytes(b"controlled bytes; not a model")
            options.update(model_cache=str(cache), allocation=str(root / "allocation"),
                           out=str(root / "allocation" / "frozen"), gpu_uuid="GPU-unqualified-control", gpu_ordinal=0)
            with self.assertRaisesRegex(ValueError, "registered corpus mismatch"):
                freeze(options)
            self.assertFalse(Path(options["out"]).exists(), "bad registered bytes published a frozen campaign")
            self.assertTrue(all((cache / name).read_bytes() == b"controlled bytes; not a model"
                                for name in ("config.json", "tokenizer.json", "model.safetensors")))


if __name__ == "__main__":
    unittest.main()
