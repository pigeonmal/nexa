# Nexa Release Checklist

[![Release scope](https://img.shields.io/badge/scope-compiler%20%7C%20iOS%20%7C%20Android-blue.svg)](../README.md)

Run this checklist before publishing a Nexa release. It covers compiler behavior, both native targets, plugins, DevRuntime, generated references, and migration notes.

## Quick start

```sh
export CARGO_TARGET_DIR=/tmp/nexa-verification-target
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Release gates

| Area | Required check | Pass condition |
|---|---|---|
| Rust workspace | The Quick start commands above | Formatting, all targets, lint, and tests pass. |
| Generated references | `cargo test -p nexa-syntax` and the applicable documentation tests | Catalogs and generated Markdown match compiler-owned data. |
| Swift AOT | `nexa test --ios` | The generated iOS host compiles and app tests pass. |
| Kotlin AOT | `nexa test --android` | The generated Android host compiles and app tests pass. |
| DevRuntime | Run the paired AOT/DevRuntime check on iOS and Android | Both hosts compile, reload the fixture, and match expected interactions. |
| Release isolation | `cargo test -p nexa-cli --test dev_runtime_command development_runtime_is_debug_only_and_removed_by_aot_regeneration` | AOT regeneration removes DevRuntime sources and plugin bridges. |
| Plugins | `cargo test -p nexa-cli --test native_plugin_build` and `cargo test -p nexa-codegen` | Plugin contracts and generated Swift, Kotlin, and supported C++ adapters pass. |
| Performance | Run [`tests/benchmark-suite.py`](../tests/benchmark-suite.py), the hot-reload latency check, and profile `examples/virtual_list.nx` on both targets when compiler, list, or rendering paths change. | Keep the JSON report with commit, device, OS, and toolchain details; compare only matching environments and inspect runtime frame time and allocations. |
| Cache compatibility | `cargo test -p nexa-cli --test cache_audit` | The cache version is documented when output-affecting behavior changes. |
| Migration notes | Review public syntax, component parameters, API signatures, and plugin contracts changed in the release | Every breaking change has a direct old-to-new example and a compiler diagnostic. |
| Release artifacts | Run `nexa release --all` with the release signing credentials | The signed iOS archive and Android bundle build from the release commit. |

## Device parity

For a release that changes shared UI or interaction behavior, run
[`tests/hot-reload-aot-parity.sh`](../tests/hot-reload-aot-parity.sh) on the supported iOS Simulator and Android emulator. Confirm that the same route, values, accessibility labels, and actions work in AOT and DevRuntime before collecting screenshots.

Record platform version, device model, route, and any remaining platform-specific behavior in the release notes. Keep human visual approval separate from automated Agent Review evidence.
