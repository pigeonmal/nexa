# VideoPlayer reference plugin

This package provides independent native player instances, typed playback state and errors, an ended event, and a `VideoView` with native playback controls. iOS uses Apple's AVPlayer; Android uses Media3 ExoPlayer with NextLib's FFmpeg fallback. Android includes Media3 HLS and DASH modules and uses app-packaged Cronet for media requests.

On iOS, playback uses AVPlayer and its native decoder. On Android, pass `softwareDecodingEnabled: false` to `VideoView` to disable the FFmpeg fallback. The default is `true`; platform decoders remain first choice. Media3 is pinned to 1.11.1 and NextLib to 1.11.1-0.16.0.

NextLib is GPL-3.0 licensed. Apps that ship its Android dependency must comply with its license. The iOS plugin has no third-party playback dependency.

The platform implementations stay in the plugin package. Nexa generates their typed Swift and Kotlin bindings during app project generation and copies the native sources into the generated iOS and Android hosts. Editing app calls, player bindings, and component arguments is compatible with DevRuntime hot reload; changing this package's contract, native source, dependencies, or host metadata requires rebuilding the host.

The complete two-instance example is in [`../video-player-demo.nx`](../video-player-demo.nx). Create a project around that `.nx` entrypoint and use `nexa dev` to generate, build, and launch it on an available simulator or emulator.

On Android 8.0 and later, the generated host opts into Picture-in-Picture when this plugin is reachable. PiP begins when the app is backgrounded while a visible player is actively playing; on Android 12 and later it uses the automatic home gesture transition. The iOS system player exposes its native PiP and playback controls.
