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

## Optional and collection types

- `T?` is an optional value.
- `Array<T>` is an ordered collection. It maps to a Swift array and a Kotlin `List<T>`.
- `Set<T>` is a set.
- `Map<K, V>` is a key-value map.
- `Pair<A, B>` and `Triple<A, B, C>` are fixed-size tuples.
- `Result<T, E>` represents a success value or a typed error value.

Set elements and map keys must be supported scalar/hashable types. Nexa collection syntax uses `Array<T>`, not `List<T>`; `List` is the Kotlin target type emitted for `Array<T>`.

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
