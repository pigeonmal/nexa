#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: deep-link-smoke.sh <nexa> <ios|android> <project-dir>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create DeepLinkDemo --directory "$project"
cp "$script_dir/../crates/nexa-cli/tests/fixtures/deep_links.nx" "$project/App.nx"
cat >"$project/nexa.config.nx" <<'NX'
config {
    app { displayName: "Deep Links", deepLinks: ["nexa://", "https://links.example.com"] }
}
NX
cd "$project"
"$nexa" dev "--$platform" --once

if [[ "$platform" == android ]]; then
    package=com.nexa.deeplinkdemo
    adb shell am start -a android.intent.action.VIEW \
        -d 'nexa://product-details/widget-17/5' \
        -n "$package/.MainActivity" >/dev/null
    xml="$project/build/deep-link-window.xml"
    for _ in $(seq 1 30); do
        adb shell uiautomator dump /sdcard/nexa-deep-link-window.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-deep-link-window.xml >"$xml"
        if grep -Fq 'text="Product widget-17, page 5"' "$xml"; then
            echo "Nexa Android custom deep link and typed route passed."
            exit 0
        fi
        sleep 1
    done
    cat "$xml" >&2
    exit 1
fi

ios_project="$project/build/ios/DeepLinkDemo.xcodeproj"
cp "$script_dir/ios-deep-link-ui.swift" \
    "$project/build/ios/NexaDeepLinkUITests.swift"
ruby "$script_dir/add-ios-ui-test-target.rb" "$ios_project" \
    NexaDeepLinkUITests.swift NexaDeepLinkUITests >/dev/null
simulator_id=$(xcrun simctl list devices booted -j | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next((device["udid"] for group in devices.values() for device in group if device["state"] == "Booted"), ""))
')
if [[ -z "$simulator_id" ]]; then
    echo "no booted iOS Simulator was found" >&2
    exit 1
fi
xcodebuild test -project "$ios_project" -scheme NexaDeepLinkUITests \
    -destination "platform=iOS Simulator,id=$simulator_id" \
    IPHONEOS_DEPLOYMENT_TARGET=16.4 CODE_SIGNING_ALLOWED=NO \
    >"$project/build/deep-link-xcodebuild.log" 2>&1 || {
        tail -n 100 "$project/build/deep-link-xcodebuild.log" >&2
        exit 1
    }
grep -Fq '** TEST SUCCEEDED **' "$project/build/deep-link-xcodebuild.log"
echo "Nexa iOS custom deep link and typed route passed."
