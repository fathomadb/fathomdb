"""Human-specified Slice30 measurement oracles; no database mocking."""

import unittest

from converter import convert


class ConversionTests(unittest.TestCase):
    def test_prefix_selection_keeps_first_seen_text_and_all_questions(self):
        data = [
            {"question": "Q one", "haystack_session_ids": ["a"],
             "haystack_sessions": [[{"content": " alpha "}, {"content": ""},
                                     {"content": "beta"}]]},
            {"question": "Q two", "haystack_session_ids": ["a", "b"],
             "haystack_sessions": [[{"content": "ignored repeated session"}],
                                    [{"content": "beta"}, {"content": "gamma"}]]},
        ]
        result = convert(data, 2)
        self.assertEqual(result["bodies"], ["alpha", "beta"])
        self.assertEqual(result["queries"], ["Q one", "Q two"])
        self.assertEqual(result["sessions"], 2)
        self.assertEqual(result["total_unique_turns"], 3)
        self.assertEqual(result["selected_count"], 2)

    def test_misaligned_session_arrays_are_rejected(self):
        data = [{"question": "Q", "haystack_session_ids": ["a", "b"],
                 "haystack_sessions": [[{"content": "alpha"}]]}]
        with self.assertRaisesRegex(ValueError, "session.*length"):
            convert(data, 2)


if __name__ == "__main__":
    unittest.main()
