use std::fmt;

use anyhow::{Context, Result};
use gdk::prelude::MonitorExt;
use gdk::Monitor;
use glib::Cast;
use gtk::prelude::{GtkWindowExt, WidgetExt};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, PropMode};
use x11rb::rust_connection::{DefaultStream, RustConnection};

use crate::window::{Window, get_gdk_monitor};

x11rb::atom_manager! {
    pub AtomCollection: AtomCollectionCookie {
        _NET_WM_WINDOW_TYPE,
        _NET_WM_WINDOW_TYPE_NORMAL,
        _NET_WM_WINDOW_TYPE_DOCK,
        _NET_WM_WINDOW_TYPE_DIALOG,
        _NET_WM_WINDOW_TYPE_TOOLBAR,
        _NET_WM_WINDOW_TYPE_UTILITY,
        _NET_WM_WINDOW_TYPE_DESKTOP,
        _NET_WM_WINDOW_TYPE_NOTIFICATION,
        _NET_WM_STATE,
        _NET_WM_STATE_STICKY,
        _NET_WM_STATE_ABOVE,
        _NET_WM_STATE_BELOW,
        _NET_WM_NAME,
        _NET_WM_STRUT,
        _NET_WM_STRUT_PARTIAL,
        WM_NAME,
        UTF8_STRING,
        COMPOUND_TEXT,
        CARDINAL,
        ATOM,
        WM_CLASS,
        STRING,
    }
}

pub struct X11BackendConnection {
    conn: RustConnection<DefaultStream>,
    atoms: AtomCollection,
}

impl X11BackendConnection {
    pub fn new() -> Result<Self> {
        let (conn, _) = RustConnection::connect(None)?;
        let atoms = AtomCollection::new(&conn)?.reply()?;
        Ok(X11BackendConnection { conn, atoms })
    }

    pub fn set_xprops_for(&self, window: &Window, monitor: Monitor) -> Result<()> {
        let monitor_rect = monitor.geometry();
        let scale_factor = monitor.scale_factor() as u32;
        let gdk_window = window
            .window()
            .context("Couldn't get gdk window from gtk window")?;
        let win_id = gdk_window
            .downcast_ref::<gdkx11::X11Window>()
            .context("Failed to get x11 window for gtk window")?
            .xid() as u32;

        let mon_x = scale_factor * monitor_rect.x() as u32;
        let mon_y = scale_factor * monitor_rect.y() as u32;
        let mon_end_x = scale_factor * (monitor_rect.x() + monitor_rect.width()) as u32 - 1u32;

        // let dist = match strut_def.side {
        //     Side::Left | Side::Right => {
        //         strut_def.distance.pixels_relative_to(monitor_rect.width()) as u32
        //     }
        //     Side::Top | Side::Bottom => {
        //         strut_def.distance.pixels_relative_to(monitor_rect.height()) as u32
        //     }
        // };
        let dist: u32 = 0;

        // don't question it,.....
        // it's how the X gods want it to be.
        // left, right, top, bottom, left_start_y, left_end_y, right_start_y, right_end_y, top_start_x, top_end_x, bottom_start_x, bottom_end_x
        #[rustfmt::skip]
        let strut_list: Vec<u8> = match Side::Top {
            // Side::Left => vec![
            //     dist + mon_x, 0,   0, 0,
            //     mon_x, mon_end_y,  0, 0,
            //     0, 0,              0, 0],
            // Side::Right => vec![
            //     0, root_window_geometry.width as u32 - mon_end_x + dist,   0, 0,
            //     0, 0,                                                      mon_x, mon_end_y,
            //     0, 0,                                                      0, 0],
            Side::Top => vec![
                0, 0,              dist + mon_y, 0,
                0, 0,              0, 0,
                mon_x, mon_end_x,  0, 0],
            // Side::Bottom => vec![
            //     0, 0,   0, root_window_geometry.height as u32 - mon_end_y + dist,
            //     0, 0,   0, 0,
            //     0, 0,   mon_x, mon_end_x],
            // This should never happen but if it does the window will be anchored on the
            // right of the screen
        }
        .iter()
        .flat_map(|x| x.to_le_bytes().to_vec())
        .collect();

        self.conn
            .change_property(
                PropMode::REPLACE,
                win_id,
                self.atoms._NET_WM_STRUT,
                self.atoms.CARDINAL,
                32,
                4,
                &strut_list[0..16],
            )?
            .check()?;
        self.conn
            .change_property(
                PropMode::REPLACE,
                win_id,
                self.atoms._NET_WM_STRUT_PARTIAL,
                self.atoms.CARDINAL,
                32,
                12,
                &strut_list,
            )?
            .check()?;

        // let ty = match window_init.backend_options.x11.window_type {
        //     X11WindowType::Dock => self.atoms._NET_WM_WINDOW_TYPE_DOCK,
        //     X11WindowType::Normal => self.atoms._NET_WM_WINDOW_TYPE_NORMAL,
        //     X11WindowType::Dialog => self.atoms._NET_WM_WINDOW_TYPE_DIALOG,
        //     X11WindowType::Toolbar => self.atoms._NET_WM_WINDOW_TYPE_TOOLBAR,
        //     X11WindowType::Utility => self.atoms._NET_WM_WINDOW_TYPE_UTILITY,
        //     X11WindowType::Desktop => self.atoms._NET_WM_WINDOW_TYPE_DESKTOP,
        //     X11WindowType::Notification => self.atoms._NET_WM_WINDOW_TYPE_NOTIFICATION,
        // };
        let ty =
            // X11WindowType::Dock => 
        self.atoms._NET_WM_WINDOW_TYPE_DOCK;
        // X11WindowType::Normal =>
        // self.atoms._NET_WM_WINDOW_TYPE_NORMAL
        // X11WindowType::Dialog =>
        // self.atoms._NET_WM_WINDOW_TYPE_DIALOG
        // X11WindowType::Toolbar =>
        // self.atoms._NET_WM_WINDOW_TYPE_TOOLBAR
        // X11WindowType::Utility =>
        // self.atoms._NET_WM_WINDOW_TYPE_UTILITY
        // X11WindowType::Desktop =>
        // self.atoms._NET_WM_WINDOW_TYPE_DESKTOP
        // X11WindowType::Notification =>
        // self.atoms._NET_WM_WINDOW_TYPE_NOTIFICATION

        // TODO possibly support setting multiple window types
        x11rb::wrapper::ConnectionExt::change_property32(
            &self.conn,
            PropMode::REPLACE,
            win_id,
            self.atoms._NET_WM_WINDOW_TYPE,
            self.atoms.ATOM,
            &[ty],
        )?
        .check()?;

        self.conn
            .flush()
            .context("Failed to send requests to X server")
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Side {
    Top,
}

pub fn a(pos: (i32, i32), monitor_geometry: gdk::Rectangle) -> crate::window::Window {
    let geometry = WindowGeometry {
        anchor_point: AnchorPoint {
            x: AnchorAlignment::START,
            y: AnchorAlignment::START,
        },
        offset: Coords { x: pos.0, y: pos.1 },
        size: Coords { x: 200, y: 50 },
    };
    let (actual_window_rect, x, y) = {
        let rect = get_window_rectangle(geometry, monitor_geometry);
        (Some(rect), rect.x(), rect.y())
    };

    let window_type = if true {
        gtk::WindowType::Popup
    } else {
        gtk::WindowType::Toplevel
    };
    let w = crate::window::Window::new(window_type, x, y);
    w.set_resizable(false);
    w.set_keep_above(true);
    w.set_keep_below(false);
    if true {
        w.stick();
    } else {
        w.unstick();
    }

    w.set_title("Overlay");
    w.set_position(gtk::WindowPosition::None);
    w.set_gravity(gdk::Gravity::Center);

    if let Some(actual_window_rect) = actual_window_rect {
        w.set_size_request(actual_window_rect.width(), actual_window_rect.height());
        w.set_default_size(actual_window_rect.width(), actual_window_rect.height());
    }
    w.set_decorated(false);
    w.set_skip_taskbar_hint(true);
    w.set_skip_pager_hint(true);

    // run on_screen_changed to set the visual correctly initially.
    on_screen_changed(&w, None);
    w.connect_screen_changed(on_screen_changed);
    return w;
}

pub fn b(w: crate::window::Window, monitor_geometry: gdk::Rectangle, geometry: WindowGeometry) {
    let _ = apply_window_position(geometry, monitor_geometry, &w);
    if true {
        w.connect_configure_event(move |window, _| {
            let _ = apply_window_position(geometry, monitor_geometry, window);
            false
        });
    }
    let backend = crate::x11::X11BackendConnection::new().unwrap();
    backend.set_xprops_for(&w, get_gdk_monitor()).unwrap();
}

fn apply_window_position(
    mut window_geometry: WindowGeometry,
    monitor_geometry: gdk::Rectangle,
    window: &Window,
) {
    let gdk_window = window
        .window()
        .expect("Failed to get gdk window from gtk window");
    window_geometry.size = Coords::from_pixels(window.size());
    let actual_window_rect = get_window_rectangle(window_geometry, monitor_geometry);

    let gdk_origin = gdk_window.origin();

    if actual_window_rect.x() != gdk_origin.1 || actual_window_rect.y() != gdk_origin.2 {
        gdk_window.move_(actual_window_rect.x(), actual_window_rect.y());
    }
}

fn on_screen_changed(window: &Window, _old_screen: Option<&gdk::Screen>) {
    let visual = gtk::prelude::GtkWindowExt::screen(window).and_then(|screen| {
        screen
            .rgba_visual()
            .filter(|_| screen.is_composited())
            .or_else(|| screen.system_visual())
    });
    window.set_visual(visual.as_ref());
}

pub fn get_window_rectangle(
    geometry: WindowGeometry,
    screen_rect: gdk::Rectangle,
) -> gdk::Rectangle {
    let (offset_x, offset_y) = geometry.offset.relative_to();
    let (width, height) = geometry.size.relative_to();
    let x = screen_rect.x()
        + offset_x
        + geometry
            .anchor_point
            .x
            .alignment_to_coordinate(width, screen_rect.width());
    let y = screen_rect.y()
        + offset_y
        + geometry
            .anchor_point
            .y
            .alignment_to_coordinate(height, screen_rect.height());
    gdk::Rectangle::new(x, y, width, height)
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct WindowGeometry {
    pub anchor_point: AnchorPoint,
    pub offset: Coords,
    pub size: Coords,
}

impl std::fmt::Display for WindowGeometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{} ({})", self.offset, self.size, self.anchor_point)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct AnchorPoint {
    pub x: AnchorAlignment,
    pub y: AnchorAlignment,
}

impl std::fmt::Display for AnchorPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use AnchorAlignment::*;
        match (self.x, self.y) {
            (CENTER, CENTER) => write!(f, "center"),
            (x, y) => write!(
                f,
                "{} {}",
                match x {
                    START => "left",
                    CENTER => "center",
                    END => "right",
                },
                match y {
                    START => "top",
                    CENTER => "center",
                    END => "bottom",
                }
            ),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AnchorAlignment {
    START,
    CENTER,
    END,
}

impl Default for AnchorAlignment {
    fn default() -> Self {
        Self::START
    }
}

impl AnchorAlignment {
    pub fn alignment_to_coordinate(&self, size_inner: i32, size_container: i32) -> i32 {
        match self {
            AnchorAlignment::START => 0,
            AnchorAlignment::CENTER => (size_container / 2) - (size_inner / 2),
            AnchorAlignment::END => size_container - size_inner,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub struct Coords {
    pub x: i32,
    pub y: i32,
}

impl fmt::Debug for Coords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CoordsWithUnits({}, {})", self.x, self.y)
    }
}

impl fmt::Display for Coords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Coords {
    pub fn from_pixels((x, y): (i32, i32)) -> Self {
        Coords { x, y }
    }

    /// resolve the possibly relative coordinates relative to a given containers size
    pub fn relative_to(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum NumWithUnit {
    Pixels(i32),
}

impl fmt::Display for NumWithUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pixels(p) => write!(f, "{p}px"),
        }
    }
}

impl Default for NumWithUnit {
    fn default() -> Self {
        Self::Pixels(0)
    }
}
