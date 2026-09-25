//! Byte-identity harness for the native application code generators.
//!
//! The C++ bridge has `nexa-codegen`'s `bridge_fingerprint`; this is the
//! application-code counterpart. It renders every render path both backends
//! expose — release, development host, and the project-feature-aware Kotlin
//! entry points — for the shipped example apps plus a shape-exercising
//! synthetic app, then prints a stable digest per artifact.
//!
//! Use it to prove that a refactor of the generators (such as routing output
//! through `SourceWriter`) is pure code motion: digests must not change.
//!
//! ```text
//! cargo run -p nexa-cli --example app_fingerprint            # digests
//! NEXA_FP_DUMP=1 cargo run -p nexa-cli --example app_fingerprint   # sources
//! ```

use std::path::{Path, PathBuf};

use nexa_backend_kotlin::KotlinBackend;
use nexa_backend_swift::SwiftBackend;
use nexa_codegen::Backend;
use nexa_compiler::Target;

/// Exercises the render paths the shipped examples leave uncovered: bottom tab
/// bars, bottom sheets, pull-to-refresh, accessibility wrappers, links,
/// direction and keyboard overrides, list modifiers, structs, enums, typed
/// `Result` flow, platform blocks, and custom components with parameters and
/// local state.
///
/// The component vocabulary is exactly `nexa_syntax::catalog::COMPONENTS`, so
/// this fixture is written against the generated syntax audit. Screen and
/// navigation coverage lives in `navigation.nx`: an app that declares screens
/// must have a `body` containing exactly one top-level `NavigationStack`, so
/// the two cannot be combined in a single fixture.
const SYNTHETIC: &str = r##"

struct Todo {
    id: Int32,
    title: String,
    done: Bool
}

component Card(title: String, done: Bool) {
    state expanded: Bool = false

    body {
        Column(spacing: 8, padding: 12, background: "#EEEEEE", cornerRadius: 8) {
            Text(title)
            if done {
                Text("done")
            }
            if expanded {
                Text("expanded")
            }
            Button("Toggle") {
                expanded = !expanded
            }
        }
    }
}

app KitchenSink {
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

    state items: Array<Todo> = [Todo(1, "alpha", false)]
    state groups: Array<Array<Todo>> = [[Todo(1, "alpha", false)], [Todo(2, "beta", true)]]
    state selectedTab: Int32 = 0
    state isRefreshing: Bool = false
    state isPresented: Bool = false
    state cursor: Int32 = 0
    state result: Result<Int32, AppError> = Ok(0)
    state label: String = "idle"

    body {
        OnAppear {
            result = useValue()
        }
        OnActive {
            cursor = cursor + 1
        }
        OnInactive {
            cursor = cursor + 2
        }
        OnBackground {
            cursor = 0
        }
        OnDisappear {
            label = "gone"
        }

        StatusBar(style: Light, hidden: false)

        Direction(value: RTL)

        KeyboardAware(dismiss: Interactive) {
            Column(spacing: 16, padding: 20, background: "#F4F5F7", cornerRadius: 16) {
                Text("Kitchen Sink", color: "#111111", fontSize: 22, fontWeight: Bold, selectable: true)
                Text(result?)
                TextInput(value: label, placeholder: "Name", keyboard: Email, autocorrect: false, capitalization: Sentences, multiline: true, maxLength: 120)
                TextInput(value: label, placeholder: "Password", secure: true)
                Switch(value: isPresented, label: "Sheet")
                Accessibility(label: "status", hint: "current value", role: Button) {
                    Text("Accessible")
                }
                Link(url: "https://nexa.dev") {
                    Text("Docs")
                }
                Pressable(disabled: false, haptic: Medium) {
                    Card(title: "Pressable child", done: true)
                }.onPress {
                    cursor = cursor + 1
                }.onLongPress {
                    cursor = cursor + 10
                }
            }
        }

        RefreshControl(isRefreshing: isRefreshing) {
            FastList(items, key: .id, axis: Vertical, rowHeight: 48) { item, index in
                Text(item.title)
            }
            .onEndReached {
                cursor = cursor + 1
            }
            .onScroll {
                cursor = cursor
            }
            .stickyHeader {
                Text("Sticky")
            }
        }.onRefresh {
            isRefreshing = !isRefreshing
        }

        FastList(count: 200) { index in
            Text(index)
        }

        FastList(sections: groups) { item, index, section in
            Text(item.title)
        }
        .sectionHeader {
            Text("Section")
        }

        AppBottomBar(selected: selectedTab) {
            Tab(index: 0, label: "Home", icon: "house") {
                Text("Home")
            }
            Tab(index: 1, label: "Search", icon: "magnifyingglass") {
                Text("Search")
            }
            Tab(index: 2, label: "Profile", icon: "person", badge: "3") {
                Text("Profile")
            }
        }

        BottomSheet(isPresented: isPresented, partial: true) {
            Column(spacing: 8) {
                Text("Sheet")
                Button("Close") {
                    isPresented = false
                }
            }
        }

        platform ios {
            Image(asset: "logo", description: "logo")
        }
        platform android {
            Image(url: "https://example.com/a.png", description: "remote")
        }
    }
}

"##;

/// Example entry points, relative to the workspace `examples/` directory.
/// Plugin examples are excluded: they need resolved plugin roots on disk, and
/// their generated bridge code is already covered end-to-end (and at runtime)
/// by `tests/native_plugin_build.rs`.
const EXAMPLE_ENTRIES: [&str; 5] = [
    "counter.nx",
    "navigation.nx",
    "showcase.nx",
    "todo_app.nx",
    "virtual_list.nx",
];

fn digest(label: &str, body: &str) {
    if std::env::var_os("NEXA_FP_DUMP").is_some() {
        println!("===== {label} =====\n{body}\n===== end {label} =====");
        return;
    }
    // FNV-1a over the rendered artifact: stable across runs and platforms.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in body.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    println!("{label}\t{hash:016x}\t{}", body.len());
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn render(label: &str, source: &str) {
    for (suffix, target) in [("swift", Target::Swift), ("kotlin", Target::Kotlin)] {
        let module = match nexa_compiler::compile_for_target(source, target) {
            Ok(module) => module,
            Err(error) => return digest(&format!("{label}/{suffix}"), &format!("ERR:{error}")),
        };

        if suffix == "swift" {
            digest(&format!("{label}/swift"), &SwiftBackend.generate(&module));
            digest(
                &format!("{label}/swift-dev"),
                &SwiftBackend.generate_for_dev(&module),
            );
        } else {
            let (release, features) = KotlinBackend.generate_with_project_features(&module);
            digest(&format!("{label}/kotlin"), &release);
            digest(
                &format!("{label}/kotlin-features"),
                &format!("{release}\n{features:?}"),
            );
            let (dev, _) = KotlinBackend.generate_for_dev_with_project_features(&module);
            digest(&format!("{label}/kotlin-dev"), &dev);
        }
    }
}

fn main() {
    let root = workspace_root();
    let examples = root.join("examples");

    for entry in EXAMPLE_ENTRIES {
        let path = examples.join(entry);
        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("skipping {}: {error}", path.display());
                continue;
            }
        };
        render(entry.trim_end_matches(".nx"), &source);
    }

    render("synthetic", SYNTHETIC);
}
