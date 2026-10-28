// minerva-wm — the Minerva native X11 window manager and compositor.
//
// v0.11.0: this is the real thing. No browser, no HTML shell underneath:
// minerva-wm talks to the X server directly over libX11 (FFI, zero crates),
// manages windows (EWMH + ICCCM), draws its own Aluminium-style decorations,
// panel, taskbar, launcher and system menus with core X drawing, and runs a
// real redirect/damage/render compositor (Composite + Damage + RENDER) the
// same way xcompmgr/compton-class compositors do.
//
// Written against Xlib headers as shipped in Debian 13 (libx11-dev).
// Build deps: libx11-dev libxext-dev libxrender-dev libxcomposite-dev
//             libxdamage-dev libxfixes-dev
//
// Runtime behaviour:
//   minerva-wm                run the WM (exits 2 if another WM owns the screen)
//   minerva-wm --smoke        self-test on a bare X display (used by CI/Xvfb)
//   minerva-wm --no-composite disable the compositing pass
//   minerva-wm --version      print version and exit
//
// Failures are never silent: every refused startup prints why to stderr (the
// session script captures it in session.log) and exits non-zero.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int, c_long, c_short, c_uchar, c_uint, c_ulong, c_ushort, c_void};
use std::path::PathBuf;
use std::process::Command;
use std::ptr;

pub const VERSION: &str = "0.11.0";

// ---------------------------------------------------------------- primitives
pub type XID = c_ulong;
pub type Window = c_ulong;
pub type Drawable = c_ulong;
pub type Pixmap = c_ulong;
pub type Atom = c_ulong;
pub type Time = c_ulong;
pub type Colormap = c_ulong;
pub type Cursor = c_ulong;
pub type Font = c_ulong;
pub type Picture = c_ulong;
pub type Damage = c_ulong;
pub type KeyCode = c_uchar;
pub type KeySym = c_ulong;
pub type Bool = c_int;
pub type Status = c_int;

#[repr(C)]
pub struct Display {
    _u: [u8; 0],
}
#[repr(C)]
pub struct Visual {
    _u: [u8; 0],
}
#[repr(C)]
pub struct XGC {
    _u: [u8; 0],
}
#[repr(C)]
pub struct XFontStruct {
    _u: [u8; 0],
}
#[repr(C)]
pub struct Screen {
    _u: [u8; 0],
}
#[repr(C)]
pub struct XRenderPictFormat {
    _u: [u8; 0],
}
pub type GC = *mut XGC;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XColor {
    pub pixel: c_ulong,
    pub red: c_ushort,
    pub green: c_ushort,
    pub blue: c_ushort,
    pub flags: c_char,
    pub pad: c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XCharStruct {
    pub lbearing: c_short,
    pub rbearing: c_short,
    pub width: c_short,
    pub ascent: c_short,
    pub descent: c_short,
    pub attributes: c_ushort,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XRectangle {
    pub x: c_short,
    pub y: c_short,
    pub width: c_short,
    pub height: c_short,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XRenderColor {
    pub red: c_ushort,
    pub green: c_ushort,
    pub blue: c_ushort,
    pub alpha: c_ushort,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XPoint {
    pub x: c_short,
    pub y: c_short,
}

#[repr(C)]
pub struct XWindowChanges {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border_width: c_int,
    pub sibling: Window,
    pub stack_mode: c_int,
}

#[repr(C)]
pub struct XSetWindowAttributes {
    pub background_pixmap: Pixmap,
    pub background_pixel: c_ulong,
    pub border_pixmap: Pixmap,
    pub border_pixel: c_ulong,
    pub bit_gravity: c_int,
    pub win_gravity: c_int,
    pub backing_store: c_int,
    pub backing_planes: c_ulong,
    pub backing_pixel: c_ulong,
    pub save_under: Bool,
    pub event_mask: c_long,
    pub do_not_propagate_mask: c_long,
    pub override_redirect: Bool,
    pub colormap: Colormap,
    pub cursor: Cursor,
}

#[repr(C)]
pub struct XWindowAttributes {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border_width: c_int,
    pub depth: c_int,
    pub visual: *mut Visual,
    pub root: Window,
    pub class: c_int,
    pub bit_gravity: c_int,
    pub win_gravity: c_int,
    pub backing_store: c_int,
    pub backing_planes: c_ulong,
    pub backing_pixel: c_ulong,
    pub save_under: Bool,
    pub colormap: Colormap,
    pub map_installed: Bool,
    pub map_state: c_int,
    pub all_event_masks: c_long,
    pub your_event_mask: c_long,
    pub do_not_propagate_mask: c_long,
    pub override_redirect: Bool,
    pub screen: *mut Screen,
}

#[repr(C)]
pub struct XGCValues {
    pub function: c_int,
    pub plane_mask: c_ulong,
    pub foreground: c_ulong,
    pub background: c_ulong,
    pub line_width: c_int,
    pub line_style: c_int,
    pub cap_style: c_int,
    pub join_style: c_int,
    pub fill_style: c_int,
    pub fill_rule: c_int,
    pub arc_mode: c_int,
    pub tile: Pixmap,
    pub stipple: Pixmap,
    pub ts_x_origin: c_int,
    pub ts_y_origin: c_int,
    pub font: Font,
    pub subwindow_mode: c_int,
    pub graphics_exposures: Bool,
    pub clip_x_origin: c_int,
    pub clip_y_origin: c_int,
    pub clip_mask: Pixmap,
    pub dash_offset: c_int,
    pub dashes: c_char,
}

#[repr(C)]
pub struct PollFd {
    pub fd: c_int,
    pub events: c_short,
    pub revents: c_short,
}

// ------------------------------------------------------------------- events
#[repr(C)]
#[derive(Copy, Clone)]
pub struct XAnyEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XKeyEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: c_uint,
    pub keycode: c_uint,
    pub same_screen: Bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XButtonEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: c_uint,
    pub button: c_uint,
    pub same_screen: Bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XMotionEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: c_uint,
    pub is_hint: c_char,
    pub same_screen: Bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XCrossingEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub mode: c_int,
    pub detail: c_int,
    pub state: c_uint,
    pub same_screen: Bool,
    pub focus: Bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XFocusChangeEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub mode: c_int,
    pub detail: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XExposeEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub count: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XMapEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub event: Window,
    pub window: Window,
    pub override_redirect: Bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XUnmapEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub event: Window,
    pub window: Window,
    pub from_configure: Bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XDestroyWindowEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub event: Window,
    pub window: Window,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XReparentEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub event: Window,
    pub window: Window,
    pub parent: Window,
    pub x: c_int,
    pub y: c_int,
    pub override_redirect: Bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XConfigureEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub event: Window,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border_width: c_int,
    pub above: Window,
    pub override_redirect: Bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XConfigureRequestEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub parent: Window,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border_width: c_int,
    pub above: Window,
    pub detail: c_int,
    pub value_mask: c_ulong,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XPropertyEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub atom: Atom,
    pub time: Time,
    pub state: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ClientMessageData {
    pub l: [c_long; 5],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XClientMessageEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub message_type: Atom,
    pub format: c_int,
    pub data: ClientMessageData,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XDamageNotifyEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub drawable: Drawable,
    pub level: c_uint,
    pub state: c_int,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub geometry_x: c_int,
    pub geometry_y: c_int,
    pub geometry_width: c_uint,
    pub geometry_height: c_uint,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XErrorEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub resourceid: XID,
    pub time: Time,
    pub error_code: c_int,
    pub request_code: c_int,
    pub minor_code: c_int,
}

#[repr(C)]
pub union XEvent {
    pub any: XAnyEvent,
    pub key: XKeyEvent,
    pub button: XButtonEvent,
    pub motion: XMotionEvent,
    pub crossing: XCrossingEvent,
    pub focus: XFocusChangeEvent,
    pub expose: XExposeEvent,
    pub map: XMapEvent,
    pub unmap: XUnmapEvent,
    pub destroy: XDestroyWindowEvent,
    pub reparent: XReparentEvent,
    pub configure: XConfigureEvent,
    pub configurereq: XConfigureRequestEvent,
    pub property: XPropertyEvent,
    pub client: XClientMessageEvent,
    pub damage: XDamageNotifyEvent,
    pub mapping: XMappingEventFfi,
    pub pad: [c_long; 24],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct XMappingEventFfi {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub first_keycode: c_int,
    pub request: c_int,
    pub count: c_int,
}

// ------------------------------------------------------------------- xproto
// event types
pub const KeyPress: c_int = 2;
pub const KeyRelease: c_int = 3;
pub const ButtonPress: c_int = 4;
pub const ButtonRelease: c_int = 5;
pub const MotionNotify: c_int = 6;
pub const EnterNotify: c_int = 7;
pub const LeaveNotify: c_int = 8;
pub const FocusIn: c_int = 9;
pub const FocusOut: c_int = 10;
pub const Expose: c_int = 12;
pub const DestroyNotify: c_int = 17;
pub const UnmapNotify: c_int = 18;
pub const MapNotify: c_int = 19;
pub const MapRequest: c_int = 20;
pub const ReparentNotify: c_int = 21;
pub const ConfigureNotify: c_int = 22;
pub const ConfigureRequest: c_int = 23;
pub const PropertyNotify: c_int = 28;
pub const SelectionClear: c_int = 29;
pub const ClientMessage: c_int = 33;
pub const MappingNotify: c_int = 34;

// event masks
pub const KeyPressMask: c_long = 1 << 0;
pub const KeyReleaseMask: c_long = 1 << 1;
pub const ButtonPressMask: c_long = 1 << 2;
pub const ButtonReleaseMask: c_long = 1 << 3;
pub const EnterWindowMask: c_long = 1 << 4;
pub const PointerMotionMask: c_long = 1 << 6;
pub const ExposureMask: c_long = 1 << 15;
pub const StructureNotifyMask: c_long = 1 << 17;
pub const SubstructureNotifyMask: c_long = 1 << 19;
pub const SubstructureRedirectMask: c_long = 1 << 20;
pub const PropertyChangeMask: c_long = 1 << 22;

// misc constants
pub const CurrentTime: Time = 0;
pub const None_: Window = 0;
pub const GrabModeAsync: c_int = 1;
pub const GrabModeSync: c_int = 0;
pub const RevertToNone: c_int = 0;
pub const RevertToPointerRoot: c_int = 2;
pub const InputOutput: c_uint = 1;
pub const InputOnly: c_uint = 2;
pub const IsViewable: c_int = 2;
pub const WithdrawnState: c_long = 0;
pub const NormalState: c_long = 1;
pub const IconicState: c_long = 3;
pub const PropModeReplace: c_int = 0;
pub const AnyPropertyType: Atom = 0;
pub const AnyKey: c_uint = 0;
pub const AnyButton: c_uint = 0;
pub const AnyModifier: c_uint = 1 << 15;
pub const Button1: c_uint = 1;
pub const Button2: c_uint = 2;
pub const Button3: c_uint = 3;
pub const ShiftMask: c_uint = 1 << 0;
pub const ControlMask: c_uint = 1 << 2;
pub const Mod1Mask: c_uint = 1 << 3; // Alt
pub const Mod4Mask: c_uint = 1 << 6; // Super
pub const CWX: c_ulong = 1 << 0;
pub const CWY: c_ulong = 1 << 1;
pub const CWWidth: c_ulong = 1 << 2;
pub const CWHeight: c_ulong = 1 << 3;
pub const CWBorderWidth: c_ulong = 1 << 4;
pub const CWSibling: c_ulong = 1 << 5;
pub const CWStackMode: c_ulong = 1 << 6;
pub const Above: c_int = 0;
pub const Below: c_int = 1;
pub const CWBackPixel: c_ulong = 1 << 1;
pub const CWBorderPixel: c_ulong = 1 << 3;
pub const CWEventMask: c_ulong = 1 << 11;
pub const CWOverrideRedirect: c_ulong = 1 << 9;
pub const CWColormap: c_ulong = 1 << 13;
pub const CWCursor: c_ulong = 1 << 14;
pub const XC_left_ptr: c_uint = 68;
pub const XC_fleur: c_uint = 52;
pub const XC_hand2: c_uint = 60;
pub const ShapeBounding: c_int = 0;
pub const ShapeSet: c_int = 0;
pub const Unsorted: c_int = 0;
pub const CompositeRedirectAutomatic: c_int = 0;
pub const CompositeRedirectManual: c_int = 1;
pub const DamageReportRawRectangles: c_int = 0;
pub const DamageReportNonEmpty: c_int = 3;
pub const PictOpSrc: c_int = 1;
pub const PictOpOver: c_int = 3;
pub const GCForeground: c_ulong = 1 << 2;
pub const GCFont: c_ulong = 1 << 14;
pub const Success_: c_int = 0;
pub const BadAccess_: c_int = 10;

// keysyms (X11/keysymdef.h)
pub const XK_Escape: KeySym = 0xff1b;
pub const XK_Return: KeySym = 0xff0d;
pub const XK_Tab: KeySym = 0xff09;
pub const XK_Left: KeySym = 0xff51;
pub const XK_Up: KeySym = 0xff52;
pub const XK_Right: KeySym = 0xff53;
pub const XK_Down: KeySym = 0xff54;
pub const XK_F4: KeySym = 0xffc1;
pub const XK_Super_L: KeySym = 0xffeb;
pub const XK_Super_R: KeySym = 0xffec;

// --------------------------------------------------------------- libX11 FFI
#[link(name = "X11")]
extern "C" {
    pub fn XOpenDisplay(name: *const c_char) -> *mut Display;
    pub fn XCloseDisplay(d: *mut Display) -> c_int;
    pub fn XDefaultScreen(d: *mut Display) -> c_int;
    pub fn XRootWindow(d: *mut Display, s: c_int) -> Window;
    pub fn XDisplayWidth(d: *mut Display, s: c_int) -> c_int;
    pub fn XDisplayHeight(d: *mut Display, s: c_int) -> c_int;
    pub fn XBlackPixel(d: *mut Display, s: c_int) -> c_ulong;
    pub fn XWhitePixel(d: *mut Display, s: c_int) -> c_ulong;
    pub fn XDefaultGC(d: *mut Display, s: c_int) -> GC;
    pub fn XDefaultDepth(d: *mut Display, s: c_int) -> c_int;
    pub fn XDefaultVisual(d: *mut Display, s: c_int) -> *mut Visual;
    pub fn XDefaultColormap(d: *mut Display, s: c_int) -> Colormap;
    pub fn XCreateGC(d: *mut Display, dr: Drawable, mask: c_ulong, values: *mut XGCValues) -> GC;
    pub fn XSetForeground(d: *mut Display, gc: GC, pixel: c_ulong) -> c_int;
    pub fn XSetBackground(d: *mut Display, gc: GC, pixel: c_ulong) -> c_int;
    pub fn XSetFont(d: *mut Display, gc: GC, font: Font) -> c_int;
    pub fn XFillRectangle(
        d: *mut Display,
        dr: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        w: c_uint,
        h: c_uint,
    ) -> c_int;
    pub fn XDrawRectangle(
        d: *mut Display,
        dr: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        w: c_uint,
        h: c_uint,
    ) -> c_int;
    pub fn XDrawLine(
        d: *mut Display,
        dr: Drawable,
        gc: GC,
        x1: c_int,
        y1: c_int,
        x2: c_int,
        y2: c_int,
    ) -> c_int;
    pub fn XDrawString(
        d: *mut Display,
        dr: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        s: *const c_char,
        len: c_int,
    ) -> c_int;
    pub fn XClearWindow(d: *mut Display, w: Window) -> c_int;
    pub fn XCreateSimpleWindow(
        d: *mut Display,
        parent: Window,
        x: c_int,
        y: c_int,
        w: c_uint,
        h: c_uint,
        border_width: c_uint,
        border: c_ulong,
        background: c_ulong,
    ) -> Window;
    pub fn XCreateWindow(
        d: *mut Display,
        parent: Window,
        x: c_int,
        y: c_int,
        w: c_uint,
        h: c_uint,
        border_width: c_uint,
        depth: c_int,
        class: c_uint,
        visual: *mut Visual,
        valuemask: c_ulong,
        attrs: *mut XSetWindowAttributes,
    ) -> Window;
    pub fn XDestroyWindow(d: *mut Display, w: Window) -> c_int;
    pub fn XMapWindow(d: *mut Display, w: Window) -> c_int;
    pub fn XMapRaised(d: *mut Display, w: Window) -> c_int;
    pub fn XUnmapWindow(d: *mut Display, w: Window) -> c_int;
    pub fn XReparentWindow(d: *mut Display, w: Window, parent: Window, x: c_int, y: c_int) -> c_int;
    pub fn XMoveResizeWindow(
        d: *mut Display,
        w: Window,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    pub fn XMoveWindow(d: *mut Display, w: Window, x: c_int, y: c_int) -> c_int;
    pub fn XResizeWindow(d: *mut Display, w: Window, w_: c_uint, h: c_uint) -> c_int;
    pub fn XRaiseWindow(d: *mut Display, w: Window) -> c_int;
    pub fn XLowerWindow(d: *mut Display, w: Window) -> c_int;
    pub fn XConfigureWindow(
        d: *mut Display,
        w: Window,
        mask: c_ulong,
        values: *mut XWindowChanges,
    ) -> c_int;
    pub fn XSetWindowBackground(d: *mut Display, w: Window, pixel: c_ulong) -> c_int;
    pub fn XSetWindowBorder(d: *mut Display, w: Window, pixel: c_ulong) -> c_int;
    pub fn XSetWindowBorderWidth(d: *mut Display, w: Window, width: c_uint) -> c_int;
    pub fn XSelectInput(d: *mut Display, w: Window, mask: c_long) -> c_int;
    pub fn XNextEvent(d: *mut Display, ev: *mut XEvent) -> c_int;
    pub fn XPending(d: *mut Display) -> c_int;
    pub fn XMaskEvent(d: *mut Display, mask: c_long, ev: *mut XEvent) -> c_int;
    pub fn XSendEvent(
        d: *mut Display,
        w: Window,
        propagate: Bool,
        mask: c_long,
        ev: *mut XEvent,
    ) -> Status;
    pub fn XSync(d: *mut Display, discard: Bool) -> c_int;
    pub fn XFlush(d: *mut Display) -> c_int;
    pub fn XInternAtom(d: *mut Display, name: *const c_char, only_if_exists: Bool) -> Atom;
    pub fn XGetAtomName(d: *mut Display, atom: Atom) -> *mut c_char;
    pub fn XChangeProperty(
        d: *mut Display,
        w: Window,
        property: Atom,
        type_: Atom,
        format: c_int,
        mode: c_int,
        data: *const c_uchar,
        nelements: c_int,
    ) -> c_int;
    pub fn XDeleteProperty(d: *mut Display, w: Window, property: Atom) -> c_int;
    pub fn XGetWindowProperty(
        d: *mut Display,
        w: Window,
        property: Atom,
        long_offset: c_long,
        long_length: c_long,
        delete: Bool,
        req_type: Atom,
        actual_type: *mut Atom,
        actual_format: *mut c_int,
        nitems: *mut c_ulong,
        bytes_after: *mut c_ulong,
        data: *mut *mut c_uchar,
    ) -> Status;
    pub fn XFree(p: *mut c_void) -> c_int;
    pub fn XSetInputFocus(d: *mut Display, w: Window, revert_to: c_int, time: Time) -> c_int;
    pub fn XGetInputFocus(d: *mut Display, focus: *mut Window, revert: *mut c_int) -> c_int;
    pub fn XGrabPointer(
        d: *mut Display,
        grab_window: Window,
        owner_events: Bool,
        event_mask: c_uint,
        pointer_mode: c_int,
        keyboard_mode: c_int,
        confine_to: Window,
        cursor: Cursor,
        time: Time,
    ) -> c_int;
    pub fn XUngrabPointer(d: *mut Display, time: Time) -> c_int;
    pub fn XGrabKeyboard(
        d: *mut Display,
        grab_window: Window,
        owner_events: Bool,
        pointer_mode: c_int,
        keyboard_mode: c_int,
        time: Time,
    ) -> c_int;
    pub fn XUngrabKeyboard(d: *mut Display, time: Time) -> c_int;
    pub fn XGrabKey(
        d: *mut Display,
        keycode: c_int,
        modifiers: c_uint,
        grab_window: Window,
        owner_events: Bool,
        pointer_mode: c_int,
        keyboard_mode: c_int,
    ) -> c_int;
    pub fn XKeysymToKeycode(d: *mut Display, keysym: KeySym) -> KeyCode;
    pub fn XSetErrorHandler(handler: Option<extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>)
        -> Option<extern "C" fn(*mut Display, *mut XErrorEvent) -> c_int>;
    pub fn XGetErrorText(d: *mut Display, code: c_int, buf: *mut c_char, len: c_int) -> c_int;
    pub fn XQueryTree(
        d: *mut Display,
        w: Window,
        root_ret: *mut Window,
        parent_ret: *mut Window,
        children_ret: *mut *mut Window,
        nchildren_ret: *mut c_uint,
    ) -> Status;
    pub fn XGetGeometry(
        d: *mut Display,
        dr: Drawable,
        root_ret: *mut Window,
        x_ret: *mut c_int,
        y_ret: *mut c_int,
        width_ret: *mut c_uint,
        height_ret: *mut c_uint,
        border_width_ret: *mut c_uint,
        depth_ret: *mut c_uint,
    ) -> Status;
    pub fn XGetWindowAttributes(d: *mut Display, w: Window, attrs: *mut XWindowAttributes)
        -> Status;
    pub fn XTranslateCoordinates(
        d: *mut Display,
        src_w: Window,
        dest_w: Window,
        src_x: c_int,
        src_y: c_int,
        dest_x_ret: *mut c_int,
        dest_y_ret: *mut c_int,
        child_ret: *mut Window,
    ) -> Bool;
    pub fn XGetSelectionOwner(d: *mut Display, selection: Atom) -> Window;
    pub fn XSetSelectionOwner(d: *mut Display, selection: Atom, owner: Window, time: Time)
        -> c_int;
    pub fn XWithdrawWindow(d: *mut Display, w: Window, screen: c_int) -> Status;
    pub fn XIconifyWindow(d: *mut Display, w: Window, screen: c_int) -> Status;
    pub fn XKillClient(d: *mut Display, resource: XID) -> c_int;
    pub fn XLoadQueryFont(d: *mut Display, name: *const c_char) -> *mut XFontStruct;
    pub fn XFreeFont(d: *mut Display, font: *mut XFontStruct) -> c_int;
    pub fn XTextExtents(
        font: *mut XFontStruct,
        s: *const c_char,
        nchars: c_int,
        direction_ret: *mut c_int,
        ascent_ret: *mut c_int,
        descent_ret: *mut c_int,
        overall_ret: *mut XCharStruct,
    ) -> c_int;
    pub fn XParseColor(
        d: *mut Display,
        cmap: Colormap,
        spec: *const c_char,
        exact_ret: *mut XColor,
    ) -> Status;
    pub fn XAllocColor(d: *mut Display, cmap: Colormap, color: *mut XColor) -> Status;
    pub fn XCreateFontCursor(d: *mut Display, shape: c_uint) -> Cursor;
    pub fn XDefineCursor(d: *mut Display, w: Window, cursor: Cursor) -> c_int;
    pub fn XFreeCursor(d: *mut Display, cursor: Cursor) -> c_int;
    pub fn XConnectionNumber(d: *mut Display) -> c_int;
    pub fn XStoreName(d: *mut Display, w: Window, name: *const c_char) -> c_int;
    pub fn XChangeWindowAttributes(
        d: *mut Display,
        w: Window,
        mask: c_ulong,
        attrs: *mut XSetWindowAttributes,
    ) -> c_int;
    pub fn XFreePixmap(d: *mut Display, p: Pixmap) -> c_int;
    pub fn XFreeGC(d: *mut Display, gc: GC) -> c_int;
    pub fn XDisplayPlanes(d: *mut Display, s: c_int) -> c_int;
}

#[link(name = "Xext")]
extern "C" {
    pub fn XShapeCombineRectangles(
        d: *mut Display,
        dest: Window,
        dest_kind: c_int,
        x_off: c_int,
        y_off: c_int,
        rectangles: *const XRectangle,
        nrects: c_int,
        op: c_int,
        ordering: c_int,
    ) -> c_int;
}

#[link(name = "Xcomposite")]
extern "C" {
    pub fn XCompositeQueryExtension(
        d: *mut Display,
        event_base: *mut c_int,
        error_base: *mut c_int,
    ) -> Bool;
    pub fn XCompositeQueryVersion(
        d: *mut Display,
        major: *mut c_int,
        minor: *mut c_int,
    ) -> Status;
    pub fn XCompositeRedirectSubwindows(d: *mut Display, w: Window, update: c_int) -> c_int;
    pub fn XCompositeUnredirectSubwindows(d: *mut Display, w: Window, update: c_int) -> c_int;
    pub fn XCompositeGetOverlayWindow(d: *mut Display, w: Window) -> Window;
    pub fn XCompositeReleaseOverlayWindow(d: *mut Display, w: Window) -> c_int;
    pub fn XCompositeNameWindowPixmap(d: *mut Display, w: Window) -> Pixmap;
}

#[link(name = "Xdamage")]
extern "C" {
    pub fn XDamageQueryExtension(
        d: *mut Display,
        event_base: *mut c_int,
        error_base: *mut c_int,
    ) -> Bool;
    pub fn XDamageQueryVersion(d: *mut Display, major: *mut c_int, minor: *mut c_int) -> Status;
    pub fn XDamageCreate(d: *mut Display, drawable: Drawable, level: c_int) -> Damage;
    pub fn XDamageDestroy(d: *mut Display, damage: Damage) -> c_int;
    pub fn XDamageSubtract(
        d: *mut Display,
        damage: Damage,
        repair: XID,
        parts: XID,
    ) -> Bool;
}

#[link(name = "Xfixes")]
extern "C" {
    pub fn XFixesQueryExtension(
        d: *mut Display,
        event_base: *mut c_int,
        error_base: *mut c_int,
    ) -> Bool;
    pub fn XFixesQueryVersion(d: *mut Display, major: *mut c_int, minor: *mut c_int) -> Status;
}

#[link(name = "Xrender")]
extern "C" {
    pub fn XRenderQueryExtension(
        d: *mut Display,
        event_base: *mut c_int,
        error_base: *mut c_int,
    ) -> Bool;
    pub fn XRenderQueryVersion(d: *mut Display, major: *mut c_int, minor: *mut c_int) -> Status;
    pub fn XRenderFindVisualFormat(d: *mut Display, visual: *mut Visual) -> *mut XRenderPictFormat;
    pub fn XRenderCreatePicture(
        d: *mut Display,
        drawable: Drawable,
        format: *mut XRenderPictFormat,
        valuemask: c_ulong,
        attrs: *const c_void,
    ) -> Picture;
    pub fn XRenderSetPictureClipRectangles(
        d: *mut Display,
        picture: Picture,
        xorigin: c_int,
        yorigin: c_int,
        rects: *const XRectangle,
        n: c_int,
    ) -> c_int;
    pub fn XRenderComposite(
        d: *mut Display,
        op: c_int,
        src: Picture,
        mask: Picture,
        dst: Picture,
        src_x: c_int,
        src_y: c_int,
        mask_x: c_int,
        mask_y: c_int,
        dst_x: c_int,
        dst_y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    pub fn XRenderFillRectangle(
        d: *mut Display,
        op: c_int,
        dst: Picture,
        color: *const XRenderColor,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    pub fn XRenderFreePicture(d: *mut Display, picture: Picture) -> c_int;
    pub fn XRenderCreateSolidFill(d: *mut Display, color: *const XRenderColor) -> Picture;
}

#[link(name = "c")]
extern "C" {
    pub fn poll(fds: *mut PollFd, nfds: c_ulong, timeout: c_int) -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
}
pub const POLLIN: c_short = 1;
pub const WNOHANG: c_int = 1;

// ============================================================================
// small utilities
// ============================================================================

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap_or_else(|_| CString::new("x").unwrap())
}

/// UTF-8 -> Latin-1 for core-font XDrawString (best effort, '?' fallback).
fn latin1(s: &str) -> Vec<c_char> {
    let mut out: Vec<c_char> = Vec::with_capacity(s.len() + 1);
    for ch in s.chars() {
        let c = ch as u32;
        if c == 0 {
            break;
        }
        out.push(if c < 256 { c as c_char } else { b'?' as c_char });
    }
    out.push(0);
    out
}

fn sh_out(cmd: &str) -> String {
    match Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

fn spawn_app(cmd: &str) {
    // detached-ish: new process group, all std handles null; the poll loop
    // reaps finished children with waitpid(WNOHANG) so nothing zombifies.
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let _ = unsafe {
        Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
    };
}

fn reap_children() {
    unsafe {
        loop {
            let mut st: c_int = 0;
            if waitpid(-1, &mut st, WNOHANG) <= 0 {
                break;
            }
        }
    }
}

static mut X_ERROR_SEEN: c_int = 0;

extern "C" fn wm_error_handler(_d: *mut Display, e: *mut XErrorEvent) -> c_int {
    unsafe {
        X_ERROR_SEEN = (*e).error_code as c_int;
    }
    0 // never abort — a WM that dies on BadWindow kills the session
}

fn error_seen() -> c_int {
    unsafe { X_ERROR_SEEN }
}
fn clear_error() {
    unsafe { X_ERROR_SEEN = 0 }
}

// ============================================================================
// atoms
// ============================================================================

#[derive(Copy, Clone)]
pub struct Atoms {
    pub wm_protocols: Atom,
    pub wm_delete_window: Atom,
    pub wm_take_focus: Atom,
    pub wm_state: Atom,
    pub wm_change_state: Atom,
    pub wm_name: Atom,
    pub wm_hints: Atom,
    pub wm_normal_hints: Atom,
    pub wm_transient_for: Atom,
    pub wm_class: Atom,
    pub net_supported: Atom,
    pub net_supporting_wm_check: Atom,
    pub net_client_list: Atom,
    pub net_client_list_stacking: Atom,
    pub net_active_window: Atom,
    pub net_close_window: Atom,
    pub net_wm_name: Atom,
    pub net_wm_state: Atom,
    pub net_wm_state_fullscreen: Atom,
    pub net_wm_state_maximized_vert: Atom,
    pub net_wm_state_maximized_horz: Atom,
    pub net_wm_state_above: Atom,
    pub net_wm_state_below: Atom,
    pub net_wm_state_sticky: Atom,
    pub net_wm_state_hidden: Atom,
    pub net_wm_state_demands_attention: Atom,
    pub net_wm_desktop: Atom,
    pub net_number_of_desktops: Atom,
    pub net_current_desktop: Atom,
    pub net_desktop_names: Atom,
    pub net_showing_desktop: Atom,
    pub net_workarea: Atom,
    pub net_frame_extents: Atom,
    pub net_wm_window_type: Atom,
    pub net_wm_window_type_dialog: Atom,
    pub net_wm_window_type_dock: Atom,
    pub net_wm_window_type_normal: Atom,
    pub net_wm_moveresize: Atom,
    pub net_request_frame_extents: Atom,
    pub utf8_string: Atom,
    pub xa_atom: Atom,
    pub xa_cardinal: Atom,
    pub xa_string: Atom,
    pub xa_window: Atom,
    pub minerva_supporting: Atom,
    pub minerva_wm_cmd: Atom,
}

fn intern(d: *mut Display, name: &str) -> Atom {
    unsafe { XInternAtom(d, cstr(name).as_ptr(), 0) }
}

fn init_atoms(d: *mut Display) -> Atoms {
    Atoms {
        wm_protocols: intern(d, "WM_PROTOCOLS"),
        wm_delete_window: intern(d, "WM_DELETE_WINDOW"),
        wm_take_focus: intern(d, "WM_TAKE_FOCUS"),
        wm_state: intern(d, "WM_STATE"),
        wm_change_state: intern(d, "WM_CHANGE_STATE"),
        wm_name: intern(d, "WM_NAME"),
        wm_hints: intern(d, "WM_HINTS"),
        wm_normal_hints: intern(d, "WM_NORMAL_HINTS"),
        wm_transient_for: intern(d, "WM_TRANSIENT_FOR"),
        wm_class: intern(d, "WM_CLASS"),
        net_supported: intern(d, "_NET_SUPPORTED"),
        net_supporting_wm_check: intern(d, "_NET_SUPPORTING_WM_CHECK"),
        net_client_list: intern(d, "_NET_CLIENT_LIST"),
        net_client_list_stacking: intern(d, "_NET_CLIENT_LIST_STACKING"),
        net_active_window: intern(d, "_NET_ACTIVE_WINDOW"),
        net_close_window: intern(d, "_NET_CLOSE_WINDOW"),
        net_wm_name: intern(d, "_NET_WM_NAME"),
        net_wm_state: intern(d, "_NET_WM_STATE"),
        net_wm_state_fullscreen: intern(d, "_NET_WM_STATE_FULLSCREEN"),
        net_wm_state_maximized_vert: intern(d, "_NET_WM_STATE_MAXIMIZED_VERT"),
        net_wm_state_maximized_horz: intern(d, "_NET_WM_STATE_MAXIMIZED_HORZ"),
        net_wm_state_above: intern(d, "_NET_WM_STATE_ABOVE"),
        net_wm_state_below: intern(d, "_NET_WM_STATE_BELOW"),
        net_wm_state_sticky: intern(d, "_NET_WM_STATE_STICKY"),
        net_wm_state_hidden: intern(d, "_NET_WM_STATE_HIDDEN"),
        net_wm_state_demands_attention: intern(d, "_NET_WM_STATE_DEMANDS_ATTENTION"),
        net_wm_desktop: intern(d, "_NET_WM_DESKTOP"),
        net_number_of_desktops: intern(d, "_NET_NUMBER_OF_DESKTOPS"),
        net_current_desktop: intern(d, "_NET_CURRENT_DESKTOP"),
        net_desktop_names: intern(d, "_NET_DESKTOP_NAMES"),
        net_showing_desktop: intern(d, "_NET_SHOWING_DESKTOP"),
        net_workarea: intern(d, "_NET_WORKAREA"),
        net_frame_extents: intern(d, "_NET_FRAME_EXTENTS"),
        net_wm_window_type: intern(d, "_NET_WM_WINDOW_TYPE"),
        net_wm_window_type_dialog: intern(d, "_NET_WM_WINDOW_TYPE_DIALOG"),
        net_wm_window_type_dock: intern(d, "_NET_WM_WINDOW_TYPE_DOCK"),
        net_wm_window_type_normal: intern(d, "_NET_WM_WINDOW_TYPE_NORMAL"),
        net_wm_moveresize: intern(d, "_NET_WM_MOVERESIZE"),
        net_request_frame_extents: intern(d, "_NET_REQUEST_FRAME_EXTENTS"),
        utf8_string: intern(d, "UTF8_STRING"),
        xa_atom: 4,
        xa_cardinal: 6,
        xa_string: 31,
        xa_window: 33,
        minerva_supporting: intern(d, "_MINERVA_SUPPORTING"),
        minerva_wm_cmd: intern(d, "_MINERVA_WM_CMD"),
    }
}

// property helpers -----------------------------------------------------------

fn prop_set_longs(d: *mut Display, w: Window, atom: Atom, vals: &[c_long]) {
    unsafe {
        XChangeProperty(
            d,
            w,
            atom,
            XA_CARDINAL_LOCAL(),
            32,
            PropModeReplace,
            vals.as_ptr() as *const c_uchar,
            vals.len() as c_int,
        );
    }
}
// _NET atoms of type CARDINAL/WINDOW are numeric 32-bit properties; XA local
// constants avoid an extra intern call in hot paths.
fn XA_CARDINAL_LOCAL() -> Atom {
    6
}
fn XA_WINDOW_LOCAL() -> Atom {
    33
}
fn XA_ATOM_LOCAL() -> Atom {
    4
}

fn prop_set_atoms(d: *mut Display, w: Window, atom: Atom, vals: &[Atom]) {
    unsafe {
        XChangeProperty(
            d,
            w,
            atom,
            XA_ATOM_LOCAL(),
            32,
            PropModeReplace,
            vals.as_ptr() as *const c_uchar,
            vals.len() as c_int,
        );
    }
}

fn prop_set_windows(d: *mut Display, w: Window, atom: Atom, vals: &[Window]) {
    unsafe {
        XChangeProperty(
            d,
            w,
            atom,
            XA_WINDOW_LOCAL(),
            32,
            PropModeReplace,
            vals.as_ptr() as *const c_uchar,
            vals.len() as c_int,
        );
    }
}

fn prop_set_str(d: *mut Display, w: Window, atom: Atom, utf8: bool, s: &str) {
    let bytes = s.as_bytes();
    unsafe {
        XChangeProperty(
            d,
            w,
            atom,
            if utf8 { intern(d, "UTF8_STRING") } else { 31 },
            8,
            PropModeReplace,
            bytes.as_ptr(),
            bytes.len() as c_int,
        );
    }
}

fn prop_get_longs(d: *mut Display, w: Window, atom: Atom) -> Vec<c_long> {
    unsafe {
        let mut actual_type: Atom = 0;
        let mut actual_format: c_int = 0;
        let mut nitems: c_ulong = 0;
        let mut bytes_after: c_ulong = 0;
        let mut data: *mut c_uchar = ptr::null_mut();
        let rc = XGetWindowProperty(
            d,
            w,
            atom,
            0,
            64,
            0,
            AnyPropertyType,
            &mut actual_type,
            &mut actual_format,
            &mut nitems,
            &mut bytes_after,
            &mut data,
        );
        let mut out = Vec::new();
        if rc == 0 && !data.is_null() && actual_format == 32 {
            let p = data as *const c_long;
            for i in 0..(nitems as usize) {
                out.push(*p.add(i));
            }
        }
        if !data.is_null() {
            XFree(data as *mut c_void);
        }
        out
    }
}

fn prop_get_string(d: *mut Display, w: Window, atom: Atom) -> Option<String> {
    unsafe {
        let mut actual_type: Atom = 0;
        let mut actual_format: c_int = 0;
        let mut nitems: c_ulong = 0;
        let mut bytes_after: c_ulong = 0;
        let mut data: *mut c_uchar = ptr::null_mut();
        let rc = XGetWindowProperty(
            d,
            w,
            atom,
            0,
            512,
            0,
            AnyPropertyType,
            &mut actual_type,
            &mut actual_format,
            &mut nitems,
            &mut bytes_after,
            &mut data,
        );
        let mut out: Option<String> = None;
        if rc == 0 && !data.is_null() && actual_format == 8 && nitems > 0 {
            let slice = std::slice::from_raw_parts(data, nitems as usize);
            out = Some(String::from_utf8_lossy(slice).trim_end_matches('\0').to_string());
        }
        if !data.is_null() {
            XFree(data as *mut c_void);
        }
        out
    }
}

fn client_msg(d: *mut Display, target: Window, type_: Atom, data: [c_long; 5]) {
    unsafe {
        let mut ev: XEvent = std::mem::zeroed();
        ev.client.type_ = ClientMessage;
        ev.client.window = target;
        ev.client.message_type = type_;
        ev.client.format = 32;
        ev.client.data.l = data;
        // mask 0 = delivered to the window's creator (WM_PROTOCOLS convention)
        XSendEvent(d, target, 0, 0, &mut ev);
        XFlush(d);
    }
}

// ============================================================================
// client + wm state
// ============================================================================

pub const DECOR_H: c_int = 28;
pub const FRAME_BW: c_uint = 1;
pub const PANEL_H: c_int = 34;
pub const DOCK_H: c_int = 44;
pub const NDESKS: u32 = 4;
pub const DESK_ALL: u32 = 0xFFFF_FFFF;
pub const RESIZE_EDGE: c_int = 8;

pub struct Client {
    pub win: Window,
    pub frame: Window,
    pub x: c_int,
    pub y: c_int,
    pub w: c_uint,
    pub h: c_uint,
    pub title: String,
    pub fullscreen: bool,
    pub maximized: bool,
    pub above: bool,
    pub sticky: bool,
    pub iconic: bool,
    pub desk: u32,
    pub transient: bool,
    pub input_hint: bool,
    pub mapped: bool, // frame currently mapped (viewable on this desk)
    pub min_w: c_uint,
    pub min_h: c_uint,
    pub max_w: c_uint,
    pub max_h: c_uint,
    // saved geometry for maximize/fullscreen restore
    pub saved: (c_int, c_int, c_uint, c_uint),
    // compositor resources
    pub damage: Damage,
    pub child_damage: Damage,
    pub pixmap: Pixmap,
    pub picture: Picture,
    pub child_pixmap: Pixmap,
    pub child_picture: Picture,
}

#[derive(Clone)]
pub enum Act {
    Nothing,
    Close,
    Launch(String),
    Desk(i64), // -1 prev, NDESKS+ next, else absolute
    ShowDesktop,
    Lock,
    Logout,
    Reboot,
    Poweroff,
    ToggleWifi,
    ToggleBt,
    VolUp,
    VolDown,
    VolMute,
    BriUp,
    BriDown,
    Record,
    OpenQs,
    OpenLauncher,
    OpenPower,
    OpenAbout,
    OpenTerminal,
    Focus(Window),
    Cycle(i32), // alt-tab direction
    Minimize(Window),
}

pub struct MenuItem {
    pub label: String,
    pub act: Act,
    pub right: String, // right-aligned hint text
}

pub struct Popup {
    pub win: Window,
    pub x: c_int,
    pub y: c_int,
    pub w: c_int,
    pub h: c_int,
    pub kind: String,
    pub items: Vec<MenuItem>,
    pub sel: usize,
    pub open: bool,
    pub damage: Damage,
    pub pixmap: Pixmap,
    pub picture: Picture,
}

pub struct Compositor {
    pub active: bool,
    pub overlay: Window,
    pub overlay_picture: Picture,
    pub root_picture: Picture,
    pub shadow_fill: Picture,
    pub damage_event: c_int,
    pub dirty: bool,
    pub fmt: *mut XRenderPictFormat,
}

pub struct Wm {
    pub dpy: *mut Display,
    pub screen: c_int,
    pub root: Window,
    pub sw: c_int,
    pub sh: c_int,
    pub a: Atoms,
    pub gc: GC,
    pub font: *mut XFontStruct,
    pub font_big: *mut XFontStruct,
    pub ascent: c_int,
    pub ascent_big: c_int,
    // palette (pixel values)
    pub c_bg: c_ulong,
    pub c_bar: c_ulong,
    pub c_bar_active: c_ulong,
    pub c_panel: c_ulong,
    pub c_text: c_ulong,
    pub c_dim: c_ulong,
    pub c_accent: c_ulong,
    pub c_border: c_ulong,
    pub c_border_active: c_ulong,
    pub c_popup: c_ulong,
    pub c_sel: c_ulong,
    pub c_red: c_ulong,
    pub cursor_ptr: Cursor,
    pub cursor_move: Cursor,
    // clients
    pub clients: HashMap<Window, Client>,
    pub stack: Vec<Window>, // bottom -> top (mapping order)
    pub focus: Window,
    pub desk: u32,
    pub showing_desktop: bool,
    pub saved_desktop: Vec<Window>,
    // chrome
    pub panel: Window,
    pub dock: Window,
    pub support_win: Window,
    pub panel_clock: String,
    pub panel_date: String,
    pub panel_battery: String,
    pub popup: Option<Popup>,
    pub app_list: Vec<(String, String, bool)>, // (name, exec, terminal)
    // compositor
    pub comp: Compositor,
    // compositor bookkeeping: chrome windows (panel/dock/popups) and the
    // reverse damage-id -> window map; expect_unmap counts the synthetic
    // UnmapNotify a ReparentWindow causes during manage()
    pub comp_windows: HashMap<Window, (Damage, Pixmap, Picture)>,
    pub damage_map: HashMap<Damage, Window>,
    pub expect_unmap: HashMap<Window, c_int>,
    pub smoke_win: Window,
    // smoke-test instrumentation
    pub smoke: bool,
    pub smoke_events: u64,
    pub smoke_steps: Vec<String>,
    // misc
    pub quit: bool,
    pub last_clock_tick: u64,
    pub tick: u64,
}

// ------------------------------------------------------------------- palette
unsafe fn alloc_pixel(d: *mut Display, cmap: Colormap, spec: &str, fallback: c_ulong) -> c_ulong {
    let mut xc: XColor = std::mem::zeroed();
    if XParseColor(d, cmap, cstr(spec).as_ptr(), &mut xc) != 0
        && XAllocColor(d, cmap, &mut xc) != 0
    {
        return xc.pixel;
    }
    fallback
}

unsafe fn load_font(d: *mut Display, names: &[&str]) -> (*mut XFontStruct, c_int) {
    for n in names {
        let f = XLoadQueryFont(d, cstr(n).as_ptr());
        if !f.is_null() {
            let mut dir = 0;
            let mut asc = 0;
            let mut desc = 0;
            let mut overall: XCharStruct = std::mem::zeroed();
            let probe = latin1("Hg");
            XTextExtents(f, probe.as_ptr(), 2, &mut dir, &mut asc, &mut desc, &mut overall);
            return (f, asc);
        }
    }
    (ptr::null_mut(), 10)
}

fn text_width(font: *mut XFontStruct, s: &str) -> c_int {
    if font.is_null() || s.is_empty() {
        return 0;
    }
    unsafe {
        let buf = latin1(s);
        let mut dir = 0;
        let mut asc = 0;
        let mut desc = 0;
        let mut overall: XCharStruct = std::mem::zeroed();
        XTextExtents(
            font,
            buf.as_ptr(),
            (buf.len() - 1) as c_int,
            &mut dir,
            &mut asc,
            &mut desc,
            &mut overall,
        );
        overall.width as c_int
    }
}

fn truncate(font: *mut XFontStruct, s: &str, maxw: c_int) -> String {
    if maxw <= 8 || text_width(font, s) <= maxw {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut lo = 0usize;
    let mut out = String::new();
    for i in 1..=chars.len() {
        let cand: String = chars[..i].iter().collect::<String>() + "..";
        if text_width(font, &cand) > maxw {
            lo = i.saturating_sub(1);
            break;
        }
        lo = i;
    }
    out = chars[..lo].iter().collect::<String>();
    if lo < chars.len() {
        out += "..";
    }
    out
}

unsafe fn draw_text(dpy: *mut Display, gc: GC, dr: Drawable, font: *mut XFontStruct, color: c_ulong, x: c_int, y: c_int, s: &str) {
    XSetForeground(dpy, gc, color);
    XSetFont(dpy, gc, font as Font);
    let buf = latin1(s);
    XDrawString(dpy, dr, gc, x, y, buf.as_ptr(), (buf.len() - 1) as c_int);
}

unsafe fn fill(dpy: *mut Display, gc: GC, dr: Drawable, color: c_ulong, x: c_int, y: c_int, w: c_int, h: c_int) {
    XSetForeground(dpy, gc, color);
    XFillRectangle(dpy, dr, gc, x, y, w.max(0) as c_uint, h.max(0) as c_uint);
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ============================================================================
// setup
// ============================================================================

unsafe fn take_wm_ownership(d: *mut Display, root: Window, a: &Atoms) -> bool {
    // claim WM_Sn so other WMs and tools can see a WM is running
    let sel = intern(d, &format!("WM_S{}", XDefaultScreen(d)));
    let owner = XGetSelectionOwner(d, sel);
    if owner != 0 {
        eprintln!("minerva-wm: another window manager already owns WM_S{} (win 0x{:x})", XDefaultScreen(d), owner);
        return false;
    }
    // try to become the WM: BadAccess here means a WM grabbed SubstructureRedirect first
    clear_error();
    XSelectInput(
        d,
        root,
        SubstructureRedirectMask
            | SubstructureNotifyMask
            | StructureNotifyMask
            | PropertyChangeMask
            | ButtonPressMask,
    );
    XSync(d, 0);
    if error_seen() == BadAccess_ {
        eprintln!("minerva-wm: another window manager is already running on this display (BadAccess on root SubstructureRedirect)");
        return false;
    }
    // own the selection with a dedicated invisible holder window
    let holder_attrs: XSetWindowAttributes = std::mem::zeroed();
    let holder = XCreateWindow(
        d,
        root,
        -100,
        -100,
        1,
        1,
        0,
        0,
        InputOnly,
        ptr::null_mut(),
        0,
        &holder_attrs as *const XSetWindowAttributes as *mut XSetWindowAttributes,
    );
    XSetSelectionOwner(d, sel, holder, CurrentTime);
    prop_set_str(d, root, a.minerva_supporting, false, &format!("minerva-wm {}", VERSION));
    true
}

impl Wm {
    unsafe fn new(dpy: *mut Display, no_composite: bool, smoke: bool) -> Wm {
        let screen = XDefaultScreen(dpy);
        let root = XRootWindow(dpy, screen);
        let sw = XDisplayWidth(dpy, screen);
        let sh = XDisplayHeight(dpy, screen);
        let cmap = XDefaultColormap(dpy, screen);
        let a = init_atoms(dpy);
        XSetErrorHandler(Some(wm_error_handler));

        let c_bg = alloc_pixel(dpy, cmap, "#0b1020", XBlackPixel(dpy, screen));
        let c_bar = alloc_pixel(dpy, cmap, "#1c2029", c_bg);
        let c_bar_active = alloc_pixel(dpy, cmap, "#262c38", c_bar);
        let c_panel = alloc_pixel(dpy, cmap, "#12151c", c_bg);
        let c_text = alloc_pixel(dpy, cmap, "#e8eaf0", XWhitePixel(dpy, screen));
        let c_dim = alloc_pixel(dpy, cmap, "#9aa3b5", c_text);
        let c_accent = alloc_pixel(dpy, cmap, "#64b6ff", c_text);
        let c_border = alloc_pixel(dpy, cmap, "#2a2f3a", c_bar);
        let c_border_active = alloc_pixel(dpy, cmap, "#64b6ff", c_accent);
        let c_popup = alloc_pixel(dpy, cmap, "#171b23", c_panel);
        let c_sel = alloc_pixel(dpy, cmap, "#233047", c_bar_active);
        let c_red = alloc_pixel(dpy, cmap, "#e57373", c_dim);

        let gc = XCreateGC(dpy, root, 0, ptr::null_mut());
        let (font, ascent) = load_font(dpy, &[
            "-misc-fixed-medium-r-normal--13-120-75-75-c-70-iso10646-1",
            "6x13",
            "fixed",
        ]);
        let (font_big, ascent_big) = load_font(dpy, &[
            "-misc-fixed-bold-r-normal--15-140-75-75-c-90-iso10646-1",
            "8x16",
            "fixed",
        ]);
        let font_big = if font_big.is_null() { font } else { font_big };
        let ascent_big = if font_big == font { ascent } else { ascent_big };

        let cursor_ptr = XCreateFontCursor(dpy, XC_left_ptr);
        let cursor_move = XCreateFontCursor(dpy, XC_fleur);
        XDefineCursor(dpy, root, cursor_ptr);

        let mut wm = Wm {
            dpy,
            screen,
            root,
            sw,
            sh,
            a,
            gc,
            font,
            font_big,
            ascent,
            ascent_big,
            c_bg,
            c_bar,
            c_bar_active,
            c_panel,
            c_text,
            c_dim,
            c_accent,
            c_border,
            c_border_active,
            c_popup,
            c_sel,
            c_red,
            cursor_ptr,
            cursor_move,
            clients: HashMap::new(),
            stack: Vec::new(),
            focus: 0,
            desk: 0,
            showing_desktop: false,
            saved_desktop: Vec::new(),
            panel: 0,
            dock: 0,
            support_win: 0,
            panel_clock: String::new(),
            panel_date: String::new(),
            panel_battery: String::new(),
            popup: None,
            app_list: Vec::new(),
            comp: Compositor {
                active: false,
                overlay: 0,
                overlay_picture: 0,
                root_picture: 0,
                shadow_fill: 0,
                damage_event: -1,
                dirty: true,
                fmt: ptr::null_mut(),
            },
            comp_windows: HashMap::new(),
            damage_map: HashMap::new(),
            expect_unmap: HashMap::new(),
            smoke_win: 0,
            smoke,
            smoke_events: 0,
            smoke_steps: Vec::new(),
            quit: false,
            last_clock_tick: 0,
            tick: 0,
        };
        wm.compositor_init(no_composite);
        wm
    }

    // ---------------------------------------------------------------- chrome
    unsafe fn chrome_init(&mut self) {
        if self.smoke {
            // smoke mode: no panel/dock, keeps the Xvfb surface clean and the
            // test focused on WM semantics
            self.set_ewmh_root_props();
            return;
        }
        let mut attrs: XSetWindowAttributes = std::mem::zeroed();
        attrs.override_redirect = 1;
        attrs.event_mask = ExposureMask | ButtonPressMask | StructureNotifyMask;
        attrs.background_pixel = self.c_panel;
        attrs.cursor = self.cursor_ptr;
        self.panel = XCreateWindow(
            self.dpy,
            self.root,
            0,
            0,
            self.sw as c_uint,
            PANEL_H as c_uint,
            0,
            XDefaultDepth(self.dpy, self.screen),
            InputOutput,
            XDefaultVisual(self.dpy, self.screen),
            CWOverrideRedirect | CWEventMask | CWBackPixel | CWCursor,
            &mut attrs,
        );
        self.dock = XCreateWindow(
            self.dpy,
            self.root,
            0,
            self.sh - DOCK_H,
            self.sw as c_uint,
            DOCK_H as c_uint,
            0,
            XDefaultDepth(self.dpy, self.screen),
            InputOutput,
            XDefaultVisual(self.dpy, self.screen),
            CWOverrideRedirect | CWEventMask | CWBackPixel | CWCursor,
            &mut attrs,
        );
        // supporting-wm-check child window (EWMH requirement)
        self.support_win = XCreateSimpleWindow(
            self.dpy,
            self.root,
            -200,
            -200,
            1,
            1,
            0,
            self.c_bg,
            self.c_bg,
        );
        prop_set_windows(
            self.dpy,
            self.support_win,
            self.a.net_supporting_wm_check,
            &[self.support_win],
        );
        prop_set_str(self.dpy, self.support_win, self.a.net_wm_name, true, "Minerva WM");
        prop_set_windows(
            self.dpy,
            self.root,
            self.a.net_supporting_wm_check,
            &[self.support_win],
        );
        XMapWindow(self.dpy, self.panel);
        XMapWindow(self.dpy, self.dock);
        self.comp_add_window(self.panel);
        self.comp_add_window(self.dock);
        self.load_apps();
        self.refresh_panel_state();
        self.draw_panel();
        self.draw_dock();
        self.set_ewmh_root_props();
    }

    unsafe fn set_ewmh_root_props(&mut self) {
        let supported = vec![
            self.a.net_client_list,
            self.a.net_client_list_stacking,
            self.a.net_active_window,
            self.a.net_close_window,
            self.a.net_wm_name,
            self.a.net_wm_state,
            self.a.net_wm_state_fullscreen,
            self.a.net_wm_state_maximized_vert,
            self.a.net_wm_state_maximized_horz,
            self.a.net_wm_state_above,
            self.a.net_wm_state_below,
            self.a.net_wm_state_sticky,
            self.a.net_wm_state_hidden,
            self.a.net_wm_desktop,
            self.a.net_number_of_desktops,
            self.a.net_current_desktop,
            self.a.net_desktop_names,
            self.a.net_showing_desktop,
            self.a.net_workarea,
            self.a.net_frame_extents,
            self.a.net_wm_moveresize,
            self.a.net_request_frame_extents,
            self.a.net_supporting_wm_check,
        ];
        prop_set_atoms(self.dpy, self.root, self.a.net_supported, &supported);
        prop_set_longs(self.dpy, self.root, self.a.net_number_of_desktops, &[NDESKS as c_long]);
        let mut names: Vec<u8> = Vec::new();
        for i in 0..NDESKS {
            names.extend_from_slice(format!("Workspace {}", i + 1).as_bytes());
            names.push(0);
        }
        XChangeProperty(
            self.dpy,
            self.root,
            self.a.net_desktop_names,
            self.a.utf8_string,
            8,
            PropModeReplace,
            names.as_ptr(),
            names.len() as c_int,
        );
        prop_set_longs(self.dpy, self.root, self.a.net_current_desktop, &[self.desk as c_long]);
        let wy = PANEL_H;
        let wh = self.sh - PANEL_H - DOCK_H;
        let workarea: Vec<c_long> = (0..NDESKS)
            .flat_map(|_| vec![0 as c_long, wy as c_long, self.sw as c_long, wh as c_long])
            .collect();
        prop_set_longs(self.dpy, self.root, self.a.net_workarea, &workarea);
        prop_set_longs(self.dpy, self.root, self.a.net_showing_desktop, &[if self.showing_desktop { 1 } else { 0 }]);
        self.update_client_list();
    }

    unsafe fn update_client_list(&mut self) {
        let list: Vec<Window> = self
            .stack
            .iter()
            .cloned()
            .filter(|w| self.clients.contains_key(w))
            .collect();
        let stacking = list.clone();
        prop_set_windows(self.dpy, self.root, self.a.net_client_list, &list);
        prop_set_windows(self.dpy, self.root, self.a.net_client_list_stacking, &stacking);
        prop_set_windows(
            self.dpy,
            self.root,
            self.a.net_active_window,
            &[if self.focus == 0 { None_ } else { self.focus }],
        );
    }

    // -------------------------------------------------------------- managing
    unsafe fn scan_existing(&mut self) {
        let mut root_r: Window = 0;
        let mut parent_r: Window = 0;
        let mut children: *mut Window = ptr::null_mut();
        let mut n: c_uint = 0;
        if XQueryTree(self.dpy, self.root, &mut root_r, &mut parent_r, &mut children, &mut n) == 0 {
            return;
        }
        let wins = std::slice::from_raw_parts(children, n as usize).to_vec();
        XFree(children as *mut c_void);
        for w in wins {
            let mut attrs: XWindowAttributes = std::mem::zeroed();
            clear_error();
            if XGetWindowAttributes(self.dpy, w, &mut attrs) == 0 {
                continue;
            }
            if attrs.override_redirect == 1 || attrs.map_state != IsViewable {
                continue;
            }
            self.manage(w, attrs.x, attrs.y, attrs.width as c_uint, attrs.height as c_uint, attrs.border_width as c_uint);
        }
    }

    unsafe fn manage(
        &mut self,
        win: Window,
        x: c_int,
        y: c_int,
        w: c_uint,
        h: c_uint,
        bw: c_uint,
    ) {
        if self.clients.contains_key(&win) || win == self.panel || win == self.dock {
            return;
        }
        // transient-for / type checks
        let tf = prop_get_longs(self.dpy, win, self.a.wm_transient_for);
        let transient = !tf.is_empty() && tf[0] != 0;
        let wtype = prop_get_longs(self.dpy, win, self.a.net_wm_window_type);
        let is_dock = wtype.iter().any(|t| *t as Atom == self.a.net_wm_window_type_dock);
        // WM_HINTS: input hint is field 2 (format 32: flags, input, ...)
        let hints = prop_get_longs(self.dpy, win, self.a.wm_hints);
        let input_hint = if hints.len() >= 2 && (hints[0] & (1 << 0)) != 0 {
            hints[1] != 0
        } else {
            true
        };
        // size hints: min at [5],[6], max at [7],[8] when flags say so
        let (mut min_w, mut min_h, mut max_w, mut max_h) = (1u32, 1u32, 0u32, 0u32);
        let sh = prop_get_longs(self.dpy, win, self.a.wm_normal_hints);
        if sh.len() >= 9 {
            let flags = sh[0] as u64;
            if flags & (1 << 4) != 0 {
                // PMinSize
                min_w = sh[5].max(1) as u32;
                min_h = sh[6].max(1) as u32;
            }
            if flags & (1 << 5) != 0 {
                // PMaxSize
                max_w = sh[7].max(0) as u32;
                max_h = sh[8].max(0) as u32;
            }
        }
        let title = prop_get_string(self.dpy, win, self.a.net_wm_name)
            .or_else(|| prop_get_string(self.dpy, win, self.a.wm_name))
            .unwrap_or_else(|| "Untitled".to_string());

        // frame geometry: titlebar on top + 1px border, dock-type windows
        // and transients stay undecorated (dialogs keep native chrome)
        let decor = if is_dock || transient { 0 } else { DECOR_H };
        let fx = x - FRAME_BW as c_int;
        let fy = y - decor - FRAME_BW as c_int;
        let fw = w + 2 * FRAME_BW;
        let fh = h + decor as c_uint + 2 * FRAME_BW;

        let mut attrs: XSetWindowAttributes = std::mem::zeroed();
        attrs.background_pixel = self.c_bar;
        attrs.border_pixel = self.c_border;
        attrs.event_mask = SubstructureRedirectMask | SubstructureNotifyMask | ExposureMask | ButtonPressMask | StructureNotifyMask;
        attrs.cursor = self.cursor_ptr;
        let frame = XCreateWindow(
            self.dpy,
            self.root,
            fx,
            fy,
            fw,
            fh,
            FRAME_BW,
            XDefaultDepth(self.dpy, self.screen),
            InputOutput,
            XDefaultVisual(self.dpy, self.screen),
            CWBackPixel | CWBorderPixel | CWEventMask | CWCursor,
            &mut attrs,
        );
        // XReparentWindow of a mapped window generates one UnmapNotify for
        // the client — swallow exactly that one, never a later real unmap
        *self.expect_unmap.entry(win).or_insert(0) += 1;
        XReparentWindow(self.dpy, win, frame, FRAME_BW as c_int, (decor + FRAME_BW as c_int));
        XSetWindowBorderWidth(self.dpy, win, 0);
        XSelectInput(
            self.dpy,
            win,
            StructureNotifyMask | PropertyChangeMask | EnterWindowMask,
        );

        let c = Client {
            win,
            frame,
            x,
            y,
            w,
            h,
            title,
            fullscreen: false,
            maximized: false,
            above: false,
            sticky: false,
            iconic: false,
            desk: self.desk,
            transient,
            input_hint,
            mapped: true,
            min_w,
            min_h,
            max_w,
            max_h,
            saved: (x, y, w, h),
            damage: 0,
            child_damage: 0,
            pixmap: 0,
            picture: 0,
            child_pixmap: 0,
            child_picture: 0,
        };
        self.clients.insert(win, c);
        self.stack.push(win);
        // WM_STATE + desktop
        prop_set_longs(self.dpy, win, self.a.wm_state, &[NormalState, None_ as c_long]);
        prop_set_longs(self.dpy, win, self.a.net_wm_desktop, &[self.desk as c_long]);
        prop_set_longs(
            self.dpy,
            win,
            self.a.net_frame_extents,
            &[
                FRAME_BW as c_long,
                FRAME_BW as c_long,
                decor as c_long,
                FRAME_BW as c_long,
            ],
        );
        self.comp_add_window(win);
        XMapWindow(self.dpy, frame);
        self.draw_decor(win);
        if !transient {
            self.focus_client(win, true);
        } else if self.focus == 0 {
            self.focus_client(win, true);
        }
        self.update_client_list();
        if !self.smoke {
            self.draw_dock();
        }
    }

    unsafe fn unmanage(&mut self, win: Window) {
        let frame = match self.clients.get(&win) {
            Some(c) => c.frame,
            None => return,
        };
        // comp cleanup must run while the client is still in the map (it
        // owns the damage ids), then the frame goes
        self.comp_remove_window(win, frame);
        let c = self.clients.remove(&win).unwrap();
        XDestroyWindow(self.dpy, c.frame);
        self.stack.retain(|w| *w != win);
        if self.focus == win {
            self.focus = 0;
            if let Some(next) = self.stack.iter().rev().find(|w| {
                self.clients
                    .get(w)
                    .map(|c| c.mapped && !c.iconic && self.desk_match(c))
                    .unwrap_or(false)
            }) {
                let n = *next;
                self.focus_client(n, true);
            }
        }
        self.update_client_list();
        if !self.smoke {
            self.draw_dock();
        }
    }

    fn desk_match(&self, c: &Client) -> bool {
        c.sticky || c.desk == DESK_ALL || c.desk == self.desk
    }

    unsafe fn frame_geom(&self, win: Window) -> (c_int, c_int, c_uint, c_uint) {
        if let Some(c) = self.clients.get(&win) {
            let decor = if c.fullscreen || c.transient { 0 } else { DECOR_H };
            if c.fullscreen {
                return (0, 0, self.sw as c_uint, self.sh as c_uint);
            }
            (
                c.x - FRAME_BW as c_int,
                c.y - decor - FRAME_BW as c_int,
                c.w + 2 * FRAME_BW,
                c.h + decor as c_uint + 2 * FRAME_BW,
            )
        } else {
            (0, 0, 0, 0)
        }
    }

    unsafe fn apply_frame_geom(&mut self, win: Window) {
        let (fx, fy, fw, fh) = self.frame_geom(win);
        let c = match self.clients.get_mut(&win) {
            Some(c) => c,
            None => return,
        };
        let decor = if c.fullscreen || c.transient { 0 } else { DECOR_H };
        XMoveResizeWindow(self.dpy, c.frame, fx, fy, fw, fh);
        XMoveResizeWindow(
            self.dpy,
            win,
            FRAME_BW as c_int,
            decor + FRAME_BW as c_int,
            c.w,
            c.h,
        );
    }

    // ----------------------------------------------------------------- focus
    unsafe fn focus_client(&mut self, win: Window, raise: bool) {
        if !self.clients.contains_key(&win) {
            return;
        }
        let old = self.focus;
        self.focus = win;
        if let Some(c) = self.clients.get(&win) {
            if raise {
                XRaiseWindow(self.dpy, c.frame);
                self.stack.retain(|w| *w != win);
                self.stack.push(win);
            }
            XSetWindowBorder(self.dpy, c.frame, self.c_border_active);
            if c.input_hint {
                XSetInputFocus(self.dpy, win, RevertToPointerRoot, CurrentTime);
            } else {
                client_msg(
                    self.dpy,
                    win,
                    self.a.wm_protocols,
                    [self.a.wm_take_focus as c_long, CurrentTime as c_long, 0, 0, 0],
                );
            }
        }
        if old != 0 && self.clients.contains_key(&old) {
            if let Some(c) = self.clients.get(&old) {
                XSetWindowBorder(self.dpy, c.frame, self.c_border);
            }
            self.draw_decor(old);
        }
        self.draw_decor(win);
        self.update_client_list();
        if !self.smoke {
            self.draw_panel();
            self.draw_dock();
        }
        self.comp.dirty = true;
    }

    unsafe fn cycle_focus(&mut self, dir: i32) {
        let cand: Vec<Window> = self
            .stack
            .iter()
            .rev()
            .cloned()
            .filter(|w| {
                self.clients
                    .get(w)
                    .map(|c| !c.iconic && self.desk_match(c))
                    .unwrap_or(false)
            })
            .collect();
        if cand.is_empty() {
            return;
        }
        let idx = cand.iter().position(|w| *w == self.focus).unwrap_or(0) as i32;
        let n = cand.len() as i32;
        let mut ni = idx + dir;
        if ni < 0 {
            ni += n;
        }
        if ni >= n {
            ni -= n;
        }
        let target = cand[ni as usize];
        self.focus_client(target, true);
    }

    // ------------------------------------------------------------ decoration
    unsafe fn draw_decor(&mut self, win: Window) {
        let c = match self.clients.get(&win) {
            Some(c) => c,
            None => return,
        };
        if c.fullscreen || c.transient {
            return;
        }
        let frame = c.frame;
        let fw = (c.w + 2 * FRAME_BW) as c_int;
        let focused = self.focus == win;
        XSetForeground(self.dpy, self.gc, if focused { self.c_bar_active } else { self.c_bar });
        XFillRectangle(self.dpy, frame, self.gc, 0, 0, fw as c_uint, DECOR_H as c_uint);
        // accent underline when focused
        XSetForeground(self.dpy, self.gc, if focused { self.c_accent } else { self.c_border });
        XFillRectangle(self.dpy, frame, self.gc, 0, DECOR_H - 1, fw as c_uint, 1);
        // title
        XSetForeground(self.dpy, self.gc, if focused { self.c_text } else { self.c_dim });
        XSetFont(self.dpy, self.gc, self.font as Font);
        let t = truncate(self.font, &c.title, fw - 92);
        let buf = latin1(&t);
        XDrawString(
            self.dpy,
            frame,
            self.gc,
            10,
            (DECOR_H + self.ascent) / 2 - 2,
            buf.as_ptr(),
            (buf.len() - 1) as c_int,
        );
        // buttons: minimize (line), maximize (rect), close (x) — right side
        let by = DECOR_H / 2;
        let bx_close = fw - 18;
        let bx_max = fw - 40;
        let bx_min = fw - 60;
        XSetForeground(self.dpy, self.gc, self.c_red);
        XDrawLine(self.dpy, frame, self.gc, bx_close - 4, by - 4, bx_close + 4, by + 4);
        XDrawLine(self.dpy, frame, self.gc, bx_close - 4, by + 4, bx_close + 4, by - 4);
        XSetForeground(self.dpy, self.gc, if focused { self.c_text } else { self.c_dim });
        XDrawRectangle(self.dpy, frame, self.gc, bx_max - 4, by - 4, 9, 9);
        XDrawLine(self.dpy, frame, self.gc, bx_min - 5, by + 3, bx_min + 5, by + 3);
        XFlush(self.dpy);
    }

    /// which decoration zone did a click land on?
    fn decor_zone(&self, win: Window, x: c_int, y: c_int) -> u8 {
        let c = match self.clients.get(&win) {
            Some(c) => c,
            None => return 0,
        };
        let fw = (c.w + 2 * FRAME_BW) as c_int;
        if y < DECOR_H {
            if x >= fw - 28 {
                return 1; // close
            }
            if x >= fw - 50 {
                return 2; // maximize
            }
            if x >= fw - 70 {
                return 3; // minimize
            }
            return 4; // titlebar
        }
        // bottom/right edges resize
        let fh = (c.h + DECOR_H as c_uint + 2 * FRAME_BW) as c_int;
        if x >= fw - RESIZE_EDGE && y >= fh - RESIZE_EDGE {
            return 6; // corner
        }
        if x >= fw - RESIZE_EDGE {
            return 7; // right edge
        }
        if y >= fh - RESIZE_EDGE {
            return 8; // bottom edge
        }
        0
    }

    // ----------------------------------------------------------------- state
    unsafe fn set_fullscreen(&mut self, win: Window, on: bool) {
        let c = match self.clients.get_mut(&win) {
            Some(c) => c,
            None => return,
        };
        if c.fullscreen == on {
            return;
        }
        if on {
            c.saved = (c.x, c.y, c.w, c.h);
            c.fullscreen = true;
            c.x = 0;
            c.y = 0;
            c.w = self.sw as c_uint;
            c.h = self.sh as c_uint;
        } else {
            c.fullscreen = false;
            c.x = c.saved.0;
            c.y = c.saved.1;
            c.w = c.saved.2;
            c.h = c.saved.3;
        }
        self.apply_frame_geom(win);
        self.update_wm_state_atoms(win);
        self.draw_decor(win);
        self.comp.dirty = true;
    }

    unsafe fn set_maximized(&mut self, win: Window, on: bool) {
        let c = match self.clients.get_mut(&win) {
            Some(c) => c,
            None => return,
        };
        if c.fullscreen {
            return;
        }
        if c.maximized == on {
            return;
        }
        if on {
            c.saved = (c.x, c.y, c.w, c.h);
            c.maximized = true;
            c.x = 0;
            c.y = PANEL_H;
            c.w = self.sw as c_uint;
            c.h = (self.sh - PANEL_H - DOCK_H) as c_uint;
        } else {
            c.maximized = false;
            c.x = c.saved.0;
            c.y = c.saved.1;
            c.w = c.saved.2;
            c.h = c.saved.3;
        }
        self.apply_frame_geom(win);
        self.update_wm_state_atoms(win);
        self.draw_decor(win);
        if !self.smoke {
            self.draw_dock();
        }
        self.comp.dirty = true;
    }

    unsafe fn set_iconic(&mut self, win: Window, iconic: bool) {
        let c = match self.clients.get_mut(&win) {
            Some(c) => c,
            None => return,
        };
        if c.iconic == iconic {
            return;
        }
        c.iconic = iconic;
        if iconic {
            XUnmapWindow(self.dpy, c.frame);
            c.mapped = false;
            self.comp_invalidate(win);
            prop_set_longs(self.dpy, win, self.a.wm_state, &[IconicState, None_ as c_long]);
            if self.focus == win {
                self.focus = 0;
                if let Some(next) = self.stack.iter().rev().find(|w| {
                    self.clients
                        .get(w)
                        .map(|c| c.mapped && !c.iconic && self.desk_match(c))
                        .unwrap_or(false)
                }) {
                    let n = *next;
                    self.focus_client(n, true);
                }
            }
        } else {
            c.mapped = true;
            XMapWindow(self.dpy, c.frame);
            self.comp_invalidate(win);
            prop_set_longs(self.dpy, win, self.a.wm_state, &[NormalState, None_ as c_long]);
        }
        self.update_wm_state_atoms(win);
        if !self.smoke {
            self.draw_dock();
        }
        self.comp.dirty = true;
    }

    unsafe fn update_wm_state_atoms(&mut self, win: Window) {
        let c = match self.clients.get(&win) {
            Some(c) => c,
            None => return,
        };
        let mut states: Vec<Atom> = Vec::new();
        if c.fullscreen {
            states.push(self.a.net_wm_state_fullscreen);
        }
        if c.maximized {
            states.push(self.a.net_wm_state_maximized_vert);
            states.push(self.a.net_wm_state_maximized_horz);
        }
        if c.above {
            states.push(self.a.net_wm_state_above);
        }
        if c.sticky {
            states.push(self.a.net_wm_state_sticky);
        }
        if c.iconic {
            states.push(self.a.net_wm_state_hidden);
        }
        prop_set_atoms(self.dpy, win, self.a.net_wm_state, &states);
    }

    unsafe fn close_client(&mut self, win: Window) {
        if !self.clients.contains_key(&win) {
            return;
        }
        // WM_DELETE_WINDOW if the client supports it, else a hard kill
        let protos = prop_get_longs(self.dpy, win, self.a.wm_protocols);
        if protos.iter().any(|p| *p as Atom == self.a.wm_delete_window) {
            client_msg(
                self.dpy,
                win,
                self.a.wm_protocols,
                [self.a.wm_delete_window as c_long, CurrentTime as c_long, 0, 0, 0],
            );
        } else {
            XKillClient(self.dpy, win);
            self.unmanage(win);
        }
    }

    unsafe fn desk_switch(&mut self, n: u32) {
        if n >= NDESKS || n == self.desk {
            return;
        }
        self.desk = n;
        self.showing_desktop = false;
        prop_set_longs(self.dpy, self.root, self.a.net_current_desktop, &[n as c_long]);
        prop_set_longs(self.dpy, self.root, self.a.net_showing_desktop, &[0]);
        let wins: Vec<Window> = self.stack.clone();
        for w in wins {
            let show = {
                match self.clients.get(&w) {
                    Some(c) => self.desk_match(c) && !c.iconic,
                    None => false,
                }
            };
            if let Some(c) = self.clients.get(&w) {
                if show && !c.mapped {
                    XMapWindow(self.dpy, c.frame);
                    self.comp_invalidate(w);
                    if let Some(c2) = self.clients.get_mut(&w) {
                        c2.mapped = true;
                    }
                } else if !show && c.mapped && !c.iconic {
                    XUnmapWindow(self.dpy, c.frame);
                    self.comp_invalidate(w);
                    if let Some(c2) = self.clients.get_mut(&w) {
                        c2.mapped = false;
                    }
                }
            }
        }
        // focus something on the new desk
        self.focus = 0;
        if let Some(next) = self.stack.iter().rev().find(|w| {
            self.clients
                .get(w)
                .map(|c| c.mapped && !c.iconic && self.desk_match(c))
                .unwrap_or(false)
        }) {
            let n2 = *next;
            self.focus_client(n2, true);
        }
        if !self.smoke {
            self.draw_panel();
            self.draw_dock();
        }
        self.comp.dirty = true;
    }

    unsafe fn show_desktop_toggle(&mut self) {
        if self.showing_desktop {
            let saved = std::mem::take(&mut self.saved_desktop);
            for w in saved {
                if self.clients.contains_key(&w) {
                    self.set_iconic(w, false);
                }
            }
            self.showing_desktop = false;
        } else {
            let wins: Vec<Window> = self.stack.clone();
            let mut saved = Vec::new();
            for w in wins {
                if let Some(c) = self.clients.get(&w) {
                    if !c.iconic && self.desk_match(c) && !c.transient {
                        saved.push(w);
                    }
                }
            }
            for w in &saved {
                self.set_iconic(*w, true);
            }
            self.saved_desktop = saved;
            self.showing_desktop = true;
        }
        prop_set_longs(
            self.dpy,
            self.root,
            self.a.net_showing_desktop,
            &[if self.showing_desktop { 1 } else { 0 }],
        );
    }

    // ------------------------------------------------------- move/resize drag
    unsafe fn interactive_move_resize(&mut self, win: Window, mode: u8, ev: &XButtonEvent) {
        let c = match self.clients.get(&win) {
            Some(c) => c,
            None => return,
        };
        if c.fullscreen {
            return;
        }
        let moving = mode == 4 || mode == 5;
        let (ox, oy, ow, oh) = (c.x, c.y, c.w, c.h);
        let min_w = c.min_w;
        let min_h = c.min_h;
        let max_w = c.max_w;
        let max_h = c.max_h;
        let frame = c.frame;
        if XGrabPointer(
            self.dpy,
            self.root,
            0,
            (ButtonReleaseMask | ButtonPressMask | PointerMotionMask) as c_uint,
            GrabModeAsync,
            GrabModeAsync,
            None_,
            if moving { self.cursor_move } else { self.cursor_ptr },
            ev.time,
        ) != 0
        {
            return;
        }
        let start_x = ev.x_root;
        let start_y = ev.y_root;
        let decor = if c.transient { 0 } else { DECOR_H };
        let mut cur_x = ox;
        let mut cur_y = oy;
        let mut cur_w = ow;
        let mut cur_h = oh;
        loop {
            let mut e: XEvent = std::mem::zeroed();
            XMaskEvent(
                self.dpy,
                ButtonReleaseMask | ButtonPressMask | PointerMotionMask,
                &mut e,
            );
            let t = e.any;
            if t.type_ == ButtonRelease {
                break;
            }
            if t.type_ == MotionNotify {
                let m = e.motion;
                let dx = m.x_root - start_x;
                let dy = m.y_root - start_y;
                if moving {
                    cur_x = ox + dx;
                    cur_y = (oy + dy).max(PANEL_H);
                    if let Some(c) = self.clients.get_mut(&win) {
                        c.x = cur_x;
                        c.y = cur_y;
                    }
                    XMoveWindow(
                        self.dpy,
                        frame,
                        cur_x - FRAME_BW as c_int,
                        cur_y - decor - FRAME_BW as c_int,
                    );
                } else {
                    if mode == 6 || mode == 7 {
                        cur_w = ((ow as c_int + dx).max(min_w as c_int)) as c_uint;
                        if max_w > 0 {
                            cur_w = cur_w.min(max_w);
                        }
                    }
                    if mode == 6 || mode == 8 {
                        cur_h = ((oh as c_int + dy).max(min_h as c_int)) as c_uint;
                        if max_h > 0 {
                            cur_h = cur_h.min(max_h);
                        }
                    }
                    if let Some(c) = self.clients.get_mut(&win) {
                        c.w = cur_w;
                        c.h = cur_h;
                    }
                    XMoveResizeWindow(self.dpy, win, FRAME_BW as c_int, decor + FRAME_BW as c_int, cur_w, cur_h);
                    XResizeWindow(self.dpy, frame, cur_w + 2 * FRAME_BW, cur_h + decor as c_uint + 2 * FRAME_BW);
                    self.comp_invalidate(win);
                }
                self.comp.dirty = true;
                if self.comp.active {
                    self.comp_paint();
                }
            }
        }
        XUngrabPointer(self.dpy, CurrentTime);
        if let Some(c) = self.clients.get_mut(&win) {
            c.x = cur_x;
            c.y = cur_y;
            c.w = cur_w;
            c.h = cur_h;
            if !moving {
                c.saved = (cur_x, cur_y, cur_w, cur_h);
            }
        }
        self.apply_frame_geom(win);
        if !self.smoke && !moving {
            self.draw_dock();
        }
        self.comp.dirty = true;
        XFlush(self.dpy);
    }

    // -------------------------------------------------------- EWMH messaging
    unsafe fn handle_net_wm_state(&mut self, win: Window, action: c_long, d1: c_long, d2: c_long) {
        let cur = prop_get_longs(self.dpy, win, self.a.net_wm_state);
        for &d in &[d1, d2] {
            if d == 0 {
                continue;
            }
            let has = cur.iter().any(|x| *x == d);
            let on = match action {
                0 => false, // remove
                1 => true,  // add
                _ => !has,  // toggle
            };
            let atom = d as Atom;
            if atom == self.a.net_wm_state_fullscreen {
                self.set_fullscreen(win, on);
            } else if atom == self.a.net_wm_state_maximized_vert
                || atom == self.a.net_wm_state_maximized_horz
            {
                self.set_maximized(win, on);
            } else if atom == self.a.net_wm_state_above {
                if let Some(c) = self.clients.get_mut(&win) {
                    c.above = on;
                    let fr = c.frame;
                    self.update_wm_state_atoms(win);
                    if on {
                        XRaiseWindow(self.dpy, fr);
                    }
                }
            } else if atom == self.a.net_wm_state_sticky {
                let mut newdesk = 0u32;
                let mut found = false;
                if let Some(c) = self.clients.get_mut(&win) {
                    c.sticky = on;
                    c.desk = if on { DESK_ALL } else { self.desk };
                    newdesk = c.desk;
                    found = true;
                }
                if found {
                    self.update_wm_state_atoms(win);
                    prop_set_longs(self.dpy, win, self.a.net_wm_desktop, &[newdesk as c_long]);
                }
            } else if atom == self.a.net_wm_state_hidden {
                self.set_iconic(win, on);
            }
        }
    }

    unsafe fn handle_client_message(&mut self, ev: &XClientMessageEvent) {
        let t = ev.message_type;
        let d = ev.data.l;
        if t == self.a.net_active_window {
            let win = d[0] as Window;
            if self.clients.contains_key(&win) {
                if let Some(c) = self.clients.get(&win) {
                    if c.desk != DESK_ALL && c.desk != self.desk {
                        let dd = c.desk;
                        self.desk_switch(dd);
                    }
                    let ic = self.clients.get(&win).map(|c| c.iconic).unwrap_or(false);
                    if ic {
                        self.set_iconic(win, false);
                    }
                }
                self.focus_client(win, true);
            }
        } else if t == self.a.net_close_window {
            let win = d[0] as Window;
            self.close_client(win);
        } else if t == self.a.net_wm_state {
            let win = ev.window;
            self.handle_net_wm_state(win, d[0], d[1], d[2]);
        } else if t == self.a.net_current_desktop {
            let n = d[0];
            if n >= 0 && (n as u32) < NDESKS {
                self.desk_switch(n as u32);
            }
        } else if t == self.a.net_wm_desktop {
            let win = ev.window;
            let n = d[0] as u32;
            if self.clients.contains_key(&win) && (n < NDESKS || n == DESK_ALL) {
                if let Some(c) = self.clients.get_mut(&win) {
                    c.desk = n;
                    c.sticky = n == DESK_ALL;
                }
                prop_set_longs(self.dpy, win, self.a.net_wm_desktop, &[n as c_long]);
                self.update_wm_state_atoms(win);
                let show = self
                    .clients
                    .get(&win)
                    .map(|c| self.desk_match(c) && !c.iconic)
                    .unwrap_or(false);
                if let Some(c) = self.clients.get(&win) {
                    if show && !c.mapped {
                        XMapWindow(self.dpy, c.frame);
                        self.comp_invalidate(win);
                        if let Some(c2) = self.clients.get_mut(&win) {
                            c2.mapped = true;
                        }
                    } else if !show && c.mapped {
                        XUnmapWindow(self.dpy, c.frame);
                        self.comp_invalidate(win);
                        if let Some(c2) = self.clients.get_mut(&win) {
                            c2.mapped = false;
                        }
                    }
                }
                if !self.smoke {
                    self.draw_dock();
                }
            }
        } else if t == self.a.net_showing_desktop {
            let on = d[0] != 0;
            if on != self.showing_desktop {
                self.show_desktop_toggle();
            }
        } else if t == self.a.wm_change_state {
            let win = ev.window;
            if d[0] == IconicState {
                self.set_iconic(win, true);
            }
        } else if t == self.a.net_wm_moveresize {
            let win = ev.window;
            // d: x_root, y_root, direction, button, source
            let dir = d[2];
            let mode: u8 = match dir {
                0..=7 => 6, // any resize direction -> corner resize
                8 => 4,     // MOVE
                _ => 0,
            };
            if mode != 0 && self.clients.contains_key(&win) {
                let mut bev: XButtonEvent = std::mem::zeroed();
                bev.time = CurrentTime;
                bev.x_root = d[0] as c_int;
                bev.y_root = d[1] as c_int;
                self.interactive_move_resize(win, mode, &bev);
            }
        } else if t == self.a.net_request_frame_extents {
            let win = ev.window;
            prop_set_longs(
                self.dpy,
                win,
                self.a.net_frame_extents,
                &[
                    FRAME_BW as c_long,
                    FRAME_BW as c_long,
                    DECOR_H as c_long,
                    FRAME_BW as c_long,
                ],
            );
        }
    }

    // =======================================================================
    // native panel / taskbar / menus — all drawn by this WM, no HTML anywhere
    // =======================================================================

    unsafe fn refresh_panel_state(&mut self) {
        let out = sh_out("date '+%H:%M'");
        if !out.is_empty() {
            self.panel_clock = out;
        }
        self.panel_date = sh_out("date '+%a %e %b'");
        let cap = sh_out(
            "cat /sys/class/power_supply/BAT*/capacity 2>/dev/null | head -1",
        );
        let stat = sh_out(
            "cat /sys/class/power_supply/BAT*/status 2>/dev/null | head -1",
        );
        self.panel_battery = if !cap.is_empty() {
            format!("{}%{}", cap, if stat == "Charging" { "+" } else { "" })
        } else {
            String::new()
        };
    }


    unsafe fn draw_panel(&mut self) {
        if self.panel == 0 {
            return;
        }
        let (sw, panel) = (self.sw, self.panel);
        fill(self.dpy, self.gc, panel, self.c_panel, 0, 0, sw, PANEL_H);
        // launcher button
        fill(self.dpy, self.gc, panel, self.c_bar_active, 6, 4, 118, PANEL_H - 8);
        draw_text(self.dpy, self.gc, panel, self.font_big, self.c_accent, 16, (PANEL_H + self.ascent_big) / 2 - 1, "Minerva");
        // focused window title
        let title = self
            .clients
            .get(&self.focus)
            .map(|c| truncate(self.font, &c.title, self.sw - 420))
            .unwrap_or_default();
        if !title.is_empty() {
            draw_text(self.dpy, self.gc, panel, self.font, self.c_text, 140, (PANEL_H + self.ascent) / 2 - 1, &title);
        }
        // right cluster: battery, date, clock, menu button
        let mut rx = self.sw - 10;
        let menu_w = 64;
        rx -= menu_w;
        fill(self.dpy, self.gc, panel, self.c_bar, rx, 4, menu_w, PANEL_H - 8);
        draw_text(self.dpy, self.gc, panel, self.font, self.c_text, rx + 12, (PANEL_H + self.ascent) / 2 - 1, "Menu");
        let clock_w = text_width(self.font_big, &self.panel_clock) + 14;
        rx -= clock_w;
        draw_text(self.dpy, self.gc, panel, self.font_big, self.c_text, rx + 7, (PANEL_H + self.ascent_big) / 2 - 1, &self.panel_clock);
        if !self.panel_date.is_empty() {
            let dw = text_width(self.font, &self.panel_date) + 12;
            rx -= dw;
            draw_text(self.dpy, self.gc, panel, self.font, self.c_dim, rx + 6, (PANEL_H + self.ascent) / 2 - 1, &self.panel_date);
        }
        if !self.panel_battery.is_empty() {
            let bw = text_width(self.font, &self.panel_battery) + 12;
            rx -= bw;
            draw_text(self.dpy, self.gc, panel, self.font, self.c_dim, rx + 6, (PANEL_H + self.ascent) / 2 - 1, &self.panel_battery);
        }
        // workspace pips
        rx -= 16;
        for i in (0..NDESKS).rev() {
            rx -= 14;
            let on = i == self.desk;
            fill(self.dpy, self.gc, 
                panel,
                if on { self.c_accent } else { self.c_bar_active },
                rx,
                PANEL_H / 2 - 4,
                8,
                8,
            );
        }
        // hairline under panel
        fill(self.dpy, self.gc, panel, self.c_border, 0, PANEL_H - 1, sw, 1);
        XFlush(self.dpy);
    }

    /// panel hit zones: (x0, x1, act)
    fn panel_zones(&self) -> Vec<(c_int, c_int, Act)> {
        let mut z = Vec::new();
        z.push((6, 124, Act::OpenLauncher));
        let menu_x0 = self.sw - 10 - 64;
        z.push((menu_x0, menu_x0 + 64, Act::OpenQs));
        let rx = menu_x0 - 14 * NDESKS as c_int - 16;
        for i in 0..(NDESKS as c_int) {
            z.push((rx + i * 14, rx + i * 14 + 10, Act::Desk(i as i64)));
        }
        z
    }

    unsafe fn draw_dock(&mut self) {
        if self.dock == 0 {
            return;
        }
        let (sw, dock) = (self.sw, self.dock);
        fill(self.dpy, self.gc, dock, self.c_panel, 0, 0, sw, DOCK_H);
        fill(self.dpy, self.gc, dock, self.c_border, 0, 0, sw, 1);
        let wins: Vec<Window> = self
            .stack
            .iter()
            .cloned()
            .filter(|w| {
                self.clients
                    .get(w)
                    .map(|c| self.desk_match(c) && !c.transient)
                    .unwrap_or(false)
            })
            .collect();
        let maxbtn = ((sw - 24) / 180).max(1) as usize;
        let mut x = 12;
        for (i, w) in wins.iter().enumerate().take(maxbtn) {
            let c = match self.clients.get(w) {
                Some(c) => c,
                None => continue,
            };
            let btnw = if wins.len() > maxbtn && i == maxbtn - 1 {
                60
            } else {
                170
            };
            let focused = *w == self.focus && !c.iconic;
            fill(self.dpy, self.gc, 
                dock,
                if focused { self.c_sel } else { self.c_bar },
                x,
                7,
                btnw,
                DOCK_H - 14,
            );
            if focused {
                fill(self.dpy, self.gc, dock, self.c_accent, x, 7, 3, DOCK_H - 14);
            }
            let raw = if c.iconic { format!("({})", c.title) } else { c.title.clone() };
            let label = truncate(self.font, &raw, btnw - 16);
            draw_text(self.dpy, self.gc, 
                dock,
                self.font,
                if c.iconic { self.c_dim } else { self.c_text },
                x + 10,
                (DOCK_H + self.ascent) / 2,
                &label,
            );
            x += btnw + 8;
        }
        XFlush(self.dpy);
    }

    /// which dock button index did x land on?
    fn dock_zone(&self, x: c_int, y: c_int) -> Option<Window> {
        if y < 5 || y > DOCK_H - 5 {
            return None;
        }
        let wins: Vec<Window> = self
            .stack
            .iter()
            .cloned()
            .filter(|w| {
                self.clients
                    .get(w)
                    .map(|c| self.desk_match(c) && !c.transient)
                    .unwrap_or(false)
            })
            .collect();
        let maxbtn = ((self.sw - 24) / 180).max(1) as usize;
        let mut bx = 12;
        for (i, w) in wins.iter().enumerate().take(maxbtn) {
            let btnw = if wins.len() > maxbtn && i == maxbtn - 1 { 60 } else { 170 };
            if x >= bx && x < bx + btnw {
                return Some(*w);
            }
            bx += btnw + 8;
        }
        None
    }

    // ------------------------------------------------------------------ apps
    unsafe fn load_apps(&mut self) {
        let mut dirs = vec![PathBuf::from("/usr/share/applications")];
        if let Ok(h) = std::env::var("HOME") {
            dirs.push(PathBuf::from(format!("{}/.local/share/applications", h)));
        }
        let mut seen = HashMap::new();
        for dir in dirs {
            let rd = match fs::read_dir(&dir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e != "desktop").unwrap_or(true) {
                    continue;
                }
                let content = match fs::read_to_string(&p) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let mut in_entry = false;
                let mut name = String::new();
                let mut exec = String::new();
                let mut terminal = false;
                let mut hidden = false;
                let mut is_app = false;
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with('[') {
                        in_entry = line == "[Desktop Entry]";
                        continue;
                    }
                    if !in_entry {
                        continue;
                    }
                    if let Some(v) = line.strip_prefix("Name=") {
                        if name.is_empty() {
                            name = v.trim().to_string();
                        }
                    } else if let Some(v) = line.strip_prefix("Exec=") {
                        if exec.is_empty() {
                            exec = v.trim().to_string();
                        }
                    } else if line == "Terminal=true" {
                        terminal = true;
                    } else if line == "NoDisplay=true" || line == "Hidden=true" {
                        hidden = true;
                    } else if line == "Type=Application" {
                        is_app = true;
                    }
                }
                if hidden || name.is_empty() || exec.is_empty() || !is_app {
                    continue;
                }
                // strip desktop-entry field codes (%u %U %f %F %i %c %k)
                let exec = exec
                    .split_whitespace()
                    .filter(|tok| !tok.starts_with('%'))
                    .collect::<Vec<_>>()
                    .join(" ");
                if exec.is_empty() {
                    continue;
                }
                seen.entry(name).or_insert((exec, terminal));
            }
        }
        let mut list: Vec<(String, String, bool)> = seen
            .into_iter()
            .map(|(n, (e, t))| (n, e, t))
            .collect();
        list.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        self.app_list = list;
    }

    fn launch_cmd(exec: &str, terminal: bool) -> String {
        if terminal {
            format!("xterm -title {:?} -e sh -c {:?}", exec, exec)
        } else {
            exec.to_string()
        }
    }

    unsafe fn run_act(&mut self, act: Act) {
        match act {
            Act::Nothing => {}
            Act::Close => {
                if self.focus != 0 {
                    let f = self.focus;
                    self.close_client(f);
                }
            }
            Act::Launch(cmd) => spawn_app(&cmd),
            Act::Desk(n) => {
                let d = if n < 0 {
                    (self.desk + NDESKS - 1) % NDESKS
                } else if n >= NDESKS as i64 {
                    (self.desk + 1) % NDESKS
                } else {
                    n as u32
                };
                self.desk_switch(d);
            }
            Act::ShowDesktop => self.show_desktop_toggle(),
            Act::Lock => {
                // real lock via the DM or systemd-logind; never a fake overlay
                if !sh_out("command -v loginctl").is_empty() {
                    spawn_app("loginctl lock-session");
                } else {
                    spawn_app("dm-tool lock");
                }
            }
            Act::Logout => spawn_app("systemctl --user exit 0 || loginctl terminate-user \"$USER\""),
            Act::Reboot => spawn_app("systemctl reboot || loginctl reboot"),
            Act::Poweroff => spawn_app("systemctl poweroff || loginctl poweroff"),
            Act::ToggleWifi => {
                let on = sh_out("nmcli radio wifi 2>/dev/null") == "enabled";
                if on {
                    spawn_app("nmcli radio wifi off");
                } else {
                    spawn_app("nmcli radio wifi on");
                }
            }
            Act::ToggleBt => {
                let on = sh_out("bluetoothctl show 2>/dev/null | grep -c 'Powered: yes' 2>/dev/null") == "1";
                if on {
                    spawn_app("bluetoothctl power off");
                } else {
                    spawn_app("bluetoothctl power on");
                }
            }
            Act::VolUp => spawn_app(
                "pactl set-sink-volume @DEFAULT_SINK@ +5% 2>/dev/null || amixer -q set Master 5%+",
            ),
            Act::VolDown => spawn_app(
                "pactl set-sink-volume @DEFAULT_SINK@ -5% 2>/dev/null || amixer -q set Master 5%-",
            ),
            Act::VolMute => spawn_app(
                "pactl set-sink-mute @DEFAULT_SINK@ toggle 2>/dev/null || amixer -q set Master toggle",
            ),
            Act::BriUp => spawn_app("brightnessctl set +10% 2>/dev/null || xbacklight -inc 10"),
            Act::BriDown => spawn_app("brightnessctl set 10%- 2>/dev/null || xbacklight -dec 10"),
            Act::Record => {
                let rt = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
                let pidf = format!("{}/minerva/wm-record.pid", rt);
                let running = fs::read_to_string(&pidf)
                    .map(|s| {
                        s.trim()
                            .parse::<i32>()
                            .map(|p| p > 0 && std::path::Path::new(&format!("/proc/{}", p)).exists())
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if running {
                    let pid = fs::read_to_string(&pidf).unwrap_or_default();
                    let _ = fs::remove_file(&pidf);
                    spawn_app(&format!("kill -INT {} 2>/dev/null", pid.trim()));
                } else if sh_out("command -v ffmpeg").is_empty() {
                    eprintln!("minerva-wm: ffmpeg not installed — screen recording unavailable (sudo apt install ffmpeg)");
                    spawn_app("dunstify -u critical 'Minerva' 'Screen recording needs ffmpeg: sudo apt install ffmpeg' 2>/dev/null || true");
                } else {
                    let vids = format!(
                        "{}/Videos",
                        std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
                    );
                    let _ = fs::create_dir_all(&vids);
                    let file = format!("{}/minerva-rec-{}.mp4", vids, sh_out("date +%Y%m%d-%H%M%S"));
                    let cmd = format!(
                        "ffmpeg -y -f x11grab -framerate 30 -video-size $(xdpyinfo | awk '/dimensions/ {{print $2}}') -i \"$DISPLAY\" '{}' >/dev/null 2>&1 & echo $! > '{}'",
                        file.replace('\'', "'\\''"),
                        pidf
                    );
                    spawn_app(&cmd);
                }
            }
            Act::OpenQs => self.popup_open_qs(),
            Act::OpenLauncher => self.popup_open_launcher(),
            Act::OpenPower => self.popup_open_power(),
            Act::OpenAbout => self.popup_open_about(),
            Act::OpenTerminal => spawn_app("xterm"),
            Act::Focus(w) => {
                if let Some(c) = self.clients.get(&w) {
                    if c.desk != DESK_ALL && c.desk != self.desk {
                        let d = c.desk;
                        self.desk_switch(d);
                    }
                    let ic = self.clients.get(&w).map(|c| c.iconic).unwrap_or(false);
                    if ic {
                        self.set_iconic(w, false);
                    }
                }
                self.focus_client(w, true);
            }
            Act::Cycle(dir) => self.cycle_focus(dir),
            Act::Minimize(w) => {
                let ic = self.clients.get(&w).map(|c| c.iconic).unwrap_or(false);
                self.set_iconic(w, !ic);
                if !ic && self.focus == w {
                    self.focus = 0;
                }
            }
        }
    }

    // --------------------------------------------------------------- popups
    unsafe fn popup_open(&mut self, kind: &str, x: c_int, y: c_int, w: c_int, items: Vec<MenuItem>) {
        self.popup_close();
        let rowh = 30;
        let h = (items.len() as c_int * rowh + 12).min(self.sh - y - 8).max(rowh + 12);
        let w = w.min(self.sw - x - 8);
        let mut attrs: XSetWindowAttributes = std::mem::zeroed();
        attrs.override_redirect = 1;
        attrs.event_mask = ExposureMask | ButtonPressMask | StructureNotifyMask;
        attrs.background_pixel = self.c_popup;
        attrs.cursor = self.cursor_ptr;
        let win = XCreateWindow(
            self.dpy,
            self.root,
            x,
            y,
            w as c_uint,
            h as c_uint,
            1,
            XDefaultDepth(self.dpy, self.screen),
            InputOutput,
            XDefaultVisual(self.dpy, self.screen),
            CWOverrideRedirect | CWEventMask | CWBackPixel | CWBorderPixel | CWCursor,
            &mut attrs,
        );
        XSetWindowBorder(self.dpy, win, self.c_border_active);
        let damage = if self.comp.active {
            let d = XDamageCreate(self.dpy, win, DamageReportNonEmpty);
            if d != 0 {
                self.damage_map.insert(d, win);
            }
            d
        } else {
            0
        };
        self.popup = Some(Popup {
            win,
            x,
            y,
            w,
            h,
            kind: kind.to_string(),
            items,
            sel: 0,
            open: true,
            damage,
            pixmap: 0,
            picture: 0,
        });
        self.comp_windows.entry(win).or_insert((0, 0, 0));
        XMapRaised(self.dpy, win);
        // capture keyboard+pointer while a popup is open (Esc closes)
        XGrabKeyboard(self.dpy, win, 0, GrabModeAsync, GrabModeAsync, CurrentTime);
        XGrabPointer(
            self.dpy,
            win,
            0,
            (ButtonPressMask | ButtonReleaseMask | PointerMotionMask) as c_uint,
            GrabModeAsync,
            GrabModeAsync,
            None_,
            self.cursor_ptr,
            CurrentTime,
        );
        self.draw_popup();
    }

    unsafe fn popup_close(&mut self) {
        if let Some(p) = self.popup.take() {
            XUngrabKeyboard(self.dpy, CurrentTime);
            XUngrabPointer(self.dpy, CurrentTime);
            if p.damage != 0 {
                XDamageDestroy(self.dpy, p.damage);
                self.damage_map.remove(&p.damage);
            }
            if let Some(e) = self.comp_windows.remove(&p.win) {
                if e.2 != 0 {
                    XRenderFreePicture(self.dpy, e.2);
                }
                if e.1 != 0 {
                    XFreePixmap(self.dpy, e.1);
                }
            }
            XDestroyWindow(self.dpy, p.win);
        }
        self.comp.dirty = true;
    }

    unsafe fn draw_popup(&mut self) {
        let p = match &self.popup {
            Some(p) => p.clone_data(),
            None => return,
        };
        let (win, w, items, sel, kind) = (p.0, p.1, p.2, p.3, p.4);
        fill(self.dpy, self.gc, win, self.c_popup, 0, 0, w, 9999);
        let mut y = 6;
        for (i, it) in items.iter().enumerate() {
            if y + 30 > p.5 {
                break;
            }
            if i == sel {
                fill(self.dpy, self.gc, win, self.c_sel, 4, y, w - 8, 30);
                fill(self.dpy, self.gc, win, self.c_accent, 4, y, 3, 30);
            }
            draw_text(self.dpy, self.gc, win, self.font, if i == sel { self.c_text } else { self.c_dim }, 16, y + 20, &it.label);
            if !it.right.is_empty() {
                let tw = text_width(self.font, &it.right);
                draw_text(self.dpy, self.gc, win, self.font, self.c_dim, w - tw - 14, y + 20, &it.right);
            }
            y += 30;
        }
        let _ = kind;
        XFlush(self.dpy);
    }

    unsafe fn popup_open_launcher(&mut self) {
        let items: Vec<MenuItem> = self
            .app_list
            .iter()
            .map(|(n, e, t)| MenuItem {
                label: n.clone(),
                act: Act::Launch(Self::launch_cmd(e, *t)),
                right: if *t { "term".into() } else { String::new() },
            })
            .collect();
        if items.is_empty() {
            eprintln!("minerva-wm: no .desktop applications found in /usr/share/applications");
        }
        self.popup_open("launcher", 6, PANEL_H + 6, 380, items);
    }

    fn wifi_state() -> String {
        if sh_out("command -v nmcli").is_empty() {
            return "n/a".into();
        }
        if sh_out("nmcli radio wifi 2>/dev/null") == "enabled" {
            let ssid = sh_out("nmcli -t -f active,ssid dev wifi 2>/dev/null | awk -F: '/^yes/ {print $2; exit}'");
            if ssid.is_empty() { "on".into() } else { ssid }
        } else {
            "off".into()
        }
    }

    fn bt_state() -> String {
        if sh_out("command -v bluetoothctl").is_empty() {
            return "n/a".into();
        }
        if sh_out("bluetoothctl show 2>/dev/null | grep -q 'Powered: yes' && echo y") == "y" {
            "on".into()
        } else {
            "off".into()
        }
    }

    fn vol_state() -> String {
        let v = sh_out("pactl get-sink-volume @DEFAULT_SINK@ 2>/dev/null | grep -o '[0-9]\\+%' | head -1");
        if !v.is_empty() {
            let m = sh_out("pactl get-sink-mute @DEFAULT_SINK@ 2>/dev/null | grep -o 'yes' | head -1");
            return if m == "yes" { "muted".into() } else { v };
        }
        let a = sh_out("amixer get Master 2>/dev/null | grep -o '\\[[0-9]\\+%\\]' | head -1 | tr -d '[]'");
        if a.is_empty() { "n/a".into() } else { a }
    }

    fn bri_state() -> String {
        let cur = sh_out("cat /sys/class/backlight/*/brightness 2>/dev/null | head -1");
        let max = sh_out("cat /sys/class/backlight/*/max_brightness 2>/dev/null | head -1");
        match (cur.parse::<i64>(), max.parse::<i64>()) {
            (Ok(c), Ok(m)) if m > 0 => format!("{}%", (c * 100 / m).clamp(0, 100)),
            _ => "n/a".into(),
        }
    }

    unsafe fn popup_open_qs(&mut self) {
        let recording = {
            let rt = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
            let pidf = format!("{}/minerva/wm-record.pid", rt);
            fs::read_to_string(&pidf)
                .map(|s| {
                    s.trim()
                        .parse::<i32>()
                        .map(|p| p > 0 && std::path::Path::new(&format!("/proc/{}", p)).exists())
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        };
        let comp_state = if self.comp.active { "on" } else { "off" };
        let items = vec![
            MenuItem { label: "Wi-Fi".into(), act: Act::ToggleWifi, right: Self::wifi_state() },
            MenuItem { label: "Bluetooth".into(), act: Act::ToggleBt, right: Self::bt_state() },
            MenuItem { label: "Volume +".into(), act: Act::VolUp, right: Self::vol_state() },
            MenuItem { label: "Volume -".into(), act: Act::VolDown, right: String::new() },
            MenuItem { label: "Mute".into(), act: Act::VolMute, right: String::new() },
            MenuItem { label: "Brightness +".into(), act: Act::BriUp, right: Self::bri_state() },
            MenuItem { label: "Brightness -".into(), act: Act::BriDown, right: String::new() },
            MenuItem {
                label: if recording { "Stop recording".into() } else { "Record screen".into() },
                act: Act::Record,
                right: if recording { "REC".into() } else { String::new() },
            },
            MenuItem { label: "Compositor".into(), act: Act::Nothing, right: comp_state.into() },
            MenuItem { label: "Lock".into(), act: Act::Lock, right: String::new() },
            MenuItem { label: "Power...".into(), act: Act::OpenPower, right: String::new() },
        ];
        let w = 320;
        let x = self.sw - w - 8;
        self.popup_open("qs", x, PANEL_H + 6, w, items);
    }

    unsafe fn popup_open_power(&mut self) {
        let items = vec![
            MenuItem { label: "Log out".into(), act: Act::Logout, right: String::new() },
            MenuItem { label: "Reboot".into(), act: Act::Reboot, right: String::new() },
            MenuItem { label: "Power off".into(), act: Act::Poweroff, right: String::new() },
        ];
        let w = 220;
        let x = self.sw - w - 8;
        self.popup_open("power", x, PANEL_H + 6, w, items);
    }

    unsafe fn popup_open_root_menu(&mut self, x: c_int, y: c_int) {
        let mut items = vec![
            MenuItem { label: "Terminal".into(), act: Act::OpenTerminal, right: "Ctrl+Alt+T".into() },
            MenuItem { label: "Applications...".into(), act: Act::OpenLauncher, right: "Super".into() },
            MenuItem { label: "Show desktop".into(), act: Act::ShowDesktop, right: "Super+D".into() },
            MenuItem { label: "System menu...".into(), act: Act::OpenQs, right: String::new() },
            MenuItem { label: "About Minerva".into(), act: Act::OpenAbout, right: String::new() },
        ];
        for i in 0..NDESKS {
            items.push(MenuItem {
                label: format!("Workspace {}", i + 1),
                act: Act::Desk(i as i64),
                right: if i == self.desk { "*".into() } else { String::new() },
            });
        }
        self.popup_open("root", x.min(self.sw - 260), y.min(self.sh - 300), 250, items);
    }

    unsafe fn popup_open_about(&mut self) {
        let items = vec![
            MenuItem { label: format!("Debian Minerva {}", VERSION), act: Act::Nothing, right: String::new() },
            MenuItem { label: "Native X11 WM + compositor (Rust)".into(), act: Act::Nothing, right: String::new() },
            MenuItem { label: format!("compositor: {}", if self.comp.active { "active" } else { "inactive" }), act: Act::Nothing, right: String::new() },
            MenuItem { label: format!("clients: {} desk {}", self.clients.len(), self.desk + 1), act: Act::Nothing, right: String::new() },
            MenuItem { label: "MIT licensed - no Google components".into(), act: Act::Nothing, right: String::new() },
        ];
        self.popup_open("about", (self.sw - 420) / 2, (self.sh - 240) / 2, 420, items);
    }

    /// returns Some(act) when a popup row was clicked / activated
    unsafe fn popup_click(&mut self, win: Window, _x: c_int, y: c_int, select_only: bool) -> Option<Act> {
        let p = self.popup.as_ref()?;
        if p.win != win {
            return None;
        }
        let idx = ((y - 6) / 30) as usize;
        if y < 6 || idx >= p.items.len() {
            return None;
        }
        if let Some(p) = self.popup.as_mut() {
            p.sel = idx;
        }
        let act = if select_only {
            None
        } else {
            self.popup.as_ref().map(|p| p.items[idx].act.clone())
        };
        self.draw_popup();
        act
    }

    unsafe fn popup_key(&mut self, keysym: KeySym) -> Option<Act> {
        let n = self.popup.as_ref().map(|p| p.items.len()).unwrap_or(0);
        match keysym {
            XK_Escape => {
                self.popup_close();
                None
            }
            XK_Up => {
                if n > 0 {
                    if let Some(p) = self.popup.as_mut() {
                        p.sel = (p.sel + n - 1) % n;
                    }
                    self.draw_popup();
                }
                None
            }
            XK_Down | XK_Tab => {
                if n > 0 {
                    if let Some(p) = self.popup.as_mut() {
                        p.sel = (p.sel + 1) % n;
                    }
                    self.draw_popup();
                }
                None
            }
            XK_Return => {
                let act = self.popup.as_ref().and_then(|p| p.items.get(p.sel)).map(|i| i.act.clone());
                if act.is_some() {
                    self.popup_close();
                }
                act
            }
            _ => None,
        }
    }
}

impl Popup {
    /// (win, w, items, sel, kind, h) snapshot so draw_popup can use &mut self
    fn clone_data(&self) -> (Window, c_int, Vec<MenuItem>, usize, String, c_int) {
        (self.win, self.w, self.items.clone(), self.sel, self.kind.clone(), self.h)
    }
}

impl Clone for MenuItem {
    fn clone(&self) -> MenuItem {
        MenuItem {
            label: self.label.clone(),
            act: self.act.clone(),
            right: self.right.clone(),
        }
    }
}

// ============================================================================
// compositor — Composite(redirect) + Damage + RENDER, painted onto the composite
// overlay window. Same architecture as xcompmgr-class compositors. Falls back
// to direct server painting when any extension is missing (loudly, on stderr).
// ============================================================================

impl Wm {
    unsafe fn compositor_init(&mut self, no_composite: bool) {
        if no_composite {
            eprintln!("minerva-wm: compositing disabled by --no-composite");
            return;
        }
        let mut eb = 0;
        let mut er = 0;
        let mut maj = 0;
        let mut min = 0;
        if XFixesQueryExtension(self.dpy, &mut eb, &mut er) == 0 {
            eprintln!("minerva-wm: XFIXES extension missing — compositor disabled (direct painting)");
            return;
        }
        let _ = XFixesQueryVersion(self.dpy, &mut maj, &mut min);
        maj = 0;
        min = 0;
        if XDamageQueryExtension(self.dpy, &mut eb, &mut er) == 0 {
            eprintln!("minerva-wm: DAMAGE extension missing — compositor disabled (direct painting)");
            return;
        }
        let _ = XDamageQueryVersion(self.dpy, &mut maj, &mut min);
        self.comp.damage_event = eb; // damage notify event base
        maj = 0;
        min = 0;
        if XCompositeQueryExtension(self.dpy, &mut eb, &mut er) == 0 {
            eprintln!("minerva-wm: COMPOSITE extension missing — compositor disabled (direct painting)");
            return;
        }
        let _ = XCompositeQueryVersion(self.dpy, &mut maj, &mut min);
        if maj == 0 && min < 2 {
            eprintln!("minerva-wm: COMPOSITE too old (0.{}) — compositor disabled", min);
            return;
        }
        maj = 0;
        min = 0;
        if XRenderQueryExtension(self.dpy, &mut eb, &mut er) == 0 {
            eprintln!("minerva-wm: RENDER extension missing — compositor disabled (direct painting)");
            return;
        }
        let _ = XRenderQueryVersion(self.dpy, &mut maj, &mut min);
        let fmt = XRenderFindVisualFormat(self.dpy, XDefaultVisual(self.dpy, self.screen));
        if fmt.is_null() {
            eprintln!("minerva-wm: no RENDER format for the root visual — compositor disabled");
            return;
        }
        clear_error();
        let overlay = XCompositeGetOverlayWindow(self.dpy, self.root);
        XSync(self.dpy, 0);
        if overlay == 0 || error_seen() != 0 {
            eprintln!("minerva-wm: could not acquire the composite overlay — compositor disabled");
            return;
        }
        // let all input fall through the overlay (empty bounding shape)
        XShapeCombineRectangles(
            self.dpy,
            overlay,
            ShapeBounding,
            0,
            0,
            ptr::null(),
            0,
            ShapeSet,
            Unsorted,
        );
        XSelectInput(self.dpy, overlay, ExposureMask | StructureNotifyMask);
        clear_error();
        XCompositeRedirectSubwindows(self.dpy, self.root, CompositeRedirectManual);
        XSync(self.dpy, 0);
        if error_seen() != 0 {
            eprintln!("minerva-wm: RedirectSubwindows refused (X error {}) — compositor disabled", error_seen());
            XCompositeReleaseOverlayWindow(self.dpy, self.root);
            return;
        }
        let overlay_picture = XRenderCreatePicture(self.dpy, overlay, fmt, 0, ptr::null());
        let bg = XRenderColor {
            red: 0x0b00,
            green: 0x1000,
            blue: 0x2000,
            alpha: 0xffff,
        };
        let root_picture = XRenderCreateSolidFill(self.dpy, &bg);
        let shadow = XRenderColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: (0.32 * 65535.0) as c_ushort,
        };
        let shadow_fill = XRenderCreateSolidFill(self.dpy, &shadow);
        self.comp = Compositor {
            active: true,
            overlay,
            overlay_picture,
            root_picture,
            shadow_fill,
            damage_event: self.comp.damage_event,
            dirty: true,
            fmt,
        };
        eprintln!("minerva-wm: compositor active (COMPOSITE+DAMAGE+RENDER overlay)");
    }

    unsafe fn comp_ensure(&mut self, win: Window) -> (Pixmap, Picture) {
        // returns a cached (pixmap, picture) pair for a redirected window,
        // re-naming the window pixmap when the cache is empty
        if let Some(e) = self.comp_windows.get(&win) {
            if e.1 != 0 && e.2 != 0 {
                return (e.1, e.2);
            }
        }
        clear_error();
        let pm = XCompositeNameWindowPixmap(self.dpy, win);
        XSync(self.dpy, 0);
        if pm == 0 || error_seen() != 0 {
            return (0, 0);
        }
        let pic = XRenderCreatePicture(self.dpy, pm, self.comp.fmt, 0, ptr::null());
        let dmg = self.comp_windows.get(&win).map(|e| e.0).unwrap_or(0);
        self.comp_windows.insert(win, (dmg, pm, pic));
        (pm, pic)
    }

    unsafe fn comp_invalidate(&mut self, win: Window) {
        if let Some(e) = self.comp_windows.get_mut(&win) {
            if e.2 != 0 {
                XRenderFreePicture(self.dpy, e.2);
                e.2 = 0;
            }
            if e.1 != 0 {
                XFreePixmap(self.dpy, e.1);
                e.1 = 0;
            }
        }
        if let Some(c) = self.clients.get_mut(&win) {
            if c.picture != 0 {
                XRenderFreePicture(self.dpy, c.picture);
                c.picture = 0;
            }
            if c.pixmap != 0 {
                XFreePixmap(self.dpy, c.pixmap);
                c.pixmap = 0;
            }
            if c.child_picture != 0 {
                XRenderFreePicture(self.dpy, c.child_picture);
                c.child_picture = 0;
            }
            if c.child_pixmap != 0 {
                XFreePixmap(self.dpy, c.child_pixmap);
                c.child_pixmap = 0;
            }
        }
        if let Some(p) = self.popup.as_mut() {
            if p.win == win {
                if p.picture != 0 {
                    XRenderFreePicture(self.dpy, p.picture);
                    p.picture = 0;
                }
                if p.pixmap != 0 {
                    XFreePixmap(self.dpy, p.pixmap);
                    p.pixmap = 0;
                }
            }
        }
        self.comp.dirty = true;
    }

    unsafe fn comp_client_ensure(&mut self, win: Window) -> (Pixmap, Picture, Pixmap, Picture) {
        // (frame_pm, frame_pic, child_pm, child_pic)
        if let Some(c) = self.clients.get(&win) {
            if c.pixmap != 0 && c.picture != 0 && c.child_pixmap != 0 && c.child_picture != 0 {
                return (c.pixmap, c.picture, c.child_pixmap, c.child_picture);
            }
        } else {
            return (0, 0, 0, 0);
        }
        self.comp_invalidate(win);
        let (frame, decor) = {
            let c = self.clients.get(&win).unwrap();
            (c.frame, if c.fullscreen || c.transient { 0 } else { DECOR_H })
        };
        clear_error();
        let fpm = XCompositeNameWindowPixmap(self.dpy, frame);
        let cpm = XCompositeNameWindowPixmap(self.dpy, win);
        XSync(self.dpy, 0);
        if error_seen() != 0 || fpm == 0 || cpm == 0 {
            return (0, 0, 0, 0);
        }
        let fpic = XRenderCreatePicture(self.dpy, fpm, self.comp.fmt, 0, ptr::null());
        let cpic = XRenderCreatePicture(self.dpy, cpm, self.comp.fmt, 0, ptr::null());
        let _ = decor;
        if let Some(c) = self.clients.get_mut(&win) {
            c.pixmap = fpm;
            c.picture = fpic;
            c.child_pixmap = cpm;
            c.child_picture = cpic;
        }
        (fpm, fpic, cpm, cpic)
    }

    unsafe fn comp_add_window(&mut self, win: Window) {
        if !self.comp.active {
            return;
        }
        if self.clients.contains_key(&win) {
            // client window: damage on the client AND its frame
            let frame = self.clients.get(&win).map(|c| c.frame).unwrap_or(0);
            if frame != 0 {
                clear_error();
                let fd = XDamageCreate(self.dpy, frame, DamageReportNonEmpty);
                let cd = XDamageCreate(self.dpy, win, DamageReportNonEmpty);
                XSync(self.dpy, 0);
                if error_seen() == 0 {
                    if let Some(c) = self.clients.get_mut(&win) {
                        c.damage = fd;
                        c.child_damage = cd;
                    }
                    self.damage_map.insert(fd, win);
                    self.damage_map.insert(cd, win);
                }
            }
        } else {
            clear_error();
            let d = XDamageCreate(self.dpy, win, DamageReportNonEmpty);
            XSync(self.dpy, 0);
            if error_seen() == 0 && d != 0 {
                self.comp_windows.insert(win, (d, 0, 0));
                self.damage_map.insert(d, win);
            }
        }
        self.comp.dirty = true;
    }

    unsafe fn comp_remove_window(&mut self, win: Window, frame: Window) {
        if !self.comp.active {
            return;
        }
        if let Some(c) = self.clients.get(&win) {
            if c.damage != 0 {
                XDamageDestroy(self.dpy, c.damage);
                self.damage_map.remove(&c.damage);
            }
            if c.child_damage != 0 {
                XDamageDestroy(self.dpy, c.child_damage);
                self.damage_map.remove(&c.child_damage);
            }
        }
        self.comp_invalidate(win);
        self.comp_invalidate(frame);
        if let Some(e) = self.comp_windows.remove(&win) {
            if e.0 != 0 {
                XDamageDestroy(self.dpy, e.0);
                self.damage_map.remove(&e.0);
            }
        }
        if let Some(e) = self.comp_windows.remove(&frame) {
            if e.0 != 0 {
                XDamageDestroy(self.dpy, e.0);
                self.damage_map.remove(&e.0);
            }
        }
        self.comp.dirty = true;
    }

    unsafe fn comp_handle_damage(&mut self, dmg: Damage) {
        if !self.comp.active {
            return;
        }
        XDamageSubtract(self.dpy, dmg, 0, 0);
        self.comp.dirty = true;
    }

    unsafe fn comp_paint(&mut self) {
        if !self.comp.active || self.comp.overlay_picture == 0 {
            return;
        }
        let d = self.dpy;
        let (sw, sh) = (self.sw, self.sh);
        // background
        XRenderComposite(
            d,
            PictOpSrc,
            self.comp.root_picture,
            0,
            self.comp.overlay_picture,
            0,
            0,
            0,
            0,
            0,
            0,
            sw as c_uint,
            sh as c_uint,
        );
        // clients bottom -> top
        let order = self.stack.clone();
        for w in order {
            let (mapped, fullscreen, transient, fx, fy, fw, fh, cw, ch) = {
                match self.clients.get(&w) {
                    Some(c) => {
                        let (fx, fy, fw, fh) = self.frame_geom(w);
                        (c.mapped, c.fullscreen, c.transient, fx, fy, fw as c_int, fh as c_int, c.w as c_int, c.h as c_int)
                    }
                    None => continue,
                }
            };
            if !mapped {
                continue;
            }
            let (_fpm, fpic, _cpm, cpic) = self.comp_client_ensure(w);
            if fpic == 0 {
                continue;
            }
            let decor = if fullscreen || transient { 0 } else { DECOR_H };
            // drop shadow for decorated, non-fullscreen windows
            if decor > 0 && self.comp.shadow_fill != 0 {
                XRenderComposite(
                    d,
                    PictOpOver,
                    self.comp.shadow_fill,
                    0,
                    self.comp.overlay_picture,
                    0,
                    0,
                    0,
                    0,
                    fx + 7,
                    fy + 7,
                    fw as c_uint,
                    fh as c_uint,
                );
            }
            // frame (decorations) then the client itself
            XRenderComposite(
                d,
                PictOpOver,
                fpic,
                0,
                self.comp.overlay_picture,
                0,
                0,
                0,
                0,
                fx,
                fy,
                fw as c_uint,
                fh as c_uint,
            );
            if cpic != 0 {
                XRenderComposite(
                    d,
                    PictOpOver,
                    cpic,
                    0,
                    self.comp.overlay_picture,
                    0,
                    0,
                    0,
                    0,
                    fx + FRAME_BW as c_int,
                    fy + decor + FRAME_BW as c_int,
                    cw as c_uint,
                    ch as c_uint,
                );
            }
        }
        // chrome last (always on top): panel, dock, popup
        for (win, x, y) in [
            (self.panel, 0, 0),
            (self.dock, 0, self.sh - DOCK_H),
        ] {
            if win == 0 {
                continue;
            }
            let e = self.comp_windows.get(&win).cloned().unwrap_or((0, 0, 0));
            let (pm, pic) = if e.1 != 0 && e.2 != 0 {
                (e.1, e.2)
            } else {
                self.comp_ensure(win)
            };
            if pic == 0 || pm == 0 {
                continue;
            }
            let hgt = if win == self.panel { PANEL_H } else { DOCK_H };
            XRenderComposite(
                d,
                PictOpOver,
                pic,
                0,
                self.comp.overlay_picture,
                0,
                0,
                0,
                0,
                x,
                y,
                self.sw as c_uint,
                hgt as c_uint,
            );
        }
        let pdata = self
            .popup
            .as_ref()
            .map(|p| (p.win, p.x, p.y, p.w, p.h));
        if let Some((pwin, px, py, pw, ph)) = pdata {
            let e = self.comp_windows.get(&pwin).cloned().unwrap_or((0, 0, 0));
            let pic = if e.1 != 0 && e.2 != 0 {
                e.2
            } else {
                self.comp_ensure(pwin).1
            };
            if pic != 0 {
                XRenderComposite(
                    d,
                    PictOpOver,
                    pic,
                    0,
                    self.comp.overlay_picture,
                    0,
                    0,
                    0,
                    0,
                    px,
                    py,
                    pw as c_uint,
                    ph as c_uint,
                );
            }
        }
        XFlush(d);
        self.comp.dirty = false;
    }
}

// ============================================================================
// event dispatch
// ============================================================================

pub const LockMask: c_uint = 1 << 1;
pub const Mod2Mask: c_uint = 1 << 4; // NumLock

#[repr(C)]
pub struct XMappingEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub first_keycode: c_int,
    pub request: c_int,
    pub count: c_int,
}

#[link(name = "X11")]
extern "C" {
    pub fn XKeycodeToKeysym(d: *mut Display, keycode: KeyCode, index: c_int) -> KeySym;
    pub fn XRefreshKeyboardMapping(d: *mut Display, ev: *mut XMappingEvent) -> c_int;
}

impl Wm {
    unsafe fn grab_keys(&mut self) {
        if self.smoke {
            return;
        }
        let binds: [(KeySym, c_uint); 16] = [
            (XK_Tab, Mod1Mask),
            (XK_Tab, Mod1Mask | ShiftMask),
            (XK_F4, Mod1Mask),
            (XK_Super_L, 0),
            (XK_Super_R, 0),
            (0x74, ControlMask | Mod1Mask), // t — terminal
            (0x64, Mod4Mask),               // d — show desktop
            (0x71, Mod4Mask),               // q — system menu
            (0x78, Mod4Mask),               // x — power menu
            (0x6c, Mod4Mask),               // l — lock
            (XK_Left, Mod4Mask),
            (XK_Right, Mod4Mask),
            (0x31, Mod4Mask), // 1..4 — workspaces
            (0x32, Mod4Mask),
            (0x33, Mod4Mask),
            (0x34, Mod4Mask),
        ];
        for (sym, mods) in binds.iter() {
            let kc = XKeysymToKeycode(self.dpy, *sym);
            if kc == 0 {
                continue;
            }
            for extra in [0 as c_uint, LockMask, Mod2Mask, LockMask | Mod2Mask] {
                XGrabKey(
                    self.dpy,
                    kc as c_int,
                    mods | extra,
                    self.root,
                    0,
                    GrabModeAsync,
                    GrabModeAsync,
                );
            }
        }
    }

    unsafe fn handle_key(&mut self, ev: &XKeyEvent) {
        let sym = XKeycodeToKeysym(self.dpy, ev.keycode as KeyCode, 0);
        let st = ev.state;
        if self.popup.is_some() {
            if let Some(act) = self.popup_key(sym) {
                self.run_act(act);
            }
            return;
        }
        if sym == XK_Tab && st & Mod1Mask != 0 {
            let dir = if st & ShiftMask != 0 { -1 } else { 1 };
            self.cycle_focus(dir);
        } else if sym == XK_F4 && st & Mod1Mask != 0 {
            self.run_act(Act::Close);
        } else if (sym == XK_Super_L || sym == XK_Super_R)
            && st & (Mod1Mask | ControlMask | Mod4Mask) == 0
        {
            if self.popup.is_some() {
                self.popup_close();
            } else {
                self.popup_open_launcher();
            }
        } else if st & Mod4Mask != 0 {
            let act = match sym {
                0x64 => Act::ShowDesktop,
                0x71 => Act::OpenQs,
                0x78 => Act::OpenPower,
                0x6c => Act::Lock,
                XK_Left => Act::Desk(-1),
                XK_Right => Act::Desk(NDESKS as i64 + 1),
                s @ 0x31..=0x34 => Act::Desk((s - 0x31) as i64),
                _ => Act::Nothing,
            };
            self.run_act(act);
        } else if sym == 0x74 && st & (ControlMask | Mod1Mask) == (ControlMask | Mod1Mask) {
            self.run_act(Act::OpenTerminal);
        }
    }

    unsafe fn handle_button(&mut self, ev: &XButtonEvent) {
        let w = ev.window;
        // popup eats every button while open (pointer is grabbed to it)
        if let Some(p) = self.popup.as_ref() {
            let pwin = p.win;
            if w == pwin {
                let inside = ev.x >= 0 && ev.y >= 0 && ev.x < p.w && ev.y < p.h;
                if !inside {
                    self.popup_close();
                    return;
                }
                if let Some(act) = self.popup_click(pwin, ev.x, ev.y, false) {
                    self.popup_close();
                    self.run_act(act);
                }
                return;
            }
        }
        if w == self.root {
            match ev.button {
                Button3 => self.popup_open_root_menu(ev.x_root, ev.y_root),
                Button2 => self.run_act(Act::ShowDesktop),
                Button1 => {
                    if self.popup.is_some() {
                        self.popup_close();
                    }
                }
                _ => {}
            }
            return;
        }
        if w == self.panel {
            for (x0, x1, act) in self.panel_zones() {
                if ev.x >= x0 && ev.x < x1 {
                    self.run_act(act);
                    return;
                }
            }
            return;
        }
        if w == self.dock {
            if let Some(cwin) = self.dock_zone(ev.x, ev.y) {
                let focused_now = self.focus == cwin && !self.clients.get(&cwin).map(|c| c.iconic).unwrap_or(true);
                if focused_now {
                    self.run_act(Act::Minimize(cwin));
                } else {
                    self.run_act(Act::Focus(cwin));
                }
            }
            return;
        }
        // frame decoration?
        if self.clients.values().any(|c| c.frame == w) {
            let cwin = self.clients.iter().find(|(_, c)| c.frame == w).map(|(k, _)| *k).unwrap_or(0);
            let zone = self.decor_zone(cwin, ev.x, ev.y);
            if self.focus != cwin {
                self.focus_client(cwin, true);
            } else if zone > 0 {
                XRaiseWindow(self.dpy, self.clients.get(&cwin).map(|c| c.frame).unwrap_or(0));
            }
            match zone {
                1 => self.close_client(cwin),
                2 => {
                    let on = self.clients.get(&cwin).map(|c| !c.maximized).unwrap_or(false);
                    self.set_maximized(cwin, on);
                }
                3 => self.set_iconic(cwin, true),
                4 | 5 => {
                    if ev.button == Button1 {
                        if ev.state & Mod1Mask != 0 || zone == 4 {
                            self.interactive_move_resize(cwin, 4, ev);
                        }
                    }
                }
                6 | 7 | 8 => {
                    if ev.button == Button1 {
                        self.interactive_move_resize(cwin, zone, ev);
                    }
                }
                _ => {}
            }
            return;
        }
        // alt+button1/3 anywhere on a client: move/resize (classic WM gesture)
        if ev.state & Mod1Mask != 0 {
            if let Some(cwin) = self.clients.iter().find(|(_, c)| c.win == w).map(|(k, _)| *k) {
                if ev.button == Button1 {
                    self.focus_client(cwin, true);
                    self.interactive_move_resize(cwin, 4, ev);
                } else if ev.button == Button3 {
                    self.focus_client(cwin, true);
                    self.interactive_move_resize(cwin, 6, ev);
                }
            }
        }
    }

    unsafe fn handle_configure_request(&mut self, ev: &XConfigureRequestEvent) {
        let w = ev.window;
        if self.clients.contains_key(&w) {
            let free = {
                let c = self.clients.get(&w).unwrap();
                !c.fullscreen && !c.maximized
            };
            if free {
                if let Some(c) = self.clients.get_mut(&w) {
                    if ev.value_mask & CWX != 0 {
                        c.x = ev.x;
                    }
                    if ev.value_mask & CWY != 0 {
                        c.y = ev.y.max(PANEL_H);
                    }
                    if ev.value_mask & CWWidth != 0 {
                        let mut nw = (ev.width as c_uint).max(c.min_w).max(1);
                        if c.max_w > 0 {
                            nw = nw.min(c.max_w);
                        }
                        c.w = nw;
                    }
                    if ev.value_mask & CWHeight != 0 {
                        let mut nh = (ev.height as c_uint).max(c.min_h).max(1);
                        if c.max_h > 0 {
                            nh = nh.min(c.max_h);
                        }
                        c.h = nh;
                    }
                    if ev.value_mask & CWStackMode != 0 {
                        let fr = c.frame;
                        if ev.detail == Above {
                            XRaiseWindow(self.dpy, fr);
                            self.stack.retain(|x| *x != w);
                            self.stack.push(w);
                        } else if ev.detail == Below {
                            XLowerWindow(self.dpy, fr);
                        }
                    }
                }
            }
            self.apply_frame_geom(w);
            self.ack_configure(w);
            self.comp_invalidate(w);
            if !self.smoke {
                self.draw_dock();
            }
        } else {
            // unmanaged window (pre-manage or override): honor verbatim
            let mut ch: XWindowChanges = std::mem::zeroed();
            ch.x = ev.x;
            ch.y = ev.y;
            ch.width = ev.width;
            ch.height = ev.height;
            ch.border_width = ev.border_width;
            ch.sibling = ev.above;
            ch.stack_mode = ev.detail;
            XConfigureWindow(self.dpy, w, ev.value_mask, &mut ch);
        }
    }

    /// synthetic ConfigureNotify so the client learns its final geometry
    unsafe fn ack_configure(&mut self, win: Window) {
        let (c_x, c_y, c_w, c_h) = match self.clients.get(&win) {
            Some(c) => {
                let decor = if c.fullscreen || c.transient { 0 } else { DECOR_H };
                (
                    FRAME_BW as c_int,
                    decor + FRAME_BW as c_int,
                    c.w as c_int,
                    c.h as c_int,
                )
            }
            None => return,
        };
        let mut e: XEvent = std::mem::zeroed();
        e.configure.type_ = ConfigureNotify;
        e.configure.send_event = 1;
        e.configure.display = self.dpy;
        e.configure.event = win;
        e.configure.window = win;
        e.configure.x = c_x;
        e.configure.y = c_y;
        e.configure.width = c_w;
        e.configure.height = c_h;
        e.configure.border_width = 0;
        e.configure.above = None_;
        e.configure.override_redirect = 0;
        XSendEvent(self.dpy, win, 0, StructureNotifyMask, &mut e);
        XFlush(self.dpy);
    }

    unsafe fn handle_configure_notify(&mut self, ev: &XConfigureEvent) {
        if ev.window == self.root && ev.send_event == 0 {
            // screen geometry changed (XRandR): re-layout chrome
            self.sw = XDisplayWidth(self.dpy, self.screen);
            self.sh = XDisplayHeight(self.dpy, self.screen);
            if self.panel != 0 {
                XMoveResizeWindow(self.dpy, self.panel, 0, 0, self.sw as c_uint, PANEL_H as c_uint);
                self.comp_invalidate(self.panel);
            }
            if self.dock != 0 {
                XMoveResizeWindow(self.dpy, self.dock, 0, self.sh - DOCK_H, self.sw as c_uint, DOCK_H as c_uint);
                self.comp_invalidate(self.dock);
            }
            let wins: Vec<Window> = self.stack.clone();
            for w in wins {
                let fs = self.clients.get(&w).map(|c| c.fullscreen).unwrap_or(false);
                let mx = self.clients.get(&w).map(|c| c.maximized).unwrap_or(false);
                if fs || mx {
                    if let Some(c) = self.clients.get_mut(&w) {
                        if fs {
                            c.w = self.sw as c_uint;
                            c.h = self.sh as c_uint;
                        } else {
                            c.w = self.sw as c_uint;
                            c.h = (self.sh - PANEL_H - DOCK_H) as c_uint;
                        }
                    }
                    self.apply_frame_geom(w);
                    self.comp_invalidate(w);
                }
            }
            self.set_ewmh_root_props();
            self.draw_panel();
            self.draw_dock();
            self.comp.dirty = true;
            return;
        }
        if self.comp.active {
            // a frame or client resized: its cached pixmap is stale
            if let Some(w) = self.clients.iter().find(|(_, c)| c.frame == ev.window).map(|(k, _)| *k) {
                self.comp_invalidate(w);
            } else if self.clients.contains_key(&ev.window) {
                self.comp_invalidate(ev.window);
            } else if ev.window == self.panel || ev.window == self.dock {
                self.comp_invalidate(ev.window);
            }
        }
    }

    unsafe fn handle_property(&mut self, ev: &XPropertyEvent) {
        if ev.window == self.root {
            if ev.atom == self.a.minerva_wm_cmd {
                let cmd = prop_get_string(self.dpy, self.root, self.a.minerva_wm_cmd)
                    .unwrap_or_default();
                XDeleteProperty(self.dpy, self.root, self.a.minerva_wm_cmd);
                let act = match cmd.as_str() {
                    "launcher" => Act::OpenLauncher,
                    "qs" | "settings" => Act::OpenQs,
                    "power" => Act::OpenPower,
                    "about" => Act::OpenAbout,
                    "lock" => Act::Lock,
                    "logout" => Act::Logout,
                    "reboot" => Act::Reboot,
                    "shutdown" => Act::Poweroff,
                    "showdesktop" => Act::ShowDesktop,
                    "overview" => Act::OpenLauncher,
                    _ => Act::Nothing,
                };
                if !matches!(act, Act::Nothing) {
                    if self.popup.is_some() && matches!(act, Act::OpenLauncher) {
                        self.popup_close();
                    } else {
                        self.run_act(act);
                    }
                }
            }
            return;
        }
        if let Some(c) = self.clients.get_mut(&ev.window) {
            if ev.atom == self.a.net_wm_name || ev.atom == self.a.wm_name {
                let t = prop_get_string(self.dpy, ev.window, self.a.net_wm_name)
                    .or_else(|| prop_get_string(self.dpy, ev.window, self.a.wm_name))
                    .unwrap_or_else(|| "Untitled".to_string());
                c.title = t;
                let w = ev.window;
                self.draw_decor(w);
                if !self.smoke {
                    self.draw_panel();
                    self.draw_dock();
                }
            } else if ev.atom == self.a.wm_hints {
                let hints = prop_get_longs(self.dpy, ev.window, self.a.wm_hints);
                if hints.len() >= 2 && (hints[0] & 1) != 0 {
                    c.input_hint = hints[1] != 0;
                }
            }
        }
    }

    unsafe fn handle_event(&mut self, ev: &mut XEvent) {
        let t = ev.any.type_;
        match t {
            MapRequest => {
                let w = ev.map.window;
                let mut attrs: XWindowAttributes = std::mem::zeroed();
                clear_error();
                if XGetWindowAttributes(self.dpy, w, &mut attrs) != 0 && error_seen() == 0 {
                    if attrs.override_redirect == 0 {
                        let (x, y) = (attrs.x, attrs.y);
                        self.manage(
                            w,
                            x,
                            y,
                            attrs.width.max(1) as c_uint,
                            attrs.height.max(1) as c_uint,
                            attrs.border_width as c_uint,
                        );
                    } else {
                        XMapWindow(self.dpy, w);
                    }
                }
            }
            UnmapNotify => {
                let w = ev.unmap.window;
                if let Some(n) = self.expect_unmap.get_mut(&w) {
                    if *n > 0 {
                        *n -= 1;
                        return;
                    }
                }
                if self.clients.contains_key(&w) {
                    self.unmanage(w);
                }
                // frame unmaps (desk switch / iconify) are ours: ignore
            }
            DestroyNotify => {
                let w = ev.destroy.window;
                if self.clients.contains_key(&w) {
                    self.unmanage(w);
                }
            }
            ReparentNotify => {
                let r = ev.reparent;
                if self.clients.contains_key(&r.window) {
                    if let Some(c) = self.clients.get(&r.window) {
                        if c.frame != r.parent {
                            let w = r.window;
                            self.unmanage(w);
                        }
                    }
                }
            }
            ConfigureRequest => self.handle_configure_request(ev.configurereq_ref()),
            ConfigureNotify => self.handle_configure_notify(ev.configure_ref()),
            ButtonPress => {
                let b = *ev.button_ref();
                self.handle_button(&b);
            }
            KeyPress => {
                let k = *ev.key_ref();
                self.handle_key(&k);
            }
            EnterNotify => {
                let cr = *ev.crossing_ref();
                // sloppy focus on normal crossings only (no grab pseudo-events)
                if cr.mode == 0 {
                    if let Some(w) = self.clients.iter().find(|(_, c)| c.win == cr.window || c.frame == cr.window).map(|(k, _)| *k) {
                        if self.focus != w {
                            let iconic = self.clients.get(&w).map(|c| c.iconic).unwrap_or(true);
                            if !iconic {
                                self.focus_client(w, false);
                            }
                        }
                    }
                }
            }
            Expose => {
                let ex = *ev.expose_ref();
                if ex.count != 0 {
                    return;
                }
                if ex.window == self.panel {
                    self.draw_panel();
                } else if ex.window == self.dock {
                    self.draw_dock();
                } else if self.comp.active && ex.window == self.comp.overlay {
                    self.comp.dirty = true;
                } else if self.popup.as_ref().map(|p| p.win == ex.window).unwrap_or(false) {
                    self.draw_popup();
                } else if let Some(w) = self.clients.iter().find(|(_, c)| c.frame == ex.window).map(|(k, _)| *k) {
                    self.draw_decor(w);
                }
            }
            PropertyNotify => {
                let p = *ev.property_ref();
                self.handle_property(&p);
            }
            ClientMessage => {
                let cm = *ev.client_ref();
                if self.smoke
                    && cm.window == self.smoke_win
                    && cm.message_type == self.a.wm_protocols
                    && cm.data.l[0] == self.a.wm_delete_window as c_long
                {
                    XDestroyWindow(self.dpy, self.smoke_win);
                    return;
                }
                self.handle_client_message(&cm);
            }
            MappingNotify => {
                let mut m: XMappingEvent = std::mem::zeroed();
                let src = ev.mapping;
                m.type_ = src.type_;
                m.serial = src.serial;
                m.send_event = src.send_event;
                m.display = src.display;
                m.window = src.window;
                m.first_keycode = src.first_keycode;
                m.request = src.request;
                m.count = src.count;
                XRefreshKeyboardMapping(self.dpy, &mut m);
            }
            SelectionClear => {
                eprintln!("minerva-wm: lost WM selection ownership — another WM took over, exiting");
                self.quit = true;
            }
            _ => {
                // damage events live at comp.damage_event + 0
                if self.comp.active && self.comp.damage_event >= 0 && t == self.comp.damage_event {
                    let d = *ev.damage_ref();
                    self.comp_handle_damage(d.drawable as Damage);
                }
            }
        }
    }

    // ------------------------------------------------------------- main loop
    unsafe fn run(&mut self) -> c_int {
        self.grab_keys();
        let fd = XConnectionNumber(self.dpy);
        let mut pfd = PollFd {
            fd,
            events: POLLIN,
            revents: 0,
        };
        let mut last_refresh = now_secs();
        loop {
            while XPending(self.dpy) > 0 {
                let mut ev: XEvent = std::mem::zeroed();
                XNextEvent(self.dpy, &mut ev);
                self.handle_event(&mut ev);
                if self.quit {
                    break;
                }
            }
            if self.quit {
                break;
            }
            if self.comp.active && self.comp.dirty {
                self.comp_paint();
            }
            self.tick += 1;
            if self.tick % 8 == 0 {
                reap_children();
            }
            let n = now_secs();
            if n.wrapping_sub(last_refresh) >= 20 {
                last_refresh = n;
                self.refresh_panel_state();
                self.draw_panel();
                if self.popup.as_ref().map(|p| p.kind == "qs").unwrap_or(false) {
                    let (x, y) = (self.sw - 328, PANEL_H + 6);
                    self.popup_close();
                    let _ = (x, y);
                    self.popup_open_qs();
                }
            }
            pfd.revents = 0;
            poll(&mut pfd, 1, 250);
        }
        eprintln!("minerva-wm: shutting down cleanly");
        if self.comp.active {
            XCompositeUnredirectSubwindows(self.dpy, self.root, CompositeRedirectManual);
            XCompositeReleaseOverlayWindow(self.dpy, self.root);
        }
        XFlush(self.dpy);
        XCloseDisplay(self.dpy);
        0
    }
}

// union access helpers (kept tiny and explicit; every use site is unsafe)
impl XEvent {
    unsafe fn configurereq_ref(&mut self) -> &mut XConfigureRequestEvent { &mut self.configurereq }
    unsafe fn configure_ref(&mut self) -> &mut XConfigureEvent { &mut self.configure }
    unsafe fn button_ref(&mut self) -> &mut XButtonEvent { &mut self.button }
    unsafe fn key_ref(&mut self) -> &mut XKeyEvent { &mut self.key }
    unsafe fn crossing_ref(&mut self) -> &mut XCrossingEvent { &mut self.crossing }
    unsafe fn expose_ref(&mut self) -> &mut XExposeEvent { &mut self.expose }
    unsafe fn property_ref(&mut self) -> &mut XPropertyEvent { &mut self.property }
    unsafe fn client_ref(&mut self) -> &mut XClientMessageEvent { &mut self.client }
    unsafe fn damage_ref(&mut self) -> &mut XDamageNotifyEvent { &mut self.damage }
    unsafe fn motion_ref(&mut self) -> &mut XMotionEvent { &mut self.motion }
}

// ============================================================================
// smoke test — runs on a bare X display (Xvfb in CI): the WM manages a real
// client window it creates itself and every EWMH path is asserted end to end.
// ============================================================================

unsafe fn ev_msg(
    d: *mut Display,
    dest: Window,
    win: Window,
    type_: Atom,
    data: [c_long; 5],
    mask: c_long,
) {
    let mut ev: XEvent = std::mem::zeroed();
    ev.client.type_ = ClientMessage;
    ev.client.display = d;
    ev.client.window = win;
    ev.client.message_type = type_;
    ev.client.format = 32;
    ev.client.data.l = data;
    XSendEvent(d, dest, 0, mask, &mut ev);
    XFlush(d);
}

const ROOT_MSG_MASK: c_long = SubstructureRedirectMask | SubstructureNotifyMask;

unsafe fn root_msg(wm: &Wm, win: Window, type_: Atom, data: [c_long; 5]) {
    ev_msg(wm.dpy, wm.root, win, type_, data, ROOT_MSG_MASK);
}

unsafe fn pump(wm: &mut Wm, step: &str, pred: fn(&Wm) -> bool) {
    let start = std::time::Instant::now();
    let fd = XConnectionNumber(wm.dpy);
    let mut pfd = PollFd {
        fd,
        events: POLLIN,
        revents: 0,
    };
    loop {
        while XPending(wm.dpy) > 0 {
            let mut ev: XEvent = std::mem::zeroed();
            XNextEvent(wm.dpy, &mut ev);
            wm.smoke_events += 1;
            wm.handle_event(&mut ev);
        }
        if wm.comp.active && wm.comp.dirty {
            wm.comp_paint();
        }
        if pred(wm) {
            wm.smoke_steps.push(step.to_string());
            return;
        }
        if start.elapsed().as_secs_f64() > 5.0 {
            eprintln!(
                "minerva-wm smoke: FAIL at step '{}' (timeout, {} events processed)",
                step, wm.smoke_events
            );
            std::process::exit(1);
        }
        pfd.revents = 0;
        poll(&mut pfd, 1, 100);
    }
}

unsafe fn geom_of(wm: &Wm, win: Window) -> (c_int, c_int, c_uint, c_uint) {
    let mut r: Window = 0;
    let mut x = 0;
    let mut y = 0;
    let mut w = 0;
    let mut h = 0;
    let mut bw = 0;
    let mut depth = 0;
    XGetGeometry(wm.dpy, win, &mut r, &mut x, &mut y, &mut w, &mut h, &mut bw, &mut depth);
    (x, y, w, h)
}

unsafe fn viewable(wm: &Wm, win: Window) -> bool {
    let mut attrs: XWindowAttributes = std::mem::zeroed();
    if XGetWindowAttributes(wm.dpy, win, &mut attrs) == 0 {
        return false;
    }
    attrs.map_state == IsViewable
}

fn smoke_check(cond: bool, msg: &str) {
    if !cond {
        eprintln!("minerva-wm smoke: FAIL — {}", msg);
        std::process::exit(1);
    }
}

fn pred_managed(wm: &Wm) -> bool {
    wm.clients.contains_key(&wm.smoke_win)
}
fn pred_geom(wm: &Wm) -> bool {
    wm.clients
        .get(&wm.smoke_win)
        .map(|c| c.x == 150 && c.y == 160 && c.w == 500 && c.h == 400)
        .unwrap_or(false)
}
fn pred_focus(wm: &Wm) -> bool {
    wm.focus == wm.smoke_win
}
fn pred_fs(wm: &Wm) -> bool {
    wm.clients.get(&wm.smoke_win).map(|c| c.fullscreen).unwrap_or(false)
}
fn pred_nofs(wm: &Wm) -> bool {
    wm.clients
        .get(&wm.smoke_win)
        .map(|c| !c.fullscreen && c.x == 150 && c.y == 160)
        .unwrap_or(false)
}
fn pred_max(wm: &Wm) -> bool {
    wm.clients
        .get(&wm.smoke_win)
        .map(|c| c.maximized && c.y == PANEL_H)
        .unwrap_or(false)
}
fn pred_nomax(wm: &Wm) -> bool {
    wm.clients
        .get(&wm.smoke_win)
        .map(|c| !c.maximized && c.x == 150 && c.y == 160)
        .unwrap_or(false)
}
fn pred_iconic(wm: &Wm) -> bool {
    wm.clients
        .get(&wm.smoke_win)
        .map(|c| c.iconic && !c.mapped)
        .unwrap_or(false)
}
fn pred_shown(wm: &Wm) -> bool {
    wm.clients
        .get(&wm.smoke_win)
        .map(|c| !c.iconic && c.mapped)
        .unwrap_or(false)
}
fn pred_desk2_hidden(wm: &Wm) -> bool {
    wm.desk == 2
        && wm
            .clients
            .get(&wm.smoke_win)
            .map(|c| !c.mapped)
            .unwrap_or(true)
}
fn pred_desk0_shown(wm: &Wm) -> bool {
    wm.desk == 0
        && wm
            .clients
            .get(&wm.smoke_win)
            .map(|c| c.mapped)
            .unwrap_or(false)
}
fn pred_closed(wm: &Wm) -> bool {
    !wm.clients.contains_key(&wm.smoke_win)
}

unsafe fn run_smoke(wm: &mut Wm) {
    eprintln!("minerva-wm smoke: starting (display {}x{})", wm.sw, wm.sh);
    // 1 — create a real client window and let the WM manage it
    let win = XCreateSimpleWindow(wm.dpy, wm.root, 100, 100, 400, 300, 0, wm.c_border, wm.c_bg);
    XStoreName(wm.dpy, win, cstr("minerva smoke").as_ptr());
    prop_set_str(wm.dpy, win, wm.a.net_wm_name, true, "minerva smoke");
    prop_set_atoms(wm.dpy, win, wm.a.wm_protocols, &[wm.a.wm_delete_window]);
    wm.smoke_win = win;
    XMapWindow(wm.dpy, win);
    pump(wm, "MapRequest -> managed with frame", pred_managed);

    // 2 — ConfigureRequest honored, frame geometry = client + decoration
    XMoveResizeWindow(wm.dpy, win, 150, 160, 500, 400);
    pump(wm, "ConfigureRequest honored", pred_geom);
    let frame = wm.clients.get(&win).map(|c| c.frame).unwrap_or(0);
    let (fx, fy, fw, fh) = geom_of(wm, frame);
    smoke_check(
        fx == 150 - FRAME_BW as c_int
            && fy == 160 - DECOR_H - FRAME_BW as c_int
            && fw == 500 + 2 * FRAME_BW
            && fh == 400 + DECOR_H as c_uint + 2 * FRAME_BW,
        &format!(
            "frame geometry {}x{}+{}+{} matches client+decor",
            fw, fh, fx, fy
        ),
    );

    // 3 — activation + input focus
    root_msg(wm, win, wm.a.net_active_window, [win as c_long, CurrentTime as c_long, 0, 0, 0]);
    pump(wm, "NET_ACTIVE_WINDOW focuses client", pred_focus);
    let mut foc: Window = 0;
    let mut rev = 0;
    XGetInputFocus(wm.dpy, &mut foc, &mut rev);
    smoke_check(foc == win, "X input focus follows activation");

    // 4 — fullscreen add/remove
    root_msg(
        wm,
        win,
        wm.a.net_wm_state,
        [1, wm.a.net_wm_state_fullscreen as c_long, 0, 0, 0],
    );
    pump(wm, "NET_WM_STATE FULLSCREEN add", pred_fs);
    let (gx, gy, gw, gh) = geom_of(wm, frame);
    smoke_check(
        gx == 0 && gy == 0 && gw == wm.sw as c_uint && gh == wm.sh as c_uint,
        "fullscreen frame covers the whole screen",
    );
    root_msg(
        wm,
        win,
        wm.a.net_wm_state,
        [0, wm.a.net_wm_state_fullscreen as c_long, 0, 0, 0],
    );
    pump(wm, "NET_WM_STATE FULLSCREEN remove restores geometry", pred_nofs);

    // 5 — maximize add/remove
    root_msg(
        wm,
        win,
        wm.a.net_wm_state,
        [
            1,
            wm.a.net_wm_state_maximized_vert as c_long,
            wm.a.net_wm_state_maximized_horz as c_long,
            0,
            0,
        ],
    );
    pump(wm, "NET_WM_STATE MAXIMIZED add (workarea-aware)", pred_max);
    root_msg(
        wm,
        win,
        wm.a.net_wm_state,
        [
            0,
            wm.a.net_wm_state_maximized_vert as c_long,
            wm.a.net_wm_state_maximized_horz as c_long,
            0,
            0,
        ],
    );
    pump(wm, "NET_WM_STATE MAXIMIZED remove restores geometry", pred_nomax);

    // 6 — iconify / restore
    root_msg(wm, win, wm.a.wm_change_state, [IconicState, 0, 0, 0, 0]);
    pump(wm, "WM_CHANGE_STATE iconic minimizes", pred_iconic);
    smoke_check(!viewable(wm, frame), "iconified frame is not viewable");
    root_msg(wm, win, wm.a.net_active_window, [win as c_long, CurrentTime as c_long, 0, 0, 0]);
    pump(wm, "NET_ACTIVE_WINDOW restores iconified client", pred_shown);
    smoke_check(viewable(wm, frame), "restored frame is viewable again");

    // 7 — workspace switch hides/reshows the client
    root_msg(wm, wm.root, wm.a.net_current_desktop, [2, 0, 0, 0, 0]);
    pump(wm, "NET_CURRENT_DESKTOP 2 hides desk-0 client", pred_desk2_hidden);
    root_msg(wm, wm.root, wm.a.net_current_desktop, [0, 0, 0, 0, 0]);
    pump(wm, "NET_CURRENT_DESKTOP 0 shows client again", pred_desk0_shown);

    // 8 — close protocol round trip
    root_msg(wm, win, wm.a.net_close_window, [win as c_long, CurrentTime as c_long, 0, 0, 0]);
    pump(wm, "NET_CLOSE_WINDOW -> WM_DELETE_WINDOW -> destroyed", pred_closed);

    eprintln!(
        "minerva-wm smoke: OK — {} steps passed, {} events processed",
        wm.smoke_steps.len(),
        wm.smoke_events
    );
    for s in &wm.smoke_steps {
        eprintln!("  ok: {}", s);
    }
    XFlush(wm.dpy);
    XCloseDisplay(wm.dpy);
    std::process::exit(0);
}

// ============================================================================
// entry point
// ============================================================================

fn usage() {
    eprintln!(
        "minerva-wm {} — native X11 window manager + compositor for Debian Minerva\n\nusage:\n  minerva-wm                 run the WM on $DISPLAY\n  minerva-wm --no-composite  run without the COMPOSITE/DAMAGE/RENDER pass\n  minerva-wm --smoke         end-to-end self-test on a bare X display\n  minerva-wm --version       print version",
        VERSION
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut smoke = false;
    let mut no_comp = false;
    for a in args.iter().skip(1) {
        match a.as_str() {
            "--version" => {
                println!("minerva-wm {}", VERSION);
                return;
            }
            "--help" => {
                usage();
                return;
            }
            "--smoke" => smoke = true,
            "--no-composite" => no_comp = true,
            other => {
                eprintln!("minerva-wm: unknown argument '{}'", other);
                usage();
                std::process::exit(2);
            }
        }
    }
    unsafe {
        XSetErrorHandler(Some(wm_error_handler));
        let dpy = XOpenDisplay(ptr::null());
        if dpy.is_null() {
            eprintln!(
                "minerva-wm: cannot open X display '{}' — is DISPLAY set and the X server running?",
                std::env::var("DISPLAY").unwrap_or_default()
            );
            std::process::exit(1);
        }
        let screen = XDefaultScreen(dpy);
        let root = XRootWindow(dpy, screen);
        let a = init_atoms(dpy);
        if !take_wm_ownership(dpy, root, &a) {
            std::process::exit(2);
        }
        let mut wm = Wm::new(dpy, no_comp, smoke);
        wm.chrome_init();
        wm.scan_existing();
        if smoke {
            run_smoke(&mut wm);
            return;
        }
        let rc = wm.run();
        std::process::exit(rc);
    }
}
