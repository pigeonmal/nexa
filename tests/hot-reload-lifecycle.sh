#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-lifecycle.sh <nexa> <ios|android> <project-dir>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/fixtures/dev_lifecycle.nx"
log_file="$project/dev.log"

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create LifecycleSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
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

wait_for_patch() {
    local before=$1
    for _ in $(seq 1 180); do
        local count
        count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
        if (( count > before )); then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for a lifecycle action patch" >&2
    return 1
}

platform_label=iOS
if [[ "$platform" == android ]]; then platform_label=Android; fi
wait_for_log "Nexa $platform_label dev runtime connected."
wait_for_log "Nexa $platform_label dev runtime applied module"

patch_source() {
    python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = "activeEvents = activeEvents + 1"
new = "activeEvents = activeEvents + 10"
if old not in source:
    raise SystemExit(f"missing expected lifecycle action: {old}")
path.write_text(source.replace(old, new, 1))
PY
}

if [[ "$platform" == android ]]; then
    app_id=$(python3 - "$project/build/android/app/build.gradle.kts" <<'PY'
import re, sys
from pathlib import Path
source = Path(sys.argv[1]).read_text()
match = re.search(r'applicationId\s*=\s*"([^"]+)"', source)
if not match:
    raise SystemExit("generated Android applicationId was not found")
print(match.group(1))
PY
)
    xml="$project/build/lifecycle.xml"
    wait_for_text() {
        local expected=$1
        for _ in $(seq 1 90); do
            adb shell uiautomator dump /sdcard/nexa-lifecycle.xml >/dev/null 2>&1
            adb exec-out cat /sdcard/nexa-lifecycle.xml >"$xml"
            if grep -Fq "text=\"$expected\"" "$xml"; then return 0; fi
            sleep 1
        done
        cat "$xml" >&2
        return 1
    }
    tap_text() {
        local target=$1
        adb shell uiautomator dump /sdcard/nexa-lifecycle.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-lifecycle.xml >"$xml"
        read -r x y < <(python3 - "$xml" "$target" <<'PY'
import re, sys
from pathlib import Path
xml, target = Path(sys.argv[1]).read_text(), sys.argv[2]
match = re.search(r'<node[^>]*text="' + re.escape(target) + r'"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"', xml)
if not match:
    raise SystemExit(f"could not find tappable text {target!r}")
x1, y1, x2, y2 = map(int, match.groups())
print((x1 + x2) // 2, (y1 + y2) // 2)
PY
)
        adb shell input tap "$x" "$y"
    }
    wait_for_text "Appears: 1"
    wait_for_text "Active: 1"
    tap_text "Open detail"
    wait_for_text "Detail visits: 1"
    tap_text "Back"
    wait_for_text "Detail leaves: 1"
    patch_count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)
    patch_source
    wait_for_patch "$patch_count"
    adb shell input keyevent 3
    sleep 2
    adb shell monkey -p "$app_id" 1 >/dev/null 2>&1
    wait_for_text "Inactive: 1"
    wait_for_text "Background: 1"
    wait_for_text "Active: 11"
    wait_for_text "Appears: 1"
    echo "Nexa Android lifecycle callbacks and hot-reloaded action passed."
    exit 0
fi

ios_project="$project/build/ios/LifecycleSmoke.xcodeproj"
test_source="$project/build/ios/NexaHotReloadLifecycleUITests.swift"
xcode_log="$log_file.xcodebuild.log"
ready_token=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
patched_token=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
sed -e "s/__READY_TOKEN__/$ready_token/" \
    -e "s/__PATCHED_TOKEN__/$patched_token/" \
    "$script_dir/ios-hot-reload-lifecycle-ui.swift" >"$test_source"
test_runner_bundle_id=$(ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
    NexaHotReloadLifecycleUITests.swift NexaHotReloadLifecycleUITests)
simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
if [[ -z "$simulator_id" ]]; then echo "no booted iOS Simulator was found" >&2; exit 1; fi
xcodebuild test -project "$ios_project" -scheme NexaHotReloadLifecycleUITests \
    -destination "platform=iOS Simulator,id=$simulator_id" CODE_SIGNING_ALLOWED=NO \
    >"$xcode_log" 2>&1 &
ui_pid=$!

wait_for_marker() {
    local file_name=$1
    local expected=$2
    for _ in $(seq 1 240); do
        local container
        container=$(xcrun simctl get_app_container "$simulator_id" "$test_runner_bundle_id" data 2>/dev/null || true)
        if [[ -n "$container" && -f "$container/tmp/$file_name" ]] \
            && [[ "$(cat "$container/tmp/$file_name")" == "$expected" ]]; then
            echo "$container"
            return 0
        fi
        if ! kill -0 "$ui_pid" 2>/dev/null; then tail -n 100 "$xcode_log" >&2; return 1; fi
        sleep 0.5
    done
    tail -n 100 "$xcode_log" >&2
    echo "timed out waiting for UI test marker $file_name" >&2
    return 1
}

container=$(wait_for_marker nexa-lifecycle-ready "$ready_token")
patch_count=$(grep -Fc "Nexa iOS dev runtime applied patch" "$log_file" || true)
patch_source
wait_for_patch "$patch_count"
container=$(xcrun simctl get_app_container "$simulator_id" "$test_runner_bundle_id" data)
printf '%s' "$patched_token" >"$container/tmp/nexa-lifecycle-patched"
if ! wait "$ui_pid"; then ui_pid=; tail -n 100 "$xcode_log" >&2; exit 1; fi
ui_pid=
grep -Fq '** TEST SUCCEEDED **' "$xcode_log" || { tail -n 100 "$xcode_log" >&2; exit 1; }
echo "Nexa iOS lifecycle callbacks and hot-reloaded action passed."
