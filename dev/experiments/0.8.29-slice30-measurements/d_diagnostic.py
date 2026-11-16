"""One private D Storage diagnostic; ordinary measurements remain independent."""

import json
import os
from pathlib import Path
import time


def memory_snapshot():
    """Read raw owning-unit cgroup memory counters without claiming an OOM verdict.

    Return explicit issues when counters are unavailable. Call only in the
    guardian outside product work, before entry and after owned child cleanup.
    """
    result = {'monotonic_ns': time.monotonic_ns(), 'raw': {}, 'issues': []}
    try:
        relative = next(line.split(':', 2)[2] for line in Path('/proc/self/cgroup').read_text().splitlines()
                        if line.startswith('0:'))
        root = Path('/sys/fs/cgroup')/relative.lstrip('/')
        result['path'] = str(root)
        for name in ('memory.events', 'memory.peak', 'memory.current', 'memory.max', 'memory.swap.max'):
            try:
                result['raw'][name] = (root/name).read_text().strip()
            except OSError as error:
                result['issues'].append(f'{name}: {type(error).__name__}: {error}')
    except (OSError, ValueError, StopIteration) as error:
        result['issues'].append(f'owning cgroup unavailable: {type(error).__name__}: {error}')
    result['complete'] = not result['issues']
    return result


def run(options, *, task, started_ns=None):
    """Run one fresh fixed100k/T/overlap diagnostic under existing D accounting.

    This route never reads/writes scientific D state or retries an attempt. Its
    native output and original guardian remain raw diagnostic evidence only.
    """
    from d_attempt import inherited_settings
    from d_runner import run_guarded
    from m_guard import _limits, write_budget
    began = time.monotonic_ns() if started_ns is None else started_ns
    task = Path(task).resolve()
    root, allocation, native = (Path(options[key]).resolve() for key in ('root', 'allocation', 'binary'))
    if (os.environ.get('AGENT_LONG') != '1' or not root.is_relative_to(allocation)
            or not allocation.is_relative_to(task) or not native.is_relative_to(task)
            or not native.is_file() or not os.access(native, os.X_OK) or root.exists()):
        raise ValueError('explicit private diagnostic requires a new owned root and native executable')
    if inherited_settings():
        raise ValueError('inherited experiments would change the diagnostic default workload')
    profile = json.loads(Path(options['profile']).read_text())
    case_ns = 1500*10**9
    if profile.get('whole_cell_ns') != case_ns:
        raise ValueError('private100k diagnostic retains the1500s whole-case envelope')
    def command():
        budget = json.loads((allocation/'g-budget.json').read_text())
        limits = _limits(profile, 'D')
        if budget['used_ns']+case_ns > limits[0]:
            raise RuntimeError('remaining shared allowance cannot fund the one diagnostic case')
        remaining = (began+case_ns-20*10**9-time.monotonic_ns())//10**6
        if remaining <= 0:
            raise RuntimeError('private diagnostic cleanup reserve is unavailable')
        spec = json.loads(Path(__file__).with_name('d-cells.json').read_text())
        cell = next(row for row in spec['cells'] if row['id'] == 'd-t-overlap-n100000-r1')
        config = dict(cell, attempt_id='private-diagnostic-only', remaining_cell_ms=remaining)
        write_budget(root/'cell.json', config)
        return [str(native), str(root/'cell.json'), str(root)]
    guardian = run_guarded(command, allocation=allocation, root=root, task=task, profile=profile,
        common_limit_ns=case_ns, started_ns=began, diagnostic_memory=True)
    return {'scope':'private D SQLite-log diagnostic', 'complete':False, 'qualifies_natural':False,
            'native_result':str(root/'result.json'), 'guardian':guardian}
