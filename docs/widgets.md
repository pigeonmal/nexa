# Universal widgets

Widgets are authored in `.nx`. Nexa's Swift and Kotlin backends generate the
platform hosts and native widget UI; app projects do not contain hand-written
Swift or Kotlin widget implementations.

```nexa
enum TaskFilter {
    today,
    upcoming
}

struct TaskWidgetConfiguration {
    filter: TaskFilter
}

widget Tasks(
    displayName: "Tasks",
    description: "A quick view of your tasks",
    configurationTitle: "Task Filter",
    configurationDescription: "Choose which tasks to display",
    configuration: TaskWidgetConfiguration(TaskFilter.today),
    placeholder: TaskWidgetData.placeholder(configuration),
    entry: await TaskWidgetData.load(configuration),
    families: [Small, Medium, Large, ExtraLarge],
    refreshSeconds: 1800
) {
    Column(spacing: 8, padding: 12) {
        Text(entry.title)
        FastList(entry.tasks) { task, index in
            Text(task.title, lineLimit: 1)
        }
    }
}
```

The entry provider supplies an immutable snapshot to the widget. Keep storage
reads and filtering in that provider, then render the resulting value from
`entry`. The generated iOS timeline/snapshot methods and Android
`provideGlance` evaluate the entry provider when the system requests widget
data; Nexa does not persist a separate task cache. The returned entry is a
bounded render snapshot, while the operating system controls how long it keeps
showing that snapshot and when it requests another one. Async providers run
before Glance composition on Android and through an asynchronous WidgetKit
timeline callback on iOS. They need a synchronous `placeholder` value for the
system's loading UI and as a fallback if loading fails. The placeholder must
have the same struct type as the entry. Async providers are family-independent
across both platforms; use `family` in the widget body to select how many of
the bounded rows to render.

`displayName`, `description`, `configurationTitle`, and
`configurationDescription` are source text. Nexa extracts these literals into
`locales/translations.json` along with visible text in the widget body.
Configuration field and enum labels are derived from their names.

## Native output

- iOS uses WidgetKit timelines and App Intents for configurable widgets.
- Android uses Jetpack Glance, a generated widget receiver, and a native
  configuration Activity for enum options.
- A widget opens the host app when tapped. `Link` can add static external links
  inside the widget.
- `family` maps to Small, Medium, Large, or ExtraLarge in both providers and
  widget content. The app chooses how many rows fit each family.
- Android's launcher controls update scheduling. Nexa clamps periodic Android
  updates to at least 30 minutes and refreshes widgets when the app leaves the
  foreground.

On iOS, an App Group is optional for widgets that only use their timeline
values. Configure `ios.appGroupIdentifier` when a widget and app need to share
storage across their processes; any plugin used by the extension must also
declare its extension-safe implementation and dependency products. The app's
deployment target remains independent of a configured widget's iOS 17 minimum.

## Configuration labels and localization

Configuration field and enum labels are generated from their source names. Widget titles, descriptions, and body text use literal source copy and the same generated translations file as the rest of the app.

## Portable widget body

WidgetKit and Glance do not support every regular app view. Widget bodies
currently accept text, shared system icons, row/column/stack layouts, spacers,
dividers, accessibility labels and values, conditionals without transitions,
static links, and vertical `FastList` content. iOS keeps an accessibility value
separate; Glance includes it in the widget's accessible description. Keep widget
entry data bounded with `Array.take(count)`. The compiler reports unsupported
widget nodes and options instead of silently generating a different layout.
Keep platform-specific differences in Nexa's backends so app authors continue
to write one widget definition.
