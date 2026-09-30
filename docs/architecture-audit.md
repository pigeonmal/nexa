# Architecture Audit Notes

This file records generated-source cache schema changes that must invalidate
previous compiler output.

## Cache schema

- `build-v128`: adds typed screen orientation requests to both AOT backends and DevRuntime, with iOS scene geometry support and generated interface-orientation declarations.
- `build-v127`: preserves valid nullable values while decoding generic plugin returns in Android DevRuntime, including nullable elements in compound values.
- `build-v125`: adds configurable Pressable long-press timing to both native backends and DevRuntime, accepts `.onTap` as the tap-handler spelling, and advances the Dev IR format.
- `build-v124`: adds typed Pressable pinch scale callbacks to both native backends and DevRuntime, and advances the Dev IR format.
- `build-v123`: adds typed Pressable drag callbacks to both native backends and DevRuntime, and advances the Dev IR format.
- `build-v122`: adds typed app-private string storage to both native backends and DevRuntime, and emits iOS required-reason privacy metadata for generated UserDefaults and file-metadata APIs.
- `build-v121`: adds statically generated JSON codecs to AOT apps and typed JSON dispatch/codecs to both DevRuntime hosts.
- `build-v120`: decodes nullable generic Android plugin collection inputs before dispatch.
- `build-v119`: makes Android launcher orientation follow the user's current orientation policy at startup.
- `build-v118`: adds typed Haptics calls to both native backends and DevRuntime dispatch.
- `build-v117`: fixes recursive Kotlin DevRuntime decoding for optional generic plugin arguments.
- `build-v116`: adds the typed iOS and Android system text clipboard API and DevRuntime dispatch.
- `build-v115`: emits Swift and Kotlin read codecs for optional values nested inside compound generic plugin values; Kotlin tracks decode failure separately from a valid null value.
- `build-v114`: includes the SecureStorage native adapters in every DevRuntime host so the API remains available after hot reload.
- `build-v113`: adds typed core cryptographic helpers to both native backends and DevRuntime.
- `build-v112`: supports optional generic plugin input codecs and parses explicit type arguments on qualified service calls.
- `build-v111`: adds locale-aware `Number.formatCurrency` to the iOS and Android AOT backends and DevRuntime.
- `build-v110`: adds cross-platform generic plugin codecs for `Result` values with enum failures, including hot-reload value adapters.
- `build-v109`: generates keyboard autofill and return-key semantics, plus `Keyboard.dismiss()` support in release apps and DevRuntime; advances the Dev IR format.
- `build-v108`: adds optional-element array codecs to DevRuntime plugin adapters for methods, properties, events, component props, and component events.
- `build-v107`: adds typed conditional view transitions to the source IR, SwiftUI and Compose output, and both DevRuntime renderers; advances the Dev IR and development protocol versions.
- `build-v104`: adds Pressable double-tap actions to typed IR, both native backends, and both Dev renderers; advances the Dev IR format version.
- `build-v103`: returns success after rendering a matched Android DevRuntime plugin visual component, so the renderer no longer reports a rendered component as unsupported.
- `build-v102`: adds `Pair` and `Triple` support to generated plugin value codecs and the DevRuntime plugin bridge, and fixes tuple member access in async DevRuntime evaluation.
- `build-v101`: moves source accessibility annotations to optional named arguments on rendered built-ins, custom components, and native plugin components; updates typed lowering while retaining the existing accessibility IR node and Dev IR format.
- `build-v100`: adds static visual effects to typed layout and text styles, both native backends, and both Dev renderers; advances the Dev IR format version.
- `build-v99`: adds a native menu-style Picker and Dev IR/runtime rendering on both platforms; advances the development protocol and format versions.
- `build-v98`: adds native string-backed SegmentedControl lowering and Dev IR/rendering on both platforms; advances the development protocol and format versions.
- `build-v97`: adds typed native Dialog alerts and Dev IR/runtime rendering on both platforms; advances the development protocol and format versions.
- `build-v96`: adds native Float64 Slider and progress controls, extends both development runtimes with their state/expression handling, and advances Dev IR and protocol versions.
- `build-v95`: added typed arithmetic IR and native lowering for subtraction,
  multiplication, division, remainder, and unary negation; added string
  concatenation lowering and native collection `count`/`isEmpty` accessors;
  extended both dev interpreters and advanced the Dev IR and WebSocket protocol
  versions.
- `build-v94`: iterates Compose state collections through immutable snapshots.
- `build-v93`: binds Android runtime context before native plugin initialization.
- `build-v92`: includes development host File and Path helpers for hot reload.
