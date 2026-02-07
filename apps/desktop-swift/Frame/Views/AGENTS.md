# Views

SwiftUI views organized by app mode: Recording, Editor, Export, Shared.

## Where to Look

| Task                      | File                             | Notes                                           |
| ------------------------- | -------------------------------- | ----------------------------------------------- |
| Mode switching            | `ContentView.swift`              | Routes `.recorder` ↔ `.editor`                  |
| WYSIWYG preview           | `Editor/PreviewCanvas.swift`     | All effects + overlays rendered live            |
| Inspector tabs            | `Editor/EditorView.swift`        | HSplitView: preview + timeline + inspector      |
| Add inspector tab         | `Editor/Inspector/*.swift`       | One file per tab, uses `inspectorSection()`     |
| Cursor/keystroke overlays | `Editor/CursorOverlayView.swift` | Rendered on top of video in PreviewCanvas       |
| Timeline                  | `Editor/TimelineView.swift`      | Playback scrubbing + trim handles               |
| Export sheet              | `Export/ExportView.swift`        | Format/quality/resolution config                |
| Toolbar items             | `Shared/ToolbarItems.swift`      | Window toolbar (mode switch, settings)          |
| Recording idle screen     | `Recording/RecordingView.swift`  | Shown when main window visible in recorder mode |

## Architecture

```
ContentView (mode router)
├── RecordingView          (recorder mode — in main window, usually hidden)
└── EditorView             (editor mode)
    ├── PreviewCanvas      (WYSIWYG video preview)
    │   ├── VideoPlayerView
    │   ├── CursorOverlayView
    │   ├── WebcamOverlayView
    │   └── KeystrokeOverlayView
    ├── TimelineView       (playback + trim)
    └── Inspector panel
        ├── BackgroundInspector
        ├── CursorInspector
        ├── KeyboardInspector
        ├── WebcamInspector
        ├── ZoomInspector
        └── AudioInspector
```

## Inspector Pattern

All inspectors receive `Binding<EffectsConfig>` for two-way editing:

```swift
let effectsBinding = Binding<EffectsConfig>(
    get: { project.effects },
    set: { newEffects in
        project.effects = newEffects
        appState.currentProject = project
    }
)
```

Reusable components in `Inspector/InspectorComponents.swift`:

- `inspectorSection(_:content:)` — labeled section with uppercase title
- `SliderRow` — labeled slider with formatted value display

## Conventions

- **AppState access**: Always via `@Environment(AppState.self)`, use `@Bindable var appState = appState` for bindings
- **PreviewCanvas scale**: Normalizes to 1920×1080 reference resolution via `fitScale()`
- **VisualEffectBackground**: NSVisualEffectView wrapper for `.hudWindow` material in editor
- **EditorView layout**: HSplitView with inspector fixed at 280px width, timeline fixed at 120px height

## Anti-Patterns

- **Never** use `@StateObject` for AppState — it's `@Observable`, use `@Environment`
- **Never** pass `EffectsConfig` by value without a Binding — changes won't propagate to project
- **Never** render webcam preview as SwiftUI `Image(nsImage:)` during recording — use CIImageView
