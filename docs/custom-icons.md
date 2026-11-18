# Custom icons in Nexa

| **Scope**: custom in-app glyphs and image icons | **Targets**: iOS and Android | **Config**: `nexa.config.nx` |

Nexa supports custom in-app icons through glyphs in TTF or OTF fonts. You register the font once, assign readable aliases to its Unicode codepoints, and use those aliases in `.nx`; the compiler emits native SwiftUI and Compose glyph rendering.

## Quick start

Add the icon font to `assets/fonts/`. This example assumes that `TrailIcons.ttf` contains the glyphs at decimal Unicode codepoints `61696` and `61697`.

```text
TrailPlanner/
├── App.nx
├── nexa.config.nx
└── assets/
    └── fonts/
        └── TrailIcons.ttf
```

Register the font and give the codepoints names in `nexa.config.nx`:

```nexa
config {
    assets {
        iconFonts { TrailIcons: "assets/fonts/TrailIcons.ttf" },
        icons {
            trail_map: ["TrailIcons", 61696],
            trail_bookmark: ["TrailIcons", 61697],
        }
    }
    android { minSdk: 24 }
}
```

Use aliases anywhere an icon name is accepted:

```nx
app TrailPlanner {
    state savedTrips: Int32 = 3

    body {
        Column(gap: 12, padding: 16) {
            Row(gap: 10, alignment: Center) {
                Icon(system: "trail_map", description: "Trail map", size: 28, tint: "#176B52")
                Column(gap: 3) {
                    Text("Weekend routes", fontSize: 20, fontWeight: Semibold)
                    Text("Trips saved: \(savedTrips)", color: "#52605B")
                }
            }
            Button("Save this route", icon: "trail_bookmark") {
                savedTrips = savedTrips + 1
            }
        }
    }
}
```

Run `nexa check` to validate the project, then `nexa dev` to launch it.

## Choose the right kind of icon

| Need | Use | Example |
|---|---|---|
| A common symbol with matching native behavior on both platforms | Nexa's shared icon catalog | `Icon(system: "search", ...)` |
| Your own monochrome, scalable glyph | A registered font glyph alias | `Icon(system: "trail_map", ...)` |
| A multicolor logo or detailed illustration | An image asset | `Image(asset: "trail_logo", ...)` |
| The image shown on the device home screen | The app icon configuration | `assets.icon` in `nexa.config.nx` |

The shared system icon catalog is documented in [System Icons](system-icons.md). Image assets are documented in the [component guide](components.md#image). The app launcher icon is separate from icons drawn inside a screen.

### Image-backed marks

For a multicolor logo or detailed illustration, put a PNG, JPEG, or WebP directly in `assets/images/` and pass its filename stem to `Image(asset:)`. Image names use lowercase letters, digits, and underscores; `Image` does not apply an icon-font tint.

```nx
app TrailBrand {
    body {
        Image(asset: "trail_logo", description: "Trail Planner", maxHeight: 48)
    }
}
```

## How font aliases work

The font contains outlines for glyphs and a character map that associates each glyph with a Unicode codepoint. Nexa uses the codepoint to select the glyph; the alias such as `trail_map` is a readable compile-time name for that mapping.

At project compilation, Nexa checks that the configured font exists, is a TTF or OTF file directly inside `assets/fonts/`, and that every alias refers to a declared font and a valid Unicode scalar. It also rejects duplicate aliases and aliases that collide with a shared Nexa icon. Nexa cannot tell whether the font's character map actually contains the selected codepoint, so confirm that mapping in the icon font's documentation or font editor.

The filename stem does not have to match the family declared in `iconFonts`. Nexa reads the font metadata at project generation and maps the configured family to the iOS PostScript name and Android resource ID. When using an automatically discovered font without an `iconFonts` mapping, use its embedded family name in `.nx`.

### Finding codepoints

Use the icon font's codepoint chart or inspect its character map with a font editor. Fonts often place custom icon glyphs in the Unicode Private Use Area; for example, `U+F100` is decimal `61696`.

`icons` expects decimal integer values, not strings such as `"U+F100"` and not hexadecimal notation. Convert the codepoint to decimal before adding it to the config. Keep the original icon font's license and codepoint chart with your project so the mapping can be maintained.

## Using custom icons in components

An alias resolves to the same typed font glyph wherever a built-in icon-name parameter is supported:

- `Icon(system: "trail_map", ...)` draws the icon by itself and lets you set its size, tint, and accessible description.
- `Button("Save route", icon: "trail_bookmark") { ... }` adds it to a native button.
- `ContentUnavailable(title: ..., icon: "trail_map")` uses it in an empty state.
- `Picker(icon: "trail_map", ...)` and `Picker(itemIcons: [...])` use it on a picker or its menu rows.
- `Tab(..., icon: "trail_map")` uses it in an app bottom bar tab.

Check the [generated syntax audit](syntax-audit.md) for each component's complete, current argument requirements. Icon names in these parameters must be string literals so Nexa can resolve them during compilation.

### Icon in a row and action

```nx
app TrailDetails {
    state routeSaved: Bool = false

    body {
        Column(gap: 14, padding: 16) {
            Row(gap: 8, alignment: Center) {
                Icon(system: "trail_map", description: "Map", size: 22, tint: "#176B52")
                Text("Pine Ridge Loop", fontSize: 20, fontWeight: Semibold)
            }
            Text("6.4 km · 2 hours · Moderate", color: "#52605B")
            Button("Save route", icon: "trail_bookmark") {
                routeSaved = true
            }
            if routeSaved {
                Text("Saved to your weekend list", color: "#176B52")
            }
        }
    }
}
```

### Picker icons

`itemIcons` entries are matched to picker items by index. Keep their order aligned with the `items` array; an item without a corresponding icon is shown without one. The native menu picker requires Android `minSdk: 24` or later.

```nx
app TrailFilters {
    state selectedFilter: String = "Nearby"

    body {
        Picker(
            items: ["Nearby", "Saved"],
            selected: selectedFilter,
            itemIcons: ["trail_map", "trail_bookmark"]
        )
    }
}
```

## Direct glyphs for advanced cases

Use `Icon(fontFamily:, glyph:, ...)` when the glyph codepoint is already held as a typed `Int32` value. This form avoids creating an alias for a value supplied dynamically, while still requiring a static font family.

```nx
app DynamicTrailMarker {
    state markerGlyph: Int32 = 61696

    body {
        Icon(
            fontFamily: "TrailIcons",
            glyph: markerGlyph,
            description: "Selected trail marker",
            size: 28,
            tint: "#176B52"
        )
    }
}
```

For fixed icons, prefer named aliases: they make `.nx` source readable and keep font codepoints in one place. The complete `Icon` signature and accepted values are in the [syntax audit](syntax-audit.md#icon).

## Native output and runtime cost

Nexa includes project fonts in the generated app resources for both targets. It lowers each alias to a typed font-family and codepoint pair; generated code does not look up icon names in a dynamic dictionary.

- **iOS:** the font is registered with the app and SwiftUI draws the Unicode character using the custom font at the requested point size.
- **Android:** the font is packaged as an Android font resource and Compose draws the same Unicode character with a native text primitive.

The same font file and codepoint mapping must work on both platforms. Use a font that includes all mapped glyphs and test its visual weight, alignment, and legibility at the sizes used in your UI.

Custom font glyphs are not supported in Android widgets. Use a shared Nexa icon in a widget.

For a glyph supplied dynamically through `Icon(glyph:)`, keep the value within the Unicode scalar range and make sure the font contains it. Fixed aliases are safer because the mapping is visible and reviewable in the project config.

## Troubleshooting

- **`unknown icon` during `nexa check`:** confirm the alias spelling in `assets.icons` and that the config is in the project root or a parent directory of the `.nx` entry file.
- **Font path or family error:** keep the file directly under `assets/fonts/`, use `.ttf` or `.otf`, and confirm the `iconFonts` family points to the correct file.
- **A square, blank space, or wrong symbol appears:** verify the codepoint against the font's character map and confirm the configured family is registered in `iconFonts`.
- **It works on one target only:** make sure both builds use the same font file and codepoint, then rebuild so the native resource bundles are regenerated.
- **A button or tab rejects the custom name:** use an alias declared under `assets.icons`; `Icon(sfsymbol:)` and `Icon(materialsymbol:)` are for platform-specific symbols, not registered font aliases.

## Related guides

- [System Icons](system-icons.md) — shared semantic names and platform mappings.
- [Component Guide](components.md) — `Icon`, `Image`, buttons, pickers, and tabs.
- [Getting Started](getting-started.md#4-configuration-nexaconfignx) — project configuration and CLI setup.
- [Syntax Audit](syntax-audit.md#icon) — exact accepted syntax for `Icon`.
