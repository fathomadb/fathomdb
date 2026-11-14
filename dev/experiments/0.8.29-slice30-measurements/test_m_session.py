"""Unanswered real child invocations retain selected wire-format identity."""

import sys
import tempfile
import unittest

from m_session import McpSession


class SessionIdentityTests(unittest.TestCase):
    """Request facts survive even when a bounded child supplies no response."""

    def test_unresolved_invocation_retains_selected_format_before_response(self):
        with tempfile.TemporaryDirectory() as root:
            with McpSession([sys.executable, "-u", "-c", "import sys\nfor line in sys.stdin: pass"],
                            root, result_format="structured", timeout=.1) as session:
                request = session.submit("echo", {}, phase="control")
                self.assertIsNone(session.wait(request, timeout=.02))
            self.assertEqual(session.exit["code"], 0)
            self.assertEqual(session.records[request].get("format"), "structured",
                             "an unanswered invocation lost its wire-format identity")
