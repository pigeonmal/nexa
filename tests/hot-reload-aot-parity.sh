#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-aot-parity.sh <nexa> <ios|android> <project-dir>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
artifacts="$project/parity-captures"
dev_log="$project/dev.log"
dev_pid=
parity_phases=(initial toolbar button switch text-entry controls-scroll-3 controls-scroll-5 navigation style-layout style-typography style-button style-button-interaction style-input style-images style-pressable large-sheet lists lists-refresh lists-scroll-1 lists-scroll-3 list-parameters list-parameters-next workspace pages pages-next bottom-sheet dialog confirmation)

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create RenderParity --directory "$project"
cp "$script_dir/fixtures/render_parity.nx" "$project/App.nx"
mkdir -p "$project/assets/images"
cp "$script_dir/fixtures/parity_sample.png" "$project/assets/images/parity_sample.png"
mkdir -p "$artifacts/aot" "$artifacts/dev"
python3 - "$artifacts/aot" "$artifacts/dev" <<'PY'
from pathlib import Path
import sys

for directory in map(Path, sys.argv[1:]):
    for capture in (*directory.glob("*.json"), *directory.glob("*.png"), *directory.glob("*.xml")):
        capture.unlink()
PY

cleanup() {
    if [[ -n "$dev_pid" ]]; then
        kill -INT "$dev_pid" 2>/dev/null || true
        for _ in $(seq 1 10); do
            kill -0 "$dev_pid" 2>/dev/null || break
            sleep 1
        done
        kill -TERM "$dev_pid" 2>/dev/null || true
        wait "$dev_pid" 2>/dev/null || true
    fi
    if [[ -e "$project/dev-stdin" ]]; then
        exec 3>&- 2>/dev/null || true
    fi
}
trap cleanup EXIT

wait_for_dev_log() {
    local expected=$1
    for _ in $(seq 1 240); do
        if grep -Fq "$expected" "$dev_log"; then return 0; fi
        if [[ -n "$dev_pid" ]] && ! kill -0 "$dev_pid" 2>/dev/null; then
            cat "$dev_log" >&2
            return 1
        fi
        sleep 1
    done
    cat "$dev_log" >&2
    echo "timed out waiting for DevRuntime: $expected" >&2
    return 1
}

capture_ios() {
    local variant=$1
    local output="$project/build/parity-$variant"
    local ios_project="$output/ios/RenderParity.xcodeproj"
    local test_name=NexaRenderParityTests
    local test_source="$output/ios/NexaRenderParityTests.swift"
    local result_bundle="$artifacts/$variant.xcresult"
    local test_runner_bundle_id

    cp "$script_dir/ios-render-parity-ui.swift" "$test_source"
    test_runner_bundle_id=$(ruby "$script_dir/add-ios-ui-test-target.rb" \
        "$ios_project" NexaRenderParityTests.swift "$test_name")
    xcodebuild test -project "$ios_project" -scheme "$test_name" \
        -destination "platform=iOS Simulator,id=$simulator_id" \
        -derivedDataPath "$output/ios-simulator-derived" \
        -resultBundlePath "$result_bundle" CODE_SIGNING_ALLOWED=NO \
        >"$artifacts/$variant.xcodebuild.log" 2>&1 || {
            tail -n 100 "$artifacts/$variant.xcodebuild.log" >&2
            return 1
        }
    grep -Fq '** TEST SUCCEEDED **' "$artifacts/$variant.xcodebuild.log"
    local runner_container
    runner_container=$(xcrun simctl get_app_container "$simulator_id" "$test_runner_bundle_id" data)
    for phase in "${parity_phases[@]}"; do
        cp "$runner_container/tmp/nexa-render-parity-$phase.json" "$artifacts/$variant/$phase.json"
        cp "$runner_container/tmp/nexa-render-parity-$phase.png" "$artifacts/$variant/$phase.png"
    done
}

android_wait_for_text() {
    local expected=$1
    local xml="$project/build/android-current.xml"
    for _ in $(seq 1 60); do
        adb shell uiautomator dump /sdcard/nexa-render-parity.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-render-parity.xml >"$xml"
        if grep -Fq "text=\"$expected\"" "$xml" || grep -Fq "content-desc=\"$expected\"" "$xml"; then
            return 0
        fi
        sleep 0.25
    done
    cat "$xml" >&2
    echo "timed out waiting for Android text: $expected" >&2
    return 1
}

android_tap_label() {
    local label=$1
    local xml="$project/build/android-current.xml"
    adb shell uiautomator dump /sdcard/nexa-render-parity.xml >/dev/null 2>&1
    adb exec-out cat /sdcard/nexa-render-parity.xml >"$xml"
    local coordinates
    coordinates=$(python3 - "$xml" "$label" <<'PY'
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
label = sys.argv[2]
nodes = list(root.iter("node"))
node = next((item for item in nodes if item.attrib.get("content-desc") == label), None)
if node is None:
    node = next((item for item in nodes if item.attrib.get("text") == label), None)
if node is None:
    raise SystemExit(f"missing Android control: {label}")
values = [int(value) for value in node.attrib["bounds"].replace("][", ",").strip("[]").split(",")]
print((values[0] + values[2]) // 2, (values[1] + values[3]) // 2)
PY
    )
    read -r x y <<<"$coordinates"
    adb shell input tap "$x" "$y"
}

android_swipe() {
    local direction=$1
    local size
    size=$(adb shell wm size | sed -n 's/.*: \([0-9][0-9]*\)x\([0-9][0-9]*\).*/\1 \2/p' | tail -n 1)
    read -r width height <<<"$size"
    local x=$((width / 2))
    if [[ "$direction" == up ]]; then
        adb shell input swipe "$x" "$((height * 3 / 4))" "$x" "$((height / 4))" 220 >/dev/null
    else
        adb shell input swipe "$x" "$((height / 4))" "$x" "$((height * 3 / 4))" 220 >/dev/null
    fi
}

android_scroll_to_label() {
    local label=$1
    local direction=${2:-up}
    local xml="$project/build/android-current.xml"
    for _ in $(seq 1 12); do
        adb shell uiautomator dump /sdcard/nexa-render-parity.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-render-parity.xml >"$xml"
        if grep -Fq "text=\"$label\"" "$xml" || grep -Fq "content-desc=\"$label\"" "$xml"; then
            return 0
        fi
        android_swipe "$direction"
    done
    cat "$xml" >&2
    echo "could not scroll to Android control: $label" >&2
    return 1
}

capture_android_phase() {
    local variant=$1
    local phase=$2
    local xml="$artifacts/$variant/$phase.xml"
    adb shell uiautomator dump /sdcard/nexa-render-parity.xml >/dev/null 2>&1
    adb exec-out cat /sdcard/nexa-render-parity.xml >"$xml"
    adb exec-out screencap -p >"$artifacts/$variant/$phase.png"
    python3 "$script_dir/compare-render-parity.py" --android-xml "$xml" "$artifacts/$variant/$phase.json"
}

run_android_interactions() {
    local variant=$1
    android_wait_for_text "Parity heading"
    capture_android_phase "$variant" initial
    android_tap_label "Toolbar action"
    android_wait_for_text "Toolbar actions: 1"
    capture_android_phase "$variant" toolbar
    android_tap_label "Increment: 0"
    android_wait_for_text "Increment: 1"
    capture_android_phase "$variant" button
    android_tap_label "Enabled"
    android_wait_for_text "Enabled: true"
    capture_android_phase "$variant" switch
    android_tap_label "Open text input screen"
    android_tap_label "Task title"
    adb shell input text NexaParity
    adb shell input keyevent 4
    android_wait_for_text "Title: NexaParity"
    capture_android_phase "$variant" text-entry
    android_tap_label "Back to catalog"
    android_swipe up
    android_swipe up
    android_swipe up
    capture_android_phase "$variant" controls-scroll-3
    android_swipe up
    android_swipe up
    capture_android_phase "$variant" controls-scroll-5

    android_scroll_to_label "Open detail screen" down
    android_tap_label "Open detail screen"
    android_wait_for_text "Navigation destination"
    capture_android_phase "$variant" navigation
    android_tap_label "Back to catalog"

    android_tap_label "Lists"
    android_wait_for_text "Virtualized list"
    capture_android_phase "$variant" lists
    android_scroll_to_label "Refresh now" up
    android_tap_label "Refresh now"
    android_wait_for_text "Refresh count: 1"
    capture_android_phase "$variant" lists-refresh
    android_swipe up
    capture_android_phase "$variant" lists-scroll-1
    android_swipe up
    android_swipe up
    capture_android_phase "$variant" lists-scroll-3
    android_scroll_to_label "Open list parameters" down
    android_tap_label "Open list parameters"
    android_wait_for_text "Page snap list parameters"
    capture_android_phase "$variant" list-parameters
    android_swipe up
    android_wait_for_text "Page snap row 1"
    capture_android_phase "$variant" list-parameters-next
    android_tap_label "Back to lists"

    android_tap_label "Workspace"
    android_wait_for_text "Workspace"
    capture_android_phase "$variant" workspace

    android_tap_label "Pages"
    android_wait_for_text "Pager page one"
    capture_android_phase "$variant" pages
    android_tap_label "Next page"
    android_wait_for_text "Pager page two"
    capture_android_phase "$variant" pages-next

    android_tap_label "Controls"
    android_scroll_to_label "Open style parameters"
    android_tap_label "Open style parameters"
    android_wait_for_text "Column style parameters"
    capture_android_phase "$variant" style-layout
    android_scroll_to_label "Typography style parameters"
    capture_android_phase "$variant" style-typography
    android_scroll_to_label "Button style parameters"
    capture_android_phase "$variant" style-button
    android_tap_label "Enable configured button"
    android_tap_label "Configured button"
    android_wait_for_text "Configured state: loading=true, disabled=false"
    capture_android_phase "$variant" style-button-interaction
    android_tap_label "Open large sheet"
    android_wait_for_text "Large sheet content"
    capture_android_phase "$variant" large-sheet
    android_tap_label "Close large sheet"
    android_scroll_to_label "Text input parameters"
    capture_android_phase "$variant" style-input
    android_scroll_to_label "Image and icon parameters"
    capture_android_phase "$variant" style-images
    android_scroll_to_label "Pressable parameters"
    capture_android_phase "$variant" style-pressable
    android_scroll_to_label "Back to catalog" down
    android_tap_label "Back to catalog"

    android_tap_label "Controls"
    android_scroll_to_label "Open bottom sheet" up
    android_tap_label "Open bottom sheet"
    android_wait_for_text "Sheet content"
    capture_android_phase "$variant" bottom-sheet
    android_tap_label "Close sheet"
    android_tap_label "Open dialog"
    android_wait_for_text "Parity dialog"
    capture_android_phase "$variant" dialog
    android_tap_label "Dismiss dialog"
    android_tap_label "Open confirmation"
    android_wait_for_text "Confirm parity"
    capture_android_phase "$variant" confirmation
    android_tap_label "Confirm action"
}

cd "$project"
"$nexa" test "--$platform" --out build/parity-aot

if [[ "$platform" == ios ]]; then
    simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
    if [[ -z "$simulator_id" ]]; then echo "no booted iOS Simulator was found" >&2; exit 1; fi
    xcodebuild -project build/parity-aot/ios/RenderParity.xcodeproj \
        -scheme RenderParity -sdk iphonesimulator -configuration Debug \
        -derivedDataPath build/parity-aot/ios-simulator-derived \
        CODE_SIGNING_ALLOWED=NO build >"$artifacts/aot-simulator-build.log" 2>&1 || {
            tail -n 100 "$artifacts/aot-simulator-build.log" >&2
            exit 1
        }
    capture_ios aot
else
    local_apk="build/parity-aot/android/app/build/outputs/apk/debug/app-debug.apk"
    app_id=$(sed -n 's/.*applicationId = "\([^"]*\)".*/\1/p' \
        build/parity-aot/android/app/build.gradle.kts | head -n 1)
    if [[ ! -f "$local_apk" || -z "$app_id" ]]; then
        echo "Android AOT build did not produce the expected APK and application id" >&2
        exit 1
    fi
    adb install -r "$local_apk" >/dev/null
    adb shell pm clear "$app_id" >/dev/null
    adb shell am start -n "$app_id/.MainActivity" >/dev/null
    run_android_interactions aot
    # Clear Compose saveable state left by the AOT interaction pass so the
    # DevRuntime comparison starts from the same declared `.nx` state.
    adb shell pm clear "$app_id" >/dev/null
fi

if [[ "$platform" == ios ]]; then
    mkfifo "$project/dev-stdin"
    exec 3<>"$project/dev-stdin"
    "$nexa" dev --ios --out build/parity-dev <&3 >"$dev_log" 2>&1 &
    dev_pid=$!
    wait_for_dev_log "Nexa iOS dev runtime connected."
    wait_for_dev_log "Nexa iOS dev runtime applied module"
    capture_ios dev
else
    mkfifo "$project/dev-stdin"
    exec 3<>"$project/dev-stdin"
    "$nexa" dev --android --out build/parity-dev <&3 >"$dev_log" 2>&1 &
    dev_pid=$!
    wait_for_dev_log "Nexa Android dev runtime connected."
    wait_for_dev_log "Nexa Android dev runtime applied module"
    run_android_interactions dev
fi

python3 "$script_dir/compare-render-parity.py" "$artifacts/aot" "$artifacts/dev"
echo "Nexa $platform AOT and hot-reload accessibility and screenshot parity passed."
