# Universal Native Widgets 📱

Nexa allows you to author cross-platform home screen widgets directly in `.nx`. The compiler generates native **SwiftUI WidgetKit** extensions for iOS and **Jetpack Glance** AppWidget providers for Android with zero manual platform bridging.

---

## 1. Widget Declaration Anatomy

Widgets are declared at the top level of any `.nx` file using the `widget` keyword:

```nexa
struct TaskSummary {
    title: String,
    pendingCount: Int32,
    overdueCount: Int32,
    accentColor: String
}

widget QuickTasks(
    displayName: "Task Summary",
    description: "Glance at your remaining tasks for today.",
    families: [Small, Medium, Large],
    refreshSeconds: 1800,
    placeholder: TaskSummary("Loading tasks...", 0, 0, "#38BDF8"),
    entry: await loadTaskSummary()
) {
    Column(spacing: 8, padding: 12) {
        Row(spacing: 6) {
            Icon(system: "checklist", size: 16, tint: entry.accentColor)
            Text(entry.title).fontSize(14).bold()
        }

        Spacer()

        Row(spacing: 12) {
            Column(spacing: 2) {
                Text(entry.pendingCount).fontSize(22).bold().foregroundColor("#FFFFFF")
                Text("Pending").fontSize(11).foregroundColor("#94A3B8")
            }
            if entry.overdueCount > 0 {
                Column(spacing: 2) {
                    Text(entry.overdueCount).fontSize(22).bold().foregroundColor("#EF4444")
                    Text("Overdue").fontSize(11).foregroundColor("#EF4444")
                }
            }
        }
    }
}
```

---

## 2. Widget Parameter Reference

| Parameter | Type | Required | Description |
|---|---|---|---|
| `displayName` | `String` | Yes | Human-readable title displayed in the system widget gallery. |
| `description` | `String` | Yes | Subtitle in the widget picker explaining the widget's function. |
| `families` | `Array<WidgetFamily>`| Yes | Target sizes: `Small`, `Medium`, `Large`, `ExtraLarge`. |
| `refreshSeconds` | `Int32` | Yes | Minimum periodic timeline refresh interval in seconds (clamped to 1800s on Android). |
| `placeholder` | `T` | Yes | Synchronous skeleton model rendered while loading or when redacted. |
| `entry` | `T` or `await T` | Yes | Evaluated data snapshot rendered by the widget body. |
| `configuration` | `StructInstance?` | No | Optional user-customizable App Intent / Preference configuration model. |

---

## 3. Supported Widget Families

Nexa maps widget families to native system slots:

| Family | iOS WidgetKit Target | Android Glance Size Slot | Recommended Content |
|---|---|---|---|
| `Small` | `systemSmall` (2x2 grid) | 2x2 launcher cell | Single metric, badge, or primary counter. |
| `Medium` | `systemMedium` (4x2 grid) | 4x2 launcher cell | Summary card, compact 2-3 row task list. |
| `Large` | `systemLarge` (4x4 grid) | 4x4 launcher cell | Rich list of 5-8 items with detailed metadata. |
| `ExtraLarge` | `systemExtraLarge` (iPad only) | Tablet expanded cell | Multi-column dashboard view. |

---

## 4. Portable Widget Body Subset

Because native home screen widgets run in separate lightweight process hosts (WidgetKit on iOS and RemoteViews via Glance on Android), only a dedicated, performant subset of UI nodes is accepted in widget bodies:

| Supported in Widgets ✅ | Unsupported in Widgets (Compile Error) ❌ |
|---|---|
| `Text`, `Icon`, `Image` | `TextInput`, `Slider`, `Picker`, `DatePicker` |
| `Column`, `Row`, `Stack`, `Spacer`, `Divider` | `BottomSheet`, `Dialog`, `ConfirmationDialog` |
| `FastList` (bounded with `.take(count)`) | Infinite scrolling or unbonded lists |
| Background colors, padding, corner radii | Complex dynamic GPU shaders or animations |
| `Link` (Static URL open actions) | Custom stateful mutation closures |

---

## 5. Shared Cross-Process Storage

When your app and widget share persistent state (such as SQLite or MMKV):

### iOS (App Groups)
In `nexa.config.nx`:
```nexa
config {
    ios {
        appGroupIdentifier: "group.dev.nexa.quicktasks"
    }
}
```

### Shared SQLite Access
```nexa
let database = SQLite.Database(
    name: "tasks.db",
    sharedWithWidgets: true
)
```
- On iOS: The database file is placed inside `containerURL(forSecurityApplicationGroupIdentifier:)`.
- On Android: Reads directly from the app's multi-process shared database directory with WAL mode enabled.
