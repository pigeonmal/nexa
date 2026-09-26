#!/usr/bin/env bash
# Shared hot-reload orchestration library for Nexa E2E tests.
set -euo pipefail

HR_SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
HR_ROOT_DIR="$(cd -- "$HR_SCRIPT_DIR/../.." && pwd)"

HR_DEV_PID=""
HR_DEV_FD_OPEN=0
HR_PLATFORM=""
HR_PLATFORM_LABEL=""
HR_PROJECT=""
HR_LOG=""
HR_ARTIFACTS=""
HR_NEXA=""

hr_init() {
    local nexa=$1
    local platform=$2
    local project=$3
    local log_file=$4
    local app_name=${5:-"HotReloadTest"}
    local fixture=${6:-""}

    HR_NEXA="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
    HR_PLATFORM="$platform"
    HR_PROJECT="$project"
    HR_LOG="$log_file"
    HR_ARTIFACTS="$HR_PROJECT/build/test-artifacts"

    if [[ "$HR_PLATFORM" == "ios" ]]; then
        HR_PLATFORM_LABEL="iOS"
    elif [[ "$HR_PLATFORM" == "android" ]]; then
        HR_PLATFORM_LABEL="Android"
    else
        echo "error: platform must be 'ios' or 'android', got '$platform'" >&2
        exit 2
    fi

    "$HR_NEXA" create "$app_name" --directory "$HR_PROJECT"
    if [[ -n "$fixture" && -f "$fixture" ]]; then
        cp "$fixture" "$HR_PROJECT/App.nx"
    fi
    mkdir -p "$HR_ARTIFACTS"

    trap hr_cleanup EXIT
}

hr_ensure_screenshot_tool() {
    local dest="${1:-"$HR_PROJECT/read-screenshot-text"}"
    local cache_dir="${TMPDIR:-/tmp}/nexa-test-cache"
    local cached_tool="$cache_dir/read-screenshot"
    local source="$HR_SCRIPT_DIR/../read-screenshot.swift"

    mkdir -p "$cache_dir"
    if [[ ! -x "$cached_tool" || "$source" -nt "$cached_tool" ]]; then
        swiftc "$source" -framework Vision -framework ImageIO -O -o "$cached_tool"
    fi
    cp "$cached_tool" "$dest"
    cp "$cached_tool" "$(dirname "$dest")/read-screenshot" 2>/dev/null || true
}

hr_start_dev() {
    local use_stdin_fifo=${1:-true}
    shift || true

    cd "$HR_PROJECT"
    if [[ "$use_stdin_fifo" == "true" ]]; then
        local dev_input="$HR_PROJECT/dev-input"
        [[ -p "$dev_input" ]] || mkfifo "$dev_input"
        exec 3<>"$dev_input"
        HR_DEV_FD_OPEN=1
        "$HR_NEXA" dev "--$HR_PLATFORM" "$@" <&3 >"$HR_LOG" 2>&1 &
    else
        "$HR_NEXA" dev "--$HR_PLATFORM" "$@" >"$HR_LOG" 2>&1 &
    fi
    HR_DEV_PID=$!
}

hr_cleanup() {
    if [[ -n "$HR_DEV_PID" ]]; then
        kill -INT "$HR_DEV_PID" 2>/dev/null || true
        for _ in $(seq 1 10); do
            kill -0 "$HR_DEV_PID" 2>/dev/null || break
            sleep 1
        done
        kill -TERM "$HR_DEV_PID" 2>/dev/null || true
        wait "$HR_DEV_PID" 2>/dev/null || true
        HR_DEV_PID=""
    fi
    if [[ "$HR_DEV_FD_OPEN" -eq 1 ]]; then
        exec 3>&- 2>/dev/null || true
        HR_DEV_FD_OPEN=0
    fi
}

hr_send_input() {
    local text=$1
    if [[ "$HR_DEV_FD_OPEN" -eq 1 ]]; then
        printf '%s' "$text" >&3
    fi
}

hr_wait_for_log() {
    local pattern=$1
    local timeout=${2:-180}
    for _ in $(seq 1 "$timeout"); do
        if grep -Fq "$pattern" "$HR_LOG"; then
            return 0
        fi
        if [[ -n "$HR_DEV_PID" ]] && ! kill -0 "$HR_DEV_PID" 2>/dev/null; then
            cat "$HR_LOG" >&2
            echo "nexa dev exited before logging: $pattern" >&2
            return 1
        fi
        sleep 1
    done
    cat "$HR_LOG" >&2
    echo "timed out waiting for log pattern: $pattern" >&2
    return 1
}

hr_get_patch_count() {
    grep -Fc "Nexa $HR_PLATFORM_LABEL dev runtime applied patch" "$HR_LOG" || true
}

hr_wait_for_patch_after() {
    local previous_count=$1
    local timeout=${2:-180}
    for _ in $(seq 1 "$timeout"); do
        local current_count
        current_count=$(hr_get_patch_count)
        if (( current_count > previous_count )); then
            return 0
        fi
        if [[ -n "$HR_DEV_PID" ]] && ! kill -0 "$HR_DEV_PID" 2>/dev/null; then
            cat "$HR_LOG" >&2
            echo "nexa dev exited before applying patch" >&2
            return 1
        fi
        sleep 1
    done
    cat "$HR_LOG" >&2
    echo "timed out waiting for patch (current: $(hr_get_patch_count), previous: $previous_count)" >&2
    return 1
}

hr_wait_for_visible_text() {
    local expected=$1
    local timeout=${2:-60}
    for _ in $(seq 1 "$timeout"); do
        if [[ "$HR_PLATFORM" == "android" ]]; then
            adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1 || true
            adb exec-out cat /sdcard/nexa-window.xml >"$HR_ARTIFACTS/window.xml" 2>/dev/null || true
            if grep -Fq "text=\"$expected\"" "$HR_ARTIFACTS/window.xml"; then
                return 0
            fi
        else
            xcrun simctl io booted screenshot "$HR_ARTIFACTS/screen.png" >/dev/null 2>&1 || true
            "$HR_PROJECT/read-screenshot-text" "$HR_ARTIFACTS/screen.png" >"$HR_ARTIFACTS/screen-text.txt" 2>/dev/null || true
            if grep -Fq "$expected" "$HR_ARTIFACTS/screen-text.txt"; then
                return 0
            fi
        fi
        sleep 1
    done
    if [[ "$HR_PLATFORM" == "android" ]]; then
        cat "$HR_ARTIFACTS/window.xml" >&2 2>/dev/null || true
    else
        cat "$HR_ARTIFACTS/screen-text.txt" >&2 2>/dev/null || true
    fi
    cat "$HR_LOG" >&2
    echo "expected visible text: $expected" >&2
    return 1
}

hr_wait_for_missing_visible_text() {
    local unexpected=$1
    local timeout=${2:-60}
    for _ in $(seq 1 "$timeout"); do
        if [[ "$HR_PLATFORM" == "android" ]]; then
            adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1 || true
            adb exec-out cat /sdcard/nexa-window.xml >"$HR_ARTIFACTS/window.xml" 2>/dev/null || true
            if ! grep -Fq "text=\"$unexpected\"" "$HR_ARTIFACTS/window.xml"; then
                return 0
            fi
        else
            xcrun simctl io booted screenshot "$HR_ARTIFACTS/screen.png" >/dev/null 2>&1 || true
            "$HR_PROJECT/read-screenshot-text" "$HR_ARTIFACTS/screen.png" >"$HR_ARTIFACTS/screen-text.txt" 2>/dev/null || true
            if ! grep -Fq "$unexpected" "$HR_ARTIFACTS/screen-text.txt"; then
                return 0
            fi
        fi
        sleep 1
    done
    echo "unexpected visible text still present: $unexpected" >&2
    return 1
}

hr_wait_for_visible_fragment() {
    local expected=$1
    local timeout=${2:-60}
    for _ in $(seq 1 "$timeout"); do
        if [[ "$HR_PLATFORM" == "android" ]]; then
            adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1 || true
            adb exec-out cat /sdcard/nexa-window.xml >"$HR_ARTIFACTS/window.xml" 2>/dev/null || true
            if grep -Fq "$expected" "$HR_ARTIFACTS/window.xml"; then
                return 0
            fi
        else
            xcrun simctl io booted screenshot "$HR_ARTIFACTS/screen.png" >/dev/null 2>&1 || true
            "$HR_PROJECT/read-screenshot-text" "$HR_ARTIFACTS/screen.png" >"$HR_ARTIFACTS/screen-text.txt" 2>/dev/null || true
            if grep -Fq "$expected" "$HR_ARTIFACTS/screen-text.txt"; then
                return 0
            fi
        fi
        sleep 1
    done
    cat "$HR_LOG" >&2
    echo "expected visible text fragment: $expected" >&2
    return 1
}

hr_replace_source() {
    local file=$1
    local old_text=$2
    local new_text=$3
    python3 - "$file" "$old_text" "$new_text" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
old = sys.argv[2]
new = sys.argv[3]
source = path.read_text()
if old not in source:
    raise SystemExit(f"target text not found in {path}: {old!r}")
path.write_text(source.replace(old, new, 1))
PY
}

hr_terminate_app() {
    local bundle_id=$1
    if [[ "$HR_PLATFORM" == "android" ]]; then
        adb shell am force-stop "$bundle_id" >/dev/null 2>&1 || true
    else
        xcrun simctl terminate booted "$bundle_id" >/dev/null 2>&1 || true
    fi
}

hr_wait_for_keyboard() {
    local timeout=${1:-60}
    for _ in $(seq 1 "$timeout"); do
        if [[ "$HR_PLATFORM" == "android" ]]; then
            adb shell dumpsys input_method >"$HR_ARTIFACTS/input-method.txt" 2>/dev/null || true
            if grep -Eq 'mInputShown=true|isInputViewShown=true|mShowRequested=true' "$HR_ARTIFACTS/input-method.txt"; then
                return 0
            fi
        else
            xcrun simctl io booted screenshot "$HR_ARTIFACTS/screen.png" >/dev/null 2>&1 || true
            "$HR_PROJECT/read-screenshot-text" "$HR_ARTIFACTS/screen.png" >"$HR_ARTIFACTS/screen-text.txt" 2>/dev/null || true
            if grep -Eiq 'space|return|123|emoji' "$HR_ARTIFACTS/screen-text.txt"; then
                return 0
            fi
        fi
        sleep 1
    done
    if [[ "$HR_PLATFORM" == "android" ]]; then
        cat "$HR_ARTIFACTS/input-method.txt" >&2 2>/dev/null || true
    else
        cat "$HR_ARTIFACTS/screen-text.txt" >&2 2>/dev/null || true
    fi
    cat "$HR_LOG" >&2
    echo "software keyboard was not visible on $HR_PLATFORM" >&2
    return 1
}

hr_android_focus_input() {
    for _ in $(seq 1 30); do
        adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1 || true
        adb exec-out cat /sdcard/nexa-window.xml >"$HR_ARTIFACTS/window.xml" 2>/dev/null || true
        local coordinates
        coordinates=$(python3 - "$HR_ARTIFACTS/window.xml" <<'PY'
import re
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
for node in root.iter("node"):
    if node.attrib.get("class") != "android.widget.EditText":
        continue
    match = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.attrib.get("bounds", ""))
    if match:
        left, top, right, bottom = map(int, match.groups())
        print((left + right) // 2, (top + bottom) // 2)
        break
PY
)
        if [[ -n "$coordinates" ]]; then
            read -r x y <<<"$coordinates"
            adb shell input tap "$x" "$y"
            return 0
        fi
        sleep 1
    done
    cat "$HR_ARTIFACTS/window.xml" >&2 2>/dev/null || true
    echo "could not find editable TextInput in Android UI tree" >&2
    return 1
}
