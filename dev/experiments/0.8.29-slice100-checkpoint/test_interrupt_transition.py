"""Harmless scripted-MI ordering regression; no debugger, ptrace, signal or product."""

import importlib.util
import json
import os
from pathlib import Path
import pty
import selectors
import subprocess
import sys
import tempfile
import time
import tty
import unittest
from types import SimpleNamespace
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("slice100_transition_control", Path(__file__).with_name("operator.py"))
control = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(control)


def mi_child(case, nonce, executable, sqlite, signal_name):
    """Model the reviewed debugger signal command; never deliver a real fault signal."""
    target, stage, killed, commands = None, 0, False, []
    frame = dict(addr="0x1234", func="walCheckpoint", line=str(control.COPY_LINE), fullname=sqlite)
    values = dict(rc="0", pIter="0x1", nBackfill="0", mxSafeFrame="4", iFrame="2", iDbpage="1", mxPage="3", szPage="4096", eMode="3")
    for raw in sys.stdin:
        text = raw.strip()
        commands.append(text)
        (case / 'script-commands.json').write_text(json.dumps(commands))
        if text == 'SCRIPTED_EXTERNAL_SIGKILL':
            raise RuntimeError('retired qualifying pidfd delivery path')
        token = text[:len(text) - len(text.lstrip('0123456789'))]
        command = text[len(token):]
        response, event = 'done', None
        if command.startswith('-inferior-tty-set '):
            target = json.loads(command.split(' ', 1)[1])
        elif command == '-interpreter-exec console "signal SIGKILL"':
            response = 'running'
            event = 'reason="exited-signalled",signal-name=' + json.dumps(signal_name)
        elif command.startswith('-interpreter-exec '):
            event = 'reason="signal-received"'
        elif command.startswith('-break-insert '):
            response += ',bkpt={number="' + ('1' if stage == 0 else '2') + '"}'
        elif command == '-exec-continue':
            response = 'running'
            if stage == 0:
                ready = dict(nonce=nonce, fixture=control.FIXTURE, pid=os.getpid(), executable=executable, ticket='scripted-ticket')
                with open(target, 'w') as stream:
                    stream.write('Q1_READY ' + json.dumps(ready) + '\n')
                event = 'reason="breakpoint-hit",bkptno="1"'
                stage = 1
            else:
                event = 'reason="breakpoint-hit",bkptno="2",thread-id="2",stopped-threads="all",frame={' + ','.join(k + '=' + json.dumps(v) for k, v in frame.items()) + '}'
                stage = 2
        elif command == '-stack-list-frames':
            names = ['sqlite3WalCheckpoint', 'wal_checkpoint_truncate_once', 'checkpoint_erasure_at_rest', 'complete_physical_erasure']
            response += ',stack=[' + ','.join('frame={func=' + json.dumps(name) + '}' for name in names) + ']'
        elif command.startswith('-data-evaluate-expression '):
            response += ',value=' + json.dumps(values[json.loads(command.split(' ', 1)[1])])
        elif command.startswith('-symbol-list-lines '):
            response += ',lines=[{pc="0x1234",line="' + str(control.COPY_LINE) + '"}]'
        elif command.startswith('-data-disassemble '):
            response += ',asm_insns=[{address="0x1234",inst="scripted"}]'
        elif command == '-thread-select 2':
            response += ',new-thread-id="2",frame={' + ','.join(k + '=' + json.dumps(v) for k, v in frame.items()) + '}'
        elif command == '-thread-info':
            response += ',current-thread-id="2",threads=[{id="1",state="stopped"},{id="2",state="stopped"}]'
        elif command == '-list-thread-groups --recurse 1':
            response += ',groups=[{id="i1",type="process",pid="' + str(os.getpid()) + '",executable=' + json.dumps(executable) + ',threads=[{id="1"},{id="2"}]}]'
        elif command == '-stack-info-frame':
            response += ',frame={' + ','.join(k + '=' + json.dumps(v) for k, v in frame.items()) + '}'
        elif command == '-gdb-exit':
            print(token + '^exit', flush=True)
            return
        print(token + '^' + response, flush=True)
        if event:
            print('*stopped,' + event, flush=True)


class ScriptedDebugger(control.Debugger):
    """Exercise the real MI parser/pump/command against a harmless transport child."""

    def __init__(self, args, case, nonce, actors):
        self.args, self.case, self.nonce, self.actors = args, case, nonce, actors
        self.deadline, self.token = time.monotonic() + 15, 0
        self.records, self.events, self.ready, self.done = [], [], [], []
        self.resources, self.next_sample = [], 0
        self.logs = {name: (case / (name + '.log')).open('wb') for name in ['mi', 'target']}
        self.buffers, self.counts = dict(mi=b'', target=b''), dict(mi=0, target=0)
        self.master, self.slave = pty.openpty()
        tty.setraw(self.slave)
        self.tty_path = os.ttyname(self.slave)
        self.process = subprocess.Popen([sys.executable, '-I', str(Path(__file__).resolve()), '--mi-child', str(case), nonce,
                                         str(args.binary), str(args.sqlite_source), args.scripted_signal],
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        actors.processes.append(self.process)
        # This names only the transport; native ownership/identity is not proved here.
        self.inferior_pid = actors.inferior = self.process.pid
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ, 'mi')
        self.selector.register(self.master, selectors.EVENT_READ, 'target')
        (case / 'preimage.json').write_text(json.dumps(dict(nonce=nonce, ticket='scripted-ticket')))
        (case / 'candidate.sqlite').write_bytes(b'scripted transport bytes, not a database')


def supervise(directory, expected_signal):
    """Run the interrupt branch with synthetic proof subjects and an outer safeguard."""
    args = SimpleNamespace(technical_go=True, mode='interrupt', source_root=directory, task_root=directory,
                           case_root=directory/'case', healthy_result=directory/'healthy.json',
                           binary=Path(sys.executable).resolve(), sqlite_source=directory/'scripted.c',
                           scripted_signal=expected_signal)
    args.healthy_result.write_text('{}')
    current = {}

    def construct(*values):
        debugger = ScriptedDebugger(*values)
        current['debugger'] = debugger
        return debugger

    def scripted_signal(_actors):
        # An input record only; this is not an OS signal or a real fault.
        debugger = current['debugger']
        debugger.process.stdin.write(b'SCRIPTED_EXTERNAL_SIGKILL\n')
        debugger.process.stdin.flush()

    try:
        with patch.object(control, 'source_subject', return_value={'scripted_transport_only': True, 'sqlite_source': str(args.sqlite_source)}), \
             patch.object(control, 'healthy_matches'), patch.object(control, 'resource_sample', return_value={}), \
             patch.object(control, 'check_resources'), patch.object(control.Actors, 'check_inferior'), \
             patch.object(control.Actors, 'kill_inferior', scripted_signal), \
             patch.object(control, 'Debugger', side_effect=construct), patch.object(control, 'utility_child') as utility:
            result = control.run_case(args)
        print(json.dumps(dict(status=result['status'], error=result.get('error'), utility_calls=utility.call_count,
                              commands=json.loads((args.case_root/'script-commands.json').read_text()))), flush=True)
    finally:
        debugger = current.get('debugger')
        if debugger and debugger.process.poll() is None:
            debugger.process.kill()
            debugger.process.wait(timeout=3)


@unittest.skipUnless(sys.platform == 'linux', 'Linux owned operator transport control')
class InterruptTransition(unittest.TestCase):
    """One scripted debugger signal is required; wrong-signal exit remains unqualified."""

    def test_stopped_transport_uses_single_debugger_signal_before_exit_notification(self):
        for expected_signal in ['SIGKILL', 'SIGTERM']:
            with self.subTest(expected_signal=expected_signal), tempfile.TemporaryDirectory(prefix='s100-transition-') as tmp:
                child = subprocess.Popen([sys.executable, '-I', str(Path(__file__).resolve()), '--supervise', tmp, expected_signal],
                                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
                try:
                    try:
                        stdout, stderr = child.communicate(timeout=12)
                    except subprocess.TimeoutExpired:
                        os.killpg(child.pid, 9)
                        stdout, stderr = child.communicate(timeout=3)
                        self.fail('outer transport deadline: ' + stderr.decode())
                    self.assertEqual(child.returncode, 0, stdout.decode() + stderr.decode())
                    result = json.loads(stdout)
                    if expected_signal == 'SIGKILL':
                        self.assertEqual(result['status'], 'QUALIFIED_INTERRUPTION', result)
                        self.assertEqual(result['utility_calls'], 2)
                        self.assertFalse(any('SCRIPTED_EXTERNAL_SIGKILL' in command for command in result['commands']))
                        self.assertEqual(sum(command.endswith('-interpreter-exec console "signal SIGKILL"') for command in result['commands']), 1)
                        selected = next(i for i, command in enumerate(result['commands']) if command.endswith('-thread-select 2'))
                        delivered = next(i for i, command in enumerate(result['commands']) if command.endswith('-interpreter-exec console "signal SIGKILL"'))
                        self.assertFalse(any('-exec-continue' in command for command in result['commands'][selected:delivered]))
                    else:
                        self.assertEqual(result['status'], 'UNQUALIFIED')
                        self.assertIn('exact owned SIGKILL not observed', result['error'])
                        self.assertEqual(result['utility_calls'], 0)
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid, 9)
                        child.wait(timeout=3)
                    child.stdout.close()
                    child.stderr.close()


if __name__ == '__main__':
    if len(sys.argv) == 7 and sys.argv[1] == '--mi-child':
        mi_child(Path(sys.argv[2]), *sys.argv[3:])
    elif len(sys.argv) == 4 and sys.argv[1] == '--supervise':
        supervise(Path(sys.argv[2]), sys.argv[3])
    else:
        unittest.main()
