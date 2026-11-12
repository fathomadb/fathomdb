"""Manual normal core-hybrid records after a real governed two-vector fixture."""

import unittest

import test_g_mean as core


class CoreHybridTests(core.CoreMeanTests):
    adapter = "engine"
    mode = "engine_hybrid"
    query = "alphaonly "*510

    def test_actual_population_records_are_retained(self):
        root,proof,rows,issues = self.run_fixture()
        if proof["api_calls"]!=128 or proof["fixture_ids"]!=["l:alpha","l:beta"]:
            raise RuntimeError("existing hybrid API literal fixture did not pass")
        if (proof["fixture"]["rows"],proof["fixture"]["vectors"],proof["fixture"]["pending"])!=(2,2,0):
            raise RuntimeError("real governed projection fixture is not drained/dense")
        self.assertEqual(len(rows),128,f"missing actual {self.adapter} hybrid measurement population")
        self.assertEqual(issues,[])
        for index,row in enumerate(rows):
            self.assertEqual((row["sample_index"],row["query_index"]),(index,0))
            self.assertEqual((row["adapter"],row["mode"],row["ok"]),(self.adapter,"hybrid",True))
            self.assertEqual(sorted(row["ids"]),["l:alpha","l:beta"])
            self.assertEqual(row["returned_hits"],2)
            self.assertEqual((row["soft_fallback"],row["degradations"]),("None","[]"))
            self.assertGreater(row["elapsed_ns"],0)
        persistence=proof["raw_persistence"]
        self.assertEqual((persistence["records"],persistence["checkpoints"],persistence["checkpoint_records"]),(128,8,16))
        self.assertEqual(persistence["encoded_bytes"],(root/"out/samples.jsonl").stat().st_size)
        self.assertGreater(persistence["checkpoint_ns"],0)
        self.assertGreaterEqual(persistence["persistence_ns"],persistence["checkpoint_ns"])


if __name__ == "__main__":
    unittest.main()
