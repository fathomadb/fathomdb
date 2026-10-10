#!/usr/bin/env python3
"""Generate source-derived SDK operation and contributor navigation indexes."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
try:
    import tomllib
except ModuleNotFoundError:
    print("generated documentation: Python 3.11 or later is required; "
          "use the configured .venv/bin/python interpreter", file=sys.stderr)
    raise SystemExit(1)

VERBS = ('build', 'lint', 'typecheck', 'test', 'verify')
LOCATORS = {'engine_static', 'engine_instance', 'read', 'graph', 'admin', 'package'}
SOURCE_URL = 'https://github.com/fathomadb/fathomdb/blob/main/'


def source(root: Path, relative: str) -> Path:
    path = (root / relative).resolve()
    if not path.is_relative_to(root):
        raise ValueError(f'source escapes repository: {relative}')
    if not path.is_file():
        raise ValueError(f'missing source input: {relative}')
    return path


def identifier(value: object, label: str) -> str:
    if not isinstance(value, str) or not re.fullmatch(r'[a-zA-Z_][a-zA-Z0-9_.-]*', value):
        raise ValueError(f'invalid {label}: {value!r}')
    return value


def link(relative: str) -> str:
    return f'[{relative}]({SOURCE_URL}{relative})'


def preamble(title: str, sources: list[str]) -> list[str]:
    return [f'# {title}', '',
            'Generated from tracked source; do not edit this page by hand.',
            'Regenerate with `python3 scripts/generate-public-docs.py --write`.', '',
            'Sources: ' + ', '.join(link(path) for path in sources) + '.', '']


def operation_index(root: Path) -> str:
    manifest = 'src/conformance/governed-operation-parity.json'
    data = json.loads(source(root, manifest).read_text())
    if data.get('schema_version') != 'fathomdb.governed-operation-parity/v1':
        raise ValueError('unsupported governed operation parity schema')
    operations = data.get('operations')
    if not isinstance(operations, list) or not operations:
        raise ValueError('operation parity input must contain operations')
    lines = preamble('SDK operation map', [manifest])
    lines += ['This operation index lists governed names and owners. It does not specify',
              'signatures, options, return shapes, or error behavior. Use the maintained',
              '[Python API](../reference/python-api.md),',
              '[TypeScript API](../reference/typescript-api.md), and',
              '[Rust SDK reference](../reference/rust-sdk.md) for those contracts.', '',
              f'Governed operations: **{len(operations)}**. Each language cell is',
              '`locator` / `spelling`; `engine_static` is on `Engine`,',
              '`engine_instance` is on an open handle, `package` is standalone,',
              'and `read`, `graph`, and `admin` name their respective surfaces.', '',
              '| Operation | State | Python | TypeScript | Rust |',
              '| --- | --- | --- | --- | --- |']
    seen = set()
    for operation in operations:
        op_id = identifier(operation.get('id'), 'operation id')
        if op_id in seen:
            raise ValueError(f'duplicate governed operation: {op_id}')
        seen.add(op_id)
        state = identifier(operation.get('state'), f'{op_id} state')
        cells = [f'`{op_id}`', state]
        for language in ('python', 'typescript', 'rust'):
            surface = operation.get(language)
            if not isinstance(surface, dict) or surface.get('locator') not in LOCATORS:
                raise ValueError(f'invalid {op_id} {language} locator')
            spelling = identifier(surface.get('spelling'), f'{op_id} {language} spelling')
            cells.append(f"`{surface['locator']}` / `{spelling}`")
        lines.append('| ' + ' | '.join(cells) + ' |')
    return '\n'.join(lines) + '\n'


def contributor_index(root: Path) -> str:
    workspace = tomllib.loads(source(root, 'Cargo.toml').read_text())
    members = workspace.get('workspace', {}).get('members')
    if not isinstance(members, list) or not members:
        raise ValueError('workspace must contain member manifest paths')
    lines = preamble('Contributor source map', ['Cargo.toml'])
    lines += ['This index lists workspace owners and typed developer entry points.',
              'For TDD, worktree isolation, verification, and release-state guidance,',
              'see [Contributing](../contributing/index.md). Package versions are omitted.', '',
              '## Rust workspace', '', '| Package | Manifest |', '| --- | --- |']
    seen = set()
    for member in sorted(members):
        if not isinstance(member, str) or any(c in member for c in '*?['):
            raise ValueError(f'workspace member needs an explicit manifest path: {member!r}')
        relative = f'{member}/Cargo.toml'
        manifest = tomllib.loads(source(root, relative).read_text())
        name = identifier(manifest.get('package', {}).get('name'), 'workspace package name')
        if name in seen:
            raise ValueError(f'duplicate workspace package: {name}')
        seen.add(name)
        lines.append(f'| `{name}` | {link(relative)} |')
    lines += ['', '## Typed developer entry points', '',
              '| Verb | Script |', '| --- | --- |']
    for verb in VERBS:
        relative = f'scripts/agent-{verb}.sh'
        content = source(root, relative).read_bytes()
        if not content.startswith(b'#!/usr/bin/env bash\n'):
            raise ValueError(f'typed entry point is not a Bash script: {relative}')
        lines.append(f'| `{verb}` | {link(relative)} |')
    lines.append('')
    for verb in VERBS:
        relative = f'scripts/agent-{verb}.sh'
        digest = hashlib.sha256(source(root, relative).read_bytes()).hexdigest()
        lines.append(f'<!-- source-sha256: {relative} {digest} -->')
    return '\n'.join(lines) + '\n'


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--write', action='store_true')
    mode.add_argument('--check', action='store_true')
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        # Validate every input before writing either page.
        outputs = {'sdk-operation-map.md': operation_index(root),
                   'contributor-map.md': contributor_index(root)}
        stale = []
        for name, text in outputs.items():
            output = root / 'docs/generated' / name
            if args.write:
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_text(text)
            elif not output.is_file() or output.read_text() != text:
                stale.append(output.relative_to(root).as_posix())
        if stale:
            raise ValueError('stale generated documentation: ' + ', '.join(stale) +
                             '; run python3 scripts/generate-public-docs.py --write')
        print('generated documentation: ' + ('written' if args.write else 'current'))
        return 0
    except (OSError, ValueError, TypeError, AttributeError, KeyError) as exc:
        print(f'generated documentation: {exc}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
