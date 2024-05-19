use std::fmt;

use gdk::Monitor;
use tokio::time::Instant;
use twitch_irc::message::Emote;

use glib::{object_subclass, wrapper};
use glib_macros::Properties;
use gtk::prelude::{ContainerExt, GtkWindowExt, WidgetExt};
use gtk::{prelude::*, subclass::prelude::*};
use std::cell::RefCell;

wrapper! {
    pub struct Window(ObjectSubclass<WindowPriv>)
    @extends gtk::Window, gtk::Bin, gtk::Container, gtk::Widget, @implements gtk::Buildable;
}

#[derive(Properties)]
#[properties(wrapper_type = Window)]
pub struct WindowPriv {
    #[property(
        get,
        name = "x",
        nick = "X",
        blurb = "Global x coordinate",
        default = 0
    )]
    x: RefCell<i32>,

    #[property(
        get,
        name = "y",
        nick = "Y",
        blurb = "Global y coordinate",
        default = 0
    )]
    y: RefCell<i32>,
}

// This should match the default values from the ParamSpecs
impl Default for WindowPriv {
    fn default() -> Self {
        WindowPriv {
            x: RefCell::new(0),
            y: RefCell::new(0),
        }
    }
}

#[object_subclass]
impl ObjectSubclass for WindowPriv {
    type ParentType = gtk::Window;
    type Type = Window;

    const NAME: &'static str = "WindowEww";
}

impl Default for Window {
    fn default() -> Self {
        glib::Object::new::<Self>()
    }
}

impl Window {
    pub fn new(type_: gtk::WindowType, x_: i32, y_: i32) -> Self {
        let w: Self = glib::Object::builder().property("type", type_).build();
        let priv_ = w.imp();
        priv_.x.replace(x_);
        priv_.y.replace(y_);
        w
    }
}

impl ObjectImpl for WindowPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }
}
impl WindowImpl for WindowPriv {}
impl BinImpl for WindowPriv {}
impl ContainerImpl for WindowPriv {}
impl WidgetImpl for WindowPriv {}

#[derive(Clone, Debug)]
pub struct SpawnedWindow {
    pub w: Window,
    pub progress: gtk::ProgressBar,
    pub created: Instant,
}

pub fn init_window(pos: (i32, i32), monitor_geometry: gdk::Rectangle) -> (Option<WindowGeometry>, Window) {
    #[cfg(target_os = "linux")]
    {
        crate::x11::a(pos, monitor_geometry)
    } 
    #[cfg(not(target_os = "linux"))]
    {
        (None, Window::new(gtk::WindowType::Toplevel, pos.0, pos.1))
    }
}

pub async fn spawn_window(
    user: &str,
    message: &str,
    emotes: &[Emote],
    pos: (i32, i32),
    monitor_geometry: gdk::Rectangle,
) -> SpawnedWindow {
    let (geometry, w) = init_window(pos, monitor_geometry);

    let progress = {
        let layout = gtk::Box::new(gtk::Orientation::Vertical, 5);

        let username = gtk::Label::new(Some(user));
        layout.add(&username);

        let messagebox = gtk::Box::new(gtk::Orientation::Horizontal, 2);

        let mut start = 0;
        for emote in emotes {
            let plain = start..emote.char_range.start;
            if !plain.is_empty() {
                let plain_txt = &message[plain];
                let label = gtk::Label::new(Some(plain_txt));
                messagebox.add(&label);
            }

            start = emote.char_range.end;

            let emote_id = &emote.id;
            let img = load_emote(emote_id).await;

            messagebox.add(&img);
        }

        let plain = start..message.len();
        if !plain.is_empty() {
            let plain_txt = &message[plain];
            let label = gtk::Label::new(Some(plain_txt));
            messagebox.add(&label);
        }

        layout.add(&messagebox);

        let progress = gtk::ProgressBar::new();
        layout.add(&progress);

        w.add(&layout);
        progress
    };

    w.realize();

    #[cfg(target_os = "linux")]
    {
        crate::x11::b(w.clone(), monitor_geometry, geometry.unwrap())
    }

    w.show_all();

    SpawnedWindow {
        w,
        progress,
        created: Instant::now(),
    }
}

async fn load_emote(id: &str) -> gtk::Image {
    let img = gtk::Image::new();

    // TODO: load_from cache
    if let Some(pixbuf) = load_emote_(id, "animated", "image/gif").await {
        img.set_pixbuf_animation(pixbuf.animation().as_ref());
    } else if let Some(pixbuf) = load_emote_(id, "static", "image/png").await {
        img.set_pixbuf(pixbuf.pixbuf().as_ref());
    } else {
        eprintln!("Cannot load emote: {id}")
    }

    img
}

async fn load_emote_(
    id: &str,
    format: &str,
    mime_type: &str,
) -> Option<gtk::gdk_pixbuf::PixbufLoader> {
    let url_gif = format!("https://static-cdn.jtvnw.net/emoticons/v2/{id}/{format}/dark/1.0");
    let Ok(emote_res) = reqwest::get(&url_gif).await else {
        println!("Error getting emote");
        return None;
    };

    if emote_res.status() == 404 {
        return None;
    }

    let img_src = emote_res.bytes().await.expect("Error getting emote");
    let img_loader = gtk::gdk_pixbuf::PixbufLoader::with_mime_type(mime_type)
        .expect("Cannot create image loader");
    _ = img_loader.write(&img_src);
    _ = img_loader.close();
    Some(img_loader)
}

/// Get the monitor geometry of a given monitor, or the default if none is given
pub fn get_gdk_monitor() -> Monitor {
    let display = gdk::Display::default().expect("could not get default display");
    let monitor = display
            .primary_monitor()
            .expect("Failed to get primary monitor from GTK. Try explicitly specifying the monitor on your window.");

    monitor
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
