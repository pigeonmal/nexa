#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-direction-android.sh <nexa> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
project=$2
log_file=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fifo="$project/dev-stdin"

"$nexa" create DirectionSmoke --directory "$project"
cp "$script_dir/fixtures/dev_direction_reload.nx" "$project/App.nx"
mkfifo "$fifo"
exec 3<>"$fifo"
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
    local expected=$1
    for _ in $(seq 1 120); do
        if grep -Fq "$expected" "$log_file"; then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 0.25
    done
    cat "$log_file" >&2
    echo "timed out waiting for: $expected" >&2
    return 1
}

check_order() {
    local expected=$1
    local xml="$project/build/direction.xml"
    for _ in $(seq 1 30); do
        adb shell uiautomator dump /sdcard/nexa-direction.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-direction.xml >"$xml"
        if python3 - "$xml" "$expected" <<'PY'
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
positions = {}
for node in root.iter("node"):
    label = node.attrib.get("text")
    if label not in ("Row Start", "Row End"):
        continue
    positions[label] = int(node.attrib["bounds"].split(",", 1)[0].lstrip("["))
if set(positions) != {"Row Start", "Row End"}:
    raise SystemExit(1)
start_before_end = positions["Row Start"] < positions["Row End"]
if start_before_end != (sys.argv[2] == "ltr"):
    raise SystemExit(1)
PY
        then
            echo "Nexa Android direction is $expected: $(tr '\n' ' ' <"$xml")"
            return 0
        fi
        sleep 0.25
    done
    cat "$xml" >&2
    echo "Android did not place Row Start before Row End for $expected direction" >&2
    return 1
}

wait_for_log "Nexa Android dev runtime connected."
wait_for_log "Nexa Android dev runtime applied module"
check_order ltr
patch_count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)
python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = "Direction(value: LTR)"
if old not in source:
    raise SystemExit("LTR direction was not found")
path.write_text(source.replace(old, "Direction(value: RTL)", 1))
PY
for _ in $(seq 1 120); do
    count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)
    if (( count > patch_count )); then break; fi
    if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
    sleep 0.25
done
if (( count <= patch_count )); then
    cat "$log_file" >&2
    echo "timed out waiting for the RTL direction patch" >&2
    exit 1
fi
check_order rtl
echo "Nexa Android Direction LTR-to-RTL hot reload passed."
