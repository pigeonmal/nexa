# FastMath Nexa C++ Plugin Example

This plugin demonstrates how to write ultra-high-performance cross-platform native plugins for **Nexa** in pure modern C++ (`c++20`).

## Features

- **Direct C++ Execution**: Code compiles directly via Clang++ on iOS and Android NDK on Android with zero runtime VM or JS engine.
- **Strongly Typed Contract (`native.nxid`)**: Nexa generates type-safe bindings for Swift and Kotlin JNI automatically.
- **Asynchronous Futures**: Asynchronous operations return standard `std::future<T>`, mapping to Swift `async` and Kotlin coroutines.
- **Zero-Copy Byte Buffers**: `Bytes` in `.nxid` maps directly to `std::vector<std::uint8_t>` without boxing.

## Plugin Structure

```
fast-math/
├── native.nxid           # Plugin IDL contract
├── plugin.config.nx      # Plugin manifest with C++ compilation settings
├── cpp/
│   ├── include/
│   │   └── FastMath.hpp  # Public C++ header
│   └── Sources/
│       └── FastMath.cpp  # C++ implementation
└── README.md
```

## Usage in Nexa (`.nx`)

```nexa
plugin "fast-math" as FastMath

app DemoApp {
    state result: Int32 = 0

    body {
        Column {
            Text("Result: ${result}")
            Button("Compute C++") {
                result = FastMath.add(10, 20)
            }
        }
    }
}
```
