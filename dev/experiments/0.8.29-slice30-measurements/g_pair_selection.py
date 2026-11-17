"""Explicit prospective S50 selections; original freeze and cumulative ledger remain authoritative."""

import json
from pathlib import Path

from g_campaign import active_bytes


def apply(manifest, request):
    """Overlay only approved prospective controls on one exact immutable primary campaign."""
    from g_pair import approved
    from g_pair_freeze import git, primary_limits
    record = json.loads(Path(request).read_text()) if not isinstance(request, dict) else request
    selected = record.get('selection', record)
    required = {'schema', 'root', 'manifest_uuid', 'profile', 'primary_limits', 'retire_complete_copies'}
    optional = {'g_donors', 'model_cache', 'gpu_uuid', 'gpu_ordinal', 'report_recovery'}
    if (set(selected) - required - optional or not required.issubset(selected)
            or selected['schema'] != 'slice50-pair-selection/v1' or manifest['scope'] != 'primary'
            or Path(selected['root']).resolve() != Path(manifest['root']).resolve()
            or selected['manifest_uuid'] != manifest['uuid']):
        raise ValueError('selection must name the exact existing primary campaign')
    profile = selected['profile']
    approved(profile, manifest['allocation'])
    limits = primary_limits(selected)
    if (profile['total_shared_ns'] > 48*3600*10**9 or profile['total_shared_retained_bytes'] > 192*1024**3
            or limits['phase_ns'] > profile['total_shared_ns']
            or limits['new_retained_bytes'] > profile['total_shared_retained_bytes']
            or type(selected['retire_complete_copies']) is not bool):
        raise ValueError('finite selected total/phase/growth/retirement bounds required')
    current = git('rev-parse', 'HEAD')
    if 'selection' in record and record.get('controller_source_sha') != current:
        raise ValueError('selection controller source differs from actual current controller')
    actual = dict(manifest, profile=profile, primary_limits=limits,
        retire_complete_copies=selected['retire_complete_copies'], common_limit_ns=limits['common_ns'],
        _original_pair_ns=manifest['primary_limits']['pair_ns'], _selection=selected,
        controller_source_sha=current)
    if not isinstance(request, dict):
        actual['_selection_path'] = str(Path(request).resolve())
    if 'selection' in record:
        actual['_selection_record'] = record
    if 'report_recovery' in selected:
        if not isinstance(selected['report_recovery'], str) or not selected['report_recovery']:
            raise ValueError('report-only recovery requires original adoption')
        actual['_report_recovery_selection'] = selected['report_recovery']
    if 'g_donors' in selected:
        donors = selected['g_donors']
        if (not isinstance(donors, dict) or not donors or set(donors) - {'10000', '100000'}
                or any(not isinstance(path, str) or not path for path in donors.values())):
            raise ValueError('qualified G donor roots must be explicitly selected by population')
        actual['g_donors'] = dict(manifest.get('g_donors', {}), **donors)
    if 'model_cache' in selected:
        if not isinstance(selected['model_cache'], str) or not selected['model_cache']:
            raise ValueError('explicit G model cache path required')
        actual['model_cache'] = selected['model_cache']
    if 'gpu_uuid' in selected or 'gpu_ordinal' in selected:
        if (not isinstance(selected.get('gpu_uuid'), str) or not selected['gpu_uuid']
                or type(selected.get('gpu_ordinal')) is not int or selected['gpu_ordinal'] < 0):
            raise ValueError('explicit G device UUID and ordinal required')
        actual['requested_gpu'] = {'uuid': selected['gpu_uuid'], 'ordinal': selected['gpu_ordinal']}
    return actual


def adopt(manifest):
    """Record an explicit selection under the admitted guardian's existing exclusive lock."""
    from g_pair_runtime import assess_primary
    root = Path(manifest['root'])
    allocation = Path(manifest['allocation'])
    budget = json.loads((allocation / 'g-budget.json').read_text())
    owner = budget.get('s50_guardian') or {}
    if (any(budget.get(key) for key in ('inflight', 'm_guardian', 'd_guardian', 'closed_guardian'))
            or owner.get('root') != manifest['root'] or owner.get('status') is not None):
        raise RuntimeError('retained shared ownership/inflight blocks selection adoption')
    assess_primary(manifest, budget, active_bytes(allocation))
    directory = root / 'selections'
    directory.mkdir(exist_ok=True)
    path = directory / f"{owner['started_monotonic_ns']}-{owner['pid']}.json"
    record = {'schema': 'slice50-pair-adoption/v1', 'selection': manifest['_selection'],
        'guardian_owner': owner, 'controller_source_sha': manifest['controller_source_sha'],
        'native_source_sha': manifest['source_sha'], 'compiler_inputs': manifest['compiler_inputs']}
    with path.open('x') as stream:
        stream.write(json.dumps(record, indent=2, allow_nan=False) + '\n')
    path.chmod(0o444)
    return path


def pair_allowance(manifest, budget, cell):
    """Keep every attempted pair's original cap and cumulative charges across selections."""
    cells = manifest['plan']['primary'] + manifest['plan']['diagnostics']
    index = next(index for index, item in enumerate(cells) if item['id'] == cell['id'])
    members = cells[index-index%2:index-index%2+2]
    key = members[0]['id']
    limit = budget.get('s50_pair_limits', {}).get(key)
    if limit is None and any(budget.get('cell_used_ns', {}).get(member['id'], 0) for member in members):
        limit = manifest.get('_original_pair_ns', manifest['primary_limits']['pair_ns'])
    return key, min(manifest['primary_limits']['pair_ns'], limit) if limit is not None else manifest['primary_limits']['pair_ns']
