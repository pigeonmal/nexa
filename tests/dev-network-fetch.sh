#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: dev-network-fetch.sh <nexa> <ios|android> <project-dir>" >&2
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

"$nexa" create NetworkFetchSmoke --directory "$project"
cp "$script_dir/fixtures/dev_network_fetch_initial.nx" "$project/App.nx"
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
for expected in \
    "Nexa $platform_label dev runtime connected." \
    "Nexa $platform_label dev runtime applied module"; do
    for _ in $(seq 1 180); do
        if grep -Fq "$expected" "$log_file"; then break; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
        sleep 1
    done
    grep -Fq "$expected" "$log_file" || { cat "$log_file" >&2; exit 1; }
done

patch_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
cp "$script_dir/fixtures/dev_network_fetch.nx" "$project/App.nx"
for _ in $(seq 1 180); do
    count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
    if (( count > patch_count )); then break; fi
    if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
    sleep 1
done
if (( count <= patch_count )); then
    cat "$log_file" >&2
    echo "timed out waiting for the async network-call hot reload" >&2
    exit 1
fi
printf 'R\n' >&3
for _ in $(seq 1 90); do
    if grep -Fq "Nexa app state restarted in the running app." "$log_file"; then break; fi
    if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
    sleep 1
done
grep -Fq "Nexa app state restarted in the running app." "$log_file" || {
    cat "$log_file" >&2
    echo "timed out waiting for the hot restart after async-call reload" >&2
    exit 1
}

if [[ "$platform" == android ]]; then
    xml="$project/build/network-fetch.xml"
    for _ in $(seq 1 90); do
        adb shell uiautomator dump /sdcard/nexa-network-fetch.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-network-fetch.xml >"$xml"
        if grep -Fq 'text="HTTP: 200"' "$xml"; then
            echo "Nexa Android dev Network.fetch passed."
            exit 0
        fi
        if grep -Fq 'text="HTTP: 599"' "$xml"; then
            cat "$xml" >&2
            cat "$log_file" >&2
            echo "Nexa Android dev Network.fetch entered its error handler" >&2
            exit 1
        fi
        sleep 1
    done
    cat "$xml" >&2
    cat "$log_file" >&2
    exit 1
fi

ios_project="$project/build/ios/NetworkFetchSmoke.xcodeproj"
test_source="$project/build/ios/NexaDevNetworkFetchUITests.swift"
xcode_log="$log_file.xcodebuild.log"
cp "$script_dir/ios-dev-network-fetch-ui.swift" "$test_source"
ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
    NexaDevNetworkFetchUITests.swift NexaDevNetworkFetchUITests >/dev/null
simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
if [[ -z "$simulator_id" ]]; then echo "no booted iOS Simulator was found" >&2; exit 1; fi
xcodebuild test -project "$ios_project" -scheme NexaDevNetworkFetchUITests \
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
echo "Nexa iOS dev Network.fetch passed."
