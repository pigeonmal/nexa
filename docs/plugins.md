# Native Plugins

Nexa plugins add statically typed native APIs through a package manifest and a `.nxid` contract. Swift and Kotlin bindings use direct generated calls; optional C++ bindings are available for a narrower set of supported method and value shapes.

| **Scope**: consume and author native plugins | **Contract**: `native.nxid` | **Targets**: Swift and Kotlin |

| Package authoring | App use | Native targets |
|---|---|---|
| `native.nxid` + `plugin.config.nx` | `plugin "path" as Namespace` | Swift and Kotlin; optional C++ |

## Quick start: use a local plugin

This app example is from the official biometrics demo. The `plugin` path is relative to `App.nx` and should point to the checked-out package. Biometrics requires Android API 28 or later; set `android.minSdk` to at least `28` in `nexa.config.nx`:

```nx
config {
    android { minSdk: 28 }
}
```

```nx
plugin "plugins/biometrics" as Biometrics

app VaultUnlock {
    state authenticated: Bool = false
    state failed: Bool = false

    body {
        Column(spacing: 12, padding: 20) {
            Text("Unlock the private vault", fontSize: 22, fontWeight: Bold)
            Biometrics.BiometricButton(
                title: "Authenticate",
                reason: "Confirm your identity to view saved credentials"
            )
                .onAuthenticated {
                    authenticated = true
                    failed = false
                }
                .onFailed { error ->
                    authenticated = false
                    failed = true
                }
            if authenticated { Text("Vault unlocked") }
            if failed { Text("Authentication was not completed") }
        }
    }
}
```

There is no plugin registry or `nexa plugin add` command. Keep the package in a local checkout and point the app at its directory, or declare it through the app's resolved local dependencies.

## Plugin package files

`nexa plugin new` creates the deterministic starter files; `nexa plugin init` remains an alias. A native package contains a `native.nxid` contract and platform implementations; a pure package contains reusable `.nx` source and no native IDL.

```text
sensor-plugin/
├── plugin.config.nx
├── native.nxid
├── ios/Sources/SensorImpl.swift
└── android/src/main/kotlin/dev/nexa/plugin/SensorImpl.kt
```

| Command | Purpose |
|---|---|
| `nexa plugin new dev.example.sensor --out plugins/sensor --name Sensor` | Create a native plugin package scaffold. |
| `nexa plugin new dev.example.formatters --out plugins/formatters --kind pure` | Create a Nexa-only package with `plugin.nx`. |
| `nexa plugin check plugins/sensor` | Validate the package manifest, IDL, source graph, and available native implementation checks. |
| `nexa plugin generate plugins/sensor --target swift --out generated/swift` | Emit a Swift binding contract. Use `kotlin` or `cpp` for the other supported outputs. |

`nexa plugin check` runs syntax and contract validation; native type-checking depends on the installed platform tools and package dependencies. C++ output creates a typed contract; it does not mean every API shape has a generated host adapter.

## `.nxid` contract declarations

The contract is the app-facing API and the implementation boundary. It can declare value structs, closed enums, typed errors, stateless services, stateful native classes, and native components. For example, the current biometrics contract is:

```nxid
enum BiometricFailure {
    notAvailable
    notEnrolled
    lockout
    userCanceled
    systemCanceled
    authenticationFailed
    passcodeNotSet
    invalidContext
    unknown
}

native component BiometricButton {
    prop title: String
    prop reason: String
    event authenticated()
    event failed(error: BiometricFailure)
}
```

`native component` properties become named arguments on `Namespace.Component(...)`. Events become `.onEvent { ... }` modifiers. `service` declarations provide stateless calls; `native class` declarations provide constructed native objects, properties, methods, and instance-scoped events. Use native classes when object identity or explicit disposal is part of the API. Concrete class and service examples live in the [official plugin contracts](../plugins/).

### Async throwing calls

Async throwing plugin calls require explicit `try { ... } catch { ... }` recovery. Catch cases and payloads must match that plugin's declared error variants; use the package's checked API reference and working demo as the source of exact method names and argument types. The [In-App Purchases demo](../plugins/in-app-purchases/tests/demo/app/App.nx) shows typed error recovery.

## Package manifest: `plugin.config.nx`

The manifest uses schema version `2` and records package identity, declared source roots, native platform requirements, and optional tooling.

| Field | Type | Meaning |
|---|---|---|
| `schema` | integer | Must be `2`. |
| `id` | string | Non-empty dot-separated plugin identifier. |
| `version` | string | Non-empty package version. |
| `sources.native` | relative path | `.nxid` contract file. |
| `sources.nexa` | relative path | Reusable source file for a pure Nexa package. |
| `assets` | string array | Package-owned assets copied to generated projects. |
| `compiler.analyzer` | string array | Optional analyzer executable and arguments, launched from the package root without a shell. |
| `ios` | platform fields | iOS minimum version, sources, extension sources, frameworks, XCFrameworks, Swift packages, resources, privacy manifest, usage strings, entitlements, delegate, background modes, and linker flags. |
| `android` | platform fields | Minimum SDK, sources, resources, AARs, Maven dependencies and repositories, ProGuard rules, permissions, application metadata, PiP, and service declarations. |
| `cpp` | platform fields | C++ standard (`c++17`, `c++20`, or `c++23`), source files, and headers. |

An iOS Swift package under `ios.dependencies` declares `url`, `products`, optional `extensionProducts`, and exactly one version selector: `from`, `branch`, or `revision`. Android Maven dependencies use `android.dependencies`; they are not app-level registry package identifiers.

Starter manifest:

```nexa-manifest
plugin {
    schema: 2
    id: "dev.example.sensor"
    version: "0.1.0"
    sources { native: "native.nxid" }
    ios {
        minVersion: "17.0"
        sources: ["ios/Sources/**/*.swift"]
    }
    android {
        minSdk: 26
        sources: ["android/src/main/kotlin/**/*.kt"]
    }
}
```

Use the scaffold as the exact template for new packages; `nexa plugin check` rejects unknown fields, invalid paths, missing sources, and unsupported metadata values.

## Native implementation and lifecycle

The code generator emits typed Swift and Kotlin contracts and uses direct plugin factories. Implement the generated contract in the package's platform source tree and keep platform-only SDK types out of `.nxid` values.

| Concern | iOS | Android |
|---|---|---|
| Call boundary | Generated Swift contract; native classes are main-actor isolated. | Generated Kotlin contract; method work stays on the caller's coroutine dispatcher. |
| Async work | Swift concurrency. | Kotlin suspend functions; Nexa does not silently move plugin work to another dispatcher. |
| Ownership | Give long-lived native classes an explicit `dispose()` contract when needed. | Release listeners, callbacks, and resources from the same explicit lifecycle. |
| C++ | Optional Objective-C++/Swift adapter for supported signatures. | Optional generated JNI adapter for supported signatures. |

Do not assume all Swift/Kotlin method signatures map to C++. The generated C++ adapter currently supports a smaller set of scalar, optional scalar, string, bytes, selected collection, and typed-error cases.

### Borrowed C++ byte inputs

Use `BufferView` for a large read-only byte input that a synchronous C++ method consumes before returning:

```nxid
service FrameDecoder {
    fn decode(frame: BufferView) -> Int32
}
```

`BufferView` is valid only as a direct parameter on a synchronous method. The compiler rejects it in stored fields, properties, events, asynchronous methods, and return values because the pointer cannot outlive the call. The Nexa app still passes its ordinary `Bytes` value.

| Target | Generated boundary | Copy behavior |
|---|---|---|
| iOS | Swift holds `Data.withUnsafeBytes` for the duration of the call; C++ receives `{data, size}`. | No intermediate `std::vector` allocation. |
| Android | Kotlin stages the `ByteArray` into a direct `ByteBuffer`; JNI reads its direct address and capacity; C++ receives `{data, size}`. | One copy into direct memory; no `jbyteArray` extraction into a second C++ vector. |

Treat the view as read-only, do not retain its pointer, and finish all access before the method returns. The Android staging copy is required because Nexa's source-level `Bytes` is an owned `ByteArray`; a future native producer that already owns direct memory can avoid that copy.

## First-party platform requirements

Every value in the three tables below is read from that package's `plugin.config.nx`, so a
package raising its deployment target updates them on regeneration. Run
`cargo test -p nexa-cli --test plugin_reference` with `NEXA_UPDATE_SNAPSHOTS=1` after changing
a manifest. `nexa create` validates an app's `android.minSdk` against these floors, so an app
below a plugin's minimum fails at build time rather than at runtime.

### Deployment targets and permissions

A permission or purpose string listed here is one the plugin's generated native code declares,
which means the app's store listing and `Info.plist` need a matching user-facing message.

<!-- nexadoc:begin native-requirements -->
| Plugin | iOS minimum | Android `minSdk` | Android permissions | iOS usage descriptions |
|---|---|---|---|---|
| **Audio Player** | `17.0` | `26` (API 26) | `android.permission.INTERNET`, `android.permission.WAKE_LOCK` | `—` |
| **Biometrics** | `13.0` | `28` (API 28) | `android.permission.USE_BIOMETRIC` | `NSFaceIDUsageDescription`: Use Face ID to confirm your identity. |
| **Browser** | `16.0` | `23` (API 23) | `—` | `—` |
| **Camera** | `17.0` | `23` (API 23) | `android.permission.CAMERA`, `android.permission.RECORD_AUDIO` | `NSCameraUsageDescription`: Use the camera features you open in this app.<br>`NSMicrophoneUsageDescription`: Record audio when you start a camera video recording with audio enabled. |
| **Data Extractor** | `13.0` | `28` (API 28) | `—` | `—` |
| **In-App Purchases** | `17.0` | `23` (API 23) | `—` | `—` |
| **Mail Composer** | `16.0` | `23` (API 23) | `—` | `—` |
| **Maps** | `17.0` | `23` (API 23) | `android.permission.INTERNET` | `—` |
| **Media Picker** | `16.0` | `23` (API 23) | `—` | `—` |
| **MMKV** | `13.0` | `21` (API 21) | `—` | `—` |
| **Notifications** | `13.0` | `23` (API 23) | `android.permission.INTERNET`, `android.permission.ACCESS_NETWORK_STATE`, `android.permission.POST_NOTIFICATIONS` | `—` |
| **Sensors** | `17.0` | `26` (API 26) | `android.permission.ACTIVITY_RECOGNITION` | `NSMotionUsageDescription`: Motion and barometer data is used only for the sensor features you start. |
| **SQLite** | `13.0` | `23` (API 23) | `—` | `—` |
| **Video Player** | `17.0` | `23` (API 23) | `android.permission.INTERNET` | `—` |
| **Websocket** | `13.0` | `21` (API 21) | `android.permission.INTERNET` | `—` |
| **Webview** | `17.0` | `24` (API 24) | `android.permission.INTERNET` | `—` |
<!-- nexadoc:end native-requirements -->

### Entitlements, background modes, and services

These decide whether a plugin links and runs on a real device: a missing entitlement or
background mode is a launch-time failure on the platform, not a compile error.

<!-- nexadoc:begin native-integration -->
| Plugin | iOS entitlements | iOS background modes | App delegate | Android services | Android metadata |
|---|---|---|---|---|---|
| **Audio Player** | `—` | `audio` | `—` | `dev.nexa.audio.AudioPlaybackService` | `—` |
| **Biometrics** | `—` | `—` | `—` | `—` | `—` |
| **Browser** | `—` | `—` | `—` | `—` | `—` |
| **Camera** | `—` | `—` | `—` | `—` | `—` |
| **Data Extractor** | `—` | `—` | `—` | `—` | `—` |
| **In-App Purchases** | `—` | `—` | `—` | `—` | `—` |
| **Mail Composer** | `—` | `—` | `—` | `—` | `—` |
| **Maps** | `—` | `—` | `—` | `—` | `com.google.android.geo.API_KEY`: ${NEXA_MAPS_API_KEY} |
| **Media Picker** | `—` | `—` | `—` | `—` | `—` |
| **MMKV** | `—` | `—` | `—` | `—` | `—` |
| **Notifications** | `aps-environment`: development | `—` | `NotificationsAppDelegate` | `dev.nexa.notifications.NotificationsFirebaseMessagingService` | `—` |
| **Sensors** | `—` | `—` | `—` | `—` | `—` |
| **SQLite** | `—` | `—` | `—` | `—` | `—` |
| **Video Player** | `—` | `—` | `—` | `—` | `—` |
| **Websocket** | `—` | `—` | `—` | `—` | `—` |
| **Webview** | `—` | `—` | `—` | `—` | `—` |
<!-- nexadoc:end native-integration -->

### Native dependency coordinates

Swift packages and Maven artifacts the plugin generator links into the generated host.

<!-- nexadoc:begin native-dependencies -->
| Plugin | Swift packages | Maven dependencies | iOS frameworks | Linker flags |
|---|---|---|---|---|
| **Audio Player** | `—` | `androidx.media3:media3-exoplayer:1.11.1`, `androidx.media3:media3-exoplayer-hls:1.11.1`, `androidx.media3:media3-session:1.11.1` | `AVFoundation`, `MediaPlayer` | `—` |
| **Biometrics** | `—` | `—` | `LocalAuthentication` | `—` |
| **Browser** | `—` | `androidx.browser:browser:1.10.0` | `SafariServices`, `UIKit` | `—` |
| **Camera** | `—` | `androidx.camera:camera-camera2:1.6.2`, `androidx.camera:camera-lifecycle:1.6.2`, `androidx.camera:camera-view:1.6.2`, `androidx.camera:camera-video:1.6.2`, `com.google.mlkit:barcode-scanning:17.3.0` | `AVFoundation` | `—` |
| **Data Extractor** | `—` | `—` | `—` | `—` |
| **In-App Purchases** | `—` | `com.android.billingclient:billing:9.1.0` | `StoreKit` | `—` |
| **Mail Composer** | `—` | `—` | `MessageUI`, `UIKit` | `—` |
| **Maps** | `—` | `com.google.maps.android:maps-compose:8.6.0` | `MapKit` | `—` |
| **Media Picker** | `—` | `—` | `PhotosUI`, `UniformTypeIdentifiers` | `—` |
| **MMKV** | `https://github.com/Tencent/MMKV.git` (from `2.4.2`) | `io.github.zhongwuzw:mmkv:2.4.2` | `—` | `—` |
| **Notifications** | `—` | `androidx.work:work-runtime-ktx:2.11.2`, `androidx.core:core-ktx:1.17.0`, `com.google.firebase:firebase-messaging:25.1.3` | `UserNotifications`, `UIKit` | `—` |
| **Sensors** | `—` | `—` | `CoreMotion` | `—` |
| **SQLite** | `—` | `—` | `—` | `-lsqlite3` |
| **Video Player** | `—` | `androidx.media3:media3-exoplayer:1.11.1`, `androidx.media3:media3-datasource:1.11.1`, `androidx.media3:media3-database:1.11.1`, `androidx.media3:media3-exoplayer-hls:1.11.1`, `androidx.media3:media3-exoplayer-dash:1.11.1`, `androidx.media3:media3-datasource-cronet:1.11.1`, `androidx.media3:media3-ui:1.11.1`, `org.chromium.net:cronet-embedded:143.7445.0` | `AVFoundation` | `—` |
| **Websocket** | `—` | `com.squareup.okhttp3:okhttp:5.5.0` | `—` | `—` |
| **Webview** | `—` | `androidx.webkit:webkit:1.17.1` | `WebKit` | `—` |
<!-- nexadoc:end native-dependencies -->

## Optional compiler analyzer

A package may declare `compiler.analyzer` to run domain-specific compile-time checks. Nexa sends a versioned JSON request to the executable and reads one JSON response; stdout is reserved for that response, and diagnostics can include severity, source span, and target. SQLite uses this hook to validate statically known SQL in `.nx` calls.

Host integrations can also opt into the in-process `nexa-plugin-compiler-api` hooks after semantic analysis. `inspect_node` receives typed component nodes, and `inspect_call` receives resolved native-call signatures, including the optional `source_span`. Use `ExtensionContext.report_call_error(call, message)` to attach a diagnostic to that source call; `finalize` runs once after traversal. This API is registered by the compiler host and is separate from the executable declared by `compiler.analyzer`.

## Further reading

- [Official plugin catalog and APIs](../plugins/README.md)
- [SQLite plugin: typed rows, migrations, and reactive queries](../plugins/sqlite/README.md)
- [Plugin IDL parser and manifest source](../crates/nexa-plugin-idl/src/)
