"""The Windows script prefix must leave uploaded binary bytes unchanged."""
import base64
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
OPERATOR = ROOT / "dev/experiments/0.8.29-slice20-windows/a6_operator.py"
spec = importlib.util.spec_from_file_location("slice20_windows_operator", OPERATOR)
op = importlib.util.module_from_spec(spec)
spec.loader.exec_module(op)


class TransportTests(unittest.TestCase):
    def test_long_script_uses_short_command_and_preserves_binary_tail(self):
        script = "$value='λ';\n" * 500
        binary = bytes(range(256)) * 300
        observed = {}

        class Endpoint:
            returncode = 0

            def __init__(self, argv, **kwargs):
                observed["argv"] = argv

            def communicate(self, data, timeout):
                observed["stdin"] = data
                return b"accepted", b""

        with patch.object(op.subprocess, "Popen", Endpoint):
            self.assertEqual(op.command(script, input_bytes=binary), "accepted")
        argv = observed["argv"]
        remote = argv[argv.index("-NoLogo") - 1:]
        self.assertLessEqual(sum(len(x) + 1 for x in remote), 4096)
        prefix = base64.b64encode(script.encode("utf-8"))
        self.assertEqual(observed["stdin"][:len(prefix)], prefix)
        self.assertEqual(observed["stdin"][len(prefix):], binary)


if __name__ == "__main__":
    unittest.main()
