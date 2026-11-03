"""Measurement arithmetic for native loaded qualification."""
import unittest
from phase2_campaign import summarize


class Accounting(unittest.TestCase):
    def test_success_latency_is_call_time_and_offer_time_separately(self):
        events = []
        for i in range(1000):
            events.extend([
                dict(kind='offer', stream='read', id=i, state='issued', due_ns=0, issued_ns=10),
                dict(kind='finish', stream='read', id=i, state='completed', call_start_ns=20,
                     finished_ns=21+i, service_ns=1+i, offer_latency_ns=21+i)])
        result = summarize(events, 'read', True, 1000)
        self.assertEqual(result['service_p50_ns'], 500)
        self.assertEqual(result['service_p99_ns'], 990)
        self.assertEqual(result['offer_p99_ns'], 1010)
        self.assertEqual(result['accounting_errors'], 0)
        events[1]['service_ns'] += 1
        self.assertGreater(summarize(events, 'read', True, 1000)['accounting_errors'], 0)

    def test_all_dispositions_reconcile_without_qualified_percentiles(self):
        events = [dict(kind='offer', stream='read', id=0, state='generator_missed', due_ns=0)]
        for i, state in enumerate(('completed', 'engine_refused', 'failed'), 1):
            events.extend([
                dict(kind='offer', stream='read', id=i, state='issued', due_ns=0, issued_ns=1),
                dict(kind='finish', stream='read', id=i, state=state, call_start_ns=2,
                     finished_ns=3, service_ns=1, offer_latency_ns=3)])
        events.append(dict(kind='offer', stream='read', id=4, state='issued', due_ns=0, issued_ns=1))
        result = summarize(events, 'read', False, 6)
        self.assertEqual([result[k] for k in ('completed', 'engine_refused', 'failed', 'generator_missed', 'unfinished', 'unknown')], [1]*6)
        self.assertEqual(result['issued'], 4)
        self.assertEqual(result['accounting_errors'], 0)
        self.assertIsNone(result['service_p99_ns'])
        self.assertIsNone(result['offer_p99_ns'])
