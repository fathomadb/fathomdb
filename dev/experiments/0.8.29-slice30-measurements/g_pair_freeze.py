"""Current private pair build/input custody; historical G subjects supply data only."""

from contextlib import closing
import json
import os
from pathlib import Path
import shutil
import sqlite3
import time
import uuid

from g_freeze import SOURCE, MODEL_FILES, PINS, compiler_inputs, git, identify
from g_span_coverage import stored_coverage


def package(store):
    """Identify a closed main/WAL package and its actual stored-span coverage."""
    store = Path(store).resolve(strict=True)
    files = {'': identify(store)}
    wal = store.with_name(store.name + '-wal')
    if wal.exists():
        files['-wal'] = identify(wal)
    return {'files': files, 'coverage': stored_coverage(store)}


def fixture(store, n, *, dense, markers=0):
    """Validate actual format/count/readiness without opening a provider or writing."""
    with closing(sqlite3.connect(Path(store).resolve().as_uri() + '?mode=ro', uri=True)) as db:
        def count(sql):
            return db.execute(sql).fetchone()[0]
        actual = {'format': count('PRAGMA user_version'),
            'natural_records': count("SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc' AND superseded_at IS NULL"),
            'marker_records': count("SELECT COUNT(*) FROM canonical_nodes WHERE kind='note' AND superseded_at IS NULL"),
            'vector_rows': count('SELECT COUNT(*) FROM _fathomdb_vector_rows'),
            'span_rows': count('SELECT COUNT(*) FROM _fathomdb_vector_spans')}
    if actual != {'format': 37, 'natural_records': n, 'marker_records': markers,
                   'vector_rows': n if dense else 0, 'span_rows': n if dense else 0}:
        raise ValueError('closed donor/template fixture differs from selected natural population')
    return actual


def donor(root, n):
    """Bind a qualified original seed's closed data without reusing native/timing proof."""
    root = Path(root).resolve()
    original = json.loads((root / 'manifest.json').read_text())
    if original.get('root') != str(root):
        raise ValueError('original G donor root differs')
    identifier = f'g1-n{n}'
    rows = {row['id']: row for row in json.loads((root / 'results/g-outcomes.json').read_text())}
    row = rows.get(identifier) or original.get('reused', {}).get(identifier) or {}
    if (row.get('status') not in ('complete', 'reused_complete') or row.get('exit') != 0
            or row.get('cleanup_uncertain') is not False or row.get('live_owned_pids') != []):
        raise ValueError('original G donor lacks qualified closed seed outcome')
    result = json.loads((Path(row['attempt']) / 'result.json').read_text())
    if result.get('complete') is not True or result.get('qualification_issues') != []:
        raise ValueError('original G donor seed qualification unavailable')
    seed = next(step for step in result['steps'] if step['family'] == 'G-seed')
    facts = seed['result']['after_reopen']
    if (facts.get('natural_records') != n or facts.get('vector_rows') != n
            or facts.get('span_rows') != n or facts.get('physical_vector_rows') != n
            or facts.get('pending_count') != 0 or facts.get('generation_failed') != 0
            or facts.get('format') != 37):
        raise ValueError('original G donor dense readiness unavailable')
    store = Path(seed.get('closed_store', str(Path(seed['output']) / 'store.sqlite')))
    value = package(store)
    value.update(fixture=fixture(store, n, dense=True),
                 origin={'root': str(root), 'attempt': row['attempt'], 'source_sha': original['source_sha']},
                 custody='data only; original native/timings do not qualify current paired work')
    return value


def require_forecasts(options, cells):
    """Refuse primary work until every current cell and common cost is observed/bounded."""
    forecasts = options.get('forecasts') or {}
    records = forecasts.get('cells') or {}
    if (forecasts.get('status') != 'CURRENT_PAIRED_BOUNDED' or set(records) != {cell['id'] for cell in cells}
            or type(forecasts.get('common_ns')) is not int or forecasts['common_ns'] <= 20 * 10**9
            or type(forecasts.get('common_bytes')) is not int or forecasts['common_bytes'] <= 0):
        raise ValueError('current paired forecasts unavailable for complete remaining table')
    for row in records.values():
        if (not row.get('observed_from') or any(type(row.get(key)) is not int or row[key] <= 0
                for key in ('whole_ns', 'new_retained_bytes'))):
            raise ValueError('current paired forecasts unavailable for complete remaining table')
    return forecasts


def freeze(options, output, *, scope):
    """Freeze exact current artifacts and private canonical obligations under the guardian."""
    from g_pair import FEATURES, approved, assemble_manifest, plan, validate_limits
    if scope not in ('primary', 'calibration'):
        raise ValueError('unknown private paired scope')
    inventory = plan()
    forecasts = require_forecasts(options, inventory['primary'] + inventory['diagnostics']) if scope == 'primary' else None
    build = json.loads(Path(options['build_receipt']).read_text())
    if build.get('runner_only') is not False:
        raise ValueError('source-only build cannot qualify public paired work')
    if (build.get('schema') != 'slice50-pair-build/v1' or build.get('compiler_inputs') != compiler_inputs()
            or build.get('lock') != identify(SOURCE / 'Cargo.lock') or git('status', '--porcelain')):
        raise ValueError('clean current private build/source/lock required')
    for name, features in FEATURES.items():
        record = build['binaries'][name]
        if record['features'] != features or any(record[key] != value for key, value in identify(record['path']).items()):
            raise ValueError('current private artifact/features differ')
    output = Path(output).resolve()
    allocation = Path(options['allocation']).resolve()
    if not output.is_relative_to(allocation) or (output / 'manifest.json').exists():
        raise ValueError('fresh private paired root inside original allocation required')
    profile = options['profile']
    profile = json.loads(Path(profile).read_text()) if not isinstance(profile, dict) else profile
    approved(profile, allocation)
    if scope == 'calibration':
        validate_limits(options['calibration'])
    inputs = {}
    for name, digest in PINS.items():
        record = identify(options[name])
        if record['sha256'] != digest:
            raise ValueError(f'registered {name} differs')
        inputs[name] = record
    for name, expected in (('corpus', 100000), ('queries', 500)):
        with Path(inputs[name]['path']).open() as stream:
            if sum(1 for _ in stream) != expected:
                raise ValueError('registered corpus/query counts differ')
    models = {}
    for name, (size, digest) in MODEL_FILES.items():
        record = identify(Path(options['model_cache']) / name)
        if (record['bytes'], record['sha256']) != (size, digest):
            raise ValueError('registered BGE cache differs')
        models[name] = record
    donors = {f'G{n}': donor(options['g_donors'][str(n)], n)
              for n in ([10000] if scope == 'calibration' else [10000, 100000])}
    from g_gpu import inspect_gpu
    gpu = inspect_gpu(options['gpu_uuid'])
    if gpu['applications'] or type(options['gpu_ordinal']) is not int or options['gpu_ordinal'] < 0:
        raise ValueError('reserved selected CUDA GPU required')
    gpu['ordinal'] = options['gpu_ordinal']
    output.mkdir(parents=True, exist_ok=True)
    owned_inputs = output / 'inputs'
    owned_inputs.mkdir()
    for name, record in inputs.items():
        destination = owned_inputs / name
        shutil.copy2(record['path'], destination)
        destination.chmod(0o444)
        inputs[name] = identify(destination)
    revision = json.loads(Path(__file__).with_name('g-cells.json').read_text())['model']['revision']
    cache = output / 'cache/huggingface'
    snapshot = cache / 'hub/models--BAAI--bge-small-en-v1.5/snapshots' / revision
    snapshot.mkdir(parents=True)
    for name, record in models.items():
        destination = snapshot / name
        shutil.copy2(record['path'], destination)
        destination.chmod(0o444)
        models[name] = identify(destination)
    manifest = assemble_manifest(dict(options, scope=scope, profile=profile), build, inputs, donors)
    manifest.update(uuid=str(uuid.uuid4()), root=str(output), allocation=str(allocation),
        source_sha=git('rev-parse', 'HEAD'), build_receipt=identify(options['build_receipt']),
        lock=identify(SOURCE / 'Cargo.lock'), model_files=models, hf_home=str(cache), gpu=gpu,
        task=str(Path(os.environ['S30_TASK']).resolve()),
        environment={key: os.environ.get(key) for key in ('CUDA_VISIBLE_DEVICES',)},
        calibration=options.get('calibration'), forecasts=forecasts, templates={})
    if scope == 'primary':
        from g_driver import run_program_step
        deadline = time.monotonic_ns() + forecasts['common_ns'] - 20 * 10**9
        for family, n in (('P1', 10000), ('P1', 100000), ('P2', 100000)):
            destination = output / 'templates' / f'{family}-{n}'
            config = dict(family='P-seed', seed_family=family, n=n, corpus=inputs['corpus']['path'])
            step = run_program_step([build['binaries']['ordinary']['path']], config, destination, deadline)
            if not step['complete'] or step['result'].get('runner_only'):
                raise RuntimeError('current provider-none template preparation failed')
            store = destination / 'store.sqlite'
            value = package(store)
            value.update(fixture=fixture(store, n, dense=False, markers=10 if family == 'P2' else 0), preparation=step)
            for record in value['files'].values():
                Path(record['path']).chmod(0o444)
            manifest['templates'][f'{family}-{n}'] = value
    path = output / 'manifest.json'
    path.write_text(json.dumps(manifest, indent=2) + '\n')
    path.chmod(0o444)
    return manifest


def validate(root):
    """Recheck exact canonical scope and current frozen artifact/data custody before dispatch."""
    from g_pair import approved, canonical_members
    root = Path(root).resolve()
    manifest = json.loads((root / 'manifest.json').read_text())
    canonical_members(manifest)
    approved(manifest['profile'], manifest['allocation'])
    if (manifest['root'] != str(root) or manifest['source_sha'] != git('rev-parse', 'HEAD')
            or manifest['compiler_inputs'] != compiler_inputs() or git('status', '--porcelain')
            or any(os.environ.get(key) != value for key, value in manifest['environment'].items())):
        raise ValueError('current clean paired source/environment differs')
    records = [*manifest['inputs'].values(), *manifest['model_files'].values(),
               manifest['build_receipt'], manifest['lock']]
    records += [{key: record[key] for key in ('path', 'bytes', 'sha256')}
                for record in manifest['build']['binaries'].values()]
    records += [record for subject in [*manifest['donors'].values(), *manifest['templates'].values()]
                for record in subject['files'].values()]
    if any(identify(record['path']) != record for record in records):
        raise ValueError('frozen current paired artifact/input/template bytes differ')
    if manifest['scope'] == 'primary':
        require_forecasts(manifest, canonical_members(manifest))
    return manifest
