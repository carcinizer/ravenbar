
use std::error::Error;
use std::collections::HashMap;
use std::rc::Rc;

use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::errors::ConnectionError;
use x11rb::connection::Connection;
use x11rb::wrapper::ConnectionExt;

use crate::font::Font;

// Just an alias for convenience
pub trait XConnection: Connection + ConnectionExt {}
impl<T: Connection + ConnectionExt> XConnection for T {}

pub struct Window<'a, T: XConnection> {
    // Maybe change this in the future
    pub window: u32,
    pub colormap: u32,
    pub conn: &'a T,
    pub fontconfig: fontconfig::Fontconfig,
    pub depth: u8,

    screen: &'a Screen
}
#[derive(Copy, Clone, PartialEq)]
pub struct Direction {
    // -1 - left, 0 - center, 1 - right
    pub xdir: i8,
    // -1 - top, 0 - center, 1 - bottom
    pub ydir: i8
}

impl Direction {
    pub fn from(s: String) -> Self {
        let ydir = match &s[0..1] {
            "N" => -1,
            "S" => 1,
            _ => {panic!("{} is not a valid direction", s);}
        };
        let xdir = if s.len() == 2 {
            match &s[1..2] {
                "W" => -1,
                "E" => 1,
                _ => {panic!("{} is not a valid direction", s);}
            }
        } else {0};
        Self {xdir, ydir}
    }
}

#[derive(PartialEq)]
pub struct WindowGeometry {
    pub dir: Direction,
    pub xoff: i16,
    pub yoff: i16,
    pub w: u16,
    pub h: u16
}

impl WindowGeometry {
    pub fn new() -> Self {
        Self {dir: Direction::from("N".to_owned()), xoff:0,yoff:0,w:0,h:0}
    }

    pub fn on_screen(&self, scrw: u16, scrh: u16) -> (i16, i16, u16, u16) {

        let xoff = if self.dir.xdir == 0 {self.xoff} else {self.xoff.abs() * -self.dir.xdir as i16};
        let yoff = if self.dir.ydir == 0 {self.yoff} else {self.yoff.abs() * -self.dir.ydir as i16};

        let x = ((self.dir.xdir + 1) as i16) * (scrw - self.w) as i16 / 2 + xoff;
        let y = ((self.dir.ydir + 1) as i16) * (scrh - self.h) as i16 / 2 + yoff;
        let width = self.w;
        let height = self.h;
        
        (x,y,width,height)
    }

    pub fn cropped(&self, x: i16, y: i16, w: u16, h: u16) -> Self {
        let xoff = match self.dir.xdir {
            0  => self.xoff + x,
            -1 => self.xoff.abs() + x,
            1  => self.xoff.abs() - x + self.w as i16 - w as i16,
            _  => panic!("You weren't supposed to see this error")
        };
        let yoff = match self.dir.ydir {
            0  => self.yoff + y,
            -1 => self.yoff.abs() + y,
            1  => self.yoff.abs() - y + self.h as i16 - h as i16,
            _  => panic!("You weren't supposed to see this error")
        };

        Self {dir: self.dir, xoff, yoff, w, h}
    }

    pub fn has_point(&self, px: i16, py: i16, scrw: u16, scrh: u16) -> bool {
        let (x,y,w,h) = self.on_screen(scrw, scrh);
        px >= x && py >= y && px <= x + w as i16 && py <= y + h as i16
    }

    pub fn strut(&self) -> [u32; 12] {
        [
            0,
            0,
            if self.dir.ydir == -1 {(self.h as i16 + self.yoff) as u32} else {0},
            if self.dir.ydir ==  1 {(self.h as i16 + self.xoff) as u32} else {0},
            0,0,0,0,0,0,0,0
        ]
    }
}

/// Get a visual with alpha, hopefully
fn get_depth_visual(screen: &Screen) -> (u8, Visualid) {
    for i in screen.allowed_depths.iter() {
        if i.depth == 32 {
           return (i.depth, i.visuals[0].visual_id);
        }
    }
    (x11rb::COPY_DEPTH_FROM_PARENT, screen.root_visual)
}

impl<T: XConnection> Window<'_, T> {
    pub fn new<'a>(conn: &'a T, screen_num: usize) -> Result<Window<'a, T>, Box<dyn Error>> {
        
        let screen = &conn.setup().roots[screen_num];

        let (depth, visual) = get_depth_visual(screen);

        let window = conn.generate_id()?;
        let colormap = conn.generate_id()?;

        conn.create_colormap(ColormapAlloc::None, colormap, screen.root, visual)?.check()?;

        conn.create_window(depth, window, screen.root,
                           0,0,100,100, 0, WindowClass::InputOutput, visual,
                           &CreateWindowAux::new()
                                .background_pixel(x11rb::NONE)
                                .border_pixel(screen.black_pixel)
                                .colormap(colormap)
                                .event_mask(EventMask::ButtonPress
                                          | EventMask::ButtonRelease
                                          | EventMask::Exposure)
        )?.check()?;

        
        conn.change_property8(PropMode::Replace, window, AtomEnum::WM_NAME, AtomEnum::STRING, b"Ravenbar")?;

        conn.flush()?;

        let fontconfig = fontconfig::Fontconfig::new().unwrap();

        let wnd = Window {window, colormap, conn, fontconfig, screen, depth};

        Ok(wnd)
    }

    pub fn configure(&self, geom: &WindowGeometry) -> Result<(), Box<dyn Error>> {

        let (x,y,w,h) = geom.on_screen(self.screen.width_in_pixels, self.screen.height_in_pixels);


        self.set_atom32(b"_NET_WM_WINDOW_TYPE", PropMode::Replace, AtomEnum::ATOM, 
                       &[self.get_atom(b"_NET_WM_WINDOW_TYPE_DOCK")?])?;
        self.set_atom32(b"_NET_WM_DESKTOP", PropMode::Replace, AtomEnum::CARDINAL, 
                       &[0xFFFFFFFF])?;
        self.set_atom32(b"_NET_WM_STATE", PropMode::Append, AtomEnum::ATOM, 
                       &[self.get_atom(b"_NET_WM_STATE_STICKY")?,
                         self.get_atom(b"_NET_WM_STATE_STAYS_ON_TOP")?])?;
        self.set_atom32(b"_NET_WM_ALLOWED_ACTIONS", PropMode::Replace, AtomEnum::ATOM, 
                       &[])?;



        self.set_atom32(b"_NET_WM_STRUT", PropMode::Replace, AtomEnum::CARDINAL, 
                       &geom.strut()[0..4])?;
        self.set_atom32(b"_NET_WM_STRUT_PARTIAL", PropMode::Replace, AtomEnum::CARDINAL, 
                       &geom.strut())?;

        self.conn.map_window(self.window)?;

        // Ensure window's position
        let aux = &ConfigureWindowAux::new().x(x as i32).y(y as i32).width(w as u32).height(h as u32);
        self.conn.configure_window(self.window, aux)?;
        
        self.flush()?;
        Ok(())
    }

    pub fn get_atom(&self, name: &[u8]) -> Result<Atom, Box<dyn Error>> {
        Ok(self.conn.intern_atom(false, name)?.reply()?.atom)
    }

    pub fn set_atom8(&self, name: &[u8], mode: PropMode, atype: AtomEnum, data: &[u8]) -> Result<(), Box<dyn Error>>{
        let atom = self.get_atom(name)?;
        
        self.conn.change_property8(mode, self.window, atom, atype, data)?;
        Ok(())
    }

    pub fn set_atom32(&self, name: &[u8], mode: PropMode, atype: AtomEnum, data: &[u32]) -> Result<(), Box<dyn Error>>{
        let atom = self.get_atom(name)?;
        
        self.conn.change_property32(mode, self.window, atom, atype, data)?;
        Ok(())
    }

    pub fn screen_width(&self) -> u16 {
        self.screen.width_in_pixels
    }

    pub fn screen_height(&self) -> u16 {
        self.screen.height_in_pixels
    }

    pub fn get_pointer(&self) -> Result<(i16, i16), Box<dyn Error>> {
        let pointer = self.conn.query_pointer(self.window)?.reply()?;
        Ok((pointer.root_x, pointer.root_y))
    }

    pub fn flush(&self) -> Result<(), ConnectionError> {
        self.conn.flush()
    }
}
