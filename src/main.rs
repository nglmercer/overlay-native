mod connection;
mod window;

extern crate gdkx11;
extern crate x11rb;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use gdk::prelude::MonitorExt;
use gdk::Monitor;
use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};
use window::Window;

use crate::connection::X11BackendConnection;

fn main() {
    gtk::init().unwrap();

    let count = 5;
    let monitor_geometry = get_gdk_monitor().geometry();
    let gap = (monitor_geometry.width() - 40) / count;

    for i in 0..count {
        let geometry = WindowGeometry {
            anchor_point: AnchorPoint {
                x: AnchorAlignment::START,
                y: AnchorAlignment::CENTER,
            },
            offset: Coords {
                x: NumWithUnit::Pixels(20 + gap * i),
                y: NumWithUnit::Pixels(0),
            },
            size: Coords {
                x: NumWithUnit::Pixels(gap - 10),
                y: NumWithUnit::Pixels(150),
            },
        };
        let (actual_window_rect, x, y) = {
            let rect = get_window_rectangle(geometry, monitor_geometry);
            (Some(rect), rect.x(), rect.y())
        };

        let window_type = if false {
            gtk::WindowType::Popup
        } else {
            gtk::WindowType::Toplevel
        };
        let w = Window::new(window_type, x, y);
        w.set_resizable(false);
        w.set_keep_above(true);
        w.set_keep_below(false);
        if false {
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

        let label = gtk::Label::new(Some(&format!("Hello World at {i}")));
        w.add(&label);

        w.realize();

        if true {
            let _ = apply_window_position(geometry, monitor_geometry, &w);
            if true {
                w.connect_configure_event(move |window, _| {
                    let _ = apply_window_position(geometry, monitor_geometry, window);
                    false
                });
            }
            let backend = X11BackendConnection::new().unwrap();
            backend.set_xprops_for(&w, get_gdk_monitor()).unwrap();
        }

        w.show_all();
    }

    gtk::main();
    println!("Hello, world!");
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

/// Get the monitor geometry of a given monitor, or the default if none is given
fn get_gdk_monitor() -> Monitor {
    let display = gdk::Display::default().expect("could not get default display");
    let monitor = display
            .primary_monitor()
            .expect("Failed to get primary monitor from GTK. Try explicitly specifying the monitor on your window.");

    monitor
}

pub fn get_window_rectangle(
    geometry: WindowGeometry,
    screen_rect: gdk::Rectangle,
) -> gdk::Rectangle {
    let (offset_x, offset_y) = geometry
        .offset
        .relative_to(screen_rect.width(), screen_rect.height());
    let (width, height) = geometry
        .size
        .relative_to(screen_rect.width(), screen_rect.height());
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

impl WindowGeometry {
    pub fn override_if_given(
        &self,
        anchor_point: Option<AnchorPoint>,
        offset: Option<Coords>,
        size: Option<Coords>,
    ) -> Self {
        WindowGeometry {
            anchor_point: anchor_point.unwrap_or(self.anchor_point),
            offset: offset.unwrap_or(self.offset),
            size: size.unwrap_or(self.size),
        }
    }
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
    pub x: NumWithUnit,
    pub y: NumWithUnit,
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
        Coords {
            x: NumWithUnit::Pixels(x),
            y: NumWithUnit::Pixels(y),
        }
    }

    /// resolve the possibly relative coordinates relative to a given containers size
    pub fn relative_to(&self, width: i32, height: i32) -> (i32, i32) {
        (
            self.x.pixels_relative_to(width),
            self.y.pixels_relative_to(height),
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum NumWithUnit {
    Percent(f32),
    Pixels(i32),
}

impl fmt::Display for NumWithUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pixels(p) => write!(f, "{p}px"),
            Self::Percent(p) => write!(f, "{p}%"),
        }
    }
}

impl Default for NumWithUnit {
    fn default() -> Self {
        Self::Pixels(0)
    }
}

impl NumWithUnit {
    pub fn pixels_relative_to(&self, max: i32) -> i32 {
        match *self {
            NumWithUnit::Percent(n) => ((max as f64 / 100.0) * n as f64) as i32,
            NumWithUnit::Pixels(n) => n,
        }
    }

    pub fn perc_relative_to(&self, max: i32) -> f32 {
        match *self {
            NumWithUnit::Percent(n) => n,
            NumWithUnit::Pixels(n) => ((n as f64 / max as f64) * 100.0) as f32,
        }
    }
}
