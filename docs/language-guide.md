# Nexa Language Guide

Nexa source files use the `.nx` extension. Nexa checks types at compile time and generates SwiftUI for iOS or Jetpack Compose for Android.

## Scalar types

The scalar types accepted in app and component declarations are:

| Nexa type | Swift type | Kotlin type |
|---|---|---|
| `String` | `String` | `String` |
| `Bool` | `Bool` | `Boolean` |
| `Int8` | `Int8` | `Byte` |
| `Int16` | `Int16` | `Short` |
| `Int32` | `Int32` | `Int` |
| `Int64` | `Int64` | `Long` |
| `UInt8` | `UInt8` | `UByte` |
| `UInt16` | `UInt16` | `UShort` |
| `UInt32` | `UInt32` | `UInt` |
| `UInt64` | `UInt64` | `ULong` |
| `Float32` | `Float` | `Float` |
| `Float64` | `Double` | `Double` |

In Nexa source, use `Float32` and `Float64`; `Float` and `Double` are native output names, not Nexa type names. Integer literals infer as `Int32`, and decimal literals infer as `Float64` when the expected type does not specify another numeric type.

`Void` is not a general app value type. It is used as the return type of void methods in native plugin contracts.

Core API calls accept positional arguments in the parameter order shown in this
guide. You can keep a parameter name when it makes a call clearer, including
after positional arguments:

```nexa
let theme = Storage.getString("theme")
Storage.setString("theme", value: "dark")
let price = Number.formatCurrency(1234.5, "EUR")
```

Positional and named arguments lower to the same statically typed native call;
you can use named arguments for optional settings or when the order is unclear.

## Classes

Use a class when a value has stable identity, owns a long-lived resource, or
groups behavior with immutable state. Constructor `val` parameters become
stored properties. Body `let` properties are initialized once when the native
object is constructed. Instance methods use `this` when needed; a field can
also be referred to by its name inside a method.

```nexa
class SettingsStorage(val storageID: String) {
    static let defaultStorageID: String = "settings"
    static fn forDefaultStorage() -> SettingsStorage {
        return SettingsStorage(SettingsStorage.defaultStorageID)
    }
    let store = MMKV.MMKVStore(storageID, null, false)

    fn loadTheme() -> String? {
        return store.getString("theme")
    }
}

let settingsStorage = SettingsStorage("settings")
let theme = settingsStorage.loadTheme()
let defaultID = SettingsStorage.defaultStorageID
let defaultStorage = SettingsStorage.forDefaultStorage()
```

Imported `.nx` declarations are available throughout the app after import
resolution. A module-level instance is emitted once and can be shared by
components without constructing storage objects during renders or disposing
the native MMKV handle after each call. Nexa emits a native Swift class on iOS
and Kotlin class on Android; class values have reference identity. The initial
class slice supports immutable properties, typed instance methods, immutable
class-level `static let` properties, and typed `static fn` methods. Static
values and functions use `ClassName.member` syntax; Swift emits `static` class
members and Kotlin emits companion members. Static functions may be async with
`static async fn` and must be called with `await`. Interfaces, inheritance,
overloads, generics, throwing methods, and implicit disposal are not supported.

## Optional and collection types

- `T?` is an optional value.
- `Array<T>` is an ordered collection. It maps to a Swift array and a Kotlin `List<T>`.
- `Set<T>` is a set.
- `Map<K, V>` is a key-value map.
- `Pair<A, B>` and `Triple<A, B, C>` are fixed-size tuples.
- `Result<T, E>` represents a success value or a typed error value.

Set elements and map keys must be supported scalar/hashable types. Nexa collection syntax uses `Array<T>`, not `List<T>`; `List` is the Kotlin target type emitted for `Array<T>`.

Mutable collection state supports direct action methods: arrays use
`append(value)`, `remove(index)`, `move(from, to)`, and
`moveSubset(from, to, orderedSubset)`; sets use
`insert(value)` and `remove(value)`; maps use `set(key, value)` and
`remove(key)`. `Array.move` takes two `Int32` indexes and moves the source
element to the destination index when both indexes exist; invalid indexes are
ignored. It is a collection operation; list drag gestures remain a separate UI
capability.
`Array.moveSubset` is for moving rows from a stable-order filtered view. It
updates only the corresponding entries in the backing array, leaving hidden
entries in place. The subset must preserve the backing array's order and must
not be an independently sorted projection.

## Apps, state, and conditions

An app has one `body` containing its view tree. Declare mutable UI state with `state`:

```nexa
app Counter {
    state count: Int32 = 0

    body {
        Column(spacing: 12) {
            Text("Count")
            Text(count)
            Button("Increment") {
                count = count + 1
            }
            if count > 10 {
                Text("More than ten")
            }
        }
    }
}
```

Swift output stores this with SwiftUI state. Kotlin output uses Compose state, selecting primitive state holders such as `mutableIntStateOf` for `Int32`. `let` declares an immutable binding; `state` declares a mutable binding. A binding can omit its type when its initializer determines one unambiguous type.

## Arithmetic

Numeric expressions support `+`, `-`, `*`, `/`, and `%`, plus unary negation. Multiplication, division, and remainder bind more tightly than addition and subtraction; parentheses can make grouping explicit. Integer division truncates toward zero. Fixed-width integer addition, subtraction, multiplication, and negation wrap on overflow.

The `+` operator also concatenates two `String` values. It emits the native Swift or Kotlin string operator. The unary `!` operator negates a `Bool`.

Strings expose the typed `trimmed` property to remove leading and trailing whitespace and newlines: `let cleanName = name.trimmed`. It lowers to the native string trimming APIs on iOS and Android.

Arrays, sets, and maps expose `count` as `Int32` and `isEmpty` as `Bool`:

```nexa
app CollectionExample {
    state names: Array<String> = ["Nexa"]

    body {
        Column {
            Text(names.count)
            Text(names.isEmpty)
        }
    }
}
```

Arrays also support `random()`, `first()`, `last()`, `shuffled()`, `reverse()`, `slice(range)`, and `take(count)`:

```nexa
app ArrayUtilities {
    state values: Array<Int32> = [10, 20, 30, 40]
    state selected: Int32? = values.random()
    state first: Int32? = values.first()
    state last: Int32? = values.last()
    state randomized: Array<Int32> = values.shuffled()
    state reversed: Array<Int32> = values.reverse()
    state middle: Array<Int32> = values.slice(1..<3)
    state firstThree: Array<Int32> = values.take(3)

    body {
        Text(selected ?? 0)
        Text(first ?? 0)
        Text(last ?? 0)
        Text(randomized.count)
        Text(reversed.count)
        Text(middle.count)
        Text(firstThree.count)
    }
}
```

Use `sortedBy` with a closure that returns a `String` or numeric key to create a
sorted copy of an array. The source array keeps its existing order.

```nexa
app SortExample {
    state values: Array<Int32> = [3, 1, 2]

    body {
        FastList(values.sortedBy { value -> value }) { value, index in
            Text(value)
        }
    }
}
```

`random()`, `first()`, and `last()` return an optional element because an empty array has no value to return. `first()` and `last()` access the existing array without creating a copy. `shuffled()`, `reverse()`, `slice`, and `take` return a new array. `slice` takes one unstepped range; `..` includes its upper index and `..<` excludes it. Indices must be valid for the source array. `take(count)` returns up to `count` elements from the start, and treats a negative count as zero on both platforms.

Arrays also support typed closure transforms with `map`, `flatMap`, `filter`, `reduce`, and `groupedBy`. `flatMap` maps each source item to an array and concatenates those results in source order. `groupedBy` accepts a `String` or numeric key and keeps groups in the order their keys first appear. Use it after sorting when section order should follow the sorted source. Use the infix `in` operator for membership: arrays and sets check scalar elements, maps check scalar keys, and strings check for a case-insensitive substring. For example, `"meeting" in task.content` matches `"Team Meeting"` on iOS and Android.

Text styles can be chained after a component call. For example, `Text("Hi").fontSize(18).bold().padding(12)` is equivalent to setting `fontSize: 18`, `fontWeight: Bold`, and `padding: 12` in the `Text` arguments. `Text`, `Column`, `Row`, and `Stack` also accept `.opacity(value)`, `.scale(value)`, `.rotation(degrees)`, `.shadow(radius: 8, x: 0, y: 4, color: "#00000040")`, `.blur(radius)`, `.clip(shape: Rounded(radius))`, and `.zIndex(value)`.

Layout `animation` accepts `Spring`, `EaseIn`, `EaseOut`, `EaseInOut`, or `Linear`. Customize spring response time in seconds and damping ratio with `Spring(response: 0.35, damping: 0.8)`; omitted values use `0.5` seconds and `0.825`. Response and damping must be finite and greater than zero. SwiftUI receives these values directly. Compose uses the damping ratio directly and maps response to a native spring stiffness, so the motion is platform-native and approximate rather than frame-for-frame identical.

Use `withAnimation` to animate mutable floating-point state updates made by one synchronous action block:

```nexa
app AnimatedProgress {
    state progress: Float64 = 0.0

    body {
        Button("Complete") {
            withAnimation(Spring(response: 0.35, damping: 0.8)) {
                progress = 1.0
            }
        }
        ProgressBar(progress: progress)
    }
}
```

`withAnimation` blocks contain mutable `Float32` or `Float64` state assignments, optionally inside control flow. Empty blocks, unrelated side effects, other state types, collection mutations, asynchronous work, and deferred event or task registration are rejected. SwiftUI uses a native animation transaction; Compose animates the rendered floating-point value while the stored state changes immediately.

Conditional view blocks accept `.transition(.fade)`, `.transition(.scale)`, or `.transition(.slide(from: .bottom))`, `.transition(.slide(from: .left))`, and `.transition(.slide(from: .right))`. The transition runs when an `if` branch or `when` case changes. Use opposite horizontal edges on neighboring branches to animate a selection marker between them:

```nexa
app ConditionalTransitions {
    state expanded: Bool = false

    body {
        Button("Toggle") {
            expanded = !expanded
        }
        if expanded {
            Text("Details")
        }.transition(.fade)
        when expanded {
            true: { Text("Expanded") }
            else: { Spacer() }
        }.transition(.slide(from: .bottom))
    }
}
```

These modifiers are part of the typed module and DevRuntime payload, so changing the transition remains compatible with hot reload after the app has a DevRuntime build.

`Spacer()` consumes remaining space on a `Row` or `Column` axis. `Divider(color: "#808080", thickness: 1)` inserts a horizontal separator.
`Slider(value: amount, min: 0.0, max: 1.0, step: 0.01)` binds a stepped native slider to mutable `Float64` state.
`ProgressBar(progress: amount)` and `ProgressRing(progress: amount)` render normalized linear and circular progress controls.
`SegmentedControl(items: filters, selected: selectedFilter)` binds a compact native selector to mutable `String` state.
`Picker(items: sizes, selected: selectedSize)` binds a native menu selector to mutable `String` state.
`Dialog(isPresented: show, title: "Confirm", message: "Continue?") { Button("OK") { show = false } }` presents a native alert tied to mutable Boolean state.

## Value-producing conditionals

An `if` expression returns the value of the branch that runs. You can also write the same conditional as `condition ? value : fallback`. Both branches must have the same type, except that a value and `null` produce an optional value:

```nexa
app ConditionalExample {
    state enabled: Bool = true
    state title: String = if enabled { "Ready" } else { "Waiting" }
    state selected: String? = if enabled { "Ready" } else { null }

    body {
        Text(title)
        Text(enabled ? "Ready" : "Waiting")
        Text(selected ?? "Unavailable")
    }
}
```

The condition must be a `Bool`. The compiler removes an unreachable branch when the condition is a constant.

Inside an action, mutable state supports compound assignments: `+=`, `-=`, `*=`, `/=`, and `%=`. Each is equivalent to assigning the current value combined with the right-hand value by the matching binary operator:

```nexa
app Counter {
    state count: Int32 = 0

    body {
        Button("Update") {
            count += 2 * 3
            count -= 1
            count *= 2
        }
    }
}
```

## Structs, enums, and functions

Value structs are declared at the top level. Closed enums and functions are declared inside an app; functions have explicit parameter and return types:

```nexa
struct UserProfile {
    id: Int64,
    username: String,
}

app ProfileApp {
    enum LoadState {
        idle,
        loading,
        loaded
    }

    fn greeting(name: String) -> String {
        return name
    }

    body {
        Text(greeting("Nexa"))
    }
}
```

App-local functions currently use typed parameters, immutable local `let` bindings, and one return expression. Struct constructors take field values in declaration order.

Imported source modules can also declare immutable file-scope `let` bindings.
The compiler emits each as one native module-level value (`private let` in
Swift and `private val` in Kotlin), initialized lazily once per process. This
is appropriate for long-lived services such as a SQLite connection owned by a
standalone persistence module; it is not UI state and is never recreated by
view recomposition. Mutable `state` remains app or screen scoped.

```nexa
plugin "dev.nexa.sqlite" as SQLite

let todoDatabase = SQLite.Database("todo", true)

struct TaskCount {
    value: Int64
}

async fn loadOpenTaskCount() -> Int64 {
    let rows: Array<TaskCount> = await todoDatabase.query(
        "SELECT COUNT(*) AS value FROM tasks WHERE isCompleted = ?",
        [false]
    )
    return rows.first()?.value ?? 0
}
```

### SQLite plugin calls and reactive queries

SQLite is used through ordinary Nexa classes and functions. A top-level value
can retain the long-lived native database handle, while migrations and SQL stay
as ordinary typed values:

```nexa
plugin "dev.nexa.sqlite" as SQLite

struct TodoItem {
    id: Int64,
    title: String,
    completed: Bool,
}

let todoDatabase = SQLite.Database("todo", true)
let todoMigrations = [
    SQLite.Migration(1, [
        "CREATE TABLE todos (id INTEGER PRIMARY KEY, title TEXT NOT NULL, completed INTEGER NOT NULL DEFAULT 0)"
    ])
]

app TodoApp {
    state todos: Array<TodoItem> = []

    body {
        OnAppear async {
            try {
                let version = await todoDatabase.migrate(todoMigrations)
                todos = await todoDatabase.query<TodoItem>(
                    "SELECT id, title, completed FROM todos WHERE completed = ? ORDER BY id DESC",
                    [false]
                )
            } catch {
                else { Log.error(message: "Could not open the todo database") }
            }
        }

        Text("Todo list")
    }
}
```

When the handle and migration history are statically resolvable, the compiler
replays the migrations and checks literal typed-query SQL against the resulting
schema. Query results map by field name, use native typed column reads, and
require optional fields for columns that may be `NULL`. SQL bind slots must
match the parameter array. The compiler uses a bundled SQLite engine; generated
apps use the operating system's SQLite library. Project checks warn when known
SQL features may exceed the configured platform minimum.

Use `queryRaw` and `executeRaw` for dynamic SQL. Typed `query` and `execute`
remain ordinary plugin calls and receive compile-time checks when their SQL is
static. For reactive queries, `observeQuery<T>(sql, params)` returns a typed
`Signal<Array<T>>` that invalidates automatically when underlying tables are
mutated via `executeTracked` or cross-process notifications. `observeTables` and
`attachTables` provide explicit table-targeted invalidation. Widget refresh
timing remains controlled by iOS WidgetKit or Android's widget host.

### In-language tests

Declare named tests at the top level of any imported `.nx` file. A test may
bind immutable local values, call synchronous app-local functions using
positional or named arguments, and assert Boolean expressions:

```nexa
fn calculateTotal(price: Float64, qty: Float64) -> Float64 {
    return price * qty
}

test "cart item calculation" {
    let result = calculateTotal(price: 50.0, qty: 2)
    assert(result == 100.0)
    assert(result > 0.0, "the total must be positive")
}
```

Tests can also exercise a custom component without launching a simulator. Add
`for Component(arguments)` to mount one component, use `tap("Button label")`
to run the first matching visible `Button` or text-labeled `Pressable` action,
assert its state by name, and check exact visible text with `assertText`:

```nexa
component Counter(start: Int32) {
    state count: Int32 = start
    body {
        Text("Count: $count")
        Button("Increment") { count += 1 }
    }
}

test "counter increments" for Counter(start: 4) {
    assert(count == 4)
    assertText("Count: 4")
    tap("Increment")
    assert(count == 5)
    assertText("Count: 5")
}
```

Headless component tests can interact with forms and native-backed controls
without launching a simulator. Use `typeText("placeholder", "value")` and
`submit("placeholder")` for `TextInput`, `toggle("label")` for `Switch`,
`slide("state", value)` for `Slider`, and `select("state", "option")` for a
`Picker` or `SegmentedControl`. `assertComponent("Namespace.Name")` verifies a
native plugin component is rendered, and `emit("Namespace.Name",
"onEventName")` invokes a declared zero-payload callback. Assertions can check
component state and visible text after each interaction.

Mounted component actions support state assignment, expressions, conditionals,
`for` and `while` loops, loop control, deterministic collection mutations, and
catch-all `try` actions. The evaluator handles typed collection literals,
indexing, membership, member counts, tuple and struct fields, collection
transforms, and deterministic collection utilities. Randomized collection
utilities are rejected to keep host tests deterministic. Platform APIs,
native plugin methods and property reads, payload-bearing native component
events, typed native plugin failures, and asynchronous task or event
registration remain outside the headless evaluator; use native host tests for
those behaviors.

`nexa test` evaluates these typed, deterministic checks before compiling the
native test hosts. Use `nexa test --unit-only` to run them without Xcode or the
Android toolchain. Test blocks are host-side compiler input and are omitted
from generated application code. Tests can call pure synchronous `.nx`
functions; platform APIs, plugin calls, and asynchronous functions cannot run
inside the host evaluator.

## Results and postfix `?`

Use an enum for the error type so the result can map to Swift's `Result`, whose failure type must conform to `Error`:

```nexa
app ResultExample {
    enum AppError {
        unavailable
    }

    fn loadValue() -> Result<Int32, AppError> {
        return Ok(42)
    }

    fn useValue() -> Result<Int32, AppError> {
        let value = loadValue()?
        return Ok(value)
    }

    state result: Result<Int32, AppError> = Ok(0)

    body {
        Column {
            Text("Result example")
            Button("Load") {
                result = useValue()
            }
        }
    }
}
```

`Ok(value)` and `Err(error)` construct results. On Swift, postfix `?` generates `try result.get()`, which throws the typed failure. On Kotlin, Nexa emits the sealed `NexaResult<T, E>` type; postfix `?` currently calls `getOrThrow()`, which turns a failure into a `RuntimeException`. It does not return `nil` on either target.

## JSON

`Json.parse<T>(text)` decodes a concrete Nexa value type and returns `Result<T, JsonError>`. `Json.stringify(value)` encodes a value to a JSON string. Named forms (`raw: text` and `value: profile`) are also accepted. The type is resolved at compile time, so model decoding uses generated typed codecs rather than reflection or an untyped object map.

Supported values include strings, booleans, numbers, bytes, optionals, arrays, sets, string-keyed maps, pairs, triples, results, declared enums, and declared structs composed from those types. Bytes use Base64 strings; enums use their case names; pairs and triples use JSON arrays; results use a one-key `success` or `failure` object. Maps require `String` keys. Unknown struct fields are ignored while decoding, and missing required fields return `JsonError.missingField`.

```nexa
struct UserProfile {
    name: String,
    age: Int32,
}

app JsonExample {
    fn parseAndEncode(raw: String) -> String {
        let profile: Result<UserProfile, JsonError> = Json.parse<UserProfile>(raw)
        return Json.stringify(profile)
    }

    state raw: String = Json.stringify(UserProfile("Ada", 37))
    state status: String = "Not parsed yet"

    body {
        Text(raw)
        Text(status)
        Button("Validate JSON") {
            status = parseAndEncode(raw)
        }
    }
}
```

Parse failures are returned as `JsonError.invalidJson`, `typeMismatch`, `missingField`, or `invalidValue`; callers can inspect the result or propagate it with `?` from a function that returns a compatible `Result`.

## Network connectivity

`Network.isOnline` synchronously reports whether the device currently has a network path. It does not guarantee that a particular server is reachable. The Android implementation checks the active network's internet capability; iOS observes the current path with `NWPathMonitor`.

```nexa
app ConnectionExample {
    state online: Bool = Network.isOnline

    body {
        Column {
            Text(online ? "Online" : "Offline")
            Button("Watch changes") {
                Network.onStatusChange { isOnline -> online = isOnline }
            }
        }
    }
}
```

The property gives an immediate path check. `Network.onStatusChange` subscribes to later changes and passes the new Boolean value to its callback. Neither API guarantees that a particular server is reachable.

`Network.upload(url:file:fields:)` sends a local file and string fields as a `multipart/form-data` POST and returns the normal `NetworkResponse`. Both platform clients stream the file without assembling the full request body in memory. Android streams multipart segments directly through Cronet; iOS uses a temporary body file with URLSession and removes it after the request completes.

```nexa
app ReceiptUploader {
    state statusCode: Int32 = 0

    body {
        Text("Receipt upload")
        OnAppear async {
            try {
                statusCode = (await Network.upload(
                    url: "https://api.example.com/receipts",
                    file: "/local/path/receipt.pdf",
                    fields: ["kind": "receipt"]
                )).statusCode
            } catch {
                statusCode = -1
            }
        }
    }
}
```

## Time

`Time` is a core API, available on every target without a plugin:

| Call | Returns | Meaning |
|---|---|---|
| `Time.now()` | `Int64` | Milliseconds since the Unix epoch, UTC |
| `Time.monotonic()` | `Int64` | Nanoseconds from a fixed origin; only differences are meaningful |
| `Time.elapsed(since: timestamp)` | `Int64` | Nanoseconds since a value returned by `Time.monotonic()` |
| `Time.sleep(milliseconds: Int64)` | `Void` | Suspends the caller |
| `Time.iso8601(timestamp: Int64)` | `String` | The instant as `yyyy-MM-ddTHH:mm:ss.SSSZ` in UTC |
| `Time.iso8601ToMillis(text: String)` | `Int64?` | The inverse, or `null` when the text is not that layout |
| `Time.startOfDay(timestamp: Int64)` | `Int64` | Local calendar start of the timestamp's day |
| `Time.addCalendarDays(timestamp: Int64, days: Int32)` | `Int64` | Advances by local calendar days while preserving wall-clock time across daylight-saving changes |
| `Time.localizedDate(timestamp: Int64)` | `String` | Native-locale date, using the device's current locale and time zone |
| `Time.localizedTime(timestamp: Int64)` | `String` | Native-locale time, using the device's current locale and time zone |
| `Time.localizedDateTime(timestamp: Int64)` | `String` | Native-locale date and time, using the device's current locale and time zone |
| `Time.format(timestamp: Int64, pattern: String)` | `String` | Formats with the device locale and time zone using a cached native formatter |

Use calendar arithmetic for relative dates rather than adding a fixed number of milliseconds:

```nexa
let tomorrow = Time.addCalendarDays(timestamp: Time.now(), days: 1)
```

Use the localized formatters for labels intended for people. Timestamps are milliseconds since the Unix epoch, just like `Time.now()`; each platform chooses its native short date and short time styles:

```nexa
let dueAt: Int64 = Time.now()
let dueDate = Time.localizedDate(dueAt)
let dueTime = Time.localizedTime(dueAt)
let dueLabel = Time.localizedDateTime(dueAt)
let taskDate = Time.format(dueAt, "d MMM, HH:mm")
```

Use `Time.format` when the app needs the same date layout as its design on both platforms. It uses the current device locale and time zone, and caches the native formatter by locale, time zone, and pattern so repeated list rows reuse it:

```nexa
let date = Time.format(dueAt, "d MMM")
let dateAndTime = Time.format(dueAt, "d MMM, HH:mm")
```

The two clocks are separate on purpose. `Time.now()` is a wall clock: it can jump when the system time is corrected, so it answers *when* something happened. `Time.monotonic()` never moves backwards, so it is the only one that can answer *how long* something took:

```nexa
app TimerExample {
    state startedAt: Int64 = 0
    state finishedAt: Int64 = 0
    state stamp: String = ""

    body {
        Column(spacing: 16) {
            Text(stamp)
        }

        OnAppear async {
            startedAt = Time.monotonic()
            stamp = Time.iso8601(timestamp: Time.now())
            try {
                await Time.sleep(milliseconds: 50)
            } catch {
            }
            finishedAt = Time.monotonic()
        }
    }
}
```

`Time.sleep` is the one clock call that can throw, because the surrounding task can be cancelled while it is suspended, so `await Time.sleep(...)` needs a `try { ... } catch { ... }` block or a throwing function, exactly like an awaited `File` or `Network` call. Awaiting a clock *reading* is an error: only `sleep` suspends.

Use `Time.elapsed(since:)` to measure durations without mixing the monotonic clock with wall-clock timestamps:

```nexa
let startedAt = Time.monotonic()
// Run the work being measured.
let elapsedNanoseconds = Time.elapsed(since: startedAt)
```

`Time.iso8601` and `Time.iso8601ToMillis` are hand-implemented on both platforms with the same leap-year rules rather than a formatter, so a timestamp written on one target reads on the other. The layout is UTC and locale-independent, and the fractional part is optional when parsing. `Time.startOfDay` uses the device's current calendar and returns a timestamp suitable for app-defined grouping keys. `Time.addCalendarDays` also uses the current calendar, so advancing over a daylight-saving boundary preserves the local wall-clock time instead of adding a fixed 24 hours. The `localized*` methods intentionally follow the operating system's locale, time zone, and native formatting conventions, so their exact strings may differ between iOS and Android.

## Alternate app icons

Declare alternate icon assets in `nexa.config.nx`. iOS entries point to Xcode `.icon` assets. Android alternate icons must be icon-set directories so every selectable icon has adaptive foreground and background layers. A default Android icon must also be configured through `android.icon` or the shared `assets.icon`. Matching directory names share the same name in app code:

```nexa
ios {
    icon: "assets/Icon.icon",
    alternateIcons: ["assets/IconBlue.icon", "assets/IconGreen.icon"]
}
android {
    icon: "assets/icons/Default",
    alternateIcons: ["assets/icons/Blue", "assets/icons/Green"]
}
```

An Android icon-set directory contains the legacy `icon.png` fallback and separate `foreground` and `background` layers. Each layer may be a square PNG or an Android drawable XML file; `monochrome.png` or `monochrome.xml` is optional. Nexa emits density-sized legacy assets, adaptive icon XML for Android 8.0+, and the monochrome element for Android 13+ when present. Sibling icon sets can reuse `shared/foreground.*` and `shared/monochrome.*` files from their common parent. Keep foreground artwork inside the centered 66-by-66 dp safe zone of the 108-by-108 dp layer. The primary icon may still be a flat PNG when adaptive layers are not needed.

```text
assets/icons/
├── Default/
│   ├── icon.png
│   └── background.xml
├── Blue/
│   ├── icon.png
│   └── background.xml
└── shared/
    ├── foreground.xml
    └── monochrome.xml
```

Set a configured icon with `AppIcon.set(name)`. Passing `null` restores the default icon. The result is `false` when the platform cannot apply that icon or the name is not configured:

```nexa
Task.launch {
    let applied = await AppIcon.set("IconBlue")
}
```

The compiler registers iOS alternate icons in the generated Xcode target and Android icons as launcher aliases. The API is native on both platforms and does not require a plugin.

## Currency formatting

`Number.formatCurrency(amount: Float64, currencyCode: String)` formats a value using the device's current locale and the requested ISO 4217 currency code. For example, a French device may display `1234.5` in EUR as `1 234,50 €`, while an English device uses its local grouping and symbol placement. An unrecognized currency code falls back to locale-aware decimal formatting rather than throwing.

```nexa
app PriceExample {
    let price: String = Number.formatCurrency(amount: 1234.5, currencyCode: "EUR")

    body {
        Text(price)
    }
}
```

The formatter uses the platform locale at the time of the call. Updating device locale or currency data can therefore change its output. The call is supported by the native AOT backends and DevRuntime.

## Language and locale

Nexa localizes UI copy from the text already written in `.nx`; translation keys are never authored by hand. Add translator context where it helps explain intent:

```nexa
Text("Save", comment: "Button to persist the user's profile")
Text("Welcome, \(userName)!", comment: "Greeting shown at the top of the screen")
Button("Continue", comment: "Moves to the next onboarding step") { page = page + 1 }
TextInput(value: userName, placeholder: "Your name", comment: "Prompt for the person's name")
```

The first project compilation creates `locales/translations.json`; later project compilations synchronize it with the UI text in the AST, and write the file only when its contents change. The file keeps source text as the key and stores translator values by BCP 47 language tag. Nexa preserves translations for unchanged source text, so you only need to translate new or changed strings. Since the literal is the key, editing a string creates a new entry and drops the old entry; removing a string also removes its entry. Existing translations for all other strings remain intact.

```json
{
  "format": 1,
  "sourceLanguage": "en",
  "strings": {
    "Save": {
      "source": "Save",
      "comment": "Button to persist the user's profile",
      "arguments": [],
      "translations": { "fr": "Enregistrer" }
    },
    "{count} task in Inbox": {
      "source": "{count} task in Inbox",
      "comment": "Task count in the Inbox",
      "arguments": ["count"],
      "argument_types": ["Int32"],
      "translations": {
        "en": {
          "one": "{count} task in Inbox",
          "other": "{count} tasks in Inbox"
        },
        "fr": {
          "one": "{count} tâche dans la boîte de réception",
          "other": "{count} tâches dans la boîte de réception"
        }
      }
    }
  }
}
```

For interpolated copy, Nexa records each argument name and type. Translators can move placeholders, and integer `count` text produces native plural resources. Add forms for the source language too when its singular and plural wording differs; Nexa cannot infer plural wording from the source literal alone. iOS emits SwiftUI `Text` literals with their comments and a generated `Localizable.xcstrings` catalog. Android receives stable resource names, `strings.xml` or plural resources, and Compose `stringResource` calls. Missing translations fall back to the source text.

In `nexa dev`, edits to `locales/translations.json` travel over the existing WebSocket connection and update visible text in memory. Native resource files are regenerated for normal builds.

Language information remains available through the built-in APIs:

```nexa
let language: String = Locale.currentLanguageCode()
let preferred: Array<String> = Locale.preferredLanguageCodes()
let frenchName: String? = Locale.displayName("fr")
let frenchNameWithLabel: String? = Locale.displayName(languageCode: "fr")
let notificationTitle: String = Locale.localized("Reminder")
let welcomeMessage: String = Locale.localized("Welcome, \(name)!")
```

`currentLanguageCode()` returns the current language subtag (for example, `fr`). `preferredLanguageCodes()` returns the device's ordered BCP 47 preferences (for example, `fr-CA`, then `en-US`). `displayName` accepts either the simple positional form or its explicit parameter label.

Write user-visible text directly in UI components whenever possible; Nexa extracts those literals automatically. Use `Locale.localized("source text")` when a localized string must be passed to a native API, such as a notification title or email body. The source text itself is the translation key, and interpolated source text keeps its typed arguments. This API takes exactly one positional source-text argument and does not require a separate key or argument arrays. Extraction includes calls inside top-level functions, state initializers, lifecycle actions, screens, components, and widget providers.

## Cryptography

`Crypto` is a synchronous core API on iOS and Android. SHA digests and HMAC-SHA256 are returned as lowercase hexadecimal strings. `randomBytes` returns Base64 text generated by the platform's secure random source; counts at or below zero return an empty string.

```nexa
let digest: String = Crypto.sha256(text: "payload")
let signature: String = Crypto.hmacSha256(key: "secret", message: "payload")
let nonce: String = Crypto.randomBytes(count: 16)
```

The implementation uses CryptoKit and Security on iOS, and `MessageDigest`, `Mac`, and `SecureRandom` on Android. Calls use direct native helpers in release builds and are available to DevRuntime hot reload.

## Clipboard

`Clipboard.setText(text: String)` writes plain text to the system clipboard. `Clipboard.getText()` returns the current plain text as `String?`, and `Clipboard.hasText()` reports whether the clipboard contains text. The API uses `UIPasteboard` on iOS and the Android clipboard service; iOS may show its system paste permission prompt when an app reads clipboard content. Clipboard calls added to `.nx` code are available through DevRuntime hot reload.

## Storage

Use `Storage` for small, app-private string preferences. `getString(key:)` returns `String?`, `setString(key:value:)` stores a string, `delete(key:)` removes one entry, and `clear()` removes entries written through this API:

```nexa
app Preferences {
    state theme: String = Storage.getString(key: "theme") ?? "system"

    body {
        Button("Dark theme") {
            theme = "dark"
            Storage.setString(key: "theme", value: theme)
        }
    }
}
```

The iOS implementation uses the app's `UserDefaults` domain and the Android implementation uses a dedicated private `SharedPreferences` file. Calls are synchronous and available through DevRuntime hot reload. Use `SecureStorage` for credentials and other secrets; ordinary `Storage` is not encrypted.

Generated Android hosts follow the system day/night setting: their Material 3 color scheme and native window theme switch together. Generated iOS hosts follow the system appearance by default. Wrap content in `Appearance(mode: "system" | "light" | "dark")` to override the scheme, and pass a `String` state to switch it at runtime. Store the selected value with `Storage` or another persistence package when the app needs to remember the user's choice.

## Background tasks

Declare periodic work at app scope. The task body is compiled into the native host and cannot read or mutate screen state, because the operating system may launch it after the app process has stopped:

```nexa
app Weather {
    background task RefreshWeather(
        identifier: "dev.example.weather.refresh",
        everyMinutes: 60
    ) {
        Storage.setString(
            key: "weather.lastRefresh",
            value: Time.iso8601(timestamp: Time.now())
        )
    }

    body {
        Text("Weather")
    }
}
```

The interval must be at least 15 minutes so the declaration is valid on Android. WorkManager and iOS background refresh are best-effort: the operating system chooses when eligible work runs and may defer it for power or scheduling reasons. Do not use periodic work for exact alarms or continuous execution. iOS hosts register each reverse-domain task identifier and enable Background Fetch; Android hosts use a persistent WorkManager worker. The Android WorkManager dependency is included only when the app declares a background task.

Task handlers are AOT code. In a `nexa dev` session, editing a background task declaration or body causes the native host to rebuild and relaunch automatically; UI-only edits continue to hot reload. See [`background_task.nx`](../examples/background_task.nx).

### Foreground tasks

Use `Task.launch(executor: ...)` for lifecycle-scoped fire-and-forget work. Each launch runs independently, so a later launch does not cancel an earlier one:

```nexa
Button("Refresh") {
    Task.launch(executor: TaskExecutor.Main) {
        refreshData()
    }
}
```

Use a `TaskHandle?` state when you need to retain and cancel one native task. Launching into the same handle cancels its previous task first:

```nexa
app TaskExample {
    state refreshTask: TaskHandle? = null
    state status: String = "Ready"

    body {
        Button("Refresh") {
            Task.launch(handle: refreshTask, executor: TaskExecutor.Main) {
                status = "Updated"
            }
        }
        Button("Cancel") {
            Task.cancel(handle: refreshTask)
        }
        Text(status)
    }
}
```

`TaskExecutor.Main` uses Swift's main actor or Compose's main dispatcher. `TaskExecutor.Background` uses a native background executor and cannot read or mutate UI state. Tasks are canceled when their owning app, screen, or component leaves its native view lifecycle; cancellation is cooperative. Catch typed plugin failures inside the task body. On Android, an uncaught task failure is logged.

Task launch and cancellation are part of DevRuntime IR, so task actions can hot reload after the development host has been built with this runtime. DevRuntime is an interpreter; use a release build when measuring native task throughput.

## Haptics

Use `Haptics.impact(style:)` for light, medium, or heavy impact feedback; `Haptics.notification(kind:)` for success or error feedback; and `Haptics.selection()` for selection changes. The style values are `Light`, `Medium`, and `Heavy`; notification values are `Success` and `Error`.

```nexa
Button("Save") {
    Haptics.notification(kind: Success)
}
```

Release builds call UIKit feedback generators on iOS and Android view haptic feedback constants on Android. DevRuntime supports all three calls, so adding them to app source can hot reload without rebuilding the host. Android uses the foreground activity's decor view and does not request the vibration permission.

## Screen orientation

Use `Screen.lockOrientation(mode:)` to request `Portrait`, `Landscape`, or `All`. On iOS, the active scene receives a geometry request; on Android, Nexa updates the foreground activity's requested orientation. The generated host declares the orientations these modes can select. The API is available in DevRuntime, so adding or changing the call supports hot reload.

```nexa
OnAppear {
    Screen.lockOrientation(mode: Landscape)
}
```

## Keyboard

`Keyboard.dismiss()` hides the software keyboard while leaving the focused input intact. Use `TextInput`'s `autofill` option to identify username, password, or one-time-code fields, and `returnKeyType` to choose Done, Search, Send, or Next. On Android the autofill hint is expressed through Compose semantics; on iOS it uses the native text content type.

## Logging

`Log` writes messages to the native platform log, visible in Android Logcat and the iOS Console:

```nexa
Log.info(message: "Loaded $count records")
Log.warning(message: "Using cached data")
Log.error(message: "The request failed")
```

## Platform-specific UI

Use `platform ios { ... }` and `platform android { ... }` for target-specific views. Nexa removes the inactive block before checking and generating the selected target:

```nexa
app PlatformExample {
    body {
        platform ios {
            Text("iOS view")
        }
        platform android {
            Text("Android view")
        }
    }
}
```

For supported components and their arguments, see [Components](components.md). For app and screen state and navigation, see [State and Navigation](state-and-navigation.md).
