# Native Widgets

Nexa widget declarations generate native WidgetKit and Android app-widget hosts from `.nx` source. Widget entries are typed snapshots; an asynchronous entry provider needs a synchronous placeholder.

| **Scope**: native home-screen widgets | **Targets**: WidgetKit and Android app widgets | **Data**: typed timeline snapshots |

## Quick start: next task widget

This example expects the app to create a `tasks` table with `title`, `priority`, and `is_completed` columns. It uses the supplied SQLite plugin to fetch a snapshot for the widget timeline.

```nx
plugin "plugins/sqlite" as SQLite

struct TaskWidgetEntry {
    taskTitle: String,
    priority: String,
}

struct TaskRow {
    title: String,
    priority: String,
}

class TaskWidgetData {
    static let database = SQLite.Database("tasks", true)
    static let migrations = [
        SQLite.Migration(1, [
            "CREATE TABLE IF NOT EXISTS tasks (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, priority TEXT NOT NULL DEFAULT 'Normal', is_completed INTEGER NOT NULL DEFAULT 0)"
        ])
    ]

    static fn placeholder() -> TaskWidgetEntry {
        return TaskWidgetEntry("Loading tasks", "Normal")
    }

    static async fn load() -> TaskWidgetEntry {
        try {
            await TaskWidgetData.database.migrate(TaskWidgetData.migrations)
            let tasks: Array<TaskRow> = await TaskWidgetData.database.query<TaskRow>(
                "SELECT title, priority FROM tasks WHERE is_completed = ? ORDER BY priority DESC, id LIMIT 1",
                [false]
            )
            if tasks.isEmpty {
                return TaskWidgetEntry("No open tasks", "")
            }
            let nextTask = tasks.first() ?? TaskRow("No open tasks", "")
            return TaskWidgetEntry(nextTask.title, nextTask.priority)
        } catch {
            else { return TaskWidgetEntry("Tasks unavailable", "") }
        }
    }
}

widget NextTask(
    displayName: "Next task",
    description: "See the highest-priority open task.",
    placeholder: TaskWidgetData.placeholder(),
    entry: await TaskWidgetData.load(),
    families: [Small, Medium],
    refreshSeconds: 1800
) {
    Column(spacing: 8, padding: 12, background: "#F1F5F9", cornerRadius: 12) {
        Row(spacing: 6) {
            Icon(system: "check_circle", description: "Tasks", size: 18, tint: "#2563EB")
            Text("Next task", fontSize: 16, fontWeight: Semibold)
        }
        Text(entry.taskTitle, fontSize: 14, lineLimit: 2)
        if entry.priority != "" {
            Text("Priority: \(entry.priority)", fontSize: 12, color: "#475569")
        }
    }
}

app TaskBoard {
    let database = SQLite.Database("tasks", true)

    body {
        Text("Task updates are shared with the NextTask widget")
    }
}
```

The plugin package must be declared in the app's `nexa.config.nx` dependencies. On iOS, set `ios.appGroupIdentifier` when the app and widget extension share the same SQLite file; the plugin reports an error if shared access is requested without that group.

## Widget parameters

| Parameter | Type | Required | Notes |
|---|---|---:|---|
| `displayName` | `String` | No | Non-empty name shown in the widget gallery when supplied. |
| `description` | `String` | No | Non-empty summary shown in the widget gallery when supplied. |
| `configurationTitle` | `String` | No | Requires `configuration`; describes the user-selectable configuration. |
| `configurationDescription` | `String` | No | Requires `configuration`; explains the available setting. |
| `configuration` | Declared struct value | No | Every field must use a declared enum type; the widget exposes the value as `configuration`. |
| `placeholder` | Entry struct value or provider call | Conditional | Required when `entry` is asynchronous or throwing; it must return the same struct type as `entry`. |
| `entry` | Provider method call | Yes | The method returns a declared struct. Use `await` for an asynchronous provider. |
| `families` | `[Small, Medium, Large, ExtraLarge]` | Yes | One or more unique supported family names. |
| `refreshSeconds` | Positive integer literal | Yes | Requested timeline refresh interval in seconds; Android may apply platform scheduling limits. |

The widget body receives `entry` and `family`. When a configuration is declared, it also receives `configuration`. These names are reserved in the widget body.

## Widget family and body support

| Family | iOS | Android | Common use |
|---|---|---|---|
| `Small` | WidgetKit small | Compact launcher slot | One status or short task summary |
| `Medium` | WidgetKit medium | Wider launcher slot | A summary with a few rows |
| `Large` | WidgetKit large | Expanded launcher slot | Several rows of detail |
| `ExtraLarge` | iPad WidgetKit | Large tablet slot where supported | Expanded dashboard |

Widget bodies support `Text`, `Icon`, `Spacer`, `Divider`, `Column`, `Row`, `Stack`, `FastList`, conditional `if` content, and static `Link` actions, with a restricted set of each component's normal options. `Image`, input controls, sheets, dialogs, gesture actions, and state mutation are unsupported in the separate widget process.

For repeated content, bound the source with `take(count)` before passing it to `FastList`. Avoid unbounded or interactive lists; widget timelines should be small snapshots.

## Shared SQLite storage

For a database shared between an iOS app and its WidgetKit extension, set the app-group identifier in `nexa.config.nx`:

```nx
config {
    ios { appGroupIdentifier: "group.dev.example.tasks" }
}
```

Open the database from both app and widget code with the same name and `sharedWithWidgets` enabled:

```nx
plugin "plugins/sqlite" as SQLite

app SharedTaskStore {
    let database = SQLite.Database("tasks", true)

    body {
        Text("Shared task database opened")
    }
}
```

On iOS the plugin uses the App Group container. On Android, app widgets run under the app identity and use the app-private database path. SQLite queries in a widget provider return a snapshot for a timeline entry; app screens can use `observeQuery<T>(...)` for live reactive UI.
