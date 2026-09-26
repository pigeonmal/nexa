#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/lib/hot_reload.sh"

hr_init "$@"
hr_terminate_app "dev.nexa.statereloadsmoke"

"$hr_nexa" create StateReloadSmoke --directory "$hr_project"
cp "$hr_fixture" "$hr_project/App.nx"
mkdir -p "$hr_artifacts"

hr_ensure_screenshot_tool
hr_start_dev --ios

hr_wait_for_log "Nexa iOS dev runtime connected."
hr_wait_for_log "Nexa iOS dev runtime applied module"
hr_wait_for_visible_text "Count: 7"

patch_count=$(hr_get_patch_count "Nexa iOS dev runtime applied patch")
hr_replace_source "state count = 7" "state count = 99"
patch_count=$(hr_wait_for_patch_after "$patch_count" "Nexa iOS dev runtime applied patch")
hr_wait_for_visible_text "Count: 7"

hr_send_input "R"
hr_wait_for_log "Nexa app state restarted in the running app."
hr_wait_for_visible_text "Count: 99"
echo "Nexa iOS hot restart restored the current source initializer."

hr_replace_source "state count = 99" 'state count: String = "fresh"'
patch_count=$(hr_wait_for_patch_after "$patch_count" "Nexa iOS dev runtime applied patch")
hr_wait_for_visible_text "Count: fresh"
echo "Nexa iOS compatible-state preservation and incompatible-state reset passed."
