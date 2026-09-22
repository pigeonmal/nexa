import AVFoundation

public final class VideoPlayerImpl: VideoPlayerSpec {
    public private(set) var state: PlayerState = .idle
    public private(set) var duration: Double = 0
    public var volume: Double
    public var onEnded: (() -> Void)?

    private var player: AVPlayer?

    public init() {
        self.volume = 1.0
        self.player = nil
    }

    public func prepare(url: String) async throws {
        guard let url = URL(string: url) else {
            throw PlayerError.invalidUrl
        }
        state = .preparing
        let item = AVPlayerItem(url: url)
        player = AVPlayer(playerItem: item)
        player?.volume = Float(volume)
        state = .ready
    }

    public func play() {
        player?.play()
        state = .playing
    }

    public func pause() {
        player?.pause()
        state = .paused
    }

    public func seek(position: Double) {
        player?.seek(to: CMTime(seconds: position, preferredTimescale: 600))
    }

    public func dispose() {
        player?.pause()
        player = nil
        state = .idle
    }
}
