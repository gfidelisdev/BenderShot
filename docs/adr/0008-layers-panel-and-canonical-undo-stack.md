# Layers Panel, Typography Styling, and Canonical Undo Stack

We decided to implement a collapsible right-hand Layers panel allowing discrete selection, reordering (Z-index), and removal of vector items, coupled with typography controls (Sans/Mono/Serif, size, and background tags).

Furthermore, we unified all canvas state mutations (vector items, crops, and raster obfuscation filters) into a canonical linear state undo/redo stack and secured viewport coordinate mapping with strict bounds checking.
