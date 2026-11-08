"""Retained bounded AC-011 sync tracing and whole-test attribution; no acceptance verdict."""

import argparse
import json
import os
import re
import select
import signal
import subprocess
import time
from decimal import Decimal
from pathlib import Path

FIXTURE = 'AC-011a_write_throughput.sqlite'
FOREGROUND = 'ac_011a_write_t'
PREFIX = re.compile(r'^(?:\[pid\s+(\d+)\]|(\d+))\s+\d+\.\d+\s+(.*)$')
CALL = re.compile(r'^(fsync|fdatasync|prctl)\((.*)\)\s+=\s+(.+?)\s+<(\d+\.\d+)>$')
RESUMED = re.compile(r'^<\.\.\. (\w+) resumed>(.*)$')


def analyze(lines, fixture_basename=FIXTURE, external_issues=()):
    """Attribute complete strace spans by numeric TID/path, preserving errors and loss."""
    issues = list(external_issues)
    pending, foreground, events, fixture_paths = {}, set(), [], set()
    for number, raw in enumerate(lines, 1):
        raw = raw.strip()
        if not raw:
            continue
        prefix = PREFIX.match(raw)
        if not prefix:
            issues.append(f'line {number}: unresolved TID or malformed record')
            continue
        tid, body = int(prefix[1] or prefix[2]), prefix[3]
        if body.startswith(('+++', '---')):
            continue
        resumed = RESUMED.match(body)
        if resumed:
            initial = pending.pop(tid, None)
            if initial is None or not initial.startswith(resumed[1] + '('):
                issues.append(f'line {number}: unmatched resumed call on TID {tid}')
                continue
            body = initial + resumed[2]
        if body.endswith('<unfinished ...>'):
            if tid in pending:
                issues.append(f'line {number}: overwritten unfinished call on TID {tid}')
            pending[tid] = body.removesuffix('<unfinished ...>').rstrip()
            continue
        call = CALL.match(body)
        if not call:
            issues.append(f'line {number}: unparsed syscall on TID {tid}')
            continue
        name, arguments, result, duration = call.groups()
        returned = result.split()[0]
        success = returned == '0'
        error = returned.startswith('-') or returned == '?'
        if error or (name != 'prctl' and not success):
            issues.append(f'line {number}: {name} error on TID {tid}: {result}')
        if name == 'prctl':
            match = re.match(r'PR_SET_NAME,\s*"([^\"]+)"', arguments)
            if success and match and match[1] == FOREGROUND:
                foreground.add(tid)
            continue
        fd = re.match(r'\d+<([^>]+)>$', arguments.strip())
        path = fd[1].removesuffix(' (deleted)') if fd else None
        if not path or not path.startswith('/'):
            issues.append(f'line {number}: unresolved FD path on TID {tid}')
            path = '<unresolved>'
        for suffix in ('/' + fixture_basename, '/' + fixture_basename + '-wal'):
            if path.endswith(suffix):
                fixture_paths.add(path.removesuffix('-wal') if suffix.endswith('-wal') else path)
        events.append((tid, path, success, Decimal(duration)))
    for tid in pending:
        issues.append(f'unfinished call remains on TID {tid}')
    if len(foreground) != 1:
        issues.append(f'foreground TID unresolved or ambiguous: {sorted(foreground)}')
    if len(fixture_paths) != 1:
        issues.append(f'fixture DB path unresolved or ambiguous: {sorted(fixture_paths)}')
    foreground_tid = next(iter(foreground)) if len(foreground) == 1 else None
    fixture_db = next(iter(fixture_paths)) if len(fixture_paths) == 1 else None
    groups = {}
    observed = Decimal(0)
    for tid, path, success, duration in events:
        if fixture_db and path == fixture_db:
            kind = 'db'
        elif fixture_db and path == fixture_db + '-wal':
            kind = 'wal'
        elif fixture_db and path == str(Path(fixture_db).parent):
            kind = 'directory'
        else:
            kind = 'other'
        key = tid, path
        if key not in groups:
            groups[key] = dict(tid=tid, role='foreground' if tid == foreground_tid else 'other',
                               path=path, path_kind=kind, calls=0, successes=0, errors=0,
                               sum_seconds=Decimal(0), max_seconds=Decimal(0))
        group = groups[key]
        group['calls'] += 1
        group['successes' if success else 'errors'] += 1
        group['sum_seconds'] += duration
        group['max_seconds'] = max(group['max_seconds'], duration)
        if tid == foreground_tid and kind in ('db', 'wal'):
            observed += duration
    for group in groups.values():
        for field in ('sum_seconds', 'max_seconds'):
            group[field] = str(group[field])
    decision = ('inconclusive' if issues else 'material_observed_sync_spans'
                if observed >= Decimal('5.0') else 'below_material_span_criterion')
    return dict(scope='whole_test_startup_warmup_measurement_teardown',
                foreground_tid=foreground_tid, fixture_db=fixture_db,
                foreground_fixture_sync_seconds=str(observed), criterion_seconds='5.0',
                decision=decision, issues=issues, groups=sorted(groups.values(),
                                                            key=lambda g: (g['tid'], g['path'])))


def _analyze_files(prefix, metadata, fixture_basename):
    issues = []
    if metadata['stop_reason']:
        issues.append('runner stopped: ' + metadata['stop_reason'])
    if metadata['return_code'] not in (0, 101):
        issues.append(f'unexpected traced command exit: {metadata["return_code"]}')
    stderr = Path(str(prefix) + '.stderr').read_text(errors='replace')
    for line in stderr.splitlines():
        if line.startswith('strace:'):
            issues.append('tracer diagnostic/fallback: ' + line)
    with Path(str(prefix) + '.trace').open(errors='replace') as stream:
        report = analyze(stream, fixture_basename, issues)
    report['runner'] = metadata
    return report


def _terminate(process):
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        pass
    # The tracer can exit before a tracee; still kill surviving owned members.
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def _run(args):
    prefix = Path(args.output_prefix)
    prefix.parent.mkdir(parents=True, exist_ok=True)
    read_fd, write_fd = os.pipe()
    arguments = args.arguments[1:] if args.arguments[:1] == ['--'] else args.arguments
    command = ['strace', '--seccomp-bpf', '-q', '-f', '-ttt', '-T', '-yy',
               '-e', 'trace=fsync,fdatasync,prctl', '-o', f'/proc/self/fd/{write_fd}',
               args.binary, *arguments]
    started = time.monotonic()
    deadline = started + args.timeout_seconds
    reason, written, process = None, 0, None
    with (Path(str(prefix) + '.trace').open('wb') as trace,
          Path(str(prefix) + '.stdout').open('wb') as stdout,
          Path(str(prefix) + '.stderr').open('wb') as stderr):
        try:
            process = subprocess.Popen(command, pass_fds=(write_fd,), start_new_session=True,
                                       stdout=stdout, stderr=stderr)
        except OSError as error:
            reason = f'tracer setup: {error}'
        finally:
            os.close(write_fd)
        try:
            while process is not None:
                remaining_time = deadline - time.monotonic()
                if remaining_time <= 0:
                    reason = '120s_watchdog' if args.timeout_seconds == 120 else 'watchdog'
                    break
                ready, _, _ = select.select([read_fd], [], [], min(remaining_time, .25))
                if not ready:
                    continue
                chunk = os.read(read_fd, 65536)
                if not chunk:
                    break
                available = args.max_bytes - written
                trace.write(chunk[:available])
                written += min(len(chunk), available)
                if len(chunk) >= available:
                    reason = 'trace_cap'
                    break
            if process is not None:
                if reason:
                    _terminate(process)
                else:
                    try:
                        process.wait(timeout=max(0, deadline - time.monotonic()))
                    except subprocess.TimeoutExpired:
                        reason = 'watchdog'
                        _terminate(process)
        except BaseException:
            if process is not None:
                _terminate(process)
            raise
        finally:
            os.close(read_fd)
    metadata = dict(command=command, return_code=process.returncode if process else None,
                    stop_reason=reason, trace_bytes=written, trace_cap_bytes=args.max_bytes,
                    watchdog_seconds=args.timeout_seconds,
                    elapsed_seconds=time.monotonic() - started, seccomp_bpf_requested=True)
    Path(str(prefix) + '.metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
    report = _analyze_files(prefix, metadata, args.fixture_basename)
    Path(str(prefix) + '.report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    return 2 if report['decision'] == 'inconclusive' else process.returncode


def main():
    """Run one explicitly requested bounded trace, or analyze its retained files."""
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='verb', required=True)
    for name in ('run', 'analyze'):
        command = sub.add_parser(name)
        command.add_argument('--output-prefix', required=True)
        command.add_argument('--fixture-basename', default=FIXTURE)
    run = sub.choices['run']
    run.add_argument('--binary', required=True)
    run.add_argument('--max-bytes', type=int, default=32 * 1024 * 1024)
    run.add_argument('--timeout-seconds', type=float, default=120)
    run.add_argument('arguments', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.verb == 'run':
        if args.max_bytes <= 0 or args.timeout_seconds <= 0:
            parser.error('trace limit and watchdog must be positive')
        return _run(args)
    metadata = json.loads(Path(args.output_prefix + '.metadata.json').read_text())
    report = _analyze_files(args.output_prefix, metadata, args.fixture_basename)
    print(json.dumps(report, indent=2))
    return 2 if report['decision'] == 'inconclusive' else 0


if __name__ == '__main__':
    raise SystemExit(main())
