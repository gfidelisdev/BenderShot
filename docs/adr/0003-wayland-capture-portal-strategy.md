# Wayland Capture Strategy via XDG Desktop Portal

We chose to use the XDG Desktop Portal (`org.freedesktop.portal.Screenshot`) for acquiring raw screen frames on Linux Wayland, combined with an in-app Loupe and pixel magnifier rendered directly over the retrieved pixel buffer inside our Editor Window.

This avoids the heavy runtime overhead and codec complexity of continuous PipeWire ScreenCast video pipelines while ensuring compliance with Wayland security constraints across GNOME and modern compositors.
