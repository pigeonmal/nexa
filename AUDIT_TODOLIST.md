# ToDoList reference audit against Nexa

Reference: `/tmp/nexa-todolist-reference` (the checked-out RanduSoft SwiftUI project supplied for this request). `/tmp/ToDoList` was confirmed to contain the same 78 app/project files. This audit describes the reference files and compares product capabilities with the Nexa framework present in this checkout. `Completed` means the relevant behavior has a supported generic Nexa path; `Partial` means some behavior exists but material requirements remain; `Absent` means no reusable framework path was found; `Out of scope` means the example deliberately does not implement the feature.

The file inventory below lists all 78 app and project files present in the reference checkout. Git internals under `.git/` and the local `accessibility-ref.xcresult` diagnostic bundle are excluded because they do not implement or configure the application.

## Feature coverage

| Feature | Status | Reference behavior and Nexa evidence / remaining gap |
|---|---|---|
| Native SwiftUI and Jetpack Compose output | Completed | Swift and Kotlin backends emit native view trees. The generated host is not a complete match for this app by itself. |
| Shared native system icons | Completed | `Icon(system:)`, `Button(icon:)`, `Picker(icon:)`, and tab icons resolve through the shared `SystemIcon` catalog in `crates/nexa-ir/src/system_icons.rs`; the Swift backend emits SF Symbols and the Kotlin backend emits imported Material vectors. DevRuntime icon dispatch is generated from that same catalog, including recognized SF aliases and app-used Material Symbols. `Icon(sfsymbol:)` and `Icon(materialsymbol:)` remain platform-specific escape forms. |
| Native bottom navigation and tab content | Completed | `AppBottomBar` lowers to native SwiftUI `TabView` and Material 3 `NavigationSuiteScaffold` (`crates/nexa-backend-swift/src/generator/components/bottom_bar.rs`, `crates/nexa-backend-kotlin/src/generator/components/bottom_bar.rs`). Both targets support selected state, per-tab content, localized navigation titles, large-title display, labels, shared icons, and badges. iOS 18+ uses SwiftUI's sidebar-adaptable tabs and a search-role destination (with native search-tab activation on iOS 26+); earlier iOS versions use native `.tabItem` fallback. Android adapts between bottom navigation and rail/drawer layouts, with per-tab saveable state. DevRuntime uses the same adaptive Android navigation and tab-state behavior. |
| Adaptive tablet, foldable, and landscape navigation | Completed | `NavigationSplitView(detailVisible:) { Sidebar { ... } Detail { ... } }` provides typed target-neutral master-detail panes. Swift emits native SwiftUI `NavigationSplitView` on iOS 16+ with compact-column state on iOS 17+, and an older-iOS fallback. Kotlin emits a two-pane wide-window layout and compact single-pane layout with Back handling. Both layouts respond to the available window size, including landscape and unfolded foldable widths. |
| Onboarding and notification permission flow | Completed | The Todo archetype and `examples/onboarding.nx` use the dedicated `PagePager` primitive for the welcome, notifications, and completion pages. The Todo root conditionally renders onboarding on first launch and when replayed from Settings. Completion persists through built-in `Storage`, and the notification action requests permission before advancing. Each page reserves the reference's 48-point bottom area. Copy is localized through the source catalog, with Romanian translations in the Todo catalog. `crates/nexa-compiler/tests/core_onboarding.rs` covers Swift and Kotlin lowering. |
| Task create/edit/delete/complete | Completed | `examples/archetypes/todo/App.nx` uses a declared `TaskItem`, typed `Array<TaskItem>` state, native text input and buttons, and collection operations to create, edit, delete, and toggle completion without task-specific framework code. Its Swift output is checked by `crates/nexa-compiler/tests/core_task_crud.rs`. Persistence is covered separately; apps choose `Storage`, `@nexa/mmkv`, or `@nexa/sqlite` and own their persistence schema. |
| Task descriptions and metadata | Completed | `examples/archetypes/todo/App.nx` models a description, optional due timestamp, date/time distinction, priority, and completion in `TaskItem`; its add/edit UI uses multiline `TextInput`, `DatePicker`, and `Picker`, and list rows show the metadata. The iOS compiler test checks the generated native controls. Apps own persistence schemas and relationships; comment and reminder behavior is tracked in their dedicated audit rows. |
| Due date and time selection | Completed | `DatePicker(timestamp:, hasTime:)` lowers to a graphical SwiftUI calendar and native Compose date/time controls. The Todo date-picker sheet is large-only and its Done action commits draft date/time values; Cancel dismisses without changing the task. |
| Date-only versus date-and-time tasks | Completed | `DatePicker` binds the timestamp and `hasTime` flag, with native controls on both platforms. Persistence semantics remain app-owned. |
| Priority values and sorting | Completed | The Todo archetype stores an app-authored priority value and rank, exposes priority choices, and generates ascending/descending priority ordering through typed `sortedBy` transforms; `core_task_crud.rs` verifies the optimized native Swift comparator. Native drag reordering and its durable policy are tracked separately under Sorting by date/priority/manual drag. |
| Task comments with text and photos | Completed | The Todo owns a typed `CommentItem` with a `taskID` relation and a durable private image-file URI. `@nexa/media-picker` copies selected images into app-private storage; SQLite stores only the URI and comment metadata. Text comments, editing, deletion, image selection, previews, and full-screen image presentation are authored using generic Nexa state, persistence, `Pressable`, `Dialog` text input, `Image`, `BottomSheet`, and `FastList`. `core_task_crud.rs` verifies generated Swift and Kotlin include the related comment model, media-picker, full-screen presentation, and native text-entry alert. The reference stores compressed image bytes in SwiftData; Nexa's private-file reference is the corresponding durable design for this storage stack. This closes feature support, not pixel-level styling parity. |
| Reminders and local notifications | Completed | The Todo owns typed, persisted `ReminderItem` records linked by `taskID`, and uses `@nexa/notifications` for native relative/absolute scheduling, pending checks, cancellation, and queued `localNotificationOpened` events on iOS and Android. App `OnActive` reconciliation checks the persisted task relationship, re-schedules only missing future reminders, and cancels notifications whose reminder/task is no longer active; generated Swift scene-phase and Compose `ON_RESUME` paths are asserted in `core_task_crud.rs`. Exact add/list styling remains in the visual parity audit. |
| App icon badge counts | Completed | The Todo recomputes the count of incomplete tasks due today after task writes and when the app becomes active, then calls `Notifications.setBadgeCount`. iOS uses `UNUserNotificationCenter.setBadgeCount` on iOS 16+ and the UIKit fallback on earlier supported versions. Android stores the count and attaches it to active and future notifications through `NotificationCompat.setNumber`; launcher display remains system-controlled and requires an active notification. These generated calls are checked for both targets in `core_task_crud.rs`. |
| Automatic reminder configuration | Completed | Todo settings persist the selected None/10/30/60-minute interval through built-in `Storage`; new tasks with a future due date and time create a typed task-linked reminder and schedule it at the selected offset. The default is 10 minutes, matching the reference. `core_task_crud.rs` checks generated Swift and Kotlin contain the setting, reminder identity, and interval calculation. |
| Sorting by date/priority/manual drag | Completed | Date and priority ordering remain app-authored. Inbox, Today, and Completed now wire native drag callbacks in Manual mode and persist the reordered task array. `Array.moveSubset(from, to, orderedSubset)` maps indexes from a stable-order filtered projection back into its backing array, keeping hidden entries in place; the operation is implemented in generated Swift/Kotlin, headless actions, and iOS/Android DevRuntime. Upcoming stays date-grouped and Search has no move action, matching the reference. |
| Search over tasks | Completed | The search tab filters non-deleted tasks by title or description, includes completed tasks like the reference, and uses case-insensitive string containment on both generated platforms. It has empty-query and no-results states, editable results, and the native search-tab role with navigation-level `.searchable` binding and prompt. Search-tab activation and search field behavior are native on supported iOS versions; Android renders its platform search field alongside the destination title. |
| Native empty states | Completed | Task lists, search, reminders, and comments use the reusable `ContentUnavailable` component. Swift emits iOS 17 `ContentUnavailableView` with an iOS 16 SwiftUI fallback; Compose uses a Material 3 empty-state layout, and both DevRuntime renderers recognize the same node. |
| Inbox/Today/Upcoming/Completed filters | Completed | The Todo source applies the reference predicates directly: non-deleted incomplete Inbox tasks, due-today tasks, non-deleted incomplete tasks due today or later, and completed tasks. Search is handled as a separate all-task filter. |
| Upcoming tasks grouped by day | Completed | Upcoming tasks are sorted first, grouped by local `Time.startOfDay` through the generic `groupedBy` transform, and rendered through native sectioned `FastList` with a localized relative date header. |
| Seven selectable color themes | Completed | Todo exposes and persists all seven reference accent colors, and applies the selected tint to native tab navigation, onboarding, and primary actions. Alternate app icons are audited separately. |
| Light and dark system appearance | Completed | Generated Android hosts follow system night mode, and generic `Appearance(mode:)` wraps app content with a runtime system/light/dark selection using native SwiftUI preferred color scheme and Compose Material 3 color schemes. |
| iOS 26 Liquid Glass across controls | Completed | The reference uses interactive tinted glass for its circular add-task buttons. Nexa's `glass: true` button path emits native `.glassEffect(.clear.tint(...).interactive(), in: .circle)` on iOS 26+, with a native material fallback on earlier iOS. The Todo applies it to Inbox, Today, and Upcoming add buttons; DevRuntime renders the same effect where iOS 26 is available. |
| Localization (English and Romanian) | Completed | Nexa extracts source text from UI literals and `Locale.localized("source text")`, preserves typed interpolation placeholders, and generates Apple string catalogs and Android `R.string`/`R.plurals` resources. The Todo catalog supplies Romanian for all extracted UI and accessibility copy; English remains the source-language fallback. |
| Settings (start page, theme, reminders, feedback) | Completed | The Todo Settings screen uses native `Form` sections, localized labeled pickers, a theme navigation screen with all seven colors, the open-links toggle, automatic reminder choices, feedback availability, onboarding replay, terms/privacy/source links, and version footer. Start page, theme, appearance, browser preference, and reminder interval persist through app storage. In-app links use `SFSafariViewController` on iOS and Custom Tabs on Android; external links go to the system browser. Feedback presents the iOS mail composer or Android mail chooser. `core_settings_form.rs` verifies generated iOS and Android controls; `nexa check --ios` and `nexa check --android` pass for the Todo app. The appearance controls extend the reference because this app also requires user-selectable light and dark themes. |
| Soft-delete task, comment, and reminder records | Completed | Todo retains deleted records with `isDeleted` tombstones, hides them through active-record filters, and persists each deletion. Deleting a task also tombstones its reminders atomically in SQLite and cancels their scheduled notifications. |
| Context menus (edit/complete/delete) | Completed | `Pressable.contextMenu` accepts typed `Button` actions and lowers to SwiftUI `.contextMenu` on iOS and a long-press Material `DropdownMenu` on Android. Menu labels and actions remain app-authored. |
| Today and Upcoming widgets | Completed | Widgets are authored in `.nx` and generate a WidgetKit extension on iOS and a Jetpack Glance receiver/configuration Activity on Android. The Todo widget provides Today/Upcoming filters, family-specific row counts, priority colors, localized dates, empty states, a shared SQLite database in the configured App Group on iOS, and the package-private database on Android. It ensures migrations have run before loading task data. Successful writes through a widget-shared SQLite handle now request a coalesced WidgetKit/Glance refresh; app backgrounding and Android's periodic update remain additional refresh paths, and each OS controls presentation timing. `docs/widgets.md` documents the portable API and unsupported widget-only constructs are rejected by the compiler. |
| Natural-language data extraction | Completed | A typed `@nexa/data-extractor` package replaces the hand-written date grammar with `NSDataDetector` on iOS and Android `TextClassifier` (API 28+). It returns date/date-time, phone, URL, email, and address matches with shared UTF-16 ranges; Todo uses asynchronous debounced extraction for due-date suggestions. Android provides date precision but no normalized instant; iOS provides an instant but not date/time precision, so Todo opens its native picker when either value is unavailable. The plugin contracts and native sources type-check on both platforms, and `nexa check` passes for the generated iOS and Android Todo targets. |
| VoiceOver/accessibility | Completed | The Todo source now gives task completion controls state labels/hints, task rows edit actions and metadata values, labeled/value-bearing sort and priority pickers, selected-theme values, descriptive comment image/edit actions, a combined reminder date/note label, and explicit save/photo actions. iOS date controls expose their native Date/Time labels; Android's generated date-time toggle now has the matching `Time` content description. The code generator keeps hints in VoiceOver hints on SwiftUI and appends them to the TalkBack content description on the pinned Compose version. |
| Local task persistence | Completed | Tasks, comments, reminders, theme, and widget data use one long-lived SQLite database and typed queries. iOS app and widgets share it through the configured App Group; Android app and Glance widgets share the package database. Small independent preferences use Nexa `Storage`. |
| App icon themes / alternate icons | Completed | `ios.alternateIcons` packages iOS Icon Composer assets and declares the Xcode alternate app-icon names. Android accepts adaptive icon-set directories containing a legacy `icon.png`, foreground and background PNG/XML layers, and optional monochrome art; it emits density fallbacks, Android 8+ adaptive resources, Android 13+ themed resources when supplied, and launcher aliases. `AppIcon.set(name?)` switches the selected icon on both platforms and accepts `null` to restore the configured primary icon. Todo applies it with the theme and logs a failed switch. |
| Feedback email composer | Completed | `@nexa/mail-composer` opens `MFMailComposeViewController` on iOS and a `mailto:` app chooser on Android, with typed unavailable/presentation errors. The Todo message includes app/version and the native device model/OS version on both platforms. Its `completed` event reports sent/saved/cancelled/failed from iOS; Android delegates delivery to the selected email app and cannot report the result. |

## Reference files, one by one

### App entry and navigation

| File | Purpose / feature evidence |
|---|---|
| `App.swift` | `@main` entry, creates the SwiftData container, seeds demo mode, selects onboarding versus main navigation, injects theme/settings, and reloads widgets on backgrounding. |
| `AppDelegate.swift` | Registers the notification delegate, displays foreground notifications, and routes a tapped notification to a task through `NotificationCenter`. |
| `App/Navigation/AdaptiveNavigationView.swift` | Selects adaptive navigation layout for iPhone/iPad. |
| `App/Navigation/ContentTabView.swift` | Owns selected tab and configures Inbox/Today/Upcoming/Completed/Search destinations and tab bar presentation. |
| `App/Onboarding/OnboardingView.swift` | Three-page onboarding pager, completion state, and notification authorization request. |

### Task lists and editing

| File | Purpose / feature evidence |
|---|---|
| `App/Tasks/List/TaskListView.swift` | Queries task data, applies view filter/sort, groups upcoming tasks by date, handles empty state, add/edit sheets, delete, completion, and manual move. |
| `App/Tasks/List/TaskRowView.swift` | Renders task title, description, completion, priority/date metadata, actions, and row context menu. Uses interactive glass styling. |
| `App/Tasks/Detail/TaskEditView.swift` | Add/edit task form, smart date detection, date quick-picks, priority, comments/reminders navigation, save, and automatic reminder scheduling. |
| `App/Tasks/Detail/DatePickerSheet.swift` | Graphical date picker with date-only/date-time selection and commit/cancel actions. |
| `Shared/Views/CheckButtonView.swift` | Reusable completion checkbox/button with native styling and accessibility. |

### Comments and images

| File | Purpose / feature evidence |
|---|---|
| `App/Comments/CommentsView.swift` | Lists and edits comments, adds text comments, selects photos, loads image data, and deletes comments. |
| `App/Comments/CommentRowView.swift` | Displays text/image comment content and row actions. |
| `App/Comments/FullImageView.swift` | Full-screen image viewer and dismissal control. |

### Reminders and search

| File | Purpose / feature evidence |
|---|---|
| `App/Reminders/RemindersView.swift` | Lists task reminders, opens add flow, deletes reminders, and manages notification schedule/cancel behavior. |
| `App/Reminders/AddReminderSheet.swift` | Reminder note/date form with save and cancel actions. |
| `App/Reminders/ReminderRowView.swift` | Displays reminder date/note and row affordances. |
| `App/Search/SearchView.swift` | Queries task records, focuses search entry, filters task text, and presents result/empty states. |

### Settings and shared navigation

| File | Purpose / feature evidence |
|---|---|
| `App/Settings/SettingsView.swift` | Configures starting page and auto-reminder interval, opens theme/onboarding/feedback/link flows, and displays app metadata. |
| `App/Settings/ThemePickerView.swift` | Applies theme colors and selects alternate app icons. |
| `App/Settings/FeedbackMailView.swift` | Wraps `MFMailComposeViewController` and handles send/cancel completion. |
| `Shared/Views/SFSafariView.swift` | Presents an in-app `SFSafariViewController` for web content. |

### Models, services, and data setup

| File | Purpose / feature evidence |
|---|---|
| `Database/DatabaseConfiguration.swift` | Defines the SwiftData schema, shared app container, widget container, preview data, and demo seed fixtures. |
| `Models/TaskModel.swift` | Persistent task fields, completion/deletion state, dates, priority, ordering, comments/reminders relationships, and demo/sample tasks. |
| `Models/CommentModel.swift` | Persistent task comment text/image payload and relationship/creation metadata. |
| `Models/ReminderModel.swift` | Persistent reminder date/note/notification identity and task relationship. |
| `Models/TaskFilter.swift` | Inbox/Today/Upcoming/Completed task filter definitions and display names. |
| `Models/TaskPriority.swift` | Priority levels, labels, colors, and ordering values. |
| `Models/TaskSort.swift` | Sort modes and task-array sorting behavior. |
| `Models/AutoReminderInterval.swift` | None/10-minute/30-minute/1-hour automatic reminder choices. |
| `Models/AppTheme.swift` | Seven color themes and theme display/color values. |
| `Services/AppSettings.swift` | Shared settings state backed by preferences, including first launch, theme, start page, icon, and reminder interval. |
| `Services/NotificationManager.swift` | Requests notification authorization; schedules/cancels reminders; reschedules persisted reminders and updates badge count. |
| `Services/SmartDateParser.swift` | Parses natural-language text into a detected date and whether it includes a time. |
| `Helpers/Extensions.swift` | App-local Swift extensions used to simplify UI/model behavior. |
| `Shared/Extensions/Date+Extensions.swift` | Calendar-day comparisons and localized date/time labels for task rows, sections, comments, and reminders. |

### Widgets

| File | Purpose / feature evidence |
|---|---|
| `Widgets/TaskWidget.swift` | Widget timeline provider, queries shared SwiftData store, filters tasks, maps records into widget entries, and defines widget configuration. |
| `Widgets/TaskWidgetFilter.swift` | AppIntent widget configuration and Today/Upcoming filter options. |
| `Widgets/TaskWidgetViews.swift` | Small, medium, large, and extra-large widget layouts, headers, rows, empty state, and previews. |
| `Widgets/WidgetsBundle.swift` | Registers the widget extension's widget bundle. |

### Tests

| File | Purpose / feature evidence |
|---|---|
| `Tests/SmartDateParserTests.swift` | Unit tests for natural-language date parsing and detected time behavior. |
| `Tests/ToDoListTests.swift` | App model and task behavior tests. |

### Assets, localization, host configuration, and project files

| File(s) | Purpose |
|---|---|
| `.gitignore` | Excludes local/generated Xcode files from version control. |
| `README.md` | Product feature list, requirements, setup instructions, and contribution information. |
| `Assets.xcassets/Contents.json` | Main app asset catalog manifest. |
| `Assets.xcassets/AccentColor.colorset/Contents.json` | Main app accent color definition. |
| `Assets.xcassets/SecondaryAccentColor.colorset/Contents.json` | Secondary accent color definition. |
| `Icon.icon/icon.json`, `Icon.icon/Assets/checkmark.svg` | Default alternate icon metadata and vector artwork. |
| `IconBlack.icon/icon.json`, `IconBlack.icon/Assets/checkmark.svg` | Black alternate icon metadata and artwork. |
| `IconBlue.icon/icon.json`, `IconBlue.icon/Assets/checkmark.svg` | Blue alternate icon metadata and artwork. |
| `IconGreen.icon/icon.json`, `IconGreen.icon/Assets/checkmark.svg` | Green alternate icon metadata and artwork. |
| `IconOrange.icon/icon.json`, `IconOrange.icon/Assets/checkmark.svg` | Orange alternate icon metadata and artwork. |
| `IconPinkAlt.icon/icon.json`, `IconPinkAlt.icon/Assets/checkmark.svg` | Pink alternate icon metadata and artwork. |
| `IconYellow.icon/icon.json`, `IconYellow.icon/Assets/checkmark.svg` | Yellow alternate icon metadata and artwork. |
| `Resources/Icon.jpg` | Raster app icon/design artwork. |
| `Resources/Icon.sketch` | Editable source design file for app icon artwork. |
| `Supportive Files/App.entitlements` | Main app entitlements for the shared app group and notifications. |
| `Supportive Files/Info.plist` | Main app bundle and platform metadata. |
| `Supportive Files/Localizable.xcstrings` | English/Romanian localized string catalog. |
| `ToDoList.xcodeproj/project.pbxproj` | Xcode targets, source/resource membership, build settings, entitlements, and extension configuration. |
| `ToDoList.xcodeproj/project.xcworkspace/contents.xcworkspacedata` | Xcode workspace metadata. |
| `ToDoList.xcodeproj/project.xcworkspace/xcshareddata/IDEWorkspaceChecks.plist` | Xcode shared workspace checks metadata. |
| `ToDoList.xcodeproj/xcshareddata/xcschemes/ToDoList.xcscheme` | Shared main app build/run/test scheme. |
| `ToDoList.xcodeproj/xcshareddata/xcschemes/Widgets.xcscheme` | Shared widget extension scheme. |
| `ToDoList.xcodeproj/project.xcworkspace/xcuserdata/radu.xcuserdatad/IDEFindNavigatorScopes.plist` | User-specific Xcode search scope state; not app behavior. |
| `ToDoList.xcodeproj/xcuserdata/radu.xcuserdatad/xcschemes/xcschememanagement.plist` | User-specific Xcode scheme management state; not app behavior. |
| `Widgets/Assets.xcassets/Contents.json` | Widget asset catalog manifest. |
| `Widgets/Assets.xcassets/AccentColor.colorset/Contents.json` | Widget accent color asset. |
| `Widgets/Assets.xcassets/AppIcon.appiconset/Contents.json` | Widget extension icon asset manifest. |
| `Widgets/Assets.xcassets/WidgetBackground.colorset/Contents.json` | Widget background color asset. |
| `Widgets/Info.plist` | Widget extension metadata. |
| `Widgets/Widgets.entitlements` | Widget app-group entitlement for shared task data. |

## Implementation choices for the Todo example

- Structured tasks, comments, reminders, theme selection, and widget projections live in SQLite. The app and its iOS widget share the configured App Group database; Android widgets use the app's package database directly. No task snapshot cache or MMKV task store remains.
- `TodoDatabase.nx`, `TodoPersistence.nx`, and `CommentPersistence.nx` own long-lived database handles and typed queries outside UI components. Task removal and its reminder tombstones use one SQLite transaction; initial automatic task reminders are inserted with their task in one transaction.
- The task-row metadata is a shared `.nx` component. Manual-order insertion does not sort; task creation currently allocates one filtered projection to find the first sort order.
- Widget providers query only the bounded task fields they render. Successful writes and applied migrations through a widget-shared SQLite handle request a coalesced refresh on iOS and Android, and host backgrounding remains a fallback. SQLite also exposes per-consumer `InvalidationSubscription` events after committed writes and migrations; app code reruns typed snapshot queries and disposes the subscription with the screen lifecycle. Invalidation is coalesced and conservatively applies to every table for the same database name, so unrelated active queries may rerun. The Todo UI still updates its Nexa state explicitly after persistence actions; it has not been migrated to subscriptions. OS scheduling controls when refreshed widget content appears.
- Nexa literals localize automatically in UI. `Locale.localized("source text")` exposes the same generated translation resources when native APIs need a string, including notification copy and the feedback message.
- Android uses native Compose and Glance behavior that adapts to the platform's navigation conventions and window size; the SwiftUI reference remains the iOS presentation target.

## Current Todo archetype recreation status (2026-10-05)

The source now covers the reference's onboarding, five tabs, task CRUD and metadata, completion, search, grouped upcoming tasks, completed-task navigation, comments/photos, reminders, automatic reminders, settings, theme colors, and local widgets. Both native backends have implementations for the app APIs used here. The audit distinguishes supported behavior from exact visual parity; no screenshot comparison is permitted for this task.

| Reference surface | Recreation | Remaining parity notes |
|---|---|---|
| Three onboarding pages | Native page pager with welcome, notification permission, and completion pages. The root destination renders onboarding at first launch and when replayed from Settings; completion persists. Bottom spacing reserves the reference's 48-point area on each page. | Page styling has not been compared with screenshots. |
| Inbox, Today, Upcoming, Settings, Search | Five native navigation destinations with localized native titles and large-title mode, plus a native search-role tab. Completed tasks open as a separate navigation destination. | Search focus timing and toolbar placement can vary by OS version. Visual alignment has not been compared with screenshots. |
| Task rows and actions | Reusable task metadata component; complete/incomplete, edit, context menu, swipe delete, priority/date metadata, comment/reminder counts, and manual drag order on Inbox/Today/Completed. Search has row actions and delete. Checkbox controls use a 34-point maximum width, 28-point icons, a spring animation, and success haptics. | Exact row insets and editor spacing are not visually verified. |
| Upcoming and search | Upcoming rows are grouped by local day and sorted; search is case-insensitive and includes completed tasks. | Search keyboard focus timing and system search activation can vary by OS version. |
| Add/edit task and date selection | The editor uses a native titled partial sheet, navigation toolbar actions, capsule-shaped quick-date controls, title autofocus for new tasks, and a separate large-only date picker sheet whose draft values commit on Done. Priority opens a native confirmation dialog from the toolbar. The title and description inputs use the reference title3/body styles and 1–5/1–3 line bounds. | Other row insets and editor spacing are not visually verified. Android uses native Compose controls and Material sheet presentation. |
| Comments and photos | Typed SQLite records, durable app-private image URIs, photo picking, edit/delete, full-size image view, localized dates, and refreshed row counts after returning. | The reference stores compressed image bytes in SwiftData; Nexa stores the copied file URI, which keeps large image data out of SQLite. Exact composer/list spacing is unverified. |
| Reminders | Typed SQLite records; sorted list; separate titled add sheet, future-date validation, local notification scheduling, cancellation, and foreground reconciliation; title/body source strings are localized. | The Nexa list and add sheet use native toolbar/form primitives; exact spacing remains unverified. |
| Settings and themes | Start page, open-in-app links, automatic reminder interval, system/light/dark appearance, seven accent colors, onboarding replay, terms/privacy/source links, feedback, version footer, and matching iOS/Android alternate app-icon switching. | Theme selection logs an icon-switch failure but does not show an in-app error. |
| Feedback email | iOS native composer and Android mail-app chooser; app/version and device/OS information are prefilled through the shared mail-composer API. | Android cannot report the external mail app's sent/cancel result. |
| Local persistence | Standalone persistence classes share a long-lived SQLite database; widget providers query that database directly. Theme is in SQLite for widget sharing; small independent preferences use Nexa `Storage`. | The reference's debug-only sample/demo data is intentionally omitted. Storage failures are logged and use safe empty/default states rather than an in-app recoverable error surface. |
| Today/Upcoming widgets | `.nx` definitions query SQLite directly, generating WidgetKit and Glance providers, configurable Today/Upcoming filters, family-specific bounded task projections, theme tint, placeholders, empty states, localized date formats, and lifecycle refresh calls. | Widget placement, operating-system update scheduling, and rendered spacing cannot be asserted from source alone. |
| Localization | Nexa extracts English source copy from the app's `.nx` source, including native notification, feedback, and accessibility strings. The Todo catalog contains Romanian translations with interpolation placeholders retained. | Date labels based on native locale formatters intentionally follow the device locale. Pixel/VoiceOver comparison to the reference remains unverified. |

### Known differences and verification boundary

The source-visible parity pass now uses native tab titles, a native confirmation dialog, a title3 task field with reference line bounds, reusable native empty-state presentation, root onboarding, and animated checkbox controls with success feedback. Remaining source differences include search focus timing and platform-native Android layout conventions. Visual spacing and exact pixel parity remain unverified because screenshots are prohibited.

### Final verification (2026-10-07)

- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets`, and `cargo test --workspace` passed. The full workspace test run includes the cache regression, compiler and backend coverage, and doctests.
- `nexa check --ios` and `nexa check --android` passed for the Todo app.
- `nexa test --all --out /tmp/nexa-todo-generated` passed generic iOS and Android native builds. Xcode reported `BUILD SUCCEEDED`; Gradle reported `BUILD SUCCESSFUL`. No simulator or emulator was launched during this verification.
- `cargo test -p nexa-compiler --test core_task_crud` passed for both target lowerings and checks generated success haptics and spring animation.
- No screenshots were captured or inspected. Exact pixel parity therefore remains unverified.

### Final verification (2026-10-05)

- `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets`, and `cargo test --workspace` passed. The full test rerun includes adaptive icon generation, app icon selection, onboarding, settings forms, task CRUD, SQLite mapping, widgets, and both native backends.
- The iOS Todo app built successfully with Xcode and launched in the iOS 26.5 simulator. Simulator logs show the app entering its foreground and opening its shared app-group container, with no app crash or fatal exception. The headless simulator session did not expose an accessibility tree through the available command-line tools, so iOS accessibility content was not independently inspected.
- Android `:app:compileDebugKotlin --rerun-tasks` passed. The Todo app installed and launched in the Android emulator. Accessibility output verified all three onboarding pages, the Today empty state, and the Inbox, Today, Upcoming, Settings, and Search destinations. Android logs contain no app fatal exception; the emulator reports that Play Store services are unavailable, which can affect Google Play-dependent integrations.
- No screenshots were captured or inspected. Exact pixel parity therefore remains unverified.
