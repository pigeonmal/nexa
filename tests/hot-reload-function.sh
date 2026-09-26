#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-function.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/lib/hot_reload.sh"

fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_function_reload.nx"
hr_init "$1" "$2" "$3" "$4" "FunctionReloadSmoke" "$fixture"

if [[ "$HR_PLATFORM" == "ios" ]]; then
    hr_ensure_screenshot_tool
fi

hr_start_dev true

hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime connected."
hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime applied module"
hr_wait_for_visible_text "11"

patch_count=$(hr_get_patch_count)

hr_replace_source "$HR_PROJECT/App.nx" "return value + 1" "return value + 10"
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "20"

echo "Nexa $HR_PLATFORM function hot reload passed."
