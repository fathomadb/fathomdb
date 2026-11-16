"""Private Slice50 scope inside the existing P/G measurement entry."""

import json
import os
import shutil
import subprocess
import sys
import time
import uuid
from pathlib import Path

TOOLS = Path(__file__).resolve().parent
CALIBRATION_ID = "S50-CAL:g-r-n10000-r0"
FEATURES = {"ordinary":["embed-cuda","slice50-measurement"],
            "diagnostic":["embed-cuda","slice50-measurement","slice30-observer"]}


def plan():
    """Derive exact search obligations without changing original P/G inventories."""
    from campaign import make_plan
    from g_campaign import make_g_plan
    bases = []
    for cell in make_plan(json.loads((TOOLS / "cells.json").read_text()), 2):
        bases.append(dict(cell, routes=["text"], provider="none", record_first_call=False,
                          kind="write" if cell["family"] == "P2" else "read"))
    for cell in make_g_plan(json.loads((TOOLS / "g-cells.json").read_text())):
        if cell["stage"] not in ("G-R", "G-W") or cell.get("routes") == ["embed"]:
            continue
        bases.append(dict(cell, family=cell["stage"],
                          routes=[route for route in cell["routes"] if route != "embed"]))
    diagnostic_ids = {"p1-n100000-r0", "p2-b10-s1-r0", "p2-b1-s100-r0", "g-r-n100000-r0"}
    diagnostic_ids.update(f"g-w-b{batch}-s{ratio}-{adapter}-{route}-r0"
                          for batch, ratio in ((10, 1), (1, 100))
                          for adapter in ("engine", "sdk") for route in ("text", "hybrid"))

    def members(cell, purpose, diagnostic=False):
        modes = ["winner", "eager"] if cell["repeat"] == 0 else ["eager", "winner"]
        return [dict(cell, id=f"s50-{cell['id']}-{purpose}-{mode}", origin_id=cell["id"],
                     purpose=purpose, slice50_pair_mode=mode, observe=diagnostic,
                     artifact="diagnostic" if diagnostic else "ordinary",
                     fresh_copy=True, fresh_seed=False, preparation="closed-copy-v1")
                for mode in modes]
    calibration = next(cell for cell in bases if cell["id"] == "g-r-n10000-r0")
    return {"primary": [item for cell in bases for item in members(cell, "ordinary")],
            "diagnostics": [item for cell in bases if cell["id"] in diagnostic_ids
                            for item in members(cell, "diagnostic", True)],
            "calibration": members(calibration, "calibration-ordinary")
                           + members(calibration, "calibration-diagnostic", True)}


def report(root):
    """Report every missing/failed/refused member without cross-crediting scopes."""
    from g_pair_report import report as summarize
    return summarize(root)


def build_tools(output, *, _run=None):
    """Build both private artifacts; injected harmless builders remain source-only."""
    from g_freeze import SOURCE, compiler_inputs, git, identify
    if _run is None and git("status", "--porcelain"):
        raise ValueError("pair build requires clean current source")
    if _run is None and str(Path('/usr/local/cuda').resolve()) != '/usr/local/cuda-12.6':
        raise ValueError('private pair build requires unchanged primary CUDA12.6')
    environment = dict(os.environ, CUDA_COMPUTE_CAP='86', CUDA_HOME='/usr/local/cuda-12.6',
                       CUDA_PATH='/usr/local/cuda-12.6')
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=False)
    receipt = {"schema":"slice50-pair-build/v1", "scope":"slice50-pair",
               "source_sha":git("rev-parse","HEAD"), "compiler_inputs":compiler_inputs(),
               "lock":identify(SOURCE / "Cargo.lock"), "runner_only":_run is not None,
               "binaries":{}}
    for variant, features in FEATURES.items():
        command = ["cargo","build","--offline","--locked","--release","--jobs","2",
                   "-p","fathomdb-sdk","--features",",".join(features),"--example","slice30_measure"]
        (_run or subprocess.run)(command, cwd=SOURCE, env=environment, check=True)
        built = Path(os.environ.get("CARGO_TARGET_DIR",str(SOURCE/'target'))) / 'release/examples/slice30_measure'
        target = output / ('slice30-measure-'+variant)
        shutil.copy2(built,target)
        target.chmod(0o555)
        receipt['binaries'][variant] = dict(identify(target),features=list(features),command=command)
    (output/'build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    return receipt


def assemble_manifest(options, build, inputs, donors):
    """Bind validated current build/data to private canonical obligations."""
    actual_features = {name:list(build['binaries'][name]['features']) for name in FEATURES}
    if actual_features != FEATURES:
        raise ValueError("current private pair build/features required")
    return {"schema":"slice50-paired-campaign/v1", "scope":options.get('scope','primary'),
            "source_sha":build['source_sha'], "compiler_inputs":build['compiler_inputs'],
            "build":build, "features":actual_features, "inputs":inputs, "donors":donors,
            "plan":plan(), "allocation":options['allocation'],
            "profile":options['profile'], "retire_complete_copies":options.get('retire_complete_copies',False)}


def admit(profile, ledger, retained_bytes, limits):
    """Reserve the one explicitly selected cumulative lower-stage allowance."""
    from m_calibration import admit as shared_admit
    validate_limits(limits)
    return shared_admit(profile, ledger, retained_bytes, limits, calibration_id=CALIBRATION_ID)


def retire_pair(root, outcomes, *, enabled, comparison, diagnostics_complete):
    """Retire only complete, compared and quiescent private working copies."""
    if (not enabled or not comparison or not diagnostics_complete or len(outcomes) < 2
            or any(row.get('status') != 'complete' or row.get('complete') is not True
                   or row.get('cleanup_uncertain') or row.get('live_owned_pids') != [] for row in outcomes)):
        return []
    root = Path(root).resolve()
    stores = [Path(row['store']).resolve() for row in outcomes]
    if any(not path.is_relative_to(root) for path in stores):
        raise ValueError('working copy is outside the private pair root')
    retired = []
    for store in stores:
        for suffix in ('', '-wal', '-shm'):
            path = store.with_name(store.name+suffix)
            if path.exists():
                retired.append({'path':str(path),'bytes':path.stat().st_size})
                path.unlink()
    return retired


def compare_results(left, right):
    """Compare exact per-call search results without comparing runtime clocks."""
    def stable(value):
        if isinstance(value, dict):
            return {key:stable(item) for key,item in value.items()
                    if key not in {'elapsed_ns','write_ns','started_ns','ended_ns','wall_ns'}}
        return [stable(item) for item in value] if isinstance(value,list) else value
    return all(stable(left.get(stream,[])) == stable(right.get(stream,[]))
               for stream in ('samples','warmups','first_calls','batches','probes'))


def validate_limits(limits):
    """Bound this cumulative full-pair calibration without changing M limits."""
    from m_calibration import validate_limits as shared_limits
    shared_limits(limits)
    if limits['payload_ns'] > 800*10**9 or limits['new_retained_bytes'] > 2*1024**3:
        raise ValueError('paired calibration exceeds selected work/growth maxima')


def drive_members(manifest, root, execute):
    """Drive exactly the active scope through existing admitted member execution."""
    root = Path(root)
    state = root / 'results/campaign-state.json'
    state.parent.mkdir(parents=True, exist_ok=True)
    previous = {row['id']:row for row in json.loads(state.read_text())} if state.exists() else {}
    cells = manifest.get('_active_members')
    if cells is None:
        cells = manifest['plan']['calibration'] if manifest['scope'] == 'calibration' else manifest['plan']['primary'] + manifest['plan']['diagnostics']
    rows = []
    for index, cell in enumerate(cells):
        old = previous.get(cell['id'])
        outcome = old if old and old.get('complete') is True else execute(cell, index)
        rows.append(outcome)
        previous[cell['id']] = outcome
        state.write_text(json.dumps(list(previous.values()),indent=2)+'\n')
    return rows


def qualify_step(step, cell, manifest):
    """Require actual current native facts; source-only helpers cannot qualify."""
    issues = []
    result = native_result(step) if step.get('output') else step.get('result') or {}
    if result.get('runner_only'):
        issues.append('runner_only_source_control')
    if not step.get('complete') or result.get('complete') is not True:
        issues.append('native_member_incomplete')
    rows = result.get('samples', []) + result.get('warmups', []) + result.get('first_calls', [])
    if (len(result.get('samples', [])) != cell['samples']*len(cell['adapters'])*len(cell['routes'])
            or len(result.get('warmups', [])) != cell['warmup']*len(cell['adapters'])*len(cell['routes'])
            or any(row.get('ok') is not True or not isinstance(row.get('hits'), list) for row in rows)):
        issues.append('exact_native_call_records_missing')
    closed = result.get('close_outcomes') or result.get('passes') or []
    if (len(closed) != len(cell['adapters'])
            or any((row.get('status') != 'ok' if 'status' in row else
                    type(row.get('close_ns')) is not int or row['close_ns'] < 0) for row in closed)):
        if not (len(cell['adapters']) == 1 and type(result.get('close_ns')) is int and result['close_ns'] >= 0):
            issues.append('native_close_facts_missing')
    reports = result.get('open_reports') or []
    if len(reports) != len(cell['adapters']):
        issues.append('actual_native_reports_missing')
    for opened in reports:
        if (opened.get('pid') != step.get('pid') or opened.get('executable') != step.get('executable')
                or opened.get('requested_provider') != cell['provider'] or opened.get('schema_version_after') != 37):
            issues.append('current_native_identity_or_provider_mismatch')
    scale = len(cell['adapters'])*len(cell['routes'])
    if result.get('sample_count') != cell['samples']*scale or result.get('warmup_count') != cell['warmup']*scale:
        issues.append('actual_native_population_mismatch')
    if cell['family'].startswith('G') and step.get('config'):
        from g_runner import qualify_step as qualify_g
        issues.extend(qualify_g(step,manifest))
    elif cell['family'].startswith('G'):
        issues.append('actual_native_config_missing')
    elif (result.get('natural_records') != cell['n'] or result.get('vector_rows') != 0
          or result.get('marker_records') != (10 if cell['family']=='P2' else 0)):
        issues.append('provider_none_fixture_mismatch')
    if cell['observe']:
        batches = result.get('observation_batches') or []
        if (not batches or any(batch.get('dropped_records') != 0 or batch.get('issues') != []
                or not any(row.get('phase')=='slice50_work' and row.get('status')=='ok'
                           for row in batch.get('records',[])) for batch in batches)):
            issues.append('diagnostic_work_unavailable')
        if not any(row.get('available') for row in step.get('rss_samples',[])):
            issues.append('diagnostic_rss_unavailable')
    return sorted(set(issues))


def approved(profile, allocation):
    """Require explicit Slice50 authority on the unchanged shared ledger and history."""
    if (profile.get('schema') != 'slice50-pair-budget/v1' or profile.get('status') != 'APPROVED'
            or profile.get('whole_cell_ns') != 900*10**9
            or Path(profile.get('ledger','')).resolve() != Path(allocation).resolve()/'g-budget.json'
            or any(type(profile.get(key)) is not int or profile[key] <= 0
                   for key in ('total_shared_ns','total_shared_retained_bytes'))):
        raise ValueError('approved private Slice50 profile on original shared ledger required')
    if (any(type(profile.get(key)) is not int or profile[key] < 0
            for key in ('reserved_other_work_ns','reserved_other_work_bytes'))
            or profile.get('other_work_cost_status') not in ('BOUNDED','HELD_UNAVAILABLE')
            or not isinstance(profile.get('reservation_disposition'),str)
            or not profile['reservation_disposition'].strip()):
        raise ValueError('explicit reviewed other-work reservations required; unknown cost is unavailable')


def canonical_members(manifest):
    """Admit only the exact private inventory, feature bindings and selected scope."""
    if (manifest.get('schema') != 'slice50-paired-campaign/v1'
            or manifest.get('scope') not in ('primary','calibration')
            or manifest.get('plan') != plan() or manifest.get('features') != FEATURES):
        raise ValueError('exact canonical Slice50 paired inventory and features required')
    return (manifest['plan']['calibration'] if manifest['scope']=='calibration'
            else manifest['plan']['primary']+manifest['plan']['diagnostics'])


def worker_admission(manifest, cell, attempt):
    """Require the actual owning parent and its canonical registered live attempt."""
    from g_boundary import require_known_containment
    from m_guard import process_start
    require_known_containment(manifest['task'])
    value=json.loads((Path(manifest['allocation'])/'g-budget.json').read_text())
    owner=value.get('s50_guardian') or {}
    active=value.get('inflight') or {}
    if (owner.get('controller_pid') != os.getppid()
            or owner.get('controller_start') != process_start(os.getppid())
            or owner.get('root') != manifest['root']
            or active.get('cell') != cell['id'] or active.get('status') is not None
            or active.get('attempt') != str(Path(attempt).resolve())
            or active.get('freeze_uuid') != manifest['uuid'] or cell not in canonical_members(manifest)):
        raise RuntimeError('private pair worker lacks canonical active controller admission')
    return owner


def public_entry(operation, root, options=None):
    """Dispatch public pair operations through the existing original-ledger guardian."""
    from m_guard import guard
    if os.environ.get('AGENT_LONG') != '1':
        raise RuntimeError('paired campaign requires explicit AGENT_LONG=1')
    began=time.monotonic_ns()
    root=Path(root).resolve()
    selected=(json.loads(Path(options).read_text()) if options else
              json.loads((root/'manifest.json').read_text()))
    if operation == 'run' and selected.get('scope') == 'calibration':
        raise ValueError('calibration executes only through the explicit cumulative pair-calibrate command')
    profile=selected['profile']
    if not isinstance(profile,dict):
        profile=json.loads(Path(profile).read_text())
    approved(profile,selected['allocation'])
    calibration=(selected['calibration'] if operation=='calibration'
                 or operation=='report' and selected.get('scope')=='calibration' else None)
    if operation in ('freeze','calibration') and root.exists():
        raise ValueError('fresh pair root required; cumulative allowance never renews')
    if calibration:
        validate_limits(calibration)
        common=calibration['phase_ns']-calibration['payload_ns']
    else:
        common=(selected.get('primary_limits') or {}).get('common_ns', selected.get('common_limit_ns'))
        if type(common) is not int or common <= 20*10**9:
            raise ValueError('positive bounded common freeze/report/cleanup forecast required')
    command=[sys.executable,str(TOOLS/'g_runner.py'),'_pair_control','--operation',operation,
             '--root',str(root)]
    if options:
        command+=['--options',str(Path(options).resolve())]
    outcome=guard(command,allocation=selected['allocation'],profile=profile,root=root,
        task=os.environ['S30_TASK'],admission=True,phase='S50',calibration=calibration,
        common_limit_ns=common,started_ns=began,
        primary_limits=(selected.get('primary_limits') if not calibration else None))
    if (root / 'manifest.json').exists():
        from g_pair_runtime import retire_qualified
        retire_qualified(root, outcome)
    if operation != 'report':
        print(json.dumps(outcome))
    return outcome['exit']==0 and not outcome['cleanup_uncertain']


def freeze(options, output, *, scope):
    """Bind current public paired inputs/artifacts to exact private obligations."""
    from g_pair_freeze import freeze as freeze_pair
    return freeze_pair(options, output, scope=scope)


def native_result(step):
    """Read the existing native raw streams for exact paired call qualification."""
    from g_pair_report import native_result as load_native
    return load_native(step)


def payload(root, config, attempt, deadline_ns):
    """Run the admitted current paired native member through existing bounded drivers."""
    from g_pair_runtime import payload as run_payload
    return run_payload(root, config, attempt, deadline_ns)


def control_entry(operation, root, options, admission_fd):
    """Consume owning guardian admission and execute the explicit private operation."""
    from g_pair_runtime import control_entry as run_control
    return run_control(operation, root, options, admission_fd)


def required_prefix(manifest, root):
    """Select the next finite canonical pairs using actual qualified required-member credit."""
    from g_pair_report import published_report
    cells = manifest['plan']['primary'] + manifest['plan']['diagnostics']
    path = Path(root) / 'results/report.json'
    report = published_report(root) if path.exists() else {'primary': [], 'diagnostics': []}
    credited = {row['id'] for name in ('primary', 'diagnostics') for row in report[name] if row.get('credited')}
    start = next((index for index in range(0, len(cells), 2)
                  if not all(cell['id'] in credited for cell in cells[index:index+2])), len(cells))
    return cells[start:start+2*manifest['primary_limits']['pairs']]
