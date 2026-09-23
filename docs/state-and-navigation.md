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
            Text("Global Counter: ${globalCounter}")

            NavigationLink(destination: Details) {
                Text("Go to Details")
            }
        }
    }

    screen Details {
        state localTaps: Int32 = 0

        Column(spacing: 12) {
            Text("Details Screen")
            Text("Local Taps: ${localTaps}")

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

- **`NavigationStack(root: ScreenName)`**: The top-level container hosting screens (maps to SwiftUI `NavigationStack` on iOS and `NavHost` on Android).
- **`NavigationLink(destination: ScreenName, when: condition)`**: Navigates onto the stack with platform push transitions and swipe-to-back gestures.
- **`NavigationBack(label: "...")`**: Programmatic or custom back button.

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
            Tab(index: 1, label: "Explore", icon: "safari") {
                Column(spacing: 8) {
                    Text("Explore Tab View")
                }
            }
            Tab(index: 2, label: "Notifications", icon: "bell", badge: "3") {
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
- **`Tab(index: Int, label: String, icon: String?, badge: String?) { ... }`**: Each tab specifies its zero-based index, label, system icon, badge counter, and body content.
- Compiles to native `TabView` on iOS and `NavigationBar` / `NavigationBarItem` on Android.

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

Screens and components can hook into platform appearance and app state events:

```nexa
screen Dashboard {
    state isRefreshed: Bool = false

    OnAppear {
        // Synchronous setup on view mount
        isRefreshed = true
    }

    OnAppear async {
        // Asynchronous background fetch
        await loadUserData()
    }

    OnDisappear {
        // Teardown when view leaves screen
        isRefreshed = false
    }

    OnActive {
        // App returned to foreground
    }

    OnBackground {
        // App moved to background
    }

    body {
        Column {
            Text("Dashboard")
        }
    }
}
```
