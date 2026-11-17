"""Exact paired record comparison and descriptive statistics from retained native calls."""

from collections import defaultdict
import json
from pathlib import Path


def native_result(step):
    """Load actual checkpointed streams, including P's nonembedded call populations."""
    result = dict(step.get('result') or {})
    output = Path(step['output'])
    for name, files in {'samples': ['samples'], 'warmups': ['warmups'],
                        'first_calls': ['first-calls'], 'batches': ['batches'],
                        'probes': ['marker-probes', 'g-marker-probes'],
                        'observation_batches': ['observations']}.items():
        found = [output / (item + '.jsonl') for item in files if (output / (item + '.jsonl')).exists()]
        if found:
            result[name] = [json.loads(line) for path in found for line in path.read_text().splitlines()]
        else:
            result.setdefault(name, [])
    if 'open_report' in result:
        result['open_reports'] = [result['open_report']]
    return result


def _native(row):
    if not row.get('attempt'):
        return None
    try:
        result = json.loads((Path(row['attempt']) / 'result.json').read_text())
        steps = result.get('steps') or []
        if len(steps) != 1:
            return None
        value = native_result(steps[0])
        value['native_member_qualified'] = (result.get('complete') is True
            and result.get('qualification_issues') == [] and steps[0].get('complete') is True
            and not value.get('runner_only'))
        return value
    except (OSError, ValueError, KeyError):
        return None


def clean(row):
    """Require successful comparison candidates with known strict attempt cleanup."""
    return (row.get('status') == 'complete' and row.get('complete') is True
            and row.get('cleanup_uncertain') is False and row.get('live_owned_pids') == [])


def _natural_gw(members):
    return bool(members) and all(cell.get('family') == 'G-W' and cell.get('kind') == 'write'
        and cell.get('provider') == 'default' and cell.get('purpose') in ('ordinary', 'diagnostic')
        and len(cell.get('adapters', [])) == 1 and cell.get('routes') in (['text'], ['hybrid'])
        for cell in members)


def _u64(value):
    return type(value) is int and 0 <= value < 2**64


def _natural_issues(cell, value):
    issues = []
    def bad(path):
        issues.append(path)
    def readiness(record, path, state_key='state'):
        if not isinstance(record, dict):
            bad(path); return
        pending = record.get('pending_count')
        if (not _u64(pending) or record.get('usable_embedder') is not True
                or record.get(state_key) != ('ready' if pending == 0 else 'processing')):
            bad(path)
    before, after = value.get('before_workload'), value.get('after_workload')
    for name, facts in (('before_workload', before), ('after_workload', after)):
        if not isinstance(facts, dict):
            bad('/'+name); continue
        readiness(facts, '/'+name, 'readiness')
        if (facts.get('format') != 37 or facts.get('natural_records') != cell['n']
                or not isinstance(facts.get('generation_id'), str) or not facts['generation_id']
                or not _u64(facts.get('generation_failed')) or facts['generation_failed'] != 0
                or not _u64(facts.get('generation_pending'))):
            bad('/'+name)
    if isinstance(before, dict) and isinstance(after, dict) and before.get('generation_id') != after.get('generation_id'):
        bad('/after_workload/generation_id')
    samples, warmups, batches, probes = (value.get(key) for key in ('samples', 'warmups', 'batches', 'probes'))
    if any(not isinstance(rows, list) or any(not isinstance(row, dict) for row in rows)
           for rows in (samples, warmups, batches, probes)):
        return sorted(set(issues+['/natural_streams']))
    ratio, size = cell.get('searches_per_batch'), cell.get('batch_size')
    if type(ratio) is not int or ratio <= 0 or type(size) is not int or size not in (1, 10):
        return sorted(set(issues+['/natural_schedule']))
    expected = (cell['samples']+ratio-1)//ratio
    if len(batches) != expected or len(probes) != expected*size:
        bad('/natural_cohort_population')
    floor = 0
    for index, row in enumerate(warmups):
        cursor = row.get('projection_cursor')
        if not _u64(cursor) or cursor < floor:
            bad(f'/warmups/{index}/projection_cursor')
        else:
            floor = cursor
    by_sample = {}
    probes_by_sample = defaultdict(list)
    for index, probe in enumerate(probes):
        before_sample = probe.get('before_sample')
        if type(before_sample) is not int or before_sample < 0:
            bad(f'/probes/{index}/before_sample')
        else:
            probes_by_sample[before_sample].append((index, probe))
    previous_receipt = floor
    for index, batch in enumerate(batches):
        path = f'/batches/{index}'
        readiness(batch.get('adjacent_readiness'), path+'/adjacent_readiness')
        ready = batch.get('adjacent_readiness')
        ready = ready if isinstance(ready, dict) else {}
        if (ready.get('available') is not True or ready.get('observation_scope')
                != 'adjacent public readiness; not query snapshot'):
            bad(path+'/adjacent_readiness')
        receipt = batch.get('after_write_cursor')
        if (not _u64(receipt) or receipt <= previous_receipt or batch.get('rows') != size
                or batch.get('before_sample') != index*ratio or batch.get('cohort_index') != index):
            bad(path+'/write_receipt')
        if _u64(receipt):
            previous_receipt = receipt
        by_sample[index*ratio] = (index, receipt)
    upper = None
    for index, row in enumerate(samples):
        if index in by_sample:
            cohort, upper = by_sample[index]
            group = probes_by_sample.get(index, [])
            if len(group) != size:
                bad(f'/probes/cohort/{cohort}')
            for probe_index, probe in group:
                path = f'/probes/{probe_index}'
                if (probe.get('cohort_index') != cohort or probe.get('marker_current') is not True
                        or not isinstance(probe.get('point_probe'), dict)
                        or probe['point_probe'].get('current') is not True):
                    bad(path+'/current_marker')
                text = probe.get('text_probe')
                cursor = text.get('projection_cursor') if isinstance(text, dict) else None
                if not _u64(cursor) or not _u64(upper) or not floor <= cursor <= upper:
                    bad(path+'/text_probe/projection_cursor')
                else:
                    floor = cursor
        cursor = row.get('projection_cursor')
        if not _u64(cursor) or not _u64(upper) or not floor <= cursor <= upper:
            bad(f'/samples/{index}/projection_cursor')
        else:
            floor = cursor
    return sorted(set(issues))


def _natural_comparison(members, values):
    """Compare results while retaining four explicitly scoped, validated observations."""
    from g_pair import compare_results
    paths = (('samples', ('projection_cursor',)), ('probes', ('text_probe', 'projection_cursor')),
             ('batches', ('adjacent_readiness', 'pending_count')),
             ('batches', ('adjacent_readiness', 'state')))
    differences = []
    counts = defaultdict(int)
    for stream, fields in paths:
        left_rows, right_rows = values[0].get(stream), values[1].get(stream)
        if not isinstance(left_rows, list) or not isinstance(right_rows, list):
            continue
        for index, (left, right) in enumerate(zip(left_rows, right_rows)):
            try:
                for field in fields:
                    left, right = left[field], right[field]
            except (KeyError, TypeError):
                continue
            if left != right:
                path = '/'+stream+'/'+str(index)+'/'+'/'.join(fields)
                differences.append({'path': path, members[0]['slice50_pair_mode']: left,
                                    members[1]['slice50_pair_mode']: right})
                counts['/'+stream+'/*/'+'/'.join(fields)] += 1
    projected = []
    for value in values:
        current = dict(value)
        for stream, fields in paths:
            if not isinstance(current.get(stream), list):
                continue
            rows = []
            for original in current[stream]:
                if not isinstance(original, dict):
                    rows.append(original); continue
                row = dict(original)
                if len(fields) == 1:
                    row.pop(fields[0], None)
                elif isinstance(row.get(fields[0]), dict):
                    row[fields[0]] = dict(row[fields[0]])
                    row[fields[0]].pop(fields[1], None)
                rows.append(row)
            current[stream] = rows
        projected.append(current)
    issues = {cell['slice50_pair_mode']: _natural_issues(cell, value)
              for cell, value in zip(members, values)}
    return {'result_equal': compare_results(*projected), 'observation_differences': differences,
            'observation_difference_counts': dict(counts), 'observation_issues': issues}


def comparisons(cells, rows):
    """Compare every declared pair; partial/failed data remains visible and uncredited."""
    from g_pair import compare_results
    grouped = defaultdict(list)
    for cell in cells:
        grouped[(cell['origin_id'], cell['purpose'])].append(cell)
    pairs = []
    for (origin, purpose), members in grouped.items():
        outcomes = [rows.get(cell['id'], {}) for cell in members]
        values = [_native(row) for row in outcomes]
        equal = compare_results(*values) if all(value is not None for value in values) else None
        natural = (_natural_comparison(members, values) if _natural_gw(members)
                   and all(value is not None for value in values) else None)
        parity = (natural['result_equal'] and not any(natural['observation_issues'].values())
                  if natural is not None else equal is True)
        complete = (all(clean(row) for row in outcomes) and parity
                    and all(value.get('native_member_qualified') and calls_complete(cell, value)
                            for cell, value in zip(members, values, strict=True)))
        pairs.append({'origin_id': origin, 'purpose': purpose, 'members': [cell['id'] for cell in members],
            'equal': equal, 'complete': complete,
            'comparison_scope': 'natural_gw_results' if natural is not None else 'full_streams',
            **(natural or {}),
            'status': ('natural_results_complete' if natural is not None else 'complete') if complete
                      else 'mismatch' if equal is False else 'incomplete',
            'statistics': _statistics(members, values) if complete else None})
    return pairs


def _statistics(cells, values):
    from metrics import aggregate, paired_difference
    groups = defaultdict(list)
    for cell, result in zip(cells, values, strict=True):
        for row in result['samples']:
            adapter = row.get('adapter', cell['adapters'][0])
            route = row.get('route', 'text')
            groups[(adapter, route)].append((cell, row))
    data = []
    for (adapter, route), records in groups.items():
        modes = {}
        paired = []
        for mode in ('winner', 'eager'):
            samples = [row for cell, row in records if cell['slice50_pair_mode'] == mode]
            modes[mode] = aggregate(samples, 0)
            paired.extend(dict(row, adapter='engine' if mode == 'winner' else 'sdk',
                               repeat=cells[0]['repeat']) for row in samples)
        interval = paired_difference(paired)
        interval['eager_minus_winner_mean_ms'] = interval.pop('sdk_minus_engine_mean_ms')
        data.append({'adapter': adapter, 'route': route, 'modes': modes, 'paired': interval})
    return data


def diagnostics(row):
    """Expose owned body work and sampled child RSS with missing facts unavailable."""
    value = _native(row)
    work = [record for batch in (value or {}).get('observation_batches', [])
            for record in batch.get('records', []) if record.get('phase') == 'slice50_work']
    rss = []
    if row.get('attempt'):
        try:
            actual = json.loads((Path(row['attempt']) / 'result.json').read_text())
            rss = [sample for step in actual.get('steps', []) for sample in step.get('rss_samples', [])]
        except (OSError, ValueError):
            pass
    known = [sample for sample in rss if sample.get('available')]
    return {'work_records': work or None, 'rss_samples': rss or None,
            'sampled_max_rss_bytes': max(sample['rss_bytes'] for sample in known) if known else None,
            'sampled_peak_rss_bytes': max(sample['peak_rss_bytes'] for sample in known) if known else None,
            'scope': 'named diagnostic only; process RSS/HWM sampled at retained cadence; unsampled peaks and total allocation/SQLite I/O unavailable'}


def report(root):
    """Retain every scope and require independently completed exact pairs for credit."""
    from g_pair import plan
    root = Path(root)
    path = root / 'results/campaign-state.json'
    previous = {row['id']: row for row in json.loads(path.read_text())} if path.exists() else {}
    result = {'scope': 'slice50-pair', 'complete': False, 'comparisons': {}}
    inventory = plan()
    for name, cells in inventory.items():
        pairs = comparisons(cells, previous)
        credited = {identifier for pair in pairs if pair['complete'] for identifier in pair['members']}
        result['comparisons'][name] = pairs
        result[name] = [dict(previous.get(cell['id'], {}), id=cell['id'], cell=cell,
            status=previous.get(cell['id'], {}).get('status', 'not_run'),
            credited=cell['id'] in credited,
            diagnostic=diagnostics(previous.get(cell['id'], {})) if cell['observe'] else None)
            for cell in cells]
    result['calibration_complete'] = all(pair['complete'] for pair in result['comparisons']['calibration'])
    result['complete'] = all(pair['complete'] for name in ('primary', 'diagnostics')
                             for pair in result['comparisons'][name])
    result['future_costs'] = {'P_preparation': None, '100k': None, 'writes': None,
                              'scope': 'unobserved future cases remain unavailable; a finite required prefix does not establish full-table fit'}
    return result


def calls_complete(cell, result):
    """Require every ordered raw call, its exact returned record fields and finite clock."""
    from collections import Counter
    scale = len(cell['adapters']) * len(cell['routes'])
    for stream, count in (('samples', cell['samples']), ('warmups', cell['warmup']),
                          ('first_calls', int(cell.get('record_first_call', True) and cell['kind'] != 'write'))):
        rows = result.get(stream, [])
        if len(rows) != count * scale:
            return False
        groups = Counter()
        indices = defaultdict(list)
        for row in rows:
            key = (row.get('adapter', cell['adapters'][0]), row.get('route', 'text'))
            if (row.get('ok') is not True or not isinstance(row.get('hits'), list)
                    or type(row.get('elapsed_ns')) is not int or row['elapsed_ns'] < 0):
                return False
            groups[key] += 1
            indices[key].append(row.get('query_index'))
        if groups != Counter({(adapter, route): count for adapter in cell['adapters'] for route in cell['routes']}):
            return False
        if any(values != [index % 500 for index in range(count)] for values in indices.values()):
            return False
    return True


def published_report(root):
    """Bind retained native comparisons to actual final execution/report guardian cleanup."""
    root = Path(root)
    result = json.loads((root / 'results/report.json').read_text())
    receipts = [json.loads(path.read_text()) for path in (root / 'guardians').glob('*/guardian.json')]
    def final(owner, guardian_root=None):
        selected = receipts if guardian_root is None or Path(guardian_root).resolve() == root.resolve() else [
            json.loads(path.read_text()) for path in (Path(guardian_root) / 'guardians').glob('*/guardian.json')]
        return next((row for row in selected
                     if row.get('started_monotonic_ns') == owner.get('started_monotonic_ns')
                     and row.get('controller_pid') == owner.get('controller_pid')), None)
    def healthy(receipt):
        boundary = (receipt or {}).get('boundary') or {}
        return (receipt is not None and receipt.get('exit') in (0, 1)
                and receipt.get('cleanup_uncertain') is False and not receipt.get('stop')
                and not receipt.get('error') and boundary.get('status') == 'quiescent'
                and boundary.get('populated_after') is False
                and boundary.get('removed_exact_empty_boundary') is True)
    receipt = final(result.get('guardian_owner') or {})
    good = healthy(receipt)
    result['guardian'] = receipt
    result['phase_qualified'] = good
    for name in ('primary', 'diagnostics', 'calibration'):
        for row in result[name]:
            row['credited'] = bool(row.get('credited') and good
                and healthy(final(row.get('guardian_owner') or result.get('guardian_owner') or {}, row.get('guardian_root'))))
    result['complete'] = bool(result.get('native_primary_complete') and good
        and all(row['credited'] for name in ('primary', 'diagnostics') for row in result[name]))
    credited = {row['id'] for name in ('primary', 'diagnostics') for row in result[name] if row.get('credited')}
    active = result.get('active_members', [])
    result['window_complete'] = bool(result.get('native_window_complete') and good and active
                                     and all(identifier in credited for identifier in active))
    result['calibration_complete'] = bool(result.get('native_calibration_complete') and good
        and all(row['credited'] for row in result['calibration']))
    return result
