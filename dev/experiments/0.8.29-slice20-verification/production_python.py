"""Owned installed production-wheel identity and real provider-free lifecycle."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

import fathomdb
import fathomdb._fathomdb as native

root = Path(os.environ['S20_SOURCE']).resolve()
module = Path(native.__file__).resolve()
assert Path(sys.prefix).resolve() == root/'.venv'
assert module.is_relative_to(root/'.venv')
receipt = json.loads((root/'.cache/agent/python-native.json').read_text())
native_hash = hashlib.sha256(module.read_bytes()).hexdigest()
assert native_hash == receipt['native']['sha256']
subprocess.run(['git', 'diff', '--exit-code', receipt['source_sha'], 'HEAD', '--',
                'Cargo.toml', 'Cargo.lock', 'build.rs', '.cargo', 'src',
                'scripts/setup-python-owned.sh'], cwd=root, check=True)
with tempfile.TemporaryDirectory(prefix='s20-production-', dir=os.environ['TMPDIR']) as directory:
    path = str(Path(directory)/'store.sqlite')
    engine = fathomdb.Engine.open(path, use_default_embedder=False)
    try:
        engine.write([{'kind': 'doc', 'body': 's20harborcanary local proof',
                       'logical_id': 's20-proof', 'source_id': 's20-proof-source'}])
        result = engine.search_text_only('s20harborcanary')
        assert [(hit.id.space, hit.id.value) for hit in result.results] == [('logical', 's20-proof')]
    finally:
        engine.close()
    engine = fathomdb.Engine.open(path, use_default_embedder=False)
    try:
        result = engine.search_text_only('s20harborcanary')
        assert [(hit.id.space, hit.id.value) for hit in result.results] == [('logical', 's20-proof')]
    finally:
        engine.close()
print(json.dumps({'module': str(module), 'sha256': native_hash,
                  'built_source_sha': receipt['source_sha'],
                  'consumer_source_sha': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
                  'compiled_input_equivalence': True, 'python': sys.version,
                  'lifecycle': 'write/text-search/close/reopen/text-search/close/normal-exit'}))
