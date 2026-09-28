#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-custom-components.sh <nexa> <ios|android> <project-dir>" >&2
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

platform_label=Android
if [[ "$platform" == ios ]]; then platform_label=iOS; fi

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

applied_count() {
    grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true
}

wait_for_patch_after() {
    local previous_count=$1
    local description=$2
    for _ in $(seq 1 180); do
        if (( $(applied_count) > previous_count )); then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 0.25
    done
    cat "$log_file" >&2
    echo "timed out waiting for $description patch" >&2
    return 1
}

wait_for_test_marker() {
    local marker=$1
    local xcode_log=$2
    for _ in $(seq 1 480); do
        if grep -Fxq "$marker" "$xcode_log"; then return 0; fi
        if ! kill -0 "$ui_pid" 2>/dev/null; then tail -n 100 "$xcode_log" >&2; return 1; fi
        sleep 0.25
    done
    tail -n 100 "$xcode_log" >&2
    echo "timed out waiting for iOS test checkpoint: $marker" >&2
    return 1
}

assert_android_text() {
    local expected=$1
    local should_exist=$2
    local dump="$project/current-ui.xml"
    for _ in $(seq 1 30); do
        adb shell uiautomator dump /sdcard/nexa-custom-component.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-custom-component.xml >"$dump"
        if [[ "$should_exist" == true ]] && grep -Fq "text=\"$expected\"" "$dump"; then return 0; fi
        if [[ "$should_exist" == false ]] && ! grep -Fq "text=\"$expected\"" "$dump"; then return 0; fi
        sleep 0.25
    done
    cat "$dump" >&2
    echo "Android UI did not satisfy text assertion: $expected (expected present=$should_exist)" >&2
    return 1
}

wait_for_log "Nexa $platform_label dev runtime connected."
wait_for_log "Nexa $platform_label dev runtime applied module"

if [[ "$platform" == ios ]]; then
    ios_project="$project/build/ios/ComponentSmoke.xcodeproj"
    test_target=NexaDevCustomComponentHotReloadUITests
    test_source="$project/build/ios/${test_target}.swift"
    xcode_log="$log_file.xcodebuild.log"
    cp "$script_dir/ios-dev-custom-component-hot-reload.swift" "$test_source"
    ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
        "${test_target}.swift" "$test_target" >/dev/null
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
    wait_for_test_marker "NEXA_COMPONENT_STAGE_INITIAL" "$xcode_log"
fi

previous_count=$(applied_count)
cp "$script_dir/fixtures/dev_component_added.nx" "$project/App.nx"
wait_for_patch_after "$previous_count" "custom component addition"
if [[ "$platform" == android ]]; then
    assert_android_text "Card: Added" true
    assert_android_text "Projected child" true
else
    wait_for_test_marker "NEXA_COMPONENT_STAGE_ADDED" "$xcode_log"
fi

previous_count=$(applied_count)
cp "$script_dir/fixtures/dev_component_renamed.nx" "$project/App.nx"
wait_for_patch_after "$previous_count" "custom component rename"
if [[ "$platform" == android ]]; then
    assert_android_text "Product: Renamed" true
    assert_android_text "Card: Added" false
    assert_android_text "Projected child" true
else
    wait_for_test_marker "NEXA_COMPONENT_STAGE_RENAMED" "$xcode_log"
fi

previous_count=$(applied_count)
cp "$script_dir/fixtures/dev_component_removed.nx" "$project/App.nx"
wait_for_patch_after "$previous_count" "custom component removal"
if [[ "$platform" == android ]]; then
    assert_android_text "After component removal" true
    assert_android_text "Product: Renamed" false
    assert_android_text "Projected child" false
else
    wait_for_test_marker "NEXA_COMPONENT_STAGE_REMOVED" "$xcode_log"
    if ! wait "$ui_pid"; then
        ui_pid=
        tail -n 100 "$xcode_log" >&2
        cat "$log_file" >&2
        exit 1
    fi
    ui_pid=
    grep -Fq '** TEST SUCCEEDED **' "$xcode_log" || { tail -n 100 "$xcode_log" >&2; exit 1; }
fi

echo "Nexa $platform_label custom component add/rename/remove hot reload passed."
