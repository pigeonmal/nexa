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
app_id=dev.nexa.renderparity
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
artifacts="$project/parity-captures"
dev_log="$project/dev.log"
dev_pid=
skip_aot=${NEXA_PARITY_SKIP_AOT:-0}
reuse_project=${NEXA_PARITY_REUSE_PROJECT:-0}
modal_only=${NEXA_PARITY_ANDROID_MODAL_ONLY:-0}
navigation_only=${NEXA_PARITY_NAVIGATION_ONLY:-0}
parity_phases=(initial toolbar button switch text-entry controls-scroll-3 controls-scroll-5 navigation navigation-nested navigation-pop navigation-root-restored style-layout style-typography style-button style-button-interaction style-input style-images style-pressable pressable-long large-sheet lists lists-refresh lists-scroll-1 lists-scroll-3 list-parameters list-parameters-next workspace pages pages-next bottom-sheet dialog confirmation)
if [[ "$navigation_only" == 1 ]]; then
    parity_phases=(navigation navigation-nested navigation-pop navigation-root-restored)
elif [[ "$navigation_only" != 0 ]]; then
    echo "NEXA_PARITY_NAVIGATION_ONLY must be 0 or 1" >&2
    exit 2
fi

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi
if [[ "$modal_only" == 1 && ( "$platform" != android || "$skip_aot" != 1 ) ]]; then
    echo "NEXA_PARITY_ANDROID_MODAL_ONLY=1 requires Android and NEXA_PARITY_SKIP_AOT=1" >&2
    exit 2
fi

if [[ "$skip_aot" != 1 && "$reuse_project" != 1 ]]; then
    "$nexa" create RenderParity --directory "$project"
    # The fixture exercises Android Material 3 Expressive menus, whose Compose
    # implementation requires API 24. Keep this test project's floor compatible
    # with that host requirement if the scaffold default changes later.
    python3 - "$project/nexa.config.nx" <<'PY'
from pathlib import Path
import re
import sys

config = Path(sys.argv[1])
source = config.read_text()
match = re.search(r"(\bandroid\s*\{\s*minSdk:\s*)(\d+)\b", source)
if match is None:
    raise SystemExit(f"could not set the render parity fixture minSdk in {config}")
if int(match.group(2)) < 24:
    config.write_text(source[:match.start(2)] + "24" + source[match.end(2):])
PY
    cp "$script_dir/fixtures/render_parity.nx" "$project/App.nx"
    mkdir -p "$project/assets/images"
    cp "$script_dir/fixtures/parity_sample.png" "$project/assets/images/parity_sample.png"
fi
if [[ "$reuse_project" == 1 && ( ! -f "$project/App.nx" || ! -f "$project/nexa.config.nx" ) ]]; then
    echo "NEXA_PARITY_REUSE_PROJECT=1 requires a prepared parity project: $project" >&2
    exit 2
fi
if [[ "$skip_aot" == 1 && ( ! -f "$project/App.nx" || ! -d "$project/build/parity-aot" ) ]]; then
    echo "NEXA_PARITY_SKIP_AOT=1 requires an existing parity project with AOT output: $project" >&2
    exit 2
fi
mkdir -p "$artifacts/aot" "$artifacts/dev"
if [[ "$skip_aot" == 1 ]]; then
    python3 - "$artifacts/dev" <<'PY'
from pathlib import Path
import sys

directory = Path(sys.argv[1])
for capture in (*directory.glob("*.json"), *directory.glob("*.png"), *directory.glob("*.xml")):
    capture.unlink()
PY
else
    python3 - "$artifacts/aot" "$artifacts/dev" <<'PY'
from pathlib import Path
import sys

for directory in map(Path, sys.argv[1:]):
    for capture in (*directory.glob("*.json"), *directory.glob("*.png"), *directory.glob("*.xml")):
        capture.unlink()
PY
fi

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

android_dump_hierarchy() {
    local xml=$1
    local attempt
    for attempt in $(seq 1 4); do
        # uiautomator can be killed by the API 31 emulator (exit 137). Remove
        # the previous dump first so callers can never mistake an old screen
        # for a fresh tree after that failure.
        if adb shell 'rm -f /sdcard/nexa-render-parity.xml && uiautomator dump /sdcard/nexa-render-parity.xml >/dev/null 2>&1 && cat /sdcard/nexa-render-parity.xml' >"$xml" 2>/dev/null \
            && [[ -s "$xml" ]] \
            && grep -q '<hierarchy' "$xml"; then
            return 0
        fi
        sleep 0.1
    done
    echo "could not obtain a fresh Android accessibility hierarchy" >&2
    return 1
}

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
    local -a test_filter=()

    if [[ "$navigation_only" == 1 ]]; then
        test_filter=(-only-testing:NexaRenderParityTests/NexaRenderParityTests/testNavigationScreenTransitions)
    fi

    cp "$script_dir/ios-render-parity-ui.swift" "$test_source"
    test_runner_bundle_id=$(ruby "$script_dir/add-ios-ui-test-target.rb" \
        "$ios_project" NexaRenderParityTests.swift "$test_name")
    xcodebuild test -project "$ios_project" -scheme "$test_name" \
        -destination "platform=iOS Simulator,id=$simulator_id" \
        -derivedDataPath "$output/ios-simulator-derived" \
        -resultBundlePath "$result_bundle" "${test_filter[@]}" CODE_SIGNING_ALLOWED=NO \
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
        if android_dump_hierarchy "$xml" \
            && { grep -Fq "text=\"$expected\"" "$xml" || grep -Fq "content-desc=\"$expected\"" "$xml"; }; then
            return 0
        fi
        sleep 0.25
    done
    cat "$xml" >&2
    echo "timed out waiting for Android text: $expected" >&2
    return 1
}

android_label_center() {
    local label=$1
    local xml="$project/build/android-current.xml"
    android_dump_hierarchy "$xml"
    local coordinates
    coordinates=$(python3 - "$xml" "$label" <<'PY'
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
label = sys.argv[2]
matches = []

def visit(node, ancestors=()):
    if node.tag == "node":
        if node.attrib.get("content-desc") == label or node.attrib.get("text") == label:
            matches.append((node, ancestors))
    for child in node:
        visit(child, (*ancestors, node) if node.tag == "node" else ancestors)

visit(root)
if not matches:
    raise SystemExit(f"missing Android control: {label}")
node, ancestors = matches[0]
for candidate in (node, *reversed(ancestors)):
    if candidate.attrib.get("clickable") == "true":
        node = candidate
        break
values = [int(value) for value in node.attrib["bounds"].replace("][", ",").strip("[]").split(",")]
print((values[0] + values[2]) // 2, (values[1] + values[3]) // 2)
PY
    ) || return 1
    printf '%s\n' "$coordinates"
}

android_tap_label() {
    local coordinates attempt
    for attempt in 1 2 3; do
        if coordinates=$(android_label_center "$1" 2>/dev/null); then break; fi
        sleep 0.25
    done
    if [[ -z "$coordinates" ]]; then
        echo "missing Android control after waiting for layout to settle: $1" >&2
        return 1
    fi
    read -r x y <<<"$coordinates"
    adb shell input tap "$x" "$y"
}

android_long_press_label() {
    local coordinates
    coordinates=$(android_label_center "$1")
    read -r x y <<<"$coordinates"
    adb shell input swipe "$x" "$y" "$x" "$y" 800
}

android_swipe() {
    local direction=$1
    local size
    size=$(adb shell wm size | sed -n 's/.*: \([0-9][0-9]*\)x\([0-9][0-9]*\).*/\1 \2/p' | tail -n 1)
    read -r width height <<<"$size"
    # Keep vertical gestures in the empty right gutter. Center swipes can land
    # on scrollable sample controls in DevRuntime and be consumed by the child.
    local x=$((width * 94 / 100))
    if [[ "$direction" == up ]]; then
        adb shell input swipe "$x" "$((height * 3 / 4))" "$x" "$((height / 4))" 220 >/dev/null
    else
        adb shell input swipe "$x" "$((height / 4))" "$x" "$((height * 3 / 4))" 220 >/dev/null
    fi
}

android_nudge_up() {
    local size
    size=$(adb shell wm size | sed -n 's/.*: \([0-9][0-9]*\)x\([0-9][0-9]*\).*/\1 \2/p' | tail -n 1)
    read -r width height <<<"$size"
    # Avoid sample buttons and sliders that can consume a center-screen swipe.
    local x=$((width * 94 / 100))
    adb shell input swipe "$x" "$((height * 3 / 4))" "$x" "$((height / 2))" 220 >/dev/null
}

android_scroll_nudge() {
    local direction=$1
    local size
    size=$(adb shell wm size | sed -n 's/.*: \([0-9][0-9]*\)x\([0-9][0-9]*\).*/\1 \2/p' | tail -n 1)
    read -r width height <<<"$size"
    # The right gutter avoids controls that intercept vertical drags.
    local x=$((width * 94 / 100))
    local start=$((height * 7 / 10))
    local end=$((height * 5 / 10))
    if [[ "$direction" == down ]]; then
        start=$((height * 4 / 10))
        end=$((height * 5 / 10))
    fi
    adb shell input swipe "$x" "$start" "$x" "$end" 700 >/dev/null
    sleep 0.2
}

android_scroll_to_label() {
    local label=$1
    local direction=${2:-up}
    local xml="$project/build/android-current.xml"
    for _ in $(seq 1 24); do
        android_dump_hierarchy "$xml"
        if grep -Fq "text=\"$label\"" "$xml" || grep -Fq "content-desc=\"$label\"" "$xml"; then
            return 0
        fi
        if [[ "$direction" == down ]]; then
            android_swipe down
            sleep 0.35
        else
            android_scroll_nudge up
        fi
    done
    cat "$xml" >&2
    echo "could not scroll to Android control: $label" >&2
    return 1
}

android_align_label_y() {
    local label=$1
    local target_y=$2
    for _ in $(seq 1 10); do
        local coordinates
        coordinates=$(android_label_center "$label") || return 1
        local x current_y
        read -r x current_y <<<"$coordinates"
        local delta=$((target_y - current_y))
        if (( delta >= -2 && delta <= 2 )); then return 0; fi

        local dimensions width height
        dimensions=$(adb shell wm size | sed -n 's/.*: \([0-9][0-9]*\)x\([0-9][0-9]*\).*/\1 \2/p' | tail -n 1)
        read -r width height <<<"$dimensions"
        # Stay in the empty right gutter. Centering the swipe can land on a
        # Button/Switch in the fixture, which consumes the drag instead of
        # scrolling its parent.
        x=$((width * 94 / 100))
        local start_y=$((height / 2))
        local distance=$delta
        if (( distance > 300 )); then distance=300; fi
        if (( distance < -300 )); then distance=-300; fi
        local end_y=$((start_y + distance))
        adb shell input swipe "$x" "$start_y" "$x" "$end_y" 700 >/dev/null
        sleep 0.25
    done

    local coordinates x current_y
    coordinates=$(android_label_center "$label") || return 1
    read -r x current_y <<<"$coordinates"
    # Android's touch slop can leave the anchor a few pixels from its requested
    # position. The screenshot comparator verifies that any AOT/Dev difference
    # is one uniform content offset and rejects changes to row geometry.
    if (( current_y < target_y - 24 || current_y > target_y + 24 )); then
        echo "could not align Android control '$label' at y=$target_y (ended at y=$current_y)" >&2
        return 1
    fi
}

android_align_label_fraction() {
    local label=$1
    local height
    height=$(adb shell wm size | sed -n 's/.*: [0-9][0-9]*x\([0-9][0-9]*\).*/\1/p' | tail -n 1)
    if [[ -z "$height" ]]; then
        echo "could not read Android screen height" >&2
        return 1
    fi
    android_align_label_y "$label" "$((height * $2 / 100))"
}

capture_android_phase() {
    local variant=$1
    local phase=$2
    local xml="$artifacts/$variant/$phase.xml"
    android_dump_hierarchy "$xml"
    adb exec-out screencap -p >"$artifacts/$variant/$phase.png"
    python3 "$script_dir/compare-render-parity.py" --android-xml "$xml" "$artifacts/$variant/$phase.json"
}

run_android_interactions() {
    local variant=$1
    android_wait_for_text "Parity heading"
    if [[ "$navigation_only" == 1 ]]; then
        android_scroll_to_label "Open detail screen" down
        android_tap_label "Open detail screen"
        android_wait_for_text "Navigation destination"
        capture_android_phase "$variant" navigation
        android_tap_label "Open nested navigation screen"
        android_wait_for_text "Nested navigation destination"
        capture_android_phase "$variant" navigation-nested
        android_tap_label "Back to previous screen"
        android_wait_for_text "Navigation destination"
        capture_android_phase "$variant" navigation-pop
        android_tap_label "Back to previous screen"
        android_wait_for_text "Parity heading"
        capture_android_phase "$variant" navigation-root-restored
        return
    fi
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
    android_scroll_to_label "Open text input screen" down
    android_tap_label "Open text input screen"
    android_tap_label "Task title"
    adb shell input text NexaParity
    adb shell input keyevent 4
    android_wait_for_text "Title: NexaParity"
    capture_android_phase "$variant" text-entry
    android_tap_label "Back to previous screen"
    android_swipe up
    android_swipe up
    android_swipe up
    android_scroll_to_label "Open style parameters" down
    android_align_label_y "Open style parameters" 400
    capture_android_phase "$variant" controls-scroll-3
    android_swipe up
    android_swipe up
    android_scroll_to_label "Open text input screen" down
    android_align_label_y "Open text input screen" 400
    capture_android_phase "$variant" controls-scroll-5

    android_scroll_to_label "Open detail screen" down
    android_tap_label "Open detail screen"
    android_wait_for_text "Navigation destination"
    capture_android_phase "$variant" navigation
    android_tap_label "Open nested navigation screen"
    android_wait_for_text "Nested navigation destination"
    capture_android_phase "$variant" navigation-nested
    android_tap_label "Back to previous screen"
    android_wait_for_text "Navigation destination"
    capture_android_phase "$variant" navigation-pop
    android_tap_label "Back to previous screen"
    android_wait_for_text "Parity heading"
    capture_android_phase "$variant" navigation-root-restored

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
    # The destination text enters accessibility as soon as the Compose
    # navigation transition starts. Let its slide/fade finish before taking a
    # screenshot or tapping the back row.
    sleep 0.8
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
    android_align_label_y "Typography style parameters" 400
    capture_android_phase "$variant" style-typography
    android_scroll_to_label "Button style parameters"
    android_align_label_y "Button style parameters" 400
    capture_android_phase "$variant" style-button
    android_tap_label "Enable configured button"
    android_tap_label "Configured button"
    android_wait_for_text "Configured state: loading=true, disabled=false"
    android_align_label_y "Button style parameters" 400
    capture_android_phase "$variant" style-button-interaction
    android_tap_label "Open large sheet"
    android_wait_for_text "Large sheet content"
    capture_android_phase "$variant" large-sheet
    android_tap_label "Close large sheet"
    android_scroll_to_label "Button style parameters" up
    android_tap_label "Stop loading indicator"
    android_wait_for_text "Configured state: loading=false, disabled=false"
    android_scroll_to_label "Text input parameters"
    android_align_label_y "Text input parameters" 400
    capture_android_phase "$variant" style-input
    android_scroll_to_label "Image and icon parameters"
    android_align_label_y "Image and icon parameters" 400
    capture_android_phase "$variant" style-images
    android_scroll_to_label "Pressable parameters"
    # This scroll container bottoms out with the heading around y=1076 on the
    # emulator's unchanged display profile; center-alignment is unreachable.
    android_align_label_y "Pressable parameters" 1076
    capture_android_phase "$variant" style-pressable
    android_scroll_to_label "Long press parity target"
    android_long_press_label "Long press parity target"
    android_wait_for_text "Pressable result: tap=0, long=1"
    capture_android_phase "$variant" pressable-long
    android_scroll_to_label "Back to catalog" down
    android_tap_label "Back to catalog"

    android_tap_label "Controls"
    android_scroll_to_label "Open bottom sheet" up
    # Keep the shared content behind the native sheet at the same scroll
    # anchor for AOT and DevRuntime captures.
    android_align_label_fraction "Open bottom sheet" 83
    capture_android_phase "$variant" bottom-sheet-underlay
    android_tap_label "Open bottom sheet"
    android_wait_for_text "Sheet content"
    capture_android_phase "$variant" bottom-sheet
    android_tap_label "Close sheet"
    android_scroll_to_label "Open dialog" up
    # Keep capture anchors above the NavigationSuite bottom bar. Higher values
    # put the target under native navigation chrome and make it unhittable.
    android_align_label_fraction "Open dialog" 80
    capture_android_phase "$variant" dialog-underlay
    android_tap_label "Open dialog"
    android_wait_for_text "Parity dialog"
    capture_android_phase "$variant" dialog
    android_tap_label "Dismiss dialog"
    android_scroll_to_label "Open confirmation" up
    android_align_label_fraction "Open confirmation" 83
    capture_android_phase "$variant" confirmation-underlay
    android_tap_label "Open confirmation"
    android_wait_for_text "Confirm parity"
    capture_android_phase "$variant" confirmation
    android_tap_label "Confirm action"
}

run_android_modal_interactions() {
    local variant=$1
    android_tap_label "Controls"
    android_scroll_to_label "Open bottom sheet" up
    android_align_label_fraction "Open bottom sheet" 83
    capture_android_phase "$variant" bottom-sheet-underlay
    android_tap_label "Open bottom sheet"
    android_wait_for_text "Sheet content"
    capture_android_phase "$variant" bottom-sheet
    android_tap_label "Close sheet"
    android_scroll_to_label "Open dialog" up
    android_align_label_fraction "Open dialog" 80
    capture_android_phase "$variant" dialog-underlay
    android_tap_label "Open dialog"
    android_wait_for_text "Parity dialog"
    capture_android_phase "$variant" dialog
    android_tap_label "Dismiss dialog"
    android_scroll_to_label "Open confirmation" up
    android_align_label_fraction "Open confirmation" 83
    capture_android_phase "$variant" confirmation-underlay
    android_tap_label "Open confirmation"
    android_wait_for_text "Confirm parity"
    capture_android_phase "$variant" confirmation
    android_tap_label "Confirm action"
}

cd "$project"
if [[ "$platform" == ios ]]; then
    simulator_id=$(xcrun simctl list devices booted -j | python3 -c 'import json,sys; d=json.load(sys.stdin)["devices"]; print(next((x["udid"] for g in d.values() for x in g if x["state"] == "Booted"), ""))')
    if [[ -z "$simulator_id" ]]; then echo "no booted iOS Simulator was found" >&2; exit 1; fi
fi
if [[ "$modal_only" == 1 ]]; then
    app_id=$(sed -n 's/.*applicationId = "\([^"]*\)".*/\1/p' \
        build/parity-aot/android/app/build.gradle.kts | head -n 1)
    if [[ -z "$app_id" ]]; then
        echo "Android AOT build does not include an application id" >&2
        exit 1
    fi
    adb shell am start -n "$app_id/.MainActivity" >/dev/null
    run_android_modal_interactions aot
    exit 0
fi
if [[ "$skip_aot" != 1 ]]; then
    "$nexa" test "--$platform" --out build/parity-aot
fi

if [[ "$skip_aot" == 1 ]]; then
    if [[ "$platform" == android ]]; then
        app_id=$(sed -n 's/.*applicationId = "\([^"]*\)".*/\1/p' \
            build/parity-aot/android/app/build.gradle.kts | head -n 1)
        if [[ -z "$app_id" ]]; then
            echo "Android AOT build does not include an application id" >&2
            exit 1
        fi
        adb shell pm clear "$app_id" >/dev/null
    fi
elif [[ "$platform" == ios ]]; then
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
    rm -f "$project/dev-stdin"
    mkfifo "$project/dev-stdin"
    exec 3<>"$project/dev-stdin"
    "$nexa" dev --ios --out build/parity-dev <&3 >"$dev_log" 2>&1 &
    dev_pid=$!
    wait_for_dev_log "Nexa iOS dev runtime connected."
    wait_for_dev_log "Nexa iOS dev runtime applied module"
    capture_ios dev
else
    rm -f "$project/dev-stdin"
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
