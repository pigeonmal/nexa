#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-switch.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/lib/hot_reload.sh"

fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_switch_reload.nx"
hr_init "$1" "$2" "$3" "$4" "SwitchReloadSmoke" "$fixture"

if [[ "$HR_PLATFORM" == "ios" ]]; then
    hr_ensure_screenshot_tool
fi

hr_start_dev true

hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime connected."
hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime applied module"
hr_wait_for_visible_text "Enable Notifications"

patch_count=$(hr_get_patch_count)

hr_replace_source "$HR_PROJECT/App.nx" 'label: "Enable Notifications"' 'label: "Notifications Ready"'
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "Notifications Ready"

echo "Nexa $HR_PLATFORM_LABEL switch render and hot reload passed."
