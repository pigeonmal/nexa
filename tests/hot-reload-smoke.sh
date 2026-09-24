#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-smoke.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
log_file=$4
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_keyboard_aware.nx"
artifacts="$project/build/test-artifacts"
if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create KeyboardAwareSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
mkdir -p "$artifacts"
dev_input="$project/dev-input"
mkfifo "$dev_input"
exec 3<>"$dev_input"
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
        if grep -Fq "$pattern" "$log_file"; then
            return 0
        fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then
            cat "$log_file" >&2
            echo "nexa dev exited before logging: $pattern" >&2
            return 1
        fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for: $pattern" >&2
    return 1
}

wait_for_patch_after() {
    local previous_count=$1
    for _ in $(seq 1 180); do
        current_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
        if (( current_count > previous_count )); then
            patch_count=$current_count
            return 0
        fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then
            cat "$log_file" >&2
            echo "nexa dev exited before the runtime applied its incremental patch" >&2
            return 1
        fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for the runtime to apply its incremental patch" >&2
    return 1
}

platform_label=iOS
if [[ "$platform" == android ]]; then
    platform_label=Android
fi

wait_for_visible_text() {
    local expected=$1
    for _ in $(seq 1 60); do
        if [[ "$platform" == android ]]; then
            adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1
            adb exec-out cat /sdcard/nexa-window.xml >"$artifacts/window.xml"
            if grep -Fq "text=\"$expected\"" "$artifacts/window.xml"; then return 0; fi
        else
            xcrun simctl io booted screenshot "$artifacts/screen.png" >/dev/null 2>&1
            "$project/read-screenshot-text" "$artifacts/screen.png" >"$artifacts/screen-text.txt"
            if grep -Fq "$expected" "$artifacts/screen-text.txt"; then return 0; fi
        fi
        sleep 1
    done
    if [[ "$platform" == android ]]; then
        cat "$artifacts/window.xml" >&2
    else
        cat "$artifacts/screen-text.txt" >&2
    fi
    cat "$log_file" >&2
    echo "expected visible text: $expected" >&2
    return 1
}

wait_for_missing_visible_text() {
    local unexpected=$1
    for _ in $(seq 1 60); do
        if [[ "$platform" == android ]]; then
            adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1
            adb exec-out cat /sdcard/nexa-window.xml >"$artifacts/window.xml"
            if ! grep -Fq "text=\"$unexpected\"" "$artifacts/window.xml"; then return 0; fi
        else
            xcrun simctl io booted screenshot "$artifacts/screen.png" >/dev/null 2>&1
            "$project/read-screenshot-text" "$artifacts/screen.png" >"$artifacts/screen-text.txt"
            if ! grep -Fq "$unexpected" "$artifacts/screen-text.txt"; then return 0; fi
        fi
        sleep 1
    done
    echo "unexpected visible text: $unexpected" >&2
    if [[ "$platform" == android ]]; then
        cat "$artifacts/window.xml" >&2
    else
        cat "$artifacts/screen-text.txt" >&2
    fi
    return 1
}

wait_for_visible_fragment() {
    local expected=$1
    for _ in $(seq 1 60); do
        if [[ "$platform" == android ]]; then
            adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1
            adb exec-out cat /sdcard/nexa-window.xml >"$artifacts/window.xml"
            if grep -Fq "$expected" "$artifacts/window.xml"; then return 0; fi
        else
            xcrun simctl io booted screenshot "$artifacts/screen.png" >/dev/null 2>&1
            "$project/read-screenshot-text" "$artifacts/screen.png" >"$artifacts/screen-text.txt"
            if grep -Fq "$expected" "$artifacts/screen-text.txt"; then return 0; fi
        fi
        sleep 1
    done
    if [[ "$platform" == android ]]; then
        cat "$artifacts/window.xml" >&2
    else
        cat "$artifacts/screen-text.txt" >&2
    fi
    cat "$log_file" >&2
    echo "expected visible text fragment: $expected" >&2
    return 1
}

wait_for_log "Nexa $platform_label dev runtime connected."
wait_for_log "Nexa $platform_label dev runtime applied module"
wait_for_missing_visible_text "Nexa Performance"
printf 'p\n' >&3
wait_for_log "Nexa performance overlay on."
wait_for_visible_text "Nexa Performance"
wait_for_visible_fragment "FPS"
wait_for_visible_fragment "Frame"
printf 'p\n' >&3
wait_for_log "Nexa performance overlay off."
wait_for_missing_visible_text "Nexa Performance"
wait_for_visible_text "Open guide"
patch_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("Open guide")'
if old not in source:
    raise SystemExit("link label was not found")
path.write_text(source.replace(old, 'Text("Updated guide")', 1))
PY
wait_for_patch_after "$patch_count"
wait_for_visible_text "Updated guide"

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = "if showPrimary"
if old not in source:
    raise SystemExit("conditional expression was not found")
path.write_text(source.replace(old, "if !showPrimary", 1))
PY
wait_for_patch_after "$patch_count"
wait_for_visible_text "Fallback branch"
wait_for_missing_visible_text "Primary branch"
wait_for_visible_text "Ready status"

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = '"ready": {'
if old not in source:
    raise SystemExit("when case value was not found")
path.write_text(source.replace(old, '"other": {', 1))
PY
wait_for_patch_after "$patch_count"
wait_for_visible_text "Fallback status"
wait_for_missing_visible_text "Ready status"

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("Enter a username")'
if old not in source:
    raise SystemExit("keyboard-aware fixture text was not found")
path.write_text(source.replace(old, 'Text("Live reload verified")', 1))
PY
wait_for_patch_after "$patch_count"
wait_for_visible_text "Live reload verified"

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("Live reload verified")'
if old not in source:
    raise SystemExit("edited text was not found")
path.write_text(source.replace(old, old + '\n                Text("Added by hot reload")', 1))
PY
wait_for_patch_after "$patch_count"
wait_for_visible_text "Added by hot reload"

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
added = '                Text("Added by hot reload")\n'
if added not in source:
    raise SystemExit("added child was not found")
path.write_text(source.replace(added, "", 1))
PY
wait_for_patch_after "$patch_count"
wait_for_missing_visible_text "Added by hot reload"

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("Live reload verified")'
if old not in source:
    raise SystemExit("reloaded text was not found")
path.write_text(source.replace(old, 'Text("broken', 1))
PY

wait_for_log "error: unterminated string literal"
wait_for_visible_text "Updated guide"
current_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
if (( current_count != patch_count )); then
    cat "$log_file" >&2
    echo "invalid source changed the last-good runtime module" >&2
    exit 1
fi

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("broken'
if old not in source:
    raise SystemExit("broken source was not found for recovery")
path.write_text(source.replace(old, 'Text("Recovered after diagnostics")', 1))
PY
wait_for_patch_after "$patch_count"
wait_for_visible_text "Recovered after diagnostics"
echo "Nexa $platform hot-reload smoke passed."
