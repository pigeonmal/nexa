#!/usr/bin/env bash
# Exercise plugin calls and a newly created imported .nx component in the
# running iOS Simulator or Android Emulator DevRuntime. Evidence is log-only.
set -euo pipefail

if [[ $# -ne 1 || ( "$1" != "ios" && "$1" != "android" ) ]]; then
  echo "Usage: $0 <ios|android>" >&2
  exit 2
fi

platform="$1"
if [[ "$platform" == "ios" ]]; then
  target_name="iOS"
else
  target_name="Android"
fi
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixtures="$root/crates/nexa-cli/tests/fixtures"
app_fixture="$fixtures/dev_runtime_plugin_probe_app"
plugin_fixture="$fixtures/dev_runtime_plugin_probe"
nexa_bin="${NEXA_BIN:-$root/target/debug/nexa}"

if [[ ! -x "$nexa_bin" ]]; then
  (cd "$root" && cargo build -p nexa-cli)
fi
nexa_bin="$(cd "$(dirname "$nexa_bin")" && pwd)/$(basename "$nexa_bin")"

tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/nexa-dev-runtime-plugin.XXXXXX")"
native_log="$tmp_root/native.log"
dev_log="$tmp_root/dev.log"
dev_pid=""
log_pid=""
device_id=""
adb_cmd=()

print_logs() {
  echo "--- Nexa dev session ---" >&2
  tail -n 120 "$dev_log" >&2 2>/dev/null || true
  echo "--- Native runtime logs ---" >&2
  tail -n 120 "$native_log" >&2 2>/dev/null || true
}

cleanup() {
  local status=$?
  if [[ -n "$dev_pid" ]]; then
    kill -TERM "$dev_pid" 2>/dev/null || true
    wait "$dev_pid" 2>/dev/null || true
  fi
  if [[ "$platform" == "ios" && -n "$device_id" ]]; then
    xcrun simctl terminate "$device_id" dev.nexa.devprobe >/dev/null 2>&1 || true
  elif [[ "$platform" == "android" && ${#adb_cmd[@]} -gt 0 ]]; then
    "${adb_cmd[@]}" shell am force-stop dev.nexa.devprobe >/dev/null 2>&1 || true
  fi
  if [[ -n "$log_pid" ]]; then
    kill "$log_pid" 2>/dev/null || true
    wait "$log_pid" 2>/dev/null || true
  fi
  if [[ $status -eq 0 && "${NEXA_KEEP_DEV_RUNTIME_PROBE:-0}" != "1" ]]; then
    rm -rf "$tmp_root"
  else
    echo "Probe files and logs: $tmp_root" >&2
  fi
}
trap cleanup EXIT

mkdir -p "$tmp_root/probe-plugin"
cp -R "$plugin_fixture/." "$tmp_root/probe-plugin/"
cp "$app_fixture/nexa.config.nx" "$app_fixture/App.nx" "$tmp_root/"

if [[ "$platform" == "ios" ]]; then
  command -v xcrun >/dev/null || { echo "xcrun is required for the iOS probe" >&2; exit 1; }
  device_id="$(xcrun simctl list devices booted | grep -Eo '[0-9A-F-]{36}' | head -n 1 || true)"
  [[ -n "$device_id" ]] || { echo "Boot an iOS Simulator before running this probe" >&2; exit 1; }
  xcrun simctl terminate "$device_id" dev.nexa.devprobe >/dev/null 2>&1 || true
  xcrun simctl spawn "$device_id" log stream --style compact --predicate 'process == "DevRuntimeProbe"' >"$native_log" 2>&1 &
  log_pid=$!
else
  command -v adb >/dev/null || { echo "adb is required for the Android probe" >&2; exit 1; }
  adb_cmd=(adb)
  if [[ -n "${ANDROID_SERIAL:-}" ]]; then
    adb_cmd+=( -s "$ANDROID_SERIAL" )
  fi
  [[ "$("${adb_cmd[@]}" get-state 2>/dev/null || true)" == "device" ]] || {
    echo "Connect or boot an Android Emulator before running this probe" >&2
    exit 1
  }
  "${adb_cmd[@]}" shell am force-stop dev.nexa.devprobe >/dev/null 2>&1 || true
  "${adb_cmd[@]}" logcat -c
  "${adb_cmd[@]}" logcat -v time 'Nexa:I' 'NexaDevRuntime:I' '*:S' >"$native_log" 2>&1 &
  log_pid=$!
fi

wait_for_marker() {
  local file="$1"
  local marker="$2"
  local attempt=0
  while [[ $attempt -lt 180 ]]; do
    grep -Fq "$marker" "$file" 2>/dev/null && return 0
    if [[ -n "$dev_pid" ]] && ! kill -0 "$dev_pid" 2>/dev/null; then
      print_logs
      echo "Nexa dev exited before logging: $marker" >&2
      return 1
    fi
    sleep 0.5
    attempt=$((attempt + 1))
  done
  print_logs
  echo "Timed out waiting for log marker: $marker" >&2
  return 1
}

(cd "$tmp_root" && exec "$nexa_bin" dev "--$platform" --out "$tmp_root/build") >"$dev_log" 2>&1 &
dev_pid=$!

wait_for_marker "$dev_log" "Nexa $target_name dev runtime connected."
for marker in \
  DEVRT_ASYNC_NESTED_EXPRESSION_PASS \
  DEVRT_SERVICE_SYNC_PASS \
  DEVRT_SERVICE_TYPED_ERROR_PASS \
  DEVRT_NATIVE_EVENT_PASS \
  DEVRT_PROPERTY_WRITE_READ_PASS \
  DEVRT_BYTE_SET_PROPERTY_RESULT_PASS \
  DEVRT_BYTE_SET_EVENT_PASS \
  DEVRT_BYTE_SET_COMPONENT_PASS \
  DEVRT_GENERIC_SCALAR_PASS \
  DEVRT_GENERIC_OPTIONAL_ARRAY_PASS \
  DEVRT_GENERIC_OPTIONAL_VALUE_PASS \
  DEVRT_GENERIC_OPTIONAL_NULL_PASS \
  DEVRT_GENERIC_SET_PASS \
  DEVRT_GENERIC_MAP_PASS \
  DEVRT_GENERIC_PAIR_PASS \
  DEVRT_GENERIC_TRIPLE_PASS \
  DEVRT_GENERIC_BYTES_PASS \
  DEVRT_GENERIC_ENUM_PASS \
  DEVRT_GENERIC_STRUCT_PASS \
  DEVRT_GENERIC_NESTED_STRUCT_PASS \
  DEVRT_GENERIC_RESULT_PASS \
  DEVRT_TYPED_ERROR_PASS \
  DEVRT_BASE_COMPONENT_PASS; do
  wait_for_marker "$native_log" "$marker"
done

# The native host is now built and running with the plugin prelinked. Add the
# imported component only after that build, then let source watching hot-reload
# its module without a native rebuild.
cp "$app_fixture/ProbePanel.nx" "$tmp_root/ProbePanel.nx"
cp "$app_fixture/App.after.template" "$tmp_root/App.nx.next"
mv "$tmp_root/App.nx.next" "$tmp_root/App.nx"

wait_for_marker "$dev_log" "Nexa source reloaded in the running app."
wait_for_marker "$native_log" DEVRT_IMPORTED_COMPONENT_PASS

for marker in \
  DEVRT_ASYNC_NESTED_EXPRESSION_FAIL \
  DEVRT_SERVICE_TYPED_ERROR_FAIL \
  DEVRT_SERVICE_TYPED_ERROR_WRONG_CASE \
  DEVRT_GENERIC_RESULT_FAIL \
  DEVRT_TYPED_ERROR_FAIL \
  DEVRT_TYPED_ERROR_WRONG_CASE; do
  if grep -Fq "$marker" "$native_log"; then
    print_logs
    echo "DevRuntime plugin probe logged failure marker: $marker" >&2
    exit 1
  fi
done

if grep -Fq "Rebuilding native app" "$dev_log"; then
  print_logs
  echo "The imported .nx update unexpectedly requested a native rebuild" >&2
  exit 1
fi

echo "DevRuntime plugin parity probe passed on $platform."
