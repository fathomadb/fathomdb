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


def primary_limits(options):
    """Require explicit finite stop allowances, without inventing future cost estimates."""
    value = options.get('primary_limits') or {}
    keys = {'pairs', 'phase_ns', 'pair_ns', 'new_retained_bytes', 'common_ns', 'common_bytes'}
    if (set(value) != keys or any(type(value[key]) is not int or value[key] <= 0 for key in keys)
            or value['pairs'] > 80 or not 14*10**9 < value['pair_ns'] <= 1800*10**9
            or value['common_ns'] <= 20*10**9 or value['phase_ns'] <= value['common_ns'] + 14*10**9
            or value['common_bytes'] >= value['new_retained_bytes']):
        raise ValueError('positive finite primary pair/phase/growth/common stop allowances required')
    return dict(value)


def prepared_inputs(manifest):
    """Load the actual charged preparation shared by later required canonical pairs."""
    path = Path(manifest['root']) / 'results/prepared.json'
    if path.exists():
        manifest = dict(manifest, **json.loads(path.read_text()))
    return manifest


def prepare_inputs(manifest, cells, deadline_ns):
    """Prepare only selected P templates or G data/model custody, once and under charge."""
    from g_driver import run_program_step
    manifest = prepared_inputs(manifest)
    output = Path(manifest['root'])
    for family, n in dict.fromkeys((cell['family'], cell['n']) for cell in cells):
        if family.startswith('P'):
            key = f'{family}-{n}'
            if key in manifest['templates']:
                continue
            directory = output / 'templates' / key
            attempts = list(directory.glob('attempt-*')) if directory.exists() else []
            destination = directory / f'attempt-{len(attempts)+1:04}'
            config = dict(family='P-seed', seed_family=family, n=n, corpus=manifest['inputs']['corpus']['path'])
            step = run_program_step([manifest['build']['binaries']['ordinary']['path']], config, destination, deadline_ns)
            if not step['complete'] or step['result'].get('runner_only'):
                raise RuntimeError('current provider-none template preparation failed')
            store = destination / 'store.sqlite'
            value = package(store)
            value.update(fixture=fixture(store, n, dense=False, markers=10 if family == 'P2' else 0), preparation=step)
            for record in value['files'].values():
                Path(record['path']).chmod(0o444)
            manifest['templates'][key] = value
        else:
            key = f'G{n}'
            if key not in manifest['donors']:
                manifest['donors'][key] = donor(manifest['g_donors'][str(n)], n)
    if any(cell['family'].startswith('G') for cell in cells):
        from g_gpu import inspect_gpu
        gpu = inspect_gpu(manifest['requested_gpu']['uuid'])
        ordinal = manifest['requested_gpu']['ordinal']
        if gpu['applications'] or type(ordinal) is not int or ordinal < 0:
            raise ValueError('reserved selected CUDA GPU required')
        manifest['gpu'] = dict(gpu, ordinal=ordinal)
        if not manifest['model_files']:
            revision = json.loads(Path(__file__).with_name('g-cells.json').read_text())['model']['revision']
            snapshot = Path(manifest['hf_home']) / 'hub/models--BAAI--bge-small-en-v1.5/snapshots' / revision
            snapshot.mkdir(parents=True, exist_ok=True)
            for name, (size, digest) in MODEL_FILES.items():
                record = identify(Path(manifest['model_cache']) / name)
                if (record['bytes'], record['sha256']) != (size, digest):
                    raise ValueError('registered BGE cache differs')
                destination = snapshot / name
                shutil.copy2(record['path'], destination)
                destination.chmod(0o444)
                manifest['model_files'][name] = identify(destination)
    if time.monotonic_ns() >= deadline_ns:
        raise TimeoutError('selected required input preparation exhausted common allowance')
    path = output / 'results/prepared.json'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({key: manifest[key] for key in ('templates', 'donors', 'gpu', 'model_files')}, indent=2) + '\n')
    return manifest


def freeze(options, output, *, scope):
    """Freeze exact current artifacts and private canonical obligations under the guardian."""
    from g_pair import FEATURES, approved, assemble_manifest, plan, validate_limits
    if scope not in ('primary', 'calibration'):
        raise ValueError('unknown private paired scope')
    inventory = plan()
    limits = primary_limits(options) if scope == 'primary' else None
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
    output.mkdir(parents=True, exist_ok=True)
    owned_inputs = output / 'inputs'
    owned_inputs.mkdir()
    for name, record in inputs.items():
        destination = owned_inputs / name
        shutil.copy2(record['path'], destination)
        destination.chmod(0o444)
        inputs[name] = identify(destination)
    manifest = assemble_manifest(dict(options, scope=scope, profile=profile), build, inputs, {})
    manifest.update(uuid=str(uuid.uuid4()), root=str(output), allocation=str(allocation),
        source_sha=git('rev-parse', 'HEAD'), build_receipt=identify(options['build_receipt']),
        lock=identify(SOURCE / 'Cargo.lock'), model_files={}, hf_home=str(output / 'cache/huggingface'), gpu=None,
        task=str(Path(os.environ['S30_TASK']).resolve()),
        requested_gpu={'uuid': options.get('gpu_uuid'), 'ordinal': options.get('gpu_ordinal')},
        model_cache=options.get('model_cache'), g_donors=options.get('g_donors', {}),
        ordinary_calibration_from=options.get('ordinary_calibration_from'),
        environment={key: os.environ.get(key) for key in ('CUDA_VISIBLE_DEVICES',)},
        calibration=options.get('calibration'), primary_limits=limits, templates={},
        common_limit_ns=(limits['common_ns'] if limits else
                         options['calibration']['phase_ns'] - options['calibration']['payload_ns']))
    selected = inventory['calibration'] if scope == 'calibration' else inventory['primary'][:2*limits['pairs']]
    manifest = prepare_inputs(manifest, selected, time.monotonic_ns() + manifest['common_limit_ns'] - 20*10**9)
    if scope == 'primary' and manifest.get('ordinary_calibration_from'):
        rows = reference_rows(manifest)
        (output / 'results/campaign-state.json').write_text(json.dumps(rows, indent=2) + '\n')
    path = output / 'manifest.json'
    path.write_text(json.dumps(manifest, indent=2) + '\n')
    path.chmod(0o444)
    return manifest


def validate(root):
    """Recheck exact canonical scope and current frozen artifact/data custody before dispatch."""
    from g_pair import approved, canonical_members
    root = Path(root).resolve()
    manifest = prepared_inputs(json.loads((root / 'manifest.json').read_text()))
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
        primary_limits(manifest)
    return manifest


def matching_reference(original, required):
    """Match the identical ordinary workload while preserving its distinct origin labels."""
    return ({key: value for key, value in original.items() if key not in ('id', 'purpose')}
            == {key: value for key, value in required.items() if key not in ('id', 'purpose')})


def reference_rows(manifest):
    """Reference eligible existing ordinary G10k events without executing or charging again."""
    from g_pair import compare_results, native_result, plan, qualify_step
    from g_pair_report import clean, published_report
    root = Path(manifest['ordinary_calibration_from']).resolve()
    original = json.loads((root / 'manifest.json').read_text())
    if Path(original['allocation']).resolve() != Path(manifest['allocation']).resolve():
        raise ValueError('ordinary references require the same allocation and existing shared charges')
    original = validate(root)
    if (original['scope'] != 'calibration' or original['source_sha'] != manifest['source_sha']
            or original['compiler_inputs'] != manifest['compiler_inputs'] or original['features'] != manifest['features']
            or any(original['inputs'][name][key] != manifest['inputs'][name][key]
                   for name in PINS for key in ('sha256', 'bytes'))
            or any(original['build']['binaries']['ordinary'][key] != manifest['build']['binaries']['ordinary'][key]
                   for key in ('sha256', 'bytes', 'features'))
            or any(original['gpu'][key] != manifest['requested_gpu'][key] for key in ('uuid', 'ordinal'))):
        raise ValueError('strict current ordinary reference source/build/input/device mismatch')
    report = published_report(root)
    rows, values = [], []
    for source_cell in plan()['calibration'][:2]:
        target = next(cell for cell in plan()['primary'] if matching_reference(source_cell, cell))
        row = next(row for row in report['calibration'] if row['id'] == source_cell['id'])
        actual = json.loads((Path(row['attempt']) / 'result.json').read_text())
        if (not clean(row) or not row.get('credited') or row.get('cell') != source_cell
                or actual.get('cell') != source_cell or actual.get('freeze_uuid') != original['uuid']
                or actual.get('complete') is not True or actual.get('qualification_issues') != []
                or len(actual.get('steps', [])) != 1
                or qualify_step(actual['steps'][0], source_cell, original)):
            raise ValueError('existing ordinary reference lacks full qualified original calls/cleanup')
        values.append(native_result(actual['steps'][0]))
        rows.append(dict(row, id=target['id'], cell=target, qualifies_primary=True,
            guardian_root=str(root),
            reference={'root': str(root), 'member': source_cell['id'], 'purpose': source_cell['purpose'],
                       'freeze_uuid': original['uuid'], 'required_member': target['id']},
            charge_disposition='already charged on original shared ledger; reference only, no new execution or charge'))
    if not compare_results(*values):
        raise ValueError('existing ordinary reference pair differs in actual record results')
    return rows
