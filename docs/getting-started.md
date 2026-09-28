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

`nexa dev` checks the `.nx` source, generates both native hosts under `build/`, compiles them, and launches on available simulators/emulators. It stays active and watches `.nx` files; compatible UI, state, and action changes reload in the running app, including adding, renaming, or removing a custom component. Async lifecycle action blocks handle branches, loops, collection updates, and loop flow control; nested async expressions still have gaps versus AOT. Calls from reloaded `.nx` code to already-declared plugin services, plus native class constructors and instance methods, hot reload when the signature uses supported scalar or `Bytes` values. Generic and compound plugin values, native class methods with declared errors, and typed plugin error catches remain incomplete. Adding/removing a plugin declaration or changing plugin source, native interfaces, dependencies, assets, or host configuration waits for `b` to rebuild and relaunch the Dev host. Native plugin property assignments, event subscriptions, and visual components are not implemented by the Dev interpreter; use an AOT build for those features. The debug renderer supports the same `FastList` sources, axes, headers, callbacks, refresh actions, state assignments, and synchronous app-local functions accepted by the compiler. Compatible app and screen state, focus, navigation path, and list position are retained across compatible reloads. In the dev terminal, `r` hot reloads, `Shift+R` hot restarts and resets app state, `b` rebuilds and relaunches the native app, and `p` toggles an on-device overlay with live FPS and average frame time. VS Code provides matching **Nexa: Hot Reload**, **Nexa: Hot Restart**, **Nexa: Rebuild and Relaunch App**, and **Nexa: Toggle Performance Overlay** commands with keyboard shortcuts. Use `nexa dev --once` as an explicit one-shot build and launch without starting the watcher.

Use `nexa dev --compile-only` to compile the debug development host, including Nexa's hot-reload runtime, without launching it or starting the watcher. `--once` builds the normal AOT host for a one-time launch.

`nexa test` generates and compiles the native projects without launching the app. `nexa release` creates an iOS archive and exported IPA plus a signed Android AAB. Use `--ios` or `--android` to build one platform. Pass `--arch arm64` to select ARM64; Android applies the `arm64-v8a` ABI filter. Set one or multiple architectures per platform in `nexa.config.nx`, such as `ios { arch: ["arm64", "x86_64"] }` or `android { arch: ["arm64", "x86_64"] }`. A single string like `arch: "arm64"` remains accepted. iOS supports simulator `x86_64`; Android supports `armv7`, `x86`, and `x86_64` in addition to `arm64`. Add `--flavor staging` to `dev`, `test`, or `release` for a separate app identity such as `dev.nexa.myapp.staging`; `--staging` is a shorthand. Define flavors in `nexa.config.nx`, for example `flavors { staging { suffix: "staging" }, production { suffix: "" } }`. A missing suffix defaults to the flavor name, while an empty suffix keeps the configured base app ID. `nexa doctor` checks the required toolchains.

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
    android { minSdk: 24, targetSdk: 36, applicationId: "dev.example.hello" }
    permissions {}
}
```

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
