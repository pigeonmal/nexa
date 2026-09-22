# VideoPlayer reference plugin

This package demonstrates a stateful native plugin with:

- `plugin.config.nx` manifest metadata and platform source globs;
- typed value, enum, and error contracts in `native.nxid`;
- an independently constructible `VideoPlayer` native class;
- direct Swift and Kotlin implementation boundaries;
- a future `VideoView` native component contract.

Generate the typed contracts before compiling the platform implementations:

```sh
nexa plugin check examples/plugins/video-player
nexa plugin generate examples/plugins/video-player --target swift --out /tmp/VideoPlayerBindings.swift
nexa plugin generate examples/plugins/video-player --target kotlin --package dev.nexa.videoplayer --out /tmp/VideoPlayerBindings.kt
```

The platform implementations intentionally stay in the plugin package. Nexa
apps construct two independent players with `VideoPlayer()` and call methods
on each receiver. `nexa generate` copies the generated Swift/Kotlin contracts
into the host project automatically; the component contract is parsed and
generated as a native contract, while UI lowering remains the next native
component phase.
