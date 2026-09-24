#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-components.sh <nexa> <ios|android> <project-dir>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
log_file="$project/dev.log"

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create ComponentSmoke --directory "$project"
cp "$script_dir/fixtures/dev_component_initial.nx" "$project/App.nx"
mkfifo "$project/dev-stdin"
exec 3<>"$project/dev-stdin"
cd "$project"
"$nexa" dev "--$platform" <&3 >"$log_file" 2>&1 &
dev_pid=$!
ui_pid=

cleanup() {
    if [[ -n "$ui_pid" ]]; then
        kill -TERM "$ui_pid" 2>/dev/null || true
        wait "$ui_pid" 2>/dev/null || true
    fi
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

label=iOS
if [[ "$platform" == android ]]; then label=Android; fi
wait_for_log() {
    local expected=$1
    for _ in $(seq 1 180); do
        if grep -Fq "$expected" "$log_file"; then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for: $expected" >&2
    return 1
}

wait_for_log "Nexa $label dev runtime connected."
wait_for_log "Nexa $label dev runtime applied module"
patch_count=$(grep -Fc "Nexa $label dev runtime applied patch" "$log_file" || true)
module_count=$(grep -Fc "Nexa $label dev runtime applied module" "$log_file" || true)
cp "$script_dir/fixtures/dev_component_reloaded.nx" "$project/App.nx"
for _ in $(seq 1 180); do
    count=$(grep -Fc "Nexa $label dev runtime applied patch" "$log_file" || true)
    modules=$(grep -Fc "Nexa $label dev runtime applied module" "$log_file" || true)
    if (( count > patch_count || modules > module_count )); then break; fi
    if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
    sleep 1
done
if (( count <= patch_count && modules <= module_count )); then
    cat "$log_file" >&2
    echo "timed out waiting for the component hot reload" >&2
    exit 1
fi
for style in Light Dark Default; do
    previous_patches=$(grep -Fc "Nexa $label dev runtime applied patch" "$log_file" || true)
    previous_modules=$(grep -Fc "Nexa $label dev runtime applied module" "$log_file" || true)
    python3 - "$project/App.nx" "$style" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
style = sys.argv[2]
source = path.read_text()
old = next((line for line in source.splitlines() if "StatusBar(style:" in line), None)
if old is None:
    raise SystemExit("StatusBar declaration is missing from the reload fixture")
new = old.replace(old.split("StatusBar(style:", 1)[1].split(",", 1)[0], f" {style}", 1)
path.write_text(source.replace(old, new, 1))
PY
    for _ in $(seq 1 180); do
        count=$(grep -Fc "Nexa $label dev runtime applied patch" "$log_file" || true)
        modules=$(grep -Fc "Nexa $label dev runtime applied module" "$log_file" || true)
        if (( count > previous_patches || modules > previous_modules )); then break; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
        sleep 0.25
    done
    if (( count <= previous_patches && modules <= previous_modules )); then
        cat "$log_file" >&2
        echo "timed out waiting for StatusBar($style) hot reload" >&2
        exit 1
    fi
done
previous_patches=$(grep -Fc "Nexa $label dev runtime applied patch" "$log_file" || true)
previous_modules=$(grep -Fc "Nexa $label dev runtime applied module" "$log_file" || true)
python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("When branch: ready")'
if old not in source:
    raise SystemExit("When ready branch was not found")
path.write_text(source.replace(old, 'Text("When branch: reloaded")', 1))
PY
for _ in $(seq 1 180); do
    count=$(grep -Fc "Nexa $label dev runtime applied patch" "$log_file" || true)
    modules=$(grep -Fc "Nexa $label dev runtime applied module" "$log_file" || true)
    if (( count > previous_patches || modules > previous_modules )); then break; fi
    if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
    sleep 0.25
done
if (( count <= previous_patches && modules <= previous_modules )); then
    cat "$log_file" >&2
    echo "timed out waiting for the When branch hot reload" >&2
    exit 1
fi
printf 'R\n' >&3
wait_for_log "Nexa app state restarted in the running app."

if [[ "$platform" == android ]]; then
    xml="$project/build/components.xml"
    dump="$project/build/components-current.xml"
    : >"$xml"
    for _ in $(seq 1 30); do
        adb shell uiautomator dump /sdcard/nexa-components.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-components.xml >"$dump"
        cat "$dump" >>"$xml"
        if grep -Fq 'text="Card: Reloaded"' "$xml" \
            && grep -Fq 'text="Increment button: 0"' "$xml" \
            && grep -Fq 'text="Reload switch"' "$xml" \
            && grep -Fq 'text="When branch: reloaded"' "$xml" \
            && grep -Fq 'text="Stack layout"' "$xml" \
            && grep -Fq 'text="Projected child"' "$xml" \
            && grep -Fq 'Binary and: true' "$xml" \
            && grep -Fq 'Contains: true' "$xml" \
            && grep -Fq 'Index: 5' "$xml" \
            && grep -Fq 'Coalesce: fallback' "$xml" \
            && grep -Fq 'Pair: 7, Mapped: 6, Filtered: 8, Reduced: 16' "$xml" \
            && grep -Fq 'Int8: 8, Int16: 16, Int64: 64' "$xml" \
            && grep -Fq 'Float32: 3.5, Float64: 6.5' "$xml" \
            && grep -Fq 'Set: true, Map: 42, Triple: 4, Enum: ready, Result: 42, Result error: null, Struct: Ada' "$xml" \
            && grep -Fq 'Array mutation: 2, Set mutation: true, Map mutation: 2' "$xml" \
            && grep -Fq 'For: 6, While: 3, Break: 2, Continue: 5, ForMap: 2, TryCatch: 1' "$xml" \
            && grep -Fq 'text="Other comparisons: true, true, true, true, true, true"' "$xml" \
            && grep -Fq 'text="Size classes: false, true, true, false"' "$xml" \
            && grep -Fq 'text="Native path: ' "$xml" \
            && grep -Fq 'text="Static style"' "$xml" \
            && grep -Fq 'text="Adaptive style"' "$xml" \
            && grep -Fq 'text="Semibold style"' "$xml" \
            && grep -Fq 'text="Bold style"' "$xml" \
            && grep -Fq 'text="Styled start"' "$xml" \
            && grep -Fq 'text="Styled center"' "$xml" \
            && grep -Fq 'text="Styled end"' "$xml" \
            && grep -Fq 'text="Ease in out"' "$xml" \
            && grep -Fq 'text="Linear"' "$xml" \
            && grep -Fq 'text="Accessible link"' "$xml" \
            && grep -Fq 'text="Accessible header"' "$xml" \
            && grep -Fq 'text="Accessible image"' "$xml" \
            && grep -Fq 'text="Accessible none"' "$xml" \
            && grep -Fq 'text="External link"' "$xml" \
            && grep -Fq 'text="Keyboard text"' "$xml" \
            && grep -Fq 'text="Keyboard number"' "$xml" \
            && grep -Fq 'text="Keyboard email"' "$xml" \
            && grep -Fq 'text="Keyboard phone"' "$xml" \
            && grep -Fq 'text="Keyboard url"' "$xml" \
            && grep -Fq 'text="Never dismiss"' "$xml" \
            && grep -Fq 'text="Haptic light"' "$xml" \
            && grep -Fq 'text="Haptic medium"' "$xml" \
            && grep -Fq 'text="Haptic heavy"' "$xml"; then
            python3 - "$dump" <<'PY'
import subprocess
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
for label in ("Haptic light", "Haptic medium", "Haptic heavy"):
    node = next((item for item in root.iter("node") if item.attrib.get("text") == label), None)
    if node is None:
        raise SystemExit(f"missing Android control for {label}")
    left, top, right, bottom = map(int, node.attrib["bounds"].strip("[]").replace("][", ",").split(","))
    x, y = (left + right) // 2, (top + bottom) // 2
    subprocess.run(["adb", "shell", "input", "tap", str(x), str(y)], check=True)
PY
            adb shell input swipe 360 400 360 1400 250 >/dev/null 2>&1 || true
            adb shell uiautomator dump /sdcard/nexa-components.xml >/dev/null 2>&1
            adb exec-out cat /sdcard/nexa-components.xml >"$dump"
            python3 - "$dump" <<'PY'
import subprocess
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
node = next((item for item in root.iter("node") if item.attrib.get("text") == "Increment button: 0"), None)
if node is None:
    raise SystemExit("missing Android increment button")
left, top, right, bottom = map(int, node.attrib["bounds"].strip("[]").replace("][", ",").split(","))
subprocess.run(["adb", "shell", "input", "tap", str((left + right) // 2), str((top + bottom) // 2)], check=True)
PY
            for _ in $(seq 1 15); do
                adb shell uiautomator dump /sdcard/nexa-components.xml >/dev/null 2>&1
                adb exec-out cat /sdcard/nexa-components.xml >"$dump"
                if grep -Fq 'text="Increment button: 1"' "$dump"; then break; fi
                sleep 0.25
            done
            if ! grep -Fq 'text="Increment button: 1"' "$dump"; then
                cat "$dump" >&2
                echo "Android Button action did not update state" >&2
                exit 1
            fi
            python3 - "$dump" <<'PY'
import subprocess
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
node = next((item for item in root.iter("node") if item.attrib.get("text") == "Enable branch"), None)
if node is None:
    raise SystemExit("missing Android conditional-branch button")
left, top, right, bottom = map(int, node.attrib["bounds"].strip("[]").replace("][", ",").split(","))
subprocess.run(["adb", "shell", "input", "tap", str((left + right) // 2), str((top + bottom) // 2)], check=True)
PY
            for _ in $(seq 1 15); do
                adb shell uiautomator dump /sdcard/nexa-components.xml >/dev/null 2>&1
                adb exec-out cat /sdcard/nexa-components.xml >"$dump"
                if grep -Fq 'text="If branch: enabled"' "$dump"; then break; fi
                sleep 0.25
            done
            if ! grep -Fq 'text="If branch: enabled"' "$dump"; then
                cat "$dump" >&2
                echo "Android If branch did not update after Button interaction" >&2
                exit 1
            fi
            echo "Nexa Android custom component and Content hot reload passed."
            exit 0
        fi
        adb shell input swipe 360 1200 360 450 250 >/dev/null 2>&1 || true
        sleep 0.25
    done
    cat "$xml" >&2
    cat "$log_file" >&2
    exit 1
fi

ios_project="$project/build/ios/ComponentSmoke.xcodeproj"
test_source="$project/build/ios/NexaDevComponentUITests.swift"
xcode_log="$log_file.xcodebuild.log"
cp "$script_dir/ios-dev-component-ui.swift" "$test_source"
ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
    NexaDevComponentUITests.swift NexaDevComponentUITests >/dev/null
simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
if [[ -z "$simulator_id" ]]; then echo "no booted iOS Simulator was found" >&2; exit 1; fi
xcodebuild test -project "$ios_project" -scheme NexaDevComponentUITests \
    -destination "platform=iOS Simulator,id=$simulator_id" CODE_SIGNING_ALLOWED=NO \
    >"$xcode_log" 2>&1 &
ui_pid=$!
if ! wait "$ui_pid"; then
    ui_pid=
    tail -n 100 "$xcode_log" >&2
    cat "$log_file" >&2
    exit 1
fi
ui_pid=
grep -Fq '** TEST SUCCEEDED **' "$xcode_log" || { tail -n 100 "$xcode_log" >&2; exit 1; }
echo "Nexa iOS custom component and Content hot reload passed."
