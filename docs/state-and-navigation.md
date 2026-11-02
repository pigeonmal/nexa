# State and Navigation

Nexa state is declared with `state` and checked at compile time. Screens compile to native navigation destinations in SwiftUI and Jetpack Compose; use `NavigationStack` for a stack or `AppBottomBar` for tab navigation.

| **Scope**: state, screens, and navigation | **Targets**: iOS and Android | **Reactive data**: `Signal<T>` |

## Quick start: product list and detail

```nx
struct Product {
    id: Int32,
    name: String,
    priceLabel: String,
}

app Storefront {
    state cartCount: Int32 = 0
    state products: Array<Product> = [
        Product(1, "Wireless headphones", "$149.00"),
        Product(2, "Travel charger", "$39.00")
    ]

    screen ProductList {
        Column(gap: 12, padding: 16) {
            Text("Featured products", fontSize: 24, fontWeight: Bold)
            Text("Cart items: \(cartCount)", color: "#475569")
            FastList(products, key: .id) { product, index in
                NavigationLink(destination: ProductDetail(product.id)) {
                    Row(gap: 12, padding: 12, background: "#F1F5F9", cornerRadius: 8) {
                        Text("\(index + 1). \(product.name)", fontWeight: Semibold)
                        Spacer()
                        Text(product.priceLabel, color: "#1D4ED8")
                    }
                }
            }
        }
    }

    screen ProductDetail(productID: Int32) {
        Column(gap: 12, padding: 20) {
            Text("Product #\(productID)", fontSize: 24, fontWeight: Bold)
            Button("Add to cart", icon: "shopping_cart") {
                cartCount = cartCount + 1
            }
            NavigationBack(label: "Back to products")
        }
    }

    body {
        NavigationStack(root: ProductList)
    }
}
```

The root and destination names, route argument count, and argument types are checked by the compiler. Screen route arguments are declared in the screen signature and passed positionally in `NavigationLink(destination: ScreenName(value))`.

Each `NavigationStack` owns a route path and root identity. Multiple stacks in separate tabs keep independent histories, and `Navigation.push` / `Navigation.back()` act on the nearest stack that contains the calling screen. Android stores destination state on its `NavBackStackEntry`, so popping a child reveals the previous screen with its state intact. iOS stores typed route values in the stack path and uses the native interactive pop gesture, including when the navigation bar is hidden. SwiftUI navigation does not use the type-erased `NavigationPath`.

## State scope

| Declaration | Typical use | Native lifetime |
|---|---|---|
| `state` in `app` | Cart count, selected tab, app-wide preferences | App view lifetime |
| `state` in `screen` | Search text, form draft, screen-local selection | Destination lifetime |
| `state` in `component` | Reusable control state | Component instance lifetime |
| `Signal<T>` from a typed plugin API | Live external data, such as a SQLite query | Managed by the generated native signal binding |

`state` is mutable and causes the owning native view to update. `let` is immutable. Value structs have immutable fields, so update an item by mapping to a replacement value rather than assigning to a row field.

## Tabs

Each `Tab` declares a non-negative, unique `Int32` index, a label, and a shared system icon. The tab body owns its content.

```nx
app ShopTabs {
    state selectedTab: Int32 = 0
    state cartCount: Int32 = 2

    body {
        AppBottomBar(selected: selectedTab) {
            Tab(index: 0, label: "Home", icon: "home", title: "Home") {
                Column(gap: 8, padding: 16) {
                    Text("New this week", fontSize: 22, fontWeight: Bold)
                    Text("Browse the latest products.")
                }
            }
            Tab(index: 1, label: "Cart", icon: "shopping_cart", badge: "2", title: "Your cart") {
                Text("Your cart contains \(cartCount) items.")
            }
        }
    }
}
```

On iOS this maps to SwiftUI `TabView`. Android uses Material 3 adaptive navigation, switching between a bottom bar and wider navigation layouts where supported. The exact tab options are listed in the [syntax audit](syntax-audit.md#tabs).

### Recommended custom bottom toolbar

When the product design needs a branded bottom toolbar, use `AppBottomBar` with one trailing `TabBar` block. This is Nexa's recommended custom-tab pattern: `Tab` declares each destination and its navigation state, while `TabBar` owns the visible toolbar. Build its icons, labels, press targets, active states, mini-player, and other persistent controls with ordinary Nexa components. A `Column` can stack a mini-player or secondary action row above the tab controls.

When porting an existing custom tab shell, keep the responsibilities mapped like this:

| Existing tab-shell responsibility | Nexa implementation |
|---|---|
| Tab destination and retained navigation stack | One `Tab` child per destination, declaring its index, label, icon, and destination body. |
| Selected tab / navigation index | One app-scoped mutable state passed to `AppBottomBar(selected:)` and updated by custom tab presses. |
| Branded tab visuals and hit targets | The trailing `TabBar` block, composed from `Row`, `Column`, `Pressable`, `Icon`, `Text`, or app components. |
| Persistent controls above the tabs, such as a mini-player | Place them in the same `TabBar` block, usually above the tab-control row in a `Column`. |

Use `Toolbar` for native navigation-bar actions at the top of a destination. It does not replace `TabBar`: a custom bottom toolbar belongs in `AppBottomBar` so Nexa keeps destination routing and the app's tab presentation together.

The custom `TabBar` is a presentation slot; `Tab` metadata does not create or populate its visible controls. Bind each custom tab press to the same app-scoped selected-tab state passed to `AppBottomBar`, so navigation and the custom active state stay synchronized across destinations:

```nx
app BrandedTabs {
    state selectedTab: Int32 = 0

    body {
        AppBottomBar(selected: selectedTab) {
            Tab(index: 0, label: "Home", icon: "home") {
                Column(gap: 8, padding: 16) {
                    Text("Your latest updates", fontSize: 22, fontWeight: Bold)
                    Text("Browse the latest products.")
                }
            }
            Tab(index: 1, label: "Profile", icon: "person") {
                Column(gap: 8, padding: 16) {
                    Text("Alex Morgan", fontSize: 22, fontWeight: Bold)
                    Text("Account preferences and saved addresses.")
                }
            }

            TabBar {
                Row(gap: 24, alignment: Center, padding: 12, background: "#09090B") {
                    Pressable(accessibilityLabel: "Home tab", accessibilityRole: Button) {
                        Column(gap: 4, alignment: Center) {
                            Icon(system: "home", description: "Home", size: 20, tint: "#FFFFFF")
                            Text("Home", color: "#FFFFFF")
                        }
                    }.onTap { selectedTab = 0 }
                    Pressable(accessibilityLabel: "Profile tab", accessibilityRole: Button) {
                        Column(gap: 4, alignment: Center) {
                            Icon(system: "person", description: "Profile", size: 20, tint: "#A1A1AA")
                            Text("Profile", color: "#A1A1AA")
                        }
                    }.onTap { selectedTab = 1 }
                }
            }
        }
    }
}
```

Nexa hides the native tab-bar chrome only when this block is present. `AppBottomBar` still switches between the declared tab destinations and retains their tab state; the custom block owns only the bar’s presentation and its controls. With the app-level selection binding, pushing another declared screen keeps the selected index; the pushed screen appears above the tab shell, and returning reveals the same selected tab.

## Adaptive master-detail

`NavigationSplitView` requires one `Sidebar` and one `Detail` child block, and a boolean `detailVisible` binding.

```nx
app Inventory {
    state showDetail: Bool = false

    body {
        NavigationSplitView(detailVisible: showDetail) {
            Sidebar {
                Column(gap: 8, padding: 16) {
                    Text("Inventory")
                    Button("Open low stock") { showDetail = true }
                }
            }
            Detail {
                Column(gap: 8, padding: 20) {
                    Text("Low stock items", fontSize: 22, fontWeight: Bold)
                    Text("Reorder ceramic travel mugs.")
                }
            }
        }
    }
}
```

## External links and modal content

`Link(url: ...)` opens a URL through the platform handler. Incoming links use the `deepLinks` app configuration and a screen slug. Scalar screen arguments can follow the slug as path segments in declaration order, or use query parameters named after the screen arguments. Path values take precedence over query values. For example, both `https://links.example.com/product-details/42/2` and `https://links.example.com/product-details?id=42&page=2` open `ProductDetails(id: "42", page: 2)`. Invalid argument values and extra path segments are ignored.

`Dialog` and `ConfirmationDialog` use mutable boolean state. `BottomSheet` uses
a typed component reference so presentation is an explicit action.

```nx
app AccountHelp {
    let helpSheetRef = ComponentRef<BottomSheet>()

    body {
        Column(gap: 12, padding: 16) {
            Button("Help options") { helpSheetRef.present() }
            Link(url: "https://support.example.com/account") {
                Text("Visit account help")
            }
            BottomSheet(ref: helpSheetRef, detents: [0.5, 1.0]) {
                Column(gap: 12, padding: 16) {
                    Text("Account help")
                    Text("Need help signing in?")
                    Button("Close") { helpSheetRef.dismiss() }
                }
            }
        }
    }
}
```

## Lifecycle actions

Use `OnAppear` and `OnDisappear` in an app body or screen. The app body may also declare `OnActive`, `OnInactive`, or `OnBackground`. An asynchronous `OnAppear async` can call an app-local `async fn` or an asynchronous typed plugin/API call with `await`.

```nx
app Welcome {
    state visits: Int32 = 0

    screen Home {
        OnAppear {
            visits = visits + 1
        }
        Column(gap: 8, padding: 16) {
            Text("Welcome back")
            Text("Screen visits: \(visits)")
        }
    }

    body {
        NavigationStack(root: Home)
    }
}
```

See the [plugin guide](plugins.md) for typed reactive signals and the [language guide](language-guide.md) for state, types, and collection updates.
