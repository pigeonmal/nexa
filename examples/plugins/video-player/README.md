# VideoPlayer reference plugin

This package provides independent native player instances, typed playback state and errors, an ended event, and a `VideoView` with native playback controls. iOS uses AVPlayer and Android uses Media3 ExoPlayer. The Android package includes the Media3 HLS and DASH modules; iOS uses AVPlayer's HLS support.

The platform implementations stay in the plugin package. Nexa generates their typed Swift and Kotlin bindings during app project generation and copies the native sources into the generated iOS and Android hosts. Editing app calls, player bindings, and component arguments is compatible with DevRuntime hot reload; changing this package's contract, native source, dependencies, or host metadata requires rebuilding the host.

The complete two-instance example is in [`../video-player-demo.nx`](../video-player-demo.nx). Create a project around that `.nx` entrypoint and use `nexa dev` to generate, build, and launch it on an available simulator or emulator.

On Android 8.0 and later, the generated host opts into Picture-in-Picture when this plugin is reachable. PiP begins when the app is backgrounded while a visible player is actively playing; on Android 12 and later it uses the automatic home gesture transition. The iOS system player exposes its native PiP and playback controls.
