#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 || ( "$2" != "ios" && "$2" != "android" ) ]]; then
  echo "Usage: $0 <nexa> <ios|android>" >&2
  exit 2
fi

nexa_bin=$1
nexa_bin="$(cd "$(dirname "$nexa_bin")" && pwd)/$(basename "$nexa_bin")"
platform=$2
if [[ "$platform" == "ios" ]]; then
  target_name="iOS"
else
  target_name="Android"
fi
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/nexa-array-initializers.XXXXXX")"
project="$tmp_root/project"
native_log="$tmp_root/native.log"
dev_log="$tmp_root/dev.log"
marker="NEXA_ARRAY_INITIALIZER_$$"
dev_pid=""
log_pid=""
device_id=""
adb_cmd=()

print_logs() {
  echo "--- Nexa dev session ---" >&2
  tail -n 100 "$dev_log" >&2 2>/dev/null || true
  echo "--- Native runtime logs ---" >&2
  tail -n 100 "$native_log" >&2 2>/dev/null || true
}

cleanup() {
  local status=$?
  if [[ -n "$dev_pid" ]]; then
    kill -TERM "$dev_pid" 2>/dev/null || true
    wait "$dev_pid" 2>/dev/null || true
  fi
  if [[ "$platform" == "ios" && -n "$device_id" ]]; then
    xcrun simctl terminate "$device_id" dev.nexa.arrayinitializersmoke >/dev/null 2>&1 || true
  elif [[ "$platform" == "android" && ${#adb_cmd[@]} -gt 0 ]]; then
    "${adb_cmd[@]}" shell am force-stop dev.nexa.arrayinitializersmoke >/dev/null 2>&1 || true
  fi
  if [[ -n "$log_pid" ]]; then
    kill "$log_pid" 2>/dev/null || true
    wait "$log_pid" 2>/dev/null || true
  fi
  if [[ $status -eq 0 && "${NEXA_KEEP_ARRAY_INITIALIZER_PROBE:-0}" != "1" ]]; then
    rm -rf "$tmp_root"
  else
    echo "Probe files and logs: $tmp_root" >&2
  fi
}
trap cleanup EXIT

"$nexa_bin" create ArrayInitializerSmoke --directory "$project"
sed "s/NEXA_ARRAY_INITIALIZER/$marker/g" "$root/tests/fixtures/dev_array_initializers.nx" >"$project/App.nx"
cp "$root/tests/fixtures/dev_array_initializers.config.nx" "$project/nexa.config.nx"

if [[ "$platform" == "ios" ]]; then
  command -v xcrun >/dev/null || { echo "xcrun is required for the iOS probe" >&2; exit 1; }
  device_id="$(xcrun simctl list devices booted | grep -Eo '[0-9A-F-]{36}' | head -n 1 || true)"
  [[ -n "$device_id" ]] || { echo "Boot an iOS Simulator before running this probe" >&2; exit 1; }
  xcrun simctl terminate "$device_id" dev.nexa.arrayinitializersmoke >/dev/null 2>&1 || true
  xcrun simctl spawn "$device_id" log stream --style compact --predicate "eventMessage CONTAINS '$marker'" >"$native_log" 2>&1 &
  log_pid=$!
else
  command -v adb >/dev/null || { echo "adb is required for the Android probe" >&2; exit 1; }
  adb_cmd=(adb)
  if [[ -n "${ANDROID_SERIAL:-}" ]]; then adb_cmd+=( -s "$ANDROID_SERIAL" ); fi
  [[ "$("${adb_cmd[@]}" get-state 2>/dev/null || true)" == "device" ]] || {
    echo "Connect or boot an Android Emulator before running this probe" >&2
    exit 1
  }
  "${adb_cmd[@]}" shell am force-stop dev.nexa.arrayinitializersmoke >/dev/null 2>&1 || true
  "${adb_cmd[@]}" logcat -c
  "${adb_cmd[@]}" logcat -v time 'Nexa:I' 'NexaDevRuntime:I' '*:S' >"$native_log" 2>&1 &
  log_pid=$!
fi

wait_for_marker() {
  local file=$1
  local expected=$2
  for _ in $(seq 1 180); do
    grep -Fq "$expected" "$file" 2>/dev/null && return 0
    if [[ -n "$dev_pid" ]] && ! kill -0 "$dev_pid" 2>/dev/null; then
      print_logs
      echo "Nexa dev exited before logging: $expected" >&2
      return 1
    fi
    sleep 0.5
  done
  print_logs
  echo "Timed out waiting for log marker: $expected" >&2
  return 1
}

(cd "$project" && exec "$nexa_bin" dev "--$platform" --out "$project/build") >"$dev_log" 2>&1 &
dev_pid=$!
wait_for_marker "$dev_log" "Nexa $target_name dev runtime connected."
wait_for_marker "$dev_log" "Nexa $target_name dev runtime applied module"
wait_for_marker "$native_log" "$marker first=10 last=30"
echo "DevRuntime collection initializer values passed on $platform (native logs only)."
