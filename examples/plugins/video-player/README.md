# VideoPlayer reference plugin

This package demonstrates a stateful native plugin with:

- `plugin.config.nx` manifest metadata and platform source globs;
- typed value and enum contracts in `native.nxid`;
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
apps construct two independent players with `VideoPlayer()`, prepare different
media URLs, and call play/pause methods on each receiver. `OnDisappear` disposes
both players. Headless runtime tests also verify that disposing one player
leaves the other independently usable. Generated SwiftUI and Compose code retains each
native player instance across view redraws. The `ended` event callback is
registered independently on each player and cleared during disposal. The two
native views also receive independent `onTapped` handlers. `nexa generate`
copies the generated Swift/Kotlin contracts into the host project automatically.
`VideoPlayer.VideoView(player: ..., controls: ...)` is lowered to the generated
SwiftUI/Compose wrapper, which calls the platform `VideoViewImpl` directly.
The component declares a content slot, so app-authored Nexa child views are
passed directly into the native SwiftUI/Compose implementation.
Android players receive an application context from their own Compose view and
do not use a process-global activity holder.

The complete two-instance example is in
[`../video-player-demo.nx`](../video-player-demo.nx). Build it with:

```sh
nexa build examples/plugins/video-player-demo.nx --target all
```
