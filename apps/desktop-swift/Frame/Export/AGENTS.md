# Export

AVAssetReader → CIImage effects compositing → AVAssetWriter pipeline.

## Where to Look

| Task            | File                 | Notes                                              |
| --------------- | -------------------- | -------------------------------------------------- |
| Export pipeline | `ExportEngine.swift` | Full read→process→write pipeline                   |
| Export settings | `ExportConfig.swift` | Format, quality, resolution, frame rate data model |

## Pipeline

```
AVAssetReader (source .mov)
  → AVAssetReaderTrackOutput (video: BGRA pixels, audio: Linear PCM)
    → CIImage effects compositing (per-frame)
      → AVAssetWriterInputPixelBufferAdaptor (rendered output)
        → AVAssetWriter (output file)
```

## Effects Compositing (per-frame)

Applied in `applyEffects(to:effects:canvasSize:)`:

1. **Background**: Solid color (`CIImage(color:)`) or gradient (`CIFilter.linearGradient()`)
2. **Corner radius**: `CIFilter.roundedRectangleGenerator()` as mask → `CIFilter.blendWithMask()`
3. **Shadow**: Source → black silhouette (`colorMatrix`) → `gaussianBlur` → position with offsetY
4. **Padding**: Source translated by padding amount, composited over background
5. **Trim**: Applied via `AVAssetReader.timeRange`

## Output Formats

| Format | Codec      | Audio       | Notes                                                            |
| ------ | ---------- | ----------- | ---------------------------------------------------------------- |
| MOV    | ProRes 422 | AAC 44.1kHz | Highest quality                                                  |
| MP4    | H.264      | AAC 44.1kHz | H.264 High profile                                               |
| GIF    | —          | None        | Defined in ExportConfig, **not yet implemented in ExportEngine** |

## Anti-Patterns

- **Never** skip `isReadyForMoreMediaData` check — busy-wait with `Thread.sleep(0.01)` if not ready
- **Never** create CIContext per-export — initialized once with GPU + `highQualityDownsample`
- **Never** forget to call `reader.cancelReading()` after writer finishes — cleanup
- **Never** use `expectsMediaDataInRealTime = true` for export — it's offline processing (set `false`)

## Key Details

- Progress: Updated every 5 frames, capped at 0.9 during rendering (0.95 during finalize)
- Cancellation: `Task.checkCancellation()` called per-frame for responsive cancel
- ExportPhase enum tracks: idle → preparing → rendering → encoding → finalizing → complete/failed
- Canvas size = source resolution + (padding × 2), then scaled to output resolution
- `autoreleasepool` wraps each frame to prevent memory accumulation
