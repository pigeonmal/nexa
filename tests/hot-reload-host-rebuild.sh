#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-host-rebuild.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

nexa=$1
nexa="$(cd -- "$(dirname -- "$nexa")" && pwd)/$(basename -- "$nexa")"
platform=$2
project=$3
log_file=$4
input_fifo="$project/dev-input"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

"$nexa" create NativeRebuildSmoke --directory "$project"
mkdir -p "$project/plugins"
cp -R "$script_dir/../examples/plugins/fast-math" "$project/plugins/fast-math"
python3 - "$project/nexa.config.nx" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text().rstrip()
if not source.endswith("}"):
    raise SystemExit("config root block was not found")
source = source[:-1] + '\n    dependencies { FastMath { id: "dev.nexa.fast-math", path: "plugins/fast-math" } }\n}\n'
path.write_text(source)
PY
cat >"$project/App.nx" <<'NX'
plugin "dev.nexa.fast-math" as FastMath

app NativeRebuildSmoke {
    state result: Int32 = 0

    body {
        Column {
            Text("Native plugin host")
            Button("Compute") {
                result = FastMath.add(1, 2)
            }
        }
    }
}
NX
if [[ "$platform" == android ]]; then
    adb shell am force-stop dev.nexa.nativerebuildsmoke
else
    xcrun simctl terminate booted dev.nexa.nativerebuildsmoke >/dev/null 2>&1 || true
fi
cd "$project"
mkfifo "$input_fifo"
exec 3<>"$input_fifo"
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
    for _ in $(seq 1 240); do
        if grep -Fq "$pattern" "$log_file"; then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for: $pattern" >&2
    return 1
}

wait_for_count_increase() {
    local pattern=$1
    local old_count=$2
    for _ in $(seq 1 240); do
        local current_count
        current_count=$(grep -Fc "$pattern" "$log_file" || true)
        if (( current_count > old_count )); then return 0; fi
        if ! kill -0 "$dev_pid" 2>/dev/null; then cat "$log_file" >&2; return 1; fi
        sleep 1
    done
    cat "$log_file" >&2
    echo "timed out waiting for another '$pattern' event" >&2
    return 1
}

platform_label=iOS
if [[ "$platform" == android ]]; then platform_label=Android; fi
connected="Nexa $platform_label dev runtime connected."
host_waiting="Native host changes are waiting. Press 'b' to rebuild and relaunch the app."
wait_for_log "$connected"
wait_for_log "Nexa $platform_label dev runtime applied module"
wait_for_native_rebuild() {
    local old_connections old_rebuilds old_patches new_patches old_waiting current_rebuilds
    old_connections=$(grep -Fc "$connected" "$log_file" || true)
    old_rebuilds=$(grep -Fc "Native app rebuilt and relaunched." "$log_file" || true)
    old_patches=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
    old_waiting=$(grep -Fc "$host_waiting" "$log_file" || true)
    wait_for_count_increase "$host_waiting" "$old_waiting"
    sleep 2
    current_rebuilds=$(grep -Fc "Native app rebuilt and relaunched." "$log_file" || true)
    if (( current_rebuilds != old_rebuilds )); then
        cat "$log_file" >&2
        echo "a native-host configuration change rebuilt without the user shortcut" >&2
        exit 1
    fi
    printf 'b\n' >&3
    wait_for_count_increase "Native app rebuilt and relaunched." "$old_rebuilds"
    wait_for_count_increase "$connected" "$old_connections"
    new_patches=$(grep -Fc "Nexa $platform_label dev runtime applied patch" "$log_file" || true)
    if (( new_patches != old_patches )); then
        cat "$log_file" >&2
        echo "a native-host configuration change was incorrectly sent as a hot-reload patch" >&2
        exit 1
    fi
}

python3 - "$project/nexa.config.nx" <<'PY'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
updated, count = re.subn(
    r'(displayName:\s*)"[^"]+"',
    r'\1"Native Rebuild Verified"',
    source,
    count=1,
)
if count != 1:
    raise SystemExit("app displayName was not found in nexa.config.nx")
path.write_text(updated)
PY

wait_for_native_rebuild

python3 - "$project/nexa.config.nx" <<'PY'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
updated, count = re.subn(
    r'permissions\s*\{\s*\}',
    'permissions { camera: "Camera access for host rebuild smoke" }',
    source,
    count=1,
)
if count != 1:
    raise SystemExit("empty permissions block was not found in nexa.config.nx")
path.write_text(updated)
PY

wait_for_native_rebuild
if [[ "$platform" == ios ]]; then
    if ! grep -Fq "NSCameraUsageDescription" "$project/build/ios/NativeRebuildSmoke/Info.plist"; then
        echo "generated iOS Info.plist is missing camera usage metadata" >&2
        exit 1
    fi
else
    if ! grep -Fq "android.permission.CAMERA" "$project/build/android/app/src/main/AndroidManifest.xml"; then
        echo "generated Android manifest is missing camera permission" >&2
        exit 1
    fi
fi

plugin_source="$project/plugins/fast-math/cpp/Sources/FastMath.cpp"
python3 - "$plugin_source" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
source = path.read_text()
old = "return a + b;"
if old not in source:
    raise SystemExit("FastMath.add implementation was not found")
path.write_text(source.replace(old, "return a + b + 1;", 1))
PY
wait_for_native_rebuild

python3 - \
    "$project/plugins/fast-math/native.nxid" \
    "$project/plugins/fast-math/cpp/include/FastMath.hpp" \
    "$project/plugins/fast-math/cpp/Sources/FastMath.cpp" <<'PY'
import sys
from pathlib import Path

contract_path, header_path, source_path = map(Path, sys.argv[1:])
contract = contract_path.read_text()
if not contract.endswith("}\n"):
    raise SystemExit("FastMath native service declaration was not found")
contract_path.write_text(contract[:-2] + "    fn subtract(a: Int32, b: Int32) -> Int32\n}\n")

header = header_path.read_text()
declaration = "std::int32_t subtract(std::int32_t a, std::int32_t b) noexcept;"
if "std::int32_t add(" not in header:
    raise SystemExit("FastMath C++ header declarations were not found")
header_path.write_text(header.replace(
    "std::int32_t add(std::int32_t a, std::int32_t b) noexcept;",
    "std::int32_t add(std::int32_t a, std::int32_t b) noexcept;\n" + declaration,
    1,
))

source = source_path.read_text()
marker = "std::int32_t multiply(std::int32_t a, std::int32_t b) noexcept {"
if marker not in source:
    raise SystemExit("FastMath C++ implementation was not found")
implementation = "std::int32_t subtract(std::int32_t a, std::int32_t b) noexcept {\n    return a - b;\n}\n\n"
source_path.write_text(source.replace(marker, implementation + marker, 1))
PY
wait_for_native_rebuild

cp -R "$project/plugins/fast-math" "$project/plugins/fast-math-replacement"
python3 - "$project/nexa.config.nx" "$project/plugins/fast-math-replacement/cpp/Sources/FastMath.cpp" <<'PY'
import sys
from pathlib import Path

config_path = Path(sys.argv[1])
source = config_path.read_text()
old = 'path: "plugins/fast-math"'
if old not in source:
    raise SystemExit("FastMath dependency path was not found")
config_path.write_text(source.replace(old, 'path: "plugins/fast-math-replacement"', 1))

plugin_source = Path(sys.argv[2])
contents = plugin_source.read_text()
old = "return a + b + 1;"
if old not in contents:
    raise SystemExit("replacement FastMath.add implementation was not found")
plugin_source.write_text(contents.replace(old, "return a + b + 2;", 1))
PY
wait_for_native_rebuild

echo "Nexa $platform app metadata, permission, plugin source/interface, and dependency changes rebuilt the host and reconnected the runtime."
