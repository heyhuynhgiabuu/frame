import SwiftUI
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "FrameApp")

@main
struct FrameApp: App {
    @State private var appState = AppState()
    /// Delegate that manages main window visibility based on app mode.
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(appState)
                .frame(minWidth: 960, minHeight: 640)
                .background(WindowAccessor(appState: appState))
        }
        .windowStyle(.hiddenTitleBar)
        .windowToolbarStyle(.unified(showsTitle: true))
        .defaultSize(width: 1280, height: 800)
        .commands {
            CommandGroup(replacing: .newItem) {}

            CommandMenu("Record") {
                Button("Start Recording") {
                    appState.startRecording()
                }
                .keyboardShortcut("r", modifiers: [.command])
                .disabled(appState.isRecording)

                Button("Stop Recording") {
                    appState.stopRecording()
                }
                .keyboardShortcut(".", modifiers: [.command])
                .disabled(!appState.isRecording)

                Divider()

                Button("Pause") {
                    appState.togglePause()
                }
                .keyboardShortcut("p", modifiers: [.command])
                .disabled(!appState.isRecording)
            }

            CommandMenu("Export") {
                Button("Export Video...") {
                    appState.showExportSheet = true
                }
                .keyboardShortcut("e", modifiers: [.command, .shift])
                .disabled(appState.mode != .editor)
            }
        }
    }
}

// MARK: - Window Accessor

/// Captures the NSWindow reference from SwiftUI's WindowGroup and hands it to AppState.
/// This is the reliable way to get the actual NSWindow — unlike searching NSApp.windows,
/// this fires exactly when the hosting window is available.
struct WindowAccessor: NSViewRepresentable {
    let appState: AppState

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        // Defer to next run loop so the view is attached to its window
        DispatchQueue.main.async {
            guard let window = view.window else {
                logger.warning("WindowAccessor: no window found on view")
                return
            }

            logger.info("WindowAccessor: captured main window")
            appState.mainWindowController = window

            // Request permissions
            PermissionsManager.shared.requestAllPermissions()

            // In recorder mode, hide the main window and show floating overlays
            if appState.mode == .recorder {
                appState.showInitialOverlays()
            }
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {}
}

// MARK: - App Delegate

/// Prevents the app from terminating when the last window is closed.
/// In recorder mode, the main window is hidden — only floating panels are visible.
/// Without this, closing the main window would quit the app.
class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return false
    }
}
