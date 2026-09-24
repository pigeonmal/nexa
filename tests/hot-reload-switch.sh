#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-switch.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
log_file=$4
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_switch_reload.nx"
artifacts="$project/build/test-artifacts"
if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create SwitchReloadSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
mkdir -p "$artifacts"
mkfifo "$project/dev-stdin"
exec 3<>"$project/dev-stdin"
if [[ "$platform" == ios ]]; then
    swiftc "$script_dir/read-screenshot-text.swift" -framework Vision -framework ImageIO \
        -o "$project/read-screenshot-text"
fi
cd "$project"
"$nexa" dev "--$platform" <&3 >"$log_file" 2>&1 &
dev_pid=$!

cleanup() {
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
    for _ in $(seq 1 180); do
        if grep -Fq "$pattern" "$log_file"; then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for: $pattern" >&2
    return 1
}

visible_text() {
    if [[ "$platform" == android ]]; then
        adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-window.xml >"$artifacts/window.xml"
    else
        xcrun simctl io booted screenshot "$artifacts/screen.png" >/dev/null 2>&1
        "$project/read-screenshot-text" "$artifacts/screen.png" >"$artifacts/screen-text.txt"
    fi
}

wait_for_visible_text() {
    local expected=$1
    for _ in $(seq 1 60); do
        visible_text
        local content_file="$artifacts/screen-text.txt"
        [[ "$platform" == android ]] && content_file="$artifacts/window.xml"
        if grep -Fq "$expected" "$content_file"; then return 0; fi
        sleep 1
    done
    cat "$artifacts/window.xml" "$artifacts/screen-text.txt" "$log_file" >&2
    echo "expected visible text: $expected" >&2
    return 1
}

wait_for_patch_after() {
    local previous_count=$1
    for _ in $(seq 1 180); do
        local current_count
        current_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
        if (( current_count > previous_count )); then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for the switch update patch" >&2
    return 1
}

platform_label=iOS
[[ "$platform" == android ]] && platform_label=Android
wait_for_log "Nexa $platform_label dev runtime connected."
wait_for_log "Nexa $platform_label dev runtime applied module"
wait_for_visible_text "Enable Notifications"
patch_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'label: "Enable Notifications"'
if old not in source:
    raise SystemExit("switch label was not found")
path.write_text(source.replace(old, 'label: "Notifications Ready"', 1))
PY

wait_for_patch_after "$patch_count"
wait_for_visible_text "Notifications Ready"
echo "Nexa $platform_label switch render and hot reload passed."
