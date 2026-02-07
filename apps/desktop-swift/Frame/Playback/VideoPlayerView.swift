import AVKit
import SwiftUI

/// NSViewRepresentable wrapper for AVPlayerView.
/// Displays video playback with native macOS controls hidden (we use our own timeline).
struct VideoPlayerView: NSViewRepresentable {
    let player: AVPlayer

    func makeNSView(context: Context) -> AVPlayerView {
        let view = AVPlayerView()
        view.player = player
        view.controlsStyle = .none       // We provide our own controls
        view.showsFullScreenToggleButton = false
        view.allowsPictureInPicturePlayback = false
        view.videoGravity = .resizeAspect
        return view
    }

    func updateNSView(_ nsView: AVPlayerView, context: Context) {
        if nsView.player !== player {
            nsView.player = player
        }
    }
}
