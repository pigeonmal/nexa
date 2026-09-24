#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-fast-list-axis.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
log_file=$4
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_fast_list_axis_reload.nx"
artifacts="$project/build/test-artifacts"

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create FastListAxisSmoke --directory "$project"
cp "$fixture" "$project/App.nx"
mkdir -p "$artifacts"
if [[ "$platform" == ios ]]; then
    swiftc "$script_dir/read-screenshot-geometry.swift" -framework Vision -framework ImageIO \
        -o "$project/read-screenshot-geometry"
fi
mkfifo "$project/dev-stdin"
exec 3<>"$project/dev-stdin"
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

platform_label=iOS
if [[ "$platform" == android ]]; then platform_label=Android; fi

capture() {
    if [[ "$platform" == android ]]; then
        adb shell uiautomator dump /sdcard/nexa-window.xml >/dev/null 2>&1
        adb exec-out cat /sdcard/nexa-window.xml >"$artifacts/window.xml"
        python3 - "$artifacts/window.xml" >"$artifacts/geometry.tsv" <<'PY'
import re
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
for node in root.iter("node"):
    match = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.attrib.get("bounds", ""))
    if not match:
        continue
    left, top, right, bottom = map(int, match.groups())
    label = node.attrib.get("text") or node.attrib.get("content-desc") or ""
    if label:
        print(f"{(left + right) / 2}\t{(top + bottom) / 2}\t{label}")
PY
    else
        xcrun simctl io booted screenshot "$artifacts/screen.png" >/dev/null 2>&1
        "$project/read-screenshot-geometry" "$artifacts/screen.png" >"$artifacts/geometry.tsv"
    fi
}

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

wait_for_text() {
    local expected=$1
    for _ in $(seq 1 60); do
        capture
        if grep -Fq "$expected" "$artifacts/geometry.tsv"; then return 0; fi
        sleep 1
    done
    cat "$artifacts/geometry.tsv" >&2
    cat "$log_file" >&2
    echo "expected visible list text: $expected" >&2
    return 1
}

assert_axis_geometry() {
    local axis=$1
    local prefix=$2
    capture
    python3 - "$artifacts/geometry.tsv" "$platform" "$axis" "$prefix" <<'PY'
import sys

path, platform, axis, prefix = sys.argv[1:]
found = {}
for line in open(path, encoding="utf-8"):
    fields = line.rstrip("\n").split("\t", 2)
    if len(fields) != 3:
        continue
    x, y, text = fields
    normalized = text.replace("O", "0").replace("o", "0")
    for index in (0, 1, 2):
        if f"{prefix} {index}" in normalized and index not in found:
            found[index] = (float(x), float(y))

if 0 not in found or 1 not in found:
    raise SystemExit(f"could not read adjacent list items 0 and 1 from {path}: {found}")
tolerance = 0.05 if platform == "ios" else 5.0
if axis == "Horizontal":
    if found[1][0] <= found[0][0] or abs(found[1][1] - found[0][1]) > tolerance:
        raise SystemExit(f"horizontal FastList items should advance on x only: {found}")
elif axis == "Grid":
    if 2 not in found:
        raise SystemExit(f"could not read grid item 2 from {path}: {found}")
    if found[1][0] <= found[0][0] or abs(found[1][1] - found[0][1]) > tolerance:
        raise SystemExit(f"grid items 0 and 1 should share a row: {found}")
    if found[2][1] <= found[0][1] + tolerance:
        raise SystemExit(f"grid item 2 should start the next row: {found}")
else:
    raise SystemExit(f"unknown list axis {axis}")
PY
}

wait_for_log "Nexa $platform_label dev runtime connected."
wait_for_log "Nexa $platform_label dev runtime applied module"
wait_for_text "Cell 1"
assert_axis_geometry Horizontal "Cell"
patch_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)

python3 - "$project/App.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = "axis: Horizontal"
if old not in source:
    raise SystemExit("horizontal FastList axis was not found")
source = source.replace(old, "axis: Grid(2)", 1)
old = 'Text("Cell $item $index")'
if old not in source:
    raise SystemExit("FastList row label was not found")
path.write_text(source.replace(old, 'Text("Updated Cell $item $index")', 1))
PY

for _ in $(seq 1 180); do
    current_count=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
    if (( current_count > patch_count )); then break; fi
    if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; exit 1; fi
    sleep 1
done
if (( current_count <= patch_count )); then
    cat "$log_file" >&2
    echo "timed out waiting for grid FastList hot reload" >&2
    exit 1
fi
wait_for_text "Updated Cell 1"
assert_axis_geometry Grid "Updated Cell"
echo "Nexa $platform horizontal and grid FastList rendering and axis hot reload passed."
