# VideoPlayer reference plugin

This package demonstrates a stateful native plugin with typed value and enum contracts in `native.nxid`, independent native player instances, events, lifecycle cleanup, and a `VideoView` component.

The platform implementations stay in the plugin package. Nexa generates their typed Swift and Kotlin bindings during app project generation and copies the native sources into the generated iOS and Android hosts.

The complete two-instance example is in [`../video-player-demo.nx`](../video-player-demo.nx). Create a project around that `.nx` entrypoint and use `nexa dev` to generate, build, and launch it on an available simulator or emulator.
