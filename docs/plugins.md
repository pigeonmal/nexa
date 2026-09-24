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
