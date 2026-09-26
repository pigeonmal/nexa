#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/lib/hot_reload.sh"

hr_init "$@"
hr_terminate_app "dev.nexa.bottombarsmoke"

"$hr_nexa" create BottomBarSmoke --directory "$hr_project"
cp "$hr_fixture" "$hr_project/App.nx"
mkdir -p "$hr_artifacts"

hr_replace_source 'state selectedTab: Int32 = 0' 'state selectedTab: Int32 = 1'
hr_ensure_screenshot_tool
hr_start_dev --ios

hr_wait_for_log "Nexa iOS dev runtime connected."
hr_wait_for_log "Nexa iOS dev runtime applied module"
hr_wait_for_visible_text "Settings tab"

baseline=$(hr_get_patch_count "Nexa iOS dev runtime applied patch")
hr_replace_source 'Text("Settings tab")' 'Text("Reloaded settings tab")'
hr_wait_for_patch_after "$baseline" "Nexa iOS dev runtime applied patch"
hr_wait_for_visible_text "Reloaded settings tab"

echo "Nexa iOS hot-reload bottom-bar smoke passed."
