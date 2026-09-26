#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-bottom-sheet-ios.sh <nexa> <project-dir> <log-file> <fixture>" >&2
    exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/lib/hot_reload.sh"

hr_init "$1" "ios" "$2" "$3" "BottomSheetSmoke" "$4"
hr_replace_source "$HR_PROJECT/App.nx" 'state showSheet: Bool = false' 'state showSheet: Bool = true'
hr_ensure_screenshot_tool
xcrun simctl terminate booted dev.nexa.bottomsheetsmoke >/dev/null 2>&1 || true

hr_start_dev false

hr_wait_for_log "Nexa iOS dev runtime connected."
hr_wait_for_log "Nexa iOS dev runtime applied module"
hr_wait_for_visible_text "Sheet content"

baseline=$(hr_get_patch_count)
hr_replace_source "$HR_PROJECT/App.nx" 'Text("Sheet content")' 'Text("Reloaded sheet content")'
hr_wait_for_patch_after "$baseline"
hr_wait_for_visible_text "Reloaded sheet content"

echo "Nexa iOS hot-reload bottom-sheet smoke passed."
