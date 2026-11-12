"""Manual SDK hybrid oracle with fixed SDK identity and literal result sets."""

import unittest

import test_g_core_hybrid as base


class SdkHybridTests(base.CoreHybridTests):
    adapter = "sdk"
    mode = "sdk_hybrid"


if __name__ == "__main__":
    unittest.main()
