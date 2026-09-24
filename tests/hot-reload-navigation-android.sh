#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-navigation-android.sh <nexa> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
project=$2
log_file=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_navigation_reload.nx"
artifacts="$project/build/test-artifacts"

"$nexa" create NavigationReloadSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
mkdir -p "$artifacts"
mkfifo "$project/dev-stdin"
exec 3<>"$project/dev-stdin"
cd "$project"
"$nexa" dev --android <&3 >"$log_file" 2>&1 &
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
    adb exec-out cat /sdcard/nexa-window.xml >"$artifacts/window.xml"
}

wait_for_visible_text() {
    local expected=$1
    for _ in $(seq 1 60); do
        visible_text
        if grep -Fq "text=\"$expected\"" "$artifacts/window.xml"; then return 0; fi
        sleep 1
    done
    cat "$artifacts/window.xml" >&2
    cat "$log_file" >&2
    echo "expected visible text: $expected" >&2
    return 1
}

tap_text() {
    local expected=$1
    visible_text
    local coordinates
    coordinates=$(python3 - "$artifacts/window.xml" "$expected" <<'PY'
import re
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
for node in root.iter("node"):
    if node.attrib.get("text") != sys.argv[2]:
        continue
    match = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.attrib.get("bounds", ""))
    if match:
        left, top, right, bottom = map(int, match.groups())
        print((left + right) // 2, (top + bottom) // 2)
        break
PY
)
    if [[ -z "$coordinates" ]]; then
        cat "$artifacts/window.xml" >&2
        echo "could not find visible text to tap: $expected" >&2
        return 1
    fi
    read -r x y <<<"$coordinates"
    adb shell input tap "$x" "$y"
}

wait_for_patch_after() {
    local previous_count=$1
    for _ in $(seq 1 180); do
        local current_count
        current_count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)
        if (( current_count > previous_count )); then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for the runtime to apply a navigation patch" >&2
    return 1
}

wait_for_log "Nexa Android dev runtime connected."
wait_for_log "Nexa Android dev runtime applied module"
wait_for_visible_text "Home Screen"
tap_text "Open Details"
wait_for_visible_text "Details Screen"
patch_count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("Details Screen")'
if old not in source:
    raise SystemExit("details screen text was not found")
path.write_text(source.replace(old, 'Text("Reloaded Details")', 1))
PY

wait_for_patch_after "$patch_count"
wait_for_visible_text "Reloaded Details"
tap_text "Go Home"
wait_for_visible_text "Home Screen"
echo "Nexa Android navigation restoration passed."
