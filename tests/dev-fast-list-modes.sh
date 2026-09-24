#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: dev-fast-list-modes.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
log_file=$4
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_fast_list_modes.nx"
test_target=NexaDevFastListModesUITests
xcode_log="$log_file.xcodebuild.log"
ui_pid=

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create FastListModesSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
mkfifo "$project/dev-stdin"
exec 3<>"$project/dev-stdin"
cd "$project"
"$nexa" dev "--$platform" <&3 >"$log_file" 2>&1 &
dev_pid=$!

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

wait_for_log "Nexa $( [[ "$platform" == ios ]] && echo iOS || echo Android ) dev runtime applied module"
if [[ "$platform" == android ]]; then
    wait_for_accessibility_text() {
        local expected=$1
        for _ in $(seq 1 30); do
            adb shell uiautomator dump /sdcard/nexa-fast-list-modes.xml >/dev/null 2>&1
            adb exec-out cat /sdcard/nexa-fast-list-modes.xml >"$project/build/nexa-fast-list-modes.xml"
            if grep -Eq "$expected" "$project/build/nexa-fast-list-modes.xml"; then return 0; fi
            sleep 0.5
        done
        return 1
    }
    for _ in $(seq 1 60); do
        adb shell uiautomator dump /sdcard/nexa-fast-list-modes.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-fast-list-modes.xml >"$project/build/nexa-fast-list-modes.xml"
        if grep -Fq 'text="Section 0"' "$project/build/nexa-fast-list-modes.xml" \
            && grep -Fq 'text="Section 0 row 0: A1"' "$project/build/nexa-fast-list-modes.xml" \
            && grep -Fq 'text="Section 1 row 0: B1"' "$project/build/nexa-fast-list-modes.xml" \
            && grep -Fq 'text="Grid 0"' "$project/build/nexa-fast-list-modes.xml"; then
            break
        fi
        sleep 1
    done
    wait_for_accessibility_text 'Events [0-9]+ pages 0 refreshes 0' || { cat "$project/build/nexa-fast-list-modes.xml" >&2; exit 1; }
    adb shell input swipe 360 240 360 1200 300
    wait_for_accessibility_text 'refreshes 1' || { cat "$project/build/nexa-fast-list-modes.xml" >&2; exit 1; }
    adb shell input swipe 360 1400 360 300 250
    adb shell input swipe 360 1400 360 300 250
    wait_for_accessibility_text 'pages 1' || { cat "$project/build/nexa-fast-list-modes.xml" >&2; exit 1; }
    grep -Eq 'Events [1-9][0-9]* pages 1 refreshes 1' "$project/build/nexa-fast-list-modes.xml" || {
        cat "$project/build/nexa-fast-list-modes.xml" >&2
        exit 1
    }
    echo "Nexa Android DevRuntime FastList sections, headers, grid, callbacks, and refresh wrapper passed."
    exit 0
fi

ios_project="$project/build/ios/FastListModesSmoke.xcodeproj"
test_source="$project/build/ios/NexaDevFastListModesUITests.swift"
cp "$script_dir/ios-dev-fast-list-modes-ui.swift" "$test_source"
test_runner_bundle_id=$(ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
    NexaDevFastListModesUITests.swift "$test_target")
simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
if [[ -z "$simulator_id" ]]; then echo "no booted iOS Simulator was found" >&2; exit 1; fi
xcodebuild test -project "$ios_project" -scheme "$test_target" \
    -destination "platform=iOS Simulator,id=$simulator_id" CODE_SIGNING_ALLOWED=NO \
    >"$xcode_log" 2>&1 &
ui_pid=$!
if ! wait "$ui_pid"; then ui_pid=; tail -n 100 "$xcode_log" >&2; exit 1; fi
ui_pid=
grep -Fq '** TEST SUCCEEDED **' "$xcode_log" || { tail -n 100 "$xcode_log" >&2; exit 1; }
echo "Nexa iOS DevRuntime FastList sections, headers, and grid rendering passed."
