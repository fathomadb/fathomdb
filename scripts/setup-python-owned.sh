#!/usr/bin/env bash
# Build and install a source-bound noneditable wheel in this checkout's venv.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
mode="${1:---build}"
case "$mode" in --provision|--build) ;; *) echo 'setup-python-owned: use --provision or --build' >&2; exit 2 ;; esac
if [ -L .venv ]; then echo 'setup-python-owned: .venv must be owned by this checkout, not a symlink' >&2; exit 1; fi
if [ -n "${VIRTUAL_ENV:-}" ] && [ "$VIRTUAL_ENV" != "$repo_root/.venv" ]; then
  echo 'setup-python-owned: foreign VIRTUAL_ENV; use this checkout environment' >&2; exit 1
fi
if [ "$mode" = --provision ] && [ ! -x .venv/bin/python ]; then
  # shellcheck source=lib/agent-python-env.sh
  . "$SCRIPT_DIR/lib/agent-python-env.sh"
  create_venv_with_selected_python .venv
fi
if [ ! -x .venv/bin/python ]; then
  echo 'setup-python-owned: checkout-owned .venv/bin/python missing; provision this checkout first' >&2; exit 1
fi
.venv/bin/python - "$mode" "$repo_root" <<'PY'
import hashlib,json,os,pathlib,subprocess,sys,tempfile,tomllib
root=pathlib.Path(sys.argv[2]).resolve(); venv=root/'.venv'; python=venv/'bin/python'
def run(args,**kwargs):
    return subprocess.run([str(x) for x in args],check=True,**kwargs)
def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
if sys.version_info < (3,11):
    raise SystemExit('setup-python-owned: developer Python >=3.11 required')
for path in (venv,root/'node_modules',root/'src/ts/node_modules'):
    if path.exists() and not path.resolve().is_relative_to(root):
        raise SystemExit(f'setup-python-owned: foreign owned path {path}')
config=tomllib.loads((root/'src/python/pyproject.toml').read_text())
run([python,'-m','pip','--version'])
if sys.argv[1]=='--provision':
    run([python,'-m','pip','install',*config['project']['optional-dependencies']['dev']])
for name in ('ruff','pyright'):
    pin=next(x.split('==',1)[1] for x in config['project']['optional-dependencies']['dev'] if x.startswith(name+'=='))
    found=run([venv/'bin'/name,'--version'],capture_output=True,text=True).stdout.strip()
    if found!=f'{name} {pin}':
        raise SystemExit(f'setup-python-owned: {name} pin mismatch: {found!r}, want {pin}')
features=','.join(config['tool']['maturin']['features'])
output=pathlib.Path(tempfile.mkdtemp(prefix='fathomdb-owned-python-'))
run([python,'-m','maturin','build','--locked','--release','--interpreter',python,'--features',features,'--out',output],cwd=root/'src/python')
wheels=list(output.glob('*.whl'))
if len(wheels)!=1:
    raise SystemExit(f'setup-python-owned: expected one wheel, got {wheels}')
wheel=wheels[0]
run([python,'-m','pip','install','--force-reinstall','--no-deps',wheel])
probe="""# S10_NATIVE_PROBE
import hashlib,json,pathlib,sys,sysconfig
import fathomdb._fathomdb as native
p=pathlib.Path(native.__file__).resolve()
print(json.dumps({'prefix':sys.prefix,'module':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'python':sys.version,'abi':sysconfig.get_config_var('SOABI')}))
"""
env=dict(os.environ);env.pop('PYTHONPATH',None)
native=json.loads(run([python,'-c',probe],cwd=output,env=env,capture_output=True,text=True).stdout)
module=pathlib.Path(native['module']).resolve()
if pathlib.Path(native['prefix']).resolve()!=venv or not module.is_relative_to(venv) or digest(module)!=native['sha256']:
    raise SystemExit('setup-python-owned: installed native identity is foreign or changed')
sha=run(['git','rev-parse','HEAD'],cwd=root,capture_output=True,text=True).stdout.strip()
delta=run(['git','diff','HEAD','--binary'],cwd=root,capture_output=True).stdout
receipt={'schema':'fathomdb.owned-python/v1','source_sha':sha,'source_delta_sha256':hashlib.sha256(delta).hexdigest(),'lock_sha256':digest(root/'Cargo.lock') if (root/'Cargo.lock').exists() else None,'features':features,'wheel':str(wheel),'wheel_sha256':digest(wheel),'interpreter':str(python),'native':native}
destination=root/'.cache/agent/python-native.json';destination.parent.mkdir(parents=True,exist_ok=True)
with tempfile.NamedTemporaryFile(mode='w',dir=destination.parent,delete=False) as temporary:
    json.dump(receipt,temporary,indent=2);temporary.write('\n');temporary.flush();os.fsync(temporary.fileno());name=temporary.name
os.replace(name,destination)
print(f'setup-python-owned: installed {wheel.name}; receipt {destination}')
PY
