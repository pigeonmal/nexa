# Nexa Documentation

Welcome to the official documentation for **Nexa**, the ultra-high-performance Ahead-Of-Time (AOT) transpiler that compiles declarative `.nx` applications directly into native **Swift (SwiftUI)** for iOS and **Kotlin (Jetpack Compose)** for Android.

---

## Documentation Index

| Guide | Description |
|---|---|
| 🚀 [**Getting Started**](getting-started.md) | Install the CLI, initialize a project, run the dev workflow, and build native apps. |
| 📖 [**Language Guide**](language-guide.md) | Syntax reference, types, functions, control flow, and explicit error handling (`Result<T, E>`). |
| 🧩 [**Component Reference**](components.md) | Built-in layout containers, interactive controls, media, styling, and event modifiers. |
| 🧭 [**State & Navigation**](state-and-navigation.md) | Reactive state management, navigation stacks, screen routing, and lifecycle hooks. |
| 🔌 [**Native Plugins (Swift, Kotlin, C++)**](plugins.md) | Write cross-platform native plugins with `.nxid` contracts, C++ acceleration, and zero VM overhead. |
| 🏗️ [**Architecture & Performance**](architecture.md) | Learn how Nexa achieves zero runtime reflection, zero type erasure, and unboxed primitives. |

---

## Core Philosophy

- **Zero Runtime VM**: Nexa does not ship a JavaScript engine, Dart runtime, or bytecode interpreter. Generated code uses standard platform UI frameworks directly.
- **Native Performance**: Generates specialized SwiftUI view hierarchies (no `AnyView`) and Compose state primitives (`mutableDoubleStateOf`, `mutableIntStateOf`) without heap boxing.
- **Single Source of Truth**: Write your UI and business logic once in `.nx`, compile natively to both iOS and Android.
- **Strongly Typed Native Plugins**: Integrate platform capabilities and native C++ logic with strongly typed contracts and zero-copy buffers.
