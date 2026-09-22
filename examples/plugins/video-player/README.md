# VideoPlayer reference plugin

This package demonstrates a stateful native plugin with:

- `plugin.config.nx` manifest metadata and platform source globs;
- typed value, enum, and error contracts in `native.nxid`;
- an independently constructible `VideoPlayer` native class;
- direct Swift and Kotlin implementation boundaries;
- a `VideoView` native component contract with a direct platform wrapper.

Generate the typed contracts before compiling the platform implementations:

```sh
nexa plugin check examples/plugins/video-player
nexa plugin generate examples/plugins/video-player --target swift --out /tmp/VideoPlayerBindings.swift
nexa plugin generate examples/plugins/video-player --target kotlin --package dev.nexa.videoplayer --out /tmp/VideoPlayerBindings.kt
```

The platform implementations intentionally stay in the plugin package. Nexa
apps construct two independent players with `VideoPlayer()` and call methods
on each receiver. `nexa generate` copies the generated Swift/Kotlin contracts
into the host project automatically. `VideoPlayer.VideoView(player: ..., controls: ...)`
is lowered to the generated SwiftUI/Compose wrapper, which calls the platform
`VideoViewImpl` directly; the two wrappers keep the player object independent
from the visual component.

The complete two-instance example is in
[`../video-player-demo.nx`](../video-player-demo.nx). Build it with:

```sh
nexa build examples/plugins/video-player-demo.nx --target all
```
