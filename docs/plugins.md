# Nexa plugins

Nexa keeps optional integrations outside the core compiler and default native
component set. The current tool can create an isolated plugin scaffold:

```sh
cargo run -p nexa-cli -- plugin init com.example.camera \
    --out CameraPlugin --name Camera --version 0.1.0
```

The scaffold contains:

- `nexa.plugin.json` with a stable format version, package id, version, and
  iOS/Android implementation source paths;
- `ios/Sources/Camera.swift` as the iOS implementation boundary;
- `android/src/main/kotlin/.../Camera.kt` as the Android implementation
  boundary;
- a README that records the package shape and current limitations.

The command is deterministic and idempotent: unchanged files are preserved and
existing edits are not overwritten when their contents differ. The generated
manifest currently has an empty `interfaces` array. Typed IDL parsing,
dependency resolution, generated `.nx` calls, Swift/Kotlin binding generation,
and plugin installation are still roadmap work. Do not add plugin source to the
core compiler or hand-edit generated application files until those boundaries
are implemented.
