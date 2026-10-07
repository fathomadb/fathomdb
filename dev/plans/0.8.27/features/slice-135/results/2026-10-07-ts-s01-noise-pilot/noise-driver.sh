#!/usr/bin/env bash
set -euo pipefail

cd /home/coreyt/projects/fathomdb-worktrees/release-0.8.27-slice-135
for rows in 32 256; do
  for index in 01 02 03 04 05; do
    if [[ $rows == 32 && $index == 01 ]]; then
      continue
    fi
    output="/tmp/slice135-ts-noise/${rows}-${index}-baseline"
    printf 'RUN %s\n' "$output"
    PYTHONDONTWRITEBYTECODE=1 .venv/bin/python scripts/slice135_ts_s01_block.py \
      --checkout /home/coreyt/projects/fathomdb-worktrees/release-0.8.26-slice-135-baseline \
      --source-sha f99e002f0d2e4002f3694c9f8d4986b56089edaa \
      --main-archive /tmp/slice135-ts-artifacts/baseline/packs/fathomdb-0.8.26.tgz \
      --platform-archive /tmp/slice135-ts-artifacts/baseline/packs/fathomdb-linux-x64-gnu-0.8.26.tgz \
      --main-sha256 90363762405041b11e6dd654da9cb6347695399eb37b81c5c09fde91296ac336 \
      --platform-sha256 b59b1862b11b8ed3edcdc2e5ee86f9db142268bb5e14571054f6245718fc28f6 \
      --install-root /tmp/slice135-ts-artifacts/baseline/consumer \
      --node /home/coreyt/.local/actions-runner/fathomdb-gpu/_work/_tool/node/25.9.0/x64/bin/node \
      --rows "$rows" --samples 100 --output-dir "$output"
    if [[ ! ( $rows == 256 && $index == 05 ) ]]; then
      sleep 20
    fi
  done
done
