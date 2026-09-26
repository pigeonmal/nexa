#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-focus.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/lib/hot_reload.sh"

fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_focus_reload.nx"
hr_init "$1" "$2" "$3" "$4" "FocusReloadSmoke" "$fixture"

if [[ "$HR_PLATFORM" == "ios" ]]; then
    hr_ensure_screenshot_tool
fi

hr_start_dev true

hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime connected."
hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime applied module"
hr_wait_for_visible_text "Focus Ready"
if [[ "$HR_PLATFORM" == "android" ]]; then
    hr_android_focus_input
fi
hr_wait_for_keyboard

patch_count=$(hr_get_patch_count)

hr_replace_source "$HR_PROJECT/App.nx" 'Text("Focus footer")' 'Text("Focus after reload")'
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "Focus after reload"
hr_wait_for_keyboard

echo "Nexa $HR_PLATFORM focused-input retention and hot reload passed."
