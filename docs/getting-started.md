# Getting Started with Nexa

Nexa projects keep one `.nx` entrypoint and one `nexa.config.nx` source of truth. Generated Xcode and Gradle projects live under `build/`.

## Requirements

- Rust 1.85 or later and the Nexa CLI
- macOS with Xcode and an installed iOS Simulator runtime for iOS builds
- Android SDK, JDK 17, and an emulator for Android builds (the generated Gradle wrapper downloads the pinned Gradle version)

Build and install the CLI from a Nexa checkout:

```bash
cargo install --path crates/nexa-cli
nexa --version
```

## Create and run a project

```bash
nexa create HelloWorld
cd HelloWorld
nexa check
nexa dev
```

`nexa check` type-checks the project for both targets before invoking Xcode or
Gradle. It also resolves configured plugin dependencies and updates
`nexa.lock`. Use `nexa check --locked` to verify that the lockfile already
matches the configured plugin packages without updating it.

`nexa dev` checks the `.nx` source, generates both native hosts under `build/`, compiles them, and launches on available simulators/emulators. It stays active and watches `.nx` files; compatible UI, state, actions, custom component changes, and navigation reload in the running app. Async expressions are recursively evaluated in lifecycle actions, including nested app-local calls, plugin calls, built-in async APIs, collection construction and transforms, and struct constructors. Calls from reloaded `.nx` code reach configured plugin services and native class constructors or methods with supported signatures, including recursively composed arrays, sets, maps, pairs, triples, and declared structs. Generic plugin codecs support scalar, `Bytes`, enum, struct, `Result` with enum failures, array, set, map, pair, and triple values. Native class property reads and writes, event subscriptions, and native visual components hot reload for supported scalar, `Bytes`, enum, optional, native-class-reference, array, set, map, pair, triple, and struct values; arrays with optional elements are supported by these DevRuntime adapters. New event subscriptions in app or active-screen `OnAppear` actions are installed after reload without replaying the lifecycle action; existing subscriptions rebind to the new action body. Declared throwing plugin calls reach typed catches, including errors with compound payloads. Optional generic inputs and values nested inside compound generic values preserve null separately from decode failures. A plugin method returning `T?` transports a missing value with the Dev host null sentinel when `T` is nonnullable. A generic read with a nullable type argument remains rejected because Kotlin cannot preserve the nested optional separately from reader decode failure. Adding or removing configured plugins, or changing plugin source, interfaces, dependencies, assets, or native host configuration requires `b` to rebuild and relaunch the Dev host. The debug renderer supports the same `FastList` sources, axes, headers, callbacks, refresh actions, state assignments, and synchronous app-local functions accepted by the compiler. Compatible app and screen state, focus, navigation path, and list position are retained across compatible reloads. In the dev terminal, `r` hot reloads, `Shift+R` hot restarts and resets app state, `b` rebuilds and relaunches the native app, and `p` toggles an on-device overlay with live FPS and average frame time. VS Code provides matching **Nexa: Hot Reload**, **Nexa: Hot Restart**, **Nexa: Rebuild and Relaunch App**, and **Nexa: Toggle Performance Overlay** commands with keyboard shortcuts. Use `nexa dev --once` as an explicit one-shot build and launch without starting the watcher.

Use `nexa dev --compile-only` to compile the debug development host, including Nexa's hot-reload runtime, without launching it or starting the watcher. `--once` builds the normal AOT host for a one-time launch.

`nexa test` runs top-level `.nx` test blocks, then generates and compiles the native projects without launching the app. Use `nexa test --unit-only` to run the in-language tests without native toolchains. `nexa release` creates an iOS archive and exported IPA plus a signed Android AAB. Use `--ios` or `--android` to build one platform. Pass `--arch arm64` to select ARM64; Android applies the `arm64-v8a` ABI filter. Set one or multiple architectures per platform in `nexa.config.nx`, such as `ios { arch: ["arm64", "x86_64"] }` or `android { arch: ["arm64", "x86_64"] }`. A single string like `arch: "arm64"` remains accepted. iOS supports simulator `x86_64`; Android supports `armv7`, `x86`, and `x86_64` in addition to `arm64`. Add `--flavor staging` to `dev`, `test`, or `release` for a separate app identity such as `dev.nexa.myapp.staging`; `--staging` is a shorthand. Define flavors in `nexa.config.nx`, for example `flavors { staging { suffix: "staging" }, production { suffix: "" } }`. A missing suffix defaults to the flavor name, while an empty suffix keeps the configured base app ID. `nexa doctor` checks the required toolchains.

Use `nexa check --audit` to include a JSON report of reachable native capabilities, generated source size, dependencies, and compiler warnings. `nexa audit App.nx --target all` prints that report directly; pass `--out audit.json` to save it. Add `--release-sizes` to build isolated Release hosts and measure the resulting iOS app bundle, Android APK, and Android App Bundle. Release measurements require the corresponding native toolchains.

Android releases read signing settings from `.nexa/signing.properties`, which `nexa create` adds as a Git-ignored local file (owner-readable on Unix). Fill it in once:

```properties
NEXA_ANDROID_KEYSTORE=keys/release.jks
NEXA_ANDROID_KEY_ALIAS=release
NEXA_ANDROID_STORE_PASSWORD=your-store-password
NEXA_ANDROID_KEY_PASSWORD=your-key-password
```

The keystore path may be absolute or relative to the project directory, and must identify an existing file. Nexa validates the settings before building, passes them to Gradle only for the release build, and verifies the generated AAB signature before reporting success. Environment variables with the same names override the local file, so CI can continue to provide credentials through its secret store. Keep signing secrets out of `nexa.config.nx` and source control.

Declare local or Git-pinned native plugin packages under `dependencies` in
`nexa.config.nx`, then use each package ID in a `plugin "package.id" as Alias`
declaration. Git dependencies require a full commit hash; Nexa records the
resolved package sources and content hashes in `nexa.lock`. Use `--locked` with
`nexa dev`, `nexa test`, or `nexa release` to reject missing or stale lockfiles.

## Project layout

```text
HelloWorld/
├── App.nx
├── nexa.config.nx
├── assets/
└── build/                 # generated; do not edit
    ├── ios/
    └── android/
```

Edit `App.nx` for app UI and `nexa.config.nx` for app identity, SDK versions, permissions, native plugin options, and assets. Generated Swift and Kotlin are build outputs.

## Configure app identity and assets

The scaffold defaults to iOS 16.0, Android minSdk 24, and Android targetSdk 36. Keep the app ID unique before distributing the app:

```nexa
config {
    app { displayName: "Hello World", version: "1.0.0", buildNumber: 1 }
    ios { minVersion: "16.0", bundleIdentifier: "dev.example.hello" }
    android {
        minSdk: 24,
        targetSdk: 36,
        applicationId: "dev.example.hello",
        cronet { provider: "play-services", diskCacheSizeMb: 64 }
    }
    permissions {}
}
```

Android Cronet defaults to the Play Services provider, so release builds do
not package the embedded Cronet native library. Set `provider: "embedded"` to
package it for devices without Google Play Services. `diskCacheSizeMb` sets
the shared network and remote-image disk cache; `0` disables that cache.

To derive launcher icons for both platforms and splash artwork from one PNG, JPEG, or WebP source:

```nexa
config {
    assets { icon: "assets/app-icon.png", splash: "assets/splash.png" }
    ios { icon: "" }
    android { icon: "" }
}
```

The shared icon source generates an iOS AppIcon asset and Android density icons, adaptive and round launcher icons, and a monochrome adaptive layer for themed icons. Platform-specific `ios.icon` or `android.icon` paths override the shared source; iOS accepts an Xcode asset-catalog icon directory or an Icon Composer `.icon` asset, while Android accepts a resource directory. iOS signing remains managed by Xcode; Android signing secrets belong in `.nexa/signing.properties` locally or CI secrets, never in `nexa.config.nx`.

## Example app

```nexa
app HelloWorld {
    state count: Int32 = 0

    body {
        Column(spacing: 16) {
            Text("Welcome to Nexa")
            Text("Count: $count")
            Button("Add one") {
                count = count + 1
            }
        }
    }
}
```

In `.nx` strings, interpolate a name with `$count` or an expression with
`\(count + 1)`. `${count}` is Kotlin's generated-code spelling; it is not the
Nexa source syntax.

For platform setup issues, run `nexa doctor`.
