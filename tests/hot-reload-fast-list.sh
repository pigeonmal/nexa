#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-fast-list.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
log_file=$4
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_fast_list.nx"
artifacts="$project/build/test-artifacts"
test_target=NexaHotReloadFastListUITests
xcode_log="$log_file.xcodebuild.log"
ui_pid=

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create FastListSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
mkdir -p "$artifacts"
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

platform_label=iOS
if [[ "$platform" == android ]]; then platform_label=Android; fi

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

wait_for_visible_text() {
    local expected=$1
    for _ in $(seq 1 60); do
        if [[ "$platform" == android ]]; then
            adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1
            adb exec-out cat /sdcard/nexa-window.xml >"$artifacts/window.xml"
            if grep -Fq "text=\"$expected\"" "$artifacts/window.xml"; then return 0; fi
        fi
        sleep 1
    done
    if [[ "$platform" == android ]]; then
        cat "$artifacts/window.xml" >&2
    fi
    cat "$log_file" >&2
    echo "expected visible text: $expected" >&2
    return 1
}

wait_for_ui_marker() {
    local marker=$1
    for _ in $(seq 1 240); do
        local runner_container
        runner_container=$(xcrun simctl get_app_container "$simulator_id" "$test_runner_bundle_id" data 2>/dev/null || true)
        if [[ -n "$runner_container" && -f "$runner_container/tmp/$marker" ]]; then return 0; fi
        if [[ -n "$ui_pid" ]] && ! kill -0 "$ui_pid" 2>/dev/null; then tail -n 100 "$xcode_log" >&2; return 1; fi
        sleep 1
    done
    tail -n 100 "$xcode_log" >&2
    echo "timed out waiting for iOS UI test marker $marker" >&2
    return 1
}

wait_for_patch_after() {
    local previous_count=$1
    for _ in $(seq 1 180); do
        local current_count
        current_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
        if (( current_count > previous_count )); then
            patch_count=$current_count
            return 0
        fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for the runtime to apply a FastList patch" >&2
    return 1
}

wait_for_log "Nexa $platform_label dev runtime connected."
wait_for_log "Nexa $platform_label dev runtime applied module"
if [[ "$platform" == "ios" ]]; then
    ios_project="$project/build/ios/FastListSmoke.xcodeproj"
    test_source="$project/build/ios/NexaHotReloadFastListUITests.swift"
    cp "$script_dir/ios-hot-reload-fast-list-ui.swift" "$test_source"
    test_runner_bundle_id=$(ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
        NexaHotReloadFastListUITests.swift "$test_target")
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
    wait_for_ui_marker nexa-fast-list-ready
else
    wait_for_visible_text "Item 15"
fi
patch_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("Item $index")'
if old not in source:
    raise SystemExit("FastList row label was not found")
path.write_text(source.replace(old, 'Text("Reloaded $index")', 1))
PY

wait_for_patch_after "$patch_count"
if [[ "$platform" == "ios" ]]; then
    wait_for_ui_marker nexa-fast-list-completed
    if ! wait "$ui_pid"; then ui_pid=; tail -n 100 "$xcode_log" >&2; exit 1; fi
    ui_pid=
    grep -Fq '** TEST SUCCEEDED **' "$xcode_log" || { tail -n 100 "$xcode_log" >&2; exit 1; }
else
    wait_for_visible_text "Reloaded 15"
fi
echo "Nexa $platform FastList rendering, scroll-position retention, and hot reload passed."
