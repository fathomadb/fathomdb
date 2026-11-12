"""Separate SDK text-only oracle on a fresh copy of the accepted BGE fixture."""

import unittest

import test_g_text as base


class SdkTextTests(base.CoreTextTests):
    adapter = "sdk"
    mode = "sdk_text"


if __name__ == "__main__":
    unittest.main()
