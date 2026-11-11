**Native list audit · 2026-11-09 · UIKit · AOT and DevRuntime**

This pass removes full-section header discovery from iOS sectioned-list refreshes. It changes the native primitive shared by generated AOT apps and DevRuntime.

```bash
python3 tests/native_section_headers_check.py --require-device
CARGO_TARGET_DIR=/tmp/nexa-verification-target cargo test -p nexa-cli --test native_ui_hotpaths
```

## Root cause and implementation

Every refresh previously called `headerView(forSection:)` for every section, including sections far outside the viewport. Large libraries therefore paid for thousands of UIKit lookups when refreshing only a few displayed headers.

- Track loaded headers through UIKit's display and end-display delegate callbacks.
- Keep entries ordered by section, with weak view references. Refresh registered headers without creating a temporary collection or sorting on each update.
- Check view identity on removal so a delayed callback for a replaced header cannot remove its replacement.
- Clear the registry before structural reloads; UIKit callbacks repopulate it.
- Reuse already normalized section counts during updates instead of allocating a second normalization array.
- Emit the registry only for apps using sectioned lists. Cache version 505 invalidates previously generated native units.

This optimizes header discovery and refresh. Section-count construction, initial normalization, and structural equality checks can still scale with the number of sections; the complete list update is not claimed to be constant time.

## Native regression evidence

The optimized Swift fixture runs against real UIKit tables in the booted simulator. It covers plain and grouped tables, empty sections, missing headers, pinned headers, collapsed header views, content insets, scrolling, reuse, structural reload, replacement identity, ordering, and weak ownership.

Header IDs match the previous full-scan reference at the tested offsets. Reused headers receive current content, and historical headers do not accumulate in the registry. Tests create no screenshots or image files.

For a 20,000-section table with seven registered headers, seven optimized-build samples measured ten refreshes each:

- Reference median: **14.849 ms** per ten refreshes, scanning 20,000 section IDs per refresh.
- Registry median: **0.0148 ms** per ten refreshes, updating seven headers with no discovery lookups.

These samples measure header discovery plus a trivial native label update. They do not measure the complete SwiftUI hosting update, app frame time, battery consumption, or physical-device performance. Raw samples: [section headers](native-lists-2026-11-09/section-headers.json).

## Visible-index scans during scrolling

Table, horizontal-list, grid, and masonry callbacks previously projected visible `IndexPath` values into temporary arrays before selecting an extremum. Sticky-header filtering also materialized a separate array.

Use Swift's lazy projections for these reductions, including the deferred viewport-restoration callback. UIKit still owns and creates its visible-path snapshots; this change removes Nexa's intermediate projections, not UIKit's own allocations. Geometry reads, callback ordering, and selected indices remain unchanged.

The native harness executes the coordinator method extracted from the backend source against deterministic UIKit inputs. **1,440 equivalence cases** cover empty and unordered paths, reverse layout, pinned-header occlusion, occlusion thresholds, content offsets, and insets. It also verifies that lazy iteration does not repeat row-geometry queries.

For forty visible paths and 10,000 scans per sample, nine optimized-build simulator samples measured:

- Without a pinned header: median **2.044 ms → 1.916 ms**, approximately 6% less CPU time.
- With a pinned header: median **30.499 ms → 28.501 ms**, approximately 7% less CPU time.

These are isolated scan timings with controlled UIKit input accessors. They exclude UIKit snapshot creation and complete scroll-frame work. No measured heap-allocation count or whole-app frame-rate gain is claimed. [Native samples and equivalence results](native-lists-2026-11-09/visible-index-scans.json).

The same native primitive serves AOT and DevRuntime. Android already uses a typed scan without projected arrays, so its implementation is unchanged. Cache version 506 refreshes generated native output.

## Build and workspace verification

- Workspace check and Clippy passed; the complete workspace suite passed **1,610 tests**.
- The temporary-directory collision gate passed.
- The performance audit app includes a bounded sectioned list alongside NumberFlow, TextReveal, Marquee, and a large plain list.
- iOS and Android Release builds completed with no audit warnings. [Release build report](native-lists-2026-11-09/release-builds.json).

Whole-app startup, retained-memory trends, battery, and 120 Hz hardware acceptance remain separate open measurements. Earlier native rendering and allocation results are recorded in the [animation audit](native-animation-2026-11-09.md).
