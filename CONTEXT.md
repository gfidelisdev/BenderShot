# Enhanced Shot

A high-performance screenshot capture, annotation, and workflow tool for Linux Wayland written in Rust.

## Language

### Core Domain

**Capture Session**:
The complete flow from triggering screen acquisition via Wayland portals (silent snapshot without invoking compositor UI) to delivering a raw pixel buffer.
_Avoid_: Screenshot job, snapping, screen grab

**Capture Mode**:
The spatial targeting strategy for a Capture Session: `Region` (custom rectangle), `Fullscreen` (all displays), or `Window` (focused application).
_Avoid_: Grab type, snap mode

**Editor Window**:
The dedicated desktop window (`egui` + `winit`) styled after GNOME Libadwaita (supporting both Dark and Light themes) displaying the captured image with inspection and annotation tools in Spectacle style.
_Avoid_: Modal overlay, freeze canvas, popup editor

**Layer Stack**:
The ordered collection of editable vector items displayed in the collapsible right-side Layers panel, defining render depth (Z-order), selection, and element manipulation.
_Avoid_: Object list, drawing tree, shape manager

**Annotation Item**:
An individual non-destructive vector object (arrow, rectangle, text, badge, obfuscation region) stored in the session scene graph.
_Avoid_: Drawing stroke, paint stamp, markup layer

**Annotation Canvas**:
The editable vector viewport rendered on top of the captured raster bitmap holding user shapes and text.
_Avoid_: Paint layer, drawing board, markup sheet

**Constraint Modifier**:
The keyboard modifier (`Ctrl`) held during geometric shape dragging (Ellipse, Rectangle) that constrains dimensions to a 1:1 aspect ratio (circle / square).
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

### Annotations & Inspection

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

### Export & Storage

**Auto-Save Destination**:
The configured filesystem location (defaults to `~/Pictures/Screenshots/`) where captured renders are persisted automatically on copy/save.
_Avoid_: Output directory, dump folder

**Clipboard Pipeline**:
The Wayland clipboard export channel delivering rasterized PNG byte buffers.
_Avoid_: Copy paste handler, clipboard transfer
