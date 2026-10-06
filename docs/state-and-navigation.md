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
        Column(spacing: 12, padding: 16) {
            Text("Featured products", fontSize: 24, fontWeight: Bold)
            Text("Cart items: \(cartCount)", color: "#475569")
            FastList(products, key: .id, native: true) { product, index in
                NavigationLink(destination: ProductDetail(product.id)) {
                    Row(spacing: 12, padding: 12, background: "#F1F5F9", cornerRadius: 8) {
                        Text("\(index + 1). \(product.name)", fontWeight: Semibold)
                        Spacer()
                        Text(product.priceLabel, color: "#1D4ED8")
                    }
                }
            }
        }
    }

    screen ProductDetail(productID: Int32) {
        Column(spacing: 12, padding: 20) {
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
                Column(spacing: 8, padding: 16) {
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

## Adaptive master-detail

`NavigationSplitView` requires one `Sidebar` and one `Detail` child block, and a boolean `detailVisible` binding.

```nx
app Inventory {
    state showDetail: Bool = false

    body {
        NavigationSplitView(detailVisible: showDetail) {
            Sidebar {
                Column(spacing: 8, padding: 16) {
                    Text("Inventory")
                    Button("Open low stock") { showDetail = true }
                }
            }
            Detail {
                Column(spacing: 8, padding: 20) {
                    Text("Low stock items", fontSize: 22, fontWeight: Bold)
                    Text("Reorder ceramic travel mugs.")
                }
            }
        }
    }
}
```

## External links and modal content

`Link(url: ...)` opens a URL through the platform handler. Incoming universal-link routing is not part of the current Nexa syntax. `BottomSheet`, `Dialog`, and `ConfirmationDialog` are controlled by a mutable boolean binding.

```nx
app AccountHelp {
    state showHelp: Bool = false

    body {
        Column(spacing: 12, padding: 16) {
            Button("Help options") { showHelp = true }
            Link(url: "https://support.example.com/account") {
                Text("Visit account help")
            }
            BottomSheet(isPresented: showHelp, partial: true, title: "Account help") {
                Column(spacing: 12, padding: 16) {
                    Text("Need help signing in?")
                    Button("Close") { showHelp = false }
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
        Column(spacing: 8, padding: 16) {
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
