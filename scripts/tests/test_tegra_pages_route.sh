#!/usr/bin/env bash
# Offline check that Tegra Pages dispatch requires the exact release branch.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
guard="$repo_root/scripts/release/check-tegra-pages-route.sh"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

cat >"$fixture/gh" <<'SHIM'
#!/usr/bin/env bash
case "$2" in
  repos/fathomadb/fathomdb/environments/github-pages/deployment-branch-policies)
    cat "$ROUTE_ENV_JSON" ;;
  *) exit 2 ;;
esac
SHIM
chmod +x "$fixture/gh"

cat >"$fixture/allowed.json" <<'JSON'
{"branch_policies":[{"name":"main","type":"branch"},{"name":"release/0.8.27","type":"branch"}]}
JSON
cat >"$fixture/denied.json" <<'JSON'
{"branch_policies":[{"name":"main","type":"branch"},{"name":"release/0.8.26","type":"branch"}]}
JSON

if PATH="$fixture:$PATH" ROUTE_ENV_JSON="$fixture/allowed.json" bash "$guard" 0.8.27 >/dev/null; then
  printf 'PASS exact release branch allowed by Pages environment\n'
else
  printf 'FAIL exact release branch rejected\n' >&2
  exit 1
fi
if PATH="$fixture:$PATH" ROUTE_ENV_JSON="$fixture/denied.json" bash "$guard" 0.8.27 >/dev/null 2>&1; then
  printf 'FAIL missing Pages branch policy accepted\n' >&2
  exit 1
else
  printf 'PASS missing Pages branch policy rejected\n'
fi
