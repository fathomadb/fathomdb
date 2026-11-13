"""Serial sensitivity controls on one parent-owned checkout; never product RED."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

CONTROL = Path(os.environ['S20_CONTROL_SOURCE']).resolve()
TASK = Path(os.environ['S20_TASK']).resolve()
EXPECTED = 'ef1f8d137ef647f172e81f400ad3e5caddef0192'
ADMISSION = 'src/rust/crates/fathomdb-mcp/tests/admission.rs'
REGISTRY = 'src/rust/crates/fathomdb-mcp/src/registry.rs'
FUSION = 'src/rust/crates/fathomdb-engine/src/fusion.rs'
EMBEDDER = 'src/rust/crates/fathomdb-embedder/src/candle_bge.rs'
PRIMARY = 'a_drain_before_enqueue_returns_the_call_to_open_wait'


def digest(value):
    return hashlib.sha256(value).hexdigest()


def git(*arguments):
    return subprocess.check_output(['git', *arguments], cwd=CONTROL, text=True).strip()


def replacement(original, before, after):
    if original.count(before) != 1:
        raise RuntimeError('mutation is not exact/unique')
    return original.replace(before, after, 1)


def main():
    if git('rev-parse', 'HEAD') != EXPECTED or git('status', '--porcelain'):
        raise RuntimeError('wrong/dirty control checkout')
    admission_hash = digest((CONTROL/ADMISSION).read_bytes())
    registry = (CONTROL/REGISTRY).read_text()
    start = registry.index('        if slot.phase != Phase::Ready || slot.epoch != epoch {')
    stop = registry.index('        slot.last_used.touch();', start)
    block = registry[start:stop]
    typed = replacement(block, 'return Ok(None);', 'return Err(ToolError::not_ready(name, "draining"));')
    bypass = replacement(block, 'if slot.phase != Phase::Ready || slot.epoch != epoch {', 'if false {')
    cases = [
        ('stale-typed-error', REGISTRY, block, typed,
         ['-p', 'fathomdb-mcp', '--features', 'test-seams', '--test', 'admission'], PRIMARY, 'assertion failed:'),
        ('stale-bypass', REGISTRY, block, bypass,
         ['-p', 'fathomdb-mcp', '--features', 'test-seams', '--test', 'admission'], PRIMARY,
         'timed out waiting for the stale phase/epoch rejection'),
    ]
    for name, before, after in [
        ('RRF_K', 'pub const RRF_K: f64 = 30.0;', 'pub const RRF_K: f64 = 31.0;'),
        ('RRF_WEIGHT_TEXT', 'pub const RRF_WEIGHT_TEXT: f64 = 3.0;', 'pub const RRF_WEIGHT_TEXT: f64 = 4.0;'),
        ('RRF_WEIGHT_VECTOR', 'pub const RRF_WEIGHT_VECTOR: f64 = 1.0;', 'pub const RRF_WEIGHT_VECTOR: f64 = 2.0;'),
    ]:
        cases.append((name, FUSION, before, after, ['-p', 'fathomdb-engine', '--lib'],
                      'fusion::tests::overflow_chunking_fusion_tripwire', name))
    for name, before, after in [
        ('DEFAULT_EMBEDDER_NAME', 'pub const DEFAULT_EMBEDDER_NAME: &str = "fathomdb-bge-small-en-v1.5";',
         'pub const DEFAULT_EMBEDDER_NAME: &str = "sensitivity-control-different-name";'),
        ('MAX_SEQUENCE_TOKENS', 'const MAX_SEQUENCE_TOKENS: usize = 512;', 'const MAX_SEQUENCE_TOKENS: usize = 513;'),
    ]:
        cases.append((name, EMBEDDER, before, after, ['-p', 'fathomdb-embedder', '--features', 'default-embedder', '--lib'],
                      'candle_bge::tests::overflow_chunking_embedder_tripwire', name))
    records = []
    for name, relative, before, after, selection, selector, failure in cases:
        path = CONTROL/relative
        original = path.read_bytes()
        changed = replacement(original.decode(), before, after).encode()
        if relative in [FUSION, EMBEDDER]:
            marker = '#[cfg(test)]\nmod tests {'
            assert original[original.rfind(marker.encode()):] == changed[changed.rfind(marker.encode()):]
        assert digest((CONTROL/ADMISSION).read_bytes()) == admission_hash
        argv = ['cargo', 'test', '--locked', *selection, '--message-format=json', selector, '--', '--exact', '--nocapture']
        environment = dict(os.environ)
        environment['CARGO_TARGET_DIR'] = str(TASK/'approved-runtime/control-target')
        environment['TOKIO_WORKER_THREADS'] = '2'
        raw = TASK/'logs'/f'approved-negative-{name}.log'
        try:
            path.write_bytes(changed)
            with raw.open('wb') as output:
                result = subprocess.run(argv, cwd=CONTROL, env=environment, stdout=output,
                                        stderr=subprocess.STDOUT, timeout=600)
            output = raw.read_text(errors='replace')
            assert result.returncode == 101, (name, result.returncode, str(raw))
            assert 'test result: FAILED. 0 passed; 1 failed' in output, (name, str(raw))
            assert failure in output, (name, failure, str(raw))
            if relative in [FUSION, EMBEDDER]:
                assert 'dev/design/overflow-chunking-reevaluation.md' in output
            executables = []
            for line in output.splitlines():
                if not line.startswith('{'):
                    continue
                try:
                    item = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if item.get('reason') == 'compiler-artifact' and item.get('executable'):
                    executable = Path(item['executable'])
                    executables.append({'path': str(executable), 'sha256': digest(executable.read_bytes())})
            assert executables, (name, 'no actual executable identity')
            record = {'case': name, 'source_sha': EXPECTED, 'path': relative,
                      'original_sha256': digest(original), 'mutation_sha256': digest(changed),
                      'argv': argv, 'exit': result.returncode, 'log': str(raw),
                      'executables': executables, 'classification': 'EXPECTED_SENSITIVITY_FAILURE'}
            records.append(record)
            print(json.dumps(record), flush=True)
        finally:
            path.write_bytes(original)
            assert path.read_bytes() == original
        if git('status', '--porcelain'):
            raise RuntimeError('control source was not exactly restored')
    destination = TASK/'receipts/approved-sensitivity.json'
    destination.write_text(json.dumps({'cases': records, 'control_sha': EXPECTED,
                                      'clean_after': not git('status', '--porcelain'),
                                      'product_red_claim': False}, indent=2)+'\n')
    print(f'PASS {len(records)} serialized sensitivity controls; exact source restored', flush=True)


if __name__ == '__main__':
    main()
