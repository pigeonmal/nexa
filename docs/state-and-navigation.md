# State & Navigation in Nexa

Nexa provides a unified, compile-time verified architecture for reactive state management and multi-screen navigation.

---

## 1. Screens & NavigationStack

Multi-screen applications declare explicit `screen` blocks inside `app` and establish routing with `NavigationStack`:

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

---

## 2. Navigation Paradigms

- **`NavigationStack(root: ScreenName)`**: Hosts the navigation container (maps to `NavigationStack` in SwiftUI and `NavHost` in Jetpack Compose).
- **`NavigationLink(destination: ScreenName)`**: Navigates onto the stack with standard platform push transitions and native swipe-to-back gestures.
- **`StatusBar(style: Light | Dark, hidden: Bool)`**: Configures platform status bar appearance per screen.

---

## 3. Lifecycle Hooks

Screens and views can respond to platform appearance events using `OnAppear` and `OnDisappear`:

```nexa
screen Dashboard {
    state isRefreshed: Bool = false

    OnAppear {
        isRefreshed = true
    }

    OnDisappear {
        isRefreshed = false
    }

    body {
        Column {
            Text("Dashboard")
        }
    }
}
```
