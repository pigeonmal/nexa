---
name: nexa-app
description: "Use when helping a Nexa user plan, author, structure, or troubleshoot a mobile app in `.nx`, especially when the user wants a complete app without writing Swift or Kotlin."
---

# Create an App with Nexa

Help people build an iOS and Android app from Nexa's shared `.nx` source without asking them to edit native Swift or Kotlin. Start from the user's product goal, clarify important screens and interactions only when needed, and build a coherent app using the framework's current core components.

## Work within today's language

- Read `README.md`, `docs/language.md`, and the examples relevant to the request before writing `.nx` syntax. The language and compiler are an early prototype; check actual parser/backend support instead of assuming the roadmap is implemented.
- Use core components already supported by Nexa, such as View, Text, Button, TextInput, Switch, Image, navigation, keyboard-aware layout, and FastList when available in the current compiler.
- Use `if`/`else`, `&&`, `||`, `!`, scalar `==`/`!=`, and numeric comparisons for supported conditional UI and button/press actions. Keep compared numeric types equal; Nexa does not implicitly convert values.
- Set a layout's cross-axis `alignment` to `Start`, `Center`, or `End` where needed; vertical layouts align horizontally, while rows align vertically.
- Keep user-authored application code in `.nx`. Do not put app features in generated `.swift` or `.kt` files and do not make native edits a prerequisite for supported functionality.
- Use FastList for large or repeating collections when supported. Keep layout and state simple so generated code remains direct and efficient.
- Define reusable `component Name(property: Type)` declarations in `.nx`, with private `state` declarations and a `body`; invoke them by name using all required named properties. Each instance owns native component state.
- Split larger apps into `.nx` files with `import "relative/path/Component.nx"`. Resolve imports relative to the importing file, keep one project-wide component name scope, and put the single `app` declaration in the entry file. Imported component files may import other component files.
- Keep component boundaries focused. Callbacks, content slots, and `NavigationLink` from inside custom components are not supported yet; pass values as typed inputs and keep interactions inside the component when possible.
- For Android images, rely on Nexa's Coil 3 based image generation; explain any host-project Coil 3 setup that the current CLI does not generate.
- Optional plugins such as SQLite, MMKV, and maps are future extensions unless the repository currently supplies them. Do not invent plugin syntax or silently replace missing support with native code.

## User experience and accuracy

- Translate nontechnical requests into screens, state, navigation, and component behavior. Explain the proposed structure in plain language and preserve the requested visual/product intent.
- When a requested feature is unsupported, identify the specific gap and offer the closest working core-only approach. Do not claim a complete build, plugin installation, or app-store-ready project if Nexa currently only emits source files.
- Keep app source modular as it grows: separate screens and reusable component declarations using only syntax the language actually supports. Avoid premature plugin or runtime architecture in app code.
- Generate Swift and Kotlin from the same `.nx` source and check that both targets express equivalent behavior. Prefer platform-native APIs through the compiler's existing mappings.
- Give clear build instructions using the current Nexa CLI and describe any required host Xcode/Android project setup without asking the user to write native source for features the language supports.
