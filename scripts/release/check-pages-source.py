#!/usr/bin/env python3
"""Reject a whole-site Pages candidate that lacks freshly fetched current main."""
import argparse
from pathlib import Path
import re
import subprocess
import sys


def git(repo: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(['git', '-C', str(repo), *args], text=True, capture_output=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path.cwd())
    parser.add_argument('--candidate', required=True)
    args = parser.parse_args()
    candidate = args.candidate
    if not re.fullmatch(r'[0-9a-f]{40}', candidate):
        print('Pages source currency: candidate must be a full lowercase commit SHA', file=sys.stderr)
        return 1
    shallow = git(args.repo, 'rev-parse', '--is-shallow-repository')
    if shallow.returncode or shallow.stdout.strip() != 'false':
        print('Pages source currency: full Git history required; use fetch-depth: 0', file=sys.stderr)
        return 1
    if git(args.repo, 'cat-file', '-e', candidate + '^{commit}').returncode:
        print(f'Pages source currency: candidate commit is absent: {candidate}', file=sys.stderr)
        return 1
    fetched = git(args.repo, '-c', 'credential.helper=', 'fetch', '--no-tags', 'origin',
                  '+refs/heads/main:refs/remotes/origin/main')
    if fetched.returncode:
        print('Pages source currency: cannot fetch current origin/main; refusing publication', file=sys.stderr)
        return 1
    current = git(args.repo, 'rev-parse', '--verify', 'refs/remotes/origin/main^{commit}')
    if current.returncode:
        print('Pages source currency: current origin/main commit is absent; refusing publication', file=sys.stderr)
        return 1
    ancestor = git(args.repo, 'merge-base', '--is-ancestor', 'origin/main', candidate)
    if ancestor.returncode:
        print(f'Pages source currency: {candidate} does not contain current origin/main '
              f'{current.stdout.strip()}; incorporate main and re-dispatch', file=sys.stderr)
        return 1
    print(f'Pages source currency: candidate {candidate} contains current main {current.stdout.strip()}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
