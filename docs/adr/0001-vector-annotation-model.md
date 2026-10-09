# Non-Destructive Vector Annotation Model

We decided to model all annotations (arrows, text, rectangles, step badges, and obfuscation masks) as discrete vector objects (`AnnotationItem`) maintained in a scene graph rather than baking strokes immediately into the raster bitmap.

This allows users to re-select, transform, adjust colors, and reorder elements after drawing, while rendering the final combined raster only upon export or clipboard copy.
