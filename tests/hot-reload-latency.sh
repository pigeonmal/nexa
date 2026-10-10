#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: hot-reload-latency.sh <nexa> <ios|android> <project-dir>" >&2
    exit 2
fi

nexa=$1
platform=$2
project=$3
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/lib/hot_reload.sh"

if [[ "$platform" != ios && "$platform" != android ]]; then
    echo "platform must be ios or android" >&2
    exit 2
fi

fixture="$script_dir/fixtures/dev_component_initial.nx"
hr_init "$nexa" "$platform" "$project" "$project/dev.log" "ComponentSmoke" "$fixture"
if [[ "$platform" == ios ]]; then
    hr_ensure_screenshot_tool
fi
hr_start_dev

hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime connected."
hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime applied module"

source_file="$project/App.nx"
for revision in 1 2 3; do
    previous=$((revision - 1))
    previous_patches=$(hr_get_patch_count)
    started_ns=$(python3 -c 'import time; print(time.monotonic_ns())')
    python3 - "$source_file" "$previous" "$revision" <<'PY'
import os
import sys
from pathlib import Path

path = Path(sys.argv[1])
old = "Initial" if sys.argv[2] == "0" else f"Reload {sys.argv[2]}"
new = f"Reload {sys.argv[3]}"
source = path.read_text()
if old not in source:
    raise SystemExit(f"expected {old!r} in {path}")
temporary = path.with_name(path.name + ".reload-tmp")
temporary.write_text(source.replace(old, new, 1))
os.replace(temporary, path)
PY

    applied=0
    for _ in $(seq 1 500); do
        current_patches=$(hr_get_patch_count)
        if (( current_patches > previous_patches )); then
            applied=1
            break
        fi
        if [[ -n "$HR_DEV_PID" ]] && ! kill -0 "$HR_DEV_PID" 2>/dev/null; then
            cat "$HR_LOG" >&2
            echo "nexa dev exited before applying reload $revision" >&2
            exit 1
        fi
        sleep 0.01
    done
    if (( applied == 0 )); then
        cat "$HR_LOG" >&2
        echo "timed out waiting for reload $revision" >&2
        exit 1
    fi

    finished_ns=$(python3 -c 'import time; print(time.monotonic_ns())')
    elapsed_ms=$(python3 - "$started_ns" "$finished_ns" <<'PY'
import sys
print((int(sys.argv[2]) - int(sys.argv[1])) / 1_000_000)
PY
    )
    printf 'Nexa %s hot reload %s applied in %s ms (budget: 1000 ms)\n' \
        "$HR_PLATFORM_LABEL" "$revision" "$elapsed_ms"
    if python3 - "$elapsed_ms" <<'PY'
import sys
raise SystemExit(0 if float(sys.argv[1]) <= 1000 else 1)
PY
    then
        :
    else
        cat "$HR_LOG" >&2
        echo "hot reload exceeded the 1000 ms interactive budget" >&2
        exit 1
    fi
done

hr_wait_for_visible_text "Reload 3" 30
echo "Nexa $HR_PLATFORM_LABEL hot-reload latency smoke passed."
