#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-bottom-bar-android.sh <nexa> <project-dir> <log-file> <fixture>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
project=$2
log_file=$3
fixture=$4

"$nexa" create BottomBarSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
adb shell am force-stop dev.nexa.bottombarsmoke
cd "$project"
"$nexa" dev --android >"$log_file" 2>&1 &
dev_pid=$!

cleanup() {
    kill -INT "$dev_pid" 2>/dev/null || true
    for _ in $(seq 1 10); do
        kill -0 "$dev_pid" 2>/dev/null || break
        sleep 1
    done
    kill -TERM "$dev_pid" 2>/dev/null || true
    wait "$dev_pid" 2>/dev/null || true
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

tap_text() {
    local point
    point=$(python3 - "$project/window.xml" "Settings" <<'PY'
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

root = ET.parse(Path(sys.argv[1])).getroot()
parents = {child: parent for parent in root.iter() for child in parent}
for element in root.iter("node"):
    if element.attrib.get("text") != sys.argv[2]:
        continue
    clickable = element
    while clickable is not None and clickable.attrib.get("clickable") != "true":
        clickable = parents.get(clickable)
    if clickable is None:
        continue
    bounds = clickable.attrib.get("bounds", "")
    numbers = [int(value) for value in bounds.replace("][", ",").strip("[]").split(",")]
    if len(numbers) == 4:
        print((numbers[0] + numbers[2]) // 2, (numbers[1] + numbers[3]) // 2)
        raise SystemExit(0)
raise SystemExit("clickable Settings tab was not found")
PY
    )
    adb shell input tap $point
}

wait_for_log "Nexa Android dev runtime connected."
wait_for_log "Nexa Android dev runtime applied module"
require_visible_text "Home tab"
visible_text
tap_text
require_visible_text "Settings tab"
baseline=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("Settings tab")'
if old not in source:
    raise SystemExit("settings tab fixture content was not found")
path.write_text(source.replace(old, 'Text("Reloaded settings tab")', 1))
PY

patched=0
for _ in $(seq 1 180); do
    current=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)
    if (( current > baseline )); then patched=1; break; fi
    if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
    sleep 1
done
if (( patched == 0 )); then
    cat "$log_file" >&2
    echo "the running bottom bar did not receive a hot-reload patch" >&2
    exit 1
fi
require_visible_text "Reloaded settings tab"
echo "Nexa Android hot-reload bottom-bar smoke passed."
