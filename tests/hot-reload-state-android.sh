#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-state-android.sh <nexa> <project-dir> <log-file> <fixture>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
project=$2
log_file=$3
fixture=$4

"$nexa" create StateReloadSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
adb shell am force-stop dev.nexa.statereloadsmoke
mkfifo "$project/dev-stdin"
exec 3<>"$project/dev-stdin"
cd "$project"
"$nexa" dev --android <"$project/dev-stdin" >"$log_file" 2>&1 &
dev_pid=$!

cleanup() {
    kill -INT "$dev_pid" 2>/dev/null || true
    for _ in $(seq 1 10); do
        kill -0 "$dev_pid" 2>/dev/null || break
        sleep 1
    done
    kill -TERM "$dev_pid" 2>/dev/null || true
    wait "$dev_pid" 2>/dev/null || true
    exec 3>&-
}
trap cleanup EXIT

wait_for_log() {
    local pattern=$1
    for _ in $(seq 1 180); do
        if grep -Fq "$pattern" "$log_file"; then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for: $pattern" >&2
    return 1
}

visible_text() {
    adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1
    adb exec-out cat /sdcard/nexa-window.xml >"$project/window.xml"
}

require_visible_text() {
    local expected=$1
    for _ in $(seq 1 60); do
        visible_text
        if grep -Fq "text=\"$expected\"" "$project/window.xml"; then return 0; fi
        sleep 1
    done
    cat "$project/window.xml" >&2
    cat "$log_file" >&2
    echo "expected visible text: $expected" >&2
    return 1
}

wait_for_patch_after() {
    local previous_count=$1
    for _ in $(seq 1 180); do
        local current_count
        current_count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)
        if (( current_count > previous_count )); then
            patch_count=$current_count
            return 0
        fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for a state reload patch" >&2
    return 1
}

wait_for_log "Nexa Android dev runtime connected."
wait_for_log "Nexa Android dev runtime applied module"
require_visible_text "Count: 7"
patch_count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = "state count = 7"
if old not in source:
    raise SystemExit("initial count state was not found")
path.write_text(source.replace(old, "state count = 99", 1))
PY
wait_for_patch_after "$patch_count"
require_visible_text "Count: 7"

printf 'R\n' >&3
wait_for_log "Nexa app state restarted in the running app."
require_visible_text "Count: 99"
echo "Nexa Android hot restart restored the current source initializer."

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = "state count = 99"
if old not in source:
    raise SystemExit("updated count initializer was not found")
path.write_text(source.replace(old, 'state count: String = "fresh"', 1))
PY
wait_for_patch_after "$patch_count"
require_visible_text "Count: fresh"
echo "Nexa Android compatible-state preservation and incompatible-state reset passed."
