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

`nexa dev` checks the `.nx` source, generates both native hosts under `build/`, compiles them, and launches on available simulators/emulators. It stays active and watches `.nx` files; compatible UI and state changes reload in the running app. Plugin, dependency, asset, and host-configuration changes wait for the user to press `b`; they never force a rebuild or relaunch. The debug renderer supports the same `FastList` sources, axes, headers, callbacks, refresh actions, state assignments, and synchronous app-local functions accepted by the compiler. Compatible app and screen state, focus, navigation path, and list position are retained across compatible reloads. In the dev terminal, `r` hot reloads, `Shift+R` hot restarts and resets app state, `b` rebuilds and relaunches the native app, and `p` toggles an on-device overlay with live FPS and average frame time. VS Code provides matching **Nexa: Hot Reload**, **Nexa: Hot Restart**, **Nexa: Rebuild and Relaunch App**, and **Nexa: Toggle Performance Overlay** commands with keyboard shortcuts. Use `nexa dev --once` as an explicit one-shot build and launch without starting the watcher.

Use `nexa dev --compile-only` to compile the debug development host, including Nexa's hot-reload runtime, without launching it or starting the watcher. `--once` builds the normal AOT host for a one-time launch.

`nexa test` generates and compiles the native projects without launching the app. `nexa release` creates an iOS archive and exported IPA plus a signed Android AAB. Use `--ios` or `--android` to build one platform. Add `--flavor staging` to `dev`, `test`, or `release` for a separate app identity such as `dev.nexa.myapp.staging`; `--staging` is a shorthand. Define flavors in `nexa.config.nx`, for example `flavors { staging { suffix: "staging" }, production { suffix: "" } }`. A missing suffix defaults to the flavor name, while an empty suffix keeps the configured base app ID. `nexa doctor` checks the required toolchains.

Android releases require non-empty `NEXA_ANDROID_KEYSTORE`, `NEXA_ANDROID_KEY_ALIAS`, `NEXA_ANDROID_STORE_PASSWORD`, and `NEXA_ANDROID_KEY_PASSWORD` environment variables. The keystore path may be absolute or relative to the project directory, and must identify an existing file. Nexa validates these inputs before building, passes Gradle the resolved keystore path, and verifies the generated AAB signature before reporting success.

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

The shared icon source generates an iOS AppIcon asset and Android density icons, adaptive and round launcher icons, and a monochrome adaptive layer for themed icons. Platform-specific `ios.icon` or `android.icon` paths override the shared source; iOS accepts an Xcode asset-catalog icon directory or an Icon Composer `.icon` asset, while Android accepts a resource directory. Signing secrets belong in Xcode or CI and the Android signing environment variables, never in `nexa.config.nx`.

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
