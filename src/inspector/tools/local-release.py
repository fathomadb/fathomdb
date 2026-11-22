#!/usr/bin/env python3
"""Verify, seal, or safely promote an unsigned local Inspector candidate."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import tarfile
import tempfile

VERSION = '0.2.1'
NAME = f'fathomdb-inspector-{VERSION}-linux-x86_64'
ARCHIVE = NAME + '.tar.gz'
FILES = {'fathomdb-inspector', 'build-description.json', 'LICENSE', 'README.md'}
ENGINE = '72fe93634321616214352a7e07066ce0f93bcf4c'
ROOT = Path(__file__).resolve().parents[3]
SUPPORT = {'userland': 'Ubuntu 24.04 / glibc 2.39', 'architecture': 'Linux x86_64',
           'kernel_qualified': '7.0.0-43-generic', 'landlock_abi_minimum': 3,
           'chromium_qualified': '156.0.8078.4', 'distribution': 'local-only',
           'models': 'existing pinned cache only; not bundled or downloaded'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def git(*args):
    return subprocess.check_output(['git', '-C', str(ROOT), *args], text=True).strip()


def check_sums(directory, expected):
    lines = (directory / 'SHA256SUMS').read_text().splitlines()
    require(len(lines) == len(expected), 'checksum file count differs')
    found = set()
    for line in lines:
        match = re.fullmatch(r'([0-9a-f]{64})  ([A-Za-z0-9_.-]+)', line)
        require(match is not None, 'invalid checksum line')
        digest, name = match.groups()
        require(name in expected and name not in found, 'unexpected/duplicate checksum member')
        require(sha(directory / name) == digest, f'checksum mismatch: {name}')
        found.add(name)
    require(found == expected, 'missing checksum member')


def payload(candidate):
    require(candidate.is_dir() and not candidate.is_symlink(), 'candidate must be a real directory')
    bundle = candidate / NAME
    require(bundle.is_dir() and not bundle.is_symlink(), 'missing real bundle directory')
    require({p.name for p in bundle.iterdir()} == FILES | {'SHA256SUMS'}, 'unexpected bundle contents')
    for path in [*bundle.iterdir(), candidate / ARCHIVE, candidate / 'SHA256SUMS']:
        require(path.is_file() and not path.is_symlink(), f'non-regular member: {path.name}')
        mode = path.stat().st_mode & 0o7777
        require(mode == 0o755 if path.name == 'fathomdb-inspector' else mode & 0o7111 == 0,
                f'invalid member mode: {path.name}')
    check_sums(bundle, FILES)
    check_sums(candidate, {ARCHIVE})
    description = json.loads((bundle / 'build-description.json').read_text())
    for key, expected in {'version': VERSION, 'target': 'x86_64-unknown-linux-gnu',
                          'profile': 'release', 'compiled_schema': 37, 'protocol': 1,
                          'fixture_control': False, 'cached_default_cpu': True}.items():
        require(type(description.get(key)) is type(expected) and description[key] == expected,
                f'invalid production build: {key}')
    require(re.fullmatch('[0-9a-f]{64}', description.get('build', '')), 'invalid build identity')
    actual = json.loads(subprocess.check_output([str(bundle / 'fathomdb-inspector'), '--build-description'], env={}, text=True))
    require(actual == description, 'executable build description differs')
    with tarfile.open(candidate / ARCHIVE, 'r:gz') as archive:
        members = archive.getmembers()
        require(len(members) == len(FILES) + 2, 'unexpected archive member count')
        require({m.name.rstrip('/') for m in members} == {NAME} | {f'{NAME}/{f}' for f in FILES | {'SHA256SUMS'}},
                'unexpected archive members')
        for member in members:
            if member.name.rstrip('/') == NAME:
                require(member.isdir(), 'archive root must be a directory')
            else:
                require(member.isfile(), 'archive member must be a regular file')
                require(member.mode == (bundle / Path(member.name).name).stat().st_mode & 0o7777,
                        f'invalid archive member mode: {member.name}')
                stream = archive.extractfile(member)
                require(hashlib.file_digest(stream, 'sha256').hexdigest() == sha(bundle / Path(member.name).name),
                        'archive/extracted checksum differs')
    return description


def verify(candidate, source=False):
    require({p.name for p in candidate.iterdir()} == {NAME, ARCHIVE, 'SHA256SUMS', 'candidate.json'},
            'unexpected candidate contents')
    description = payload(candidate)
    metadata_path = candidate / 'candidate.json'
    require(metadata_path.is_file() and not metadata_path.is_symlink(), 'candidate metadata must be regular')
    metadata = json.loads(metadata_path.read_text())
    require(metadata.get('version') == VERSION and metadata.get('engine_commit') == ENGINE, 'wrong product/Engine identity')
    require(re.fullmatch('[0-9a-f]{40}', metadata.get('source_sha', '')), 'invalid source SHA')
    require(metadata.get('build') == description, 'candidate build metadata differs')
    require(metadata.get('archive_sha256') == sha(candidate / ARCHIVE), 'archive metadata checksum differs')
    require(metadata.get('executable_sha256') == sha(candidate / NAME / 'fathomdb-inspector'), 'executable metadata checksum differs')
    if source:
        require(not git('status', '--porcelain'), 'source checkout must be clean')
        base = metadata['source_sha']
        subprocess.run(['git', '-C', str(ROOT), 'merge-base', '--is-ancestor', base, 'HEAD'], check=True)
        changed = git('diff', '--name-only', base, 'HEAD').splitlines()
        require(all((p.startswith(('dev/inspector/', 'docs/inspector/')) and p.endswith('.md'))
                    or p in ('src/inspector/README.md', 'src/inspector/tools/README.md', 'mkdocs.yml') for p in changed),
                'candidate source differs beyond permitted documentation closure')
    return metadata


def seal(candidate):
    require(not (candidate / 'candidate.json').exists(), 'candidate is already sealed')
    require(not git('status', '--porcelain'), 'source checkout must be clean before sealing')
    description = payload(candidate)
    metadata = {'version': VERSION, 'source_sha': git('rev-parse', 'HEAD'), 'engine_commit': ENGINE,
                'build': description, 'archive_sha256': sha(candidate / ARCHIVE),
                'executable_sha256': sha(candidate / NAME / 'fathomdb-inspector'), 'support': SUPPORT}
    with (candidate / 'candidate.json').open('x') as stream:
        stream.write(json.dumps(metadata, indent=2, sort_keys=True) + '\n')
    return verify(candidate, source=True)


def promote(candidate, destination):
    verify(candidate)
    require(destination.is_absolute() and destination.parent.resolve(strict=True) == destination.parent,
            'destination requires an absolute canonical existing parent')
    require(not destination.exists() and not destination.is_symlink(), 'destination already exists; refusing overwrite')
    def terminate(_signum, _frame):
        raise SystemExit(143)

    previous_term = signal.signal(signal.SIGTERM, terminate)
    stage = None
    try:
        stage = Path(tempfile.mkdtemp(prefix='.inspector-stage-', dir=destination.parent))
        shutil.copytree(candidate, stage, dirs_exist_ok=True, symlinks=True)
        result = verify(stage)
        # Linux renameat2 makes the final no-overwrite condition atomic, including empty directories.
        libc = ctypes.CDLL(None, use_errno=True)
        rename = libc.renameat2
        rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
        rename.restype = ctypes.c_int
        if rename(-100, os.fsencode(stage), -100, os.fsencode(destination), 1) != 0:
            error = ctypes.get_errno()
            raise OSError(error, os.strerror(error), str(destination))
        return result
    finally:
        try:
            if stage is not None and stage.exists():
                shutil.rmtree(stage)
        finally:
            signal.signal(signal.SIGTERM, previous_term)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['verify', 'seal', 'promote'])
    parser.add_argument('candidate', type=Path)
    parser.add_argument('destination', type=Path, nargs='?')
    parser.add_argument('--source', action='store_true', help='require clean same source or docs-only descendant')
    args = parser.parse_args()
    require(args.candidate.is_absolute(), 'candidate must be absolute')
    if args.command == 'promote':
        require(args.destination is not None, 'promote requires NEW_DESTINATION')
        result = promote(args.candidate, args.destination)
    else:
        require(args.destination is None, 'unexpected destination')
        result = seal(args.candidate) if args.command == 'seal' else verify(args.candidate, args.source)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError, tarfile.TarError) as error:
        raise SystemExit(f'local-release: {error}') from error
