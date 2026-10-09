# GUI Stack Selection: egui and winit

We chose `egui` coupled with `winit` for the Editor Window instead of GTK4/Libadwaita or a custom software-rendered toolkit.

This provides near-instant cold startup (< 30ms), zero heavyweight native C library dependencies, a memory footprint under 40 MB, and immediate-mode controls for toolbars, color palettes, and inspector menus while allowing custom canvas rendering.
