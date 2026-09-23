# FastMath Nexa C++ Plugin Example

This plugin demonstrates how to write ultra-high-performance cross-platform native plugins for **Nexa** in pure modern C++ (`c++20`).

## Features

- **Direct C++ Execution**: C++ sources compile into the host app with Clang on iOS and the Android NDK on Android.
- **Strongly Typed Contract (`native.nxid`)**: Nexa generates type-safe bindings for Swift and Kotlin JNI automatically.
- **Asynchronous Futures**: Asynchronous operations return standard `std::future<T>`, mapping to Swift `async` and Kotlin coroutines.
- **Typed Byte Buffers**: `Bytes` in `.nxid` maps to `std::vector<std::uint8_t>` in C++; generated platform bindings convert byte collections at the native boundary.

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
