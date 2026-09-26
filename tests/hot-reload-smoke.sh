#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: hot-reload-smoke.sh <nexa> <ios|android> <project-dir> <log-file>" >&2
    exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/lib/hot_reload.sh"

fixture="$script_dir/../crates/nexa-cli/tests/fixtures/dev_keyboard_aware.nx"
hr_init "$1" "$2" "$3" "$4" "KeyboardAwareSmoke" "$fixture"

if [[ "$HR_PLATFORM" == "ios" ]]; then
    hr_ensure_screenshot_tool
fi

hr_start_dev true

hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime connected."
hr_wait_for_log "Nexa $HR_PLATFORM_LABEL dev runtime applied module"
hr_wait_for_missing_visible_text "Nexa Performance"

hr_send_input "p\n"
hr_wait_for_log "Nexa performance overlay on."
hr_wait_for_visible_text "Nexa Performance"
hr_wait_for_visible_fragment "FPS"
hr_wait_for_visible_fragment "Frame"

hr_send_input "p\n"
hr_wait_for_log "Nexa performance overlay off."
hr_wait_for_missing_visible_text "Nexa Performance"
hr_wait_for_visible_text "Open guide"

patch_count=$(hr_get_patch_count)

hr_replace_source "$HR_PROJECT/App.nx" 'Text("Open guide")' 'Text("Updated guide")'
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "Updated guide"

hr_replace_source "$HR_PROJECT/App.nx" "if showPrimary" "if !showPrimary"
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "Fallback branch"
hr_wait_for_missing_visible_text "Primary branch"
hr_wait_for_visible_text "Ready status"

hr_replace_source "$HR_PROJECT/App.nx" '"ready": {' '"other": {'
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "Fallback status"
hr_wait_for_missing_visible_text "Ready status"

hr_replace_source "$HR_PROJECT/App.nx" 'Text("Enter a username")' 'Text("Live reload verified")'
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "Live reload verified"

hr_replace_source "$HR_PROJECT/App.nx" 'Text("Live reload verified")' $'Text("Live reload verified")\n                Text("Added by hot reload")'
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "Added by hot reload"

hr_replace_source "$HR_PROJECT/App.nx" $'                Text("Added by hot reload")\n' ""
hr_wait_for_patch_after "$patch_count"
hr_wait_for_missing_visible_text "Added by hot reload"

hr_replace_source "$HR_PROJECT/App.nx" 'Text("Live reload verified")' 'Text("broken'

hr_wait_for_log "error: unterminated string literal"
hr_wait_for_visible_text "Updated guide"
current_count=$(hr_get_patch_count)
if (( current_count != patch_count )); then
    cat "$HR_LOG" >&2
    echo "invalid source changed the last-good runtime module" >&2
    exit 1
fi

hr_replace_source "$HR_PROJECT/App.nx" 'Text("broken' 'Text("Recovered after diagnostics")'
hr_wait_for_patch_after "$patch_count"
hr_wait_for_visible_text "Recovered after diagnostics"

echo "Nexa $HR_PLATFORM hot-reload smoke passed."
