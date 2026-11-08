"""Single-purpose stopped-user-benchmark perf attachment and exclusive sample analysis."""

import argparse
import collections
import json
import os
import re
import select
import signal
import subprocess
import tempfile
import time
from pathlib import Path

FOREGROUND = 'ac_011a_write_t'
LIMIT = 256 * 1024 * 1024
HEADER = re.compile(r'^\s*(\S.*?)\s+(\d+)/(\d+)\s+\d+\.\d+:\s+cpu-clock:u:\s*(.*)$')
COMPILERS = {'sqlite3Prepare', 'sqlite3Reprepare', 'sqlite3LockAndPrepare',
             'sqlite3_prepare_v2', 'sqlite3_prepare_v3', 'sqlite3RunParser',
             'sqlite3Parser', 'yy_reduce'}
FTS = {'fts5IndexFlush', 'sqlite3Fts5IndexSync', 'sqlite3Fts5StorageSync', 'fts5SyncMethod'}
GUARDS = {
    'erasure_queue::guard_plain', 'erasure_queue::validate_active',
    'physical_erasure::active', 'physical_erasure::root', 'physical_erasure::state',
    'physical_erasure::load', 'physical_erasure::admission::guard_plain_write',
    'physical_erasure::admission::guard_plain_write_inner',
    'dependency_closure::guard_no_pending_physical',
    'dependency_closure::maintain_before_writer',
    'write_commit::canonical_batch_has_no_custom_triggers',
    'vector_spans::cleanup_trigger_is_valid', 'write_validation::validate_batch',
    'projection_registry::validate_nested_projection_sources_for_write',
}


def _symbol(frame):
    value = re.sub(r'\+0x[0-9a-fA-F]+$', '', frame[0])
    value = re.sub(r'::h[0-9a-f]{16}$|\.(?:llvm|constprop|isra)\..*$', '', value)
    return value


def _known(frame):
    symbol = _symbol(frame)
    return bool(symbol and symbol not in ('[unknown]', 'unknown', '??')
                and not re.fullmatch(r'(?:0x[0-9a-fA-F]+|[0-9a-fA-F]{6,})', symbol))


def _classify(frames):
    for frame in frames:
        if not _known(frame):
            return 'unclassified', None
        symbol = _symbol(frame)
        if symbol in COMPILERS:
            return 'compilation', symbol
        if symbol in FTS or symbol.startswith('fts5IndexMerge'):
            return 'fts_flush_merge', symbol
        if symbol.startswith('fathomdb_engine::') and symbol.removeprefix('fathomdb_engine::') in GUARDS:
            return 'engine_guard', symbol
    return 'other', None


def analyze(text, diagnostics=()):
    """Count exclusive foreground families without treating samples as acceptance evidence."""
    issues = [line.strip() for line in diagnostics if line.strip()]
    samples, current = [], None
    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip() or line.startswith('#'):
            continue
        if 'PERF_RECORD_LOST' in line or re.search(r'\blost (?:events|samples)\b', line, re.I):
            issues.append(f'line {number}: lost events: {line.strip()}')
            continue
        header = HEADER.match(line)
        if header:
            current = dict(comm=header[1].strip(), pid=int(header[2]), tid=int(header[3]), frames=[])
            samples.append(current)
            line = header[4]
            if not line:
                continue
        frame = re.match(r'^\s*[0-9a-fA-F]+\s+(.+)\s+\((.*)\)\s*$', line)
        if frame and current is not None:
            current['frames'].append((frame[1].strip(), frame[2]))
        else:
            issues.append(f'line {number}: unparsed sample/frame: {line.strip()}')
    foreground_tids = {s['tid'] for s in samples if s['comm'] == FOREGROUND}
    if len(foreground_tids) != 1:
        issues.append('foreground numeric TID unresolved or ambiguous')
    foreground_tid = next(iter(foreground_tids)) if len(foreground_tids) == 1 else None
    foreground = [s for s in samples if s['tid'] == foreground_tid]
    workers = collections.Counter(str(s['tid']) for s in samples if s['tid'] != foreground_tid)
    families = dict.fromkeys(('compilation', 'fts_flush_merge', 'engine_guard',
                              'other', 'unclassified'), 0)
    leaves, chains = collections.Counter(), collections.Counter()
    known, unknown_frames = 0, 0
    for sample in foreground:
        frames = sample['frames']
        if not frames:
            issues.append(f'empty/unwound sample on TID {sample["tid"]}')
            frames = [('[unknown]', '[unknown]')]
        known += int(_known(frames[0]))
        unknown_frames += sum(not _known(frame) for frame in frames)
        family, anchor = _classify(frames)
        families[family] += 1
        leaves[frames[0]] += 1
        chains[family, anchor, tuple(frames)] += 1
    total = len(foreground)
    if total == 0 or known * 100 < 95 * total:
        issues.append('fewer than 95 percent known foreground leaves')
    candidates = [name for name in ('compilation', 'fts_flush_merge', 'engine_guard')
                  if families[name] >= 200 and families[name] * 100 >= total * 20]
    if issues:
        candidates = []
    return dict(scope='whole_test_user_cpu_samples_only', foreground_tid=foreground_tid,
                foreground_samples=total, worker_samples=dict(workers), known_leaf_samples=known,
                unknown_leaf_samples=total - known, unknown_frames=unknown_frames,
                known_leaf_fraction=known / total if total else 0,
                exclusive_families=families, candidate_families=candidates,
                decision='inconclusive' if issues else 'supports_focused_design_only'
                if candidates else 'no_qualifying_family', issues=issues,
                top_symbols=[dict(symbol=symbol, dso=dso, samples=count)
                             for (symbol, dso), count in leaves.most_common()],
                top_callchains=[dict(family=family, anchor=anchor, leaf=frames[0][0],
                                     frames=[dict(symbol=s, dso=d) for s, d in frames], samples=count)
                                for (family, anchor, frames), count in chains.most_common()])


def _identity(pid):
    status = Path(f'/proc/{pid}/status').read_text()
    uids = re.search(r'^Uid:\s+(.+)$', status, re.M)[1].split()
    return dict(pid=pid, uids=uids, sid=os.getsid(pid), pgid=os.getpgid(pid),
                autogroup=Path(f'/proc/{pid}/autogroup').read_text().strip())


def _kill_benchmark(child):
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    child.wait()


def start_stopped_benchmark(binary, arguments, stdout, stderr, env):
    """Stop a normal-user launcher without changing its session or autogroup."""
    parent = _identity(os.getpid())
    child = subprocess.Popen(['/bin/sh', '-c', 'kill -STOP "$$"; exec "$@"',
                              'profile-benchmark', binary, *arguments], process_group=0,
                             stdout=stdout, stderr=stderr, env=env)
    try:
        while True:
            waited, status = os.waitpid(child.pid, os.WUNTRACED | os.WNOHANG)
            if waited:
                if not os.WIFSTOPPED(status):
                    child.returncode = os.waitstatus_to_exitcode(status)
                    raise RuntimeError('benchmark launcher exited before STOP')
                break
            time.sleep(.005)
        identity = _identity(child.pid)
        if any(parent[key] != identity[key] for key in ('uids', 'sid', 'autogroup')):
            raise RuntimeError('benchmark UID/session/autogroup changed')
        return child, dict(parent=parent, child=identity, observed_stopped=True)
    except BaseException:
        _kill_benchmark(child)
        raise


def enable_then_continue(control_fd, ack_fd, continuation, pump=lambda: None):
    """Require perf's enable ACK before permitting benchmark continuation."""
    os.write(control_fd, b'enable\n')
    received = b''
    while b'\n' not in received:
        pump()
        ready, _, _ = select.select([ack_fd], [], [], .1)
        if ready:
            received += os.read(ack_fd, 1024)
    if received.split(b'\n', 1)[0] != b'ack':
        raise RuntimeError('perf enable was not acknowledged')
    pump()
    continuation()



def finish_collector(collector, request_stop, pump, drained=lambda: True):
    """Finish collector control before releasing its stopped benchmark target."""
    request_stop()
    while collector.poll() is None or not drained():
        pump()
    return []


def recording_issues(metadata):
    """Return recording failures that prevent a CPU attribution verdict."""
    return [metadata['stop_reason']] if metadata['stop_reason'] else []


def _record(args):
    prefix = Path(args.output_prefix)
    prefix.parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, DEBUGINFOD_URLS='')
    arguments = args.arguments[1:] if args.arguments[:1] == ['--'] else args.arguments
    started = time.monotonic()
    metadata = dict(stop_reason=None, enabled=False, data_bytes=0, benchmark_command=[args.binary, *arguments],
                    perf_version=subprocess.check_output(['/usr/bin/perf', 'version'], text=True).strip())
    child, collector, eof, retaining = None, None, False, True
    with (tempfile.TemporaryDirectory(prefix='cpu-control-', dir=prefix.parent) as directory,
          Path(str(prefix) + '.data').open('wb') as data,
          Path(str(prefix) + '.benchmark.stdout').open('wb') as stdout,
          Path(str(prefix) + '.benchmark.stderr').open('wb') as stderr,
          Path(str(prefix) + '.collector.stderr').open('wb') as collector_errors):
        ctl, ack = [Path(directory) / name for name in ('ctl', 'ack')]
        for path in (ctl, ack):
            os.mkfifo(path, 0o600)
        control_fd, ack_fd = [os.open(path, os.O_RDWR | os.O_NONBLOCK) for path in (ctl, ack)]
        try:
            child, metadata['identity'] = start_stopped_benchmark(args.binary, arguments, stdout, stderr, env)
            command = ['pkexec', '--disable-internal-agent', '/usr/bin/timeout', '--signal=INT',
                       '--kill-after=2', '120', '/usr/bin/perf', 'record', '-D', '-1',
                       f'--control=fifo:{ctl},{ack}', '-e', 'cpu-clock:u', '-F', '99',
                       '--call-graph', 'dwarf,8192', '--max-size=256M', '--no-buildid-cache',
                       '-p', str(child.pid), '-o', '-']
            metadata['collector_command'] = command
            print('Waiting for desktop authentication / perf ACK; benchmark remains stopped.', flush=True)
            collector = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=collector_errors, env=env)
            os.set_blocking(collector.stdout.fileno(), False)

            def pump():
                nonlocal eof, retaining
                if eof:
                    time.sleep(.02)
                    return
                ready, _, _ = select.select([collector.stdout], [], [], .1)
                if ready:
                    chunk = os.read(collector.stdout.fileno(), 65536)
                    if not chunk:
                        eof = True
                        return
                    available = LIMIT - metadata['data_bytes']
                    if retaining:
                        try:
                            data.write(chunk[:available])
                            metadata['data_bytes'] += min(available, len(chunk))
                        except OSError as error:
                            metadata['stop_reason'] = f'profile data write: {error}'
                            retaining = False
                    if len(chunk) >= available:
                        metadata['stop_reason'] = metadata['stop_reason'] or 'hard_256MiB_cap'
                        retaining = False

            def ready_pump():
                pump()
                if metadata['stop_reason'] or collector.poll() is not None:
                    raise RuntimeError('collector stopped before enable ACK')

            def resume():
                metadata['enable_ack_seconds'] = time.monotonic() - started
                metadata['enabled'] = True
                metadata['continued_at'] = time.monotonic()
                os.kill(child.pid, signal.SIGCONT)

            try:
                enable_then_continue(control_fd, ack_fd, resume, ready_pump)
                while child.poll() is None and metadata['stop_reason'] is None:
                    pump()
                    if collector.poll() is not None and child.poll() is None:
                        metadata['stop_reason'] = metadata['stop_reason'] or 'collector_exited_early'
                    if time.monotonic() - metadata['continued_at'] >= 120:
                        metadata['stop_reason'] = 'benchmark_120s_watchdog'
            except BaseException as error:
                metadata['stop_reason'] = metadata['stop_reason'] or f'{type(error).__name__}: {error}'
            finally:
                os.write(control_fd, b'stop\n')
                if metadata['enabled'] and child.poll() is None:
                    _kill_benchmark(child)
                # Auth precedes the privileged timeout. Reserve a stopped PID until
                # the collector is terminal, even if GUI authorization is delayed.
                while collector.poll() is None or not eof:
                    try:
                        pump()
                    except KeyboardInterrupt:
                        metadata['stop_reason'] = 'interrupted_waiting_for_collector'
                        print('Cancel/finish desktop authentication; stopped PID remains reserved.', flush=True)
                metadata['collector_exit'] = collector.wait()
                collector.stdout.close()
        except BaseException as error:
            metadata['stop_reason'] = metadata['stop_reason'] or f'{type(error).__name__}: {error}'
            if collector is not None and collector.poll() is None:
                os.write(control_fd, b'stop\n')
                print('Waiting for collector termination before releasing stopped benchmark PID.', flush=True)
                collector.wait()
        finally:
            if child is not None:
                if child.poll() is None:
                    _kill_benchmark(child)
                metadata['benchmark_exit'] = child.returncode
            os.close(control_fd)
            os.close(ack_fd)
    metadata['elapsed_seconds_including_auth'] = time.monotonic() - started
    if metadata.get('collector_exit') != 0:
        metadata['stop_reason'] = metadata['stop_reason'] or 'collector_nonzero_exit'
    Path(str(prefix) + '.metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
    print(json.dumps(metadata, indent=2))
    return 2 if metadata['stop_reason'] else metadata['benchmark_exit']


def _export_analyze(args):
    prefix = args.output_prefix
    env = dict(os.environ, DEBUGINFOD_URLS='')
    cache = Path(prefix + '.buildids')
    cache.mkdir(parents=True, exist_ok=True)
    command = ['/usr/bin/perf', '--buildid-dir', str(cache), 'script', '--demangle',
               '--show-lost-events', '-F', 'comm,pid,tid,time,event,ip,sym,dso', '-i', prefix + '.data']
    with open(prefix + '.script', 'w') as output, open(prefix + '.script.stderr', 'w') as errors:
        exported = subprocess.run(command, stdout=output, stderr=errors, env=env, check=False)
    metadata = json.loads(Path(prefix + '.metadata.json').read_text())
    issues = Path(prefix + '.script.stderr').read_text().splitlines()
    collector_log = Path(prefix + '.collector.stderr').read_text()
    issues.extend(line for line in collector_log.splitlines()
                  if re.search(r'\blost\b|unwind|warning|truncat|error|failed', line, re.I))
    issues.extend(recording_issues(metadata))
    if exported.returncode:
        issues.append(f'export_exit={exported.returncode}')
    report = analyze(Path(prefix + '.script').read_text(), issues)
    report['export_command'] = command
    report['recording'] = metadata
    Path(prefix + '.report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    return 2 if report['decision'] == 'inconclusive' else 0


def main():
    """Record only an explicitly requested target, or export/analyze retained perf data."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='verb', required=True)
    record = sub.add_parser('record')
    record.add_argument('--output-prefix', required=True)
    record.add_argument('--binary', required=True)
    record.add_argument('arguments', nargs=argparse.REMAINDER)
    analysis = sub.add_parser('analyze')
    analysis.add_argument('--output-prefix', required=True)
    args = parser.parse_args()
    return _record(args) if args.verb == 'record' else _export_analyze(args)


if __name__ == '__main__':
    raise SystemExit(main())
