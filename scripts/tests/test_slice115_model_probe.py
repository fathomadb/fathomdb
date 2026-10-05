"""Required real-default-model projection evidence cannot be inferred from inference alone."""

import importlib.util
from pathlib import Path
import unittest

from test_slice115_receipt import valid_data


SCRIPT = Path(__file__).resolve().parents[1] / "slice115_receipt.py"
SPEC = importlib.util.spec_from_file_location("slice115_receipt", SCRIPT)
assert SPEC and SPEC.loader
receipt = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(receipt)


class ModelProbeTests(unittest.TestCase):
    def test_model_cell_requires_one_real_projection(self):
        raw = valid_data()
        for cell in raw["cells"]["model_cpu"]:
            del cell["model_projection_rows"]
            del cell["projection_ns"]
        with self.assertRaisesRegex(ValueError, "model projection"):
            receipt.validate(raw, "a" * 40, "b" * 64, "c" * 64)
        for cell in raw["cells"]["model_cpu"]:
            cell["model_projection_rows"] = 1
            cell["projection_ns"] = 25
        receipt.validate(raw, "a" * 40, "b" * 64, "c" * 64)


if __name__ == "__main__":
    unittest.main()
