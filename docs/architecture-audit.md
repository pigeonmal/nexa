# Architecture Audit Notes

This file records generated-source cache schema changes that must invalidate
previous compiler output.

## Cache schema

| Generator Schema | Architectural Change / IR Lowering |
|---|---|
| `build-v131` | Adds typed foreground task launch/cancellation actions, native executor lowering, and task lifecycle cleanup in both AOT backends and DevRuntime. |
| `build-v130` | Adds typed optional array `first()` and `last()` utilities to both AOT backends and DevRuntime; advances the Dev IR format. |
| `build-v129` | Generates periodic OS background-task handlers and scheduling metadata for iOS and Android; advances the Dev IR format for task declarations. |
| `build-v128` | Adds typed screen orientation requests to both AOT backends and DevRuntime, with iOS scene geometry support and generated interface-orientation declarations. |
| `build-v127` | Preserves valid nullable values while decoding generic plugin returns in Android DevRuntime, including nullable elements in compound values. |
| `build-v125` | Adds configurable Pressable long-press timing to both native backends and DevRuntime, accepts `.onTap` as the tap-handler spelling, and advances the Dev IR format. |
| `build-v124` | Adds typed Pressable pinch scale callbacks to both native backends and DevRuntime, and advances the Dev IR format. |
| `build-v123` | Adds typed Pressable drag callbacks to both native backends and DevRuntime, and advances the Dev IR format. |
| `build-v122` | Adds typed app-private string storage to both native backends and DevRuntime, and emits iOS required-reason privacy metadata for generated UserDefaults and file-metadata APIs. |
| `build-v121` | Adds statically generated JSON codecs to AOT apps and typed JSON dispatch/codecs to both DevRuntime hosts. |
| `build-v120` | Decodes nullable generic Android plugin collection inputs before dispatch. |
| `build-v119` | Makes Android launcher orientation follow the user's current orientation policy at startup. |
| `build-v118` | Adds typed Haptics calls to both native backends and DevRuntime dispatch. |
| `build-v117` | Fixes recursive Kotlin DevRuntime decoding for optional generic plugin arguments. |
| `build-v116` | Adds the typed iOS and Android system text clipboard API and DevRuntime dispatch. |
| `build-v115` | Emits Swift and Kotlin read codecs for optional values nested inside compound generic plugin values; Kotlin tracks decode failure separately from a valid null value. |
| `build-v114` | Includes the SecureStorage native adapters in every DevRuntime host so the API remains available after hot reload. |
| `build-v113` | Adds typed core cryptographic helpers to both native backends and DevRuntime. |
| `build-v112` | Supports optional generic plugin input codecs and parses explicit type arguments on qualified service calls. |
| `build-v111` | Adds locale-aware `Number.formatCurrency` to the iOS and Android AOT backends and DevRuntime. |
| `build-v110` | Adds cross-platform generic plugin codecs for `Result` values with enum failures, including hot-reload value adapters. |
| `build-v109` | Generates keyboard autofill and return-key semantics, plus `Keyboard.dismiss()` support in release apps and DevRuntime; advances the Dev IR format. |
| `build-v108` | Adds optional-element array codecs to DevRuntime plugin adapters for methods, properties, events, component props, and component events. |
| `build-v107` | Adds typed conditional view transitions to the source IR, SwiftUI and Compose output, and both DevRuntime renderers; advances the Dev IR and development protocol versions. |
| `build-v104` | Adds Pressable double-tap actions to typed IR, both native backends, and both Dev renderers; advances the Dev IR format version. |
| `build-v103` | Returns success after rendering a matched Android DevRuntime plugin visual component, so the renderer no longer reports a rendered component as unsupported. |
| `build-v102` | Adds `Pair` and `Triple` support to generated plugin value codecs and the DevRuntime plugin bridge, and fixes tuple member access in async DevRuntime evaluation. |
| `build-v101` | Moves source accessibility annotations to optional named arguments on rendered built-ins, custom components, and native plugin components; updates typed lowering while retaining the existing accessibility IR node and Dev IR format. |
| `build-v100` | Adds static visual effects to typed layout and text styles, both native backends, and both Dev renderers; advances the Dev IR format version. |
| `build-v99` | Adds a native menu-style Picker and Dev IR/runtime rendering on both platforms; advances the development protocol and format versions. |
| `build-v98` | Adds native string-backed SegmentedControl lowering and Dev IR/rendering on both platforms; advances the development protocol and format versions. |
| `build-v97` | Adds typed native Dialog alerts and Dev IR/runtime rendering on both platforms; advances the development protocol and format versions. |
| `build-v96` | Adds native Float64 Slider and progress controls, extends both development runtimes with their state/expression handling, and advances Dev IR and protocol versions. |
| `build-v95` | Added typed arithmetic IR and native lowering for subtraction, multiplication, division, remainder, and unary negation; added string concatenation lowering and native collection `count`/`isEmpty` accessors; extended both dev interpreters and advanced the Dev IR and WebSocket protocol versions. |
| `build-v94` | Iterates Compose state collections through immutable snapshots. |
| `build-v93` | Binds Android runtime context before native plugin initialization. |
| `build-v92` | Includes development host File and Path helpers for hot reload. |
