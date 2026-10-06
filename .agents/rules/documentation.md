# Nexa Documentation Rules & Style Guide

All documentation across the Nexa repository (`docs/`, `README.md`) and the plugins repository (`plugins/`) must strictly satisfy the 5 laws of Nexa documentation.

## The 5 Laws of Nexa Documentation

### 1. Scannability First (No Walls of Text)
- Any prose paragraph longer than 3–4 lines must be refactored into:
  - Markdown tables (for options, parameters, return types, flags)
  - Bulleted or numbered lists (for sequential steps or feature lists)
  - Mermaid architecture diagrams (for flows, lifecycles, and hierarchies)
  - Semantic alert callouts: `> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]`
- Every documentation page must feature:
  - Document Title & Badges (Platforms, min SDK, status, crate/package)
  - Executive Overview (1–2 punchy sentences)
  - Copy-pasteable 10-second Quick Start
  - Real-world production examples
  - 100% Comprehensive API Reference tables

### 2. 100% Complete API Surface Coverage
- Every property, argument, method, constructor, event callback, error variant, and return type must be explicitly listed in reference tables.
- Never summarize with "and others...", "etc.", or leave parameters unmentioned.
- For UI Components:
  - Property name
  - Type (including nullability `Type?`)
  - Default value
  - Description and platform behavior notes
  - Allowed child blocks
  - Trailing dot-modifiers
- For Native Plugins (`.nxid`):
  - Structs & enums with every field and case
  - Error enum variants with associated payload types
  - Native class constructors, properties, and methods (synchronous & async)
  - Native event handlers with payload types

### 3. Production-Ready, Realistic Examples
- Prohibit generic placeholders (`foo`, `bar`, `myVar`, `1 + 1`, `doSomething()`).
- All examples must reflect real-world mobile app design:
  - E-commerce cart checkout with price arithmetic and currency formatting
  - User profile with avatar images, status badges, and settings toggles
  - Task / todo lists with completion states, priority badges, and relative timestamps
  - Live database queries with reactive `Signal<T>` subscriptions and empty states
  - Video and audio player controls with timeline scrubbing and playback state

### 4. Zero Deprecated Syntax
- Code samples must match current `.nx` grammar.
- Prohibit legacy `@query` annotations, untyped maps, obsolete lifecycle hooks, and removed compiler keywords.
- Always use typed state (`state count: Int32 = 0`), typed error recovery (`try { ... } catch { ... }`), and reactive primitives (`observeQuery`, `Signal<T>`).

### 5. Architectural Transparency
- Detail native platform mappings:
  - Swift / SwiftUI translation on iOS
  - Kotlin / Jetpack Compose translation on Android
  - C++20 JNI / bridging mechanics for plugins
- Detail threading models (main UI thread, background dispatchers, serial actors) and memory lifecycles (`OnAppear`, `OnDisappear`, `dispose()`).
