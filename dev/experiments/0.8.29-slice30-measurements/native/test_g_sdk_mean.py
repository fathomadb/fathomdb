"""Manual SDK mean population oracle, independent of the core output route."""

import unittest

import test_g_mean as core


class SdkMeanTests(core.CoreMeanTests):
    adapter = "sdk"
    mode = "sdk_mean"


if __name__ == "__main__":
    unittest.main()
