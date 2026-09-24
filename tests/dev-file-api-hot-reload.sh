#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: dev-file-api-hot-reload.sh <nexa> <ios|android> <project-dir>" >&2
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

"$nexa" create FileApiSmoke --directory "$project"
cp "$script_dir/fixtures/dev_file_api_initial.nx" "$project/App.nx"
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

platform_label=iOS
if [[ "$platform" == android ]]; then platform_label=Android; fi
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
    echo "timed out waiting for the file API hot reload" >&2
    return 1
}

wait_for_log "Nexa $platform_label dev runtime connected."
wait_for_log "Nexa $platform_label dev runtime applied module"
patch_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
cp "$script_dir/fixtures/dev_file_api_reloaded.nx" "$project/App.nx"
wait_for_patch "$patch_count"
printf 'R\n' >&3
wait_for_log "Nexa app state restarted in the running app."

if [[ "$platform" == android ]]; then
    xml="$project/build/file-api.xml"
    for _ in $(seq 1 90); do
        adb shell uiautomator dump /sdcard/nexa-file-api.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-file-api.xml >"$xml"
        if grep -Fq 'text="File: FILE_OK"' "$xml" \
            && grep -Fq 'text="Exists: true"' "$xml" \
            && grep -Eq 'text="Permission: (granted|denied|restricted|notDetermined)"' "$xml"; then
            echo "Nexa Android dev File/Path hot reload passed."
            exit 0
        fi
        if grep -Fq 'text="File: ERROR"' "$xml"; then
            cat "$xml" >&2
            cat "$log_file" >&2
            exit 1
        fi
        sleep 1
    done
    cat "$xml" >&2
    cat "$log_file" >&2
    exit 1
fi

ios_project="$project/build/ios/DevFileApiSmoke.xcodeproj"
test_source="$project/build/ios/NexaDevFileApiUITests.swift"
xcode_log="$log_file.xcodebuild.log"
cp "$script_dir/ios-dev-file-api-ui.swift" "$test_source"
ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
    NexaDevFileApiUITests.swift NexaDevFileApiUITests >/dev/null
simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
if [[ -z "$simulator_id" ]]; then echo "no booted iOS Simulator was found" >&2; exit 1; fi
xcodebuild test -project "$ios_project" -scheme NexaDevFileApiUITests \
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
echo "Nexa iOS dev File/Path hot reload passed."
