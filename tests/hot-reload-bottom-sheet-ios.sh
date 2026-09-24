#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-bottom-sheet-ios.sh <nexa> <project-dir> <log-file> <fixture>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
project=$2
log_file=$3
fixture=$4
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
artifacts="$project/build/test-artifacts"

"$nexa" create BottomSheetSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
mkdir -p "$artifacts"
python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'state showSheet: Bool = false'
if old not in source:
    raise SystemExit("bottom sheet visibility state was not found")
path.write_text(source.replace(old, 'state showSheet: Bool = true', 1))
PY
swiftc "$script_dir/read-screenshot-text.swift" -framework Vision -framework ImageIO \
    -o "$project/read-screenshot-text"
xcrun simctl terminate booted dev.nexa.bottomsheetsmoke >/dev/null 2>&1 || true
cd "$project"
"$nexa" dev --ios >"$log_file" 2>&1 &
dev_pid=$!

cleanup() {
    kill -INT "$dev_pid" 2>/dev/null || true
    for _ in $(seq 1 10); do
        kill -0 "$dev_pid" 2>/dev/null || break
        sleep 1
    done
    kill -TERM "$dev_pid" 2>/dev/null || true
    wait "$dev_pid" 2>/dev/null || true
}
trap cleanup EXIT

wait_for_log() {
    local pattern=$1
    for _ in $(seq 1 180); do
        if grep -Fq "$pattern" "$log_file"; then return 0; fi
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

verify_visible_text() {
    local expected=$1
    for _ in $(seq 1 60); do
        xcrun simctl io booted screenshot "$artifacts/screen.png" >/dev/null 2>&1
        "$project/read-screenshot-text" "$artifacts/screen.png" >"$artifacts/screen-text.txt"
        if grep -Fq "$expected" "$artifacts/screen-text.txt"; then return 0; fi
        sleep 1
    done
    cat "$artifacts/screen-text.txt" >&2
    cat "$log_file" >&2
    echo "expected visible text: $expected" >&2
    return 1
}

wait_for_log "Nexa iOS dev runtime connected."
wait_for_log "Nexa iOS dev runtime applied module"
verify_visible_text "Sheet content"
baseline=$(grep -Fc "Nexa iOS dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = 'Text("Sheet content")'
if old not in source:
    raise SystemExit("bottom sheet fixture content was not found")
path.write_text(source.replace(old, 'Text("Reloaded sheet content")', 1))
PY

patched=0
for _ in $(seq 1 180); do
    current=$(grep -Fc "Nexa iOS dev runtime applied patch" "$log_file" || true)
    if (( current > baseline )); then patched=1; break; fi
    if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
    sleep 1
done
if (( patched == 0 )); then
    cat "$log_file" >&2
    echo "the running bottom sheet did not receive a hot-reload patch" >&2
    exit 1
fi
verify_visible_text "Reloaded sheet content"
echo "Nexa iOS hot-reload bottom-sheet smoke passed."
