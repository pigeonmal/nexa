# State & Navigation in Nexa

Nexa provides a unified, compile-time verified architecture for reactive state management, navigation stacks, and bottom tab bars.

---

## 1. Stack Navigation: `NavigationStack` & `NavigationLink`

Multi-screen applications declare explicit `screen` blocks inside `app` and establish stack-based routing with `NavigationStack`:

```nexa
app NavigationDemo {
    state globalCounter: Int32 = 0

    screen Home {
        Column(spacing: 12) {
            Text("Home Screen")
            Text("Global Counter: $globalCounter")

            NavigationLink(destination: Details) {
                Text("Go to Details")
            }
        }
    }

    screen Details {
        state localTaps: Int32 = 0

        Column(spacing: 12) {
            Text("Details Screen")
            Text("Local Taps: $localTaps")

            Button("Tap Locally") {
                localTaps = localTaps + 1
            }
            Button("Increment Global") {
                globalCounter = globalCounter + 1
            }
        }
    }

    body {
        NavigationStack(root: Home)
    }
}
```

Screens can also live in their own `.nx` files. A standalone file exports a
top-level `screen`; import that file from the app entry file and use the screen
name with the same `NavigationStack` and `NavigationLink` syntax. Imported
screens share the app's state and functions, while their own state remains local
to that screen:

```nexa
// App.nx
import "screens/Home.nx"
import "screens/Details.nx"

app ModularNavigation {
    state visits: Int32 = 0

    body {
        NavigationStack(root: Home)
    }
}
```

```nexa
// screens/Home.nx
screen Home {
    Column {
        Text("Home")
        NavigationLink(destination: Details) {
            Text("Open details")
        }
    }
}
```

```nexa
// screens/Details.nx
screen Details {
    state localCount: Int32 = 0

    Column {
        Text(visits)
        Text(localCount)
    }
}
```

For tab layouts, put each tab's content in an imported `component` and call it
inside the existing `Tab` block. This keeps the tab bar definition in the app
entry file and each tab's view in a separate source file:

```nexa
// App.nx
import "tabs/HomeTab.nx"
import "tabs/ProfileTab.nx"

app ModularTabs {
    state currentTab: Int32 = 0

    body {
        AppBottomBar(selected: currentTab) {
            Tab(index: 0, label: "Home", icon: "home", comment: "Primary navigation destination") {
                HomeTab()
            }
            Tab(index: 1, label: "Profile", icon: "person") {
                ProfileTab()
            }
        }
    }
}
```

Each tab module declares a normal component, for example
`component HomeTab() { body { Column { Text("Home") } } }`. Imported modules
can contain screens, components, structs, functions, and plugin declarations.

- **`NavigationStack(root: ScreenName)`**: The top-level container hosting screens (maps to SwiftUI `NavigationStack` on iOS and `NavHost` on Android).
- **`NavigationLink(destination: ScreenName, when: condition)`**: Navigates onto the stack with platform push transitions and swipe-to-back gestures.
- **`NavigationBack(label: "...")`**: Programmatic or custom back button.

### Deep links

Declare custom URL schemes and HTTPS link domains in `nexa.config.nx`:

```nexa
config {
    app {
        deepLinks: ["nexa://", "https://links.example.com"]
    }
}
```

Routes use the screen name converted to lowercase kebab case, followed by each
screen parameter as a path segment. For example, `ProductDetails(id: String,
page: Int32)` opens with `nexa://product-details/widget-17/5` or
`https://links.example.com/product-details/widget-17/5`. Nexa registers custom
schemes with iOS and Android, configures iOS Associated Domains and Android
verified App Links for HTTPS origins, and decodes route parameters using their
declared scalar types. HTTPS domains also need the platform association file
hosted by the app owner.

---

## 2. Tab Bar Navigation: `AppBottomBar`

For bottom tab navigation, Nexa provides `AppBottomBar`. It binds to an active tab state index and manages tab switching across `Tab` content blocks:

```nexa
app TabbedApp {
    state currentTab: Int32 = 0

    body {
        AppBottomBar(selected: currentTab) {
            Tab(index: 0, label: "Home", icon: "house") {
                Column(spacing: 8) {
                    Text("Home Tab View")
                }
            }
            Tab(index: 1, label: "Explore", icon: "language") {
                Column(spacing: 8) {
                    Text("Explore Tab View")
                }
            }
            Tab(index: 2, label: "Notifications", icon: "notifications", badge: "3") {
                Column(spacing: 8) {
                    Text("Notifications Tab View")
                }
            }
            Tab(index: 3, label: "Profile", icon: "person") {
                Column(spacing: 8) {
                    Text("Profile Tab View")
                }
            }
        }
    }
}
```

- **`selected: stateVar`**: Reactive two-way binding tracking the active tab index.
- **`Tab(index: Int32, label: String, icon: String?, badge: String?) { ... }`**: Each tab specifies its zero-based index, label, optional shared system icon, optional badge text, and body content. The label stays in the tab bar; it does not become an automatic screen title.
- Compiles to native SwiftUI `TabView` on iOS and Material 3 `NavigationSuiteScaffold` on Android. iOS 18+ uses the sidebar-adaptable tab style with a native tab bar on iPhone; older versions use the native `TabView` tab bar. Android adapts between bottom navigation on compact windows and rail or drawer navigation on wide windows. Android retains saveable state for each tab while another destination is selected.

---

## 3. Modal Presentation: `BottomSheet`

Present modal sheets over screens with reactive presentation state:

```nexa
app SheetDemo {
    state showSheet: Bool = false

    body {
        Column {
            Button("Open Filters") {
                showSheet = true
            }

            BottomSheet(isPresented: showSheet, partial: true) {
                Column(spacing: 16, padding: 20) {
                    Text("Filter Options")
                    Button("Dismiss") {
                        showSheet = false
                    }
                }
            }
        }
    }
}
```

---

## 4. Lifecycle Hooks

Apps and named screens can hook into platform appearance events. App lifecycle callbacks are only allowed in the app body; screen lifecycle callbacks are only allowed in screen bodies:

```nexa
app LifecycleDemo {
    state userName: String = ""

    async fn loadUserData() -> String {
        return "Nexa user"
    }

    screen Dashboard {
        state isRefreshed: Bool = false

        OnAppear async {
            isRefreshed = true
            userName = await loadUserData()
        }

        OnDisappear {
            isRefreshed = false
        }

        Column {
            Text(userName)
        }
    }

    body {
        OnActive {
            userName = "App active"
        }

        OnBackground {
            userName = "App in background"
        }

        NavigationStack(root: Dashboard)
    }
}
```

`OnAppear` and `OnDisappear` belong at the top level of an app body or named screen. Use `OnAppear async` for awaited work. `OnActive`, `OnInactive`, and `OnBackground` are app-body callbacks; they are not screen callbacks.

In `nexa dev`, async app-local functions and nested async expressions run in the native DevRuntime. It evaluates plugin and built-in async calls recursively through actions, expressions, collections, transforms, and struct constructors. Native plugin services and class constructors or methods dispatch to generated direct adapters for signatures accepted by the native plugin validators, including recursively composed arrays, sets, maps, pairs, triples, and declared structs. Generic plugin calls use typed codecs for scalar, `Bytes`, enum, struct, `Result` with enum failures, array, set, map, pair, and triple values. Native class property reads and writes, event subscriptions, and native visual components hot reload for supported scalar, `Bytes`, enum, optional, native-class-reference, array, set, map, pair, triple, and struct values; arrays with optional elements are supported by these DevRuntime adapters. New event subscriptions in app or active-screen `OnAppear` actions are installed after a reload without rerunning the lifecycle action; existing callbacks rebind to the new action body. Plugin declarations in any imported `.nx` file are collected across the app source graph. A newly added module can import a plugin already configured in `nexa.config.nx` and use its prelinked native adapters after hot reload. Declared throwing plugin calls reach typed catches, including errors with compound payloads. Optional generic inputs and values nested inside compound generic values preserve null separately from decode failures. A plugin method returning `T?` transports a missing value with the Dev host null sentinel when `T` is nonnullable.

The log-only device probe in `scripts/test-dev-runtime-plugin-hot-reload.sh` exercises plugin calls, typed errors, properties, events, components, generic values, and a newly imported component on iOS Simulator and Android Emulator. Its coverage inventory uses these runtime assertions for plugin property/event/component variants instead of treating interpreter dispatch markers as semantic proof. The native plugin host is prelinked: a new `.nx` file may import an already configured plugin, but adding a plugin or changing its IDL, native source, or host dependencies requires `b` to rebuild and relaunch the Dev host. The static DevRuntime adapters do not support `Set<Bytes>` values because iOS `Data` and Android `ByteArray` do not share value-equality semantics for set membership. DevRuntime accepts generic reads with nullable type arguments and uses tagged decode results to distinguish a valid null payload from reader failure. On Android, the plugin method's `T?` return still collapses a missing result and a valid null when `T` itself is nullable, matching Kotlin's type model. AOT compilation continues to reject nullable generic read arguments because its cross-platform contract does not yet represent the outer missing-result layer separately. The generated AOT app uses the normal native API and plugin implementations.
