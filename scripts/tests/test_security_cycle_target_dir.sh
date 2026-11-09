#!/usr/bin/env bash
# CLI selection only: fake tracers/namespace tools do not qualify security.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/bin" "$WORK/caller"
cat >"$WORK/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[ "$PWD" = "$EXPECTED_CARGO_ROOT" ] || exit 91
printf 'build\n' >>"$BUILD_LOG"
mkdir -p "$(dirname "$EXPECTED_BINARY")"
cat >"$EXPECTED_BINARY" <<'BIN'
#!/usr/bin/env bash
printf '%s\n' "$0" >>"$SELECTED_LOG"
BIN
chmod +x "$EXPECTED_BINARY"
SH
cat >"$WORK/bin/strace" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
while [ "$#" -gt 0 ]; do
    case "$1" in
        -f) shift ;;
        -e) shift 2 ;;
        -o) : >"$2"; shift 2 ;;
        *) break ;;
    esac
done
exec "$@"
SH
cat >"$WORK/bin/unshare" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
shift
if [ "${1:-}" = -- ]; then shift; fi
exec "$@"
SH
printf '#!/usr/bin/env bash\nexit 0\n' >"$WORK/bin/ip"
chmod +x "$WORK/bin/"*
failures=0
for gate in check-no-listen check-netns-deny-egress; do
    for mode in existing build; do
        for location in default relative absolute relative-space absolute-space; do
            fixture="$WORK/$gate-$mode-$location"
            mkdir -p "$fixture/scripts/security"
            cp "$REPO_ROOT/scripts/security/$gate.sh" "$fixture/scripts/security/"
            cp "$REPO_ROOT/scripts/security/lib-egress-allowlist.sh" "$fixture/scripts/security/"
            case "$location" in
                default) target="$fixture/target"; configured='' ;;
                relative) target="$fixture/artifacts"; configured='artifacts' ;;
                absolute) target="$WORK/absolute-$gate-$mode"; configured="$target" ;;
                relative-space) target="$fixture/artifacts with spaces"; configured='artifacts with spaces' ;;
                absolute-space) target="$WORK/absolute with spaces-$gate-$mode"; configured="$target" ;;
            esac
            # An explicit target must leave an unrelated unavailable cache alone.
            if [ "$location" != default ]; then
                ln -s "$fixture/unavailable-cold-cache" "$fixture/target"
            fi
            export EXPECTED_BINARY="$target/debug/examples/security_cycle"
            export EXPECTED_CARGO_ROOT="$fixture" BUILD_LOG="$fixture/build.log" SELECTED_LOG="$fixture/selected.log"
            : >"$BUILD_LOG"
            if [ "$mode" = existing ]; then
                (cd "$fixture" && PATH="$WORK/bin:$PATH" cargo)
                : >"$BUILD_LOG"
            fi
            set +e
            output="$(cd "$WORK/caller" && {
                if [ "$location" = default ]; then
                    env -u CARGO_TARGET_DIR PATH="$WORK/bin:$PATH" bash "$fixture/scripts/security/$gate.sh"
                else
                    CARGO_TARGET_DIR="$configured" PATH="$WORK/bin:$PATH" bash "$fixture/scripts/security/$gate.sh"
                fi
            } 2>&1)"
            status=$?
            set -e
            expected_builds=0
            if [ "$mode" = build ]; then expected_builds=1; fi
            actual_builds="$(wc -l <"$BUILD_LOG")"
            target_link=''
            if [ "$location" != default ] && [ -L "$fixture/target" ]; then
                target_link="$(readlink "$fixture/target")"
            fi
            if [ "$status" -ne 0 ] || [ "$(cat "$SELECTED_LOG" 2>/dev/null || true)" != "$EXPECTED_BINARY" ] || [ "$actual_builds" -ne "$expected_builds" ]; then
                printf 'FAIL %s %s %s: exit=%s; %s\n' "$gate" "$mode" "$location" "$status" "$output" >&2
                failures=$((failures + 1))
            elif [ "$location" != default ] && { [ ! -L "$fixture/target" ] || [ "$target_link" != "$fixture/unavailable-cold-cache" ]; }; then
                printf 'FAIL %s changed the unrelated target symlink\n' "$gate" >&2
                failures=$((failures + 1))
            else
                printf 'PASS %s %s %s\n' "$gate" "$mode" "$location"
            fi
        done
    done
done
[ "$failures" -eq 0 ]
