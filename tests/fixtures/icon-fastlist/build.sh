#!/usr/bin/env bash
set -euo pipefail

if [[ $# != 4 ]]; then
    echo "usage: $0 <nexa> <benchmark-directory> <variant> <ios-simulator-udid>" >&2
    exit 2
fi

fixture="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd -- "$fixture/../../.." && pwd)"
nexa="$(cd -- "$(dirname -- "$1")" && pwd)/$(basename -- "$1")"
mkdir -p "$2"
directory="$(cd -- "$2" && pwd)"
variant=$3
simulator=$4

if [[ ! -f "$directory/project/nexa.config.nx" ]]; then
    "$nexa" create IconFastList --directory "$directory/project"
fi
cp "$fixture/App.nx" "$directory/project/App.nx"
mkdir -p "$directory/project/assets/fonts"
cp -R "$fixture/assets/icons" "$directory/project/assets/"
cp "$repo/examples/component-parity/assets/fonts/"mazicon.{ttf,icons.json} \
    "$directory/project/assets/fonts/"

cd "$directory/project"
out="build/$variant"
"$nexa" test --all --out "$out" > "$directory/$variant.log" 2>&1
python3 - "$out/ios/IconFastList/IconFastListApp.swift" "$fixture/FrameProbe.swift" <<'PY'
from pathlib import Path
import sys

host, probe = map(Path, sys.argv[1:])
source = host.read_text()
if source.count("NexaIconFastListScreen()") != 1:
    raise SystemExit("unexpected generated benchmark host")
host.write_text(
    source.replace("NexaIconFastListScreen()",
                   "NexaIconFastListScreen().onAppear { IconFastListFrameProbe.shared.start() }")
    + "\n" + probe.read_text()
)
PY

xcodebuild -project "$out/ios/IconFastList.xcodeproj" -scheme IconFastList \
    -configuration Release -destination "platform=iOS Simulator,id=$simulator" \
    -derivedDataPath "$out/ios-simulator-derived" CODE_SIGNING_ALLOWED=NO build \
    > "$directory/$variant-ios-release.log" 2>&1

cd "$out/android"
# Sign only the benchmark APK with the emulator debug key so the same app
# can be installed repeatedly while retaining Release optimization settings.
NEXA_ANDROID_KEYSTORE="$HOME/.android/debug.keystore" \
NEXA_ANDROID_STORE_PASSWORD=android NEXA_ANDROID_KEY_ALIAS=androiddebugkey \
NEXA_ANDROID_KEY_PASSWORD=android ./gradlew :app:assembleRelease \
    > "$directory/$variant-android-release.log" 2>&1
