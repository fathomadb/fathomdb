"""One private D Storage diagnostic; ordinary measurements remain independent."""

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
