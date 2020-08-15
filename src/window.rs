
use std::error::Error;
use std::collections::HashMap;

use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::errors::ConnectionError;
use x11rb::connection::Connection;
use x11rb::wrapper::ConnectionExt;


pub struct Window<'a, T: Connection + ConnectionExt> {
    window : u32,
    colormap : u32,
    conn : &'a T
}

pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color{r,g,b,a}
    }
    pub fn as_xcolor(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

pub enum Drawable<'a> {
    Rect(Color),
    Text(Color, &'a str)
}

impl Drawable<'_> {
    pub fn draw<T: Connection + ConnectionExt>(&self, window: &Window<T>, rect: Rectangle) -> Result<(), Box<dyn Error>> {
        match self {
            Drawable::Rect(c) => {
                let gc = window.conn.generate_id()?;

                window.conn.create_gc(gc, window.window, &CreateGCAux::new().foreground(c.as_xcolor()))?;
                window.conn.poly_fill_rectangle(window.window, gc, &[rect])?;
                
                window.conn.flush()?;

                window.conn.free_gc(gc)?;
            }

            Drawable::Text(c, text) => {
                // TODO
            }
        }
        Ok(())
    }
}

impl<T: Connection + ConnectionExt> Window<'_, T> {
    pub fn new<'a>(conn: &'a T, screen: &Screen) -> Result<Window<'a, T>, Box<dyn Error>> {
                
        let window = conn.generate_id()?;

        conn.create_window(x11rb::COPY_DEPTH_FROM_PARENT, window, screen.root,
                           0,0,300,300, 0, WindowClass::InputOutput, 0,
                           &CreateWindowAux::new()
                                .background_pixel(Color::new(255,100,200,150).as_xcolor())
                                .event_mask(EventMask::Exposure
                                          | EventMask::ButtonPress
                                          | EventMask::ButtonRelease
                                          | EventMask::PointerMotion))?;


        let colormap = screen.default_colormap;
        
        conn.change_property8(PropMode::Replace, window, AtomEnum::WM_NAME, AtomEnum::STRING, b"Ravenbar")?;

        conn.map_window(window)?;
        conn.flush()?;

        let wnd = Window {window, colormap, conn};

        wnd.set_atom32(b"_NET_WM_WINDOW_TYPE", PropMode::Replace, AtomEnum::ATOM, 
                       &[wnd.get_atom(b"_NET_WM_WINDOW_TYPE_DOCK")?])?;
        wnd.set_atom32(b"_NET_WM_DESKTOP", PropMode::Replace, AtomEnum::CARDINAL, 
                       &[0xFFFFFFFF])?;
        
        wnd.flush()?;

        Ok(wnd)
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

    pub fn flush(&self) -> Result<(), ConnectionError> {
        self.conn.flush()
    }
}
