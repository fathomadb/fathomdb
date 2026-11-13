"""Synthetic MI/guard controls only: no debugger, process, product or native imports."""

import copy
import importlib.util
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("slice100_operator", Path(__file__).with_name("operator.py"))
control = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(control)


def synthetic_witness():
    """Return a human-specified positive protocol transcript, never product evidence."""
    subject = {"sqlite_source": "/owned/sqlite3.c", "binary_sha256": "synthetic-only"}
    witness = {"subject": subject, "pid": 123, "nonce": "case", "ready_count": 1, "done_count": 0,
               "frame": {"func": "walCheckpoint", "line": "69275", "addr": "0x1010", "fullname": "/owned/sqlite3.c"},
               "relocated_line_addresses": [0x1010], "instruction_address": 0x1010,
               "stack": [{"func": n} for n in ["walCheckpoint", "sqlite3WalCheckpoint", "Engine::wal_checkpoint_truncate_once", "Engine::checkpoint_erasure_at_rest", "ErasureMaintenance::complete_physical_erasure"]],
               "locals": dict(rc="0", pIter="0x100", nBackfill="1", mxSafeFrame="3", iFrame="2", iDbpage="1", mxPage="4", szPage="4096", eMode="3")}
    return subject, witness


class ParserControls(unittest.TestCase):
    """Exercise real MI grammar and strict failure paths without constructing a debugger."""

    def test_token_tuple_and_repeated_frame_list(self):
        record = control.parse_mi('12^done,stack=[frame={level="0",func="walCheckpoint"},frame={level="1",func="sqlite3WalCheckpoint"}]')
        self.assertEqual(record["token"], "12")
        self.assertEqual([x["frame"]["level"] for x in record["values"]["stack"]], ["0", "1"])

    def test_async_stop_and_c_escapes(self):
        self.assertEqual(control.parse_mi('*stopped,reason="breakpoint-hit",bkptno="2"')["values"]["bkptno"], "2")
        self.assertEqual(control.parse_mi('~"hello\\n\\033\\\""')["stream"], 'hello\n\x1b"')

    def test_rejects_malformed_or_ambiguous_records(self):
        for line in ['not MI', '^done,value="x"trailing', '^done,a="1",a="2"', '^done,a={x="1",x="2"}', '^done,a=["unfinished"', '^done,a=bare', '~"unterminated', '~"bad\\q"']:
            with self.subTest(line=line), self.assertRaises(control.Unqualified):
                control.parse_mi(line)

    def test_unavailable_native_scalar_is_not_zero(self):
        for value in ['<optimized out>', '0x100 <unverified>', 'NaN', '', None, '1 + 1']:
            with self.subTest(value=value), self.assertRaises(control.Unqualified):
                control.number(value)
        self.assertEqual(control.number("0x100"), 256)
        self.assertEqual(control.number("4096"), 4096)


class WitnessControls(unittest.TestCase):
    """Every negative is explicitly a NEW operator refusal test, not a product RED."""

    def test_complete_synthetic_witness_is_accepted(self):
        subject, w = synthetic_witness()
        control.validate_witness(w, subject, 123, "case")

    def test_actor_subject_and_event_binding(self):
        for key, value in [('pid', 124), ('nonce', 'wrong'), ('subject', {}), ('ready_count', 0), ('ready_count', 2), ('done_count', 1)]:
            subject, w = synthetic_witness()
            w[key] = value
            with self.subTest(key=key, value=value), self.assertRaises(control.Unqualified):
                control.validate_witness(w, subject, 123, "case")

    def test_wrong_source_position_and_relocation(self):
        for key, value in [('func', 'sqlite3WalCheckpoint'), ('line', '69278'), ('addr', '0x9999'), ('fullname', '/other/sqlite3.c')]:
            subject, w = synthetic_witness()
            w['frame'][key] = value
            with self.subTest(key=key), self.assertRaises(control.Unqualified):
                control.validate_witness(w, subject, 123, "case")
        subject, w = synthetic_witness()
        w['instruction_address'] += 1
        with self.assertRaises(control.Unqualified):
            control.validate_witness(w, subject, 123, "case")

    def test_missing_erasure_callstack_cannot_qualify_boot_checkpoint(self):
        subject, w = synthetic_witness()
        for index in range(1, len(w['stack'])):
            bad = copy.deepcopy(w)
            del bad['stack'][index]
            with self.subTest(index=index), self.assertRaises(control.Unqualified):
                control.validate_witness(bad, subject, 123, "case")

    def test_all_native_frame_preconditions_are_required(self):
        for key, value in [('rc', '1'), ('pIter', '0x0'), ('nBackfill', '3'), ('iFrame', '1'), ('iFrame', '4'), ('iDbpage', '0'), ('iDbpage', '5'), ('szPage', '0'), ('eMode', '0'), ('rc', '<optimized out>')]:
            subject, w = synthetic_witness()
            w['locals'][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(control.Unqualified):
                control.validate_witness(w, subject, 123, "case")
        subject, w = synthetic_witness()
        del w['locals']['pIter']
        with self.assertRaises(control.Unqualified):
            control.validate_witness(w, subject, 123, "case")

    def test_healthy_requires_same_subject_real_normal_completion_fields(self):
        subject, w = synthetic_witness()
        record = dict(status='QUALIFIED_HEALTHY', subject=subject, witness=w, pid=123, nonce='case', exit=0, done_count=1)
        control.healthy_matches(record, subject)
        for field, value in [('exit', 73), ('exit', 99), ('done_count', 0), ('status', 'UNQUALIFIED'), ('subject', {})]:
            bad = copy.deepcopy(record)
            bad[field] = value
            with self.subTest(field=field, value=value), self.assertRaises(control.Unqualified):
                control.healthy_matches(bad, subject)

    def test_scope_approval_cannot_launch_without_technical_go(self):
        with patch.object(control.subprocess, 'Popen', side_effect=AssertionError('no process permitted')), self.assertRaises(control.Unqualified):
            control.run_case(SimpleNamespace(technical_go=False))

    def test_resource_floor_and_exact_owned_cap(self):
        sample = dict(root_free=32*control.GIB, task_free=128*control.GIB, memory_available=16*control.GIB, owned=control.GIB)
        control.check_resources(sample, start=True)
        for key, value in [('root_free', 20*control.GIB-1), ('task_free', 64*control.GIB-1), ('memory_available', 8*control.GIB-1), ('owned', 64*control.GIB)]:
            bad = dict(sample, **{key: value})
            with self.subTest(key=key), self.assertRaises(control.Unqualified):
                control.check_resources(bad)


if __name__ == '__main__':
    unittest.main()
