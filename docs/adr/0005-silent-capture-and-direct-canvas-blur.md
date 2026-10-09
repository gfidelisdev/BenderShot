# Silent Capture and Direct Pixel Buffer Obfuscation

We decided to disable interactive portal prompts (`interactive: false`) to capture the desktop instantly without triggering GNOME's native screenshot overlay dialog, opening our Editor Window directly.

Furthermore, obfuscation operations (Blur and Pixelate) are baked immediately into the live working image buffer upon mouse release with full undo/redo support, guaranteeing instant WYSIWYG feedback inside the editor before export.
