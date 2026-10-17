#!/usr/bin/env python3
"""BenderShot GNOME helper.

Subcommands:
  window OUT   Capture the currently focused window via org.gnome.Mutter.ScreenCast
               (RecordWindow without window-id = focused window) into a PNG at OUT.
  monitors     Print the logical monitor layout (org.gnome.Mutter.DisplayConfig) as JSON.
  select       Show a live-screen transparent region selector overlay across all monitors;
               print JSON {"rect": [x0, y0, x1, y1], "overlay_size": [w, h]} or null on cancel.
"""
import json
import math
import os
import sys

import gi
from gi.repository import Gio, GLib


def bus():
    return Gio.bus_get_sync(Gio.BusType.SESSION, None)


def capture_window(out):
    gi.require_version('Gst', '1.0')
    from gi.repository import Gst
    Gst.init(None)
    b = bus()
    dest = 'org.gnome.Mutter.ScreenCast'

    def call(path, iface, method, args, rtype):
        return b.call_sync(dest, path, iface, method, args,
                           GLib.VariantType(rtype) if rtype else None,
                           Gio.DBusCallFlags.NONE, 5000, None)

    sess = call('/org/gnome/Mutter/ScreenCast', 'org.gnome.Mutter.ScreenCast', 'CreateSession',
                GLib.Variant('(a{sv})', ({},)), '(o)').unpack()[0]
    stream = call(sess, 'org.gnome.Mutter.ScreenCast.Session', 'RecordWindow',
                  GLib.Variant('(a{sv})', ({'cursor-mode': GLib.Variant('u', 0)},)), '(o)').unpack()[0]
    loop = GLib.MainLoop()
    node = {}

    def on_added(_conn, _sender, _path, _iface, _signal, params, *_a):
        node['id'] = params.unpack()[0]
        loop.quit()

    b.signal_subscribe(None, 'org.gnome.Mutter.ScreenCast.Stream', 'PipeWireStreamAdded', stream,
                       None, Gio.DBusSignalFlags.NONE, on_added)
    call(sess, 'org.gnome.Mutter.ScreenCast.Session', 'Start', None, None)
    GLib.timeout_add_seconds(5, loop.quit)
    loop.run()

    msg = None
    if 'id' in node:
        pipe = Gst.parse_launch(
            'pipewiresrc path=%d always-copy=true ! videoconvert ! video/x-raw,format=RGBA ! '
            'pngenc snapshot=true ! filesink location="%s"' % (node['id'], out))
        pipe.set_state(Gst.State.PLAYING)
        msg = pipe.get_bus().timed_pop_filtered(5 * Gst.SECOND, Gst.MessageType.EOS | Gst.MessageType.ERROR)
        pipe.set_state(Gst.State.NULL)
    try:
        call(sess, 'org.gnome.Mutter.ScreenCast.Session', 'Stop', None, None)
    except Exception:
        pass
    if msg is None or msg.type != Gst.MessageType.EOS:
        sys.stderr.write('window capture failed\n')
        return 3
    print(out)
    return 0


def get_monitors_list():
    b = bus()
    res = b.call_sync('org.gnome.Mutter.DisplayConfig', '/org/gnome/Mutter/DisplayConfig',
                      'org.gnome.Mutter.DisplayConfig', 'GetCurrentState', None, None,
                      Gio.DBusCallFlags.NONE, 5000, None).unpack()
    _serial, phys, logical, props = res
    layout_mode = props.get('layout-mode', 1)
    modes = {}
    for spec, mode_list, _p in phys:
        for m in mode_list:
            if m[6].get('is-current', False):
                modes[spec[0]] = (m[1], m[2])
    out = []
    for x, y, scale, transform, primary, specs, _p in logical:
        if not specs or specs[0][0] not in modes:
            continue
        w, h = modes[specs[0][0]]
        if transform % 2 == 1:
            w, h = h, w
        lw, lh = w / scale, h / scale
        sw, sh = (lw, lh) if layout_mode == 1 else (w, h)
        out.append({'x': x, 'y': y, 'w': sw, 'h': sh, 'lw': lw, 'lh': lh,
                    'scale': scale, 'primary': bool(primary)})
    return out


def monitors():
    print(json.dumps(get_monitors_list()))
    return 0


def rounded_rect(cr, x, y, w, h, r):
    r = min(r, w / 2.0, h / 2.0)
    cr.new_sub_path()
    cr.arc(x + w - r, y + r, r, -math.pi / 2, 0)
    cr.arc(x + w - r, y + h - r, r, 0, math.pi / 2)
    cr.arc(x + r, y + h - r, r, math.pi / 2, math.pi)
    cr.arc(x + r, y + r, r, math.pi, 3 * math.pi / 2)
    cr.close_path()


def select_region(demo_rect=None):
    os.environ['GDK_BACKEND'] = 'x11'
    gi.require_version('Gtk', '3.0')
    gi.require_version('Gdk', '3.0')
    from gi.repository import Gtk, Gdk
    import cairo

    win = Gtk.Window(type=Gtk.WindowType.POPUP)
    screen = win.get_screen()
    visual = screen.get_rgba_visual()
    if visual is not None:
        win.set_visual(visual)
    win.set_app_paintable(True)
    win.set_decorated(False)
    win.set_can_focus(True)

    sw = screen.get_width()
    sh = screen.get_height()
    win.move(0, 0)
    win.resize(sw, sh)

    win.add_events(
        Gdk.EventMask.BUTTON_PRESS_MASK
        | Gdk.EventMask.BUTTON_RELEASE_MASK
        | Gdk.EventMask.POINTER_MOTION_MASK
        | Gdk.EventMask.KEY_PRESS_MASK
        | Gdk.EventMask.STRUCTURE_MASK
    )

    display = Gdk.Display.get_default()
    seat = display.get_default_seat()

    try:
        mons = get_monitors_list()
    except Exception:
        mons = [{'x': 0, 'y': 0, 'w': sw, 'h': sh, 'primary': True}]

    state = {
        'sel': list(demo_rect) if demo_rect else None,  # [x0, y0, x1, y1] normalized
        'mode': 'none',      # 'none' | 'creating' | 'moving' | 'resizing'
        'start': (0.0, 0.0),
        'offset': (0.0, 0.0),
        'handle': None,
        'pointer': (sw / 2.0, sh / 2.0),
        'result': None,
        'grabbed': False,
    }

    MIN_SIZE = 8.0
    GRAB_DIST = 12.0

    def norm_rect(x0, y0, x1, y1):
        return [min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1)]

    def get_handles(sel):
        x0, y0, x1, y1 = sel
        cx = (x0 + x1) / 2.0
        cy = (y0 + y1) / 2.0
        return [
            ('tl', x0, y0, Gdk.CursorType.TOP_LEFT_CORNER),
            ('tc', cx, y0, Gdk.CursorType.TOP_SIDE),
            ('tr', x1, y0, Gdk.CursorType.TOP_RIGHT_CORNER),
            ('lc', x0, cy, Gdk.CursorType.LEFT_SIDE),
            ('rc', x1, cy, Gdk.CursorType.RIGHT_SIDE),
            ('bl', x0, y1, Gdk.CursorType.BOTTOM_LEFT_CORNER),
            ('bc', cx, y1, Gdk.CursorType.BOTTOM_SIDE),
            ('br', x1, y1, Gdk.CursorType.BOTTOM_RIGHT_CORNER),
        ]

    def get_action_bar(sel):
        x0, y0, x1, y1 = sel
        if (x1 - x0) < MIN_SIZE or (y1 - y0) < MIN_SIZE:
            return None
        bar_w, bar_h, margin = 88.0, 42.0, 8.0
        bx = x1 - bar_w
        by = y1 + margin
        if by + bar_h > sh - 4.0:
            by = y1 - bar_h - margin
        bx = max(4.0, min(bx, sw - bar_w - 4.0))
        by = max(4.0, min(by, sh - bar_h - 4.0))
        cancel_center = (bx + 23.0, by + bar_h / 2.0)
        tick_center = (bx + bar_w - 23.0, by + bar_h / 2.0)
        return (bx, by, bar_w, bar_h), cancel_center, tick_center, 15.0

    def point_in_circle(px, py, cx, cy, r):
        return math.hypot(px - cx, py - cy) <= r

    def point_in_rect(px, py, rx, ry, rw, rh):
        return rx <= px <= rx + rw and ry <= py <= ry + rh

    def set_cursor(ctype):
        gdk_win = win.get_window()
        if gdk_win is not None:
            cur = Gdk.Cursor.new_for_display(display, ctype)
            gdk_win.set_cursor(cur)

    def finish(res):
        state['result'] = res
        try:
            if state['grabbed']:
                seat.ungrab()
                state['grabbed'] = False
        except Exception:
            pass
        win.hide()
        win.destroy()
        display.flush()
        display.sync()
        Gtk.main_quit()

    def try_grab():
        gdk_win = win.get_window()
        if gdk_win is None or state['grabbed']:
            return
        gdk_win.focus(Gdk.CURRENT_TIME)
        cur = Gdk.Cursor.new_for_display(display, Gdk.CursorType.CROSSHAIR)
        st = seat.grab(gdk_win, Gdk.SeatCapabilities.ALL, True, cur, None, None)
        if st == Gdk.GrabStatus.SUCCESS:
            state['grabbed'] = True

    def on_map(_widget, _event):
        GLib.timeout_add(30, lambda: (try_grab(), False)[1])
        return False

    def on_key_press(_widget, event):
        key = event.keyval
        if key == Gdk.KEY_Escape:
            finish(None)
            return True
        if key in (Gdk.KEY_Return, Gdk.KEY_KP_Enter):
            sel = state['sel']
            if sel and (sel[2] - sel[0]) >= MIN_SIZE and (sel[3] - sel[1]) >= MIN_SIZE:
                finish({'rect': sel, 'overlay_size': [float(sw), float(sh)]})
            return True
        return False

    def on_button_press(_widget, event):
        try_grab()
        px, py = float(event.x), float(event.y)
        state['pointer'] = (px, py)

        # Right-click cancels
        if event.button == 3:
            finish(None)
            return True

        if event.button != 1:
            return False

        sel = state['sel']
        if sel is not None:
            bar_info = get_action_bar(sel)
            if bar_info is not None:
                (bx, by, bw, bh), (cx, cy), (tx, ty), r = bar_info
                if point_in_circle(px, py, tx, ty, r + 3.0):
                    finish({'rect': sel, 'overlay_size': [float(sw), float(sh)]})
                    return True
                if point_in_circle(px, py, cx, cy, r + 3.0):
                    finish(None)
                    return True
                if point_in_rect(px, py, bx, by, bw, bh):
                    return True

            for hname, hx, hy, ctype in get_handles(sel):
                if math.hypot(px - hx, py - hy) <= GRAB_DIST:
                    state['mode'] = 'resizing'
                    state['handle'] = hname
                    set_cursor(ctype)
                    win.queue_draw()
                    return True

            if sel[0] <= px <= sel[2] and sel[1] <= py <= sel[3]:
                state['mode'] = 'moving'
                state['offset'] = (px - sel[0], py - sel[1])
                set_cursor(Gdk.CursorType.FLEUR)
                win.queue_draw()
                return True

        state['mode'] = 'creating'
        state['start'] = (px, py)
        state['sel'] = [px, py, px, py]
        set_cursor(Gdk.CursorType.CROSSHAIR)
        win.queue_draw()
        return True

    def on_motion(_widget, event):
        px = max(0.0, min(float(event.x), float(sw)))
        py = max(0.0, min(float(event.y), float(sh)))
        state['pointer'] = (px, py)
        mode = state['mode']
        sel = state['sel']

        if mode == 'creating':
            sx, sy = state['start']
            state['sel'] = norm_rect(sx, sy, px, py)
            win.queue_draw()
            return True

        if mode == 'moving' and sel is not None:
            w = sel[2] - sel[0]
            h = sel[3] - sel[1]
            ox, oy = state['offset']
            nx0 = max(0.0, min(px - ox, sw - w))
            ny0 = max(0.0, min(py - oy, sh - h))
            state['sel'] = [nx0, ny0, nx0 + w, ny0 + h]
            win.queue_draw()
            return True

        if mode == 'resizing' and sel is not None:
            x0, y0, x1, y1 = sel
            h = state['handle']
            if h in ('tl', 'lc', 'bl'):
                x0 = min(px, x1 - MIN_SIZE)
            if h in ('tr', 'rc', 'br'):
                x1 = max(px, x0 + MIN_SIZE)
            if h in ('tl', 'tc', 'tr'):
                y0 = min(py, y1 - MIN_SIZE)
            if h in ('bl', 'bc', 'br'):
                y1 = max(py, y0 + MIN_SIZE)
            state['sel'] = [x0, y0, x1, y1]
            win.queue_draw()
            return True

        # Hover cursor update when idle
        if sel is not None:
            bar_info = get_action_bar(sel)
            if bar_info is not None:
                (bx, by, bw, bh), (cx, cy), (tx, ty), r = bar_info
                if point_in_rect(px, py, bx, by, bw, bh):
                    set_cursor(Gdk.CursorType.HAND2)
                    win.queue_draw()
                    return True
            for _hname, hx, hy, ctype in get_handles(sel):
                if math.hypot(px - hx, py - hy) <= GRAB_DIST:
                    set_cursor(ctype)
                    return True
            if sel[0] <= px <= sel[2] and sel[1] <= py <= sel[3]:
                set_cursor(Gdk.CursorType.FLEUR)
                return True

        set_cursor(Gdk.CursorType.CROSSHAIR)
        if sel is not None:
            win.queue_draw()
        return True

    def on_button_release(_widget, event):
        if event.button != 1:
            return False
        if state['mode'] != 'none':
            state['mode'] = 'none'
            sel = state['sel']
            if sel is not None and ((sel[2] - sel[0]) < MIN_SIZE or (sel[3] - sel[1]) < MIN_SIZE):
                state['sel'] = None
            win.queue_draw()
        return True

    def on_draw(_widget, cr):
        # 1. Clear with dark translucent veil over the entire desktop
        cr.set_operator(cairo.OPERATOR_SOURCE)
        sel = state['sel']
        if sel is not None and (sel[2] - sel[0]) >= 1.0 and (sel[3] - sel[1]) >= 1.0:
            x0, y0, x1, y1 = sel
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.45)
            cr.paint()
            # Punch transparent hole so live screen is 100% clear inside selection
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0)
            cr.rectangle(x0, y0, x1 - x0, y1 - y0)
            cr.fill()

            cr.set_operator(cairo.OPERATOR_OVER)
            # Selection border (#3584e4)
            cr.set_source_rgba(53 / 255.0, 132 / 255.0, 228 / 255.0, 1.0)
            cr.set_line_width(2.0)
            cr.rectangle(x0, y0, x1 - x0, y1 - y0)
            cr.stroke()

            # Dimension badge ("W × H px")
            w_px = int(round(x1 - x0))
            h_px = int(round(y1 - y0))
            label = f"{w_px} × {h_px} px"
            cr.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
            cr.set_font_size(12.0)
            ext = cr.text_extents(label)
            bw = ext.width + 18.0
            bh = 24.0
            bx = x0
            by = y0 - bh - 6.0 if y0 > bh + 10.0 else y0 + 6.0
            rounded_rect(cr, bx, by, bw, bh, 5.0)
            cr.set_source_rgba(0.08, 0.08, 0.1, 0.85)
            cr.fill()
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.95)
            cr.move_to(bx + 9.0, by + bh / 2.0 + ext.height / 2.0 - 1.0)
            cr.show_text(label)

            # 8 circular anchor handles (corners + 4 side midpoints)
            for _hname, hx, hy, _ctype in get_handles(sel):
                cr.arc(hx, hy, 5.5, 0, 2 * math.pi)
                cr.set_source_rgba(1.0, 1.0, 1.0, 1.0)
                cr.fill_preserve()
                cr.set_source_rgba(53 / 255.0, 132 / 255.0, 228 / 255.0, 1.0)
                cr.set_line_width(2.0)
                cr.stroke()

            # Attached action bar with Cancel (✕) and Confirm (✔) when not actively dragging
            if state['mode'] == 'none':
                bar_info = get_action_bar(sel)
                if bar_info is not None:
                    (bx, by, bar_w, bar_h), (cx, cy), (tx, ty), r = bar_info
                    px, py = state['pointer']
                    hov_cancel = point_in_circle(px, py, cx, cy, r + 2.0)
                    hov_tick = point_in_circle(px, py, tx, ty, r + 2.0)

                    # Pill background
                    rounded_rect(cr, bx, by, bar_w, bar_h, 21.0)
                    cr.set_source_rgba(30 / 255.0, 30 / 255.0, 38 / 255.0, 0.96)
                    cr.fill_preserve()
                    cr.set_source_rgba(70 / 255.0, 70 / 255.0, 86 / 255.0, 0.9)
                    cr.set_line_width(1.2)
                    cr.stroke()

                    # Cancel button (✕)
                    cr.arc(cx, cy, r, 0, 2 * math.pi)
                    if hov_cancel:
                        cr.set_source_rgba(224 / 255.0, 27 / 255.0, 36 / 255.0, 1.0)
                    else:
                        cr.set_source_rgba(56 / 255.0, 56 / 255.0, 68 / 255.0, 1.0)
                    cr.fill()
                    # Draw crisp ✕ icon
                    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0)
                    cr.set_line_width(2.2)
                    cr.set_line_cap(cairo.LINE_CAP_ROUND)
                    d = 4.8
                    cr.move_to(cx - d, cy - d)
                    cr.line_to(cx + d, cy + d)
                    cr.move_to(cx + d, cy - d)
                    cr.line_to(cx - d, cy + d)
                    cr.stroke()

                    # Confirm / Tick button (✔)
                    cr.arc(tx, ty, r, 0, 2 * math.pi)
                    if hov_tick:
                        cr.set_source_rgba(58 / 255.0, 215 / 255.0, 142 / 255.0, 1.0)
                    else:
                        cr.set_source_rgba(46 / 255.0, 194 / 255.0, 126 / 255.0, 1.0)
                    cr.fill()
                    # Draw crisp ✔ icon
                    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0)
                    cr.set_line_width(2.4)
                    cr.set_line_cap(cairo.LINE_CAP_ROUND)
                    cr.set_line_join(cairo.LINE_JOIN_ROUND)
                    cr.move_to(tx - 5.5, ty + 0.5)
                    cr.line_to(tx - 1.5, ty + 4.8)
                    cr.line_to(tx + 6.0, ty - 4.5)
                    cr.stroke()
        else:
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.28)
            cr.paint()
            cr.set_operator(cairo.OPERATOR_OVER)

            # Show instruction pill on the primary monitor (or first monitor)
            pm = next((m for m in mons if m.get('primary')), mons[0] if mons else {'x': 0, 'y': 0, 'w': sw, 'h': sh})
            banner_text = "Clique e arraste para selecionar a região  •  Esc ou botão direito para cancelar"
            cr.select_font_face("Sans", cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_BOLD)
            cr.set_font_size(13.0)
            ext = cr.text_extents(banner_text)
            bw = ext.width + 36.0
            bh = 36.0
            bx = pm['x'] + (pm['w'] - bw) / 2.0
            by = pm['y'] + 44.0
            rounded_rect(cr, bx, by, bw, bh, 18.0)
            cr.set_source_rgba(20 / 255.0, 20 / 255.0, 26 / 255.0, 0.88)
            cr.fill_preserve()
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.2)
            cr.set_line_width(1.0)
            cr.stroke()
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.95)
            cr.move_to(bx + 18.0, by + bh / 2.0 + ext.height / 2.0 - 1.0)
            cr.show_text(banner_text)

        return False

    win.connect('map-event', on_map)
    win.connect('draw', on_draw)
    win.connect('key-press-event', on_key_press)
    win.connect('button-press-event', on_button_press)
    win.connect('motion-notify-event', on_motion)
    win.connect('button-release-event', on_button_release)

    win.show_all()
    GLib.timeout_add(30, lambda: (try_grab(), False)[1])

    if demo_rect is not None:
        # Used only for automated visual verification: auto-confirm after 350ms
        GLib.timeout_add(350, lambda: (finish({'rect': list(demo_rect), 'overlay_size': [float(sw), float(sh)]}), False)[1])

    Gtk.main()
    print(json.dumps(state['result']))
    return 0


if __name__ == '__main__':
    if len(sys.argv) >= 3 and sys.argv[1] == 'window':
        sys.exit(capture_window(sys.argv[2]))
    if len(sys.argv) >= 2 and sys.argv[1] == 'monitors':
        sys.exit(monitors())
    if len(sys.argv) >= 2 and sys.argv[1] == 'select':
        demo = [float(x) for x in sys.argv[2].split(',')] if len(sys.argv) >= 3 else None
        sys.exit(select_region(demo))
    sys.stderr.write('usage: window OUT | monitors | select\n')
    sys.exit(1)
