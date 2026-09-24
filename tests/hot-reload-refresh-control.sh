#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-refresh-control.sh <nexa> <ios|android> <project-dir>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_refresh_control.nx"
log_file="$project/dev.log"

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create RefreshControlSmoke --directory "$project"
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
        if (( count > before )); then patch_count=$count; return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for the refresh-control patch" >&2
    return 1
}

wait_for_text() {
    local expected=$1
    for _ in $(seq 1 60); do
        adb shell uiautomator dump /sdcard/nexa-refresh-control.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-refresh-control.xml >"$project/build/refresh-control.xml"
        if grep -Fq "text=\"$expected\"" "$project/build/refresh-control.xml"; then return 0; fi
        sleep 0.5
    done
    cat "$project/build/refresh-control.xml" >&2
    return 1
}

platform_label=iOS
if [[ "$platform" == android ]]; then platform_label=Android; fi
wait_for_log "Nexa $platform_label dev runtime connected."
wait_for_log "Nexa $platform_label dev runtime applied module"

if [[ "$platform" == android ]]; then
    wait_for_text "Before refresh"
    patch_count=$(grep -Fc "Nexa Android dev runtime applied patch" "$log_file" || true)
    python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
replacements = {
    'Text("Before refresh")': 'Text("After reload")',
    'refreshes = refreshes + 1': 'refreshes = refreshes + 10',
}
for old, new in replacements.items():
    if old not in source:
        raise SystemExit(f"missing expected source fragment: {old}")
    source = source.replace(old, new, 1)
path.write_text(source)
PY
    wait_for_patch "$patch_count"
    wait_for_text "After reload"
    if grep -Fq 'text="Before refresh"' "$project/build/refresh-control.xml"; then
        echo "stale refresh-control content remained after reload" >&2
        exit 1
    fi
    size=$(adb shell wm size | sed -n 's/.*: \([0-9][0-9]*\)x\([0-9][0-9]*\).*/\1 \2/p' | tail -n 1)
    read -r width height <<<"$size"
    x=$((width / 2))
    start_y=$((height * 45 / 100))
    end_y=$((height * 82 / 100))
    adb shell input swipe "$x" "$start_y" "$x" "$end_y" 500
    wait_for_text "Refreshes: 10"
    echo "Nexa Android generic RefreshControl rendering, hot reload, and action passed."
    exit 0
fi

ios_project="$project/build/ios/RefreshControlSmoke.xcodeproj"
test_source="$project/build/ios/NexaHotReloadRefreshControlUITests.swift"
xcode_log="$log_file.xcodebuild.log"
ready_token=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
patched_token=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
verified_token=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
sed -e "s/__READY_TOKEN__/$ready_token/" \
    -e "s/__PATCHED_TOKEN__/$patched_token/" \
    -e "s/__VERIFIED_TOKEN__/$verified_token/" \
    "$script_dir/ios-hot-reload-refresh-control-ui.swift" >"$test_source"
test_runner_bundle_id=$(ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
    NexaHotReloadRefreshControlUITests.swift NexaHotReloadRefreshControlUITests)
simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
if [[ -z "$simulator_id" ]]; then echo "no booted iOS Simulator was found" >&2; exit 1; fi
xcodebuild test -project "$ios_project" -scheme NexaHotReloadRefreshControlUITests \
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

container=$(wait_for_marker nexa-refresh-control-ready "$ready_token")
patch_count=$(grep -Fc "Nexa iOS dev runtime applied patch" "$log_file" || true)
python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
replacements = {
    'Text("Before refresh")': 'Text("After reload")',
    'refreshes = refreshes + 1': 'refreshes = refreshes + 10',
}
for old, new in replacements.items():
    if old not in source:
        raise SystemExit(f"missing expected source fragment: {old}")
    source = source.replace(old, new, 1)
path.write_text(source)
PY
wait_for_patch "$patch_count"
container=$(wait_for_marker nexa-refresh-control-patched "$patched_token")
printf '%s' "$verified_token" >"$container/tmp/nexa-refresh-control-verified"
if ! wait "$ui_pid"; then ui_pid=; tail -n 100 "$xcode_log" >&2; exit 1; fi
ui_pid=
grep -Fq '** TEST SUCCEEDED **' "$xcode_log" || { tail -n 100 "$xcode_log" >&2; exit 1; }
echo "Nexa iOS generic RefreshControl rendering, hot reload, and action passed."
