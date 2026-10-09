# Freehand Smudge Brush for Organic Obfuscation

We decided to implement Smudge as an interactive, continuous freehand brush (`ToolMode::Smudge`) that blends and transports color pixels dynamically along the cursor stroke path, rather than as a static bounding-box filter.

This allows users to obfuscate irregularly shaped text fragments, signatures, faces, or single lines with fluid finger-paint aesthetics, integrating directly into the unified undo/redo snapshot stack.
