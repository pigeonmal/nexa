#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-interaction-ios.sh <nexa> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
project=$2
log_file=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_ios_interaction_reload.nx"
xcode_log="$log_file.xcodebuild.log"
app_name=IosInteractionSmoke
test_target=NexaHotReloadInteractionUITests
ios_project="$project/build/ios/$app_name.xcodeproj"
test_source="$project/build/ios/NexaHotReloadInteractionUITests.swift"
ready_token=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
patched_token=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
verified_token=$(python3 -c 'import secrets; print(secrets.token_hex(24))')
artifacts="$project/build/test-artifacts"

"$nexa" create "$app_name" --directory "$project"
cp "$fixture" "$project/App.nx"
mkdir -p "$project/build/ios" "$artifacts"
swiftc "$script_dir/read-screenshot-layout.swift" -framework Vision -framework ImageIO \
    -o "$project/read-screenshot-layout"
sed -e "s/__READY_TOKEN__/$ready_token/" \
    -e "s/__PATCHED_TOKEN__/$patched_token/" \
    -e "s/__VERIFIED_TOKEN__/$verified_token/" \
    "$script_dir/ios-hot-reload-interaction-ui.swift" >"$test_source"
mkfifo "$project/dev-stdin"
exec 3<>"$project/dev-stdin"
cd "$project"
"$nexa" dev --ios <&3 >"$log_file" 2>&1 &
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
    local pattern=$1
    for _ in $(seq 1 240); do
        if grep -Fq "$pattern" "$log_file"; then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for: $pattern" >&2
    return 1
}

wait_for_ui_file() {
    local file_name=$1
    local expected_token=$2
    for _ in $(seq 1 240); do
        local runner_container
        runner_container=$(xcrun simctl get_app_container "$simulator_id" "$test_runner_bundle_id" data 2>/dev/null || true)
        if [[ -n "$runner_container" && -f "$runner_container/tmp/$file_name" ]] \
            && [[ "$(cat "$runner_container/tmp/$file_name")" == "$expected_token" ]]; then
            return 0
        fi
        if ! kill -0 "$ui_pid" 2>/dev/null; then tail -n 120 "$xcode_log" >&2; return 1; fi
        sleep 1
    done
    tail -n 120 "$xcode_log" >&2
    echo "timed out waiting for the iOS UI test readiness file" >&2
    return 1
}

capture_screenshot_text() {
    xcrun simctl io "$simulator_id" screenshot "$artifacts/screen.png" >/dev/null 2>&1
    "$project/read-screenshot-layout" "$artifacts/screen.png" >"$artifacts/screen.tsv"
}

assert_direction() {
    local expected=$1
    local coordinates
    for _ in $(seq 1 15); do
        capture_screenshot_text
        coordinates=$(python3 - "$artifacts/screen.tsv" "$expected" <<'PY' 2>/dev/null || true
import sys

path, expected = sys.argv[1:]
positions = {}
for line in open(path, encoding="utf-8"):
    try:
        x, text = line.rstrip("\n").split("\t", 1)
        normalized = "".join(character for character in text.lower() if character.isalpha())
    except ValueError:
        continue
    for name in ("rowstart", "rowend"):
        if name in normalized and name not in positions:
            positions[name] = float(x)
if set(positions) == {"rowstart", "rowend"}:
    start, end = positions["rowstart"], positions["rowend"]
    if (expected == "rtl" and start > end) or (expected == "ltr" and start < end):
        print(f"Row Start x={start:.3f}; Row End x={end:.3f}")
        raise SystemExit(0)
raise SystemExit(1)
PY
)
        if [[ -n "$coordinates" ]]; then
            echo "$coordinates"
            return 0
        fi
        sleep 0.5
    done
    cat "$artifacts/screen.tsv" >&2
    echo "iOS screenshot did not show the expected $expected row order" >&2
    return 1
}

assert_status_bar_visible() {
    capture_screenshot_text
    if ! python3 - "$artifacts/screen.tsv" <<'PY'
import re
import sys

if any(re.fullmatch(r"\d{1,2}:\d{2}", line.rstrip("\n").split("\t", 1)[-1]) for line in open(sys.argv[1], encoding="utf-8")):
    raise SystemExit(0)
raise SystemExit(1)
PY
    then
        cat "$artifacts/screen.tsv" >&2
        echo "iOS status bar clock was not visible before the patch" >&2
        return 1
    fi
}

assert_status_bar_hidden() {
    for _ in $(seq 1 15); do
        capture_screenshot_text
        if ! python3 - "$artifacts/screen.tsv" <<'PY'
import re
import sys

if not any(re.fullmatch(r"\d{1,2}:\d{2}", line.rstrip("\n").split("\t", 1)[-1]) for line in open(sys.argv[1], encoding="utf-8")):
    raise SystemExit(0)
raise SystemExit(1)
PY
        then
            return 0
        fi
        sleep 0.5
    done
    cat "$artifacts/screen.tsv" >&2
    echo "iOS status bar remained visible after the hot reload" >&2
    return 1
}

wait_for_log "Nexa iOS dev runtime connected."
wait_for_log "Nexa iOS dev runtime applied module"
test_runner_bundle_id=$(ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
    NexaHotReloadInteractionUITests.swift "$test_target")
simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
if [[ -z "$simulator_id" ]]; then
    echo "no booted iOS Simulator was found" >&2
    exit 1
fi
xcodebuild test -project "$ios_project" -scheme "$test_target" \
    -destination "platform=iOS Simulator,id=$simulator_id" CODE_SIGNING_ALLOWED=NO \
    >"$xcode_log" 2>&1 &
ui_pid=$!
wait_for_ui_file "nexa-hot-reload-ready" "$ready_token"
assert_status_bar_visible
assert_direction rtl

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
replacements = {
    'count = count + 1': 'count = count + 10',
    'taps = taps + 1': 'taps = taps + 10',
    'Text("Details V1")': 'Text("Details V2")',
    'Text("Stack Overlay V1")': 'Text("Stack Overlay V2")',
    'label: "Accessible Action V1"': 'label: "Accessible Action V2"',
    'label: "Accessible Link V1"': 'label: "Accessible Link V2"',
    'label: "Accessible Header V1"': 'label: "Accessible Header V2"',
    'label: "Accessible Image V1"': 'label: "Accessible Image V2"',
    'label: "Accessible Plain V1"': 'label: "Accessible Plain V2"',
    'Direction(value: RTL)': 'Direction(value: LTR)',
    'StatusBar(style: Light, hidden: false, background: "#223344")': 'StatusBar(style: Dark, hidden: true, background: "#445566")',
}
for old, new in replacements.items():
    if old not in source:
        raise SystemExit(f"hot-reload fixture does not contain {old}")
    source = source.replace(old, new, 1)
path.write_text(source)
PY

wait_for_log "Nexa iOS dev runtime applied patch"
wait_for_ui_file "nexa-hot-reload-patched" "$patched_token"
assert_direction ltr
assert_status_bar_hidden
runner_container=$(xcrun simctl get_app_container "$simulator_id" "$test_runner_bundle_id" data)
printf '%s' "$verified_token" >"$runner_container/tmp/nexa-hot-reload-verified"
if ! wait "$ui_pid"; then
    ui_pid=
    tail -n 120 "$xcode_log" >&2
    exit 1
fi
ui_pid=
grep -Fq '** TEST SUCCEEDED **' "$xcode_log" || {
    tail -n 120 "$xcode_log" >&2
    echo "iOS hot-reload UI test did not report success" >&2
    exit 1
}
echo "Nexa iOS action, navigation, and direction hot reload passed."
