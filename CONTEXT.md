# BenderShot

A high-performance screenshot capture, screen recording, annotation, and workflow tool for Linux Wayland written in Rust.

## Language

### Core Domain

**Capture Session**:
The complete flow from triggering screen acquisition via Wayland portals (silent snapshot without invoking compositor UI) to delivering a raw pixel buffer.
_Avoid_: Screenshot job, snapping, screen grab

**Capture Mode**:
The spatial targeting strategy for a Capture Session: `Region` (custom rectangle), `Fullscreen` (all displays), or `Window` (focused application).
_Avoid_: Grab type, snap mode

**Editor Window**:
The dedicated desktop window (`egui` + `winit`) styled after GNOME Libadwaita (supporting both Dark and Light themes) displaying the captured image or video with inspection, temporal timeline, and annotation tools in Spectacle style.
_Avoid_: Modal overlay, freeze canvas, popup editor

**Layer Stack**:
The ordered collection of editable vector items displayed in the collapsible right-side Layers panel, defining render depth (Z-order), selection, and element manipulation.
_Avoid_: Object list, drawing tree, shape manager

**Annotation Item**:
An individual non-destructive vector object (arrow, rectangle, polygon, ellipse, freehand stroke, bezier curve, text, badge, obfuscation region) stored in the session scene graph.
_Avoid_: Drawing stroke, paint stamp, markup layer

**Annotation Canvas**:
The editable vector viewport rendered on top of the captured raster bitmap or video stream holding user shapes and text.
_Avoid_: Paint layer, drawing board, markup sheet

**Constraint Modifier**:
The keyboard modifier (`Ctrl`) held during geometric shape dragging (Ellipse, Polygon, Rectangle) that constrains dimensions to a 1:1 aspect ratio or symmetric regular bounds.
_Avoid_: Shift lock, aspect lock

**Inline Text Editor**:
An interactive text input overlay spawned directly at canvas click coordinates with configurable font family, size, and background tags.
_Avoid_: Floating caption, title label, prompt popup

**Adwaita Theme**:
The official GNOME Libadwaita visual styling system applied to all controls (capsule buttons, pill segmented tools, rounded cards, comfortable spacing) with toggleable Dark and Light variants.
_Avoid_: Retro skin, default egui style

**Pin Window**:
A lightweight, borderless, always-on-top floating window holding an image slice for reference during multitasking.
_Avoid_: Sticky screenshot, float snip, picture-in-picture widget

**Indicator**:
The resident background service and top-bar status item exposing quick capture triggers and options via the StatusNotifierItem (SNI) D-Bus protocol.
_Avoid_: System tray applet, taskbar icon, daemon agent

### Viewport & Navigation

**Canvas Zoom**:
The interactive scaling transformation applied to the visual representation of the Annotation Canvas and base screenshot or video frame in the Editor Window.
_Avoid_: Scale factor, canvas resize, window magnification

**Zoom Anchor**:
The invariant focal point (positioned precisely under the mouse cursor during focal zoom) around which canvas coordinates expand or contract.
_Avoid_: Pivot point, zoom center, focus spot

**Viewport Transform**:
The mathematical projection composed of zoom scale and pan offset that maps elements between Image Space and Screen Space.
_Avoid_: Canvas matrix, screen mapping, view offset

**Pan Offset**:
The 2D positional translation vector applied to the Annotation Canvas viewport allowing inspection of off-screen regions of an enlarged image.
_Avoid_: Scroll coordinate, view shift, canvas drag

**Image Space**:
The canonical 1:1 pixel coordinate system of the underlying raster capture or video stream, in which all Annotation Item geometries and crop bounds are stored and exported.
_Avoid_: Canvas space, world pixels, raw coordinates

**Screen Space**:
The display-level window coordinate system in which GUI widgets, selection handles, and hit-test targets are measured in device-independent points.
_Avoid_: Window pixels, physical coordinates

**Pan Tool**:
A dedicated navigation tool selectable via the toolbar or keyboard shortcut that translates the canvas viewport via primary mouse drag, complementing middle-click and spacebar gestures.
_Avoid_: Hand tool, grab tool, mover

### Annotations & Drawing Tools

**Freehand Pen**:
An opaque continuous vector drawing stroke recording cursor trajectories with configurable line width, color, and optional smoothing.
_Avoid_: Pencil tool, brush scribble

**Freehand Highlighter**:
A translucid continuous vector stroke tool (calibrated at 35–40% alpha) designed to emphasize text or irregularly shaped visual areas while preserving background legibility.
_Avoid_: Marker pen, yellow highlighter, paint smear

**Stroke Smoothing**:
The algorithmic filter (Chaikin subdivision / Catmull-Rom spline) that refines freehand pointer coordinates, offering both a binary toggle (`Raw` vs `Smooth`) and a customizable smoothing intensity factor.
_Avoid_: Line stabilizer, curve flattener

**Bézier Curve**:
A vector curve defined by start, end, and curvature control points (3-point arc or multi-point spline), allowing fluid organic lines and curved arrows.
_Avoid_: Curved line, noodle, vector wire

**Polygon Tool**:
A unified geometric tool allowing creation of regular N-sided closed shapes (triangles, rectangles/quadrilaterals, pentagons, hexagons, octagons from 3 to 12 vertices) as well as freeform vertex-by-vertex polygons.
_Avoid_: Shape box, n-gon maker

**Step Badge**:
An automatically incrementing numbered circular marker placed on the canvas via a single click to document ordered workflows.
_Avoid_: Number callout, sequential bubble, ordered stamp

**Obfuscation**:
A destructive masked filter (blur or pixelation) applied immediately over the underlying visual buffer upon shape release.
_Avoid_: Redaction blur, censor box

**Smudge Brush**:
A continuous freehand stroke tool that smears and blends pixel colors dynamically along the mouse drag trajectory to obscure sensitive irregular visual areas.
_Avoid_: Finger smear, rub tool, blur pen

**Loupe**:
A pixel-magnifying inspection view with color value readout (Hex/RGB) for precise region selection and color sampling.
_Avoid_: Zoom preview, color picker magnifier

**Selection Boundary**:
The interactive visual handles and bounding outlines displayed when selecting an Annotation Item in the Editor Window. Hidden strictly during video playback and export pipelines to prevent visual pollution.
_Avoid_: Blue box, drag frame, item border

**Post-Creation Selection**:
The automatic transition from a geometric shape authoring tool to the Selection Tool upon completing the placement of an Annotation Item, immediately exposing interactive handles for fine-tuning.
_Avoid_: Auto pick, auto grab, tool switch on drop

### Video Recording & Timeline

**Recording Session**:
The active screen capture process utilizing native GNOME Shell Screencast API (`org.gnome.Shell.Screencast`) to record display outputs with zero window popups and native top-bar indicators.
_Avoid_: Video grab, screencast job, desktop stream

**Video Timeline**:
The multi-track time ruler, playhead scrubber, and temporal inspector component displayed along the lower edge of the Editor Window during video editing.
_Avoid_: Track editor, time bar, player seek

**Annotation Span**:
The temporal validity interval (`[start_ms, end_ms]`) associated with an Annotation Item, defining precisely when the element is rendered and active during video playback and export.
_Avoid_: Keyframe duration, layer time, visibility mark

**Write-on Transition**:
The organic entrance animation where an Annotation Item is progressively drawn from its origin across an elapsed time interval (`draw_transition_ms`, default 500ms), accompanied by optional fade-out exit transitions.
_Avoid_: Draw-in effect, pencil animation, line growth

**Video Export Pipeline**:
The rendering process that burns active Annotation Spans and transitions over the underlying video frame stream and encodes the result via `ffmpeg` to MP4, WebM, or animated GIF.
_Avoid_: Video burner, converter, save video

### Export & Storage

**Auto-Save Destination**:
The configured filesystem location (defaults to `~/Pictures/Screenshots/`) where captured renders are persisted automatically on copy/save.
_Avoid_: Output directory, dump folder

**Clipboard Pipeline**:
The Wayland clipboard export channel delivering rasterized PNG byte buffers.
_Avoid_: Copy paste handler, clipboard transfer

### Lifecycle, System Integration & Packaging

**Portal Permission Pre-Authorization**:
The proactive registration of `bendershot` authorization (`yes`) in `org.freedesktop.impl.portal.PermissionStore` table `screenshot`, preventing Wayland compositors (Mutter/GNOME Shell) from rejecting un-focused background shortcut invocations with access dialog errors.
_Avoid_: Force grant, hacky permission, bypass prompt

**Capture Retry Loop**:
The resilient, bounded attempt cycle (up to 3 attempts with 5-second timeouts and backoffs) ensuring transient D-Bus latency or portal startup races are handled transparently on the first user key press.
_Avoid_: Polling loop, retry spin

**Local Build Script**:
The root automation entry point (`./build_local.sh`) that compiles release/debug binaries, runs test validation, and packages native `.rpm`, `.deb`, `.AppImage`, and `.tar.gz` bundles into `packaging/dist`.
_Avoid_: Compile helper, manual package script

**In-Place Self-Updating**:
The automatic or manual upgrade workflow that queries GitHub Releases (`gfidelisdev/BenderShot`), matches the host distribution packaging (AppImage, RPM, DEB, tarball binary, Windows setup), downloads and validates release assets, applies updates (elevating via `pkexec` when system root permissions are required), and gracefully restarts resident background daemons.
_Avoid_: External updater, third-party installer script

