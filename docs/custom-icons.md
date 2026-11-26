# Custom Icons in Nexa

| **Scope**: project SVG and font icon sets | **Targets**: SwiftUI and Jetpack Compose | **Since**: named icon set API |

Nexa discovers project icons from their assets, resolves each set and icon during compilation, and emits native text glyphs. SVG files are the default; existing TTF and OTF icon fonts can use a small JSON name map.

## Quick start

Add `assets/icons/Brand/heart.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <path fill="#E11D48" d="M12 21 3 12a5 5 0 0 1 9-5 5 5 0 0 1 9 5z"/>
</svg>
```

Use it in `App.nx`:

```nx
app SavedItems {
    state saved: Bool = false

    body {
        Button("Save item", icon: Icon(Icons.Brand.heart)) {
            saved = !saved
        }
    }
}
```

No `nexa.config.nx` registration is needed. Add files to a set folder and refer to them as `Icons.<Set>.<name>`.

## 1. Sets and names

```text
assets/
└── icons/
    ├── Brand/
    │   ├── heart.svg
    │   └── logo-mark.svg
    └── Commerce/
        └── heart.svg
```

| Part | Rule | Example |
|---|---|---|
| Set folder | A Nexa identifier: ASCII letters, digits, and `_`; starts with a letter or `_` | `Brand`, `commerce_icons` |
| SVG filename | The filename without `.svg` is the icon name; names cannot contain whitespace | `heart.svg` becomes `"heart"` |
| Duplicate names | Allowed in different sets; the set selector disambiguates them | `Icon(Icons.Brand.heart)`, `Icon(Icons.Commerce.heart)` |
| Source per set | A set uses either `assets/icons/<Set>/` or `assets/fonts/<Set>.ttf` / `.otf` with a sidecar | Do not define both `Brand` sources |

Set and icon names are checked during compilation. An unknown set reports the
available sets; an unknown icon reports names in the selected set. A custom
selector never falls back to a similarly named system icon.

## 2. System and custom icons

`Icons.system.<name>` selects Nexa's shared system catalog. A custom folder or font
set is accessed under `Icons.<Set>.<name>`:

```nx
Column(gap: 12) {
    Icon(Icons.system.search)
    Icon(Icons.Brand.heart)
}
```

An icon slot accepts any `Icon` expression (`Icons.system.<name>`, `Icons.<Set>.<name>`, `Icon(...)`, or an `Icon` variable/array element):

```nx
app ShopIcons {
    state selectedTab: Int32 = 0
    state sortOrder: String = "Newest"
    state rowIcons: Array<Icon> = [Icons.system.calendar, Icons.Brand.heart]

    body {
        Button("Save", icon: Icon(Icons.Brand.heart, color: "#E11D48")) { }
        Picker(
            items: ["Newest", "Price"],
            selected: sortOrder,
            icon: Icons.Commerce.heart,
            itemIcons: rowIcons
        )
        ContentUnavailable(
            title: "No saved products",
            icon: Icons.Brand.heart,
            description: "Save a product and it will appear here."
        )
        AppBottomBar(selected: selectedTab) {
            Tab(index: 0, label: "Saved", icon: Icons.Brand.heart) {
                Text("Your saved products")
            }
        }
    }
}
```

| Slot | Accepted value | Notes |
|---|---|---|
| `Icon(...)` standalone | System selector or custom set selector | Renders at its own native/default size unless `size:` is supplied |
| `Button(icon:)` | Shared system name or `Icon(...)` | Omit `description` to use button accessibility context; label-only buttons are unchanged |
| `Tab(icon:)` | Shared system name or `Icon(...)` | Applies to `Tab` declarations inside `AppBottomBar` |
| `Picker(icon:)` | Shared system name or `Icon(...)` | Icon for the picker control |
| `Picker(itemIcons:)` | Array of system names and/or `Icon(...)` values | Entries align by index with `items`; omit an entry to show no row icon |
| `ContentUnavailable(icon:)` | Shared system name or `Icon(...)` | Omit `description` to use the native empty-state accessibility context |

The compiler lowers all of these values to typed IR. AOT output does not look
up project icon names at runtime.

## 3. Optional icon parameters

| Parameter | Type | Default | Behavior |
|---|---|---|---|
| `description` | `String` | Omitted | Standalone icons keep native accessibility behavior; component slots use their parent context. An explicit empty string marks an icon decorative. |
| `size` | positive numeric literal, `1` through `512` | Omitted | Standalone system icons use the platform symbol default. Custom glyphs use the platform body text size. Component slots use their native contextual size. |
| `color` | color expression (`String`) | Omitted | System icons use native content color. SVG glyphs keep their original color layers; supplying `color:` tints every layer with one color. |

The `size` value is in points on iOS and dp on Android. `color` accepts a hex
color or a dynamic `String` expression such as a theme color state.

## 4. SVG support and generated fonts

Nexa builds one deterministic TrueType font per SVG set under
`.nexa/build/icon-fonts/`. It sorts source filenames, assigns stable private-use
codepoints, normalizes SVG geometry, and writes the generated font as a static
app resource. The content-versioned font family lets `nexa dev` replace changed
artwork without colliding with the font already loaded by the host.

| Supported artwork | Examples |
|---|---|
| Paths | `path` and normal path commands |
| Basic shapes | `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon` |
| Composition | Groups, nested groups, transforms, viewBox scaling |
| Paint | Solid fills and strokes, including multiple colors in one SVG |

Paint order follows the SVG tree, so layered fills remain layered in the font.
The generator expands strokes to outlines and stores solid color layers in the
font. A supplied `color:` intentionally turns the whole glyph into a single
tinted icon.

Text, images, filters, masks, patterns, markers, gradients, external paint
references, and unsupported SVG elements or attributes fail compilation with
the source file and offending element or attribute. Convert text to paths and
keep icon files self-contained when importing artwork.

## 5. Existing TTF and OTF icon fonts

Keep the font in `assets/fonts/` and add a sidecar with the same filename stem.
The sidecar maps readable icon names to Unicode codepoints:

```text
assets/fonts/
├── Brand.ttf
└── Brand.icons.json
```

```json
{
  "heart": "U+F100",
  "search": 61697
}
```

Numeric values are decimal codepoints. String values are hexadecimal and may
use `U+` or `0x` prefixes. The compiler validates the JSON, sidecar pairing,
font file, set name, icon name, and that every mapped codepoint exists in the
font. Nexa uses the font's PostScript name for iOS rendering; the set stem
continues to identify the asset and Android resource.

Use the font set exactly like an SVG set: `Icon(Icons.Brand.heart)`. The font is
packaged statically with generated iOS and Android apps; no configuration table
or runtime alias lookup is involved.

## 6. Development builds and native rendering

`nexa dev` watches SVG files, paired font sidecars, and their TTF or OTF files.
An edit regenerates SVG fonts when needed and sends the icon font bytes with the
typed module update. iOS registers these fonts with CoreText; Android loads them
from the app cache into Compose. Icon asset edits do not rebuild the DevRuntime
host, and ordinary module updates keep compatible app state.

`nexa release` and native `nexa test` stage subsets of icon fonts based on the
glyphs used by each target. They omit unused icon fonts and keep full faces
used by text, fonts that disallow subsetting, and fonts whose glyph data cannot
be preserved in a smaller file. DevRuntime keeps the full sets for hot reload.

Release output stays ahead-of-time: SwiftUI renders symbols and font glyphs
with native `Image` and `Text`; Compose renders them with Material `Icon` and
`Text`. A single-layer SVG resolves to one glyph and one color at compile
time. It renders in one native text view, while multi-layer art keeps a
stack of glyphs. Project glyphs are tracked separately from system icons, so a
custom-only Android app does not acquire Nexa's Material system-icon imports.
Explicit system icon usage still enables the required native icon dependency.
