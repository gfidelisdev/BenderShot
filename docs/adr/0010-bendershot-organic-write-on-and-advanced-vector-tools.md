# BenderShot Rebranding, Organic Write-on Transitions, and Advanced Vector Geometry Tools

## Context
1. The project has evolved from a basic screenshot grabber into a full-featured Wayland screen capture, recording, temporal video editor, and vector annotation suite. The user requested rebranding the application to **BenderShot**.
2. Video annotations previously appeared via sudden pop-in cuts. Users requested organic write-on transitions (progressively drawing shapes like arrows, ellipses, and freehand strokes over ~500ms as if drawn by hand in real-time) and smooth fade-out exits.
3. Selection boundary rectangles on curved or non-rectangular shapes (such as ellipses) caused visual pollution, especially during playback and export.
4. Users requested a dedicated Freehand Highlighter (translucent marker pen preserving background contrast), customizable stroke smoothing (both rapid toggle and granular intensity), Bézier curve modes, and a unified Polygon Tool supporting regular polygons (3 to 12 sides, defaulting to rectangles) as well as arbitrary vertex-by-vertex paths.

## Decisions
1. **Rebranding to BenderShot**:
   - Update binary, desktop metadata, user systemd services, packaging scripts, and UI window titles to `BenderShot` (`bendershot`). Maintain compatibility aliases for existing shortcuts.

2. **Selection Boundary Suppression**:
   - Strictly suppress all selection handles, bounding boxes, and transformer controls during active video playback (`is_playing == true`) and inside the video rasterization and export pipelines.
   - For curved objects like ellipses, use discrete cardinal tangent handles rather than intrusive rectangular boxes.

3. **Temporal Write-on and Fade-out Transitions**:
   - Extend `AnnotationSpan` with `draw_transition_ms: u32` (default 500ms, configurable) and `fade_out_ms: u32` (default 200ms).
   - During the entrance phase (`time_ms - start_ms < draw_transition_ms`), interpolate geometry length:
     - *Arrows*: The shaft extends towards the target; the arrowhead is displayed and travels with the tip as soon as length exceeds arrowhead dimension.
     - *Ellipses & Polygons*: The perimeter path traces continuously from 0° to 360° or vertex-by-vertex.
     - *Freehand Pen & Highlighter*: Trajectory points are rendered proportionally to elapsed progress.
   - During the exit phase (`end_ms - time_ms < fade_out_ms`), apply smooth alpha fading.

4. **Dedicated Freehand Tools & Dual-Mode Smoothing**:
   - Separate `FreehandPen` (opaque) and `FreehandHighlighter` (semi-transparent ~35% alpha with background preservation).
   - Provide both a quick `[Raw / Smooth]` toggle and an adjustable smoothing intensity slider using Chaikin subdivision and Catmull-Rom spline filtering.

5. **Bézier Curves & Multi-sided Polygon Tool**:
   - Introduce 3-point Bézier Arc creation (start, end, curvature control point) for fast curved lines and arrows, with multi-node support.
   - Upgrade the Rectangle tool into a unified `PolygonTool`: regular mode with 3 to 12 vertices (with 4-sided rectangle as immediate default) plus freeform vertex-by-vertex polygon mode.
