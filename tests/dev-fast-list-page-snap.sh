#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: dev-fast-list-page-snap.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
log_file=$4
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_fast_list_page_snap.nx"
test_target=NexaDevFastListPageSnapUITests
xcode_log="$log_file.xcodebuild.log"
ui_pid=

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create PageSnapSmoke --directory "$project"
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

wait_for_log "Nexa $platform_label dev runtime applied module"
if [[ "$platform" == ios ]]; then
    ios_project="$project/build/ios/PageSnapSmoke.xcodeproj"
    test_source="$project/build/ios/NexaDevFastListPageSnapUITests.swift"
    cp "$script_dir/ios-dev-fast-list-page-snap-ui.swift" "$test_source"
    test_runner_bundle_id=$(ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
        NexaDevFastListPageSnapUITests.swift "$test_target")
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
    rg -F "NEXA_PAGE_SNAP iOS" "$xcode_log" || true
    echo "Nexa iOS DevRuntime FastList page snapping passed from XCTest logs."
    exit 0
fi

artifact_dir="$project/build/test-artifacts"
mkdir -p "$artifact_dir"
initial_xml="$artifact_dir/page-snap-initial.xml"
settled_xml="$artifact_dir/page-snap-settled.xml"

dump_accessibility_tree() {
    local destination=$1
    adb shell uiautomator dump /sdcard/nexa-page-snap.xml >/dev/null 2>&1
    adb exec-out cat /sdcard/nexa-page-snap.xml >"$destination"
}

for _ in $(seq 1 60); do
    dump_accessibility_tree "$initial_xml"
    if grep -Fq 'text="Current page 0"' "$initial_xml" \
        && grep -Fq 'text="Feed page 0"' "$initial_xml" \
        && grep -Fq 'text="Settled events ' "$initial_xml"; then
        break
    fi
    sleep 1
done
grep -Fq 'text="Current page 0"' "$initial_xml" || { cat "$initial_xml" >&2; exit 1; }

physical_size=$(adb shell wm size | sed -n 's/^Physical size: //p' | tail -n 1 | tr -d '\r')
width=${physical_size%x*}
height=${physical_size#*x}
x=$((width / 2))
start_y=$((height * 4 / 5))
end_y=$((height / 4))
adb shell input swipe "$x" "$start_y" "$x" "$end_y" 450

for _ in $(seq 1 60); do
    dump_accessibility_tree "$settled_xml"
    if grep -Fq 'text="Current page 1"' "$settled_xml" \
        && grep -Fq 'text="Feed page 1"' "$settled_xml"; then
        break
    fi
    sleep 0.5
done
python3 - "$initial_xml" "$settled_xml" <<'PY'
import re
import sys
import xml.etree.ElementTree as ET

def nodes(path):
    return list(ET.parse(path).getroot().iter("node"))

before, after = map(nodes, sys.argv[1:])

def node_text(items, text):
    return next(node for node in items if node.attrib.get("text") == text)

def top(node):
    match = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.attrib["bounds"])
    if match is None:
        raise SystemExit(f"unreadable accessibility bounds: {node.attrib.get('bounds')}")
    return int(match.group(2))

def status_count(items):
    label = next(node.attrib["text"] for node in items if node.attrib.get("text", "").startswith("Settled events "))
    return int(label.rsplit(" ", 1)[1])

before_list = next(node for node in before if node.attrib.get("scrollable") == "true")
after_list = next(node for node in after if node.attrib.get("scrollable") == "true")
before_offset = top(node_text(before, "Feed page 0")) - top(before_list)
after_offset = top(node_text(after, "Feed page 1")) - top(after_list)
if before_offset != after_offset:
    raise SystemExit(f"page row did not settle at the same viewport offset: {before_offset} -> {after_offset}")
initial_count = status_count(before)
settled_count = status_count(after)
if settled_count != initial_count + 1:
    raise SystemExit(f"one page swipe should report one settled page: {initial_count} -> {settled_count}")
print(f"NEXA_PAGE_SNAP Android page=1 rowViewportOffset={after_offset}px settledEvents={settled_count}")
PY
echo "Nexa Android DevRuntime FastList page snapping passed from accessibility and log output."
