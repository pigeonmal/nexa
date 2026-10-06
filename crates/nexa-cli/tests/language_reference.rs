//! The published scalar type table is read from the backends.
//!
//! `docs/language-guide.md` mapped `.nx` types to Swift and Kotlin spellings by
//! hand, in a table with no compiler behind it. The mappings are not a matter of
//! taste -- `Float32` is `Float` on Swift and `Float` on Kotlin, `Int32` is `Int32`
//! and `Int` -- and they are what the generator emits, so a second hand-written
//! copy of them can only rot.
//!
//! The table is now rendered from `swift_scalar_types` and `kotlin_scalar_types`,
//! which call the same `swift_type` / `kotlin_type` functions code generation
//! uses.

use nexa_cli::docs::render_scalar_types;

fn language_guide() -> std::path::PathBuf {
    nexa_testkit::workspace_root().join("docs/language-guide.md")
}

#[test]
fn scalar_type_table_matches_backend_spellings() {
    nexa_testkit::assert_region(
        &language_guide(),
        "scalar-types",
        &render_scalar_types().expect("render scalar types"),
        "docs/language-guide.md#scalar-types",
    );
}

/// The spellings that are easy to get wrong, and that a reader would notice.
///
/// The region test covers all of them; naming the awkward ones keeps a
/// regression legible instead of a diff in a 14-row table.
#[test]
fn published_spellings_match_the_backends() {
    let rendered = render_scalar_types().expect("render scalar types");
    for expected in [
        "| `Float32` | `Float` | `Float` |",
        "| `Float64` | `Double` | `Double` |",
        "| `Int32` | `Int32` | `Int` |",
        "| `Int64` | `Int64` | `Long` |",
        "| `UInt8` | `UInt8` | `UByte` |",
        "| `Bytes` | `Data` | `ByteArray` |",
        "| `Bool` | `Bool` | `Boolean` |",
        "| `Void` | `Void` | `Unit` |",
    ] {
        assert!(
            rendered.contains(expected),
            "the type table must state `{expected}`"
        );
    }
}

/// Every scalar the backends can name appears in the published table.
///
/// The generator iterates its note list and fails on a missing spelling, so this
/// covers the other direction: a scalar added to a backend with no published row
/// would otherwise be missing silently.
#[test]
fn every_backend_scalar_is_published() {
    let rendered = render_scalar_types().expect("render scalar types");
    for (name, _) in nexa_backend_swift::swift_scalar_types() {
        assert!(
            rendered.contains(&format!("| `{name}` |")),
            "the type table must publish `{name}`, which the Swift backend can emit"
        );
    }
    for (name, _) in nexa_backend_kotlin::kotlin_scalar_types() {
        assert!(
            rendered.contains(&format!("| `{name}` |")),
            "the type table must publish `{name}`, which the Kotlin backend can emit"
        );
    }
}
