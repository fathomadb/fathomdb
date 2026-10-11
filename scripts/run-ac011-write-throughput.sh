#!/usr/bin/env bash
# Run the binding AC-011a/b write-throughput gates on the named tier-1 host.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

runner_os="$(uname -s)"
runner_arch="$(uname -m)"
if [ "$runner_os" != "Linux" ] || [ "$runner_arch" != "x86_64" ]; then
  echo "FAIL AC-011 runner: requires Linux x86_64" >&2
  exit 2
fi

runner_host="$(hostname -s)"
if [ "$runner_host" != "windchill3" ]; then
  echo "FAIL AC-011 runner: canonical host is windchill3; found $runner_host" >&2
  exit 2
fi

cpu_count="$(getconf _NPROCESSORS_ONLN)"
if [ "$cpu_count" -lt 8 ]; then
  echo "FAIL AC-011 runner: requires at least 8 online CPUs; found $cpu_count" >&2
  exit 2
fi

memory_kib="$(awk '/^MemTotal:/ { print $2 }' /proc/meminfo)"
if [ "$memory_kib" -lt 15728640 ]; then
  echo "FAIL AC-011 runner: requires at least 15 GiB visible RAM; found ${memory_kib} KiB" >&2
  exit 2
fi

perf_tmp="${TMPDIR:-/tmp}"
storage_source="$(findmnt -T "$perf_tmp" -no SOURCE)"
case "$storage_source" in
  /dev/nvme*) ;;
  *)
    echo "FAIL AC-011 runner: $perf_tmp must resolve to local NVMe; found $storage_source" >&2
    exit 2
    ;;
esac

runner_kernel="$(uname -r)"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
echo "AC011_RUNNER host=$runner_host cpu_count=$cpu_count memory_kib=$memory_kib storage=$storage_source arch=$runner_arch kernel=$runner_kernel"
echo "AC011_RUNNER rustc=$rustc_version cargo=$cargo_version"

export AGENT_LONG=1
export FATHOMDB_TIER1_WRITE_PERF=1

cargo test --release -p fathomdb-engine --test perf_gates -- \
  --test-threads=1 --nocapture ac_011a_write_throughput_1kb
cargo test --release -p fathomdb-engine --test perf_gates -- \
  --test-threads=1 --nocapture ac_011b_write_throughput_100kb
