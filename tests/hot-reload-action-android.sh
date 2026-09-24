#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-action-android.sh <nexa> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
project=$2
log_file=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_action_reload.nx"
artifacts="$project/build/test-artifacts"

"$nexa" create ActionReloadSmoke --directory "$project"
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

wait_for_accessible_description() {
    local expected=$1
    for _ in $(seq 1 60); do
        visible_text
        if grep -Fq "content-desc=\"$expected\"" "$artifacts/window.xml" \
            && python3 - "$artifacts/window.xml" "$expected" <<'PY'
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
matches = []
for node in root.iter("node"):
    descendants = list(node.iter())
    has_description = any(child.attrib.get("content-desc") == sys.argv[2] for child in descendants)
    has_button_role = any(child.attrib.get("class") == "android.widget.Button" for child in descendants)
    if has_description and has_button_role:
        matches.append((len(descendants), node.attrib.get("class")))
if matches and min(matches)[1] in {"android.view.View", "android.widget.Button"}:
    raise SystemExit(0)
raise SystemExit(1)
PY
        then
            return 0
        fi
        sleep 1
    done
    cat "$artifacts/window.xml" >&2
    cat "$log_file" >&2
    echo "expected accessibility description: $expected" >&2
    return 1
}

wait_for_accessible_label() {
    local expected=$1
    for _ in $(seq 1 60); do
        visible_text
        if grep -Fq "content-desc=\"$expected\"" "$artifacts/window.xml"; then return 0; fi
        sleep 1
    done
    cat "$artifacts/window.xml" >&2
    cat "$log_file" >&2
    echo "expected accessibility label: $expected" >&2
    return 1
}

row_x() {
    python3 - "$artifacts/window.xml" "$1" <<'PY'
import re
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
for node in root.iter("node"):
    if node.attrib.get("text") == sys.argv[2]:
        match = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.attrib.get("bounds", ""))
        if match:
            left, _, right, _ = map(int, match.groups())
            print((left + right) // 2)
            break
PY
}

stack_center() {
    python3 - "$artifacts/window.xml" "$1" <<'PY'
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
        print((left + right) // 2)
        break
PY
}

screen_center_x() {
    python3 - "$artifacts/window.xml" <<'PY'
import re
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
for node in root.iter("node"):
    if node.attrib.get("class") != "androidx.compose.ui.platform.ComposeView":
        continue
    match = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.attrib.get("bounds", ""))
    if match:
        left, _, right, _ = map(int, match.groups())
        print((left + right) // 2)
        break
PY
}

assert_stack_overlay() {
    local overlay=$1
    visible_text
    local viewport_center overlay_center
    viewport_center=$(screen_center_x)
    overlay_center=$(stack_center "$overlay")
    if [[ -z "$viewport_center" || -z "$overlay_center" ]] || (( overlay_center < viewport_center - 2 || overlay_center > viewport_center + 2 )); then
        echo "stack overlay is not horizontally centered: viewport=$viewport_center overlay=$overlay_center" >&2
        cat "$artifacts/window.xml" >&2
        return 1
    fi
}

assert_direction() {
    local expected=$1
    visible_text
    local start_x end_x
    start_x=$(row_x "Row Start")
    end_x=$(row_x "Row End")
    if [[ -z "$start_x" || -z "$end_x" ]]; then
        cat "$artifacts/window.xml" >&2
        echo "could not read row item positions" >&2
        return 1
    fi
    if [[ "$expected" == rtl && "$start_x" -le "$end_x" ]] || [[ "$expected" == ltr && "$start_x" -ge "$end_x" ]]; then
        echo "expected $expected row order; start=$start_x end=$end_x" >&2
        cat "$artifacts/window.xml" >&2
        return 1
    fi
}

assert_status_bar_visibility() {
    local expected=$1
    visible_text
    local content_top
    content_top=$(python3 - "$artifacts/window.xml" <<'PY'
import re
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
for node in root.iter("node"):
    if node.attrib.get("resource-id") == "android:id/content":
        match = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.attrib.get("bounds", ""))
        if match:
            print(match.group(2))
            break
PY
)
    if [[ -z "$content_top" ]]; then
        cat "$artifacts/window.xml" >&2
        echo "could not read Android content top edge" >&2
        return 1
    fi
    if [[ "$expected" == visible && "$content_top" -eq 0 ]] || [[ "$expected" == hidden && "$content_top" -ne 0 ]]; then
        echo "expected status bar $expected; app content begins at y=$content_top" >&2
        cat "$artifacts/window.xml" >&2
        return 1
    fi
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
expected = sys.argv[2]
for node in root.iter("node"):
    if node.attrib.get("text") != expected:
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
        echo "could not find clickable text: $expected" >&2
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
    echo "timed out waiting for the runtime to apply an action patch" >&2
    return 1
}

wait_for_log "Nexa Android dev runtime connected."
wait_for_log "Nexa Android dev runtime applied module"
wait_for_visible_text "Row Start"
assert_direction rtl
wait_for_visible_text "Stack Overlay V1"
assert_stack_overlay "Stack Overlay V1"
assert_status_bar_visibility visible
wait_for_accessible_description "Accessible Action V1"
wait_for_accessible_label "Accessible Link V1"
wait_for_accessible_label "Accessible Header V1"
wait_for_accessible_label "Accessible Image V1"
wait_for_accessible_label "Accessible Plain V1"
wait_for_visible_text "Count: 0"
wait_for_visible_text "Presses: 0"
tap_text "Increment"
wait_for_visible_text "Count: 1"
tap_text "Press me"
wait_for_visible_text "Presses: 1"
patch_count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = "count = count + 1"
if old not in source:
    raise SystemExit("button action was not found")
source = source.replace(old, "count = count + 10", 1)
old_pressable = "taps = taps + 1"
if old_pressable not in source:
    raise SystemExit("pressable action was not found")
source = source.replace(old_pressable, "taps = taps + 10", 1)
old_direction = "Direction(value: RTL)"
if old_direction not in source:
    raise SystemExit("RTL direction setting was not found")
source = source.replace(old_direction, "Direction(value: LTR)", 1)
old_status = 'StatusBar(style: Light, hidden: false, background: "#223344")'
if old_status not in source:
    raise SystemExit("visible light status bar setting was not found")
source = source.replace(old_status, 'StatusBar(style: Dark, hidden: true, background: "#445566")', 1)
old_accessibility = 'label: "Accessible Action V1"'
if old_accessibility not in source:
    raise SystemExit("accessible button label was not found")
source = source.replace(old_accessibility, 'label: "Accessible Action V2"', 1)
old_stack = 'Text("Stack Overlay V1")'
if old_stack not in source:
    raise SystemExit("stack overlay label was not found")
source = source.replace(old_stack, 'Text("Stack Overlay V2")', 1)
for old, new in [
    ('label: "Accessible Link V1"', 'label: "Accessible Link V2"'),
    ('label: "Accessible Header V1"', 'label: "Accessible Header V2"'),
    ('label: "Accessible Image V1"', 'label: "Accessible Image V2"'),
    ('label: "Accessible Plain V1"', 'label: "Accessible Plain V2"'),
]:
    if old not in source:
        raise SystemExit(f"accessibility label was not found: {old}")
    source = source.replace(old, new, 1)
path.write_text(source)
PY

wait_for_patch_after "$patch_count"
wait_for_visible_text "Row Start"
assert_direction ltr
wait_for_visible_text "Stack Overlay V2"
assert_stack_overlay "Stack Overlay V2"
assert_status_bar_visibility hidden
wait_for_accessible_description "Accessible Action V2"
wait_for_accessible_label "Accessible Link V2"
wait_for_accessible_label "Accessible Header V2"
wait_for_accessible_label "Accessible Image V2"
wait_for_accessible_label "Accessible Plain V2"
tap_text "Increment"
wait_for_visible_text "Count: 11"
tap_text "Press me"
wait_for_visible_text "Presses: 11"
echo "Nexa Android action hot reload passed."
