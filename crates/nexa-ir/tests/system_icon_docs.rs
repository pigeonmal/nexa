//! The published system icon table is a view of the IR catalog.
//!
//! `docs/system-icons.md` lists every portable icon name and both of its
//! native spellings. Hand-maintaining that list is how `comment_left` went
//! missing from the documentation while remaining fully supported in the
//! compiler, so the table is generated and gated here.

use nexa_ir::system_icons::{SHARED_ICONS, render_shared_icon_table};

#[test]
fn system_icon_catalog_matches_documentation() {
    nexa_testkit::assert_region(
        &nexa_testkit::workspace_root().join("docs/system-icons.md"),
        "icons",
        &render_shared_icon_table(),
        "docs/system-icons.md#icons",
    );
}

/// Guards the generator itself rather than the file.
///
/// A row that lost its alias list or its backticks would still match a
/// regenerated file and read as valid Markdown, so assert the shape directly:
/// one row per definition, and every row naming a catalog entry.
#[test]
fn rendered_table_has_one_row_per_catalog_entry() {
    let rendered = render_shared_icon_table();
    let rows: Vec<&str> = rendered.lines().filter(|line| line.starts_with("| `")).collect();
    assert_eq!(
        rows.len(),
        SHARED_ICONS.len(),
        "every shared icon must produce exactly one table row"
    );
    for (definition, row) in SHARED_ICONS.iter().zip(&rows) {
        assert!(
            row.contains(&format!("`{}`", definition.name)),
            "row for `{}` must be present: {row}",
            definition.name
        );
        assert!(
            row.contains(&format!("`{}`", definition.sf_symbol)),
            "row for `{}` must name its SF Symbol: {row}",
            definition.name
        );
        assert!(
            row.contains(&format!(
                "`{}.{}`",
                definition.material_namespace, definition.material_name
            )),
            "row for `{}` must name its Material icon: {row}",
            definition.name
        );
    }
}
