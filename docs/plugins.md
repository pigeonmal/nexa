# Native Plugin System (Swift, Kotlin, C++)

Nexa's typed native plugin system allows developers to integrate platform SDKs, native hardware features, and C++ libraries into `.nx` applications through generated Swift, Kotlin, and C++ bindings.

---

## App plugin dependencies

Declare a local plugin package in `nexa.config.nx`, then reference its package
ID from the `.nx` entry file:

```nexa
config {
    dependencies {
        FastMath { id: "dev.example.fast-math", path: "../fast-math" }
    }
}
```

```nexa
plugin "dev.example.fast-math" as FastMath
```

Git dependencies must use a full commit hash. `package` selects a plugin
subdirectory within the pinned repository, so multiple packages can share one
repository revision:

```nexa
config {
    dependencies {
        FastMath {
            id: "dev.example.fast-math",
            git: "https://example.com/mobile-plugins.git",
            rev: "0123456789abcdef0123456789abcdef01234567",
            package: "plugins/fast-math"
        }
        DateTime {
            id: "dev.example.date-time",
            git: "https://example.com/mobile-plugins.git",
            rev: "0123456789abcdef0123456789abcdef01234567",
            package: "plugins/date-time"
        }
    }
}
```

`nexa dev`, `nexa test`, and `nexa release` resolve these packages, verify
that each package manifest ID matches the configured ID, and write the
resolved package sources and content hashes to `nexa.lock`. Use `--locked` on
`check`, `dev`, `test`, or `release` to require an existing matching lockfile.
If a local plugin changes, run `nexa check` to update the lock before a locked
build. Git checkouts are cached under `.nexa/plugins/`; that directory is
generated and should not be committed.

## Official plugins

Official plugin packages live in their own repository,
[`pigeonmal/nexa-plugins`](https://github.com/pigeonmal/nexa-plugins), which is
mounted in this repository as the `plugins/` submodule. One plugin occupies one
directory at its root, so `package: "mmkv"` selects it inside a pinned
`nexa-plugins` revision, and the same directory is reachable as
`plugins/mmkv` for local path dependencies and end-to-end tests.

```bash
git clone --recurse-submodules https://github.com/pigeonmal/nexa.git
# existing clone
git submodule update --init --recursive
```

Plugin development happens in the plugin repository, where `nexa plugin check`
and `nexa plugin generate` validate every package in CI.

## 1. Plugin Architecture Overview

A Nexa plugin consists of three components:
1. **Interface Contract (`native.nxid`)**: Strongly-typed IDL defining structs, enums, error types, native classes, and services.
2. **Plugin Manifest (`plugin.config.nx`)**: Declares package metadata, platform source patterns, framework dependencies, and C++ standards.
3. **Native Implementations**: Direct Swift (`.swift`), Kotlin (`.kt`), or C++ (`.cpp`, `.hpp`) implementations.

```
my-plugin/
├── native.nxid           # Strongly typed schema
├── plugin.config.nx      # Plugin metadata & compiler flags
├── cpp/                  # C++ implementation (optional)
│   ├── include/
│   └── Sources/
├── ios/                  # Swift implementation (optional)
│   └── Sources/
└── android/              # Kotlin implementation (optional)
    └── src/main/kotlin/
```

---

## 2. Defining the IDL Contract (`.nxid`)

```nxid
struct ComputeConfig {
    threads: Int32 = 4
    precision: Float64 = 0.001
}

service FastEngine {
    fn add(a: Int32, b: Int32) -> Int32
    fn processBytes(data: Bytes) -> Bytes
    async fn runHeavyJob(threads: Int32, precision: Float64) -> Int64
}
```

### Generic value methods

A method may declare value type parameters. Every call site binds them, and the
bound type selects one generated value codec, so a plugin implements the method
once instead of once per value type:

```nxid
native class Store {
    init(instanceID: String)

    // Scalars, strings, and buffers are stored natively, one call each.
    fn setString(key: String, value: String) -> Bool
    fn getString(key: String) -> String?

    // Everything else goes through the value codec of the bound type.
    fn setObject<T>(key: String, value: T) -> Bool
    fn getObject<T>(key: String) -> T?
    fn setList<T>(key: String, values: Array<T>) -> Bool
    fn getList<T>(key: String) -> Array<T>?
    fn setSet<T>(key: String, values: Set<T>) -> Bool
    fn getSet<T>(key: String) -> Set<T>?
    fn setMap<K, V>(key: String, values: Map<K, V>) -> Bool
    fn getMap<K, V>(key: String) -> Map<K, V>?
}
```

The codec is not written by the plugin author. The generated contract takes one
codec parameter per parameter or return type that mentions a type parameter,
and the host compiler passes a concrete writer or reader at each call site:

```swift
func setObject<T>(_ key: String, _ value: T, _ encode: (T, NexaValueWriter) -> Void) -> Bool
func getObject<T>(_ key: String, _ decode: (NexaValueReader) -> T?) -> T?
```

```kotlin
fun <T> setObject(key: String, value: T, encode: (T, NexaValueWriter) -> Unit): Boolean
fun <T> getObject(key: String, decode: (NexaValueReader) -> T?): T?
```

So the plugin body is one call into MMKV, a file, or a database:

```swift
public func setObject<T>(_ key: String, _ value: T, _ encode: (T, NexaValueWriter) -> Void) -> Bool {
    let writer = NexaValueWriter()
    encode(value, writer)
    return setBuffer(key, writer.data)
}
```

Rules for the bound type:

- It is a scalar, `Bytes`, an enum, an app-declared struct, or a collection of
  those. An optional is not a storable value: a missing key already means
  `null`.
- A setter binds from the value it is given. A getter binds from the type it has
  to produce, or from an explicit type argument when there is no other
  information: `store.getObject<PlayerOptions>("key")`.
- Every type parameter must be bound. An unbound one is an error that names the
  binding syntax.
- The generated layout is fixed: little-endian scalars, a length-prefixed
  string or buffer, a `UInt32` element count, and struct fields in declaration
  order. Sets and maps are written in a canonical order, so a value written on
  one platform reads back on the other. The schema is not versioned, so a struct
  change means a new key or a version field.
- The C++ bridge has no spelling for a per-call-site value type, so generics are
  Swift and Kotlin only. A package that declares a C++ block and uses them is
  rejected.

On Android the writer and reader live in the generated `dev.nexa.core` package,
which also holds the application context, because a Kotlin plugin is generated
into the plugin's own package and has no other way to reach either.

## Plugin host metadata

Platform declarations can request host configuration that the native
implementation cannot add by itself. For example, background audio uses:

```nexa
ios {
    backgroundModes: ["audio"]
}
android {
    mediaPlaybackService: "dev.nexa.audio.AudioPlaybackService"
}
```

The iOS mode is added to `UIBackgroundModes`. Android adds the foreground
service permissions and declares the named Media3 service with its
`MediaSessionService` intent action and `mediaPlayback` foreground type.
Service names must be fully qualified Java or Kotlin class names.

---

## 3. High-Performance C++ Implementation

When implementing plugins in C++, Nexa generates JNI bindings on Android and Swift-C++ interop bindings on iOS. Values cross these language boundaries through generated adapters; collection values such as byte arrays may be copied during conversion.

### Manifest Configuration (`plugin.config.nx`):

```nexa
plugin {
    schema: 2
    id: "dev.nexa.fast-engine"
    version: "1.0.0"
    sources {
        native: "native.nxid"
    }
    cpp {
        standard: "c++20" // Options: "c++17", "c++20", "c++23"
        sources: ["cpp/Sources/**"]
        headers: ["cpp/include/**"]
    }
}
```

### C++ Header (`cpp/include/FastEngine.hpp`):

```cpp
#pragma once
#include <cstdint>
#include <future>
#include <vector>

namespace plugin_dev::plugin_nexa::plugin_fast_dash_engine::FastEngine {

std::int32_t add(std::int32_t a, std::int32_t b) noexcept;
std::vector<std::uint8_t> processBytes(std::vector<std::uint8_t> data) noexcept;
std::future<std::int64_t> runHeavyJob(int32_t threads, double precision) noexcept;

}
```

### C++ Source (`cpp/Sources/FastEngine.cpp`):

```cpp
#include "FastEngine.hpp"

namespace plugin_dev::plugin_nexa::plugin_fast_dash_engine::FastEngine {

std::int32_t add(std::int32_t a, std::int32_t b) noexcept {
    return a + b;
}

std::vector<std::uint8_t> processBytes(std::vector<std::uint8_t> data) noexcept {
    for (auto &byte : data) {
        byte ^= 0x5A;
    }
    return data;
}

std::future<std::int64_t> runHeavyJob(int32_t threads, double precision) noexcept {
    std::promise<std::int64_t> p;
    p.set_value(static_cast<std::int64_t>(threads * 1000));
    return p.get_future();
}

}
```

---

## 4. Consuming Plugins in `.nx`

Import the plugin at the top of your `.nx` file:

Plugin service methods and native class methods use positional arguments in `.nxid` declaration order. Generated Swift bindings also omit external argument labels; `fn add(a: Int32, b: Int32)` becomes `func add(_ a: Int32, _ b: Int32)` in Swift.

A Swift package dependency declares exactly one of `from`, `branch`, or
`revision`. Use `revision` when the upstream repository has no SwiftPM release
tags: the fork's tags can predate its `Package.swift`, and a version range then
resolves to a commit with no manifest.

```nexa
plugin "fast-engine" as FastEngine

app PluginApp {
    state result: Int32 = 0

    body {
        Column {
            Text("Result: $result")
            Button("Execute C++ Native Function") {
                result = FastEngine.add(15, 27)
            }
        }
    }
}
```

Nexa automatically links the native C++ code into your iOS Xcode and Android Gradle build targets!
