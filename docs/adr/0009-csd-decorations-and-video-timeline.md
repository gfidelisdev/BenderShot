# Client-Side Window Decorations, Wayland ScreenCast Recording, and Temporal Annotation Timeline

## Context
1. In Wayland environments (specifically GNOME Shell), relying on system window decorations via `libdecor` frequently results in generic or missing window controls (such as missing close "✕", minimize "─", or restore buttons).
2. Users require recording video screencasts (via CLI toggle or desktop shortcuts), reviewing recorded footage, and applying time-bounded annotations (such as arrows or obfuscation blur/pixelate over sensitive credentials like API keys) that activate during specific time intervals (`AnnotationSpan`) and disappear afterwards, exporting the result to MP4, WebM, or animated GIF.

## Decisions
1. **Integrated Adwaita Client-Side Decorations (CSD)**:
   - Configure the editor viewport with borderless/custom decorations (`with_decorations(false)` or headerbar window controls) and draw native GNOME Adwaita window buttons (Close "✕" with gentle red hover, Maximize/Restore "🗖", and Minimize "─") directly on the right side of the `adwaita_headerbar`. Window drag, maximize, minimize, and close commands are dispatched directly through `egui::ViewportCommand`.

2. **ScreenCast Recording Session via PipeWire / Portal**:
   - Implement `enhanced-shot record` as a toggle command: if not currently recording, initiate a screen capture session via Wayland ScreenCast portal/PipeWire using `ffmpeg` into a temporary lossless container (`/tmp/enhanced-shot-rec-*.mp4`). If already recording, signal termination, flush the stream, and open the recorded session directly in the Editor Window.
   - Display a lightweight, always-on-top `RecordingPill` during recording displaying elapsed duration and a stop action button.

3. **Multi-Track Video Timeline and Temporal Annotation Spans**:
   - Extend `AnnotationItem` and `Scene` with an optional `AnnotationSpan { start_ms: u64, end_ms: u64 }`.
   - Render a bottom `VideoTimeline` in the Editor Window featuring playback controls (Play/Pause, Scrubber playhead) and interactive horizontal span blocks allowing users to drag the start and end visibility times of any shape, arrow, or obfuscation filter.
   - Export through a `VideoExportPipeline` utilizing `ffmpeg` to composite active vector elements into the output stream in MP4 (H.264), WebM, or GIF format.
