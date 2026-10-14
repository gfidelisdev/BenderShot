# Canvas Zoom, Viewport Transform, and Post-Creation Selection

## Context
1. Users requested the ability to zoom into captured screenshots and video frames using <kbd>Ctrl</kbd> + <kbd>Scroll</kbd> to inspect fine pixel details and place accurate annotations.
2. In previous versions, the canvas displayed images clamped strictly to the available window size with basic uniform scaling. There was no mechanism to zoom in/out, pan around enlarged content, or inspect pixels at true 1:1 or magnified ratios.
3. Users also identified an ergonomic friction when placing geometric annotations (arrows, rectangles, ellipses, béziers, polygons, and text): after creating a shape, the authoring tool remained active, requiring a manual click on the Selection tool before its handles could be adjusted.

## Decisions

1. **Focal Canvas Zoom (Cursor Anchor)**:
   - <kbd>Ctrl</kbd> + <kbd>Scroll</kbd> scales the canvas using a geometric step factor of `1.15x` per scroll tick, bounded between `10%` (`0.1x`) and `2000%` (`20.0x`).
   - The zoom anchor is focal to the current mouse cursor position, ensuring the pixel in `Image Space` beneath the cursor remains fixed in `Screen Space`.

2. **Pan Navigation (Pan Tool & Direct Gestures)**:
   - Provide direct gestures: Middle-click drag and <kbd>Space</kbd> + Left-click drag perform 2D viewport translation (`Pan Offset`) without altering the active annotation tool.
   - Normal vertical scroll pans vertically; <kbd>Shift</kbd> + scroll pans horizontally.
   - Introduce a dedicated `ToolMode::Pan` accessible via toolbar and <kbd>H</kbd> shortcut, displaying `Grab`/`Grabbing` mouse cursors.
   - Panning coordinates are elastically bounded to prevent the image from being lost off-screen (retaining a safety margin visible at all times).

3. **Strict Separation of Coordinate Spaces**:
   - All `AnnotationItem` models store coordinates strictly in `Image Space` (1:1 native pixels). Export pipelines render directly from image space to preserve native resolution.
   - Vector strokes scale visually with zoom (WYSIWYG preview).
   - Selection handles and hit-test targets maintain constant dimension in `Screen Space` (independent of zoom factor).

4. **Floating Adwaita HUD & Shortcuts**:
   - Render a floating Adwaita pill badge in the bottom-right corner: `[ - ]  [ 100% ]  [ + ]  [ Fit ]`.
   - In video editing mode, the HUD floats directly above the Video Timeline.
   - Standard shortcuts: <kbd>Ctrl+0</kbd> (Fit to Window), <kbd>Ctrl+1</kbd> (100% Actual Size), <kbd>Ctrl++</kbd> / <kbd>Ctrl+-</kbd>.
   - Viewport starts in intelligent `Fit-to-Window` mode, responsively adjusting during window resizing until manual zoom/pan occurs.

5. **Post-Creation Selection**:
   - Upon completing the placement of structured geometric shapes (**Arrow**, **Bézier**, **Rectangle**, **Ellipse**, **Polygon**, **Text**), the active tool automatically switches to `ToolMode::Select`, exposing interactive handles for immediate fine-tuning.
   - Continuous and destructive tools (**Freehand Pen**, **Freehand Highlighter**, **Step Badge**, **Obfuscation**, **Smudge Brush**) remain active for sequential operations.
