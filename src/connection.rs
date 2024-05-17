use anyhow::{Context, Result};
use gdk::prelude::MonitorExt;
use gdk::Monitor;
use glib::Cast;
use gtk::prelude::WidgetExt;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, PropMode};
use x11rb::rust_connection::{DefaultStream, RustConnection};

use crate::window::Window;
use crate::NumWithUnit;

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
    root_window: u32,
    atoms: AtomCollection,
}

impl X11BackendConnection {
    pub fn new() -> Result<Self> {
        let (conn, screen_num) = RustConnection::connect(None)?;
        let screen = conn.setup().roots[screen_num].clone();
        let atoms = AtomCollection::new(&conn)?.reply()?;
        Ok(X11BackendConnection {
            conn,
            root_window: screen.root,
            atoms,
        })
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
        let root_window_geometry = self.conn.get_geometry(self.root_window)?.reply()?;

        let mon_x = scale_factor * monitor_rect.x() as u32;
        let mon_y = scale_factor * monitor_rect.y() as u32;
        let mon_end_x = scale_factor * (monitor_rect.x() + monitor_rect.width()) as u32 - 1u32;
        let mon_end_y = scale_factor * (monitor_rect.y() + monitor_rect.height()) as u32 - 1u32;

        // let dist = match strut_def.side {
        //     Side::Left | Side::Right => {
        //         strut_def.distance.pixels_relative_to(monitor_rect.width()) as u32
        //     }
        //     Side::Top | Side::Bottom => {
        //         strut_def.distance.pixels_relative_to(monitor_rect.height()) as u32
        //     }
        // };
        let dist = NumWithUnit::Pixels(0).pixels_relative_to(monitor_rect.width()) as u32;

        // don't question it,.....
        // it's how the X gods want it to be.
        // left, right, top, bottom, left_start_y, left_end_y, right_start_y, right_end_y, top_start_x, top_end_x, bottom_start_x, bottom_end_x
        #[rustfmt::skip]
        let strut_list: Vec<u8> = match Side::Top {
            Side::Left => vec![
                dist + mon_x, 0,   0, 0,
                mon_x, mon_end_y,  0, 0,
                0, 0,              0, 0],
            Side::Right => vec![
                0, root_window_geometry.width as u32 - mon_end_x + dist,   0, 0,
                0, 0,                                                      mon_x, mon_end_y,
                0, 0,                                                      0, 0],
            Side::Top => vec![
                0, 0,              dist + mon_y, 0,
                0, 0,              0, 0,
                mon_x, mon_end_x,  0, 0],
            Side::Bottom => vec![
                0, 0,   0, root_window_geometry.height as u32 - mon_end_y + dist,
                0, 0,   0, 0,
                0, 0,   mon_x, mon_end_x],
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
    Left,
    Right,
    Bottom,
}
