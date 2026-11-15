"""Memory-only MI/operator regressions; no child, native import, signal or database."""

import io
import json
from pathlib import Path
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from test_operator import control, synthetic_witness


class MemoryActors:
    """Synthetic ownership facts only; never an actual pidfd or process proof."""

    def __init__(self):
        self.inferior, self.adopted_cleanup, self.pidfd_requests = 123, [], []
        self.checks, self.reject_after_checks = 0, None

    def begin_launch_scope(self):
        pass

    def check_inferior(self):
        self.checks += 1
        if self.reject_after_checks and self.checks >= self.reject_after_checks:
            raise control.Unqualified('synthetic actor identity changed')

    def kill_inferior(self):
        self.pidfd_requests.append(self.inferior)

    def cleanup(self):
        return []


class MemoryDebugger(control.Debugger):
    """Use real command/stop dispatch with parsed MI packets and in-memory I/O."""

    def __init__(self, args, case, nonce, actors, arrival, configure):
        self.args, self.case, self.nonce, self.actors = args, case, nonce, actors
        self.token, self.deadline = 0, time.monotonic() + 60
        self.records, self.events, self.ready, self.done, self.resources = [], [], [], [], []
        self.inferior_pid, self.tty_path = actors.inferior, '/synthetic/tty'
        self.process = SimpleNamespace(stdin=io.BytesIO(), wait=lambda timeout: 0)
        self.offset, self.stage, self.delayed = 0, 0, None
        self.commands, self.arrival, self.error_record = [], arrival, None
        self.selected, self.revalidating = '2', False
        self.subject, self.native = synthetic_witness()
        self.subject['binary'] = str(args.binary)
        self.native.update(pid=actors.inferior, nonce=nonce)
        self.group_pid, self.group_threads, self.other_running = '123', ['1', '2'], False
        self.fresh_frame, self.fresh_locals, self.ready_mutation = {}, {}, {}
        self.stop_all, self.stop_thread, self.exit_signal = 'all', '2', 'SIGKILL'
        self.group_binary = str(args.binary)
        configure(self)
        (case/'preimage.json').write_text(json.dumps(dict(nonce=nonce, ticket='ticket')))

    def pump(self, until):
        if self.delayed is not None:
            self.records.append(self.delayed)
            self.delayed = None
            return
        wire = self.process.stdin.getvalue()[self.offset:]
        self.offset += len(wire)
        if not wire:
            raise control.Unqualified('actor/phase deadline')
        line = wire.decode().strip()
        token = line[:len(line)-len(line.lstrip('0123456789'))]
        command = line[len(token):]
        self.commands.append(command)
        values, event, cls = {}, None, 'done'
        if command.startswith('-interpreter-exec console "starti"'):
            event = dict(reason='signal-received')
        elif command.startswith('-break-insert '):
            values = dict(bkpt=dict(number='1' if self.stage == 0 else '2'))
        elif command == '-exec-continue' or command == '-interpreter-exec console "signal SIGKILL"':
            cls = 'running'
            if self.stage == 0:
                self.ready.append(dict(nonce=self.nonce, fixture=control.FIXTURE, pid=123,
                                       executable=str(self.args.binary), ticket='ticket'))
                event, self.stage = dict(bkptno='1', reason='breakpoint-hit'), 1
            elif self.stage == 1:
                event = dict(bkptno='2', reason='breakpoint-hit', frame=self.native['frame'],
                             **{'thread-id': self.stop_thread, 'stopped-threads': self.stop_all})
                self.stage = 2
            elif self.arrival:
                self.error_record = control.parse_mi(token+'^error,msg="Command aborted."')
                self.records.append(control.parse_mi(token+'^running'))
                if self.arrival == 'together':
                    self.records.append(self.error_record)
                else:
                    self.delayed = self.error_record
                return
            else:
                event = dict(reason='exited-signalled', **{'signal-name': self.exit_signal})
        elif command == '-stack-list-frames':
            values = dict(stack=[dict(frame=f) for f in self.native['stack']])
        elif command.startswith('-data-evaluate-expression '):
            name = json.loads(command.split(' ', 1)[1])
            values = dict(value=(self.fresh_locals if self.revalidating else {}).get(name, self.native['locals'][name]))
        elif command.startswith('-symbol-list-lines '):
            values = dict(lines=[dict(pc='0x1010', line='69275')])
        elif command.startswith('-data-disassemble '):
            values = dict(asm_insns=[dict(address='0x1010', inst='synthetic')])
        elif command.startswith('-thread-select '):
            self.revalidating = True
            self.ready[0].update(self.ready_mutation)
            values = {'new-thread-id': self.selected, 'frame': self.native['frame']}
        elif command == '-thread-info':
            values = {'current-thread-id': self.selected, 'threads': [
                dict(id='1', state='running' if self.other_running else 'stopped'),
                dict(id='2', state='stopped')]}
        elif command == '-list-thread-groups --recurse 1':
            values = dict(groups=[dict(id='i1', type='process', pid=self.group_pid,
                                      executable=self.group_binary, threads=[dict(id=i) for i in self.group_threads])])
        elif command == '-stack-info-frame':
            values = dict(frame=dict(self.native['frame'], **self.fresh_frame))
        elif command == '-gdb-exit':
            cls = 'exit'
        self.records.append(dict(token=token, kind='^', **{'class': cls}, values=values))
        if event is not None:
            self.events.append(dict(token='', kind='*', **{'class': 'stopped'}, values=event))

    def finish(self):
        self.process.stdin.close()


def memory_case(arrival=None, configure=lambda debugger: None):
    """Exercise actual run_case while all external/native actors and utilities are fenced."""
    with tempfile.TemporaryDirectory(prefix='s100-memory-mi-') as tmp:
        root = Path(tmp)
        args = SimpleNamespace(technical_go=True, mode='interrupt', source_root=root/'source',
                               task_root=root, case_root=root/'case', healthy_result=root/'healthy.json',
                               binary=Path('/synthetic/q1'), sqlite_source=Path('/owned/sqlite3.c'))
        args.healthy_result.write_text('{}')
        subject, _ = synthetic_witness()
        subject['binary'] = str(args.binary)
        actors, instances = MemoryActors(), []

        def construct(*arguments):
            instance = MemoryDebugger(*arguments, arrival, configure)
            instances.append(instance)
            return instance

        sample = dict(root_free=64*control.GIB, task_free=256*control.GIB,
                      memory_available=32*control.GIB, owned=control.GIB)
        fake_ctypes = SimpleNamespace(CDLL=lambda *a, **k: SimpleNamespace(prctl=lambda *a: 0))
        with patch.dict(sys.modules, ctypes=fake_ctypes), \
             patch.object(control, 'source_subject', return_value=subject), \
             patch.object(control, 'resource_sample', return_value=sample), \
             patch.object(control, 'healthy_matches'), patch.object(control, 'Actors', return_value=actors), \
             patch.object(control, 'Debugger', side_effect=construct), \
             patch.object(control.subprocess, 'Popen', side_effect=AssertionError('no child permitted')), \
             patch.object(control, 'snapshot_database', return_value=root/'untouched') as snapshot, \
             patch.object(control, 'utility_child') as utility:
            result = control.run_case(args)
        return result, instances[0], actors, snapshot.call_count, utility.call_count


class LateExecutionError(unittest.TestCase):
    """Both observed packet timings must retain the debugger failure and refuse recovery."""

    def check_arrival(self, arrival):
        result, debugger, _, snapshots, utilities = memory_case(arrival)
        self.assertEqual(result['status'], 'UNQUALIFIED')
        self.assertNotIn('fault_exit', result)
        self.assertEqual((snapshots, utilities), (0, 0))
        self.assertEqual(result['error'], 'Unqualified: debugger command failed: '+json.dumps(debugger.error_record))

    def test_running_and_error_in_one_batch(self):
        self.check_arrival('together')

    def test_error_in_later_batch(self):
        self.check_arrival('separate')

    def test_normal_stop_without_error_is_preserved(self):
        debugger = control.Debugger.__new__(control.Debugger)
        debugger.deadline = time.monotonic()+5
        debugger.records = []
        debugger.events = [control.parse_mi('*stopped,reason="exited-normally"')]
        self.assertEqual(debugger.stopped(1), dict(reason='exited-normally'))


class StoppedDelivery(unittest.TestCase):
    """Reviewed delivery policy only; synthetic MI is never real signal/exit proof."""

    signal_command = '-interpreter-exec console "signal SIGKILL"'

    def test_one_debugger_signal_after_revalidation_without_pidfd_duplicate(self):
        result, debugger, actors, snapshots, utilities = memory_case()
        self.assertEqual(result['status'], 'QUALIFIED_INTERRUPTION', result)
        self.assertEqual(actors.pidfd_requests, [])
        self.assertEqual(debugger.commands.count(self.signal_command), 1)
        selected = debugger.commands.index('-thread-select 2')
        delivered = debugger.commands.index(self.signal_command)
        for command in ['-thread-info', '-list-thread-groups --recurse 1', '-stack-info-frame',
                        '-stack-list-frames', '-data-evaluate-expression "iFrame"']:
            self.assertIn(command, debugger.commands[selected:delivered])
        self.assertFalse(any(command.startswith('-exec-') for command in debugger.commands[selected:delivered]))
        self.assertGreaterEqual(actors.checks, 4)
        self.assertEqual((snapshots, utilities), (1, 2))
        self.assertEqual(result['fault_exit'], dict(reason='exited-signalled', **{'signal-name': 'SIGKILL'}))

    def test_changed_delivery_preconditions_refuse_before_any_signal_or_recovery(self):
        cases = {
            'not-all-stopped': lambda d: setattr(d, 'stop_all', '2'),
            'missing-witness-thread': lambda d: setattr(d, 'stop_thread', None),
            'wrong-selected-thread': lambda d: setattr(d, 'selected', '1'),
            'wrong-group-pid': lambda d: setattr(d, 'group_pid', '124'),
            'wrong-group-executable': lambda d: setattr(d, 'group_binary', '/other/q1'),
            'thread-not-in-group': lambda d: setattr(d, 'group_threads', ['1']),
            'another-thread-running': lambda d: setattr(d, 'other_running', True),
            'changed-frame': lambda d: setattr(d, 'fresh_frame', dict(addr='0x9999')),
            'empty-native-work': lambda d: setattr(d, 'fresh_locals', dict(iFrame='1')),
            'changed-ready-nonce': lambda d: setattr(d, 'ready_mutation', dict(nonce='foreign')),
            'changed-actor': lambda d: setattr(d.actors, 'reject_after_checks', 3),
        }
        for name, configure in cases.items():
            with self.subTest(name=name):
                result, debugger, actors, snapshots, utilities = memory_case(configure=configure)
                self.assertEqual(result['status'], 'UNQUALIFIED', result)
                self.assertEqual(actors.pidfd_requests, [])
                self.assertNotIn(self.signal_command, debugger.commands)
                self.assertNotIn('fault_exit', result)
                self.assertEqual((snapshots, utilities), (0, 0))

    def test_wrong_exit_signal_refuses_without_fallback_or_recovery(self):
        result, debugger, actors, snapshots, utilities = memory_case(configure=lambda d: setattr(d, 'exit_signal', 'SIGTERM'))
        self.assertEqual(result['status'], 'UNQUALIFIED')
        self.assertIn('exact owned SIGKILL not observed', result['error'])
        self.assertEqual(debugger.commands.count(self.signal_command), 1)
        self.assertEqual(actors.pidfd_requests, [])
        self.assertEqual((snapshots, utilities), (0, 0))

    def test_delivery_error_never_redelivers_or_qualifies_recovery(self):
        result, debugger, actors, snapshots, utilities = memory_case('separate')
        self.assertEqual(result['status'], 'UNQUALIFIED')
        self.assertIn('Command aborted.', result['error'])
        self.assertEqual(debugger.commands.count(self.signal_command), 1)
        self.assertEqual(actors.pidfd_requests, [])
        self.assertEqual((snapshots, utilities), (0, 0))


if __name__ == '__main__':
    unittest.main()
