package dev.nexa.videoplayer

import androidx.media3.common.MediaItem
import androidx.media3.exoplayer.ExoPlayer
import android.content.Context

public object VideoPlayerHost {
    public lateinit var context: Context
}

public class VideoPlayerImpl : VideoPlayerSpec {
    public var state: PlayerState = PlayerState.idle
        private set
    public var duration: Double = 0.0
        private set
    public override var volume: Double = 1.0
    public var onEnded: (() -> Unit)? = null

    private var player: ExoPlayer? = null

    public suspend fun prepare(url: String) {
        state = PlayerState.preparing
        val exoPlayer = player ?: ExoPlayer.Builder(VideoPlayerHost.context).build()
        exoPlayer.setMediaItem(MediaItem.fromUri(url))
        exoPlayer.prepare()
        player = exoPlayer
        state = PlayerState.ready
    }

    public override fun play() {
        player?.play()
        state = PlayerState.playing
    }

    public override fun pause() {
        player?.pause()
        state = PlayerState.paused
    }

    public override fun seek(position: Double) {
        player?.seekTo((position * 1000).toLong())
    }

    public override fun dispose() {
        player?.release()
        player = null
        state = PlayerState.idle
    }
}
