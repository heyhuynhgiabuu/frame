import Foundation
import ScreenCaptureKit
import AVFoundation

/// Manages macOS permission requests for screen recording, camera, and microphone.
@MainActor
final class PermissionsManager: Sendable {

    static let shared = PermissionsManager()

    private init() {}

    // MARK: - Request All

    func requestAllPermissions() {
        Task {
            await requestScreenRecordingPermission()
            await requestCameraPermission()
            await requestMicrophonePermission()
        }
    }

    // MARK: - Screen Recording

    /// Requests screen recording permission by fetching shareable content.
    /// ScreenCaptureKit will show the system permission dialog if not yet granted.
    func requestScreenRecordingPermission() async {
        do {
            // Fetching shareable content triggers the permission dialog
            _ = try await SCShareableContent.current
        } catch {
            print("[Permissions] Screen recording permission request failed: \(error.localizedDescription)")
        }
    }

    /// Checks if screen recording permission has been granted.
    func hasScreenRecordingPermission() async -> Bool {
        do {
            let content = try await SCShareableContent.current
            return !content.displays.isEmpty
        } catch {
            return false
        }
    }

    // MARK: - Camera

    func requestCameraPermission() async {
        let status = AVCaptureDevice.authorizationStatus(for: .video)
        switch status {
        case .notDetermined:
            let granted = await AVCaptureDevice.requestAccess(for: .video)
            print("[Permissions] Camera permission: \(granted ? "granted" : "denied")")
        case .authorized:
            print("[Permissions] Camera permission: already granted")
        case .denied, .restricted:
            print("[Permissions] Camera permission: denied or restricted")
        @unknown default:
            break
        }
    }

    // MARK: - Microphone

    func requestMicrophonePermission() async {
        let status = AVCaptureDevice.authorizationStatus(for: .audio)
        switch status {
        case .notDetermined:
            let granted = await AVCaptureDevice.requestAccess(for: .audio)
            print("[Permissions] Microphone permission: \(granted ? "granted" : "denied")")
        case .authorized:
            print("[Permissions] Microphone permission: already granted")
        case .denied, .restricted:
            print("[Permissions] Microphone permission: denied or restricted")
        @unknown default:
            break
        }
    }
}
