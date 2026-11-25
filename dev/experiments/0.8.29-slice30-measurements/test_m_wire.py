"""Authored current wire facts; historical campaign decoders stay frozen."""

import copy
import json
import unittest

from hypothesis import given, settings, strategies as st

from m_wire import decode_tool


DISCLOSURE = ["verdict", "kinds", "counts"]
PROFILE = {"key": "default", "name": "fixture", "revision": "1", "dimension": 8}
AVAILABLE = {
    "status": "available", "population": "stored_mapped_spans", "profile": PROFILE,
    "records": "6", "known_records": "4", "unknown_records": "2",
    "truncated_known_records": "1", "known_body_bytes": "40",
    "unrepresented_known_bytes": "10", "unattributed_heads": "3",
    "truncated_known_share": {"numerator": "1", "denominator": "4"},
    "unrepresented_known_byte_share": {"numerator": "10", "denominator": "40"},
}


def message(text, structured=None):
    """Frame an authored response without changing its payload."""
    result = {"content": [{"type": "text", "text": text}], "isError": False}
    if structured is not None:
        result["structuredContent"] = structured
    return {"result": result}


def orient(coverage):
    """Build the authored orient text with its exact compact byte count."""
    template = '\n'.join([
        'orient store="a" epoch=1 validity_instant=0 tokens={used}/4000',
        'not_in_this_release=' + json.dumps(DISCLOSURE, separators=(',', ':')),
        'projection "body" class=node roles=searchable fts=true vector=false dense=not_declared',
        'embedding: state=blocked usable_embedder=false pending=2 affected_kinds=["note"]',
        'runtime_embedder: available=false reason=no_runtime',
        'vector_unsupported_kinds=["edge"]',
        'coverage: ' + json.dumps(coverage, separators=(',', ':')),
    ])
    used = 0
    for _ in range(4):
        text = template.replace('{used}', str(used))
        count = (len(text.encode()) + 3) // 4
        if used == count:
            return text
        used = count
    raise AssertionError("authored compact budget did not converge")


class CurrentMetadataTests(unittest.TestCase):
    """Retain approved metadata and reject malformed or unknown representations."""
    def test_store_header_retains_required_nullable_session_default_and_rows(self):
        for default, flag in [('"a"', ' default'), ('null', '')]:
            text = f'stores: 2 default={default}\n"a" open=false readiness=closed{flag}\n"b" open=true readiness=ready'
            payload = decode_tool(message(text))["payload"]
            self.assertEqual(payload, {
                "default_store": "a" if flag else None,
                "stores": [{"name": "a", "open": False, "readiness": "closed", "default": bool(flag)},
                           {"name": "b", "open": True, "readiness": "ready", "default": False}],
                "hints": [],
            })
            self.assertEqual(decode_tool(message("unused", payload))["payload"], payload)
        for header in ['stores: 0', 'stores: 0 default=a', 'stores: 0 default=1',
                       'stores: 0 default=true', 'stores: 0 default=[]',
                       'stores: 0 default={} ', 'stores: 0 default=null future=true']:
            with self.subTest(header=header), self.assertRaises(ValueError):
                decode_tool(message(header))

    def test_orient_retains_exact_disclosure_and_both_coverage_variants(self):
        for coverage in [AVAILABLE, {"status": "unavailable", "population": "stored_mapped_spans",
                                    "profile": PROFILE, "reason": "cache_cold"}]:
            text = orient(coverage)
            payload = decode_tool(message(text))["payload"]
            self.assertEqual(payload["not_in_this_release"], DISCLOSURE)
            self.assertEqual(payload["vector_coverage"], coverage)
            self.assertEqual(payload["budget"]["used_tokens"], (len(text.encode()) + 3) // 4)
            self.assertEqual(payload["embedding_readiness"], {"state": "blocked",
                "usable_embedder": False, "pending_count": 2, "affected_kinds": ["note"]})
            self.assertEqual(payload["projection_status"]["vector_unsupported_kinds"], ["edge"])
            self.assertEqual(payload["projections"], [{"entity_class": "node", "name": "body",
                                                     "roles": ["searchable"],
                                                     "fts": True, "vector": False}])
            self.assertEqual(decode_tool(message("unused", payload))["payload"], payload)
            self.assertNotIn("state", payload)
            self.assertNotIn("kinds", payload)
            self.assertNotIn("counts", payload)
        for changed in [orient(AVAILABLE) + '\nfuture: true',
                        orient(AVAILABLE).replace('"counts"]', '"counts","coverage"]'),
                        orient(AVAILABLE).replace('not_in_this_release=', 'missing_disclosure=')]:
            with self.subTest(text=changed), self.assertRaises(ValueError):
                decode_tool(message(changed))

    def test_coverage_keeps_exact_large_decimal_domains_and_null_shares(self):
        coverage = copy.deepcopy(AVAILABLE)
        coverage.update(known_body_bytes=str(2**127 + 1), unrepresented_known_bytes=str(2**127),
            unrepresented_known_byte_share={"numerator": str(2**127), "denominator": str(2**127 + 1)})
        self.assertEqual(decode_tool(message(orient(coverage)))["payload"]["vector_coverage"], coverage)
        empty = dict(coverage, **{name: "0" for name in ["records", "known_records", "unknown_records",
            "truncated_known_records", "known_body_bytes", "unrepresented_known_bytes", "unattributed_heads"]},
            truncated_known_share=None, unrepresented_known_byte_share=None)
        self.assertEqual(decode_tool(message(orient(empty)))["payload"]["vector_coverage"], empty)

    def test_malformed_coverage_never_becomes_a_successful_measurement(self):
        changes = [("records", 6), ("records", "06"), ("records", str(2**63)),
                   ("known_body_bytes", str(2**128)), ("unknown_records", "3"),
                   ("truncated_known_records", "5"), ("unrepresented_known_bytes", "41"),
                   ("truncated_known_share", {"numerator": "1", "denominator": "04"}),
                   ("truncated_known_share", None), ("unexpected", "0")]
        for key, value in changes:
            coverage = dict(AVAILABLE, **{key: value})
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                decode_tool(message(orient(coverage)))
        for profile in [dict(PROFILE, dimension="8"), dict(PROFILE, dimension=True),
                        dict(PROFILE, dimension=0), dict(PROFILE, key="other")]:
            with self.subTest(profile=profile), self.assertRaises(ValueError):
                decode_tool(message(orient(dict(AVAILABLE, profile=profile))))
        unavailable = {"status": "unavailable", "population": "stored_mapped_spans",
                       "profile": PROFILE, "reason": "cache_cold"}
        for changed in [dict(unavailable, records="0"), dict(unavailable, reason="other")]:
            with self.subTest(coverage=changed), self.assertRaises(ValueError):
                decode_tool(message(orient(changed)))


class ExactCoverageProperties(unittest.TestCase):
    """Legal i64/u128 coverage facts survive both wire representations exactly."""

    @settings(max_examples=64, deadline=None, database=None)
    @given(data=st.data())
    def test_exact_coverage_roundtrip(self, data):
        known = data.draw(st.integers(0, 2**63 - 1))
        unknown = data.draw(st.integers(0, 2**63 - 1 - known))
        truncated = data.draw(st.integers(0, known))
        body = data.draw(st.integers(0, 2**128 - 1))
        unseen = data.draw(st.integers(0, body))
        coverage = dict(AVAILABLE,
            profile=dict(PROFILE, dimension=data.draw(st.integers(1, 2**32 - 1))),
            records=str(known + unknown), known_records=str(known), unknown_records=str(unknown),
            truncated_known_records=str(truncated), known_body_bytes=str(body),
            unrepresented_known_bytes=str(unseen),
            unattributed_heads=str(data.draw(st.integers(0, 2**63 - 1))),
            truncated_known_share=None if known == 0 else {
                "numerator": str(truncated), "denominator": str(known)},
            unrepresented_known_byte_share=None if body == 0 else {
                "numerator": str(unseen), "denominator": str(body)})
        text = orient(coverage)
        compact = decode_tool(message(text))["payload"]
        self.assertEqual(compact["vector_coverage"], coverage)
        self.assertEqual(compact["budget"]["used_tokens"], (len(text.encode()) + 3) // 4)
        self.assertEqual(decode_tool(message("unused", compact))["payload"]["vector_coverage"], coverage)
